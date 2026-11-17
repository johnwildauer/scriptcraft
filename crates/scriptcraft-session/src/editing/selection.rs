//! Surface selection validation, commits, and save-time remapping.

use super::{EditError, EditingSession, Position, Selection};
use scriptcraft_model::formatted_text::{project_style_at_caret, RunBoundaryAffinity};
use scriptcraft_model::Project;

pub(super) trait SelectionOrdering {
    fn ordered(self) -> (Position, Position);
}

impl SelectionOrdering for Selection {
    fn ordered(self) -> (Position, Position) {
        if (self.anchor.element, self.anchor.offset) <= (self.focus.element, self.focus.offset) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }
}

impl EditingSession {
    pub fn set_selection(&mut self, selection: Selection) -> Result<(), EditError> {
        validate_selection(&self.project, selection)?;
        let previous_selection = self.selection;
        let previous_typing_style = self.body_typing_style;
        let next_typing_style = if selection.anchor == selection.focus {
            let element = &self.project.elements[selection.focus.element];
            project_style_at_caret(
                &element.text,
                &element.runs,
                selection.focus.offset,
                RunBoundaryAffinity::Right,
                previous_typing_style,
            )
            .map_err(EditError::FormattedText)?
        } else {
            previous_typing_style
        };

        self.selection = selection;
        self.body_typing_style = next_typing_style;
        if self.selection != previous_selection || self.body_typing_style != previous_typing_style {
            self.mark_mutated();
        }
        Ok(())
    }

    /// Sets a title-page-local selection without affecting the body caret,
    /// history, revision, or dirty state.
    pub fn set_title_selection(&mut self, selection: Selection) -> Result<(), EditError> {
        validate_title_selection(&self.project, selection)?;
        let previous_selection = self.title_selection;
        let previous_typing_style = self.title_typing_style;
        let next_typing_style = if selection.anchor == selection.focus {
            if let Some(paragraph) = self.project.title_page.get(selection.focus.element) {
                project_style_at_caret(
                    &paragraph.text,
                    &paragraph.runs,
                    selection.focus.offset,
                    RunBoundaryAffinity::Right,
                    previous_typing_style,
                )
                .map_err(EditError::FormattedText)?
            } else {
                previous_typing_style
            }
        } else {
            previous_typing_style
        };

        self.title_selection = selection;
        self.title_typing_style = next_typing_style;
        if self.title_selection != previous_selection
            || self.title_typing_style != previous_typing_style
        {
            self.mark_mutated();
        }
        Ok(())
    }
}

pub(crate) fn map_selection_to_project(
    selection: Selection,
    old_project: &Project,
    new_project: &Project,
) -> Option<Selection> {
    fn map_position(position: Position, old: &Project, new: &Project) -> Option<Position> {
        if new.elements.is_empty() {
            return None;
        }
        let old_element = old.elements.get(position.element);
        let old_source = old_element.and_then(|element| element.source_provenance.as_ref());
        let source_id = old_source.and_then(|provenance| provenance.foreign_id.as_deref());
        let index = if let Some(source_id) = source_id {
            if old.elements.iter().filter(|element| {
                element.source_provenance.as_ref()
                    .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(source_id)
            }).count() != 1 {
                return None;
            }
            let matches = new.elements.iter().enumerate().filter(|(_, element)| {
                element.source_provenance.as_ref().is_some_and(|provenance| {
                    provenance.foreign_id.as_deref() == Some(source_id)
                        && old_source.is_some_and(|old| {
                            provenance.format == old.format
                                && provenance.section == old.section
                                && provenance.foreign_role == old.foreign_role
                        })
                })
            }).map(|(index, _)| index).collect::<Vec<_>>();
            let [index] = matches.as_slice() else { return None };
            *index
        } else {
            let old_element = old_element?;
            let stable_matches = if old_source.is_none() {
                new.elements.iter().enumerate().filter(|(_, element)| {
                    element.id == old_element.id
                }).map(|(index, _)| index).collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if let [index] = stable_matches.as_slice() {
                *index
            } else {
                // Source-less projects acquire FDX identities during Save As.
                // Map them by a unique ordered semantic occurrence, never by a
                // bare paragraph index that could now refer to unrelated text.
                let old_occurrences = old.elements.iter().enumerate().filter(|(_, element)| {
                    element.role == old_element.role && element.text == old_element.text
                }).map(|(index, _)| index).collect::<Vec<_>>();
                let occurrence = old_occurrences.iter().position(|index| *index == position.element)?;
                let new_occurrences = new.elements.iter().enumerate().filter(|(_, element)| {
                    element.role == old_element.role && element.text == old_element.text
                }).map(|(index, _)| index).collect::<Vec<_>>();
                if old_occurrences.len() != new_occurrences.len() {
                    return None;
                }
                *new_occurrences.get(occurrence)?
            }
        };
        let element = &new.elements[index];
        let mut offset = position.offset.min(element.text.len());
        while !element.text.is_char_boundary(offset) {
            offset = offset.saturating_sub(1);
        }
        Some(Position { element: index, offset })
    }
    Some(Selection {
        anchor: map_position(selection.anchor, old_project, new_project)?,
        focus: map_position(selection.focus, old_project, new_project)?,
    })
}

pub(crate) fn map_title_selection_to_project(
    selection: Selection,
    old_project: &Project,
    new_project: &Project,
) -> Selection {
    fn map_position(position: Position, old: &Project, new: &Project) -> Position {
        if new.title_page.is_empty() {
            return Position { element: 0, offset: 0 };
        }
        let old_paragraph = old.title_page.get(position.element);
        let source_id = old_paragraph
            .and_then(|paragraph| paragraph.source_provenance.as_ref())
            .and_then(|provenance| provenance.foreign_id.as_deref());
        let unique_source_id = source_id.filter(|source_id| {
            old.title_page.iter().filter(|paragraph| {
                paragraph.source_provenance.as_ref()
                    .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(*source_id)
            }).count() == 1
                && new.title_page.iter().filter(|paragraph| {
                    paragraph.source_provenance.as_ref()
                        .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(*source_id)
                }).count() == 1
        });
        let index = unique_source_id
            .and_then(|source_id| new.title_page.iter().position(|paragraph| {
                paragraph.source_provenance.as_ref()
                    .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(source_id)
            }))
            .or_else(|| {
                let old_paragraph = old_paragraph?;
                if old_paragraph.source_provenance.is_some() {
                    return None;
                }
                new.title_page.iter().position(|paragraph| {
                    paragraph.id == old_paragraph.id
                        && paragraph.source_provenance.as_ref()
                            .is_none_or(|provenance| provenance.foreign_id.is_none())
                })
            })
            .unwrap_or_else(|| position.element.min(new.title_page.len() - 1));
        let paragraph = &new.title_page[index];
        let mut offset = position.offset.min(paragraph.text.len());
        while !paragraph.text.is_char_boundary(offset) {
            offset = offset.saturating_sub(1);
        }
        Position { element: index, offset }
    }
    Selection {
        anchor: map_position(selection.anchor, old_project, new_project),
        focus: map_position(selection.focus, old_project, new_project),
    }
}

pub(super) fn validate_position(project: &Project, position: Position) -> Result<(), EditError> {
    let element = project
        .elements
        .get(position.element)
        .ok_or(EditError::InvalidElement)?;
    if element.is_boundary(position.offset) {
        Ok(())
    } else {
        Err(EditError::InvalidUtf8Boundary)
    }
}

pub(super) fn validate_selection(project: &Project, selection: Selection) -> Result<(), EditError> {
    validate_position(project, selection.anchor)?;
    validate_position(project, selection.focus)
}

pub(super) fn validate_title_position(
    project: &Project,
    position: Position,
) -> Result<(), EditError> {
    if project.title_page.is_empty() && position == (Position { element: 0, offset: 0 }) {
        return Ok(());
    }
    let paragraph = project
        .title_page
        .get(position.element)
        .ok_or(EditError::InvalidElement)?;
    if paragraph.is_boundary(position.offset) {
        Ok(())
    } else {
        Err(EditError::InvalidUtf8Boundary)
    }
}

pub(super) fn validate_title_selection(
    project: &Project,
    selection: Selection,
) -> Result<(), EditError> {
    validate_title_position(project, selection.anchor)?;
    validate_title_position(project, selection.focus)
}
