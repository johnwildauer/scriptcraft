//! Story Map-local primary Beat move and tail-resize gestures.

use std::sync::Arc;

use eframe::egui::{self, Pos2, Rect, Sense};
use scriptcraft_commands::{
    BeatCardIdValue, Command, StoryMapLaneIdValue, StoryMapLengthEighthsValue,
    StoryMapStartEighthsValue,
};
use scriptcraft_model::{Project, StableId, StoryMapLane, StoryMapPlacement};
use scriptcraft_session::editing::{
    plan_story_map_placement, StoryMapPlacementChangeKind, StoryMapPlacementPlanError,
    StoryMapPlacementRequest,
};
use crate::views::beat_drag::{
    BeatDragCoordinator, BeatDragSource, BeatPlacementPreview,
};

use super::{
    constants::STORY_MAP_BEAT_DRAG_THRESHOLD,
    geometry::{placed_beat_rect, StripGeometry, TimelineGeometry},
    pointer::{self, BeatPointerMode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragMode {
    Move,
    TailResize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    lane_uuid: String,
    start_eighths: u64,
    length_eighths: u64,
}

#[derive(Debug, Clone)]
struct Press {
    generation: u64,
    card_id: StableId,
    source_lane_uuid: String,
    original_start: u64,
    original_length: u64,
    pointer_to_anchor_page: f64,
    origin: Pos2,
    latest_pointer: Pos2,
    document_identity: Arc<()>,
    mutation_token: u64,
    target_pages: u32,
    lane_ids: Vec<String>,
    lane_rects: Vec<Rect>,
    timeline: TimelineGeometry,
    mode: DragMode,
    crossed_threshold: bool,
    candidate: Option<Candidate>,
}

#[derive(Debug, Default)]
pub(crate) struct StoryMapBeatInteractionState {
    press: Option<Press>,
    swallow_until_release: bool,
    planner_preview: Option<PlannerPreviewCache>,
    external_geometry: Option<(u64, TimelineGeometry, Vec<Rect>)>,
    primary_down: bool,
}

#[derive(Debug, Default)]
pub(crate) struct BeatInteractionOutput {
    pub(crate) suppressed_cards: Vec<StableId>,
    pub(crate) ghosts: Vec<BeatGroupGhost>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BeatGroupRole {
    Primary,
    CarriedFollower,
    Displaced,
    PropagatedFollower,
    Rejected,
}

#[derive(Debug, Clone)]
pub(crate) struct BeatGroupGhost {
    pub(crate) card_id: StableId,
    pub(crate) lane_uuid: String,
    pub(crate) start_eighths: u64,
    pub(crate) length_eighths: u64,
    pub(crate) role: BeatGroupRole,
}

#[derive(Debug, Clone)]
struct PreviewKey {
    generation: u64,
    card_id: StableId,
    source: BeatDragSource,
    document_identity: Arc<()>,
    mutation_token: u64,
    primary_original: Option<StoryMapPlacement>,
    target_pages: u32,
    board_identity: Option<String>,
    story_map_identity: Option<String>,
    lane_ids: Vec<String>,
    card_semantics: Vec<(StableId, String, Option<StoryMapPlacement>)>,
    request: Candidate,
}

impl PreviewKey {
    fn matches(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.card_id == other.card_id
            && self.source == other.source
            && Arc::ptr_eq(&self.document_identity, &other.document_identity)
            && self.mutation_token == other.mutation_token
            && self.primary_original == other.primary_original
            && self.target_pages == other.target_pages
            && self.board_identity == other.board_identity
            && self.story_map_identity == other.story_map_identity
            && self.lane_ids == other.lane_ids
            && self.card_semantics == other.card_semantics
            && self.request == other.request
    }
}

#[derive(Debug, Clone)]
enum CachedPlan {
    Valid(Vec<BeatGroupGhost>),
    NoOp,
    Rejected,
    Failed,
}

#[derive(Debug, Clone)]
struct PlannerPreviewCache {
    key: PreviewKey,
    plan: CachedPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SaveProjectSignature {
    target_pages: u32,
    story_map_identity: Option<String>,
    lanes: Vec<StoryMapLane>,
    board_identity: Option<String>,
    cards: Vec<(StableId, String, Option<StoryMapPlacement>)>,
}

impl SaveProjectSignature {
    fn capture(project: &Project) -> Self {
        Self {
            target_pages: project.story_map().target_pages,
            story_map_identity: project.story_map().foreign_id.clone(),
            lanes: project.story_map().lanes.clone(),
            board_identity: project.beat_board().foreign_id.clone(),
            cards: project.beat_board().cards.iter().map(|card| (
                card.id.clone(), card.foreign_id.clone(), card.story_map_placement.clone(),
            )).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoryMapSaveCompatibility {
    generation: u64,
    source: BeatDragSource,
    card_id: StableId,
    durable_card_id: String,
    original: Option<StoryMapPlacement>,
    source_lane_uuid: Option<String>,
    destination_lane_uuid: Option<String>,
    document_identity: Arc<()>,
    mutation_token: u64,
    pointer_origin: Pos2,
    current_pointer: Pos2,
    crossed_threshold: bool,
    timeline: Option<TimelineGeometry>,
    lane_ids: Vec<String>,
    lane_rects: Vec<Rect>,
    request: Option<Candidate>,
    project: SaveProjectSignature,
}

impl StoryMapBeatInteractionState {
    pub(crate) fn reset(&mut self) {
        self.press = None;
        self.swallow_until_release = false;
        self.planner_preview = None;
        self.external_geometry = None;
        self.primary_down = false;
    }

    pub(crate) fn blocks_auto_scroll(&self) -> bool {
        self.press.is_some()
            || self.swallow_until_release
            || self.external_geometry.is_some()
    }

    pub(crate) fn last_primary_down(&self) -> bool {
        self.primary_down
    }

    pub(crate) fn cancel_for_unavailable_geometry(&mut self, primary_down: bool, coordinator: &mut BeatDragCoordinator) {
        self.primary_down = primary_down;
        self.cancel_capture(primary_down, coordinator);
        if let Some(candidate) = coordinator.candidate().filter(|candidate| {
            candidate.source == BeatDragSource::BeatBoard && candidate.externally_engaged
        }) {
            coordinator.claim_invalid(candidate.generation);
        }
        if let Some(candidate) = coordinator.candidate().filter(|candidate| {
            candidate.source == BeatDragSource::BeatBoard && !candidate.externally_engaged
        }) {
            coordinator.cancel_candidate(candidate.generation, BeatDragSource::BeatBoard);
        }
    }

    pub(crate) fn cancel_local_capture(&mut self, primary_down: bool, coordinator: &mut BeatDragCoordinator) {
        self.primary_down = primary_down;
        self.cancel_capture(primary_down, coordinator);
    }

    pub(crate) fn reconcile_semantic_change(
        &mut self,
        primary_down: bool,
        coordinator: &mut BeatDragCoordinator,
    ) {
        self.primary_down = primary_down;
        self.cancel_capture(primary_down, coordinator);
        if let Some(candidate) = coordinator.candidate().filter(|candidate| {
            candidate.source == BeatDragSource::BeatBoard
                && candidate.externally_engaged
                && candidate.terminal.is_none()
        }) {
            coordinator.claim_invalid(candidate.generation);
        }
    }

    pub(crate) fn complete_terminal_command(&mut self) {
        self.press = None;
        self.planner_preview = None;
        self.external_geometry = None;
    }

    pub(crate) fn capture_save_compatibility(
        &self,
        project: &Project,
        coordinator: &BeatDragCoordinator,
    ) -> Option<StoryMapSaveCompatibility> {
        if let Some(press) = self.press.as_ref() {
            let card = unique_card(project, &press.card_id)?;
            return Some(StoryMapSaveCompatibility {
                generation: press.generation,
                source: BeatDragSource::StoryMap,
                card_id: press.card_id.clone(),
                durable_card_id: card.foreign_id.clone(),
                original: card.story_map_placement.clone(),
                source_lane_uuid: card.story_map_placement.as_ref().map(|placement| placement.lane_uuid.clone()),
                destination_lane_uuid: press.candidate.as_ref().map(|candidate| candidate.lane_uuid.clone()),
                document_identity: Arc::clone(&press.document_identity),
                mutation_token: press.mutation_token,
                pointer_origin: press.origin,
                current_pointer: press.latest_pointer,
                crossed_threshold: press.crossed_threshold,
                timeline: Some(press.timeline),
                lane_ids: press.lane_ids.clone(),
                lane_rects: press.lane_rects.clone(),
                request: press.candidate.clone(),
                project: SaveProjectSignature::capture(project),
            });
        }

        let candidate = coordinator.active_for(BeatDragSource::BeatBoard)?;
        if !candidate.externally_engaged || candidate.terminal.is_some() {
            return None;
        }
        let (generation, timeline, lane_rects) = self.external_geometry.as_ref()
            .filter(|(generation, _, _)| *generation == candidate.generation)?;
        let card = unique_card(project, &candidate.card_id)?;
        let request = candidate.preview.as_ref().map(|preview| Candidate {
            lane_uuid: preview.lane_uuid.clone(),
            start_eighths: preview.start_eighths,
            length_eighths: preview.length_eighths,
        });
        Some(StoryMapSaveCompatibility {
            generation: *generation,
            source: BeatDragSource::BeatBoard,
            card_id: candidate.card_id,
            durable_card_id: card.foreign_id.clone(),
            original: card.story_map_placement.clone(),
            source_lane_uuid: card.story_map_placement.as_ref().map(|placement| placement.lane_uuid.clone()),
            destination_lane_uuid: request.as_ref().map(|request| request.lane_uuid.clone()),
            document_identity: candidate.document_identity,
            mutation_token: candidate.mutation_token,
            pointer_origin: candidate.press_origin,
            current_pointer: candidate.current_pointer,
            crossed_threshold: true,
            timeline: Some(*timeline),
            lane_ids: project.story_map().lanes.iter().map(|lane| lane.uuid.clone()).collect(),
            lane_rects: lane_rects.clone(),
            request,
            project: SaveProjectSignature::capture(project),
        })
    }

    pub(crate) fn reconcile_after_successful_save(
        &mut self,
        compatibility: Option<StoryMapSaveCompatibility>,
        rebased_project: &Project,
        new_document_identity: Arc<()>,
        new_mutation_token: u64,
        primary_down: bool,
        coordinator: &mut BeatDragCoordinator,
        board_interaction: &mut crate::views::beat_board::BeatInteraction,
    ) {
        self.primary_down = primary_down;
        let Some(snapshot) = compatibility else {
            if self.press.is_some() {
                self.cancel_capture(primary_down, coordinator);
            }
            if let Some(candidate) = coordinator.candidate().filter(|candidate| {
                candidate.source == BeatDragSource::BeatBoard
                    && candidate.externally_engaged
                    && candidate.terminal.is_none()
            }) {
                coordinator.claim_invalid(candidate.generation);
                self.planner_preview = None;
                self.external_geometry = None;
            }
            return;
        };

        let rebased_signature = SaveProjectSignature::capture(rebased_project);
        let same_project_inputs = snapshot.project == rebased_signature
            && durable_card_correspondence(rebased_project, &snapshot)
            && (!snapshot.crossed_threshold || snapshot.request.is_some())
            && snapshot.project.story_map_identity.is_some()
            && snapshot.project.board_identity.is_some()
            && snapshot.destination_lane_uuid.as_ref().is_none_or(|lane| {
                lane_resolves_once(rebased_project, lane)
            })
            && snapshot.source_lane_uuid.as_ref().is_none_or(|lane| {
                lane_resolves_once(rebased_project, lane)
            });

        let generation_still_active = match snapshot.source {
            BeatDragSource::StoryMap => self.press.as_ref().is_some_and(|press| {
                press.generation == snapshot.generation
                    && press.card_id == snapshot.card_id
                    && Arc::ptr_eq(&press.document_identity, &snapshot.document_identity)
                    && press.mutation_token == snapshot.mutation_token
                    && press.origin == snapshot.pointer_origin
                    && press.latest_pointer == snapshot.current_pointer
                    && Some(press.timeline) == snapshot.timeline
                    && press.lane_ids == snapshot.lane_ids
                    && press.lane_rects == snapshot.lane_rects
                    && press.candidate == snapshot.request
            }),
            BeatDragSource::BeatBoard => coordinator.active_for(BeatDragSource::BeatBoard).is_some_and(|candidate| {
                candidate.generation == snapshot.generation
                    && candidate.card_id == snapshot.card_id
                    && candidate.externally_engaged
                    && candidate.terminal.is_none()
                    && Arc::ptr_eq(&candidate.document_identity, &snapshot.document_identity)
                    && candidate.mutation_token == snapshot.mutation_token
                    && candidate.press_origin == snapshot.pointer_origin
                    && candidate.current_pointer == snapshot.current_pointer
                    && candidate.preview.as_ref().map(|preview| Candidate {
                        lane_uuid: preview.lane_uuid.clone(),
                        start_eighths: preview.start_eighths,
                        length_eighths: preview.length_eighths,
                    }) == snapshot.request
                    && self.external_geometry.as_ref().is_some_and(|(generation, timeline, rows)| {
                        *generation == snapshot.generation
                            && Some(*timeline) == snapshot.timeline
                            && *rows == snapshot.lane_rects
                    })
                    && board_interaction.matches_external_generation(
                        snapshot.generation,
                        snapshot.mutation_token,
                    )
            }),
        };

        if same_project_inputs && generation_still_active {
            let refreshed = coordinator.rebase_context(
                snapshot.generation,
                snapshot.source,
                Arc::clone(&new_document_identity),
                new_mutation_token,
            );
            if refreshed {
                if snapshot.source == BeatDragSource::BeatBoard
                    && !board_interaction.rebase_external_generation(
                        snapshot.generation,
                        snapshot.mutation_token,
                        new_mutation_token,
                    )
                {
                    coordinator.claim_invalid(snapshot.generation);
                    self.planner_preview = None;
                    self.external_geometry = None;
                    return;
                }
                if let Some(press) = self.press.as_mut().filter(|press| {
                    press.generation == snapshot.generation
                }) {
                    press.document_identity = Arc::clone(&new_document_identity);
                    press.mutation_token = new_mutation_token;
                }
                self.planner_preview = None;
                return;
            }
        }

        self.planner_preview = None;
        if let Some(press) = self.press.as_ref().filter(|press| press.generation == snapshot.generation) {
            let generation = press.generation;
            self.press = None;
            self.swallow_until_release = primary_down;
            coordinator.cancel_generation(generation, BeatDragSource::StoryMap);
        }
        if let Some(candidate) = coordinator.active_for(snapshot.source).filter(|candidate| {
            candidate.generation == snapshot.generation && candidate.terminal.is_none()
        }) {
            if candidate.source == BeatDragSource::BeatBoard && candidate.externally_engaged {
                coordinator.claim_invalid(candidate.generation);
            } else {
                coordinator.cancel_generation(candidate.generation, candidate.source);
            }
        }
        self.external_geometry = None;
    }

    pub(crate) fn process(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        document_identity: Arc<()>,
        mutation_token: u64,
        strip: &StripGeometry,
        timeline: &TimelineGeometry,
        enabled: bool,
        coordinator: &mut BeatDragCoordinator,
    ) -> BeatInteractionOutput {
        let input = ui.ctx().input(|input| {
            (
                input.pointer.primary_pressed(),
                input.pointer.primary_released(),
                input.pointer.primary_down(),
                input.pointer.interact_pos(),
                input.pointer.press_origin(),
                input.viewport().focused != Some(false),
                input.key_pressed(egui::Key::Escape),
                input.events.iter().any(|event| matches!(event, egui::Event::PointerGone)),
            )
        });
        let (pressed, released, down, pointer, press_origin, focused, escape, pointer_gone) = input;
        self.primary_down = down;
        if let Some(output) = self.process_external_candidate(
            ui, project, &document_identity, mutation_token, strip, timeline, enabled,
            pointer, released, down, focused, escape, pointer_gone, coordinator,
        ) {
            return output;
        }
        if escape && self.press.is_some() && enabled {
            self.cancel_capture(down, coordinator);
            ui.ctx().input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
        }
        let Some(pointer) = pointer.filter(|point| finite_pos(*point)) else {
            if self.press.is_some() {
                self.cancel_capture(down, coordinator);
            }
            self.finish_swallow(released, down);
            return BeatInteractionOutput::default();
        };

        if self.press.is_some() && (!focused || !enabled || pointer_gone) {
            self.cancel_capture(down, coordinator);
        }
        let lanes = &project.story_map().lanes;
        let target_pages = project.story_map().target_pages;
        let lane_ids: Vec<String> = lanes.iter().map(|lane| lane.uuid.clone()).collect();
        let lane_rects = strip.lane_rows.clone();

        if let Some(press) = self.press.as_ref() {
            let compatible = Arc::ptr_eq(&press.document_identity, &document_identity)
                && press.mutation_token == mutation_token
                && press.target_pages == target_pages
                && press.lane_ids == lane_ids
                && project.beat_board().cards.iter().any(|card| {
                    card.id == press.card_id
                        && card.story_map_placement.as_ref().is_some_and(|placement| {
                            placement.lane_uuid == press.source_lane_uuid
                                && placement.start_eighths == press.original_start
                                && placement.length_eighths == press.original_length
                        })
                });
            if !compatible {
                self.cancel_capture(down, coordinator);
            } else if let Some(press) = self.press.as_mut() {
                // Navigation/viewport changes only alter the presentation transform.
                // Keep the exact semantic request key and remap the live pointer
                // through the current geometry without rerunning a same-key plan.
                press.lane_rects = lane_rects.clone();
                press.timeline = *timeline;
            }
        }

        if let Some(press) = self.press.as_ref() {
            self.register_captured_target(ui, project, strip, timeline, press);
        }

        let hit_point = if pressed {
            press_origin.filter(|point| finite_pos(*point)).unwrap_or(pointer)
        } else {
            pointer
        };
        let hit = if self.press.is_none() && !self.swallow_until_release && enabled {
            pointer::hit_placed_beat(ui, strip, timeline, project, hit_point)
        } else {
            None
        };
        if pressed {
            if let Some((card_id, pointer_mode)) = hit {
                let Some(card) = project.beat_board().cards.iter().find(|card| card.id == card_id) else {
                    return BeatInteractionOutput::default();
                };
                let Some(placement) = card.story_map_placement.as_ref() else {
                    return BeatInteractionOutput::default();
                };
                let mode = match pointer_mode {
                    BeatPointerMode::Move => DragMode::Move,
                    BeatPointerMode::TailResize => DragMode::TailResize,
                };
                if let Some(pointer_page) = timeline.screen_to_page(f64::from(hit_point.x)) {
                    let anchor_page = match mode {
                        DragMode::Move => placement.start_eighths as f64 / 8.0,
                        DragMode::TailResize => match placement.checked_end_eighths() {
                            Some(end) => end as f64 / 8.0,
                            None => f64::NAN,
                        },
                    };
                    let offset = pointer_page - anchor_page;
                    if anchor_page.is_finite() && offset.is_finite() {
                        let Some(generation) = coordinator.begin(
                            BeatDragSource::StoryMap,
                            card_id.clone(),
                            Arc::clone(&document_identity),
                            mutation_token,
                            hit_point,
                            pointer,
                        ) else { return BeatInteractionOutput::default(); };
                        let mut press_state = Press {
                            generation,
                            card_id: card_id.clone(),
                            source_lane_uuid: placement.lane_uuid.clone(),
                            original_start: placement.start_eighths,
                            original_length: placement.length_eighths,
                            pointer_to_anchor_page: offset,
                            origin: hit_point,
                            latest_pointer: pointer,
                            document_identity: Arc::clone(&document_identity),
                            mutation_token,
                            target_pages,
                            lane_ids,
                            lane_rects,
                            timeline: *timeline,
                            mode,
                            crossed_threshold: false,
                            candidate: None,
                        };
                        update_candidate(&mut press_state, pointer, strip, lanes, timeline);
                        self.press = Some(press_state);
                    } else {
                        self.cancel_capture(down, coordinator);
                    }
                } else {
                    self.cancel_capture(down, coordinator);
                }
            }
        }

        if let Some(press_state) = self.press.as_mut() {
            press_state.latest_pointer = pointer;
            let dx = pointer.x - press_state.origin.x;
            let dy = pointer.y - press_state.origin.y;
            if dx.is_finite() && dy.is_finite()
                && f64::from(dx).hypot(f64::from(dy)) >= f64::from(STORY_MAP_BEAT_DRAG_THRESHOLD)
            {
                press_state.crossed_threshold = true;
            }
            if press_state.crossed_threshold {
                let latest_pointer = press_state.latest_pointer;
                update_candidate(press_state, latest_pointer, strip, lanes, timeline);
                coordinator.publish_candidate(press_state.generation, latest_pointer);
                coordinator.set_preview(press_state.generation, press_state.candidate.as_ref().map(|candidate| {
                    BeatPlacementPreview {
                        lane_uuid: candidate.lane_uuid.clone(),
                        start_eighths: candidate.start_eighths,
                        length_eighths: candidate.length_eighths,
                    }
                }));
            }
        }

        let preview_request = self.press.as_ref().and_then(|press| {
            (press.crossed_threshold).then(|| (
                press.generation,
                press.card_id.clone(),
                Arc::clone(&press.document_identity),
                press.mutation_token,
                press.candidate.clone(),
                press.mode,
            ))
        });
        let mut output = preview_request.and_then(|(generation, card_id, identity, token, request, mode)| {
            ui.ctx().set_cursor_icon(match mode {
                DragMode::Move => egui::CursorIcon::Grabbing,
                DragMode::TailResize => egui::CursorIcon::ResizeHorizontal,
            });
            request.map(|request| self.planner_preview(
                project, generation, &card_id, BeatDragSource::StoryMap, identity, token, &request,
            ))
        }).unwrap_or_default();
        if self.press.as_ref().is_some_and(|press| press.crossed_threshold)
            && self.press.as_ref().is_some_and(|press| press.candidate.is_none())
        {
            self.planner_preview = None;
        }

        if released {
            if let Some(press_state) = self.press.take() {
                if press_state.crossed_threshold {
                    if let Some(candidate) = press_state.candidate {
                        let command = make_command(&press_state.card_id, candidate.clone());
                        let valid = command.is_some_and(|command| coordinator.claim_valid(
                            press_state.generation,
                            BeatPlacementPreview {
                                lane_uuid: candidate.lane_uuid,
                                start_eighths: candidate.start_eighths,
                                length_eighths: candidate.length_eighths,
                            },
                            command,
                        ));
                        if !valid { coordinator.claim_invalid(press_state.generation); }
                    } else {
                        coordinator.claim_invalid(press_state.generation);
                    }
                    coordinator.acknowledge_source(press_state.generation, BeatDragSource::StoryMap);
                    self.planner_preview = None;
                } else {
                    coordinator.finish_unclaimed(press_state.generation, BeatDragSource::StoryMap);
                    self.planner_preview = None;
                }
                self.swallow_until_release = false;
            }
            output = BeatInteractionOutput::default();
            self.finish_swallow(true, down);
        } else if self.press.is_some() && !down {
            self.cancel_capture(false, coordinator);
        }
        output
    }

    #[allow(clippy::too_many_arguments)]
    fn process_external_candidate(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        document_identity: &Arc<()>,
        mutation_token: u64,
        strip: &StripGeometry,
        timeline: &TimelineGeometry,
        enabled: bool,
        pointer: Option<Pos2>,
        released: bool,
        primary_down: bool,
        focused: bool,
        escape: bool,
        pointer_gone: bool,
        coordinator: &mut BeatDragCoordinator,
    ) -> Option<BeatInteractionOutput> {
        let candidate = coordinator.candidate().filter(|candidate| {
            candidate.source == BeatDragSource::BeatBoard
        })?;
        if candidate.terminal.is_some() {
            self.planner_preview = None;
            self.external_geometry = None;
            return Some(BeatInteractionOutput::default());
        }
        let context_compatible = Arc::ptr_eq(&candidate.document_identity, document_identity)
            && candidate.mutation_token == mutation_token;
        if !context_compatible {
            self.planner_preview = None;
            self.external_geometry = None;
            coordinator.invalidate_context(BeatDragSource::BeatBoard, document_identity, mutation_token);
            if candidate.externally_engaged {
                coordinator.claim_invalid(candidate.generation);
            }
            return Some(BeatInteractionOutput::default());
        }
        if !project.beat_board().cards.iter().any(|card| card.id == candidate.card_id) {
            self.planner_preview = None;
            self.external_geometry = None;
            coordinator.cancel_candidate(candidate.generation, BeatDragSource::BeatBoard);
            return Some(BeatInteractionOutput::default());
        }
        if escape && enabled {
            ui.ctx().input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
        }
        let Some(pointer) = pointer.filter(|point| finite_pos(*point)) else {
            self.planner_preview = None;
            self.external_geometry = None;
            if candidate.externally_engaged {
                coordinator.claim_invalid(candidate.generation);
            } else {
                coordinator.cancel_candidate(candidate.generation, BeatDragSource::BeatBoard);
            }
            let _ = primary_down;
            return Some(BeatInteractionOutput::default());
        };
        coordinator.update_pointer(candidate.generation, pointer);
        if escape || pointer_gone || !focused || !enabled {
            self.planner_preview = None;
            self.external_geometry = None;
            if candidate.externally_engaged {
                coordinator.claim_invalid(candidate.generation);
            } else {
                coordinator.cancel_candidate(candidate.generation, BeatDragSource::BeatBoard);
            }
            let _ = primary_down;
            return Some(BeatInteractionOutput::default());
        }
        if !candidate.externally_engaged {
            if !strip.rect.contains(pointer) {
                return None;
            }
            if !coordinator.engage_external(candidate.generation) {
                return Some(BeatInteractionOutput::default());
            }
        }
        self.external_geometry = Some((candidate.generation, *timeline, strip.lane_rows.clone()));

        let card = project.beat_board().cards.iter().find(|card| card.id == candidate.card_id)?;
        let story_map = project.story_map();
        let preview = (pointer.x >= strip.timeline_viewport.left()
            && pointer.x <= strip.timeline_viewport.right())
            .then(|| strip.lane_rows.iter().position(|row| {
            pointer.y >= row.top() && pointer.y < row.bottom()
        })).flatten().and_then(|lane_index| {
            let lane = story_map.lanes.get(lane_index)?;
            let target_eighths = u64::from(story_map.target_pages).checked_mul(8)?;
            let length = card.story_map_placement.as_ref()
                .map(|placement| placement.length_eighths)
                .unwrap_or_else(|| target_eighths.min(8));
            let maximum_start = target_eighths.checked_sub(length)?;
            let page = timeline.screen_to_page(f64::from(pointer.x))?;
            if !page.is_finite() { return None; }
            let maximum_start_page = maximum_start as f64 / 8.0;
            let snapped = snap_nonnegative_eighths(page.clamp(0.0, maximum_start_page))?;
            Some(BeatPlacementPreview {
                lane_uuid: lane.uuid.clone(),
                start_eighths: snapped.min(maximum_start),
                length_eighths: length,
            })
        });
        coordinator.set_preview(candidate.generation, preview.clone());

        let mut output = if let Some(preview) = preview.as_ref() {
            let request = Candidate {
                lane_uuid: preview.lane_uuid.clone(),
                start_eighths: preview.start_eighths,
                length_eighths: preview.length_eighths,
            };
            self.planner_preview(
                project,
                candidate.generation,
                &candidate.card_id,
                BeatDragSource::BeatBoard,
                Arc::clone(document_identity),
                mutation_token,
                &request,
            )
        } else {
            self.planner_preview = None;
            self.external_geometry = None;
            BeatInteractionOutput::default()
        };
        if released {
            let claimed = preview.as_ref().and_then(|preview| {
                let command = make_command(&candidate.card_id, Candidate {
                    lane_uuid: preview.lane_uuid.clone(),
                    start_eighths: preview.start_eighths,
                    length_eighths: preview.length_eighths,
                })?;
                coordinator.claim_valid(candidate.generation, preview.clone(), command).then_some(())
            }).is_some();
            if !claimed {
                coordinator.claim_invalid(candidate.generation);
            }
            self.planner_preview = None;
            self.external_geometry = None;
            output = BeatInteractionOutput::default();
            ui.ctx().request_repaint();
        }
        Some(output)
    }

    fn planner_preview(
        &mut self,
        project: &Project,
        generation: u64,
        card_id: &StableId,
        source: BeatDragSource,
        document_identity: Arc<()>,
        mutation_token: u64,
        request: &Candidate,
    ) -> BeatInteractionOutput {
        let story_map = project.story_map();
        let cards = &project.beat_board().cards;
        let Some(primary) = cards.iter().find(|card| &card.id == card_id) else {
            self.planner_preview = None;
            return BeatInteractionOutput::default();
        };
        let key = PreviewKey {
            generation,
            card_id: card_id.clone(),
            source,
            document_identity,
            mutation_token,
            primary_original: primary.story_map_placement.clone(),
            target_pages: story_map.target_pages,
            board_identity: project.beat_board().foreign_id.clone(),
            story_map_identity: story_map.foreign_id.clone(),
            lane_ids: story_map.lanes.iter().map(|lane| lane.uuid.clone()).collect(),
            card_semantics: cards.iter().map(|card| (
                card.id.clone(), card.foreign_id.clone(), card.story_map_placement.clone(),
            )).collect(),
            request: request.clone(),
        };
        let cached = self.planner_preview.as_ref()
            .filter(|cached| cached.key.matches(&key))
            .map(|cached| cached.plan.clone());
        let plan = cached.unwrap_or_else(|| {
            let request = StoryMapPlacementRequest {
                card: card_id.clone(),
                lane_uuid: request.lane_uuid.clone(),
                start_eighths: request.start_eighths,
                length_eighths: request.length_eighths,
            };
            match plan_story_map_placement(project, &request) {
                Ok(plan) if plan.affected.is_empty() => CachedPlan::NoOp,
                Ok(plan) => {
                    let mut ghosts = plan.affected.into_iter().filter_map(|affected| {
                        let placement = affected.final_placement?;
                        let role = match affected.change {
                            StoryMapPlacementChangeKind::Primary => BeatGroupRole::Primary,
                            StoryMapPlacementChangeKind::InitiallyCarriedFollower => BeatGroupRole::CarriedFollower,
                            StoryMapPlacementChangeKind::CollisionDisplacement => BeatGroupRole::Displaced,
                            StoryMapPlacementChangeKind::DisplacementTriggeredFollower => BeatGroupRole::PropagatedFollower,
                        };
                        Some(BeatGroupGhost {
                            card_id: affected.card,
                            lane_uuid: placement.lane_uuid,
                            start_eighths: placement.start_eighths,
                            length_eighths: placement.length_eighths,
                            role,
                        })
                    }).collect::<Vec<_>>();
                    ghosts.sort_by(|left, right| {
                        let left_lane = story_map.lanes.iter().position(|lane| lane.uuid == left.lane_uuid).unwrap_or(usize::MAX);
                        let right_lane = story_map.lanes.iter().position(|lane| lane.uuid == right.lane_uuid).unwrap_or(usize::MAX);
                        (left_lane, left.start_eighths, left.card_id.as_str())
                            .cmp(&(right_lane, right.start_eighths, right.card_id.as_str()))
                    });
                    CachedPlan::Valid(ghosts)
                }
                Err(StoryMapPlacementPlanError::Rejected(_)) => CachedPlan::Rejected,
                Err(StoryMapPlacementPlanError::Failed(_)) => CachedPlan::Failed,
            }
        });
        self.planner_preview = Some(PlannerPreviewCache { key, plan: plan.clone() });
        match plan {
            CachedPlan::Valid(ghosts) => BeatInteractionOutput {
                suppressed_cards: ghosts.iter().map(|ghost| ghost.card_id.clone()).collect(),
                ghosts,
            },
            CachedPlan::NoOp | CachedPlan::Failed => BeatInteractionOutput::default(),
            CachedPlan::Rejected => {
                let target_eighths = u64::from(story_map.target_pages).checked_mul(8);
                let legal = target_eighths.is_some_and(|target| {
                    request.length_eighths > 0
                        && request.start_eighths.checked_add(request.length_eighths)
                            .is_some_and(|end| end <= target)
                }) && story_map.lanes.iter().filter(|lane| lane.uuid == request.lane_uuid).count() == 1;
                if legal {
                    BeatInteractionOutput {
                        suppressed_cards: vec![card_id.clone()],
                        ghosts: vec![BeatGroupGhost {
                            card_id: card_id.clone(),
                            lane_uuid: request.lane_uuid.clone(),
                            start_eighths: request.start_eighths,
                            length_eighths: request.length_eighths,
                            role: BeatGroupRole::Rejected,
                        }],
                    }
                } else {
                    BeatInteractionOutput::default()
                }
            }
        }
    }

    fn register_captured_target(
        &self,
        ui: &mut egui::Ui,
        project: &Project,
        strip: &StripGeometry,
        timeline: &TimelineGeometry,
        press: &Press,
    ) {
        let lanes = &project.story_map().lanes;
        let Some(lane_index) = lanes.iter().position(|lane| lane.uuid == press.source_lane_uuid) else {
            return;
        };
        let clip = strip
            .timeline_viewport
            .intersect(strip.rect)
            .intersect(ui.clip_rect());
        let Some(rect) = placed_beat_rect(
            strip,
            lane_index,
            press.original_start,
            press.original_length,
            press.target_pages,
            timeline,
            clip,
        ) else { return };
        let _ = ui.interact(
            rect,
            ui.id().with(("story-map-beat-placement", press.card_id.as_str())),
            Sense::click_and_drag(),
        );
    }

    fn cancel_capture(&mut self, primary_down: bool, coordinator: &mut BeatDragCoordinator) {
        self.planner_preview = None;
        self.external_geometry = None;
        if let Some(press) = self.press.take() {
            coordinator.cancel_generation(press.generation, BeatDragSource::StoryMap);
        }
        self.swallow_until_release = primary_down;
    }

    fn finish_swallow(&mut self, released: bool, primary_down: bool) {
        if released || !primary_down {
            self.swallow_until_release = false;
        }
    }
}

fn unique_card<'a>(project: &'a Project, card_id: &StableId) -> Option<&'a scriptcraft_model::BeatCard> {
    let mut cards = project.beat_board().cards.iter().filter(|card| &card.id == card_id);
    let card = cards.next()?;
    (cards.next().is_none() && !card.foreign_id.is_empty()).then_some(card)
}

fn durable_card_correspondence(project: &Project, snapshot: &StoryMapSaveCompatibility) -> bool {
    let mut matches = project.beat_board().cards.iter()
        .filter(|card| card.foreign_id == snapshot.durable_card_id);
    let Some(card) = matches.next() else { return false };
    card.id == snapshot.card_id
        && card.story_map_placement == snapshot.original
        && matches.next().is_none()
}

fn lane_resolves_once(project: &Project, lane_uuid: &str) -> bool {
    project.story_map().lanes.iter().filter(|lane| lane.uuid == lane_uuid).count() == 1
}

fn update_candidate(
    press: &mut Press,
    pointer: Pos2,
    strip: &StripGeometry,
    lanes: &[StoryMapLane],
    timeline: &TimelineGeometry,
) {
    let Some(pointer_page) = timeline.screen_to_page(f64::from(pointer.x)) else {
        press.candidate = None;
        return;
    };
    match press.mode {
        DragMode::Move => {
            if pointer.x < strip.timeline_viewport.left() {
                press.candidate = None;
                return;
            }
            let Some(lane_index) = strip.lane_rows.iter().position(|row| {
                pointer.y >= row.top() && pointer.y < row.bottom()
            }) else {
                press.candidate = None;
                return;
            };
            let Some(lane) = lanes.get(lane_index) else { press.candidate = None; return; };
            let Some(target_eighths) = u64::from(press.target_pages).checked_mul(8) else {
                press.candidate = None;
                return;
            };
            let Some(maximum_start) = target_eighths.checked_sub(press.original_length) else {
                press.candidate = None;
                return;
            };
            let maximum_start_page = maximum_start as f64 / 8.0;
            let requested_page = pointer_page - press.pointer_to_anchor_page;
            if !requested_page.is_finite() { press.candidate = None; return; }
            let clamped = requested_page.clamp(0.0, maximum_start_page);
            let Some(start) = snap_nonnegative_eighths(clamped) else { press.candidate = None; return; };
            press.candidate = Some(Candidate {
                lane_uuid: lane.uuid.clone(),
                start_eighths: start.min(maximum_start),
                length_eighths: press.original_length,
            });
        }
        DragMode::TailResize => {
            let Some(target_eighths) = u64::from(press.target_pages).checked_mul(8) else {
                press.candidate = None;
                return;
            };
            let Some(maximum_length) = target_eighths.checked_sub(press.original_start) else {
                press.candidate = None;
                return;
            };
            let requested_end = pointer_page - press.pointer_to_anchor_page;
            let requested_length = requested_end - press.original_start as f64 / 8.0;
            if !requested_length.is_finite() { press.candidate = None; return; }
            let clamped = requested_length.clamp(0.125, maximum_length as f64 / 8.0);
            let Some(length) = snap_nonnegative_eighths(clamped) else { press.candidate = None; return; };
            press.candidate = Some(Candidate {
                lane_uuid: press.source_lane_uuid.clone(),
                start_eighths: press.original_start,
                length_eighths: length.max(1).min(maximum_length),
            });
        }
    }
}

fn snap_nonnegative_eighths(page_position: f64) -> Option<u64> {
    if !page_position.is_finite() || page_position < 0.0 { return None; }
    let scaled = page_position * 8.0;
    if !scaled.is_finite() || scaled < 0.0 || scaled >= (u64::MAX as f64) { return None; }
    let snapped = (scaled + 0.5).floor();
    if !snapped.is_finite() || snapped < 0.0 || snapped >= (u64::MAX as f64) { return None; }
    Some(snapped as u64)
}

fn make_command(card_id: &StableId, candidate: Candidate) -> Option<Command> {
    let card = BeatCardIdValue::from_core(card_id.clone());
    let lane_uuid = StoryMapLaneIdValue::from_core(&candidate.lane_uuid).ok()?;
    let start_eighths = u32::try_from(candidate.start_eighths).ok().and_then(StoryMapStartEighthsValue::new)?;
    let length_eighths = u32::try_from(candidate.length_eighths).ok().and_then(StoryMapLengthEighthsValue::new)?;
    Some(Command::StoryMapBeatPlacementSet { card, lane_uuid, start_eighths, length_eighths })
}

fn finite_pos(position: Pos2) -> bool {
    position.x.is_finite() && position.y.is_finite()
}
