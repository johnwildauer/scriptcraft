use super::{EditingSession, LayoutCache, Project, ScriptCraftApp, Selection};
use eframe::egui;
use scriptcraft_commands::{
    execute_editor_command, Appearance, Command, CommandId, CommandOrigin, CommandOutcome,
    CommandRejection, ImportResolution, PointerPhase, Position as CommandPosition,
    Selection as CommandSelection, SprintTimerMode, UnavailableReason,
};
use super::sprint_timer::NotificationSink;
use std::time::Instant;

/// An activation plus its diagnostic source. Origins never change execution
/// semantics; adapters assign the origin as they create the invocation.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CommandInvocation {
    pub(super) command: Command,
    pub(super) origin: CommandOrigin,
}

impl CommandInvocation {
    pub(super) const fn new(command: Command, origin: CommandOrigin) -> Self {
        Self { command, origin }
    }
}

/// Static menu and toolbar catalogs contain only parameterless stable IDs.
/// Dynamic payloads (such as a toolbar Role) are constructed at activation.
pub(super) fn command_for_id(id: CommandId) -> Option<Command> {
    Some(match id {
        CommandId::FileNew => Command::FileNew,
        CommandId::FileOpenRequest => Command::FileOpenRequest,
        CommandId::FileClose => Command::FileClose,
        CommandId::FileSave => Command::FileSave,
        CommandId::FileSaveAsRequest => Command::FileSaveAsRequest,
        CommandId::FileSaveAsPdf => Command::FileSaveAsPdf,
        CommandId::EditUndo => Command::EditUndo,
        CommandId::EditRedo => Command::EditRedo,
        CommandId::EditCut => Command::EditCut,
        CommandId::EditCopy => Command::EditCopy,
        CommandId::EditPaste => Command::EditPaste,
        CommandId::EditDelete => Command::EditDelete,
        CommandId::EditSelectAll => Command::EditSelectAll,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AppCommandAvailability {
    pub(super) modal_blocked: bool,
    pub(super) undo: bool,
    pub(super) redo: bool,
    pub(super) delete: bool,
    pub(super) select_all: bool,
    pub(super) cut: bool,
    pub(super) copy: bool,
    pub(super) paste: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DesktopUnavailableReason {
    ModalBlocked,
    UndoHistory,
    RedoHistory,
    DeleteTarget,
    Selection,
}

/// Typed result of the desktop dispatch boundary. Effect handlers remain the
/// existing desktop adapters; `EffectDispatched` does not claim their external
/// operation succeeded (their existing status/error channel remains authoritative).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DesktopCommandOutcome {
    EffectDispatched,
    Editor(CommandOutcome),
    Unavailable(DesktopUnavailableReason),
    Rejected(CommandRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DesktopCommandResult {
    pub(super) command: CommandId,
    pub(super) origin: CommandOrigin,
    pub(super) outcome: DesktopCommandOutcome,
}

impl ScriptCraftApp {
    /// Returns egui focus to the one screenplay interaction surface after an
    /// Edit command, without making the command target depend on popup focus.
    pub(super) fn restore_editor_focus(&mut self, ctx: &egui::Context) {
        if let Some(editor_id) = self.editor_surface_id {
            ctx.memory_mut(|memory| memory.request_focus(editor_id));
            ctx.request_repaint();
        }
    }

    pub(super) fn app_command_availability(&self) -> AppCommandAvailability {
        let migration_pending = self.pending_fdx_open.is_some();
        let selection = self.session.selection();
        let delete_target_exists = if selection.anchor != selection.focus {
            true
        } else {
            self.session
                .project()
                .elements
                .get(selection.focus.element)
                .is_some_and(|element| {
                    selection.focus.offset < element.text.len()
                        || (element.role == super::Role::Parenthetical
                            && matches!(element.text.as_str(), "(" | "()" | ")"))
                })
        };

        AppCommandAvailability {
            modal_blocked: migration_pending,
            undo: !migration_pending && self.session.can_undo(),
            redo: !migration_pending && self.session.can_redo(),
            delete: !migration_pending && delete_target_exists,
            // EditingSession projects a document with at least one body element.
            select_all: !migration_pending && !self.session.project().elements.is_empty(),
            cut: !migration_pending && selection.anchor != selection.focus,
            copy: !migration_pending && selection.anchor != selection.focus,
            // Clipboard state is read only when Paste is activated, never per frame.
            paste: !migration_pending,
        }
    }

    /// The sole desktop activation boundary. It refreshes availability at the
    /// moment of dispatch and keeps platform/document effects behind their
    /// existing adapters while semantic editing delegates to WO-002's executor.
    pub(super) fn dispatch_command(
        &mut self,
        invocation: CommandInvocation,
        ctx: &egui::Context,
    ) -> DesktopCommandResult {
        let command_id = invocation.command.id();
        let origin = invocation.origin;
        let restore_focus = matches!(
            command_id,
            CommandId::EditUndo
                | CommandId::EditRedo
                | CommandId::EditCut
                | CommandId::EditCopy
                | CommandId::EditPaste
                | CommandId::EditDelete
                | CommandId::EditSelectAll
                | CommandId::EditorTextInsert
                | CommandId::EditorTextPaste
                | CommandId::EditorDeleteBackward
                | CommandId::EditorParagraphEnter
                | CommandId::EditorParagraphTab
                | CommandId::EditorElementChange
        );
        let outcome = self.dispatch_command_inner(invocation.command, ctx);
        if restore_focus {
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
        let availability = self.app_command_availability();
        if !availability.is_enabled(command.id()) {
            return DesktopCommandOutcome::Unavailable(
                availability.unavailable_reason(command.id()),
            );
        }

        match command {
            Command::FileNew => {
                match Project::new()
                    .map_err(|error| format!("new project failed: {error:?}"))
                    .and_then(|project| {
                        EditingSession::with_policy(project, self.session.policy())
                            .map_err(|error| format!("session failed: {error:?}"))
                    }) {
                    Ok(session) => {
                        self.session = session;
                        self.page_geometry.reset_for_document_replacement();
                        self.layout_cache = LayoutCache::default();
                        self.surface_focused = false;
                        self.native_path = None;
                        self.fdx_baseline = None;
                        self.pending_fdx_open = None;
                        self.next_cast_list_refresh =
                            Instant::now() + super::CAST_LIST_REFRESH_INTERVAL;
                        self.rendered_revision = None;
                        self.status = "New in-memory project".into();
                        self.clear_error();
                    }
                    Err(error) => self.show_error(error),
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileOpenRequest => {
                self.open_native(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileOpen { path } => {
                self.open_native_at(path.as_path());
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileClose => {
                DesktopViewportEffects::close(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSave => {
                self.save_native(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSaveAsRequest => {
                self.save_as_native(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSaveAs { path } => {
                self.save_fdx_as_at(path.as_path());
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileImportResolve { resolution } => {
                self.resolve_pending_fdx_import(match resolution {
                    ImportResolution::Continue => super::FdxMigrationConsent::Continue,
                    ImportResolution::Cancel => super::FdxMigrationConsent::Cancel,
                });
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSaveAsPdf => {
                self.export_pdf();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::EditUndo => self.execute_editor(
                &Command::EditUndo,
                "Undid last edit",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditRedo => self.execute_editor(
                &Command::EditRedo,
                "Redid last edit",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditDelete => self.execute_editor(
                &Command::EditDelete,
                "Deleted forward",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorTextInsert { text } => self.execute_editor(
                &Command::EditorTextInsert { text },
                "Edited through EditingSession",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorTextPaste { text } => self.execute_editor(
                &Command::EditorTextPaste { text },
                "Edited through EditingSession",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorDeleteBackward => self.execute_editor(
                &Command::EditorDeleteBackward,
                "Edited through EditingSession",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorParagraphEnter => self.execute_editor(
                &Command::EditorParagraphEnter,
                "Edited through EditingSession",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorParagraphTab => self.execute_editor(
                &Command::EditorParagraphTab,
                "Edited through EditingSession",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditSelectAll => self.execute_editor(
                &Command::EditSelectAll,
                "Selected all screenplay text",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorElementChange { role } => self.execute_editor(
                &Command::EditorElementChange { role },
                "Changed focused element role",
                super::RevealIntent::None,
                ctx,
            ),
            Command::EditorSelectionSet { selection } => self.execute_editor(
                &Command::EditorSelectionSet { selection },
                "Edited through EditingSession",
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorSelectionPointer { position, phase } => {
                self.dispatch_pointer_selection(position, phase, ctx)
            }
            Command::EditorNavigationHorizontal { direction, extend } => {
                let previous_vertical_goal = self.preferred_vertical_x;
                let selection = self.navigation_selection_horizontal(direction, extend);
                let outcome = self.execute_navigation_selection(selection, false, ctx);
                if matches!(&outcome, DesktopCommandOutcome::Rejected(_)) {
                    self.preferred_vertical_x = previous_vertical_goal;
                }
                outcome
            }
            Command::EditorNavigationVertical { direction, extend } => {
                let previous_vertical_goal = self.preferred_vertical_x;
                match self.navigation_selection_vertical(direction, extend) {
                    Ok(selection) => {
                        let outcome = self.execute_navigation_selection(selection, true, ctx);
                        if matches!(&outcome, DesktopCommandOutcome::Rejected(_)) {
                            self.preferred_vertical_x = previous_vertical_goal;
                        }
                        outcome
                    }
                    Err(error) => {
                        self.show_edit_error(error.clone());
                        ctx.request_repaint();
                        DesktopCommandOutcome::Editor(CommandOutcome::Failed(error))
                    }
                }
            }
            Command::EditorNavigationLineEdge { edge, extend } => {
                let previous_vertical_goal = self.preferred_vertical_x;
                let selection = self.navigation_selection_line_edge(edge, extend);
                let outcome = self.execute_navigation_selection(selection, false, ctx);
                if matches!(&outcome, DesktopCommandOutcome::Rejected(_)) {
                    self.preferred_vertical_x = previous_vertical_goal;
                }
                outcome
            }
            Command::EditorFocus => {
                self.restore_editor_focus(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewZoomSet { percent } => {
                let next = percent.get();
                if next != self.zoom_percent {
                    self.capture_pending_zoom_anchor(ctx);
                    self.zoom_percent = next;
                    self.sync_pinch_zoom_from_controls();
                    self.page_geometry.invalidate();
                    self.page_scale = None;
                    ctx.request_repaint();
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewZoomPinch { factor } => {
                if let Some(next) =
                    super::status_bar::zoom_after_native_factor(self.pinch_zoom_percent, factor.get())
                {
                    let next_zoom = super::status_bar::normalized_zoom_percent(next);
                    if next_zoom != self.zoom_percent {
                        self.capture_pending_zoom_anchor(ctx);
                        self.zoom_percent = next_zoom;
                        self.page_geometry.invalidate();
                        self.page_scale = None;
                        ctx.request_repaint();
                    }
                    self.pinch_zoom_percent = next;
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewAppearanceSet { appearance } => {
                let appearance = match appearance {
                    Appearance::System => super::theme::AppearanceMode::System,
                    Appearance::Day => super::theme::AppearanceMode::Day,
                    Appearance::Night => super::theme::AppearanceMode::Night,
                    Appearance::Midnight => super::theme::AppearanceMode::Midnight,
                };
                if self.appearance_mode != appearance {
                    self.appearance_mode = appearance;
                    ctx.request_repaint();
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewFocusModeToggle => {
                if let Some(request) = self.focus_mode.request(self.timer_now()) {
                    DesktopViewportEffects::fullscreen(ctx, request.target);
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerTogglePopup => {
                self.sprint_timer.popup_open = !self.sprint_timer.popup_open;
                if self.sprint_timer.popup_open {
                    self.sprint_timer.just_opened = true;
                } else {
                    self.sprint_timer.close_popup();
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerClosePopup => {
                self.sprint_timer.popup_open = false;
                self.sprint_timer.close_popup();
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerSetMode { mode } => {
                let mode = match mode {
                    SprintTimerMode::Countdown => super::sprint_timer::TimerMode::Countdown,
                    SprintTimerMode::Stopwatch => super::sprint_timer::TimerMode::Stopwatch,
                };
                self.sprint_timer.select_mode(mode);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerSetMinutes { text } => {
                self.sprint_timer.set_minutes_input(text);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerStart => {
                let now = self.timer_now();
                let notification = match self.sprint_timer.mode {
                    super::sprint_timer::TimerMode::Countdown => {
                        self.sprint_timer.start_countdown(now)
                    }
                    super::sprint_timer::TimerMode::Stopwatch => {
                        self.sprint_timer.start_stopwatch(now)
                    }
                };
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerPause => {
                let notification = self.sprint_timer.pause(self.timer_now());
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerResume => {
                let notification = self.sprint_timer.resume(self.timer_now());
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerReset => {
                self.sprint_timer.reset();
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerEnd => {
                let notification = self.sprint_timer.end(self.timer_now());
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::EditCopy => {
                match canonical_selection_text(self.session.project(), self.session.selection()) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
                            self.status = "Copied selection to clipboard".into();
                            self.clear_error();
                        }
                        Err(ClipboardAccessError::Unavailable) => self.set_clipboard_notice(
                            "Clipboard write failed; screenplay unchanged",
                        ),
                    },
                    None => self.set_clipboard_notice(
                        "Copy unavailable: selection could not be read; screenplay unchanged",
                    ),
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::EditCut => {
                match canonical_selection_text(self.session.project(), self.session.selection()) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
                            let outcome = self.dispatch_internal_command(Command::EditDelete, ctx);
                            if matches!(
                                &outcome,
                                DesktopCommandOutcome::Editor(
                                    CommandOutcome::Changed | CommandOutcome::NoOp
                                )
                            ) {
                                self.status = "Cut selection to clipboard".into();
                                self.clear_error();
                            }
                            outcome
                        }
                        Err(ClipboardAccessError::Unavailable) => {
                            self.set_clipboard_notice(
                                "Clipboard write failed; screenplay unchanged",
                            );
                            ctx.request_repaint();
                            DesktopCommandOutcome::EffectDispatched
                        }
                    },
                    None => {
                        self.set_clipboard_notice(
                            "Cut unavailable: selection could not be read; screenplay unchanged",
                        );
                        ctx.request_repaint();
                        DesktopCommandOutcome::EffectDispatched
                    }
                }
            }
            Command::EditPaste => match DesktopClipboard::read_text() {
                Ok(text) if text.is_empty() => {
                    self.set_clipboard_notice(
                        "Paste unavailable: clipboard text is empty; screenplay unchanged",
                    );
                    ctx.request_repaint();
                    DesktopCommandOutcome::EffectDispatched
                }
                Ok(text) => {
                    let outcome = self.dispatch_internal_command(
                        Command::EditorTextPaste { text },
                        ctx,
                    );
                    if matches!(
                        &outcome,
                        DesktopCommandOutcome::Editor(
                            CommandOutcome::Changed | CommandOutcome::NoOp
                        )
                    ) {
                        self.status = "Pasted clipboard text".into();
                        self.clear_error();
                    }
                    outcome
                }
                Err(ClipboardAccessError::Unavailable) => {
                    self.set_clipboard_notice(
                        "Paste unavailable: clipboard has no readable text; screenplay unchanged",
                    );
                    ctx.request_repaint();
                    DesktopCommandOutcome::EffectDispatched
                }
            },
            other => DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(
                other.id(),
            )),
        }
    }

    fn execute_editor(
        &mut self,
        command: &Command,
        success: &str,
        reveal_intent: super::RevealIntent,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        let outcome = execute_editor_command(&mut self.session, command);
        match &outcome {
            CommandOutcome::Changed | CommandOutcome::NoOp => {
                self.mutate(Ok(()), success, reveal_intent);
            }
            CommandOutcome::Unavailable(_) | CommandOutcome::Rejected(_) => {}
            CommandOutcome::Failed(error) => self.show_edit_error(error.clone()),
        }
        ctx.request_repaint();
        match outcome {
            CommandOutcome::Unavailable(reason) => {
                DesktopCommandOutcome::Unavailable(match reason {
                    UnavailableReason::UndoHistory => DesktopUnavailableReason::UndoHistory,
                    UnavailableReason::RedoHistory => DesktopUnavailableReason::RedoHistory,
                    UnavailableReason::Selection => DesktopUnavailableReason::Selection,
                })
            }
            CommandOutcome::Rejected(rejection) => DesktopCommandOutcome::Rejected(rejection),
            semantic_outcome => DesktopCommandOutcome::Editor(semantic_outcome),
        }
    }

    fn dispatch_internal_command(
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

    pub(super) fn restore_imported_window(
        &self,
        commands: [egui::ViewportCommand; 2],
        ctx: &egui::Context,
    ) {
        DesktopViewportEffects::send(ctx, commands);
    }

    fn execute_navigation_selection(
        &mut self,
        selection: super::Selection,
        retain_vertical_goal: bool,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        let command_selection = match CommandSelection::from_core(selection) {
            Ok(selection) => selection,
            Err(error) => {
                return DesktopCommandOutcome::Rejected(CommandRejection::PositionOutOfRange(
                    error,
                ));
            }
        };
        let preferred_vertical_x = self.preferred_vertical_x;
        let outcome = self.execute_editor(
            &Command::EditorSelectionSet {
                selection: command_selection,
            },
            "Edited through EditingSession",
            super::RevealIntent::Conditional,
            ctx,
        );
        if retain_vertical_goal
            && matches!(
                &outcome,
                DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)
            )
        {
            self.preferred_vertical_x = preferred_vertical_x;
        }
        outcome
    }

    fn dispatch_pointer_selection(
        &mut self,
        position: CommandPosition,
        phase: PointerPhase,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        let core_position = match position.to_core() {
            Ok(position) => position,
            Err(error) => {
                return DesktopCommandOutcome::Rejected(CommandRejection::PositionOutOfRange(
                    error,
                ));
            }
        };
        let collapsed = CommandSelection {
            anchor: position,
            focus: position,
        };
        if let Err(error) = collapsed.validate_for(self.session.project()) {
            return DesktopCommandOutcome::Rejected(CommandRejection::InvalidSelection(error));
        }
        let current = self.session.selection();
        let desired = match phase {
            PointerPhase::Press => super::Selection::caret(core_position),
            PointerPhase::Drag => super::Selection {
                anchor: self.drag_anchor.unwrap_or(current.anchor),
                focus: core_position,
            },
            PointerPhase::Release => super::Selection::caret(core_position),
        };
        let success_status = self.status.clone();
        let outcome = self.execute_navigation_selection(desired, false, ctx);
        if matches!(
            &outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)
        ) {
            match phase {
                PointerPhase::Press => self.drag_anchor = Some(core_position),
                PointerPhase::Drag => {}
                PointerPhase::Release => self.drag_anchor = None,
            }
            self.preferred_vertical_x = None;
            self.rendered_revision = None;
            // Pointer selection is authoritative: it must not inherit a reveal
            // request queued by an earlier edit or navigation command.
            self.reveal_intent = super::RevealIntent::None;
            self.status = success_status;
            self.clear_error();
        }
        outcome
    }

    fn set_clipboard_notice(&mut self, message: &str) {
        self.status = message.into();
        self.error = Some(message.into());
    }

    pub(super) fn request_timer_notification(
        &mut self,
        notification: Option<super::sprint_timer::CompletionNotification>,
    ) {
        if let Some(notification) = notification {
            let result = DesktopTimerNotificationEffects::request(
                &mut self.notification_sink,
                notification,
            );
            DesktopTimerNotificationEffects::apply_result(
                &mut self.sprint_timer,
                notification.generation,
                result,
            );
        }
    }

    pub(super) fn apply_timer_notification_result(
        &mut self,
        generation: u64,
        result: super::sprint_timer::NotificationResult,
    ) -> bool {
        DesktopTimerNotificationEffects::apply_result(
            &mut self.sprint_timer,
            generation,
            result,
        )
    }
}

impl AppCommandAvailability {
    /// One command-state projection consumed by both menu presentations and
    /// checked again at dispatch time to reject stale or modal-blocked input.
    pub(super) const fn is_enabled(self, command: CommandId) -> bool {
        if matches!(command, CommandId::FileImportResolve) {
            return self.modal_blocked;
        }
        if self.modal_blocked {
            return false;
        }
        match command {
            CommandId::EditUndo => self.undo,
            CommandId::EditRedo => self.redo,
            CommandId::EditDelete => self.delete,
            CommandId::EditSelectAll => self.select_all,
            CommandId::EditCut => self.cut,
            CommandId::EditCopy => self.copy,
            CommandId::EditPaste => self.paste,
            _ => true,
        }
    }

    const fn unavailable_reason(self, command: CommandId) -> DesktopUnavailableReason {
        if self.modal_blocked {
            return DesktopUnavailableReason::ModalBlocked;
        }
        match command {
            CommandId::EditUndo => DesktopUnavailableReason::UndoHistory,
            CommandId::EditRedo => DesktopUnavailableReason::RedoHistory,
            CommandId::EditDelete => DesktopUnavailableReason::DeleteTarget,
            CommandId::EditSelectAll | CommandId::EditCut | CommandId::EditCopy => {
                DesktopUnavailableReason::Selection
            }
            _ => DesktopUnavailableReason::ModalBlocked,
        }
    }
}

/// The viewport adapter owns native close/restore requests; imported geometry
/// never becomes semantic screenplay state or a command payload.
struct DesktopViewportEffects;

impl DesktopViewportEffects {
    fn close(ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn fullscreen(ctx: &egui::Context, target: bool) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(target));
    }

    fn send(ctx: &egui::Context, commands: [egui::ViewportCommand; 2]) {
        for command in commands {
            ctx.send_viewport_cmd(command);
        }
    }
}

/// Completion notification requests and asynchronous results remain outside
/// command payloads; this adapter preserves the existing native sink contract.
struct DesktopTimerNotificationEffects;

impl DesktopTimerNotificationEffects {
    fn request(
        sink: &mut super::sprint_timer::NativeNotificationSink,
        notification: super::sprint_timer::CompletionNotification,
    ) -> super::sprint_timer::NotificationResult {
        sink.request(notification)
    }

    fn apply_result(
        timer: &mut super::sprint_timer::SprintTimer,
        generation: u64,
        result: super::sprint_timer::NotificationResult,
    ) -> bool {
        timer.apply_notification_result(generation, result)
    }
}

#[derive(Debug, Clone, Copy)]
enum ClipboardAccessError {
    Unavailable,
}

/// The single desktop-owned boundary for explicit system clipboard access.
struct DesktopClipboard;

impl DesktopClipboard {
    fn write_text(text: &str) -> Result<(), ClipboardAccessError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|_| ClipboardAccessError::Unavailable)?;
        clipboard
            .set_text(text.to_owned())
            .map_err(|_| ClipboardAccessError::Unavailable)
    }

    fn read_text() -> Result<String, ClipboardAccessError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|_| ClipboardAccessError::Unavailable)?;
        clipboard
            .get_text()
            .map_err(|_| ClipboardAccessError::Unavailable)
    }
}

/// Extracts the ordered selection directly from canonical body-element UTF-8.
/// Each crossed element boundary contributes exactly one newline; content is
/// otherwise copied byte-for-byte without export or layout transformations.
fn canonical_selection_text(project: &Project, selection: Selection) -> Option<String> {
    let (start, end) = if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    };
    if start == end {
        return None;
    }

    let start_element = project.elements.get(start.element)?;
    if start.element == end.element {
        return Some(start_element.text.get(start.offset..end.offset)?.to_owned());
    }

    let mut text = String::new();
    text.push_str(start_element.text.get(start.offset..)?);
    for element_index in (start.element + 1)..=end.element {
        text.push('\n');
        let element = project.elements.get(element_index)?;
        if element_index == end.element {
            text.push_str(element.text.get(..end.offset)?);
        } else {
            text.push_str(&element.text);
        }
    }
    Some(text)
}
