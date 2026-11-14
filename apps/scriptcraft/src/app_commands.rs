use super::{LayoutCache, Position, Project, ScriptCraftApp, Selection};
use super::document_lifecycle::SaveOutcome;
use eframe::egui;
use scriptcraft_core::editing::EditError;
use scriptcraft_commands::{
    execute_editor_command_on_surface, pointer_selection, Appearance, Command, CommandId, CommandOrigin, CommandOutcome,
    CommandRejection, DiagnosticMessage, EditorSurface, HorizontalDirection, HostEffectDescription,
    ImportResolution, LineEdge, PointerPhase, VerticalDirection,
    Position as CommandPosition, Selection as CommandSelection, SprintTimerMode,
    UnavailableReason, MAX_DIAGNOSTIC_MESSAGE_BYTES,
};
use super::sprint_timer::NotificationSink;
use std::collections::VecDeque;

fn close_sprint_timer_popup_once(timer: &mut super::sprint_timer::SprintTimer) -> bool {
    if !timer.popup_open {
        return false;
    }
    timer.popup_open = false;
    timer.close_popup();
    true
}

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

/// Static menu, toolbar, and native application-menu IDs map to parameterless
/// commands. Dynamic payloads (such as a toolbar Role) are built at activation.
pub(super) fn command_for_id(id: CommandId) -> Option<Command> {
    Some(match id {
        CommandId::FileNew => Command::FileNew,
        CommandId::FileOpenRequest => Command::FileOpenRequest,
        CommandId::FileClose => Command::FileClose,
        CommandId::FileSave => Command::FileSave,
        CommandId::FileSaveAsRequest => Command::FileSaveAsRequest,
        CommandId::FileSaveAsPdf => Command::FileSaveAsPdf,
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
        CommandId::EditorStyleClear => Command::EditorStyleClear { surface: EditorSurface::Body },
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AppCommandAvailability {
    pub(super) modal_blocked: bool,
    pub(super) native_output_active: bool,
    /// Replacement handoff lock; deliberately independent from the migration
    /// modal that alone enables File Import Resolve.
    pub(super) replacement_pending: bool,
    pub(super) title_page_only_close: bool,
    pub(super) undo: bool,
    pub(super) redo: bool,
    pub(super) delete: bool,
    pub(super) select_all: bool,
    pub(super) cut: bool,
    pub(super) copy: bool,
    pub(super) paste: bool,
    pub(super) body_only: bool,
    pub(super) dual_dialogue_toggle: bool,
    pub(super) title_open: bool,
    pub(super) title_close: bool,
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
    /// The save handler completed synchronously; routing or picker launch alone
    /// must never be represented as a successful save.
    SaveCompleted(SaveOutcome),
    HostEffectRequested(HostEffectDescription),
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
    fn active_selection(&self) -> super::Selection {
        match self.active_editor_surface() {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        }
    }

    fn navigation_selection_horizontal_for(
        &mut self,
        surface: EditorSurface,
        direction: HorizontalDirection,
        extend: bool,
    ) -> super::Selection {
        if surface == EditorSurface::Body {
            return self.navigation_selection_horizontal(direction, extend);
        }
        self.set_preferred_vertical_x_for(surface, None);
        let current = self.session.title_selection();
        let (start, end) = if (current.anchor.element, current.anchor.offset)
            <= (current.focus.element, current.focus.offset)
        {
            (current.anchor, current.focus)
        } else {
            (current.focus, current.anchor)
        };
        let focus = if !extend && start != end {
            match direction {
                HorizontalDirection::Left => start,
                HorizontalDirection::Right => end,
            }
        } else {
            current.focus
        };
        let Some(paragraph) = self.session.project().title_page.get(focus.element) else {
            return current;
        };
        let next = match direction {
            HorizontalDirection::Left if focus.offset > 0 => {
                let offset = paragraph.text[..focus.offset].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                Position { element: focus.element, offset }
            }
            HorizontalDirection::Left if focus.element > 0 => {
                let previous = &self.session.project().title_page[focus.element - 1];
                Position { element: focus.element - 1, offset: previous.text.len() }
            }
            HorizontalDirection::Right if focus.offset < paragraph.text.len() => {
                let amount = paragraph.text[focus.offset..].chars().next().map(char::len_utf8).unwrap_or(0);
                Position { element: focus.element, offset: focus.offset + amount }
            }
            HorizontalDirection::Right if focus.element + 1 < self.session.project().title_page.len() => {
                Position { element: focus.element + 1, offset: 0 }
            }
            _ => focus,
        };
        if extend {
            super::Selection { anchor: current.anchor, focus: next }
        } else {
            super::Selection::caret(next)
        }
    }

    fn navigation_selection_vertical_for(
        &mut self,
        surface: EditorSurface,
        direction: VerticalDirection,
        extend: bool,
        ctx: &egui::Context,
    ) -> Result<super::Selection, super::EditError> {
        if surface == EditorSurface::Body {
            return self.navigation_selection_vertical(direction, extend, ctx);
        }
        let current = self.session.title_selection();
        let focus = current.focus;
        let layout = self
            .layout_for_render(EditorSurface::TitlePage)
            .map_err(|_| EditError::InvalidElement)?;
        let Some(current_index) =
            layout.title_line_index_for_position(focus.element, focus.offset)
        else {
            self.set_preferred_vertical_x_for(surface, None);
            return Ok(current);
        };
        let Some(current_line) = layout.title_line(current_index) else {
            self.set_preferred_vertical_x_for(surface, None);
            return Ok(current);
        };
        let target_index = match direction {
            VerticalDirection::Up => current_index.checked_sub(1),
            VerticalDirection::Down => current_index.checked_add(1),
        };
        let Some(target_line) = target_index.and_then(|index| layout.title_line(index)) else {
            self.set_preferred_vertical_x_for(surface, None);
            let next = Position {
                element: current_line.element,
                offset: match direction {
                    VerticalDirection::Up => current_line.source.start,
                    VerticalDirection::Down => current_line.source.end,
                },
            };
            return Ok(if extend {
                super::Selection { anchor: current.anchor, focus: next }
            } else {
                super::Selection::caret(next)
            });
        };
        let scale = if self.title_view_state_swapped {
            self.page_scale.unwrap_or(1.0)
        } else {
            self.title_view.page_scale.unwrap_or(1.0)
        };
        let current_projection = self
            .project_line_for_interaction(ctx, current_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let target_projection = self
            .project_line_for_interaction(ctx, target_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let goal = self.preferred_vertical_x_for(surface).unwrap_or_else(|| {
            super::presentation_geometry::preferred_vertical_document_x(
                current_line,
                focus.offset,
                current_projection.as_ref(),
                scale,
            )
            .unwrap_or(current_line.geometry.x.raw())
        });
        self.set_preferred_vertical_x_for(surface, Some(goal));
        let display_boundary = if let Some(projection) = target_projection.as_ref() {
            super::presentation_geometry::projected_target_boundary(projection, goal, scale)
                .ok_or(EditError::InvalidElement)?
        } else {
            let cell = target_line.char_width.raw().max(1);
            (goal
                .saturating_sub(target_line.geometry.x.raw())
                .saturating_add(cell / 2)
                / cell)
                .max(0) as usize
        };
        let next = Position {
            element: target_line.element,
            offset: target_line.source_for_display_char(display_boundary),
        };
        Ok(if extend {
            super::Selection { anchor: current.anchor, focus: next }
        } else {
            super::Selection::caret(next)
        })
    }

    fn navigation_selection_line_edge_for(
        &mut self,
        surface: EditorSurface,
        edge: LineEdge,
        extend: bool,
    ) -> super::Selection {
        if surface == EditorSurface::Body {
            return self.navigation_selection_line_edge(edge, extend);
        }
        self.set_preferred_vertical_x_for(surface, None);
        let current = self.session.title_selection();
        let offset = match edge {
            LineEdge::Start => 0,
            LineEdge::End => self.session.project().title_page
                .get(current.focus.element).map(|paragraph| paragraph.text.len()).unwrap_or(0),
        };
        let focus = Position { element: current.focus.element, offset };
        if extend {
            super::Selection { anchor: current.anchor, focus }
        } else {
            super::Selection::caret(focus)
        }
    }

    /// Returns egui focus to the one screenplay interaction surface after an
    /// Edit command, without making the command target depend on popup focus.
    pub(super) fn restore_editor_focus(&mut self, ctx: &egui::Context) {
        if self.active_editor_surface == EditorSurface::TitlePage
            && !self.title_view_state_swapped
        {
            ctx.send_viewport_cmd_to(
                crate::title_page_window::viewport_id(),
                egui::ViewportCommand::Focus,
            );
            ctx.request_repaint();
            return;
        }
        let editor_id = match self.active_editor_surface {
            EditorSurface::Body => self.editor_surface_id,
            EditorSurface::TitlePage => self.title_editor_surface_id,
        };
        if let Some(editor_id) = editor_id {
            ctx.memory_mut(|memory| memory.request_focus(editor_id));
            ctx.request_repaint();
        }
    }

    pub(super) fn app_command_availability(&self) -> AppCommandAvailability {
        let migration_pending = self.pending_fdx_open.is_some();
        let replacement_pending = self.replacement_open_pending();
        #[cfg(target_os = "windows")]
        let native_output_active = self.native_output_modal_active
            || self.windows_print_preparation.is_some()
            || self.windows_print_session.is_some();
        #[cfg(not(target_os = "windows"))]
        let native_output_active = self.native_output_modal_active;
        let (selection, text_len, role_is_empty_parenthetical, element_count) = match self.active_editor_surface {
            EditorSurface::Body => {
                let selection = self.session.selection();
                let element = self.session.project().elements.get(selection.focus.element);
                (selection, element.map(|e| e.text.len()).unwrap_or(0), element.is_some_and(|e| e.role == super::Role::Parenthetical && matches!(e.text.as_str(), "(" | "()" | ")")), self.session.project().elements.len())
            }
            EditorSurface::TitlePage => {
                let selection = self.session.title_selection();
                let text_len = self.session.project().title_page.get(selection.focus.element).map(|p| p.text.len()).unwrap_or(0);
                (selection, text_len, false, self.session.project().title_page.len())
            }
        };
        let delete_target_exists = if selection.anchor != selection.focus {
            true
        } else {
            selection.focus.offset < text_len || role_is_empty_parenthetical
        };

        let dual_dialogue_target = self.dual_dialogue_toggle_target();
        AppCommandAvailability {
            modal_blocked: migration_pending && !replacement_pending,
            native_output_active,
            replacement_pending,
            title_page_only_close: self.active_editor_surface() == EditorSurface::TitlePage
                && self.title_page_window_open,
            undo: !migration_pending && !replacement_pending && self.session.can_undo(),
            redo: !migration_pending && !replacement_pending && self.session.can_redo(),
            delete: !migration_pending && !replacement_pending && delete_target_exists,
            // EditingSession projects a document with at least one body element.
            select_all: !migration_pending && !replacement_pending && element_count != 0,
            cut: !migration_pending && !replacement_pending && selection.anchor != selection.focus,
            copy: !migration_pending && !replacement_pending && selection.anchor != selection.focus,
            // Clipboard state is read only when Paste is activated, never per frame.
            paste: !migration_pending && !replacement_pending,
            body_only: self.active_editor_surface == EditorSurface::Body,
            dual_dialogue_toggle: !replacement_pending && dual_dialogue_target.is_some(),
            // In-app Open is a selected-path replacement request and is blocked
            // while the current document is resolving another lifecycle gate.
            title_open: !migration_pending && !replacement_pending && !self.title_viewport_unsupported,
            // Closing the presentation-only child viewport does not destroy
            // the root document and remains available during replacement.
            title_close: !migration_pending && self.title_page_window_open,
        }
    }

    pub(super) fn dual_dialogue_menu_should_say_undual(&self) -> bool {
        matches!(
            self.dual_dialogue_toggle_target(),
            Some(scriptcraft_core::dual_dialogue::ToggleTarget::Disable(_))
        )
    }

    fn dual_dialogue_toggle_target(
        &self,
    ) -> Option<scriptcraft_core::dual_dialogue::ToggleTarget> {
        (self.active_editor_surface == EditorSurface::Body)
            .then(|| self.session.dual_dialogue_toggle_target(
                scriptcraft_core::dual_dialogue::SelectionSurface::Body,
            ).ok().flatten())
            .flatten()
    }

    fn with_native_output_modal(&mut self, operation: impl FnOnce(&mut ScriptCraftApp)) {
        self.native_output_modal_active = true;
        operation(self);
        self.native_output_modal_active = false;
    }

    /// The sole desktop activation boundary. It refreshes availability at the
    /// moment of dispatch and keeps platform/document effects behind their
    /// existing adapters while semantic editing delegates to WO-002's executor.
    pub(super) fn dispatch_command(
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
                | CommandId::EditorSelectionSet
                | CommandId::EditorNavigationHorizontal
                | CommandId::EditorNavigationVertical
                | CommandId::EditorNavigationLineEdge
                | CommandId::EditorStyleToggle
                | CommandId::EditorStyleClear
                | CommandId::EditorAlignmentSet
                | CommandId::EditorDualDialogueToggle
        );
        let command = route_to_focused_surface(
            invocation.command,
            origin,
            self.active_editor_surface(),
        );
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
        let availability = self.app_command_availability();
        if !availability.is_enabled(command.id()) {
            if matches!(
                command.id(),
                CommandId::FileSave | CommandId::FileSaveAsRequest | CommandId::FileSaveAs
            ) {
                let reason = availability.unavailable_reason(command.id());
                self.show_error(format!(
                    "SAVE_DISPATCH_UNAVAILABLE: {reason:?}; document unchanged"
                ));
                return DesktopCommandOutcome::SaveCompleted(SaveOutcome::Failed);
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
            Command::FileNew => {
                self.launch_blank_instance();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileOpenRequest => {
                self.open_native(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileOpen { path } => {
                self.stage_replacement_document_path(path.as_path());
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileClose => {
                // Body Close is guarded at the next root-update boundary:
                // CancelClose -> native prompt -> one-shot confirmed Close.
                // Title Page remains a separate child-only viewport close.
                match file_close_target(self.active_editor_surface()) {
                    FileCloseTarget::TitlePage => self.close_title_page_window(ctx),
                    FileCloseTarget::Root => DesktopViewportEffects::close_root(ctx),
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ApplicationQuit => {
                // Application Quit always requests root Close, even when the
                // Title Page owns focus. The root-close coordinator performs
                // the ordinary asynchronous CancelClose/prompt/save gate.
                DesktopViewportEffects::close_root(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSave => {
                DesktopCommandOutcome::SaveCompleted(self.save_native(ctx))
            }
            Command::FileSaveAsRequest => {
                DesktopCommandOutcome::SaveCompleted(self.save_as_native(ctx))
            }
            Command::FileSaveAs { path, overwrite } => {
                DesktopCommandOutcome::SaveCompleted(
                    self.save_fdx_as_at(path.as_path(), overwrite),
                )
            }
            Command::FileImportResolve { resolution } => {
                let _outcome = self.resolve_pending_fdx_import(match resolution {
                    ImportResolution::Continue => super::FdxMigrationConsent::Continue,
                    ImportResolution::Cancel => super::FdxMigrationConsent::Cancel,
                });
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSaveAsPdf => {
                self.with_native_output_modal(export_pdf);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FilePrint => {
                #[cfg(target_os = "windows")]
                {
                    // Windows printing outlives this dispatch: preparation runs
                    // in the background and the owner-HWND session is started
                    // and polled from a later eframe frame. Keep the shared
                    // output gate raised until that asynchronous owner releases it.
                    self.native_output_modal_active = true;
                    open_print_panel(self);
                }
                #[cfg(not(target_os = "windows"))]
                self.with_native_output_modal(open_print_panel);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::WindowTitlePageOpen => {
                self.open_title_page_window(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::WindowTitlePageClose => {
                self.close_title_page_window(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::PdfIncludeTitlePageResolve { options } => {
                self.with_native_output_modal(|app| resolve_pdf_options(app, options));
                DesktopCommandOutcome::EffectDispatched
            }
            Command::EditorStyleToggle { surface, style } => self.execute_editor(
                &Command::EditorStyleToggle { surface, style },
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorStyleClear { surface } => self.execute_editor(
                &Command::EditorStyleClear { surface },
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorAlignmentSet { surface, alignment } => self.execute_editor(
                &Command::EditorAlignmentSet { surface, alignment },
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorDualDialogueToggle => self.execute_editor(
                &Command::EditorDualDialogueToggle,
                super::RevealIntent::None,
                ctx,
            ),
            Command::EditUndo => self.execute_editor(
                &Command::EditUndo,
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditRedo => self.execute_editor(
                &Command::EditRedo,
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditDelete => self.execute_editor(
                &Command::EditDelete,
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorTextInsert { text, surface } => self.execute_editor(
                &Command::EditorTextInsert { text, surface },
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorTextPaste { text, surface } => self.execute_editor(
                &Command::EditorTextPaste { text, surface },
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorDeleteBackward => self.execute_editor(
                &Command::EditorDeleteBackward,
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorParagraphEnter => self.execute_editor(
                &Command::EditorParagraphEnter,
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorParagraphTab => self.execute_editor(
                &Command::EditorParagraphTab,
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditSelectAll => self.execute_editor(
                &Command::EditSelectAll,
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorElementChange { role, surface } => self.execute_editor(
                &Command::EditorElementChange { role, surface },
                super::RevealIntent::None,
                ctx,
            ),
            Command::EditorSelectionSet { selection, surface } => self.execute_editor(
                &Command::EditorSelectionSet { selection, surface },
                super::RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorSelectionPointer { position, phase, surface } => {
                self.dispatch_pointer_selection(position, phase, surface, ctx)
            }
            Command::EditorNavigationHorizontal { direction, extend, surface } => {
                let previous_vertical_goal = self.preferred_vertical_x_for(surface);
                let selection = self.navigation_selection_horizontal_for(surface, direction, extend);
                let outcome = self.execute_navigation_selection(selection, surface, false, ctx);
                if matches!(&outcome, DesktopCommandOutcome::Rejected(_)) {
                    self.set_preferred_vertical_x_for(surface, previous_vertical_goal);
                }
                outcome
            }
            Command::EditorNavigationVertical { direction, extend, surface } => {
                let previous_vertical_goal = self.preferred_vertical_x_for(surface);
                match self.navigation_selection_vertical_for(surface, direction, extend, ctx) {
                    Ok(selection) => {
                        let outcome = self.execute_navigation_selection(selection, surface, true, ctx);
                        if matches!(&outcome, DesktopCommandOutcome::Rejected(_)) {
                            self.set_preferred_vertical_x_for(surface, previous_vertical_goal);
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
            Command::EditorNavigationLineEdge { edge, extend, surface } => {
                let previous_vertical_goal = self.preferred_vertical_x_for(surface);
                let selection = self.navigation_selection_line_edge_for(surface, edge, extend);
                let outcome = self.execute_navigation_selection(selection, surface, false, ctx);
                if matches!(&outcome, DesktopCommandOutcome::Rejected(_)) {
                    self.set_preferred_vertical_x_for(surface, previous_vertical_goal);
                }
                outcome
            }
            Command::EditorFocus => {
                self.restore_editor_focus(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewZoomSet { percent } => {
                let next = percent.get();
                self.set_zoom_for_surface(self.active_editor_surface(), next, ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewZoomPinch { factor } => {
                let surface = self.active_editor_surface();
                if let Some(next) = super::status_bar::zoom_after_native_factor(
                    self.pinch_zoom_for_surface(surface),
                    factor.get(),
                ) {
                    let next_zoom = super::status_bar::normalized_zoom_percent(next);
                    self.set_zoom_for_surface(surface, next_zoom, ctx);
                    self.set_pinch_zoom_for_surface(surface, next);
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
                if self.sprint_timer.popup_open {
                    self.sprint_timer.request_popup_close(
                        ctx.input(|input| input.time),
                        ctx.content_rect(),
                    );
                } else {
                    self.sprint_timer.clear_popup_transition(ctx);
                    self.clear_sprint_timer_popup_geometry();
                    self.sprint_timer.popup_open = true;
                    self.sprint_timer.just_opened = true;
                    self.sprint_timer
                        .start_popup_opening(ctx.input(|input| input.time));
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerClosePopup => {
                self.sprint_timer.request_popup_close(
                    ctx.input(|input| input.time),
                    ctx.content_rect(),
                );
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
                let selection = self.active_selection();
                match canonical_selection_text(self.session.project(), selection, self.active_editor_surface()) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
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
                let selection = self.active_selection();
                match canonical_selection_text(self.session.project(), selection, self.active_editor_surface()) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
                            let outcome = self.dispatch_internal_command(Command::EditDelete, ctx);
                            if matches!(
                                &outcome,
                                DesktopCommandOutcome::Editor(
                                    CommandOutcome::Changed | CommandOutcome::NoOp
                                )
                            ) {
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
                        Command::EditorTextPaste { text, surface: self.active_editor_surface() },
                        ctx,
                    );
                    if matches!(
                        &outcome,
                        DesktopCommandOutcome::Editor(
                            CommandOutcome::Changed | CommandOutcome::NoOp
                        )
                    ) {
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
        reveal_intent: super::RevealIntent,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        // Defense in depth for stale/direct editor calls that bypass command
        // availability after a replacement handoff has engaged the lock.
        if self.replacement_open_pending() {
            return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::ModalBlocked);
        }
        let active_surface = self.active_editor_surface();
        let outcome = execute_editor_command_on_surface(
            &mut self.session,
            command,
            active_surface,
        );
        match &outcome {
            CommandOutcome::Changed => {
                self.mutate(Ok(()), reveal_intent);
            }
            CommandOutcome::NoOp if command.id() != CommandId::EditorDualDialogueToggle => {
                self.mutate(Ok(()), reveal_intent);
            }
            CommandOutcome::NoOp => {}
            CommandOutcome::Unavailable(_) | CommandOutcome::Rejected(_) => {}
            CommandOutcome::Failed(error) => self.show_edit_error(error.clone()),
        }
        if matches!(
            &outcome,
            CommandOutcome::Changed | CommandOutcome::NoOp
        ) && !(command.id() == CommandId::EditorDualDialogueToggle
            && matches!(&outcome, CommandOutcome::NoOp))
        {
            self.reconcile_body_active_group();
        }
        ctx.request_repaint();
        match outcome {
            CommandOutcome::Unavailable(reason) => {
                DesktopCommandOutcome::Unavailable(match reason {
                    UnavailableReason::UndoHistory => DesktopUnavailableReason::UndoHistory,
                    UnavailableReason::RedoHistory => DesktopUnavailableReason::RedoHistory,
                    UnavailableReason::Selection => DesktopUnavailableReason::Selection,
                    UnavailableReason::DualDialogueTarget => DesktopUnavailableReason::Selection,
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
        surface: EditorSurface,
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
        let preferred_vertical_x = self.preferred_vertical_x_for(surface);
        let outcome = self.execute_editor(
            &Command::EditorSelectionSet {
                selection: command_selection,
                surface,
            },
            super::RevealIntent::Conditional,
            ctx,
        );
        if retain_vertical_goal
            && matches!(
                &outcome,
                DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)
            )
        {
            self.set_preferred_vertical_x_for(surface, preferred_vertical_x);
        }
        outcome
    }

    fn dispatch_pointer_selection(
        &mut self,
        position: CommandPosition,
        phase: PointerPhase,
        surface: EditorSurface,
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
        if let Err(error) = collapsed.validate_for_surface(self.session.project(), surface) {
            return DesktopCommandOutcome::Rejected(CommandRejection::InvalidSelection(error));
        }
        let current = self.active_selection();
        let drag_current = super::Selection {
            anchor: self.drag_anchor.unwrap_or(current.anchor),
            focus: current.focus,
        };
        let Some(desired) = pointer_selection(
            self.session.project(),
            drag_current,
            core_position,
            surface,
            phase,
        ) else {
            return DesktopCommandOutcome::Rejected(CommandRejection::InvalidSelection(
                scriptcraft_commands::SelectionError::ElementOutOfRange,
            ));
        };
        let outcome = self.execute_navigation_selection(desired, surface, false, ctx);
        if matches!(
            &outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)
        ) {
            match phase {
                PointerPhase::Press => self.drag_anchor = Some(core_position),
                PointerPhase::Drag => {}
                PointerPhase::Release | PointerPhase::DoubleClick | PointerPhase::TripleClick => {
                    self.drag_anchor = None;
                }
            }
            self.preferred_vertical_x = None;
            self.rendered_revision = None;
            // Pointer selection is authoritative: it must not inherit a reveal
            // request queued by an earlier edit or navigation command.
            self.reveal_intent = super::RevealIntent::None;
            self.clear_error();
        }
        outcome
    }

    fn set_clipboard_notice(&mut self, message: &str) {
        self.show_error(message);
    }

    pub(super) fn flush_pending_diagnostic_commands(&mut self, ctx: &egui::Context) {
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

    pub(super) fn post_diagnostic(&mut self, message: &str) {
        let mut bounded = String::new();
        for character in message.chars() {
            let character = if character.is_control()
                || matches!(character, '\u{2028}' | '\u{2029}')
            {
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

    /// Completes an app-owned popup transition, or immediately tears it down
    /// when its viewport no longer has a drawable surface. Typed close commands
    /// never call this path; they only request the ordinary animated dismissal.
    pub(super) fn finalize_sprint_timer_popup(&mut self, ctx: &egui::Context) -> bool {
        let finalized = close_sprint_timer_popup_once(&mut self.sprint_timer);
        self.sprint_timer.clear_popup_transition(ctx);
        self.clear_sprint_timer_popup_geometry();
        ctx.request_repaint();
        finalized
    }
}

impl AppCommandAvailability {
    /// One command-state projection consumed by both menu presentations and
    /// checked again at dispatch time to reject stale or modal-blocked input.
    pub(super) const fn is_enabled(self, command: CommandId) -> bool {
        if matches!(command, CommandId::UiStatusSet) {
            return true;
        }
        if self.native_output_active {
            return false;
        }
        if self.replacement_pending {
            return match command {
                // Presentation-only controls remain usable. File Close is
                // allowed only when it resolves to the title-page child.
                CommandId::WindowTitlePageClose => self.title_close,
                CommandId::FileClose => self.title_page_only_close,
                CommandId::ToolsSprintTimerTogglePopup
                | CommandId::ToolsSprintTimerClosePopup
                | CommandId::ToolsSprintTimerSetMode
                | CommandId::ToolsSprintTimerSetMinutes
                | CommandId::ToolsSprintTimerStart
                | CommandId::ToolsSprintTimerPause
                | CommandId::ToolsSprintTimerResume
                | CommandId::ToolsSprintTimerReset
                | CommandId::ToolsSprintTimerEnd => true,
                _ => false,
            };
        }
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
            CommandId::EditorElementChange => self.body_only,
            CommandId::EditorDualDialogueToggle => self.dual_dialogue_toggle,
            CommandId::WindowTitlePageOpen => self.title_open,
            CommandId::WindowTitlePageClose => self.title_close,
            // The global modal gates above still disable Quit during migration
            // and native-output dialogs; otherwise it is focus-independent.
            CommandId::ApplicationQuit => true,
            CommandId::FilePrint => cfg!(any(target_os = "macos", target_os = "windows")),
            _ => true,
        }
    }

    const fn unavailable_reason(self, command: CommandId) -> DesktopUnavailableReason {
        if self.modal_blocked || self.native_output_active || self.replacement_pending {
            return DesktopUnavailableReason::ModalBlocked;
        }
        match command {
            CommandId::EditUndo => DesktopUnavailableReason::UndoHistory,
            CommandId::EditRedo => DesktopUnavailableReason::RedoHistory,
            CommandId::EditDelete => DesktopUnavailableReason::DeleteTarget,
            CommandId::EditSelectAll | CommandId::EditCut | CommandId::EditCopy => {
                DesktopUnavailableReason::Selection
            }
            CommandId::EditorDualDialogueToggle => DesktopUnavailableReason::Selection,
            _ => DesktopUnavailableReason::ModalBlocked,
        }
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
        Command::EditorElementChange { role, .. } => Command::EditorElementChange { role, surface },
        Command::EditorStyleToggle { style, .. } => Command::EditorStyleToggle { surface, style },
        Command::EditorStyleClear { .. } => Command::EditorStyleClear { surface },
        Command::EditorAlignmentSet { alignment, .. } => {
            Command::EditorAlignmentSet { surface, alignment }
        }
        Command::EditorSelectionSet { selection, .. } => Command::EditorSelectionSet { selection, surface },
        Command::EditorSelectionPointer { position, phase, .. } => {
            Command::EditorSelectionPointer { position, phase, surface }
        }
        Command::EditorNavigationHorizontal { direction, extend, .. } => {
            Command::EditorNavigationHorizontal { direction, extend, surface }
        }
        Command::EditorNavigationVertical { direction, extend, .. } => {
            Command::EditorNavigationVertical { direction, extend, surface }
        }
        Command::EditorNavigationLineEdge { edge, extend, .. } => {
            Command::EditorNavigationLineEdge { edge, extend, surface }
        }
        other => other,
    }
}

/// The viewport adapter owns native close/restore requests; imported geometry
/// never becomes semantic screenplay state or a command payload.
pub(super) struct DesktopViewportEffects;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileCloseTarget {
    Root,
    TitlePage,
}

fn file_close_target(surface: EditorSurface) -> FileCloseTarget {
    match surface {
        EditorSurface::Body => FileCloseTarget::Root,
        EditorSurface::TitlePage => FileCloseTarget::TitlePage,
    }
}

impl DesktopViewportEffects {
    pub(super) fn title(ctx: &egui::Context, title: String) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn close_root(ctx: &egui::Context) {
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



#[cfg(test)]
mod surface_command_availability_tests {
    use super::{file_close_target, CommandInvocation, DesktopCommandOutcome, FileCloseTarget};
    use crate::ScriptCraftApp;
    use crate::document_lifecycle::SaveOutcome;
    use scriptcraft_commands::{Command, CommandId, CommandOrigin, DocumentPath, EditorSurface};
    use super::super::sprint_timer::{NotificationFeedback, SprintTimer};

    #[test]
    fn file_close_targets_only_the_focused_viewport() {
        assert_eq!(
            file_close_target(EditorSurface::Body),
            FileCloseTarget::Root
        );
        assert_eq!(
            file_close_target(EditorSurface::TitlePage),
            FileCloseTarget::TitlePage
        );
    }

    #[test]
    fn availability_and_typed_routing_follow_focus_and_surface_policy() {
        let Ok(mut app) = ScriptCraftApp::new() else { return };
        assert!(app.app_command_availability().is_enabled(CommandId::WindowTitlePageOpen));
        app.set_active_editor_surface(EditorSurface::TitlePage);
        let title = app.app_command_availability();
        assert!(title.is_enabled(CommandId::ApplicationQuit));
        assert!(title.is_enabled(CommandId::EditorStyleToggle));
        assert!(!title.is_enabled(CommandId::EditorDualDialogueToggle));

        let keyboard = super::route_to_focused_surface(
            Command::EditorTextInsert { text: "x".into(), surface: EditorSurface::Body },
            CommandOrigin::Keyboard,
            EditorSurface::TitlePage,
        );
        assert!(matches!(keyboard, Command::EditorTextInsert { surface: EditorSurface::TitlePage, .. }));
        let external = super::route_to_focused_surface(
            Command::EditorStyleClear { surface: EditorSurface::Body },
            CommandOrigin::Mcp,
            EditorSurface::TitlePage,
        );
        assert!(matches!(external, Command::EditorStyleClear { surface: EditorSurface::Body }));
    }

    #[test]
    fn direct_save_as_dispatch_reports_rejected_destination_as_failed() {
        let Ok(mut app) = ScriptCraftApp::new() else { return };
        let Ok(path) = DocumentPath::new("/tmp/rejected-save.txt") else {
            return;
        };
        let context = eframe::egui::Context::default();

        let result = app.dispatch_command(
            CommandInvocation::new(
                Command::FileSaveAs {
                    path,
                    overwrite: false,
                },
                CommandOrigin::InternalFollowUp,
            ),
            &context,
        );

        assert_eq!(
            result.outcome,
            DesktopCommandOutcome::SaveCompleted(SaveOutcome::Failed)
        );
    }

    #[test]
    fn direct_path_open_only_stages_and_reports_dispatch_not_open_success() {
        let Ok(mut app) = ScriptCraftApp::new() else { return };
        let Ok(path) = DocumentPath::new("/drafts/Été selected.fdx") else {
            return;
        };
        let context = eframe::egui::Context::default();
        let initial_native_path = app.native_path.clone();
        let initial_dirty = app.session.dirty();

        let result = app.dispatch_command(
            CommandInvocation::new(Command::FileOpen { path }, CommandOrigin::InternalFollowUp),
            &context,
        );

        assert_eq!(result.outcome, DesktopCommandOutcome::EffectDispatched);
        assert_eq!(
            app.staged_replacement_open.as_deref(),
            Some(std::path::Path::new("/drafts/Été selected.fdx")),
        );
        assert!(app.pending_replacement.is_none());
        assert_eq!(app.native_path, initial_native_path);
        assert_eq!(app.session.dirty(), initial_dirty);
    }

    #[test]
    fn migration_modal_preserves_resolution_but_blocks_surface_commands() {
        let availability = super::AppCommandAvailability {
            modal_blocked: true, native_output_active: false,
            replacement_pending: false, title_page_only_close: false,
            undo: false, redo: false, delete: false, select_all: false,
            cut: false, copy: false, paste: false, body_only: true,
            dual_dialogue_toggle: false, title_open: false, title_close: false,
        };
        assert!(availability.is_enabled(CommandId::FileImportResolve));
        assert!(!availability.is_enabled(CommandId::ApplicationQuit));
        assert!(!availability.is_enabled(CommandId::WindowTitlePageOpen));
        assert!(!availability.is_enabled(CommandId::EditorDualDialogueToggle));

        let native_output = super::AppCommandAvailability {
            modal_blocked: false,
            native_output_active: true,
            replacement_pending: false,
            title_page_only_close: false,
            ..availability
        };
        assert!(!native_output.is_enabled(CommandId::ApplicationQuit));
    }

    #[test]
    fn replacement_lock_blocks_document_commands_but_preserves_safe_child_and_timer_controls() {
        let mut availability = super::AppCommandAvailability {
            modal_blocked: false,
            native_output_active: false,
            replacement_pending: true,
            title_page_only_close: false,
            undo: true,
            redo: true,
            delete: true,
            select_all: true,
            cut: true,
            copy: true,
            paste: true,
            body_only: true,
            dual_dialogue_toggle: true,
            title_open: true,
            title_close: true,
        };

        for command in [
            CommandId::FileNew,
            CommandId::FileOpenRequest,
            CommandId::FileOpen,
            CommandId::FileSave,
            CommandId::FileSaveAsRequest,
            CommandId::FileSaveAs,
            CommandId::FileImportResolve,
            CommandId::FilePrint,
            CommandId::FileClose,
            CommandId::ApplicationQuit,
            CommandId::EditUndo,
            CommandId::EditCut,
            CommandId::EditCopy,
            CommandId::EditPaste,
            CommandId::EditorTextInsert,
            CommandId::EditorStyleToggle,
            CommandId::WindowTitlePageOpen,
        ] {
            assert!(!availability.is_enabled(command), "{command:?} must be locked");
        }
        assert!(availability.is_enabled(CommandId::WindowTitlePageClose));
        assert!(availability.is_enabled(CommandId::ToolsSprintTimerStart));
        assert!(availability.is_enabled(CommandId::ToolsSprintTimerClosePopup));
        assert!(!availability.is_enabled(CommandId::FileClose));

        availability.title_page_only_close = true;
        assert!(availability.is_enabled(CommandId::FileClose));
    }

    #[test]
    fn replacement_lock_does_not_enable_migration_resolution_or_survive_resume_projection() {
        let locked = super::AppCommandAvailability {
            modal_blocked: false,
            native_output_active: false,
            replacement_pending: true,
            title_page_only_close: false,
            undo: false,
            redo: false,
            delete: false,
            select_all: false,
            cut: false,
            copy: false,
            paste: false,
            body_only: true,
            dual_dialogue_toggle: false,
            title_open: false,
            title_close: false,
        };
        assert!(!locked.is_enabled(CommandId::FileImportResolve));

        let resumed = super::AppCommandAvailability {
            replacement_pending: false,
            undo: true,
            redo: true,
            ..locked
        };
        assert!(resumed.is_enabled(CommandId::EditUndo));
        assert!(resumed.is_enabled(CommandId::EditorTextInsert));
    }

    #[test]
    fn application_quit_requests_root_close_while_title_page_is_focused() {
        let Ok(mut app) = ScriptCraftApp::new() else { return };
        app.set_active_editor_surface(EditorSurface::TitlePage);
        let context = eframe::egui::Context::default();

        let result = app.dispatch_command(
            CommandInvocation::new(Command::ApplicationQuit, CommandOrigin::NativeMenu),
            &context,
        );

        assert_eq!(result.command, CommandId::ApplicationQuit);
        assert_eq!(result.outcome, DesktopCommandOutcome::EffectDispatched);
    }

    #[test]
    fn duplicate_popup_close_requests_run_idle_cleanup_once() {
        let mut timer = SprintTimer::default();
        timer.popup_open = true;
        timer.set_minutes_input("7".to_owned());
        timer.validation_error = Some("invalid input".to_owned());
        timer.notification_feedback = Some(NotificationFeedback::Unavailable);

        assert!(super::close_sprint_timer_popup_once(&mut timer));
        assert!(!super::close_sprint_timer_popup_once(&mut timer));
        assert!(!timer.popup_open);
        assert!(timer.is_idle());
        assert_eq!(timer.minutes_input, "25");
        assert!(timer.validation_error.is_none());
        assert!(timer.notification_feedback.is_none());
    }

    #[test]
    fn popup_close_preserves_running_and_paused_timer_state() {
        let mut running = SprintTimer::default();
        running.popup_open = true;
        running.set_minutes_input("2".to_owned());
        let _ = running.start_countdown(std::time::Duration::ZERO);
        assert!(super::close_sprint_timer_popup_once(&mut running));
        assert!(running.is_running_or_paused());
        assert_eq!(running.remaining(std::time::Duration::from_secs(15)), Some(std::time::Duration::from_secs(105)));

        let mut paused = running.clone();
        paused.popup_open = true;
        let _ = paused.pause(std::time::Duration::from_secs(15));
        let remaining = paused.remaining(std::time::Duration::from_secs(20));
        assert!(super::close_sprint_timer_popup_once(&mut paused));
        assert!(!super::close_sprint_timer_popup_once(&mut paused));
        assert!(paused.is_running_or_paused());
        assert_eq!(paused.remaining(std::time::Duration::from_secs(20)), remaining);
    }

    #[test]
    fn popup_close_resets_completed_timer_once() {
        let mut timer = SprintTimer::default();
        timer.popup_open = true;
        timer.set_minutes_input("1".to_owned());
        let _ = timer.start_countdown(std::time::Duration::ZERO);
        let _ = timer.reconcile(std::time::Duration::from_secs(60));
        assert!(timer.is_completed());

        assert!(super::close_sprint_timer_popup_once(&mut timer));
        assert!(!super::close_sprint_timer_popup_once(&mut timer));
        assert!(timer.is_idle());
        assert_eq!(timer.minutes_input, "25");
        assert_eq!(timer.mode, super::super::sprint_timer::TimerMode::Countdown);
    }

    #[test]
    fn typed_close_and_icon_toggle_coalesce_then_reopen_with_a_clean_transition() {
        let Ok(mut app) = ScriptCraftApp::new() else { return };
        let ctx = eframe::egui::Context::default();
        let screen = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(600.0, 400.0),
        );
        app.sprint_timer.popup_open = true;
        app.sprint_timer.set_minutes_input("7".to_owned());
        app.sprint_timer.start_popup_opening(5.0);
        let revision = app.session.revision();
        let dirty = app.session.dirty();
        let selection = app.session.selection();
        let project = app.session.project().clone();

        let _ = ctx.run(
            eframe::egui::RawInput {
                screen_rect: Some(screen),
                time: Some(5.07),
                ..Default::default()
            },
            |ctx| {
                let _ = app.dispatch_command(
                    CommandInvocation::new(
                        Command::ToolsSprintTimerClosePopup,
                        CommandOrigin::Pointer,
                    ),
                    ctx,
                );
                let _ = app.dispatch_command(
                    CommandInvocation::new(
                        Command::ToolsSprintTimerTogglePopup,
                        CommandOrigin::Pointer,
                    ),
                    ctx,
                );
            },
        );
        assert!(app.sprint_timer.popup_open);
        assert!(app.sprint_timer.is_closing());
        assert_eq!(app.sprint_timer.minutes_input, "7");
        assert_eq!(app.session.revision(), revision);
        assert_eq!(app.session.dirty(), dirty);
        assert_eq!(app.session.selection(), selection);
        assert_eq!(app.session.project(), &project);

        assert!(app.finalize_sprint_timer_popup(&ctx));
        assert!(!app.sprint_timer.popup_open);
        assert!(!app.finalize_sprint_timer_popup(&ctx));
        assert_eq!(app.sprint_timer.minutes_input, "25");
        assert_eq!(app.session.revision(), revision);
        assert_eq!(app.session.dirty(), dirty);
        assert_eq!(app.session.selection(), selection);
        assert_eq!(app.session.project(), &project);

        let _ = ctx.run(
            eframe::egui::RawInput {
                screen_rect: Some(screen),
                time: Some(6.0),
                ..Default::default()
            },
            |ctx| {
                let _ = app.dispatch_command(
                    CommandInvocation::new(
                        Command::ToolsSprintTimerTogglePopup,
                        CommandOrigin::Pointer,
                    ),
                    ctx,
                );
            },
        );
        assert!(app.sprint_timer.popup_open);
        assert!(!app.sprint_timer.is_closing());
        assert_eq!(app.sprint_timer.popup_anchor, eframe::egui::Rect::NOTHING);
        assert_eq!(app.sprint_timer.popup_rect, eframe::egui::Rect::NOTHING);
    }

    #[test]
    fn completed_cleanup_waits_for_close_completion_but_viewport_teardown_is_immediate() {
        let Ok(mut app) = ScriptCraftApp::new() else { return };
        let ctx = eframe::egui::Context::default();
        let screen = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(600.0, 400.0),
        );
        app.sprint_timer.popup_open = true;
        app.sprint_timer.set_minutes_input("1".to_owned());
        let _ = app.sprint_timer.start_countdown(std::time::Duration::ZERO);
        let _ = app.sprint_timer.reconcile(std::time::Duration::from_secs(60));
        assert!(app.sprint_timer.is_completed());
        let revision = app.session.revision();
        let project = app.session.project().clone();

        let _ = ctx.run(
            eframe::egui::RawInput {
                screen_rect: Some(screen),
                time: Some(9.0),
                ..Default::default()
            },
            |ctx| {
                let _ = app.dispatch_command(
                    CommandInvocation::new(
                        Command::ToolsSprintTimerClosePopup,
                        CommandOrigin::Pointer,
                    ),
                    ctx,
                );
            },
        );
        assert!(app.sprint_timer.is_completed());
        assert_eq!(app.sprint_timer.minutes_input, "1");
        assert_eq!(app.session.revision(), revision);
        assert_eq!(app.session.project(), &project);
        assert!(app.finalize_sprint_timer_popup(&ctx));
        assert!(app.sprint_timer.is_idle());
        assert_eq!(app.sprint_timer.minutes_input, "25");

        app.sprint_timer.popup_open = true;
        app.sprint_timer.set_minutes_input("2".to_owned());
        let _ = app.sprint_timer.start_countdown(std::time::Duration::from_secs(10));
        assert!(app.finalize_sprint_timer_popup(&ctx));
        assert!(!app.sprint_timer.popup_open);
        assert!(app.sprint_timer.is_running_or_paused());
        assert_eq!(
            app.sprint_timer.remaining(std::time::Duration::from_secs(20)),
            Some(std::time::Duration::from_secs(110))
        );
        assert_eq!(app.session.revision(), revision);
        assert_eq!(app.session.project(), &project);
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
fn canonical_selection_text(
    project: &Project,
    selection: Selection,
    surface: EditorSurface,
) -> Option<String> {
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

    let paragraph_text = |index: usize| match surface {
        EditorSurface::Body => project.elements.get(index).map(|element| element.text.as_str()),
        EditorSurface::TitlePage => project.title_page.get(index).map(|paragraph| paragraph.text.as_str()),
    };
    let start_text = paragraph_text(start.element)?;
    if start.element == end.element {
        return Some(start_text.get(start.offset..end.offset)?.to_owned());
    }

    let mut text = String::new();
    text.push_str(start_text.get(start.offset..)?);
    for element_index in (start.element + 1)..=end.element {
        text.push('\n');
        let paragraph = paragraph_text(element_index)?;
        if element_index == end.element {
            text.push_str(paragraph.get(..end.offset)?);
        } else {
            text.push_str(paragraph);
        }
    }
    Some(text)
}
