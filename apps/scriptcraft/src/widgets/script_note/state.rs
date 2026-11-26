use scriptcraft_model::{formatted_text::InlineColor, StableId};
#[cfg(target_os = "windows")]
use std::collections::HashMap;

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePresentationState {
    Hidden,
    Revealed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MarkerAnchor {
    pub(crate) line_key: (usize, usize),
    pub(crate) baseline: f32,
    pub(crate) marker_left: f32,
    pub(crate) marker_right: f32,
    pub(crate) baseline_visible: bool,
}

/// Validated current-monitor work bounds in logical points and top-left screen
/// coordinates. This is transient presentation input, never document state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WorkAreaBounds {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) right: f32,
    pub(crate) bottom: f32,
}

impl WorkAreaBounds {
    pub(crate) fn new(left: f32, top: f32, right: f32, bottom: f32) -> Option<Self> {
        (left.is_finite()
            && top.is_finite()
            && right.is_finite()
            && bottom.is_finite()
            && right > left
            && bottom > top)
            .then_some(Self { left, top, right, bottom })
    }

    pub(crate) fn rect(self) -> eframe::egui::Rect {
        eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(self.left, self.top),
            eframe::egui::pos2(self.right, self.bottom),
        )
    }
}

/// Presentation-only state for the currently open ScriptNote. Note text and
/// properties remain authoritative in the current Project.
#[derive(Debug, Clone)]
pub(crate) struct ScriptNoteWorkspaceState {
    pub(crate) open_note: Option<StableId>,
    pub(crate) focus_body_on_next_present: bool,
    pub(crate) last_color: Option<InlineColor>,
    pub(crate) last_type: Option<String>,
    pub(crate) unsupported_viewport: bool,
    pub(crate) reset_generation: u64,
    pub(crate) hovered_marker: Option<(usize, usize)>,
    pub(crate) type_menu_open: bool,
    pub(crate) color_menu_open: bool,
    pub(crate) type_menu_focused: bool,
    pub(crate) color_menu_focused: bool,
    pub(crate) type_menu_focus_pending: bool,
    pub(crate) color_menu_focus_pending: bool,
    pub(crate) type_menu_close_pending: bool,
    pub(crate) color_menu_close_pending: bool,
    pub(crate) type_menu_measured_size: Option<eframe::egui::Vec2>,
    pub(crate) color_menu_measured_size: Option<eframe::egui::Vec2>,
    pub(crate) new_type_dialog_open: bool,
    pub(crate) type_draft: String,
    pub(crate) type_return_focus: Option<scriptcraft_session::editing::ScriptNoteField>,
    pub(crate) type_dialog_focused: bool,
    pub(crate) type_dialog_error: bool,
    pub(crate) focused: bool,
    pub(crate) field: Option<scriptcraft_session::editing::ScriptNoteField>,
    pub(crate) body_offset: f32,
    pub(crate) title_offset: f32,
    pub(crate) preferred_x: Option<f32>,
    pub(crate) ime_active: bool,
    pub(crate) ime_preedit: String,
    pub(crate) suppress_text_commit: Option<String>,
    pub(crate) caret_reset_time: f64,
    pub(crate) pointer_anchor: Option<eframe::egui::Pos2>,
    pub(crate) marker_anchor: Option<MarkerAnchor>,
    pub(crate) work_bounds: Option<WorkAreaBounds>,
    pub(crate) native_position: Option<eframe::egui::Pos2>,
    pub(crate) viewport_close_pending: bool,
    pub(crate) type_dialog_close_pending: bool,
    pub(crate) pointer_captured: bool,
    pub(crate) body_scrollbar_dragging: bool,
    pub(crate) delete_hovered: bool,
    pub(crate) text_drag_anchor: Option<scriptcraft_session::editing::ScriptNoteTextPosition>,
    pub(crate) family_focus_loss_frames: u8,
    pub(crate) focus_viewport_on_next_present: bool,
    pub(crate) viewport_focused: bool,
    pub(crate) reveal_pending: bool,
    #[cfg(target_os = "windows")]
    pub(crate) native_presentations: HashMap<eframe::egui::ViewportId, NativePresentationState>,
}

impl Default for ScriptNoteWorkspaceState {
    fn default() -> Self {
        Self {
            open_note: None,
            focus_body_on_next_present: false,
            last_color: None,
            last_type: None,
            unsupported_viewport: false,
            reset_generation: 0,
            hovered_marker: None,
            type_menu_open: false,
            color_menu_open: false,
            type_menu_focused: false,
            color_menu_focused: false,
            type_menu_focus_pending: false,
            color_menu_focus_pending: false,
            type_menu_close_pending: false,
            color_menu_close_pending: false,
            type_menu_measured_size: None,
            color_menu_measured_size: None,
            new_type_dialog_open: false,
            type_draft: String::new(),
            type_return_focus: None,
            type_dialog_focused: false,
            type_dialog_error: false,
            focused: false,
            field: None,
            body_offset: 0.0,
            title_offset: 0.0,
            preferred_x: None,
            ime_active: false,
            ime_preedit: String::new(),
            suppress_text_commit: None,
            caret_reset_time: 0.0,
            pointer_anchor: None,
            marker_anchor: None,
            work_bounds: None,
            native_position: None,
            viewport_close_pending: false,
            type_dialog_close_pending: false,
            pointer_captured: false,
            body_scrollbar_dragging: false,
            delete_hovered: false,
            text_drag_anchor: None,
            family_focus_loss_frames: 0,
            focus_viewport_on_next_present: false,
            viewport_focused: false,
            reveal_pending: false,
            #[cfg(target_os = "windows")]
            native_presentations: HashMap::new(),
        }
    }
}

impl ScriptNoteWorkspaceState {
    #[cfg(target_os = "windows")]
    pub(crate) fn needs_hidden_first_presentation(&self, id: eframe::egui::ViewportId) -> bool {
        !self.native_presentations.contains_key(&id)
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn needs_reveal(&self, id: eframe::egui::ViewportId) -> bool {
        self.native_presentations.get(&id) == Some(&NativePresentationState::Hidden)
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn mark_hidden(&mut self, id: eframe::egui::ViewportId) {
        self.native_presentations
            .insert(id, NativePresentationState::Hidden);
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn mark_revealed(&mut self, id: eframe::egui::ViewportId) {
        self.native_presentations
            .insert(id, NativePresentationState::Revealed);
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn forget_presentation(&mut self, id: eframe::egui::ViewportId) {
        self.native_presentations.remove(&id);
    }

    pub(crate) fn present(&mut self, note: StableId, focus_body: bool) {
        self.type_menu_close_pending |= self.type_menu_open;
        self.color_menu_close_pending |= self.color_menu_open;
        self.type_dialog_close_pending |= self.new_type_dialog_open;
        self.open_note = Some(note);
        self.reveal_pending = false;
        self.focus_body_on_next_present = focus_body;
        self.hovered_marker = None;
        self.body_offset = 0.0;
        self.title_offset = 0.0;
        self.preferred_x = None;
        self.ime_active = false;
        self.ime_preedit.clear();
        self.suppress_text_commit = None;
        self.pointer_captured = false;
        self.body_scrollbar_dragging = false;
        self.delete_hovered = false;
        self.text_drag_anchor = None;
        self.family_focus_loss_frames = 0;
        self.focus_viewport_on_next_present = true;
        self.viewport_focused = false;
        self.type_menu_open = false;
        self.color_menu_open = false;
        self.type_menu_measured_size = None;
        self.color_menu_measured_size = None;
        self.type_menu_focused = false;
        self.color_menu_focused = false;
        self.type_menu_focus_pending = false;
        self.color_menu_focus_pending = false;
        self.new_type_dialog_open = false;
        self.type_draft.clear();
        self.type_return_focus = None;
        self.type_dialog_focused = false;
        self.type_dialog_error = false;
        self.hovered_marker = None;
    }

    pub(crate) fn dismiss(&mut self) -> bool {
        let was_open = self.open_note.take().is_some();
        #[cfg(target_os = "windows")]
        {
            self.native_presentations.clear();
        }
        self.reveal_pending = false;
        self.type_dialog_close_pending |= self.new_type_dialog_open;
        self.type_menu_close_pending |= self.type_menu_open;
        self.color_menu_close_pending |= self.color_menu_open;
        self.focus_body_on_next_present = false;
        self.type_menu_open = false;
        self.color_menu_open = false;
        self.type_menu_measured_size = None;
        self.color_menu_measured_size = None;
        self.type_menu_focused = false;
        self.color_menu_focused = false;
        self.type_menu_focus_pending = false;
        self.color_menu_focus_pending = false;
        self.new_type_dialog_open = false;
        self.type_draft.clear();
        self.type_return_focus = None;
        self.type_dialog_focused = false;
        self.type_dialog_error = false;
        self.hovered_marker = None;
        self.focused = false;
        self.field = None;
        self.body_offset = 0.0;
        self.title_offset = 0.0;
        self.preferred_x = None;
        self.ime_active = false;
        self.ime_preedit.clear();
        self.suppress_text_commit = None;
        self.pointer_captured = false;
        self.body_scrollbar_dragging = false;
        self.delete_hovered = false;
        self.text_drag_anchor = None;
        self.family_focus_loss_frames = 0;
        self.focus_viewport_on_next_present = false;
        self.viewport_focused = false;
        self.marker_anchor = None;
        self.native_position = None;
        self.pointer_anchor = None;
        was_open
    }

    pub(crate) fn reset_document_activation(&mut self) {
        self.dismiss();
        self.work_bounds = None;
        self.viewport_close_pending = true;
        self.type_dialog_close_pending = false;
        self.last_color = None;
        self.last_type = None;
        self.unsupported_viewport = false;
        self.reset_generation = self.reset_generation.wrapping_add(1);
    }

    pub(crate) fn update_defaults(&mut self, color: Option<InlineColor>, note_type: Option<String>) {
        self.last_color = color;
        self.last_type = note_type;
    }
}
