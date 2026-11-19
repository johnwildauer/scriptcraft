//! Script View Body navigation selection geometry.

use super::{paint::ordered_selection, ScriptCraftApp};
use crate::app::add_performance_nanos;
use crate::presentation::geometry as presentation_geometry;
use eframe::egui;
use scriptcraft_commands::{HorizontalDirection, LineEdge, VerticalDirection};
use scriptcraft_layout::InteractionQueryDiagnostics;
use scriptcraft_session::editing::{EditError, Position, Selection};
use std::time::Instant;

fn dual_group_navigation_edge(
    project: &scriptcraft_model::Project,
    group: &scriptcraft_model::dual_dialogue::DualDialogueGroup,
    forward: bool,
) -> Option<Position> {
    let element = if forward {
        scriptcraft_model::dual_dialogue::first_member(project, group)?
    } else {
        scriptcraft_model::dual_dialogue::last_member(project, group)?
    };
    let offset = if forward {
        0
    } else {
        project.elements.get(element)?.text.len()
    };
    Some(Position { element, offset })
}

impl ScriptCraftApp {
    pub(crate) fn navigation_selection_horizontal(
        &mut self,
        direction: HorizontalDirection,
        extend: bool,
    ) -> Selection {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        if selection.anchor != selection.focus && !extend {
            let position = if direction == HorizontalDirection::Right {
                ordered_selection(selection).1
            } else {
                ordered_selection(selection).0
            };
            return Selection::caret(position);
        }
        let position = selection.focus;
        let project = self.session.project();
        let origin_group =
            scriptcraft_model::dual_dialogue::group_at_element_index(project, position.element);
        let item = &project.elements[position.element];
        let next = if direction == HorizontalDirection::Right {
            if position.offset < item.text.len() {
                Position {
                    offset: item.text[position.offset..]
                        .chars()
                        .next()
                        .map_or(position.offset, |c| position.offset + c.len_utf8()),
                    ..position
                }
            } else if position.element + 1 < project.elements.len() {
                Position {
                    element: position.element + 1,
                    offset: 0,
                }
            } else {
                position
            }
        } else if position.offset > 0 {
            Position {
                offset: item.text[..position.offset]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(at, _)| at),
                ..position
            }
        } else if position.element > 0 {
            let previous = &project.elements[position.element - 1];
            Position {
                element: position.element - 1,
                offset: previous.text.len(),
            }
        } else {
            position
        };
        let target_group =
            scriptcraft_model::dual_dialogue::group_at_element_index(project, next.element);
        let next = if let Some(group) =
            target_group.filter(|group| origin_group.map_or(true, |origin| origin.id != group.id))
        {
            dual_group_navigation_edge(project, group, direction == HorizontalDirection::Right)
                .unwrap_or(next)
        } else {
            next
        };
        if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        }
    }

    pub(crate) fn navigation_selection_vertical(
        &mut self,
        direction: VerticalDirection,
        extend: bool,
        ctx: &egui::Context,
    ) -> Result<Selection, EditError> {
        let selection = self.session.selection();
        let current = selection.focus;
        let layout = match self.layout_for_render(self.active_editor_surface()) {
            Ok(layout) => layout,
            Err(_) => {
                self.preferred_vertical_x = None;
                return Err(EditError::InvalidElement);
            }
        };
        let interaction_started = Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let current_index = layout.body_line_index_for_position_with_diagnostics(
            current.element,
            current.offset,
            &mut query_diagnostics,
        );
        self.performance.indexed_interaction_queries = self
            .performance
            .indexed_interaction_queries
            .saturating_add(1);
        self.performance.interaction_index_entries_examined = self
            .performance
            .interaction_index_entries_examined
            .saturating_add(query_diagnostics.index_entries_examined);
        add_performance_nanos(
            &mut self.performance.caret_lookup_nanos,
            interaction_started.elapsed().as_nanos(),
        );
        let Some(current_index) = current_index else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let down = direction == VerticalDirection::Down;
        let target_index = if down {
            current_index
                .checked_add(1)
                .filter(|index| *index < layout.body_line_count())
        } else {
            current_index.checked_sub(1)
        };
        let Some(current_line) = layout.body_line(current_index) else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let Some(target_index) = target_index else {
            self.preferred_vertical_x = None;
            let next = Position {
                element: current_line.element,
                offset: if down {
                    current_line.source.end
                } else {
                    current_line.source.start
                },
            };
            return Ok(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        };
        let Some(target_line) = layout.body_line(target_index) else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let project = self.session.project();
        let origin_group =
            scriptcraft_model::dual_dialogue::group_at_element_index(project, current.element);
        let target_group =
            scriptcraft_model::dual_dialogue::group_at_element_index(project, target_line.element);
        let mode_transition = match (origin_group, target_group) {
            (Some(origin), Some(target)) => origin.id != target.id,
            (None, Some(_)) | (Some(_), None) => true,
            (None, None) => false,
        };
        if let Some(group) = target_group.filter(|_| mode_transition) {
            self.preferred_vertical_x = None;
            let next =
                dual_group_navigation_edge(project, group, direction == VerticalDirection::Down)
                    .unwrap_or(Position {
                        element: target_line.element,
                        offset: target_line.source.start,
                    });
            return Ok(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        }
        let scale = self.page_scale.unwrap_or(1.0);
        let current_projection = self
            .project_line_for_interaction(ctx, current_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let target_projection = self
            .project_line_for_interaction(ctx, target_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let existing_goal = self.preferred_vertical_x;
        let current_fallback = current_projection
            .is_none()
            .then(|| presentation_geometry::project_line(current_line, scale).ok());
        let current_projection = current_projection
            .as_ref()
            .or_else(|| current_fallback.as_ref().and_then(Option::as_ref));
        let goal = existing_goal.unwrap_or_else(|| {
            presentation_geometry::preferred_vertical_document_x(
                current_line,
                current.offset,
                current_projection,
                scale,
            )
            .unwrap_or(current_line.geometry.x.raw())
        });
        self.preferred_vertical_x = Some(goal);
        let target_fallback = target_projection
            .is_none()
            .then(|| presentation_geometry::project_line(target_line, scale).ok());
        let target_projection = target_projection
            .as_ref()
            .or_else(|| target_fallback.as_ref().and_then(Option::as_ref));
        let target_display = target_projection
            .and_then(|projection| {
                presentation_geometry::projected_target_boundary(projection, goal, scale)
            })
            .ok_or(EditError::InvalidElement)?;
        let next = Position {
            element: target_line.element,
            offset: target_line.source_for_display_char(target_display),
        };
        Ok(if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    pub(crate) fn navigation_selection_line_edge(
        &mut self,
        edge: LineEdge,
        extend: bool,
    ) -> Selection {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        let position = if edge == LineEdge::End {
            Position {
                element: selection.focus.element,
                offset: self.session.project().elements[selection.focus.element]
                    .text
                    .len(),
            }
        } else {
            Position {
                element: selection.focus.element,
                offset: 0,
            }
        };
        if extend {
            Selection {
                anchor: selection.anchor,
                focus: position,
            }
        } else {
            Selection::caret(position)
        }
    }
}
