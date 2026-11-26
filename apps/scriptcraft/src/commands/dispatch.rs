use crate::app::ScriptCraftApp;
use super::DesktopUnavailableReason;
use crate::documents::lifecycle::SaveOutcome;
use eframe::egui;
use scriptcraft_commands::{
    Command, CommandId, CommandOrigin, CommandOutcome, CommandRejection, DiagnosticMessage,
    EditorSurface, HostEffectDescription, MAX_DIAGNOSTIC_MESSAGE_BYTES,
    ScriptNoteColorValue, ScriptNoteCreateValue, ScriptNoteIdValue,
    ScriptNoteNavigationValue, ScriptNotePresentValue, ScriptNotePropertiesValue,
    ScriptNoteSelectionValue, ScriptNoteTextEditValue, ScriptNoteTextOperationValue,
    ScriptNoteTimestampValue, BeatPointValue,
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

/// Static menu, toolbar, and native application-menu IDs map to shared commands,
/// including fixed payloads where needed. Dynamic payloads (such as an exact
/// toolbar element type) are built at activation.
pub(crate) fn command_for_id(id: CommandId) -> Option<Command> {
    Some(match id {
        CommandId::FileNew => Command::FileNew,
        CommandId::FileNewFromTemplateOpen => Command::FileNewFromTemplateOpen,
        CommandId::FileOpenRequest => Command::FileOpenRequest,
        CommandId::FileClose => Command::FileClose,
        CommandId::FileSave => Command::FileSave,
        CommandId::FileSaveAsRequest => Command::FileSaveAsRequest,
        CommandId::FileSaveAsPdf => Command::FileSaveAsPdf,
        CommandId::FileExportFountain => Command::FileExportFountain,
        CommandId::ViewInvisiblesToggle => Command::ViewInvisiblesToggle,
        CommandId::ViewRulerToggle => Command::ViewRulerToggle,
        CommandId::ViewBeatBoardShow => Command::ViewBeatBoardShow,
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
        CommandId::ScriptNoteCreate => Command::ScriptNoteCreate {
            input: ScriptNoteCreateValue {
                color: None,
                note_type: None,
                timestamp: current_local_timestamp_value()?,
            },
        },
        CommandId::ScriptNoteDelete => Command::ScriptNoteDelete {
            note: ScriptNoteIdValue::new("script-note").expect("fixed ScriptNote ID is valid"),
        },
        CommandId::ScriptNotePropertiesSet => Command::ScriptNotePropertiesSet {
            properties: ScriptNotePropertiesValue {
                note: ScriptNoteIdValue::new("script-note").expect("fixed ScriptNote ID is valid"),
                color: None,
                note_type: None,
                modified: current_local_timestamp_value()?,
            },
        },
        CommandId::ScriptNoteSelectionSet => Command::ScriptNoteSelectionSet {
            selection: ScriptNoteSelectionValue::None,
        },
        CommandId::ScriptNoteTextEdit => Command::ScriptNoteTextEdit {
            edit: ScriptNoteTextEditValue {
                operation: ScriptNoteTextOperationValue::Enter,
                modified: current_local_timestamp_value()?,
            },
        },
        CommandId::ScriptNoteTextNavigate => Command::ScriptNoteTextNavigate {
            target: ScriptNoteNavigationValue::Horizontal { direction: scriptcraft_commands::HorizontalDirection::Left },
            extend: false,
        },
        CommandId::ScriptNotePresent => Command::ScriptNotePresent {
            presentation: ScriptNotePresentValue {
                note: ScriptNoteIdValue::new("script-note").expect("fixed ScriptNote ID is valid"),
                move_caret: false,
                focus_body: false,
            },
        },
        CommandId::ScriptNoteDismiss => Command::ScriptNoteDismiss,
        CommandId::ScriptNoteStyleToggle => Command::ScriptNoteStyleToggle {
            style: scriptcraft_commands::TitleTextStyle::Bold,
            modified: current_local_timestamp_value()?,
        },
        CommandId::ScriptNoteStyleClear => Command::ScriptNoteStyleClear {
            modified: current_local_timestamp_value()?,
        },
        CommandId::ScriptNoteRunFormattingSet => Command::ScriptNoteRunFormattingSet {
            patch: Default::default(),
            modified: current_local_timestamp_value()?,
        },
        CommandId::EditorDialogueAltAdd => Command::EditorDialogueAltAdd,
        CommandId::EditorDialogueAltDelete => Command::EditorDialogueAltDelete,
        CommandId::EditorDialogueAltNavigate => Command::EditorDialogueAltNavigate {
            direction: scriptcraft_commands::HorizontalDirection::Left,
        },
        CommandId::BeatBoardCardCreate => Command::BeatBoardCardCreate {
            position: BeatPointValue { x: 20_000, y: 20_000 },
        },
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
        let mut command = invocation.command.clone();
        let mut note_ids_before_create = None;
        let activation_availability = self.command_dispatch_availability(&command);
        if matches!(command, Command::ScriptNoteCreate { .. })
            && activation_availability.is_enabled(CommandId::ScriptNoteCreate)
        {
            let Some(timestamp) = current_local_timestamp_value() else {
                self.post_diagnostic("ScriptNote timestamp could not be represented; note unchanged");
                self.flush_pending_diagnostic_commands(ctx);
                return DesktopCommandResult {
                    command: command_id,
                    origin,
                    outcome: DesktopCommandOutcome::Rejected(CommandRejection::InvalidScriptNoteValue),
                };
            };
            note_ids_before_create = Some(self.session.project().script_notes().iter()
                .map(|note| note.id.clone()).collect::<Vec<_>>());
            self.script_note_workspace.dismiss();
            self.script_note_workspace.viewport_close_pending = true;
            let color = self.script_note_workspace.last_color.and_then(ScriptNoteColorValue::from_core);
            command = Command::ScriptNoteCreate {
                input: ScriptNoteCreateValue {
                    color,
                    note_type: self.script_note_workspace.last_type.clone(),
                    timestamp,
                },
            };
        }
        if activation_availability.is_enabled(command_id)
            && should_dismiss_script_note_for(command_id)
            && self.script_note_workspace.open_note.is_some()
        {
            self.dismiss_script_note(ctx);
        }
        let dispatched_command = command.clone();
        let shared_body_formatting_intent = matches!(&invocation.command,
            Command::EditorStyleToggle { surface: EditorSurface::Body, .. }
            | Command::EditorStyleClear { surface: EditorSurface::Body }
            | Command::EditorRunFormattingSet { surface: EditorSurface::Body, .. });
        if self.root_document_view == crate::app::RootDocumentView::BeatBoard
            && shared_body_formatting_intent
            && !matches!(origin, CommandOrigin::Menu | CommandOrigin::Toolbar | CommandOrigin::NativeMenu)
        {
            return DesktopCommandResult { command: command_id, origin,
                outcome: DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::Selection) };
        }
        let restore_focus = super::navigation::command_restores_editor_focus(command_id);
        let command = route_to_focused_surface(command, origin, self.active_editor_surface());
        let command = route_to_beat_formatting(command, self.root_document_view, origin);
        let command = if self.script_note_body_owns_input()
            && matches!(command, Command::EditorStyleToggle { surface: EditorSurface::Body, .. }
                | Command::EditorStyleClear { surface: EditorSurface::Body }
                | Command::EditorRunFormattingSet { surface: EditorSurface::Body, .. })
        {
            let Some(modified) = current_local_timestamp_value() else {
                self.post_diagnostic("ScriptNote timestamp could not be represented; note unchanged");
                self.flush_pending_diagnostic_commands(ctx);
                return DesktopCommandResult { command: command_id, origin,
                    outcome: DesktopCommandOutcome::Rejected(CommandRejection::InvalidScriptNoteValue) };
            };
            route_to_script_note_formatting(command, modified)
        } else { command };
        let outcome = self.dispatch_command_inner_with_pdf_handlers(
            command,
            ctx,
            export_pdf,
            resolve_pdf_options,
            open_print_panel,
        );
        self.finish_script_note_follow_up(
            &dispatched_command, &outcome, note_ids_before_create, ctx,
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

    fn finish_script_note_follow_up(
        &mut self,
        command: &Command,
        outcome: &DesktopCommandOutcome,
        note_ids_before_create: Option<Vec<scriptcraft_model::StableId>>,
        ctx: &egui::Context,
    ) {
        match command {
            Command::ScriptNoteCreate { .. }
                if matches!(outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed)) =>
            {
                let before = note_ids_before_create.unwrap_or_default();
                let added = self.session.project().script_notes().iter()
                    .filter(|note| !before.contains(&note.id)).map(|note| note.id.clone())
                    .collect::<Vec<_>>();
                if let [note] = added.as_slice() {
                    let present = Command::ScriptNotePresent { presentation: ScriptNotePresentValue {
                        note: ScriptNoteIdValue::from_core(note.clone()),
                        move_caret: false,
                        focus_body: true,
                    }};
                    if !matches!(self.dispatch_internal_command(present, ctx),
                        DesktopCommandOutcome::EffectDispatched)
                    {
                        self.post_diagnostic("ScriptNote was created but could not be presented safely");
                    }
                } else {
                    self.post_diagnostic("ScriptNote was created but its new identity could not be established safely");
                }
            }
            Command::ScriptNotePropertiesSet { properties }
                if matches!(outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)) =>
            {
                if let Some(note) = self.session.project().script_notes().get(properties.note.as_core()) {
                    self.script_note_workspace.update_defaults(
                        crate::widgets::script_note::properties::resolve_color(
                            note.color_projection(),
                            self.theme_palette.script_workspace.page.text,
                        ).default_color,
                        note.note_type.clone(),
                    );
                }
            }
            Command::ScriptNoteDelete { note }
                if matches!(outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed))
                    && self.script_note_workspace.open_note.as_ref() == Some(note.as_core()) =>
            {
                self.script_note_workspace.dismiss();
                ctx.request_repaint();
            }
            _ => {}
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
            if self.development_mode && is_alt_command(command.id()) {
                self.responsiveness.record_alt_availability_rejection();
            }
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
            | Command::FileExportFountain
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
            | Command::ViewInvisiblesToggle
            | Command::ViewRulerToggle) => self.dispatch_view_command(command, ctx),
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
            | Command::EditorRunFormattingSet { .. }
            | Command::EditorParagraphFormattingSet { .. }
            | Command::EditorDualDialogueToggle
            | Command::EditorDialogueAltAdd
            | Command::EditorDialogueAltDelete
            | Command::EditorDialogueAltNavigate { .. }
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
            | Command::EditorSelectionSet { .. }
            | Command::BeatBoardCardCreate { .. }
            | Command::BeatBoardCardActivate { .. }
            | Command::BeatBoardCardGeometrySet { .. }
            | Command::BeatBoardCardDelete { .. }
            | Command::BeatBoardSelectionSet { .. }
            | Command::BeatBoardTextEdit { .. }
            | Command::BeatBoardNavigation { .. }
            | Command::BeatBoardStyleToggle { .. }
            | Command::BeatBoardStyleClear
            | Command::BeatBoardRunFormattingSet { .. }) => self.dispatch_editing_command(command, ctx),
            command @ (Command::ScriptNoteCreate { .. }
            | Command::ScriptNoteDelete { .. }
            | Command::ScriptNotePropertiesSet { .. }
            | Command::ScriptNoteSelectionSet { .. }
            | Command::ScriptNoteTextEdit { .. }
            | Command::ScriptNoteTextNavigate { .. }
            | Command::ScriptNoteStyleToggle { .. }
            | Command::ScriptNoteStyleClear { .. }
            | Command::ScriptNoteRunFormattingSet { .. }) => self.dispatch_editing_command(command, ctx),
            command @ (Command::ScriptNotePresent { .. } | Command::ScriptNoteDismiss) => {
                self.dispatch_script_note_host_command(command, ctx)
            }
            command @ (Command::EditorSelectionPointer { .. }
            | Command::EditorNavigationHorizontal { .. }
            | Command::EditorNavigationVertical { .. }
            | Command::EditorNavigationLineEdge { .. }
            | Command::EditorFocus) => self.dispatch_navigation_command(command, ctx),
            command @ (Command::ViewZoomSet { .. }
            | Command::ViewZoomPinch { .. }
            | Command::ViewBeatBoardShow
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

fn is_alt_command(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::EditorDialogueAltAdd
            | CommandId::EditorDialogueAltDelete
            | CommandId::EditorDialogueAltNavigate
    )
}

fn route_to_script_note_formatting(
    command: Command,
    modified: ScriptNoteTimestampValue,
) -> Command {
    match command {
        Command::EditorStyleToggle { surface: EditorSurface::Body, style } =>
            Command::ScriptNoteStyleToggle { style, modified },
        Command::EditorStyleClear { surface: EditorSurface::Body } =>
            Command::ScriptNoteStyleClear { modified },
        Command::EditorRunFormattingSet { surface: EditorSurface::Body, patch } =>
            Command::ScriptNoteRunFormattingSet { patch, modified },
        other => other,
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
        Command::EditorStyleToggle { style, .. } => Command::EditorStyleToggle { surface, style },
        Command::EditorStyleClear { .. } => Command::EditorStyleClear { surface },
        Command::EditorAlignmentSet { alignment, .. } => {
            Command::EditorAlignmentSet { surface, alignment }
        }
        // These new formatting commands intentionally retain their explicit
        // Body target rather than following the focused-surface rewrite.
        Command::EditorRunFormattingSet { surface, patch } => {
            Command::EditorRunFormattingSet { surface, patch }
        }
        Command::EditorParagraphFormattingSet { surface, patch } => {
            Command::EditorParagraphFormattingSet { surface, patch }
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

/// Existing shared menu/toolbar intents are translated only while the Beat
/// root owns the workspace. Script and Title continue through their original
/// command variants and executor paths.
fn route_to_beat_formatting(
    command: Command,
    root: crate::app::RootDocumentView,
    origin: CommandOrigin,
) -> Command {
    if root != crate::app::RootDocumentView::BeatBoard
        || !matches!(origin, CommandOrigin::Menu | CommandOrigin::Toolbar | CommandOrigin::NativeMenu)
    {
        return command;
    }
    match command {
        Command::EditorStyleToggle { surface: EditorSurface::Body, style } =>
            Command::BeatBoardStyleToggle { style },
        Command::EditorStyleClear { surface: EditorSurface::Body } =>
            Command::BeatBoardStyleClear,
        Command::EditorRunFormattingSet { surface: EditorSurface::Body, patch } =>
            Command::BeatBoardRunFormattingSet { patch },
        other => other,
    }
}

fn should_dismiss_script_note_for(command: CommandId) -> bool {
    matches!(command,
        CommandId::FileNew | CommandId::FileOpenRequest | CommandId::FileOpen
        | CommandId::FileClose | CommandId::FileSave | CommandId::FileSaveAsRequest
        | CommandId::FileSaveAs | CommandId::FileNewFromTemplateOpen
        | CommandId::WindowTitlePageOpen | CommandId::ViewBeatBoardShow
        | CommandId::FormatElementsOpen | CommandId::FormatTemplateSelectorOpen
        | CommandId::FileSaveAsPdf | CommandId::FileExportFountain | CommandId::FilePrint
        | CommandId::PdfIncludeTitlePageResolve
    )
}

/// Convert the shared local-wall-clock provider into the bounded command value.
pub(crate) fn current_local_timestamp_value() -> Option<ScriptNoteTimestampValue> {
    ScriptNoteTimestampValue::new(scriptcraft_fdx::current_timestamp()).ok()
}
