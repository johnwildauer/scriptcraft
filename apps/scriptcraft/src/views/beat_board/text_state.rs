//! Beat-only transient text focus, selection gesture, viewport, and IME state.

use eframe::egui;
use scriptcraft_model::StableId;
use scriptcraft_session::editing::{BeatTextField, BeatTextPosition};

#[derive(Default)]
pub(crate) struct BeatTextState {
    pub(crate) focused: bool,
    pub(crate) field: Option<(StableId, BeatTextField)>,
    pub(crate) drag_anchor: Option<BeatTextPosition>,
    pub(crate) title_offset: f32,
    pub(crate) body_offset: f32,
    pub(crate) preferred_x: Option<f32>,
    pub(crate) ime_active: bool,
    pub(crate) ime_preedit: String,
    pub(crate) suppress_text_commit: Option<String>,
    pub(crate) caret_reset_time: f64,
    pub(crate) viewport_id: Option<egui::Id>,
}

impl BeatTextState {
    pub(crate) fn clear_field_view(&mut self) {
        self.title_offset = 0.0;
        self.body_offset = 0.0;
        self.preferred_x = None;
        self.drag_anchor = None;
        self.ime_preedit.clear();
        self.ime_active = false;
        self.suppress_text_commit = None;
    }

    pub(crate) fn blur(&mut self) {
        self.focused = false;
        self.field = None;
        self.clear_field_view();
    }

    pub(crate) fn focus(&mut self, card: StableId, field: BeatTextField, time: f64) {
        if self.field.as_ref().is_none_or(|(old_card, old_field)| old_card != &card || *old_field != field) {
            self.clear_field_view();
        }
        self.field = Some((card, field));
        self.focused = true;
        self.caret_reset_time = time;
    }
}
