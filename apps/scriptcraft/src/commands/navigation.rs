use super::DesktopCommandOutcome;
use crate::app::ScriptCraftApp;
use eframe::egui;
use scriptcraft_commands::{
    pointer_selection, Command, CommandId, CommandOutcome, CommandRejection, EditorSurface,
    HorizontalDirection, LineEdge, PointerPhase, Position as CommandPosition,
    Selection as CommandSelection, VerticalDirection,
};
use scriptcraft_session::editing::{EditError, Position, Selection};

impl ScriptCraftApp {
    pub(super) fn active_selection(&self) -> Selection {
        match self.active_editor_surface() {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        }
    }

    pub(super) fn dispatch_navigation_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        match command {
            Command::EditorSelectionPointer {
                position,
                phase,
                surface,
            } => self.dispatch_pointer_selection(position, phase, surface, ctx),
            Command::EditorNavigationHorizontal {
                direction,
                extend,
                surface,
            } => {
                let previous_vertical_goal = self.preferred_vertical_x_for(surface);
                let selection =
                    self.navigation_selection_horizontal_for(surface, direction, extend);
                let outcome = self.execute_navigation_selection(selection, surface, false, ctx);
                if matches!(&outcome, DesktopCommandOutcome::Rejected(_)) {
                    self.set_preferred_vertical_x_for(surface, previous_vertical_goal);
                }
                outcome
            }
            Command::EditorNavigationVertical {
                direction,
                extend,
                surface,
            } => {
                let previous_vertical_goal = self.preferred_vertical_x_for(surface);
                match self.navigation_selection_vertical_for(surface, direction, extend, ctx) {
                    Ok(selection) => {
                        let outcome =
                            self.execute_navigation_selection(selection, surface, true, ctx);
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
            Command::EditorNavigationLineEdge {
                edge,
                extend,
                surface,
            } => {
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
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    fn navigation_selection_horizontal_for(
        &mut self,
        surface: EditorSurface,
        direction: HorizontalDirection,
        extend: bool,
    ) -> Selection {
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
                let offset = paragraph.text[..focus.offset]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                Position {
                    element: focus.element,
                    offset,
                }
            }
            HorizontalDirection::Left if focus.element > 0 => {
                let previous = &self.session.project().title_page[focus.element - 1];
                Position {
                    element: focus.element - 1,
                    offset: previous.text.len(),
                }
            }
            HorizontalDirection::Right if focus.offset < paragraph.text.len() => {
                let amount = paragraph.text[focus.offset..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(0);
                Position {
                    element: focus.element,
                    offset: focus.offset + amount,
                }
            }
            HorizontalDirection::Right
                if focus.element + 1 < self.session.project().title_page.len() =>
            {
                Position {
                    element: focus.element + 1,
                    offset: 0,
                }
            }
            _ => focus,
        };
        if extend {
            Selection {
                anchor: current.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        }
    }

    fn navigation_selection_vertical_for(
        &mut self,
        surface: EditorSurface,
        direction: VerticalDirection,
        extend: bool,
        ctx: &egui::Context,
    ) -> Result<Selection, EditError> {
        if surface == EditorSurface::Body {
            return self.navigation_selection_vertical(direction, extend, ctx);
        }
        let current = self.session.title_selection();
        let focus = current.focus;
        let layout = self
            .layout_for_render(EditorSurface::TitlePage)
            .map_err(|_| EditError::InvalidElement)?;
        let Some(current_index) = layout.title_line_index_for_position(focus.element, focus.offset)
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
                Selection {
                    anchor: current.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
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
            crate::presentation::geometry::preferred_vertical_document_x(
                current_line,
                focus.offset,
                current_projection.as_ref(),
                scale,
            )
            .unwrap_or(current_line.geometry.x.raw())
        });
        self.set_preferred_vertical_x_for(surface, Some(goal));
        let target_projection = target_projection
            .as_ref()
            .ok_or(EditError::InvalidElement)?;
        let display_boundary = crate::presentation::geometry::projected_target_boundary(
            target_projection,
            goal,
            scale,
        )
        .ok_or(EditError::InvalidElement)?;
        let next = Position {
            element: target_line.element,
            offset: target_line.source_for_display_char(display_boundary),
        };
        Ok(if extend {
            Selection {
                anchor: current.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    fn navigation_selection_line_edge_for(
        &mut self,
        surface: EditorSurface,
        edge: LineEdge,
        extend: bool,
    ) -> Selection {
        if surface == EditorSurface::Body {
            return self.navigation_selection_line_edge(edge, extend);
        }
        self.set_preferred_vertical_x_for(surface, None);
        let current = self.session.title_selection();
        let offset = match edge {
            LineEdge::Start => 0,
            LineEdge::End => self
                .session
                .project()
                .title_page
                .get(current.focus.element)
                .map(|paragraph| paragraph.text.len())
                .unwrap_or(0),
        };
        let focus = Position {
            element: current.focus.element,
            offset,
        };
        if extend {
            Selection {
                anchor: current.anchor,
                focus,
            }
        } else {
            Selection::caret(focus)
        }
    }

    /// Returns egui focus to the one screenplay interaction surface after an
    /// Edit command, without making the command target depend on popup focus.
    pub(crate) fn restore_editor_focus(&mut self, ctx: &egui::Context) {
        if self.active_editor_surface == EditorSurface::TitlePage && !self.title_view_state_swapped
        {
            ctx.send_viewport_cmd_to(
                crate::views::title_page::viewport_id(),
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

    fn execute_navigation_selection(
        &mut self,
        selection: Selection,
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
            crate::views::script::RevealIntent::Conditional,
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
        let drag_current = Selection {
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
            self.reveal_intent = crate::views::script::RevealIntent::None;
            self.clear_error();
        }
        outcome
    }
}

pub(super) fn command_restores_editor_focus(command: CommandId) -> bool {
    matches!(
        command,
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
    )
}
