use super::{LayoutCache, Position, Project, ScriptCraftApp, Selection};
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
        CommandId::FilePrint => Command::FilePrint,
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
            modal_blocked: migration_pending,
            native_output_active,
            undo: !migration_pending && self.session.can_undo(),
            redo: !migration_pending && self.session.can_redo(),
            delete: !migration_pending && delete_target_exists,
            // EditingSession projects a document with at least one body element.
            select_all: !migration_pending && element_count != 0,
            cut: !migration_pending && selection.anchor != selection.focus,
            copy: !migration_pending && selection.anchor != selection.focus,
            // Clipboard state is read only when Paste is activated, never per frame.
            paste: !migration_pending,
            body_only: self.active_editor_surface == EditorSurface::Body,
            dual_dialogue_toggle: dual_dialogue_target.is_some(),
            // The same open command focuses/raises the singleton child when it
            // already exists; modal state, not open state, blocks activation.
            title_open: !migration_pending && !self.title_viewport_unsupported,
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
                self.launch_document_instance(path.as_path());
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileClose => {
                if self.active_editor_surface() == EditorSurface::TitlePage {
                    self.close_title_page_window(ctx);
                } else {
                    DesktopViewportEffects::close(ctx);
                }
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
            Command::FileSaveAs { path, overwrite } => {
                self.save_fdx_as_at(path.as_path(), overwrite);
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
            CommandId::FilePrint => cfg!(any(target_os = "macos", target_os = "windows")),
            _ => true,
        }
    }

    const fn unavailable_reason(self, command: CommandId) -> DesktopUnavailableReason {
        if self.modal_blocked || self.native_output_active {
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

impl DesktopViewportEffects {
    pub(super) fn title(ctx: &egui::Context, title: String) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

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

#[cfg(test)]
mod diagnostic_command_tests {
    use super::{AppCommandAvailability, CommandInvocation};
    use crate::ScriptCraftApp;
    use scriptcraft_commands::{
        Command, CommandId, CommandOrigin, DiagnosticMessage, MAX_DIAGNOSTIC_MESSAGE_BYTES,
    };

    #[test]
    fn diagnostic_set_replace_and_clear_are_dispatched_commands() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else {
            return;
        };
        let context = eframe::egui::Context::default();
        for (text, expected) in [
            (Some("first diagnostic"), Some("first diagnostic")),
            (Some("replacement diagnostic"), Some("replacement diagnostic")),
            (None, None),
        ] {
            let message = text.and_then(|text| {
                let result = DiagnosticMessage::new(text);
                assert!(result.is_ok());
                result.ok()
            });
            let result = app.dispatch_command(
                CommandInvocation::new(
                    Command::UiStatusSet { message },
                    CommandOrigin::InternalFollowUp,
                ),
                &context,
            );
            assert_eq!(result.command, CommandId::UiStatusSet);
            assert_eq!(
                app.diagnostic_message
                    .as_ref()
                    .map(DiagnosticMessage::as_str),
                expected
            );
        }
    }

    #[test]
    fn diagnostic_command_remains_available_during_migration_modal() {
        let availability = AppCommandAvailability {
            modal_blocked: true,
            native_output_active: false,
            undo: false,
            redo: false,
            delete: false,
            select_all: false,
            cut: false,
            copy: false,
            paste: false,
            body_only: false,
            dual_dialogue_toggle: false,
            title_open: false,
            title_close: false,
        };
        assert!(availability.is_enabled(CommandId::UiStatusSet));
        assert!(!availability.is_enabled(CommandId::EditUndo));
    }

    #[test]
    fn queued_diagnostic_reports_preserve_dispatch_order() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else {
            return;
        };
        let context = eframe::egui::Context::default();
        app.post_diagnostic("earlier");
        app.post_diagnostic("later");
        app.flush_pending_diagnostic_commands(&context);
        assert_eq!(
            app.diagnostic_message
                .as_ref()
                .map(DiagnosticMessage::as_str),
            Some("later")
        );
    }

    #[test]
    fn diagnostic_payload_is_single_line_bounded_and_retains_its_prefix_and_code() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else {
            return;
        };
        let context = eframe::egui::Context::default();
        let long_detail = format!("FDX_IMPORT_FAILED: 30: {}\ntrailing detail", "x".repeat(1024));
        app.post_diagnostic(&long_detail);
        app.flush_pending_diagnostic_commands(&context);

        let message = app
            .diagnostic_message
            .as_ref()
            .map(DiagnosticMessage::as_str);
        assert!(message.is_some_and(|message| {
            message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES
                && !message.contains('\n')
                && message.starts_with("FDX_IMPORT_FAILED: 30:")
        }));
    }
}

#[cfg(test)]
mod pdf_export_command_tests {
    use super::CommandInvocation;
    use crate::ScriptCraftApp;
    use scriptcraft_commands::{Command, CommandId, CommandOrigin, PdfExportOptions};
    use std::cell::Cell;

    #[test]
    fn direct_pdf_activation_repeats_without_pending_modal_or_blocking_commands() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        app.surface_focused = true;
        let focus_before = app.surface_focused;
        let active_group_before = app.active_body_dual_dialogue_group.clone();
        let requests = Cell::new(0);
        let unexpected_resolve = Cell::new(false);

        for origin in [CommandOrigin::Menu, CommandOrigin::Toolbar] {
            let result = app.dispatch_command_with_pdf_handlers(
                CommandInvocation::new(Command::FileSaveAsPdf, origin),
                &context,
                |app| {
                    requests.set(requests.get() + 1);
                    app.export_pdf_with_picker(|filename| {
                        assert_eq!(filename, crate::UNTITLED_PDF_FILENAME);
                        crate::pdf_export::PdfExportPickerResult::Cancel
                    });
                },
                |_, _| unexpected_resolve.set(true),
                |_| {},
            );
            assert_eq!(result.command, CommandId::FileSaveAsPdf);
            assert_eq!(result.origin, origin);
            assert_eq!(result.outcome, super::DesktopCommandOutcome::EffectDispatched);
        }

        assert_eq!(requests.get(), 2);
        assert!(!unexpected_resolve.get());
        assert!(app.surface_focused);
        assert_eq!(app.surface_focused, focus_before);
        assert_eq!(app.active_body_dual_dialogue_group, active_group_before);
        assert!(app.app_command_availability().is_enabled(CommandId::FileNew));
        assert!(app.app_command_availability().is_enabled(CommandId::FileSaveAsPdf));
    }

    #[test]
    fn typed_resolve_command_keeps_its_id_and_forwards_supplied_options() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        let options = PdfExportOptions {
            include_title_page: false,
        };
        let command = Command::PdfIncludeTitlePageResolve { options };
        assert_eq!(command.id(), CommandId::PdfIncludeTitlePageResolve);
        let received = Cell::new(None);
        let unexpected_direct = Cell::new(false);

        let result = app.dispatch_command_with_pdf_handlers(
            CommandInvocation::new(command, CommandOrigin::Menu),
            &context,
            |_| unexpected_direct.set(true),
            |app, resolved_options| {
                received.set(Some(resolved_options));
                app.finish_pdf_export(crate::pdf_export::PdfExportPickerResult::Cancel);
            },
            |_| {},
        );

        assert_eq!(result.command, CommandId::PdfIncludeTitlePageResolve);
        assert_eq!(result.origin, CommandOrigin::Menu);
        assert_eq!(result.outcome, super::DesktopCommandOutcome::EffectDispatched);
        assert!(!unexpected_direct.get());
        assert_eq!(received.get(), Some(options));
        assert!(app.app_command_availability().is_enabled(CommandId::FileSaveAsPdf));
    }

    #[test]
    fn building_pdf_preserves_document_session_native_and_view_state() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(app) = app_result else { return };
        let project_before = app.session.project().clone();
        let dirty_before = app.session.dirty();
        let revision_before = app.session.revision();
        let selection_before = app.session.selection();
        let title_selection_before = app.session.title_selection();
        let undo_before = app.session.can_undo();
        let redo_before = app.session.can_redo();
        let native_path_before = app.native_path.clone();
        let native_title_before = app.native_title.clone();
        let zoom_before = app.zoom_percent;
        let body_viewport_before = app.editor_surface_id;
        let title_viewport_before = app.title_editor_surface_id;
        let title_open_before = app.title_page_window_open;

        let output = app.build_pdf_with_options(scriptcraft_commands::PdfExportOptions {
            include_title_page: false,
        });
        assert!(output.is_ok());
        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.dirty(), dirty_before);
        assert_eq!(app.session.revision(), revision_before);
        assert_eq!(app.session.selection(), selection_before);
        assert_eq!(app.session.title_selection(), title_selection_before);
        assert_eq!(app.session.can_undo(), undo_before);
        assert_eq!(app.session.can_redo(), redo_before);
        assert_eq!(app.native_path, native_path_before);
        assert_eq!(app.native_title, native_title_before);
        assert!(app.fdx_baseline.is_none());
        assert_eq!(app.zoom_percent, zoom_before);
        assert_eq!(app.editor_surface_id, body_viewport_before);
        assert_eq!(app.title_editor_surface_id, title_viewport_before);
        assert_eq!(app.title_page_window_open, title_open_before);
    }
}

#[cfg(test)]
mod print_request_command_tests {
    use super::{
        command_for_id, CommandInvocation, DesktopCommandOutcome, DesktopUnavailableReason,
    };
    use crate::ScriptCraftApp;
    use scriptcraft_commands::{Command, CommandId, CommandOrigin, PdfExportOptions};
    use std::path::PathBuf;

    #[test]
    fn file_print_is_typed_and_available_only_on_native_print_platforms() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        let panel_requests = std::cell::Cell::new(0);
        let unexpected_pdf_build = std::cell::Cell::new(false);
        assert_eq!(command_for_id(CommandId::FilePrint), Some(Command::FilePrint));
        assert_eq!(
            app.app_command_availability().is_enabled(CommandId::FilePrint),
            cfg!(any(target_os = "macos", target_os = "windows")),
        );

        let result = app.dispatch_command_with_pdf_handlers(
            CommandInvocation::new(Command::FilePrint, CommandOrigin::Menu),
            &context,
            |_| unexpected_pdf_build.set(true),
            |_, _| unexpected_pdf_build.set(true),
            |_| panel_requests.set(panel_requests.get() + 1),
        );
        assert_eq!(result.command, CommandId::FilePrint);
        assert_eq!(result.origin, CommandOrigin::Menu);
        if cfg!(any(target_os = "macos", target_os = "windows")) {
            assert_eq!(result.outcome, DesktopCommandOutcome::EffectDispatched);
            assert_eq!(panel_requests.get(), 1);
            assert!(!unexpected_pdf_build.get());
            assert_eq!(
                app.app_command_availability().native_output_active,
                cfg!(target_os = "windows"),
            );
            // This injected dispatch test has no native frame/session to own
            // cleanup; model the terminal callback before dropping the fixture.
            app.native_output_modal_active = false;
        } else {
            assert!(matches!(result.outcome, DesktopCommandOutcome::Unavailable(_)));
            assert_eq!(panel_requests.get(), 0);
            assert!(!unexpected_pdf_build.get());
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_file_print_dispatch_is_single_and_keeps_the_async_gate_until_terminal_cleanup() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        let requests = std::cell::Cell::new(0);
        let project_before = app.session.project().clone();
        let revision_before = app.session.revision();
        let dirty_before = app.session.dirty();
        let selection_before = app.session.selection();
        let title_selection_before = app.session.title_selection();
        let history_before = (app.session.can_undo(), app.session.can_redo());
        let path_before = app.native_path.clone();
        let baseline_before = app.fdx_baseline.clone();
        let body_zoom_before = app.zoom_percent;
        let title_zoom_before = app.title_view.zoom_percent;
        let active_group_before = app.active_body_dual_dialogue_group.clone();

        let result = app.dispatch_command_with_pdf_handlers(
            CommandInvocation::new(Command::FilePrint, CommandOrigin::Menu),
            &context,
            |_| panic!("FilePrint must not invoke PDF output"),
            |_, _| panic!("FilePrint must not enter PDF options"),
            |app| {
                requests.set(requests.get() + 1);
                assert!(app.app_command_availability().native_output_active);
                for command in [
                    Command::FilePrint,
                    Command::FileSaveAsPdf,
                    Command::FileNew,
                    Command::FileOpenRequest,
                    Command::FileClose,
                    Command::WindowTitlePageOpen,
                ] {
                    let nested = app.dispatch_command_with_pdf_handlers(
                        CommandInvocation::new(command, CommandOrigin::Shortcut),
                        &context,
                        |_| panic!("active print session must block PDF output"),
                        |_, _| panic!("active print session must block PDF options"),
                        |_| panic!("active print session must block duplicate print"),
                    );
                    assert_eq!(
                        nested.outcome,
                        DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::ModalBlocked),
                    );
                }
            },
        );

        assert_eq!(result.command, CommandId::FilePrint);
        assert_eq!(result.outcome, DesktopCommandOutcome::EffectDispatched);
        assert_eq!(requests.get(), 1);
        assert!(app.app_command_availability().native_output_active);
        assert!(!app.app_command_availability().is_enabled(CommandId::FileSaveAsPdf));
        assert!(!app.app_command_availability().is_enabled(CommandId::FilePrint));
        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.revision(), revision_before);
        assert_eq!(app.session.dirty(), dirty_before);
        assert_eq!(app.session.selection(), selection_before);
        assert_eq!(app.session.title_selection(), title_selection_before);
        assert_eq!((app.session.can_undo(), app.session.can_redo()), history_before);
        assert_eq!(app.native_path, path_before);
        assert_eq!(app.fdx_baseline, baseline_before);
        assert_eq!(app.zoom_percent, body_zoom_before);
        assert_eq!(app.title_view.zoom_percent, title_zoom_before);
        assert_eq!(app.active_body_dual_dialogue_group, active_group_before);

        // Production clears the same persistent gate only when native polling
        // observes terminal completion/cancellation/failure.
        app.native_output_modal_active = false;
        assert!(app.app_command_availability().is_enabled(CommandId::FilePrint));

        let repeated = app.dispatch_command_with_pdf_handlers(
            CommandInvocation::new(Command::FilePrint, CommandOrigin::Shortcut),
            &context,
            |_| panic!("FilePrint must not invoke PDF output"),
            |_, _| panic!("FilePrint must not enter PDF options"),
            |app| {
                requests.set(requests.get() + 1);
                assert!(app.app_command_availability().native_output_active);
            },
        );
        assert_eq!(repeated.command, CommandId::FilePrint);
        assert_eq!(repeated.origin, CommandOrigin::Shortcut);
        assert_eq!(repeated.outcome, DesktopCommandOutcome::EffectDispatched);
        assert_eq!(requests.get(), 2);
        assert!(app.app_command_availability().native_output_active);
        app.native_output_modal_active = false;
    }

    #[test]
    fn native_pdf_picker_blocks_reentrant_print_until_it_returns() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        let nested_print_requests = std::cell::Cell::new(0);

        let result = app.dispatch_command_with_pdf_handlers(
            CommandInvocation::new(Command::FileSaveAsPdf, CommandOrigin::Menu),
            &context,
            |app| {
                let nested_print = app.dispatch_command_with_pdf_handlers(
                    CommandInvocation::new(Command::FilePrint, CommandOrigin::Shortcut),
                    &context,
                    |_| {},
                    |_, _| {},
                    |_| nested_print_requests.set(nested_print_requests.get() + 1),
                );
                assert_eq!(
                    nested_print.outcome,
                    DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::ModalBlocked),
                );
            },
            |_, _| {},
            |_| nested_print_requests.set(nested_print_requests.get() + 1),
        );

        assert_eq!(result.command, CommandId::FileSaveAsPdf);
        assert_eq!(result.outcome, DesktopCommandOutcome::EffectDispatched);
        assert_eq!(nested_print_requests.get(), 0);
        assert!(!app.app_command_availability().native_output_active);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn menu_and_primary_p_print_dispatch_once_and_guard_native_output_modals() {
        for origin in [CommandOrigin::Menu, CommandOrigin::Shortcut] {
            let app_result = ScriptCraftApp::new();
            assert!(app_result.is_ok());
            let Ok(mut app) = app_result else { return };
            let context = eframe::egui::Context::default();
            let panel_requests = std::cell::Cell::new(0);
            let nested_print_requests = std::cell::Cell::new(0);
            let nested_pdf_requests = std::cell::Cell::new(0);
            let command = command_for_id(CommandId::FilePrint)
                .expect("File > Print and primary-P resolve to the typed FilePrint command");
            let project_before = app.session.project().clone();
            let revision_before = app.session.revision();
            let dirty_before = app.session.dirty();
            let selection_before = app.session.selection();
            let native_path_before = app.native_path.clone();
            let baseline_before = app.fdx_baseline.clone();

            let result = app.dispatch_command_with_pdf_handlers(
                CommandInvocation::new(command, origin),
                &context,
                |_| nested_pdf_requests.set(nested_pdf_requests.get() + 1),
                |_, _| {},
                |app| {
                    panel_requests.set(panel_requests.get() + 1);
                    assert!(app.app_command_availability().native_output_active);

                    let nested_print = app.dispatch_command_with_pdf_handlers(
                        CommandInvocation::new(Command::FilePrint, CommandOrigin::Shortcut),
                        &context,
                        |_| nested_pdf_requests.set(nested_pdf_requests.get() + 1),
                        |_, _| {},
                        |_| nested_print_requests.set(nested_print_requests.get() + 1),
                    );
                    let nested_pdf = app.dispatch_command_with_pdf_handlers(
                        CommandInvocation::new(Command::FileSaveAsPdf, CommandOrigin::Menu),
                        &context,
                        |_| nested_pdf_requests.set(nested_pdf_requests.get() + 1),
                        |_, _| {},
                        |_| nested_print_requests.set(nested_print_requests.get() + 1),
                    );
                    assert_eq!(
                        nested_print.outcome,
                        DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::ModalBlocked),
                    );
                    assert_eq!(
                        nested_pdf.outcome,
                        DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::ModalBlocked),
                    );
                },
            );

            assert_eq!(result.command, CommandId::FilePrint);
            assert_eq!(result.origin, origin);
            assert_eq!(result.outcome, DesktopCommandOutcome::EffectDispatched);
            assert_eq!(panel_requests.get(), 1);
            assert_eq!(nested_print_requests.get(), 0);
            assert_eq!(nested_pdf_requests.get(), 0);
            assert!(!app.app_command_availability().native_output_active);
            assert_eq!(app.session.project(), &project_before);
            assert_eq!(app.session.revision(), revision_before);
            assert_eq!(app.session.dirty(), dirty_before);
            assert_eq!(app.session.selection(), selection_before);
            assert_eq!(app.native_path, native_path_before);
            assert_eq!(app.fdx_baseline, baseline_before);
        }
    }

    #[test]
    fn immutable_print_request_preserves_document_session_and_view_state() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        app.native_path = Some(PathBuf::from("/private/scripts/episode.fdx"));

        let project_before = app.session.project().clone();
        let dirty_before = app.session.dirty();
        let revision_before = app.session.revision();
        let body_selection_before = app.session.selection();
        let title_selection_before = app.session.title_selection();
        let history_before = (app.session.can_undo(), app.session.can_redo());
        let native_path_before = app.native_path.clone();
        let baseline_before = app.fdx_baseline.clone();
        let body_zoom_before = app.zoom_percent;
        let title_zoom_before = app.title_view.zoom_percent;
        let active_group_before = app.active_body_dual_dialogue_group.clone();

        let options = PdfExportOptions {
            include_title_page: false,
        };
        let first = app.build_print_request(options);
        let repeated = app.build_print_request(options);
        assert!(first.is_ok());
        assert!(repeated.is_ok());
        let (Ok(first), Ok(repeated)) = (first, repeated) else { return };
        assert_eq!(first, repeated);
        assert_eq!(first.job_title(), "episode");
        assert!(!first.pdf_data().is_empty());

        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.dirty(), dirty_before);
        assert_eq!(app.session.revision(), revision_before);
        assert_eq!(app.session.selection(), body_selection_before);
        assert_eq!(app.session.title_selection(), title_selection_before);
        assert_eq!((app.session.can_undo(), app.session.can_redo()), history_before);
        assert_eq!(app.native_path, native_path_before);
        assert_eq!(app.fdx_baseline, baseline_before);
        assert_eq!(app.zoom_percent, body_zoom_before);
        assert_eq!(app.title_view.zoom_percent, title_zoom_before);
        assert_eq!(app.active_body_dual_dialogue_group, active_group_before);
    }
}

#[cfg(test)]
mod surface_command_availability_tests {
    use crate::ScriptCraftApp;
    use scriptcraft_commands::{Command, CommandId, CommandOrigin, EditorSurface};
    use scriptcraft_core::{editing::{Position, Selection}, Project};

    #[test]
    fn title_lifecycle_body_only_role_and_mixed_formatting_availability_are_surface_aware() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };

        let closed = app.app_command_availability();
        assert!(closed.is_enabled(CommandId::WindowTitlePageOpen));
        assert!(!closed.is_enabled(CommandId::WindowTitlePageClose));
        assert!(!closed.undo);

        app.set_active_editor_surface(EditorSurface::TitlePage);
        assert!(app.session.set_title_selection(super::Selection {
            anchor: super::Position { element: 0, offset: 0 },
            focus: super::Position { element: 1, offset: 0 },
        }).is_ok());
        let title_focused = app.app_command_availability();
        assert!(!title_focused.is_enabled(CommandId::EditorElementChange));
        assert!(title_focused.is_enabled(CommandId::EditorStyleToggle));
        assert!(title_focused.is_enabled(CommandId::EditorStyleClear));
        assert!(title_focused.is_enabled(CommandId::EditorAlignmentSet));

        app.set_title_page_window_open(true);
        let open = app.app_command_availability();
        assert!(open.is_enabled(CommandId::WindowTitlePageOpen));
        assert!(open.is_enabled(CommandId::WindowTitlePageClose));
        assert!(open.is_enabled(CommandId::EditorStyleToggle));
    }

    #[test]
    fn title_clipboard_projection_copies_canonical_plain_text() {
        let project_result = Project::new();
        assert!(project_result.is_ok());
        let Ok(project) = project_result else { return };
        let title = &project.title_page[17].text;
        assert_eq!(title, "Script Title");
        let selected = super::canonical_selection_text(
            &project,
            Selection {
                anchor: Position { element: 17, offset: 0 },
                focus: Position { element: 17, offset: title.len() },
            },
            EditorSurface::TitlePage,
        );
        assert_eq!(selected.as_deref(), Some("Script Title"));
    }

    #[test]
    fn keyboard_editor_payloads_follow_active_surface_but_external_payloads_keep_wire_target() {
        let keyboard = super::route_to_focused_surface(
            Command::EditorTextInsert { text: "x".into(), surface: EditorSurface::Body },
            CommandOrigin::Keyboard,
            EditorSurface::TitlePage,
        );
        assert!(matches!(keyboard, Command::EditorTextInsert { surface: EditorSurface::TitlePage, .. }));
        for origin in [CommandOrigin::Menu, CommandOrigin::NativeMenu, CommandOrigin::Shortcut] {
            let clear = super::route_to_focused_surface(
                Command::EditorStyleClear { surface: EditorSurface::Body },
                origin,
                EditorSurface::TitlePage,
            );
            assert!(matches!(clear, Command::EditorStyleClear { surface: EditorSurface::TitlePage }));
        }
        let external = super::route_to_focused_surface(
            Command::EditorStyleClear { surface: EditorSurface::Body },
            CommandOrigin::Mcp,
            EditorSurface::TitlePage,
        );
        assert!(matches!(external, Command::EditorStyleClear { surface: EditorSurface::Body }));
        assert!(matches!(
            super::route_to_focused_surface(Command::EditUndo, CommandOrigin::Keyboard, EditorSurface::TitlePage),
            Command::EditUndo
        ));
    }
}

#[cfg(test)]
mod title_page_window_command_tests {
    use crate::app_commands::{CommandInvocation, DesktopCommandOutcome};
    use crate::ScriptCraftApp;
    use scriptcraft_commands::{
        Command, CommandId, CommandOrigin, EditorSurface, PointerPhase,
        Position as CommandPosition,
    };
    use scriptcraft_core::editing::{Position as CorePosition, Selection as CoreSelection};

    #[test]
    fn toolbar_open_is_singleton_focusable_and_close_preserves_shared_document_state() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        let project_before = app.session.project().clone();
        let dirty_before = app.session.dirty();
        let revision_before = app.session.revision();
        let selection_before = app.session.selection();
        let title_selection_before = app.session.title_selection();

        let created = app.dispatch_command(
            CommandInvocation::new(Command::WindowTitlePageOpen, CommandOrigin::Toolbar),
            &context,
        );
        assert_eq!(created.command, CommandId::WindowTitlePageOpen);
        assert_eq!(created.origin, CommandOrigin::Toolbar);
        assert_eq!(created.outcome, DesktopCommandOutcome::EffectDispatched);
        assert!(app.title_page_window_open);

        app.set_active_editor_surface(EditorSurface::TitlePage);
        let focused = app.dispatch_command(
            CommandInvocation::new(Command::WindowTitlePageOpen, CommandOrigin::Toolbar),
            &context,
        );
        assert_eq!(focused.origin, CommandOrigin::Toolbar);
        assert_eq!(focused.outcome, DesktopCommandOutcome::EffectDispatched);
        assert!(app.title_page_window_open);
        assert!(app.title_page_focus_requested);

        let closed = app.dispatch_command(
            CommandInvocation::new(Command::WindowTitlePageClose, CommandOrigin::Toolbar),
            &context,
        );
        assert_eq!(closed.outcome, DesktopCommandOutcome::EffectDispatched);
        assert!(!app.title_page_window_open);
        assert_eq!(app.active_editor_surface(), EditorSurface::Body);
        assert!(app.title_view_reset_pending);
        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.dirty(), dirty_before);
        assert_eq!(app.session.revision(), revision_before);
        assert_eq!(app.session.selection(), selection_before);
        assert_eq!(app.session.title_selection(), title_selection_before);

        app.open_title_page_window(&context);
        app.set_active_editor_surface(EditorSurface::TitlePage);
        let file_close = app.dispatch_command(
            CommandInvocation::new(Command::FileClose, CommandOrigin::Menu),
            &context,
        );
        assert_eq!(file_close.outcome, DesktopCommandOutcome::EffectDispatched);
        assert!(!app.title_page_window_open);
        assert_eq!(app.session.project(), &project_before);
    }

    #[test]
    fn migration_modal_precedence_remains_after_pdf_modal_removal() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(app) = app_result else { return };

        let modal_availability = super::AppCommandAvailability {
            modal_blocked: true,
            native_output_active: false,
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
        assert!(!modal_availability.is_enabled(CommandId::WindowTitlePageOpen));
        assert!(!modal_availability.is_enabled(CommandId::EditorDualDialogueToggle));
        assert!(!modal_availability.is_enabled(CommandId::FileSaveAsPdf));
        assert!(!modal_availability.is_enabled(CommandId::FilePrint));
        assert!(modal_availability.is_enabled(CommandId::FileImportResolve));
        assert!(app
            .app_command_availability()
            .is_enabled(CommandId::WindowTitlePageOpen));
    }

    #[test]
    fn body_and_title_zoom_are_independent_while_document_history_is_shared() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        app.zoom_percent = 137;
        app.set_active_editor_surface(EditorSurface::TitlePage);
        app.set_zoom_for_surface(EditorSurface::TitlePage, 125, &context);
        assert_eq!(app.zoom_percent, 137);
        assert_eq!(app.title_view.zoom_percent, 125);

        let original_title = app.session.project().title_page[0].text.clone();
        let inserted = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorTextInsert {
                    text: "X".to_owned(),
                    surface: EditorSurface::TitlePage,
                },
                CommandOrigin::Keyboard,
            ),
            &context,
        );
        assert!(matches!(inserted.outcome, DesktopCommandOutcome::Editor(_)));
        assert_ne!(app.session.project().title_page[0].text, original_title);
        app.set_active_editor_surface(EditorSurface::Body);
        let undone = app.dispatch_command(
            CommandInvocation::new(Command::EditUndo, CommandOrigin::Menu),
            &context,
        );
        assert!(matches!(undone.outcome, DesktopCommandOutcome::Editor(_)));
        assert_eq!(app.session.project().title_page[0].text, original_title);
        assert_eq!(app.zoom_percent, 137);
        assert_eq!(app.title_view.zoom_percent, 125);
    }

    #[test]
    fn rejected_title_selection_keeps_shared_project_and_both_surface_states_intact() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        let project_before = app.session.project().clone();
        let revision_before = app.session.revision();
        let body_selection_before = app.session.selection();
        let title_selection_before = app.session.title_selection();
        app.title_view.zoom_percent = 143;
        app.set_active_editor_surface(EditorSurface::TitlePage);

        let rejected = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorSelectionSet {
                    selection: scriptcraft_commands::Selection {
                        anchor: scriptcraft_commands::Position {
                            element: u32::MAX,
                            offset: 0,
                        },
                        focus: scriptcraft_commands::Position {
                            element: u32::MAX,
                            offset: 0,
                        },
                    },
                    surface: EditorSurface::TitlePage,
                },
                CommandOrigin::Toolbar,
            ),
            &context,
        );

        assert!(matches!(rejected.outcome, DesktopCommandOutcome::Rejected(_)));
        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.revision(), revision_before);
        assert_eq!(app.session.selection(), body_selection_before);
        assert_eq!(app.session.title_selection(), title_selection_before);
        assert_eq!(app.title_view.zoom_percent, 143);
    }

    #[test]
    fn title_pointer_selection_updates_only_the_title_selection() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        let body_selection = app.session.selection();
        let title_index = 17;
        let title_offset = 4;
        app.set_active_editor_surface(EditorSurface::TitlePage);

        let result = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorSelectionPointer {
                    position: CommandPosition {
                        element: title_index as u32,
                        offset: title_offset as u32,
                    },
                    phase: PointerPhase::Release,
                    surface: EditorSurface::TitlePage,
                },
                CommandOrigin::Pointer,
            ),
            &context,
        );

        assert!(matches!(result.outcome, DesktopCommandOutcome::Editor(_)));
        assert_eq!(app.session.selection(), body_selection);
        assert_eq!(
            app.session.title_selection(),
            super::Selection::caret(super::Position {
                element: title_index as usize,
                offset: title_offset as usize,
            }),
        );
    }

    #[test]
    fn root_text_state_projection_uses_only_body_zoom_and_selection() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let body_selection = app.session.selection();
        let title_selection = app.session.title_selection();
        app.zoom_percent = 131;
        app.title_view.zoom_percent = 217;
        app.set_active_editor_surface(EditorSurface::TitlePage);
        let root_state = app.root_text_state();
        assert_eq!(root_state.scaling, 131);
        assert_eq!(root_state.selection, body_selection);
        assert_eq!(app.session.title_selection(), title_selection);

        // The child callback swaps its local renderer fields into Script View's
        // shared implementation. Root persistence must still project body zoom.
        app.swap_title_view_state();
        assert_eq!(app.zoom_percent, 217);
        assert_eq!(app.title_view.zoom_percent, 131);
        let root_state_during_child = app.root_text_state();
        assert_eq!(root_state_during_child.scaling, 131);
        assert_eq!(root_state_during_child.selection, body_selection);
        assert_eq!(app.session.title_selection(), title_selection);
        app.swap_title_view_state();
    }

    #[test]
    fn root_teardown_uses_the_same_idempotent_title_close_path() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        app.open_title_page_window(&context);
        app.set_active_editor_surface(EditorSurface::TitlePage);
        app.close_title_page_window(&context);
        app.close_title_page_window(&context);
        assert!(!app.title_page_window_open);
        assert_eq!(app.active_editor_surface(), EditorSurface::Body);
        assert!(app.title_view_reset_pending);
    }

    #[test]
    fn save_rebase_invalidates_child_layout_without_discarding_title_selection() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        app.title_view.rendered_revision = Some(app.session.revision());
        app.title_view.preferred_vertical_x = Some(42);
        let title_selection = app.session.title_selection();

        app.invalidate_after_save_rebase();

        assert_eq!(app.title_view.rendered_revision, None);
        assert_eq!(app.title_view.preferred_vertical_x, None);
        assert_eq!(app.session.title_selection(), title_selection);
    }

    #[test]
    fn replacing_document_while_title_viewport_is_open_keeps_one_child_and_resets_surface_publication() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = eframe::egui::Context::default();
        app.open_title_page_window(&context);
        app.set_active_editor_surface(EditorSurface::TitlePage);
        app.title_view.zoom_percent = 143;
        app.title_view.rendered_revision = Some(app.session.revision());
        app.title_view.preferred_vertical_x = Some(37);
        let old_title_selection = CoreSelection::caret(CorePosition {
            element: 17,
            offset: 4,
        });
        app.session
            .set_title_selection(old_title_selection)
            .unwrap_or_else(|_| unreachable!("current title selection is valid"));

        let replacement = scriptcraft_core::fdx::read_v6(include_bytes!(
            "../../../.reference/reference3.fdx"
        ))
        .unwrap_or_else(|_| unreachable!("reference3 is a valid replacement fixture"))
        .project;
        let activated = app.activate_opened_project(
            replacement,
            std::path::Path::new("replacement.fdx"),
            None,
            None,
            None,
            scriptcraft_core::fdx::TextState {
                scaling: 100,
                selection: CoreSelection::caret(CorePosition {
                    element: 0,
                    offset: 0,
                }),
            },
        );

        assert!(activated);
        assert!(app.title_page_window_open);
        assert_eq!(app.active_editor_surface(), EditorSurface::TitlePage);
        assert_eq!(app.session.project().elements[1].text, "Action with BOLD, italics, and underline text styles. ");
        assert_eq!(app.title_view.zoom_percent, 143);
        assert_eq!(app.title_view.rendered_revision, None);
        assert_eq!(app.title_view.preferred_vertical_x, None);
        assert_eq!(app.session.selection(), CoreSelection::caret(CorePosition {
            element: 0,
            offset: 0,
        }));
        assert_eq!(app.session.title_selection(), CoreSelection::caret(CorePosition {
            element: 0,
            offset: 0,
        }));
        assert_ne!(app.session.title_selection(), old_title_selection);
    }
}

#[cfg(test)]
mod save_as_consent_tests {
    use super::CommandInvocation;
    use crate::ScriptCraftApp;
    use scriptcraft_commands::{Command, CommandId, CommandOrigin, DocumentPath};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_destination() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "scriptcraft-save-as-consent-{}-{nonce}.fdx",
            std::process::id()
        ))
    }

    #[test]
    fn direct_save_as_requires_explicit_consent_to_replace_existing_path() {
        let path = unique_destination();
        std::fs::write(&path, b"prior destination")
            .unwrap_or_else(|_| unreachable!("temporary destination can be written"));
        let path_text = path
            .to_str()
            .unwrap_or_else(|| unreachable!("temporary path is UTF-8"));
        let document_path = DocumentPath::new(path_text)
            .unwrap_or_else(|_| unreachable!("temporary document path is valid"));

        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else {
            let _ = std::fs::remove_file(path);
            return;
        };
        app.session
            .insert("pending screenplay text")
            .unwrap_or_else(|_| unreachable!("test insertion is valid"));
        let before = app.session.clone();
        let context = eframe::egui::Context::default();

        let rejected = app.dispatch_command(
            CommandInvocation::new(
                Command::FileSaveAs {
                    path: document_path.clone(),
                    overwrite: false,
                },
                CommandOrigin::InternalFollowUp,
            ),
            &context,
        );
        assert_eq!(rejected.command, CommandId::FileSaveAs);
        assert_eq!(std::fs::read(&path).ok().as_deref(), Some(&b"prior destination"[..]));
        assert_eq!(app.session, before);
        assert!(app.session.dirty());
        assert!(app.native_path.is_none());
        assert!(app.fdx_baseline.is_none());

        let accepted = app.dispatch_command(
            CommandInvocation::new(
                Command::FileSaveAs {
                    path: document_path,
                    overwrite: true,
                },
                CommandOrigin::InternalFollowUp,
            ),
            &context,
        );
        assert_eq!(accepted.command, CommandId::FileSaveAs);
        let saved_bytes = std::fs::read(&path)
            .unwrap_or_else(|_| unreachable!("successful Save As produced a destination"));
        assert_ne!(saved_bytes, b"prior destination");
        let reopened = scriptcraft_core::fdx::read_v6(&saved_bytes)
            .unwrap_or_else(|_| unreachable!("successful Save As output is valid FDX"));
        assert!(reopened
            .project
            .elements
            .iter()
            .any(|element| element.text == "pending screenplay text"));
        assert!(!app.session.dirty());
        assert_eq!(app.native_path.as_deref(), Some(path.as_path()));
        assert!(app.fdx_baseline.is_some());

        let _ = std::fs::remove_file(path);
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

#[cfg(test)]
mod dual_dialogue_dispatch_tests {
    use super::{CommandInvocation, DesktopCommandOutcome};
    use crate::{menu::PendingDesktopCommands, ScriptCraftApp};
    use eframe::egui;
    use scriptcraft_commands::{Command, CommandId, CommandOrigin, CommandOutcome, EditorSurface};
    use scriptcraft_core::editing::{EditingSession, Position, Selection};
    use scriptcraft_core::{fdx::read_v6, Project, Role, ScreenplayElement};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    fn app_with_toggle_target() -> Option<ScriptCraftApp> {
        let mut app = ScriptCraftApp::new().ok()?;
        let mut project = Project::new().ok()?;
        project.elements = [Role::Character, Role::Dialogue, Role::Character, Role::Dialogue]
            .into_iter()
            .enumerate()
            .map(|(index, role)| {
                ScreenplayElement::new(format!("desktop-toggle-{index}"), role, "line").ok()
            })
            .collect::<Option<Vec<_>>>()?;
        app.session = EditingSession::new(project).ok()?;
        Some(app)
    }

    fn app_with_toggle_target_and_outside_action() -> Option<ScriptCraftApp> {
        let mut app = app_with_toggle_target()?;
        let mut project = app.session.project().clone();
        project
            .elements
            .push(ScreenplayElement::new("desktop-outside-action", Role::Action, "outside").ok()?);
        app.session = EditingSession::with_policy(project, app.session.policy()).ok()?;
        Some(app)
    }

    fn app_with_imported_dual_group() -> Option<ScriptCraftApp> {
        let mut app = ScriptCraftApp::new().ok()?;
        let source = include_str!("../../../crates/scriptcraft-core/tests/fixtures/dual_dialogue_basic.fdx");
        let project = read_v6(source.as_bytes()).ok()?.project;
        app.session = EditingSession::new(project).ok()?;
        app.surface_focused = true;
        app.session.set_selection(Selection::caret(Position { element: 1, offset: 0 })).ok()?;
        app.reconcile_body_active_group();
        Some(app)
    }

    fn app_with_imported_cue_only_group() -> Option<ScriptCraftApp> {
        let mut app = ScriptCraftApp::new().ok()?;
        let source = include_str!("../../../crates/scriptcraft-core/tests/fixtures/dual_dialogue_empty_sides.fdx");
        let mut project = read_v6(source.as_bytes()).ok()?.project;
        let group = project.dual_dialogue_groups.iter().find(|group| group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty())?.clone();
        let member_index = project.elements.iter().position(|element| element.id == group.left.character)?;
        app.session = EditingSession::new(project).ok()?;
        app.surface_focused = true;
        app.session.set_selection(Selection::caret(Position { element: member_index, offset: 0 })).ok()?;
        app.reconcile_body_active_group();
        Some(app)
    }

    #[test]
    fn scheduled_derived_refresh_uses_flat_dual_cues_without_wrapper_participation() {
        let source = include_str!("../../../crates/scriptcraft-core/tests/fixtures/dual_dialogue_basic.fdx")
            .replace(
                "  <Content>",
                "  <Content>\n    <Paragraph Type=\"Scene Heading\" id=\"scheduled-scene\"><Text>INT. ROOM - DAY</Text><SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\"><SceneArcBeats/></SceneProperties></Paragraph>\n    <Paragraph Type=\"Cast List\" id=\"scheduled-cast\"><Text>(STALE)</Text></Paragraph>",
            );
        let imported = read_v6(source.as_bytes());
        assert!(imported.is_ok(), "scheduled refresh fixture imports");
        let Ok(imported) = imported else { return };
        let Ok(mut app) = ScriptCraftApp::new() else { return };
        let Ok(session) = EditingSession::new(imported.project) else { return };
        app.session = session;
        let original_group = app.session.project().dual_dialogue_groups[0].clone();
        let now = Instant::now() + Duration::from_secs(2);
        app.next_cast_list_refresh = now;

        app.refresh_cast_lists_on_schedule(&egui::Context::default(), now);

        assert_eq!(app.performance.derived_refresh_attempts, 1);
        assert_eq!(app.performance.derived_refresh_completed, 1);
        assert_eq!(app.session.project().dual_dialogue_groups, vec![original_group]);
        assert_eq!(
            app.session.project().elements.iter()
                .filter(|element| element.role == Role::Character).count(),
            2,
        );
        assert_eq!(
            app.session.project().elements.iter()
                .find(|element| element.role == Role::CastList)
                .map(|element| element.text.as_str()),
            Some("(MARA, NOAH)"),
        );
    }

    #[test]
    fn save_rebase_maps_active_group_and_failed_open_keeps_current_instance_isolated() {
        let Some(mut app) = app_with_imported_dual_group() else { return };
        let original_active = app.active_body_dual_dialogue_group.clone();
        assert!(original_active.is_some());
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos()).unwrap_or_default();
        let first = std::env::temp_dir().join(format!("scriptcraft-wo014-{nonce}-first.fdx"));
        let second = std::env::temp_dir().join(format!("scriptcraft-wo014-{nonce}-second.fdx"));

        app.save_fdx_as_at(&first, false);
        assert_eq!(app.native_path.as_deref(), Some(first.as_path()));
        assert_eq!(app.session.selection().focus.element, 1);
        let active_after_first = app.active_body_dual_dialogue_group.clone();
        assert!(active_after_first.is_some());
        assert!(app.session.project().dual_dialogue_groups.iter()
            .any(|group| Some(&group.id) == active_after_first.as_ref()));
        assert!(!app.session.dirty());

        app.save_native_at(&first);
        assert_eq!(app.active_body_dual_dialogue_group, active_after_first);
        let failed_save_session = app.session.clone();
        let failed_save_active = app.active_body_dual_dialogue_group.clone();
        let failed_save_path = app.native_path.clone();
        app.save_fdx_as_at(&first, false);
        assert_eq!(app.session, failed_save_session);
        assert_eq!(app.active_body_dual_dialogue_group, failed_save_active);
        assert_eq!(app.native_path, failed_save_path);
        app.save_fdx_as_at(&second, false);
        assert_eq!(app.native_path.as_deref(), Some(second.as_path()));
        assert!(app.active_body_dual_dialogue_group.as_ref().is_some_and(|id| {
            app.session.project().dual_dialogue_groups.iter().any(|group| &group.id == id)
        }));
        let _ = std::fs::remove_file(&first);
        let _ = std::fs::remove_file(&second);

        let preserved_session = app.session.clone();
        let preserved_active = app.active_body_dual_dialogue_group.clone();
        let preserved_path = app.native_path.clone();
        let missing = std::env::temp_dir().join(format!("scriptcraft-wo014-{nonce}-missing.fdx"));
        assert!(!app.open_staged_native_at(&missing));
        assert_eq!(app.session, preserved_session);
        assert_eq!(app.active_body_dual_dialogue_group, preserved_active);
        assert_eq!(app.native_path, preserved_path);
        assert_eq!(app.session.project().dual_dialogue_groups.len(), 1);
        assert!(!app.session.dirty());
    }

    #[test]
    fn save_as_rebase_maps_active_cue_only_group_from_complete_real_member_evidence() {
        let Some(mut app) = app_with_imported_cue_only_group() else { return };
        let before = app.session.project().clone();
        let old_group = before.dual_dialogue_groups.iter().find(|group| group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()).cloned();
        assert!(old_group.is_some());
        let Some(old_group) = old_group else { return };
        assert_eq!(old_group.member_ids().count(), 2);
        assert_eq!(app.active_body_dual_dialogue_group.as_ref(), Some(&old_group.id));
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_nanos()).unwrap_or_default();
        let destination = std::env::temp_dir().join(format!("scriptcraft-cue-only-rebase-{nonce}.fdx"));
        app.save_fdx_as_at(&destination, false);
        assert_eq!(app.native_path.as_deref(), Some(destination.as_path()));
        let active_after = app.active_body_dual_dialogue_group.clone();
        assert!(active_after.is_some());
        assert!(app.session.project().dual_dialogue_groups.iter().any(|group| {
            Some(&group.id) == active_after.as_ref()
                && group.member_ids().count() == 2
                && group.left.paragraphs.is_empty()
                && group.right.paragraphs.is_empty()
        }));
        assert_eq!(
            app.session.project().dual_dialogue_groups.iter().find(|group| Some(&group.id) == active_after.as_ref()).map(|group| group.member_ids().cloned().collect::<Vec<_>>()),
            Some(old_group.member_ids().cloned().collect()),
            "rebase maps both real cue identities and no placeholder",
        );
        assert!(!app.session.dirty());
        let _ = std::fs::remove_file(destination);
    }

    fn enqueue_toggle(app: &mut ScriptCraftApp, origin: CommandOrigin) {
        app.pending_commands
            .enqueue_id(CommandId::EditorDualDialogueToggle, origin);
    }

    #[test]
    fn shared_availability_rechecks_collapsed_body_target_and_rejects_title_focus() {
        let Some(mut app) = app_with_toggle_target() else { return };
        assert!(app
            .app_command_availability()
            .is_enabled(CommandId::EditorDualDialogueToggle));
        assert!(!app.dual_dialogue_menu_should_say_undual());
        let toggle = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorDualDialogueToggle,
                CommandOrigin::Shortcut,
            ),
            &egui::Context::default(),
        );
        assert_eq!(toggle.outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed));
        assert!(app.dual_dialogue_menu_should_say_undual());

        assert!(app
            .session
            .set_selection(Selection {
                anchor: Position { element: 0, offset: 0 },
                focus: Position { element: 0, offset: 1 },
            })
            .is_ok());
        assert!(!app
            .app_command_availability()
            .is_enabled(CommandId::EditorDualDialogueToggle));
        assert!(!app.dual_dialogue_menu_should_say_undual());

        assert!(app
            .session
            .set_selection(Selection::caret(Position { element: 0, offset: 0 }))
            .is_ok());
        app.set_active_editor_surface(EditorSurface::TitlePage);
        assert!(!app
            .app_command_availability()
            .is_enabled(CommandId::EditorDualDialogueToggle));
    }

    #[test]
    fn fifo_dispatch_executes_one_typed_activation_and_undoes_as_one_history_step() {
        let Some(mut app) = app_with_toggle_target() else { return };
        let context = egui::Context::default();
        let mut pending = PendingDesktopCommands::default();
        pending.enqueue_id(CommandId::EditorDualDialogueToggle, CommandOrigin::Menu);
        app.pending_commands = pending;
        app.dispatch_pending_commands(&context);

        assert_eq!(app.session.project().dual_dialogue_groups.len(), 1);
        assert!(app.session.can_undo());
        let undone = app.dispatch_command(
            CommandInvocation::new(Command::EditUndo, CommandOrigin::Menu),
            &context,
        );
        assert_eq!(
            undone.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed),
        );
        assert!(app.session.project().dual_dialogue_groups.is_empty());
        let second_undo = app.dispatch_command(
            CommandInvocation::new(Command::EditUndo, CommandOrigin::Menu),
            &context,
        );
        assert_eq!(
            second_undo.outcome,
            DesktopCommandOutcome::Unavailable(super::DesktopUnavailableReason::UndoHistory),
        );
    }

    #[test]
    fn queued_toggle_refreshes_availability_and_pdf_activation_does_not_block_mutation() {
        let Some(mut app) = app_with_toggle_target() else { return };
        let context = egui::Context::default();
        let mut pending = PendingDesktopCommands::default();
        pending.enqueue_id(CommandId::EditorDualDialogueToggle, CommandOrigin::NativeMenu);
        app.pending_commands = pending;
        assert!(app
            .session
            .set_selection(Selection {
                anchor: Position { element: 0, offset: 0 },
                focus: Position { element: 0, offset: 1 },
            })
            .is_ok());
        let stale_baseline = app.session.clone();
        app.dispatch_pending_commands(&context);
        assert_eq!(app.session, stale_baseline);
        assert!(!app.session.can_undo());

        assert!(app
            .session
            .set_selection(Selection::caret(Position { element: 0, offset: 0 }))
            .is_ok());
        let opened = app.dispatch_command_inner_with_pdf_handlers(
            Command::FileSaveAsPdf,
            &context,
            |app| {
                app.export_pdf_with_picker(|_| crate::pdf_export::PdfExportPickerResult::Cancel);
            },
            |_, _| {},
            |_| {},
        );
        assert_eq!(opened, DesktopCommandOutcome::EffectDispatched);
        let modal_baseline = app.session.clone();
        let toggled = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorDualDialogueToggle,
                CommandOrigin::Shortcut,
            ),
            &context,
        );
        assert_eq!(
            toggled.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed),
        );
        assert_ne!(app.session, modal_baseline);
    }

    #[test]
    fn native_and_shortcut_activations_use_the_shared_fifo_dispatcher_once() {
        for origin in [CommandOrigin::NativeMenu, CommandOrigin::Shortcut] {
            let Some(mut app) = app_with_toggle_target() else { return };
            let context = egui::Context::default();
            enqueue_toggle(&mut app, origin);
            app.dispatch_pending_commands(&context);
            assert_eq!(app.session.project().dual_dialogue_groups.len(), 1);
            assert!(app.session.can_undo());
        }
    }

    #[test]
    fn semantic_toggle_dispatch_restores_focus_to_the_body_editor() {
        let Some(mut app) = app_with_toggle_target() else { return };
        let context = egui::Context::default();
        let editor_id = egui::Id::new("dual-dialogue-focus-restoration-body-editor");
        app.editor_surface_id = Some(editor_id);

        let dispatched = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorDualDialogueToggle,
                CommandOrigin::Menu,
            ),
            &context,
        );

        assert_eq!(
            dispatched.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed),
        );
        assert!(context.memory(|memory| memory.has_focus(editor_id)));
    }

    #[test]
    fn body_group_activation_tracks_enable_disable_undo_and_redo_without_history_state() {
        let Some(mut app) = app_with_toggle_target() else { return };
        let context = egui::Context::default();
        app.surface_focused = true;
        let enabled = app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        assert_eq!(
            enabled.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed),
        );
        let restored_group_id = app
            .session
            .project()
            .dual_dialogue_groups
            .first()
            .map(|group| group.id.clone());
        assert_eq!(app.active_body_dual_dialogue_group, restored_group_id);

        let undo = app.dispatch_command(
            CommandInvocation::new(Command::EditUndo, CommandOrigin::Menu),
            &context,
        );
        assert_eq!(undo.outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed));
        assert!(app.session.project().dual_dialogue_groups.is_empty());
        assert_eq!(app.active_body_dual_dialogue_group, None);

        let redo = app.dispatch_command(
            CommandInvocation::new(Command::EditRedo, CommandOrigin::Menu),
            &context,
        );
        assert_eq!(redo.outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed));
        assert_eq!(
            app.session.project().dual_dialogue_groups.first().map(|group| &group.id),
            restored_group_id.as_ref(),
        );
        assert_eq!(app.active_body_dual_dialogue_group, restored_group_id);

        let disabled = app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        assert_eq!(
            disabled.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed),
        );
        assert!(app.session.project().dual_dialogue_groups.is_empty());
        assert_eq!(app.active_body_dual_dialogue_group, None);
    }

    #[test]
    fn accepted_pointer_selection_uses_focus_for_group_entry_and_click_away() {
        let Some(mut app) = app_with_toggle_target_and_outside_action() else { return };
        let context = egui::Context::default();
        app.surface_focused = true;
        app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        let group_id = app.active_body_dual_dialogue_group.clone();
        assert!(group_id.is_some());

        let outside = app.execute_navigation_selection(
            Selection::caret(Position { element: 4, offset: 0 }),
            EditorSurface::Body,
            false,
            &context,
        );
        assert_eq!(outside, DesktopCommandOutcome::Editor(CommandOutcome::Changed));
        assert_eq!(app.active_body_dual_dialogue_group, None);

        let noncollapsed_with_group_focus = Selection {
            anchor: Position { element: 4, offset: 0 },
            focus: Position { element: 0, offset: 0 },
        };
        app.preferred_vertical_x = Some(37);
        app.rendered_revision = Some(app.session.revision());
        app.page_scale = Some(1.25);
        let entered = app.execute_navigation_selection(
            noncollapsed_with_group_focus,
            EditorSurface::Body,
            false,
            &context,
        );
        assert_eq!(entered, DesktopCommandOutcome::Editor(CommandOutcome::Changed));
        assert_eq!(app.active_body_dual_dialogue_group, group_id);
        assert_eq!(app.preferred_vertical_x, None);
        assert_eq!(app.rendered_revision, None);
        assert_eq!(app.page_scale, None);
        assert_eq!(app.reveal_intent, crate::RevealIntent::Conditional);
        let active_layout = app.layout_for_render(EditorSurface::Body);
        assert!(active_layout.is_ok_and(|layout| {
            layout.active_group_id == app.active_body_dual_dialogue_group
        }));
    }

    #[test]
    fn automatic_pattern_unwrap_deactivates_the_group_and_failed_commands_do_not_activate() {
        let Some(mut app) = app_with_toggle_target() else { return };
        let context = egui::Context::default();
        app.surface_focused = true;
        app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        assert!(app.active_body_dual_dialogue_group.is_some());
        let selection = app.execute_navigation_selection(
            Selection::caret(Position { element: 1, offset: 0 }),
            EditorSurface::Body,
            false,
            &context,
        );
        assert_eq!(selection, DesktopCommandOutcome::Editor(CommandOutcome::Changed));
        let changed_role = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorElementChange {
                    role: scriptcraft_commands::RoleValue::new(Role::Action),
                    surface: EditorSurface::Body,
                },
                CommandOrigin::Menu,
            ),
            &context,
        );
        assert!(matches!(
            changed_role.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed)
        ));
        assert!(app.session.project().dual_dialogue_groups.is_empty());
        assert_eq!(app.active_body_dual_dialogue_group, None);

        app.set_active_body_group(None);
        let noncollapsed = Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 1, offset: 1 },
        };
        let Ok(selection) = scriptcraft_commands::Selection::from_core(noncollapsed) else { return };
        let selected = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorSelectionSet {
                    selection,
                    surface: EditorSurface::Body,
                },
                CommandOrigin::Pointer,
            ),
            &context,
        );
        assert_eq!(selected.outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed));
        let unavailable = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorDualDialogueToggle,
                CommandOrigin::Menu,
            ),
            &context,
        );
        assert!(matches!(unavailable.outcome, DesktopCommandOutcome::Unavailable(_)));
        assert_eq!(app.active_body_dual_dialogue_group, None);
    }

    #[test]
    fn focus_loss_title_surface_and_pdf_export_clear_only_transient_body_presentation() {
        let Some(mut app) = app_with_toggle_target() else { return };
        let context = egui::Context::default();
        app.surface_focused = true;
        app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        let semantic_baseline = app.session.clone();
        let active_id = app.active_body_dual_dialogue_group.clone();
        assert!(active_id.is_some());

        app.surface_focused = false;
        app.reconcile_body_active_group();
        assert_eq!(app.active_body_dual_dialogue_group, None);
        assert_eq!(app.session, semantic_baseline);

        app.surface_focused = true;
        app.reconcile_body_active_group();
        assert_eq!(app.active_body_dual_dialogue_group, active_id);
        app.set_active_editor_surface(EditorSurface::TitlePage);
        assert_eq!(app.active_body_dual_dialogue_group, None);
        app.swap_title_view_state();
        assert_eq!(app.active_body_dual_dialogue_group, None);
        app.swap_title_view_state();
        app.set_active_editor_surface(EditorSurface::Body);
        app.surface_focused = true;
        app.reconcile_body_active_group();
        assert_eq!(app.active_body_dual_dialogue_group, active_id);

        let revision = app.session.revision();
        let dirty = app.session.dirty();
        let history = (app.session.can_undo(), app.session.can_redo());
        app.export_pdf_with_picker(|_| crate::pdf_export::PdfExportPickerResult::Cancel);
        assert_eq!(app.active_body_dual_dialogue_group, active_id);
        assert_eq!(app.session.revision(), revision);
        assert_eq!(app.session.dirty(), dirty);
        assert_eq!((app.session.can_undo(), app.session.can_redo()), history);
    }

    #[test]
    fn document_replacement_drops_the_old_active_identity_and_print_layout_has_no_boundaries() {
        let Some(mut app) = app_with_toggle_target() else { return };
        let context = egui::Context::default();
        app.surface_focused = true;
        app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        let old_group_id = app.active_body_dual_dialogue_group.clone();
        assert!(old_group_id.is_some());
        let body_layout = app.layout_for_render(EditorSurface::Body);
        assert!(body_layout.is_ok());
        assert!(body_layout.is_ok_and(|layout| !layout.editor_group_boundaries.is_empty()));

        let print_layout = app
            .layout_cache
            .get_or_build_with_composition(
                app.session.revision(),
                app.session.project(),
                app.session.layout_inputs(),
                scriptcraft_core::layout::LayoutComposition::Print {
                    include_title_page: false,
                },
            );
        assert!(print_layout.is_ok_and(|(layout, _)| {
            layout.active_group_id.is_none() && layout.editor_group_boundaries.is_empty()
        }));

        let replacement = Project::new();
        assert!(replacement.is_ok());
        let Some(replacement) = replacement.ok() else { return };
        let text_state = app.root_text_state();
        assert!(app.activate_opened_project(
            replacement,
            std::path::Path::new("replacement.fdx"),
            None,
            None,
            None,
            text_state,
        ));
        assert_eq!(app.active_body_dual_dialogue_group, None);
        assert!(app.session.project().dual_dialogue_groups.is_empty());
        assert!(!app.surface_focused);
    }
}

#[cfg(test)]
mod dual_dialogue_navigation_dispatch_tests {
    use super::{CommandInvocation, DesktopCommandOutcome};
    use crate::{RevealIntent, ScriptCraftApp};
    use eframe::egui;
    use scriptcraft_commands::{
        Command, CommandOrigin, CommandOutcome, EditorSurface, HorizontalDirection, LineEdge,
        VerticalDirection,
    };
    use scriptcraft_core::dual_dialogue::{DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance};
    use scriptcraft_core::editing::{EditingSession, Position, Selection};
    use scriptcraft_core::{Project, Role, ScreenplayElement};

    fn navigation_app(right_text: &str) -> Option<ScriptCraftApp> {
        let mut app = ScriptCraftApp::new().ok()?;
        let mut project = Project::new().ok()?;
        project.elements = [
            (Role::Action, "outside above"),
            (Role::Character, "LEFT CUE"),
            (Role::Dialogue, "a fairly long left-side line"),
            (Role::Character, "RIGHT CUE"),
            (Role::Dialogue, right_text),
            (Role::Action, "outside below"),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (role, text))| {
            ScreenplayElement::new(format!("navigation-member-{index}"), role, text).ok()
        })
        .collect::<Option<Vec<_>>>()?;
        app.session = EditingSession::new(project).ok()?;
        app.surface_focused = true;
        app.session
            .set_selection(Selection::caret(Position { element: 1, offset: 0 }))
            .ok()?;
        let toggled = app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &egui::Context::default(),
        );
        matches!(
            toggled.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed)
        )
        .then_some(app)
    }

    fn cue_only_navigation_app() -> Option<ScriptCraftApp> {
        let mut app = ScriptCraftApp::new().ok()?;
        let mut project = Project::new().ok()?;
        project.elements = [
            (Role::Action, "outside above"),
            (Role::Character, "LEFT CUE"),
            (Role::Character, "RIGHT é"),
            (Role::Action, "outside below"),
        ].into_iter().enumerate().map(|(index, (role, text))| {
            ScreenplayElement::new(format!("cue-only-navigation-{index}"), role, text).ok()
        }).collect::<Option<Vec<_>>>()?;
        project.dual_dialogue_groups.push(DualDialogueGroup {
            id: scriptcraft_core::StableId::new("cue-only-navigation-group").ok()?,
            left: DualDialogueSide { character: project.elements[1].id.clone(), paragraphs: Vec::new() },
            right: DualDialogueSide { character: project.elements[2].id.clone(), paragraphs: Vec::new() },
            wrapper: DualDialogueWrapperProvenance::default(),
        });
        app.session = EditingSession::new(project).ok()?;
        app.surface_focused = true;
        Some(app)
    }

    fn navigation_app_with_adjacent_groups() -> Option<ScriptCraftApp> {
        let mut app = ScriptCraftApp::new().ok()?;
        let mut project = Project::new().ok()?;
        project.elements = [
            (Role::Action, "outside"),
            (Role::Character, "LEFT ONE"),
            (Role::Dialogue, "left one"),
            (Role::Character, "RIGHT ONE"),
            (Role::Dialogue, "right one"),
            (Role::Character, "LEFT TWO"),
            (Role::Dialogue, "left two"),
            (Role::Character, "RIGHT TWO"),
            (Role::Dialogue, "right two"),
            (Role::Action, "after"),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (role, text))| {
            ScreenplayElement::new(format!("adjacent-navigation-{index}"), role, text).ok()
        })
        .collect::<Option<Vec<_>>>()?;
        app.session = EditingSession::new(project).ok()?;
        app.surface_focused = true;
        let context = egui::Context::default();
        app.session
            .set_selection(Selection::caret(Position { element: 1, offset: 0 }))
            .ok()?;
        let first = app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        if !matches!(first.outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed)) {
            return None;
        }
        app.session
            .set_selection(Selection::caret(Position { element: 5, offset: 0 }))
            .ok()?;
        let second = app.dispatch_command(
            CommandInvocation::new(Command::EditorDualDialogueToggle, CommandOrigin::Menu),
            &context,
        );
        matches!(second.outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed))
            .then_some(app)
    }

    fn select(app: &mut ScriptCraftApp, selection: Selection, context: &egui::Context) {
        let _ = app.execute_navigation_selection(
            selection,
            EditorSurface::Body,
            false,
            context,
        );
    }

    fn navigate_vertical(
        app: &mut ScriptCraftApp,
        direction: VerticalDirection,
        extend: bool,
        context: &egui::Context,
    ) -> DesktopCommandOutcome {
        app.dispatch_command(
            CommandInvocation::new(
                Command::EditorNavigationVertical {
                    direction,
                    extend,
                    surface: EditorSurface::Body,
                },
                CommandOrigin::Shortcut,
            ),
            context,
        )
        .outcome
    }

    fn navigate_horizontal(
        app: &mut ScriptCraftApp,
        direction: HorizontalDirection,
        extend: bool,
        context: &egui::Context,
    ) -> DesktopCommandOutcome {
        app.dispatch_command(
            CommandInvocation::new(
                Command::EditorNavigationHorizontal {
                    direction,
                    extend,
                    surface: EditorSurface::Body,
                },
                CommandOrigin::Shortcut,
            ),
            context,
        )
        .outcome
    }

    #[test]
    fn vertical_entry_uses_group_edges_for_equal_y_columns_and_reveals_after_activation() {
        let Some(mut app) = navigation_app("右é") else { return };
        let context = egui::Context::default();
        select(
            &mut app,
            Selection::caret(Position { element: 0, offset: 0 }),
            &context,
        );
        assert_eq!(app.active_body_dual_dialogue_group, None);
        let Ok(layout) = app.layout_for_render(EditorSurface::Body) else { return };
        let left_cue = layout.pages.iter().flat_map(|page| &page.lines).find(|line| line.element == 1);
        let right_cue = layout.pages.iter().flat_map(|page| &page.lines).find(|line| line.element == 3);
        if let (Some(left), Some(right)) = (left_cue, right_cue) {
            assert_eq!(left.geometry.y, right.geometry.y);
        }
        app.preferred_vertical_x = Some(991);

        assert!(matches!(
            navigate_vertical(&mut app, VerticalDirection::Down, false, &context),
            DesktopCommandOutcome::Editor(CommandOutcome::Changed)
                | DesktopCommandOutcome::Editor(CommandOutcome::NoOp)
        ));
        assert_eq!(app.session.selection().focus, Position { element: 1, offset: 0 });
        assert!(app.active_body_dual_dialogue_group.is_some());
        assert_eq!(app.preferred_vertical_x, None);
        assert_eq!(app.reveal_intent, RevealIntent::Conditional);

        let Some(mut app) = navigation_app("右é") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 5, offset: 0 }),
            &context,
        );
        assert_eq!(app.active_body_dual_dialogue_group, None);
        assert!(matches!(
            navigate_vertical(&mut app, VerticalDirection::Up, false, &context),
            DesktopCommandOutcome::Editor(CommandOutcome::Changed)
                | DesktopCommandOutcome::Editor(CommandOutcome::NoOp)
        ));
        assert_eq!(
            app.session.selection().focus,
            Position { element: 4, offset: "右é".len() },
        );
        assert!(app.active_body_dual_dialogue_group.is_some());

        let Some(mut app) = navigation_app("") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 5, offset: 0 }),
            &context,
        );
        let _ = navigate_vertical(&mut app, VerticalDirection::Up, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 4, offset: 0 });
    }

    #[test]
    fn cue_only_navigation_enters_from_both_edges_and_uses_utf8_right_cue_end() {
        let context = egui::Context::default();
        let Some(mut app) = cue_only_navigation_app() else { return };
        select(&mut app, Selection::caret(Position { element: 0, offset: 0 }), &context);
        assert!(matches!(navigate_vertical(&mut app, VerticalDirection::Down, false, &context), DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)));
        assert_eq!(app.session.selection().focus, Position { element: 1, offset: 0 });
        assert!(app.active_body_dual_dialogue_group.is_some());

        let Some(mut app) = cue_only_navigation_app() else { return };
        select(&mut app, Selection::caret(Position { element: 3, offset: 0 }), &context);
        assert!(matches!(navigate_vertical(&mut app, VerticalDirection::Up, false, &context), DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)));
        assert_eq!(app.session.selection().focus, Position { element: 2, offset: "RIGHT é".len() });
        assert!(app.active_body_dual_dialogue_group.is_some());

        let Some(mut app) = cue_only_navigation_app() else { return };
        select(&mut app, Selection::caret(Position { element: 0, offset: "outside above".len() }), &context);
        let _ = navigate_horizontal(&mut app, HorizontalDirection::Right, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 1, offset: 0 });
        let Some(mut app) = cue_only_navigation_app() else { return };
        select(&mut app, Selection::caret(Position { element: 3, offset: 0 }), &context);
        let _ = navigate_horizontal(&mut app, HorizontalDirection::Left, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 2, offset: "RIGHT é".len() });
    }

    #[test]
    fn vertical_and_horizontal_exits_resume_columns_and_horizontal_entry_uses_nearest_edge() {
        let context = egui::Context::default();

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 1, offset: 0 }),
            &context,
        );
        app.preferred_vertical_x = Some(100);
        let _ = navigate_vertical(&mut app, VerticalDirection::Up, false, &context);
        assert_eq!(app.session.selection().focus.element, 0);
        assert_eq!(app.active_body_dual_dialogue_group, None);
        assert_eq!(app.preferred_vertical_x, None);

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 4, offset: 5 }),
            &context,
        );
        let _ = navigate_vertical(&mut app, VerticalDirection::Down, false, &context);
        assert_eq!(app.session.selection().focus.element, 5);
        assert_eq!(app.active_body_dual_dialogue_group, None);

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 1, offset: 0 }),
            &context,
        );
        let _ = navigate_horizontal(&mut app, HorizontalDirection::Left, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 0, offset: "outside above".len() });
        assert_eq!(app.active_body_dual_dialogue_group, None);

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 4, offset: 5 }),
            &context,
        );
        let _ = navigate_horizontal(&mut app, HorizontalDirection::Right, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 5, offset: 0 });
        assert_eq!(app.active_body_dual_dialogue_group, None);

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 0, offset: "outside above".len() }),
            &context,
        );
        let _ = navigate_horizontal(&mut app, HorizontalDirection::Right, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 1, offset: 0 });
        assert!(app.active_body_dual_dialogue_group.is_some());

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 5, offset: 0 }),
            &context,
        );
        let _ = navigate_horizontal(&mut app, HorizontalDirection::Left, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 4, offset: "right".len() });
        assert!(app.active_body_dual_dialogue_group.is_some());
    }

    #[test]
    fn repeated_vertical_moves_shift_focus_local_home_end_and_page_edge_goals_are_safe() {
        let Some(mut app) = navigation_app("short") else { return };
        let context = egui::Context::default();
        select(
            &mut app,
            Selection::caret(Position { element: 1, offset: 3 }),
            &context,
        );
        let _ = navigate_vertical(&mut app, VerticalDirection::Down, false, &context);
        let preferred = app.preferred_vertical_x;
        assert!(preferred.is_some());
        let _ = navigate_vertical(&mut app, VerticalDirection::Down, false, &context);
        assert_eq!(app.preferred_vertical_x, preferred);
        let focus = app.session.selection().focus;
        let text = &app.session.project().elements[focus.element].text;
        assert!(focus.offset <= text.len() && text.is_char_boundary(focus.offset));

        let current = app.session.selection().focus;
        let start = app.navigation_selection_line_edge(LineEdge::Start, true);
        assert_eq!(start.anchor, app.session.selection().anchor);
        assert_eq!(start.focus, Position { element: current.element, offset: 0 });
        let end = app.navigation_selection_line_edge(LineEdge::End, true);
        assert_eq!(end.focus.element, current.element);
        assert_eq!(end.focus.offset, app.session.project().elements[current.element].text.len());

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection {
                anchor: Position { element: 0, offset: 0 },
                focus: Position { element: 0, offset: "outside above".len() },
            },
            &context,
        );
        let _ = navigate_vertical(&mut app, VerticalDirection::Down, true, &context);
        assert_eq!(app.session.selection().anchor, Position { element: 0, offset: 0 });
        assert_eq!(app.session.selection().focus, Position { element: 1, offset: 0 });
        assert!(app.active_body_dual_dialogue_group.is_some());

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection {
                anchor: Position { element: 4, offset: 5 },
                focus: Position { element: 1, offset: 0 },
            },
            &context,
        );
        let _ = navigate_vertical(&mut app, VerticalDirection::Up, true, &context);
        assert_eq!(app.session.selection().anchor, Position { element: 4, offset: 5 });
        assert_eq!(app.session.selection().focus.element, 0);
        assert_eq!(app.active_body_dual_dialogue_group, None);

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 5, offset: "outside below".len() }),
            &context,
        );
        app.preferred_vertical_x = Some(300);
        let _ = app.navigation_selection_vertical(VerticalDirection::Down, false, &context);
        assert_eq!(app.preferred_vertical_x, None);
        assert_eq!(app.session.selection().focus.element, 5);
    }

    #[test]
    fn final_right_enter_and_pattern_unwrap_reconcile_active_mode_without_navigation_edits() {
        let Some(mut app) = navigation_app("right") else { return };
        let context = egui::Context::default();
        select(
            &mut app,
            Selection::caret(Position { element: 4, offset: "right".len() }),
            &context,
        );
        let enter = app.dispatch_command(
            CommandInvocation::new(Command::EditorParagraphEnter, CommandOrigin::Shortcut),
            &context,
        );
        assert!(matches!(
            enter.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed)
        ));
        assert_eq!(app.session.project().dual_dialogue_groups.len(), 1);
        assert_eq!(app.active_body_dual_dialogue_group, None);
        let focus = app.session.selection().focus;
        assert!(scriptcraft_core::dual_dialogue::group_at_element_index(
            app.session.project(),
            focus.element,
        )
        .is_none());

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 2, offset: 0 }),
            &context,
        );
        let changed = app.dispatch_command(
            CommandInvocation::new(
                Command::EditorElementChange {
                    role: scriptcraft_commands::RoleValue::new(Role::Action),
                    surface: EditorSurface::Body,
                },
                CommandOrigin::Menu,
            ),
            &context,
        );
        assert!(matches!(changed.outcome, DesktopCommandOutcome::Editor(CommandOutcome::Changed)));
        assert!(app.session.project().dual_dialogue_groups.is_empty());
        assert_eq!(app.active_body_dual_dialogue_group, None);
    }

    #[test]
    fn adjacent_groups_enter_at_the_directional_edge_and_internal_enter_retains_activation() {
        let context = egui::Context::default();
        let Some(mut app) = navigation_app_with_adjacent_groups() else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 4, offset: "right one".len() }),
            &context,
        );
        let second_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            app.session.project(),
            5,
        )
        .map(|group| group.id.clone());
        let _ = navigate_vertical(&mut app, VerticalDirection::Down, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 5, offset: 0 });
        assert_eq!(app.active_body_dual_dialogue_group, second_group);

        let Some(mut app) = navigation_app_with_adjacent_groups() else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 4, offset: "right one".len() }),
            &context,
        );
        let _ = navigate_horizontal(&mut app, HorizontalDirection::Right, false, &context);
        assert_eq!(app.session.selection().focus, Position { element: 5, offset: 0 });
        assert!(app.active_body_dual_dialogue_group.is_some());

        let Some(mut app) = navigation_app("right") else { return };
        select(
            &mut app,
            Selection::caret(Position { element: 2, offset: 2 }),
            &context,
        );
        let internal_enter = app.dispatch_command(
            CommandInvocation::new(Command::EditorParagraphEnter, CommandOrigin::Shortcut),
            &context,
        );
        assert!(matches!(
            internal_enter.outcome,
            DesktopCommandOutcome::Editor(CommandOutcome::Changed)
        ));
        assert!(app.active_body_dual_dialogue_group.is_some());
        assert!(scriptcraft_core::dual_dialogue::group_at_element_index(
            app.session.project(),
            app.session.selection().focus.element,
        )
        .is_some());
    }
}

#[cfg(all(test, target_os = "windows"))]
mod windows_pdf_command_tests {
    use super::{DesktopCommandOutcome, ScriptCraftApp};
    use crate::pdf_export::PdfExportPickerResult;
    use eframe::egui;
    use scriptcraft_commands::Command;

    #[test]
    fn windows_pdf_command_uses_direct_picker_without_opening_egui_options() {
        let Ok(mut app) = ScriptCraftApp::new() else {
            return;
        };
        let context = egui::Context::default();

        let outcome = app.dispatch_command_inner_with_pdf_handlers(
            Command::FileSaveAsPdf,
            &context,
            |app| {
                app.export_pdf_with_picker(|filename| {
                    assert_eq!(filename, crate::UNTITLED_PDF_FILENAME);
                    PdfExportPickerResult::Cancel
                })
            },
            |_, _| {},
            |_| {},
        );

        assert!(matches!(outcome, DesktopCommandOutcome::EffectDispatched));
        assert!(app.app_command_availability().is_enabled(
            scriptcraft_commands::CommandId::FileNew
        ));
    }
}
