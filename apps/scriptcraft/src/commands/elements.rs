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
