mod constants;
pub(crate) mod context_menu;
pub(crate) mod geometry;
mod interaction;
mod paint;
mod pointer;
mod preview;
pub(crate) mod projection;
mod ruler;
mod scrollbar;
mod script_track;
mod style;
pub(crate) mod target_dialog;
mod viewport;

use eframe::egui::{self, Pos2, Sense, Vec2};
use scriptcraft_commands::EditorSurface;
use scriptcraft_layout::{project_story_map, StoryMapProjectionError};
use std::time::Instant;

use crate::app::ScriptCraftApp;
use crate::chrome::theme::ApplicationPalette;
use crate::views::story_map::constants::{MIN_STRIP_HEIGHT, STORY_MAP_SCROLLBAR_ROW_HEIGHT};

#[derive(Debug, Default)]
pub(crate) struct StoryMapResizeState {
    active: bool,
    start_y: f32,
    start_height: u32,
}

#[derive(Debug, Default)]
pub(crate) struct StoryMapInteraction {
    pub(crate) scene_clicked: bool,
    pub(crate) pointer_on_scene_press: bool,
}

impl StoryMapResizeState {
    pub(crate) fn reset(&mut self) {
        self.active = false;
        self.start_y = 0.0;
        self.start_height = 0;
    }
}

#[derive(Debug, Default)]
pub(crate) struct StoryMapNavigationState {
    pub(crate) scrollbars: scrollbar::StoryMapScrollbarState,
}

pub(crate) use interaction::StoryMapBeatInteractionState;

pub(crate) use projection::StoryMapProjectionState;
pub(crate) use preview::StoryMapPreviewState;

impl StoryMapNavigationState {
    pub(crate) fn reset(&mut self) {
        self.scrollbars.reset();
    }
}

impl ScriptCraftApp {
    pub(crate) fn show_story_map(&mut self, ui: &mut egui::Ui) -> StoryMapInteraction {
        if self.root_document_view != crate::app::RootDocumentView::BeatBoard {
            if self.beat_drag_coordinator.active_for(crate::views::beat_drag::BeatDragSource::BeatBoard).is_some() {
                self.beat_interaction.cancel();
            }
            self.beat_drag_coordinator.source_removed(crate::views::beat_drag::BeatDragSource::BeatBoard);
        }
        let visible = self.app_command_projection().story_map_visible;
        self.story_map_projection.observe(
            self.session.project(),
            self.session.layout_inputs(),
            self.session.selection().focus,
            self.active_body_dual_dialogue_group.clone(),
            self.session.revision(),
            self.session.mutation_token(),
            visible,
            Instant::now(),
            constants::PROJECTION_QUIET_DELAY,
        );
        if !visible {
            self.story_map_context_menu.close();
            self.beat_drag_coordinator.cancel_external_if_hidden();
            self.story_map_resize.reset();
            self.story_map_navigation.reset();
            let primary_down = ui.ctx().input(|input| input.pointer.primary_down());
            self.story_map_beat_interaction.cancel_local_capture(primary_down, &mut self.beat_drag_coordinator);
            self.story_map_preview.close();
            return StoryMapInteraction::default();
        }

        let available = ui.available_size();
        if !available.x.is_finite()
            || !available.y.is_finite()
            || available.x <= 0.0
            || available.y <= 0.0
        {
            self.story_map_context_menu.close();
            self.story_map_resize.reset();
            self.story_map_navigation.reset();
            let primary_down = ui.ctx().input(|input| input.pointer.primary_down());
            self.story_map_beat_interaction.cancel_for_unavailable_geometry(primary_down, &mut self.beat_drag_coordinator);
            self.schedule_story_map_projection_repaint(ui);
            self.story_map_preview.close();
            return StoryMapInteraction::default();
        }

        if available.x > constants::LABEL_COLUMN_WIDTH && available.y >= MIN_STRIP_HEIGHT {
            let current = self.story_map_view_state;
            if let Some(height) = geometry::clamp_height(current.height() as f32, available.y) {
                if height != current.height() {
                    if let Some(next) = scriptcraft_fdx::StoryMapViewState::new(
                        height,
                        current.width(),
                        current.scroll_origin(),
                        current.zoom_milli_percent(),
                    ) {
                        self.story_map_view_state = next;
                        ui.ctx().request_repaint();
                    }
                }
            }
        }

        let strip_height = self.story_map_view_state.height() as f32;
        if !strip_height.is_finite() || strip_height <= 0.0 {
            self.story_map_context_menu.close();
            self.story_map_resize.reset();
            self.story_map_preview.close();
            let primary_down = ui.ctx().input(|input| input.pointer.primary_down());
            self.story_map_beat_interaction.cancel_for_unavailable_geometry(primary_down, &mut self.beat_drag_coordinator);
            return StoryMapInteraction::default();
        }
        let (strip_rect, _) = ui.allocate_exact_size(
            Vec2::new(available.x, strip_height),
            Sense::hover(),
        );
        let lane_count = self.session.project().story_map().lanes.len();
        let Some(geometry) = geometry::allocate_strip(
            strip_rect,
            lane_count,
            STORY_MAP_SCROLLBAR_ROW_HEIGHT,
        ) else {
            self.story_map_context_menu.close();
            self.story_map_resize.reset();
            self.story_map_navigation.reset();
            let primary_down = ui.ctx().input(|input| input.pointer.primary_down());
            self.story_map_beat_interaction.cancel_for_unavailable_geometry(primary_down, &mut self.beat_drag_coordinator);
            self.schedule_story_map_projection_repaint(ui);
            self.story_map_preview.close();
            return StoryMapInteraction::default();
        };

        let (secondary_released, pointer_position) = ui.ctx().input(|input| {
            (
                input.pointer.button_released(egui::PointerButton::Secondary),
                input.pointer.interact_pos(),
            )
        });
        if secondary_released
            && pointer_position.is_some_and(|point| geometry.rect.contains(point))
            && !self.story_map_preview_input_blocked(ui.ctx())
        {
            if let Some(anchor) = pointer_position {
                self.story_map_context_menu.open_at(anchor);
                self.story_map_preview.close();
                ui.ctx().request_repaint();
            }
        }

        self.refresh_story_map_projection(ui);

        let interaction_area_usable = available.y >= MIN_STRIP_HEIGHT
            && geometry.timeline_viewport.width().is_finite()
            && geometry.timeline_viewport.width() > 0.0;
        if !interaction_area_usable {
            self.story_map_resize.reset();
            self.story_map_navigation.reset();
        }

        let mut resize_response = None;
        if interaction_area_usable && geometry.resize_band.is_positive() {
            let resize_cursor = if self.story_map_view_state.height() as f32 <= MIN_STRIP_HEIGHT {
                egui::CursorIcon::ResizeSouth
            } else {
                egui::CursorIcon::ResizeVertical
            };
            let response = ui
                .interact(
                    geometry.resize_band,
                    ui.id().with("story-map-bottom-resize"),
                    Sense::drag(),
                )
                .on_hover_cursor(resize_cursor);
            resize_response = Some(response);
        }

        let focused = ui
            .ctx()
            .input(|input| input.viewport().focused != Some(false));
        let pointer_down = ui.ctx().input(|input| input.pointer.primary_down());
        let pointer_pos = ui.ctx().input(|input| input.pointer.interact_pos());
        if let Some(response) = resize_response.as_ref() {
            if response.drag_started() {
                if let Some(pos) = pointer_pos.filter(|pos| finite_pos(*pos)) {
                    self.story_map_resize.active = true;
                    self.story_map_resize.start_y = pos.y;
                    self.story_map_resize.start_height = self.story_map_view_state.height();
                }
            }
            if self.story_map_resize.active && !focused {
                self.story_map_resize.reset();
            }
            if self.story_map_resize.active {
                if let Some(pos) = pointer_pos.filter(|pos| finite_pos(*pos)) {
                    let requested = self.story_map_resize.start_height as f32
                        + (pos.y - self.story_map_resize.start_y);
                    if let Some(height) = geometry::clamp_height(requested, available.y) {
                        let current = self.story_map_view_state;
                        if height != current.height() {
                            if let Some(next) = scriptcraft_fdx::StoryMapViewState::new(
                                height,
                                current.width(),
                                current.scroll_origin(),
                                current.zoom_milli_percent(),
                            ) {
                                self.story_map_view_state = next;
                                ui.ctx().request_repaint();
                            }
                        }
                    } else {
                        self.story_map_resize.reset();
                    }
                } else {
                    self.story_map_resize.reset();
                }
                if self.story_map_resize.active
                    && (response.drag_stopped() || !pointer_down)
                {
                    self.story_map_resize.reset();
                }
            }
        } else {
            self.story_map_resize.reset();
        }

        let palette: ApplicationPalette = self.theme_palette;
        paint::paint_shell(
            ui,
            &geometry,
            self.session.project(),
            self.prepared_document_fonts.as_ref(),
            palette,
        );
        if let Some(response) = paint::paint_target_length_button(
            ui,
            &geometry,
            palette.story_map,
        ) {
            if response.clicked() && !self.story_map_preview_input_blocked(ui.ctx()) {
                let _ = target_dialog::open(self, ui.ctx(), response.rect.center());
            }
        }
        let mut scene_clicked = false;
        let mut pointer_on_scene_press = false;
        if interaction_area_usable {
            let target_pages = f64::from(self.session.project().story_map().target_pages);
            let domain_pages = self
                .story_map_projection
                .displayed_domain_pages(self.session.project().story_map().target_pages);
            let shared_timeline = viewport::compose_navigation(
                ui,
                &geometry,
                target_pages,
                domain_pages,
                &mut self.story_map_view_state,
                &mut self.story_map_navigation.scrollbars,
                palette,
            );
            if let Some((timeline, _)) = shared_timeline {
                let pixels_per_point = f64::from(ui.ctx().pixels_per_point());
                ruler::paint(
                    ui,
                    &geometry,
                    &timeline,
                    pixels_per_point,
                    palette.story_map,
                );
                paint::paint_dynamic_lane_grid(
                    ui,
                    &geometry,
                    &timeline,
                    pixels_per_point,
                    palette.story_map,
                );
                let project = self.session.project();
                let story_map = project.story_map();
                let beat_enabled = !self.story_map_preview.is_open()
                    && !self.story_map_preview_input_blocked(ui.ctx())
                    && !self.story_map_resize.active;
                let beat_interaction = self.story_map_beat_interaction.process(
                    ui,
                    project,
                    project.layout_identity().clone(),
                    self.session.mutation_token(),
                    &geometry,
                    &timeline,
                    beat_enabled,
                    &mut self.beat_drag_coordinator,
                );
                paint::paint_static_beats(
                    ui,
                    &geometry,
                    project,
                    &story_map.lanes,
                    &project.beat_board().cards,
                    story_map.target_pages,
                    &timeline,
                    palette.story_map,
                    palette.script_workspace.page,
                    self.prepared_document_fonts.as_ref(),
                    &beat_interaction.suppressed_cards,
                );
                paint::paint_group_preview(
                    ui,
                    &geometry,
                    project,
                    &story_map.lanes,
                    &project.beat_board().cards,
                    &beat_interaction,
                    story_map.target_pages,
                    &timeline,
                    palette.story_map,
                    palette.script_workspace.page,
                    self.prepared_document_fonts.as_ref(),
                );
                script_track::paint(
                    ui,
                    &geometry,
                    &timeline,
                    pixels_per_point,
                    palette.story_map,
                    palette.script_workspace.page,
                    project,
                    self.story_map_projection.snapshot(),
                    self.prepared_document_fonts.as_ref(),
                );
                let snapshot = self.story_map_projection.snapshot();
                if let Some(snapshot) = snapshot {
                    let (primary_pressed, pointer_position) = ui.ctx().input(|input| {
                        (input.pointer.primary_pressed(), input.pointer.interact_pos())
                    });
                    pointer_on_scene_press = primary_pressed
                        && pointer_position.is_some_and(|point| {
                            pointer::pointer_over_scene(
                                ui,
                                &geometry,
                                &timeline,
                                snapshot,
                                point,
                            )
                        });
                    if let Some(click) = pointer::primary_scene_click(
                        ui,
                        &geometry,
                        &timeline,
                        snapshot,
                        !self.story_map_preview_input_blocked(ui.ctx()),
                    ) {
                        self.story_map_preview.open_scene(
                            snapshot.document_identity.clone(),
                            click.element_index,
                            click.owner_id,
                            click.anchor,
                        );
                        scene_clicked = true;
                    }
                }
                let project = self.session.project();
                let snapshot = self.story_map_projection.snapshot();
                let _ = self.story_map_preview.resolve(project, snapshot, |scene| {
                    pointer::current_anchor(ui, &geometry, &timeline, scene)
                });
            } else {
                let primary_down = ui.ctx().input(|input| input.pointer.primary_down());
                self.story_map_beat_interaction.cancel_for_unavailable_geometry(primary_down, &mut self.beat_drag_coordinator);
                self.story_map_preview.close();
            }
        } else {
            let primary_down = ui.ctx().input(|input| input.pointer.primary_down());
            self.story_map_beat_interaction.cancel_for_unavailable_geometry(primary_down, &mut self.beat_drag_coordinator);
            self.story_map_preview.close();
        }
        paint::paint_lane_foreground_and_divider(
            ui,
            &geometry,
            f64::from(ui.ctx().pixels_per_point()),
            palette.story_map,
        );
        StoryMapInteraction { scene_clicked, pointer_on_scene_press }
    }

    pub(crate) fn story_map_preview_input_blocked(&self, ctx: &egui::Context) -> bool {
        self.element_windows.owns_input()
            || self.pending_fdx_open.is_some()
            || self.replacement_open_pending()
            || self.native_output_modal_active
            || self.title_view.surface_focused
            || self.title_page_window_open
            || self.script_note_workspace.open_note.is_some()
            || self.script_note_workspace.new_type_dialog_open
            || self.story_map_target_dialog.owns_input()
            || self.story_map_context_menu.open
            || self.sprint_timer.popup_open
            || (self.menu_policy.in_window_shortcuts_owned()
                && crate::chrome::menu::in_window_menu_popup_open(ctx))
            || ctx.input(|input| input.viewport().focused == Some(false))
    }

    fn refresh_story_map_projection(&mut self, ui: &egui::Ui) {
        let now = Instant::now();
        if self.story_map_projection.begin_due_attempt(now) {
            match self.layout_for_render(EditorSurface::Body) {
                Err(_) => {
                    self.story_map_projection.complete_failure();
                    self.show_error(
                        "STORY_MAP_PROJECTION_LAYOUT_UNAVAILABLE: prior projection retained",
                    );
                }
                Ok(layout) => {
                    let focus = self.session.selection().focus;
                    match project_story_map(self.session.project(), focus, &layout) {
                        Err(StoryMapProjectionError::MissingSceneLine) => {
                            self.story_map_projection.complete_failure();
                            self.show_error(
                                "STORY_MAP_PROJECTION_SCENE_LINE_MISSING: prior projection retained",
                            );
                        }
                        Err(StoryMapProjectionError::InvalidLayout)
                        | Err(StoryMapProjectionError::InvalidGeometry) => {
                            self.story_map_projection.complete_failure();
                            self.show_error(
                                "STORY_MAP_PROJECTION_GEOMETRY_INVALID: prior projection retained",
                            );
                        }
                        Ok(snapshot) => {
                            let inputs = projection::ProjectionInputs::capture(
                                self.session.project(),
                                self.session.layout_inputs(),
                                focus,
                                self.active_body_dual_dialogue_group.clone(),
                            );
                            let installed = self.story_map_projection.complete_success(
                                snapshot,
                                inputs,
                                self.session.revision(),
                                self.session.mutation_token(),
                            );
                            if !installed {
                                self.story_map_projection.complete_failure();
                                self.show_error(
                                    "STORY_MAP_PROJECTION_IDENTITY_CHANGED: prior projection retained",
                                );
                            }
                        }
                    }
                }
            }
        }
        self.schedule_story_map_projection_repaint(ui);
    }

    fn schedule_story_map_projection_repaint(&self, ui: &egui::Ui) {
        if let Some(delay) = self
            .story_map_projection
            .remaining_delay(Instant::now())
        {
            ui.ctx().request_repaint_after(delay);
        }
    }
}

fn finite_pos(pos: Pos2) -> bool {
    pos.x.is_finite() && pos.y.is_finite()
}
