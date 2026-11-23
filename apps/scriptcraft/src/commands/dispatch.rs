use crate::app::ScriptCraftApp;
use crate::documents::lifecycle::SaveOutcome;
use eframe::egui;
use scriptcraft_commands::{
    Command, CommandId, CommandOrigin, CommandOutcome, CommandRejection, DiagnosticMessage,
    EditorSurface, HostEffectDescription, MAX_DIAGNOSTIC_MESSAGE_BYTES,
};
use std::collections::VecDeque;

/// An activation plus its diagnostic source. Origins never change execution
/// semantics; adapters assign the origin as they create the invocation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandInvocation {
    command: Command,
    origin: CommandOrigin,
}

impl CommandInvocation {
    pub(crate) const fn new(command: Command, origin: CommandOrigin) -> Self {
        Self { command, origin }
    }

    pub(crate) const fn command(&self) -> &Command {
        &self.command
    }

    pub(crate) const fn origin(&self) -> CommandOrigin {
        self.origin
    }
}

/// Static menu, toolbar, and native application-menu IDs map to parameterless
/// commands. Dynamic payloads (such as an exact toolbar element type) are built at activation.
pub(crate) fn command_for_id(id: CommandId) -> Option<Command> {
    Some(match id {
        CommandId::FileNew => Command::FileNew,
        CommandId::FileNewFromTemplateOpen => Command::FileNewFromTemplateOpen,
        CommandId::FileOpenRequest => Command::FileOpenRequest,
        CommandId::FileClose => Command::FileClose,
        CommandId::FileSave => Command::FileSave,
        CommandId::FileSaveAsRequest => Command::FileSaveAsRequest,
        CommandId::FileSaveAsPdf => Command::FileSaveAsPdf,
        CommandId::ViewInvisiblesToggle => Command::ViewInvisiblesToggle,
        CommandId::FilePrint => Command::FilePrint,
        CommandId::ApplicationQuit => Command::ApplicationQuit,
        CommandId::EditUndo => Command::EditUndo,
        CommandId::EditRedo => Command::EditRedo,
        CommandId::EditCut => Command::EditCut,
        CommandId::EditCopy => Command::EditCopy,
        CommandId::EditPaste => Command::EditPaste,
        CommandId::EditDelete => Command::EditDelete,
        CommandId::EditSelectAll => Command::EditSelectAll,
        CommandId::WindowTitlePageOpen => Command::WindowTitlePageOpen,
        CommandId::WindowTitlePageClose => Command::WindowTitlePageClose,
        CommandId::EditorDualDialogueToggle => Command::EditorDualDialogueToggle,
        CommandId::EditorStyleClear => Command::EditorStyleClear {
            surface: EditorSurface::Body,
        },
        CommandId::FormatElementsOpen => Command::FormatElementsOpen,
        CommandId::FormatElementsClose => Command::FormatElementsClose,
        CommandId::FormatTemplateSelectorOpen => Command::FormatTemplateSelectorOpen,
        CommandId::FormatTemplateSelectorCancel => Command::FormatTemplateSelectorCancel,
        _ => return None,
    })
}

/// Typed result of the desktop dispatch boundary. Effect handlers remain the
/// existing desktop adapters; `EffectDispatched` does not claim their external
/// operation succeeded (their existing status/error channel remains authoritative).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopCommandOutcome {
    EffectDispatched,
    /// The save handler completed synchronously; routing or picker launch alone
    /// must never be represented as a successful save.
    SaveCompleted(SaveOutcome),
    HostEffectRequested(HostEffectDescription),
    Editor(CommandOutcome),
    Unavailable(super::DesktopUnavailableReason),
    Rejected(CommandRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopCommandResult {
    pub(crate) command: CommandId,
    pub(crate) origin: CommandOrigin,
    pub(crate) outcome: DesktopCommandOutcome,
}

impl ScriptCraftApp {
    /// The sole desktop activation boundary. It refreshes availability at the
    /// moment of dispatch and keeps platform/document effects behind their
    /// existing adapters while semantic editing delegates to WO-002's executor.
    pub(crate) fn dispatch_command(
        &mut self,
        invocation: CommandInvocation,
        ctx: &egui::Context,
    ) -> DesktopCommandResult {
        self.dispatch_command_with_pdf_handlers(
            invocation,
            ctx,
            |app| app.export_pdf(),
            |app, options| app.export_pdf_with_options(options),
            |app| app.open_print_panel(),
        )
    }

    fn dispatch_command_with_pdf_handlers(
        &mut self,
        invocation: CommandInvocation,
        ctx: &egui::Context,
        export_pdf: impl FnOnce(&mut ScriptCraftApp),
        resolve_pdf_options: impl FnOnce(&mut ScriptCraftApp, scriptcraft_commands::PdfExportOptions),
        open_print_panel: impl FnOnce(&mut ScriptCraftApp),
    ) -> DesktopCommandResult {
        let command_id = invocation.command.id();
        let origin = invocation.origin;
        self.flush_pending_diagnostic_commands(ctx);
        let restore_focus = super::navigation::command_restores_editor_focus(command_id);
        let command =
            route_to_focused_surface(invocation.command, origin, self.active_editor_surface());
        let outcome = self.dispatch_command_inner_with_pdf_handlers(
            command,
            ctx,
            export_pdf,
            resolve_pdf_options,
            open_print_panel,
        );
        self.flush_pending_diagnostic_commands(ctx);
        if restore_focus && !self.replacement_open_pending() {
            self.restore_editor_focus(ctx);
        }
        DesktopCommandResult {
            command: command_id,
            origin,
            outcome,
        }
    }

    fn dispatch_command_inner(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        self.dispatch_command_inner_with_pdf_handlers(
            command,
            ctx,
            |app| app.export_pdf(),
            |app, options| app.export_pdf_with_options(options),
            |app| app.open_print_panel(),
        )
    }

    fn dispatch_command_inner_with_pdf_handlers(
        &mut self,
        command: Command,
        ctx: &egui::Context,
        export_pdf: impl FnOnce(&mut ScriptCraftApp),
        resolve_pdf_options: impl FnOnce(&mut ScriptCraftApp, scriptcraft_commands::PdfExportOptions),
        open_print_panel: impl FnOnce(&mut ScriptCraftApp),
    ) -> DesktopCommandOutcome {
        let availability = self.command_dispatch_availability(&command);
        if !availability.is_enabled(command.id()) {
            if let Some(outcome) = self.unavailable_save_outcome(
                command.id(),
                availability.unavailable_reason(command.id()),
            ) {
                return outcome;
            }
            return DesktopCommandOutcome::Unavailable(
                availability.unavailable_reason(command.id()),
            );
        }

        match command {
            Command::UiStatusSet { message } => {
                self.diagnostic_message = message;
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            command @ (Command::FileNew
            | Command::FileOpenRequest
            | Command::FileOpen { .. }
            | Command::FileClose
            | Command::ApplicationQuit
            | Command::FileSave
            | Command::FileSaveAsRequest
            | Command::FileSaveAs { .. }
            | Command::FileImportResolve { .. }
            | Command::FileSaveAsPdf
            | Command::FilePrint
            | Command::PdfIncludeTitlePageResolve { .. }) => self.dispatch_document_command(
                command,
                ctx,
                export_pdf,
                resolve_pdf_options,
                open_print_panel,
            ),
            command @ (Command::WindowTitlePageOpen
            | Command::WindowTitlePageClose
            | Command::ViewInvisiblesToggle) => {
                self.dispatch_view_command(command, ctx)
            }
            command @ (Command::FormatElementsOpen
            | Command::FormatElementsClose
            | Command::FormatTemplateSelectorOpen
            | Command::FormatTemplateSelectorCancel
            | Command::FormatTemplateApply { .. }
            | Command::FileNewFromTemplateOpen
            | Command::FileNewFromTemplate { .. }) => self.dispatch_elements_command(command, ctx),
            command @ (Command::EditorStyleToggle { .. }
            | Command::EditorStyleClear { .. }
            | Command::EditorAlignmentSet { .. }
            | Command::EditorDualDialogueToggle
            | Command::EditUndo
            | Command::EditRedo
            | Command::EditDelete
            | Command::EditorTextInsert { .. }
            | Command::EditorTextPaste { .. }
            | Command::EditorDeleteBackward
            | Command::EditorParagraphEnter
            | Command::EditorParagraphTab
            | Command::EditSelectAll
            | Command::EditorElementChange { .. }
            | Command::EditorSelectionSet { .. }) => self.dispatch_editing_command(command, ctx),
            command @ (Command::EditorSelectionPointer { .. }
            | Command::EditorNavigationHorizontal { .. }
            | Command::EditorNavigationVertical { .. }
            | Command::EditorNavigationLineEdge { .. }
            | Command::EditorFocus) => self.dispatch_navigation_command(command, ctx),
            command @ (Command::ViewZoomSet { .. }
            | Command::ViewZoomPinch { .. }
            | Command::ViewScriptViewSet { .. }
            | Command::ViewAppearanceSet { .. }
            | Command::ViewFocusModeToggle) => self.dispatch_view_command(command, ctx),
            command @ (Command::ToolsSprintTimerTogglePopup
            | Command::ToolsSprintTimerClosePopup
            | Command::ToolsSprintTimerSetMode { .. }
            | Command::ToolsSprintTimerSetMinutes { .. }
            | Command::ToolsSprintTimerStart
            | Command::ToolsSprintTimerPause
            | Command::ToolsSprintTimerResume
            | Command::ToolsSprintTimerReset
            | Command::ToolsSprintTimerEnd) => self.dispatch_timer_command(command, ctx),
            command @ (Command::EditCopy | Command::EditCut | Command::EditPaste) => {
                self.dispatch_clipboard_command(command, ctx)
            }
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    pub(super) fn dispatch_internal_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        self.dispatch_command(
            CommandInvocation::new(command, CommandOrigin::InternalFollowUp),
            ctx,
        )
        .outcome
    }

    pub(crate) fn flush_pending_diagnostic_commands(&mut self, ctx: &egui::Context) {
        if self.flushing_diagnostic_commands {
            return;
        }
        self.flushing_diagnostic_commands = true;
        while let Some(command) = self.pending_diagnostic_commands.pop_front() {
            let _ = self.dispatch_command(
                CommandInvocation::new(command, CommandOrigin::InternalFollowUp),
                ctx,
            );
        }
        self.flushing_diagnostic_commands = false;
    }

    pub(crate) fn post_diagnostic(&mut self, message: &str) {
        let mut bounded = String::new();
        for character in message.chars() {
            let character =
                if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') {
                    ' '
                } else {
                    character
                };
            if bounded.len() + character.len_utf8() > MAX_DIAGNOSTIC_MESSAGE_BYTES {
                break;
            }
            bounded.push(character);
        }
        let bounded = bounded.trim();
        let message = if bounded.is_empty() {
            "Diagnostic detail unavailable"
        } else {
            bounded
        };
        let Ok(message) = DiagnosticMessage::new(message.to_owned()) else {
            return;
        };
        self.pending_diagnostic_commands
            .push_back(Command::UiStatusSet {
                message: Some(message),
            });
    }
}

fn route_to_focused_surface(
    command: Command,
    origin: CommandOrigin,
    surface: EditorSurface,
) -> Command {
    if !matches!(
        origin,
        CommandOrigin::Keyboard
            | CommandOrigin::Shortcut
            | CommandOrigin::Pointer
            | CommandOrigin::Menu
            | CommandOrigin::NativeMenu
            | CommandOrigin::ImeCommit
    ) {
        return command;
    }
    match command {
        Command::EditorTextInsert { text, .. } => Command::EditorTextInsert { text, surface },
        Command::EditorTextPaste { text, .. } => Command::EditorTextPaste { text, surface },
        Command::EditorElementChange { element_type, .. } => Command::EditorElementChange {
            element_type,
            surface,
        },
        Command::EditorStyleToggle { style, .. } => Command::EditorStyleToggle { surface, style },
        Command::EditorStyleClear { .. } => Command::EditorStyleClear { surface },
        Command::EditorAlignmentSet { alignment, .. } => {
            Command::EditorAlignmentSet { surface, alignment }
        }
        Command::EditorSelectionSet { selection, .. } => {
            Command::EditorSelectionSet { selection, surface }
        }
        Command::EditorSelectionPointer {
            position, phase, ..
        } => Command::EditorSelectionPointer {
            position,
            phase,
            surface,
        },
        Command::EditorNavigationHorizontal {
            direction, extend, ..
        } => Command::EditorNavigationHorizontal {
            direction,
            extend,
            surface,
        },
        Command::EditorNavigationVertical {
            direction, extend, ..
        } => Command::EditorNavigationVertical {
            direction,
            extend,
            surface,
        },
        Command::EditorNavigationLineEdge { edge, extend, .. } => {
            Command::EditorNavigationLineEdge {
                edge,
                extend,
                surface,
            }
        }
        other => other,
    }
}
