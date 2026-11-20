use crate::{app::ScriptCraftApp, widgets::elements::state::TemplateSelectorMode};
use scriptcraft_commands::{
    builtin_template_id, execute_editor_command_on_surface, Command, CommandOrigin, CommandOutcome,
    CommandRejection, EditorSurface,
};
use scriptcraft_session::editing::ApplyTemplateError;

use super::DesktopCommandOutcome;

impl ScriptCraftApp {
    /// Routes typed Elements/selector commands through the common desktop
    /// dispatcher. Apply alone delegates semantic work to the shared headless
    /// executor after revalidating the live selector state.
    pub(super) fn dispatch_elements_command(
        &mut self,
        command: Command,
        ctx: &eframe::egui::Context,
    ) -> DesktopCommandOutcome {
        match command {
            Command::FormatElementsOpen => {
                self.element_windows
                    .open_elements(self.active_editor_surface());
                self.transfer_focus_to_element_windows(ctx);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FormatElementsClose => {
                let return_surface = self.element_windows.focus_return_surface();
                if !self.element_windows.close_elements() {
                    return DesktopCommandOutcome::Unavailable(
                        super::DesktopUnavailableReason::ModalBlocked,
                    );
                }
                self.command_projection_cache = None;
                self.restore_focus_after_element_windows(return_surface, ctx);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FormatTemplateSelectorOpen => {
                self.element_windows
                    .open_selector(TemplateSelectorMode::ApplyToCurrent);
                self.transfer_focus_to_element_windows(ctx);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileNewFromTemplateOpen => {
                self.element_windows
                    .open_new_template_selector(self.active_editor_surface());
                self.command_projection_cache = None;
                self.transfer_focus_to_element_windows(ctx);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FormatTemplateSelectorCancel => {
                let return_surface = self.element_windows.focus_return_surface();
                let inspector_remains_open = self.element_windows.elements_open();
                if !self.element_windows.cancel_selector() {
                    return DesktopCommandOutcome::Unavailable(
                        super::DesktopUnavailableReason::ModalBlocked,
                    );
                }
                self.command_projection_cache = None;
                if inspector_remains_open {
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
                } else {
                    self.restore_focus_after_element_windows(return_surface, ctx);
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FormatTemplateApply { template } => {
                let template_id = builtin_template_id(template);
                if !self.element_windows.selector_apply_matches(template_id) {
                    return DesktopCommandOutcome::Unavailable(
                        super::DesktopUnavailableReason::Selection,
                    );
                }

                let command = Command::FormatTemplateApply { template };
                let active_surface = self.active_editor_surface();
                let outcome =
                    execute_editor_command_on_surface(&mut self.session, &command, active_surface);
                match &outcome {
                    CommandOutcome::Changed => {
                        self.refresh_after_builtin_template_apply(ctx);
                        self.element_windows.cancel_selector();
                        self.command_projection_cache = None;
                    }
                    CommandOutcome::NoOp => {
                        self.element_windows.cancel_selector();
                        self.command_projection_cache = None;
                        ctx.request_repaint();
                    }
                    CommandOutcome::ApplyTemplateFailed(error) => {
                        self.element_windows
                            .set_selector_error(selector_error_for_apply(*error));
                        ctx.request_repaint();
                    }
                    CommandOutcome::Unavailable(_) | CommandOutcome::Rejected(_) => {
                        ctx.request_repaint();
                    }
                    CommandOutcome::Failed(_) => {
                        self.element_windows.set_selector_error(
                            crate::widgets::elements::state::TemplateSelectorError::CurrentDocumentEdit,
                        );
                        ctx.request_repaint();
                    }
                }
                DesktopCommandOutcome::Editor(outcome)
            }
            Command::FileNewFromTemplate { template } => {
                let template_id = builtin_template_id(template);
                if !self.element_windows.selector_new_matches(template_id) {
                    return DesktopCommandOutcome::Unavailable(
                        super::DesktopUnavailableReason::Selection,
                    );
                }
                if self.launch_template_instance(template_id).is_ok() {
                    let return_surface = self.element_windows.focus_return_surface();
                    self.element_windows.cancel_selector();
                    self.command_projection_cache = None;
                    self.restore_focus_after_element_windows(return_surface, ctx);
                } else {
                    self.element_windows.set_selector_error(
                        crate::widgets::elements::state::TemplateSelectorError::NewDocumentLaunch,
                    );
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    fn refresh_after_builtin_template_apply(&mut self, ctx: &eframe::egui::Context) {
        self.invalidate_surface(EditorSurface::Body);
        self.invalidate_surface(EditorSurface::TitlePage);
        self.prepared_document_fonts = None;
        self.invalidate_dynamic_controls();
        self.reconcile_body_active_group();
        ctx.request_repaint();
    }

    fn transfer_focus_to_element_windows(&mut self, ctx: &eframe::egui::Context) {
        // The dialogs own input even though Body and Title remain painted. Clear
        // only transient interaction/focus presentation; semantic selections and
        // any active IME composition are retained for a safe return.
        self.surface_focused = false;
        self.title_view.surface_focused = false;
        self.set_active_body_group(None);
        self.drag_anchor = None;
        self.title_view.drag_anchor = None;
        self.reveal_intent = crate::views::script::RevealIntent::None;
        self.title_view.reveal_intent = crate::views::script::RevealIntent::None;
        ctx.memory_mut(|memory| {
            if let Some(editor_id) = self.editor_surface_id {
                memory.surrender_focus(editor_id);
            }
            if let Some(editor_id) = self.title_editor_surface_id {
                memory.surrender_focus(editor_id);
            }
        });
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
    }

    fn restore_focus_after_element_windows(
        &mut self,
        requested_surface: Option<EditorSurface>,
        ctx: &eframe::egui::Context,
    ) {
        if self.element_windows.owns_input() || self.replacement_open_pending() {
            return;
        }
        let availability = self.app_command_availability();
        if !crate::documents::replacement::replacement_resume_focus_allowed(
            availability.modal_blocked,
            availability.native_output_active,
        ) {
            return;
        }

        let surface = dialog_focus_return_target(
            requested_surface,
            self.title_page_window_open,
            self.title_viewport_unsupported,
        );
        self.set_active_editor_surface(surface);
        self.surface_focused = surface == EditorSurface::Body;
        self.title_view.surface_focused = surface == EditorSurface::TitlePage;
        if surface == EditorSurface::Body {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
        }
        self.restore_editor_focus(ctx);
    }
}

fn dialog_focus_return_target(
    requested_surface: Option<EditorSurface>,
    title_page_window_open: bool,
    title_viewport_unsupported: bool,
) -> EditorSurface {
    match requested_surface {
        Some(EditorSurface::TitlePage) if title_page_window_open && !title_viewport_unsupported => {
            EditorSurface::TitlePage
        }
        _ => EditorSurface::Body,
    }
}

fn selector_error_for_apply(
    error: ApplyTemplateError,
) -> crate::widgets::elements::state::TemplateSelectorError {
    use crate::widgets::elements::state::TemplateSelectorError;
    match error {
        ApplyTemplateError::TargetLoadFailed | ApplyTemplateError::TargetExtractionFailed => {
            TemplateSelectorError::TargetLoad
        }
        ApplyTemplateError::PlanningFailed | ApplyTemplateError::CandidateRejected => {
            TemplateSelectorError::CurrentDocumentEdit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{CommandInvocation, DesktopCommandOutcome};
    use scriptcraft_commands::BuiltinTemplateValue;

    fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|error| panic!("prepared app setup failed: {error:?}"))
    }

    #[test]
    fn dialog_focus_return_uses_only_a_still_open_supported_title_viewport() {
        assert_eq!(
            dialog_focus_return_target(Some(EditorSurface::TitlePage), true, false),
            EditorSurface::TitlePage
        );
        assert_eq!(
            dialog_focus_return_target(Some(EditorSurface::TitlePage), false, false),
            EditorSurface::Body
        );
        assert_eq!(
            dialog_focus_return_target(Some(EditorSurface::TitlePage), true, true),
            EditorSurface::Body
        );
        assert_eq!(
            dialog_focus_return_target(Some(EditorSurface::Body), true, false),
            EditorSurface::Body
        );
        assert_eq!(
            dialog_focus_return_target(None, true, false),
            EditorSurface::Body
        );
    }

    #[test]
    fn open_cancel_and_close_dispatch_only_transient_state_effects() {
        let mut app = valid(ScriptCraftApp::new_for_template_lifecycle_tests());
        let ctx = eframe::egui::Context::default();
        let elements = app.session.project().elements.clone();
        let definitions = app
            .session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let revision = app.session.revision();
        let token = app.session.mutation_token();
        let dirty = app.session.dirty();

        let dispatch = |app: &mut ScriptCraftApp, command| {
            app.dispatch_command(CommandInvocation::new(command, CommandOrigin::Menu), &ctx)
                .outcome
        };

        assert_eq!(
            dispatch(&mut app, Command::FormatElementsOpen),
            DesktopCommandOutcome::EffectDispatched
        );
        assert!(app.element_windows.elements_open());
        assert_eq!(
            dispatch(&mut app, Command::FormatTemplateSelectorOpen),
            DesktopCommandOutcome::EffectDispatched
        );
        assert!(app.element_windows.selector_open());
        assert!(app
            .element_windows
            .select_template(scriptcraft_fdx::BuiltinTemplateId::Stageplay));
        assert_eq!(
            dispatch(&mut app, Command::FormatTemplateSelectorCancel),
            DesktopCommandOutcome::EffectDispatched
        );
        assert!(app.element_windows.elements_open());
        assert!(!app.element_windows.selector_open());

        assert_eq!(
            dispatch(&mut app, Command::FormatElementsClose),
            DesktopCommandOutcome::EffectDispatched
        );
        assert!(!app.element_windows.elements_open());
        assert!(matches!(
            dispatch(
                &mut app,
                Command::FormatTemplateApply {
                    template: BuiltinTemplateValue::Screenplay,
                },
            ),
            DesktopCommandOutcome::Unavailable(_)
        ));

        assert_eq!(app.session.project().elements, elements);
        assert_eq!(
            app.session
                .project()
                .element_definitions()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            definitions
        );
        assert_eq!(app.session.revision(), revision);
        assert_eq!(app.session.mutation_token(), token);
        assert_eq!(app.session.dirty(), dirty);
        assert!(app.native_path.is_none());
        assert!(app.fdx_baseline.is_none());
    }

    #[test]
    fn new_selector_is_standalone_and_rejects_empty_or_stale_launch_payloads() {
        let mut app = valid(ScriptCraftApp::new_for_template_lifecycle_tests());
        let ctx = eframe::egui::Context::default();
        let before_project = app.session.project().clone();
        let before_revision = app.session.revision();
        let before_token = app.session.mutation_token();
        let open = app.dispatch_command(
            CommandInvocation::new(Command::FileNewFromTemplateOpen, CommandOrigin::Menu),
            &ctx,
        );
        assert_eq!(open.outcome, DesktopCommandOutcome::EffectDispatched);
        assert!(!app.element_windows.elements_open());
        assert_eq!(
            app.element_windows
                .selector()
                .map(|selector| selector.mode()),
            Some(TemplateSelectorMode::NewDocument)
        );

        for template in [
            BuiltinTemplateValue::Screenplay,
            BuiltinTemplateValue::Stageplay,
        ] {
            let attempt = app.dispatch_command(
                CommandInvocation::new(
                    Command::FileNewFromTemplate { template },
                    CommandOrigin::InternalFollowUp,
                ),
                &ctx,
            );
            assert!(matches!(
                attempt.outcome,
                DesktopCommandOutcome::Unavailable(_)
            ));
        }
        assert_eq!(app.session.project(), &before_project);
        assert_eq!(app.session.revision(), before_revision);
        assert_eq!(app.session.mutation_token(), before_token);
        assert!(app.native_path.is_none());
        assert!(app.fdx_baseline.is_none());
        assert_eq!(
            app.element_windows
                .selector()
                .and_then(|selector| selector.error()),
            None
        );
    }

    #[test]
    fn apply_dispatch_rejects_stale_payload_then_applies_once_through_the_executor() {
        let mut app = valid(ScriptCraftApp::new_for_template_lifecycle_tests());
        let ctx = eframe::egui::Context::default();
        let revision = app.session.revision();
        let token = app.session.mutation_token();
        let original_content = app
            .session
            .project()
            .elements
            .iter()
            .map(|owner| owner.as_element().text.clone())
            .collect::<Vec<_>>();
        app.element_windows.open_elements(EditorSurface::Body);
        app.element_windows
            .open_selector(TemplateSelectorMode::ApplyToCurrent);

        let dispatch = |app: &mut ScriptCraftApp, template| {
            app.dispatch_command(
                CommandInvocation::new(
                    Command::FormatTemplateApply { template },
                    CommandOrigin::Menu,
                ),
                &ctx,
            )
            .outcome
        };
        assert!(matches!(
            dispatch(&mut app, BuiltinTemplateValue::Screenplay),
            DesktopCommandOutcome::Unavailable(_)
        ));
        app.element_windows
            .select_template(scriptcraft_fdx::BuiltinTemplateId::Stageplay);
        assert!(matches!(
            dispatch(&mut app, BuiltinTemplateValue::Screenplay),
            DesktopCommandOutcome::Unavailable(_)
        ));
        assert_eq!(
            dispatch(&mut app, BuiltinTemplateValue::Stageplay),
            DesktopCommandOutcome::Editor(CommandOutcome::Changed)
        );
        assert_eq!(app.session.revision(), revision + 1);
        assert_eq!(app.session.mutation_token(), token + 1);
        assert!(!app.element_windows.selector_open());
        assert!(app.element_windows.elements_open());
        assert!(app
            .session
            .project()
            .element_definitions()
            .get_by_label("Notations")
            .is_some());
        assert_eq!(
            app.session
                .project()
                .elements
                .iter()
                .map(|owner| owner.as_element().text.clone())
                .collect::<Vec<_>>(),
            original_content
        );
        assert!(app.native_path.is_none());
        assert!(app.fdx_baseline.is_none());
        assert!(app.prepared_document_fonts.is_none());
        let refreshed_choices = app
            .refresh_dynamic_controls()
            .choices
            .iter()
            .map(|choice| choice.key.as_str().to_owned())
            .collect::<Vec<_>>();
        let live_definition_keys = app
            .session
            .project()
            .element_definitions()
            .iter()
            .map(|definition| definition.key.as_str().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(refreshed_choices, live_definition_keys);
    }

    #[test]
    fn apply_noop_closes_only_the_selector_without_mutation_or_surface_refresh() {
        let mut app = valid(ScriptCraftApp::new_for_template_lifecycle_tests());
        let ctx = eframe::egui::Context::default();
        let revision = app.session.revision();
        let token = app.session.mutation_token();
        app.element_windows.open_elements(EditorSurface::Body);
        app.element_windows
            .open_selector(TemplateSelectorMode::ApplyToCurrent);
        app.element_windows
            .select_template(scriptcraft_fdx::BuiltinTemplateId::Screenplay);

        let result = app.dispatch_command(
            CommandInvocation::new(
                Command::FormatTemplateApply {
                    template: BuiltinTemplateValue::Screenplay,
                },
                CommandOrigin::Menu,
            ),
            &ctx,
        );

        assert_eq!(
            result.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::NoOp)
        );
        assert_eq!(app.session.revision(), revision);
        assert_eq!(app.session.mutation_token(), token);
        assert!(!app.element_windows.selector_open());
        assert!(app.element_windows.elements_open());
    }

    #[test]
    fn apply_failures_map_to_two_fixed_value_free_selector_categories() {
        use crate::widgets::elements::state::TemplateSelectorError;

        assert_eq!(
            selector_error_for_apply(ApplyTemplateError::TargetLoadFailed),
            TemplateSelectorError::TargetLoad
        );
        assert_eq!(
            selector_error_for_apply(ApplyTemplateError::TargetExtractionFailed),
            TemplateSelectorError::TargetLoad
        );
        assert_eq!(
            selector_error_for_apply(ApplyTemplateError::PlanningFailed),
            TemplateSelectorError::CurrentDocumentEdit
        );
        assert_eq!(
            selector_error_for_apply(ApplyTemplateError::CandidateRejected),
            TemplateSelectorError::CurrentDocumentEdit
        );

        let mut state = crate::widgets::elements::state::ElementWindowState::default();
        state.open_elements(EditorSurface::Body);
        state.open_selector(TemplateSelectorMode::ApplyToCurrent);
        state.select_template(scriptcraft_fdx::BuiltinTemplateId::Stageplay);
        state.set_selector_error(TemplateSelectorError::CurrentDocumentEdit);
        assert!(state.selector_open());
        assert_eq!(
            state
                .selector()
                .and_then(|selector| selector.selected_template()),
            Some(scriptcraft_fdx::BuiltinTemplateId::Stageplay)
        );
        assert_eq!(
            state.selector().and_then(|selector| selector.error()),
            Some(TemplateSelectorError::CurrentDocumentEdit)
        );
    }
}
