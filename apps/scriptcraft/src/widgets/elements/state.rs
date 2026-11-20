//! Ephemeral visibility and pending-operation state for the Elements windows.
//!
//! This value intentionally contains only desktop presentation state. It has
//! no document/session references and cannot construct or mutate a candidate.

use scriptcraft_commands::EditorSurface;
use scriptcraft_fdx::BuiltinTemplateId;

/// The selector operation currently presented to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemplateSelectorMode {
    ApplyToCurrent,
    /// Starts a selected built-in in a fresh independent child document.
    NewDocument,
}

/// A fixed, value-free failure category suitable for bounded inline feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemplateSelectorError {
    TargetLoad,
    CurrentDocumentEdit,
    NewDocumentLaunch,
}

/// Pending selector presentation state; never stores template contents or plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TemplateSelectorState {
    mode: TemplateSelectorMode,
    selected_template: Option<BuiltinTemplateId>,
    error: Option<TemplateSelectorError>,
    focus_requested: bool,
}

impl TemplateSelectorState {
    fn new(mode: TemplateSelectorMode) -> Self {
        Self {
            mode,
            selected_template: None,
            error: None,
            focus_requested: true,
        }
    }

    pub(crate) const fn mode(self) -> TemplateSelectorMode {
        self.mode
    }

    pub(crate) const fn selected_template(self) -> Option<BuiltinTemplateId> {
        self.selected_template
    }

    pub(crate) const fn error(self) -> Option<TemplateSelectorError> {
        self.error
    }

    pub(crate) const fn can_apply(self) -> bool {
        selector_mode_can_apply(self.mode, self.selected_template)
    }

    fn select(&mut self, selected_template: BuiltinTemplateId) {
        self.selected_template = Some(selected_template);
        self.error = None;
    }

    fn set_error(&mut self, error: TemplateSelectorError) {
        self.error = Some(error);
    }
}

pub(crate) const fn selector_mode_can_apply(
    mode: TemplateSelectorMode,
    selected_template: Option<BuiltinTemplateId>,
) -> bool {
    matches!(mode, TemplateSelectorMode::ApplyToCurrent) && selected_template.is_some()
}

pub(crate) const fn selector_mode_can_create_new(
    mode: TemplateSelectorMode,
    selected_template: Option<BuiltinTemplateId>,
) -> bool {
    matches!(mode, TemplateSelectorMode::NewDocument) && selected_template.is_some()
}

/// All Elements inspector/selector state owned by the desktop app.
///
/// `focus_return_surface` is captured once when Elements opens. Repeated open
/// activations do not disturb an in-progress selector or overwrite that return
/// target. A New-document selector may stand alone; Apply remains nested under
/// Elements. Both cases are represented here rather than in separate flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ElementWindowState {
    elements_open: bool,
    selector: Option<TemplateSelectorState>,
    focus_return_surface: Option<EditorSurface>,
    elements_focus_requested: bool,
}

impl ElementWindowState {
    pub(crate) const fn elements_open(self) -> bool {
        self.elements_open
    }

    pub(crate) const fn selector_open(self) -> bool {
        self.selector.is_some()
    }

    /// Element windows own keyboard and pointer input while either surface is
    /// visible, independently of the migration/native-modal lock state.
    pub(crate) const fn owns_input(self) -> bool {
        self.elements_open || self.selector.is_some()
    }

    pub(crate) const fn selector(self) -> Option<TemplateSelectorState> {
        self.selector
    }

    pub(crate) const fn focus_return_surface(self) -> Option<EditorSurface> {
        self.focus_return_surface
    }

    /// Consumed by the root window composition to focus Elements after an open
    /// activation. This flag is presentation-only and never encodes a document
    /// selection.
    pub(crate) fn take_elements_focus_request(&mut self) -> bool {
        std::mem::take(&mut self.elements_focus_requested)
    }

    /// Consumed by the root window composition to focus the selector after an
    /// open/reopen activation without resetting its pending selection/error.
    pub(crate) fn take_selector_focus_request(&mut self) -> bool {
        self.selector
            .as_mut()
            .is_some_and(|selector| std::mem::take(&mut selector.focus_requested))
    }

    pub(crate) const fn selector_apply_ready(self) -> bool {
        match self.selector {
            Some(selector) => selector.can_apply(),
            None => false,
        }
    }

    pub(crate) const fn selector_new_ready(self) -> bool {
        match self.selector {
            Some(selector) => {
                selector_mode_can_create_new(selector.mode(), selector.selected_template())
            }
            None => false,
        }
    }

    /// True only for an open Apply selector and the exact currently selected ID.
    pub(crate) fn selector_apply_matches(&self, requested: BuiltinTemplateId) -> bool {
        matches!(
            self.selector,
            Some(TemplateSelectorState {
                mode: TemplateSelectorMode::ApplyToCurrent,
                selected_template: Some(selected),
                ..
            }) if selected == requested
        )
    }

    pub(crate) fn selector_new_matches(&self, requested: BuiltinTemplateId) -> bool {
        matches!(
            self.selector,
            Some(TemplateSelectorState {
                mode: TemplateSelectorMode::NewDocument,
                selected_template: Some(selected),
                ..
            }) if selected == requested
        )
    }

    /// Opens once and captures the currently active editor surface. Reopening
    /// an already-open Elements window is intentionally a non-resetting focus
    /// activation, including when the selector is currently topmost.
    pub(crate) fn open_elements(&mut self, return_surface: EditorSurface) -> bool {
        if self.elements_open {
            if let Some(selector) = self.selector.as_mut() {
                selector.focus_requested = true;
            } else {
                self.elements_focus_requested = true;
            }
            return false;
        }
        self.elements_open = true;
        self.focus_return_surface = Some(return_surface);
        self.elements_focus_requested = true;
        true
    }

    /// Opens the Apply selector only as a child of the Elements inspector, or
    /// leaves an existing selector untouched so activation only focuses it.
    pub(crate) fn open_selector(&mut self, mode: TemplateSelectorMode) -> bool {
        if let Some(selector) = self.selector.as_mut() {
            selector.focus_requested = true;
            return false;
        }
        if !self.elements_open || mode != TemplateSelectorMode::ApplyToCurrent {
            return false;
        }
        self.selector = Some(TemplateSelectorState::new(mode));
        true
    }

    /// File → New from Template presents the shared selector without opening
    /// the read-only Elements inspector behind it.
    pub(crate) fn open_new_template_selector(&mut self, return_surface: EditorSurface) -> bool {
        if let Some(selector) = self.selector.as_mut() {
            selector.focus_requested = true;
            return false;
        }
        self.focus_return_surface = Some(return_surface);
        self.selector = Some(TemplateSelectorState::new(
            TemplateSelectorMode::NewDocument,
        ));
        true
    }

    /// Changes only pending selection/error presentation state.
    pub(crate) fn select_template(&mut self, selected_template: BuiltinTemplateId) -> bool {
        let Some(selector) = self.selector.as_mut() else {
            return false;
        };
        selector.select(selected_template);
        true
    }

    /// Records a bounded category only while a selector is open.
    pub(crate) fn set_selector_error(&mut self, error: TemplateSelectorError) -> bool {
        let Some(selector) = self.selector.as_mut() else {
            return false;
        };
        selector.set_error(error);
        true
    }

    /// Cancel, title close, and Escape share this state-only transition.
    pub(crate) fn cancel_selector(&mut self) -> bool {
        let Some(selector) = self.selector.take() else {
            return false;
        };
        if self.elements_open {
            self.elements_focus_requested = true;
        } else if selector.mode() == TemplateSelectorMode::NewDocument {
            *self = Self::default();
        }
        true
    }

    /// Closing Elements closes its child selector and clears all return/error
    /// state without touching any project/session authority.
    pub(crate) fn close_elements(&mut self) -> bool {
        if !self.elements_open && self.selector.is_none() && self.focus_return_surface.is_none() {
            return false;
        }
        *self = Self::default();
        true
    }

    /// Document activation and authorized root close discard all dialog state.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_open_preserves_selection_error_mode_and_focus_return() {
        let mut state = ElementWindowState::default();
        assert!(state.open_elements(EditorSurface::TitlePage));
        assert!(state.take_elements_focus_request());
        assert!(state.open_selector(TemplateSelectorMode::ApplyToCurrent));
        assert!(state.take_selector_focus_request());
        assert!(state.select_template(BuiltinTemplateId::Stageplay));
        assert!(state.set_selector_error(TemplateSelectorError::TargetLoad));

        assert!(!state.open_elements(EditorSurface::Body));
        assert!(!state.take_elements_focus_request());
        assert!(state.take_selector_focus_request());
        assert!(!state.open_selector(TemplateSelectorMode::NewDocument));
        assert!(state.take_selector_focus_request());
        let selector = state.selector().expect("selector remains open");
        assert_eq!(selector.mode(), TemplateSelectorMode::ApplyToCurrent);
        assert_eq!(
            selector.selected_template(),
            Some(BuiltinTemplateId::Stageplay)
        );
        assert_eq!(selector.error(), Some(TemplateSelectorError::TargetLoad));
        assert_eq!(state.focus_return_surface(), Some(EditorSurface::TitlePage));
    }

    #[test]
    fn fresh_selector_is_empty_cancel_preserves_inspector_and_close_clears_all() {
        let mut state = ElementWindowState::default();
        assert!(!state.open_selector(TemplateSelectorMode::ApplyToCurrent));
        assert!(state.open_elements(EditorSurface::Body));
        assert!(state.open_selector(TemplateSelectorMode::ApplyToCurrent));
        assert_eq!(
            state
                .selector()
                .and_then(TemplateSelectorState::selected_template),
            None
        );
        assert!(!state.selector_apply_ready());

        assert!(state.select_template(BuiltinTemplateId::Screenplay));
        assert!(state.cancel_selector());
        assert!(state.elements_open());
        assert!(!state.selector_open());
        assert_eq!(state.focus_return_surface(), Some(EditorSurface::Body));

        assert!(state.open_selector(TemplateSelectorMode::ApplyToCurrent));
        assert_eq!(
            state
                .selector()
                .and_then(TemplateSelectorState::selected_template),
            None
        );
        assert!(state.close_elements());
        assert_eq!(state, ElementWindowState::default());
    }

    #[test]
    fn input_ownership_follows_visible_topmost_dialog_and_cancel_returns_to_inspector() {
        let mut state = ElementWindowState::default();
        assert!(!state.owns_input());

        state.open_elements(EditorSurface::TitlePage);
        assert!(state.owns_input());
        state.open_selector(TemplateSelectorMode::ApplyToCurrent);
        assert!(state.owns_input());
        assert!(state.take_selector_focus_request());
        assert!(state.cancel_selector());
        assert!(state.owns_input());
        assert!(state.take_elements_focus_request());

        assert!(state.close_elements());
        assert!(!state.owns_input());
    }

    #[test]
    fn readiness_requires_apply_mode_and_exact_current_selection() {
        let mut state = ElementWindowState::default();
        state.open_elements(EditorSurface::Body);
        state.open_selector(TemplateSelectorMode::ApplyToCurrent);
        assert!(!state.selector_apply_matches(BuiltinTemplateId::Screenplay));
        state.select_template(BuiltinTemplateId::Stageplay);
        assert!(state.selector_apply_ready());
        assert!(state.selector_apply_matches(BuiltinTemplateId::Stageplay));
        assert!(!state.selector_apply_matches(BuiltinTemplateId::Screenplay));

        state.cancel_selector();
        state.open_new_template_selector(EditorSurface::Body);
        state.select_template(BuiltinTemplateId::Stageplay);
        assert!(!state.selector_apply_ready());
        assert!(!state.selector_apply_matches(BuiltinTemplateId::Stageplay));
        assert!(state.selector_new_ready());
        assert!(state.selector_new_matches(BuiltinTemplateId::Stageplay));
    }

    #[test]
    fn new_selector_opens_without_inspector_and_cancel_clears_its_transient_state() {
        let mut state = ElementWindowState::default();
        assert!(state.open_new_template_selector(EditorSurface::TitlePage));
        assert!(!state.elements_open());
        assert!(state.selector_open());
        assert_eq!(state.focus_return_surface(), Some(EditorSurface::TitlePage));
        assert!(!state.selector_new_ready());

        assert!(state.select_template(BuiltinTemplateId::Stageplay));
        assert!(state.selector_new_matches(BuiltinTemplateId::Stageplay));
        assert!(state.set_selector_error(TemplateSelectorError::NewDocumentLaunch));
        assert!(!state.open_new_template_selector(EditorSurface::Body));
        assert!(state.take_selector_focus_request());
        let retained = state
            .selector()
            .expect("New selector stays open after launch failure");
        assert_eq!(
            retained.selected_template(),
            Some(BuiltinTemplateId::Stageplay)
        );
        assert_eq!(
            retained.error(),
            Some(TemplateSelectorError::NewDocumentLaunch)
        );
        assert!(state.cancel_selector());
        assert_eq!(state, ElementWindowState::default());
    }

    #[test]
    fn changing_selection_clears_only_the_bounded_presentation_error() {
        let mut state = ElementWindowState::default();
        state.open_elements(EditorSurface::Body);
        state.open_selector(TemplateSelectorMode::ApplyToCurrent);
        state.select_template(BuiltinTemplateId::Screenplay);
        state.set_selector_error(TemplateSelectorError::TargetLoad);

        state.select_template(BuiltinTemplateId::DramatistsGuildMusical);
        let selector = state.selector().expect("selector remains open");
        assert_eq!(
            selector.selected_template(),
            Some(BuiltinTemplateId::DramatistsGuildMusical)
        );
        assert_eq!(selector.error(), None);
        assert!(state.elements_open());
    }

    #[test]
    fn cancel_discards_selection_and_error_and_a_fresh_selector_is_empty() {
        let mut state = ElementWindowState::default();
        state.open_elements(EditorSurface::Body);
        state.open_selector(TemplateSelectorMode::ApplyToCurrent);
        state.select_template(BuiltinTemplateId::Stageplay);
        state.set_selector_error(TemplateSelectorError::TargetLoad);

        assert!(state.cancel_selector());
        assert!(state.elements_open());
        assert!(!state.selector_open());
        assert_eq!(state.focus_return_surface(), Some(EditorSurface::Body));

        assert!(state.open_selector(TemplateSelectorMode::ApplyToCurrent));
        let fresh = state.selector().expect("fresh selector state");
        assert_eq!(fresh.selected_template(), None);
        assert_eq!(fresh.error(), None);
    }

    #[test]
    fn reset_discards_pending_values_without_any_document_authority() {
        let mut state = ElementWindowState::default();
        state.open_elements(EditorSurface::TitlePage);
        state.open_selector(TemplateSelectorMode::ApplyToCurrent);
        state.select_template(BuiltinTemplateId::DramatistsGuildMusical);
        state.set_selector_error(TemplateSelectorError::CurrentDocumentEdit);
        state.reset();
        assert_eq!(state, ElementWindowState::default());
    }
}
