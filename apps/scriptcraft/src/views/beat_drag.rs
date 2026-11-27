//! App-private transient handoff between Beat Board and Story Map gestures.

use std::sync::Arc;

use eframe::egui::Pos2;
use scriptcraft_commands::Command;
use scriptcraft_model::StableId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BeatDragSource {
    BeatBoard,
    StoryMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BeatPlacementPreview {
    pub(crate) lane_uuid: String,
    pub(crate) start_eighths: u64,
    pub(crate) length_eighths: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct BeatDragCandidate {
    pub(crate) generation: u64,
    pub(crate) card_id: StableId,
    pub(crate) source: BeatDragSource,
    pub(crate) document_identity: Arc<()>,
    pub(crate) mutation_token: u64,
    pub(crate) press_origin: Pos2,
    pub(crate) current_pointer: Pos2,
    pub(crate) preview: Option<BeatPlacementPreview>,
    pub(crate) externally_engaged: bool,
    pub(crate) terminal: Option<BeatDragTerminal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BeatDragTerminal {
    ValidDrop,
    InvalidOrCancelled,
}

struct Generation {
    candidate: BeatDragCandidate,
    candidate_published: bool,
    command_intent: Option<Command>,
    intent_consumed: bool,
    source_acknowledged: bool,
}

#[derive(Default)]
pub(crate) struct BeatDragCoordinator {
    next_generation: u64,
    active: Option<Generation>,
}

impl BeatDragCoordinator {
    pub(crate) fn begin(
        &mut self,
        source: BeatDragSource,
        card_id: StableId,
        document_identity: Arc<()>,
        mutation_token: u64,
        press_origin: Pos2,
        current_pointer: Pos2,
    ) -> Option<u64> {
        self.clear_if_complete();
        if self.active.is_some() || !finite_pos(press_origin) || !finite_pos(current_pointer) {
            return None;
        }
        let generation = self.next_generation.checked_add(1)?;
        self.next_generation = generation;
        self.active = Some(Generation {
            candidate: BeatDragCandidate {
                generation,
                card_id,
                source,
                document_identity,
                mutation_token,
                press_origin,
                current_pointer,
                preview: None,
                externally_engaged: false,
                terminal: None,
            },
            candidate_published: false,
            command_intent: None,
            intent_consumed: false,
            source_acknowledged: false,
        });
        Some(generation)
    }

    pub(crate) fn update_pointer(&mut self, generation: u64, pointer: Pos2) {
        if !finite_pos(pointer) {
            return;
        }
        if let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation && active.candidate.terminal.is_none()
        }) {
            active.candidate.current_pointer = pointer;
        }
    }

    /// Refresh document/token baselines only for a fully audited successful
    /// save rebase. It never changes a terminal or reconstructs command intent.
    pub(crate) fn rebase_context(
        &mut self,
        generation: u64,
        source: BeatDragSource,
        document_identity: Arc<()>,
        mutation_token: u64,
    ) -> bool {
        let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation
                && active.candidate.source == source
                && active.candidate.terminal.is_none()
                && active.command_intent.is_none()
        }) else {
            return false;
        };
        active.candidate.document_identity = document_identity;
        active.candidate.mutation_token = mutation_token;
        true
    }

    pub(crate) fn publish_candidate(&mut self, generation: u64, pointer: Pos2) {
        self.update_pointer(generation, pointer);
        if let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation
                && active.candidate.terminal.is_none()
        }) {
            active.candidate_published = true;
        }
    }

    pub(crate) fn candidate(&self) -> Option<BeatDragCandidate> {
        self.active
            .as_ref()
            .filter(|active| active.candidate_published)
            .map(|active| active.candidate.clone())
    }

    pub(crate) fn active_for(&self, source: BeatDragSource) -> Option<BeatDragCandidate> {
        self.active
            .as_ref()
            .filter(|active| active.candidate.source == source)
            .map(|active| active.candidate.clone())
    }

    pub(crate) fn set_preview(
        &mut self,
        generation: u64,
        preview: Option<BeatPlacementPreview>,
    ) {
        if let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation && active.candidate.terminal.is_none()
        }) {
            active.candidate.preview = preview;
        }
    }

    pub(crate) fn engage_external(&mut self, generation: u64) -> bool {
        let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation
                && active.candidate.source == BeatDragSource::BeatBoard
                && active.candidate_published
                && active.candidate.terminal.is_none()
        }) else {
            return false;
        };
        active.candidate.externally_engaged = true;
        true
    }

    pub(crate) fn claim_valid(
        &mut self,
        generation: u64,
        preview: BeatPlacementPreview,
        command: Command,
    ) -> bool {
        let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation
                && active.candidate.terminal.is_none()
        }) else {
            return false;
        };
        active.candidate.preview = Some(preview);
        active.candidate.terminal = Some(BeatDragTerminal::ValidDrop);
        active.command_intent = Some(command);
        true
    }

    pub(crate) fn claim_invalid(&mut self, generation: u64) -> bool {
        let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation
                && active.candidate.terminal.is_none()
        }) else {
            return false;
        };
        active.candidate.preview = None;
        active.candidate.terminal = Some(BeatDragTerminal::InvalidOrCancelled);
        true
    }

    pub(crate) fn cancel_candidate(&mut self, generation: u64, source: BeatDragSource) {
        if let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation
                && active.candidate.source == source
                && active.candidate_published
                && active.candidate.terminal.is_none()
        }) {
            active.candidate.preview = None;
            active.candidate.terminal = Some(BeatDragTerminal::InvalidOrCancelled);
        }
    }

    pub(crate) fn cancel_generation(&mut self, generation: u64, source: BeatDragSource) {
        let Some(externally_engaged) = self.active.as_ref().filter(|active| {
            active.candidate.generation == generation
                && active.candidate.source == source
                && active.candidate.terminal.is_none()
        }).map(|active| active.candidate.externally_engaged) else {
            return;
        };
        if externally_engaged {
            let Some(active) = self.active.as_mut() else { return };
            active.candidate.preview = None;
            active.candidate.terminal = Some(BeatDragTerminal::InvalidOrCancelled);
        } else {
            self.active = None;
        }
    }

    pub(crate) fn invalidate_context(
        &mut self,
        source: BeatDragSource,
        document_identity: &Arc<()>,
        mutation_token: u64,
    ) {
        let Some((generation, compatible, published, engaged)) = self.active.as_ref().filter(|active| {
            active.candidate.source == source && active.candidate.terminal.is_none()
        }).map(|active| (
            active.candidate.generation,
            Arc::ptr_eq(&active.candidate.document_identity, document_identity)
                && active.candidate.mutation_token == mutation_token,
            active.candidate_published,
            active.candidate.externally_engaged,
        )) else {
            return;
        };
        if !compatible {
            if published || engaged {
                self.cancel_candidate(generation, source);
            } else {
                self.cancel_generation(generation, source);
            }
        }
    }

    pub(crate) fn source_terminal_for_board(&self) -> Option<(u64, BeatDragTerminal)> {
        self.active.as_ref().and_then(|active| {
            (active.candidate.source == BeatDragSource::BeatBoard)
                .then_some(active.candidate.terminal)
                .flatten()
                .map(|terminal| (active.candidate.generation, terminal))
        })
    }

    pub(crate) fn external_engaged(&self) -> bool {
        self.active.as_ref().is_some_and(|active| {
            active.candidate.source == BeatDragSource::BeatBoard
                && active.candidate.externally_engaged
        })
    }

    pub(crate) fn finish_unclaimed(&mut self, generation: u64, source: BeatDragSource) {
        if self.active.as_ref().is_some_and(|active| {
            active.candidate.generation == generation
                && active.candidate.source == source
                && !active.candidate.externally_engaged
                && active.candidate.terminal.is_none()
        }) {
            self.active = None;
        }
    }

    pub(crate) fn acknowledge_source(&mut self, generation: u64, source: BeatDragSource) {
        if let Some(active) = self.active.as_mut().filter(|active| {
            active.candidate.generation == generation && active.candidate.source == source
        }) {
            active.source_acknowledged = true;
        }
        self.clear_if_complete();
    }

    pub(crate) fn take_command_intent(&mut self) -> Option<Command> {
        let active = self.active.as_mut()?;
        let command = active.command_intent.take()?;
        active.intent_consumed = true;
        self.clear_if_complete();
        Some(command)
    }

    pub(crate) fn source_removed(&mut self, source: BeatDragSource) {
        let Some((generation, terminal)) = self.active.as_ref().filter(|active| {
            active.candidate.source == source
        }).map(|active| (active.candidate.generation, active.candidate.terminal)) else {
            return;
        };
        if terminal.is_none() {
            self.cancel_generation(generation, source);
        }
        if self.active.as_ref().is_some_and(|active| {
            active.candidate.generation == generation
                && active.candidate.terminal.is_some()
        }) {
            self.acknowledge_source(generation, source);
        }
    }

    pub(crate) fn cancel_external_if_hidden(&mut self) {
        if let Some(active) = self.active.as_ref().filter(|active| {
            active.candidate.source == BeatDragSource::BeatBoard
                && active.candidate.externally_engaged
                && active.candidate.terminal.is_none()
        }) {
            let generation = active.candidate.generation;
            self.claim_invalid(generation);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.active = None;
    }

    fn clear_if_complete(&mut self) {
        let clear = self.active.as_ref().is_some_and(|active| {
            active.candidate.terminal.is_some()
                && active.source_acknowledged
                && (active.command_intent.is_none() || active.intent_consumed)
        });
        if clear {
            self.active = None;
        }
    }
}

fn finite_pos(position: Pos2) -> bool {
    position.x.is_finite() && position.y.is_finite()
}
