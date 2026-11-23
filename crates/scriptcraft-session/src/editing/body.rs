//! Body text insertion and deletion operations.

use super::candidate::MutationValidationScope;
use super::policy::is_delimiter_only_parenthetical;
use super::selection::SelectionOrdering;
use super::structure::{join_at, remove_body_owner, update_preserved_type};
use super::text::{next_boundary, previous_boundary, replace_range, synchronize_body_typing_style};
use super::{EditError, EditingSession, Position, Selection};
use scriptcraft_model::{formatted_text::{set_paragraph_run_formatting, ParagraphTarget}, BodyOwner, ElementTypeKey, Project};
use std::collections::BTreeSet;

impl EditingSession {
    /// Inserts or replaces text while retaining the existing element identity.
    /// Mapped source runs and text leaves are transformed in the same candidate;
    /// unsafe inline intersections reject the whole operation.
    pub fn insert(&mut self, text: &str) -> Result<(), EditError> {
        let typing = self.body_typing_style;
        let inline_typing = self.body_typing_formatting;
        let (start, end) = self.selection.ordered();
        let operation = |project: &mut Project,
                         selection: &Selection,
                         _generated: &mut BTreeSet<String>,
                         _policy: &super::EditPolicy| {
            let (start, end) = selection.ordered();
            let insertion_style = if start == end {
                typing
            } else {
                let element = project
                    .elements
                    .get(start.element)
                    .ok_or(EditError::InvalidElement)?;
                scriptcraft_model::formatted_text::project_style_at_caret(
                    &element.text,
                    &element.runs,
                    start.offset,
                    scriptcraft_model::formatted_text::RunBoundaryAffinity::Right,
                    typing,
                )
                .map_err(EditError::FormattedText)?
            };
            replace_range(project, start, end, text)?;
            if !text.is_empty() {
                synchronize_body_typing_style(project, start, text, insertion_style)?;
                if start == end && !inline_typing.is_empty() {
                    set_paragraph_run_formatting(
                        project,
                        &[(ParagraphTarget::Body(start.element), start.offset..start.offset.saturating_add(text.len()))],
                        inline_typing,
                    ).map_err(EditError::FormattedText)?;
                }
            }
            Ok(Selection::caret(Position {
                element: start.element,
                offset: start.offset + text.len(),
            }))
        };
        if start.element != end.element {
            self.mutate_structural(MutationValidationScope::SingleParagraph, operation)
        } else {
            self.mutate(MutationValidationScope::SingleParagraph, operation)
        }
    }

    /// Inserts clipboard text as one transactional semantic edit.
    ///
    /// Newline characters remain in the active element. Layout is responsible
    /// for visual line breaks; paste does not silently invent screenplay roles
    /// or multiple undo entries.
    pub fn insert_paste(&mut self, text: &str) -> Result<(), EditError> {
        self.insert(text)
    }

    /// Deletes text or, when the selection spans paragraph boundaries, removes
    /// the selected source-anchored paragraphs transactionally.
    pub fn delete(&mut self) -> Result<(), EditError> {
        let (selection_start, selection_end) = self.selection.ordered();
        let generated_scene_heading_deletions = if selection_start.element < selection_end.element {
            self.project
                .elements
                .iter()
                .enumerate()
                .filter(|(index, element)| {
                    *index > selection_start.element
                        && *index <= selection_end.element
                        && has_scene_heading_capability(&self.project, element)
                        && (*index < selection_end.element
                            || selection_end.offset == element.text.len())
                })
                .map(|(_, element)| element.id.clone())
                .collect()
        } else if selection_start == selection_end {
            deletion_candidate_empty_element(
                &self.project,
                selection_start,
                DeleteDirection::Forward,
            )
            .into_iter()
            .filter_map(|index| self.project.elements.get(index))
            .filter(|element| has_scene_heading_capability(&self.project, element))
            .map(|element| element.id.clone())
            .collect()
        } else {
            BTreeSet::new()
        };
        self.mutate_structural_with_scene_heading_deletions(
            MutationValidationScope::SingleParagraph,
            generated_scene_heading_deletions,
            |project, selection, generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end {
                    replace_range(project, start, end, "")?;
                    return Ok(Selection::caret(start));
                }
                let item = project
                    .elements
                    .get(start.element)
                    .ok_or(EditError::InvalidElement)?;
                if item.text.is_empty() {
                    if project.elements.len() == 1 {
                        return Ok(Selection::caret(start));
                    }
                    return Ok(remove_empty_body_element(
                        project,
                        start.element,
                        DeleteDirection::Forward,
                    ));
                }
                let has_parenthetical_capability = project
                    .element_semantics(&item.element_type())
                    .is_some_and(|semantics| semantics.capabilities().parenthetical);
                let after_closing_delimiter =
                    item.text.ends_with(')') && start.offset == item.text.len();
                if has_parenthetical_capability
                    && is_delimiter_only_parenthetical(item.text.as_str())
                    && !after_closing_delimiter
                {
                    let item_text_len = item.text.len();
                    let id = item.id.as_str().to_owned();
                    if start.element == 0 || project.elements.len() == 1 {
                        let Some(element_type) = unique_dialogue_capability_key(project) else {
                            return Ok(Selection::caret(start));
                        };
                        replace_range(
                            project,
                            Position {
                                element: 0,
                                offset: 0,
                            },
                            Position {
                                element: 0,
                                offset: item_text_len,
                            },
                            "",
                        )?;
                        update_preserved_type(project, 0, &element_type)?;
                        BodyOwner::update(&mut project.elements[0], |element| {
                            element.set_element_type(element_type);
                        });
                        generated.remove(&id);
                        return Ok(Selection::caret(Position {
                            element: 0,
                            offset: 0,
                        }));
                    }
                    let previous = start.element.saturating_sub(1);
                    let offset = project.elements[previous].text.len();
                    generated.remove(&id);
                    remove_body_owner(project, start.element);
                    return Ok(Selection::caret(Position {
                        element: previous,
                        offset,
                    }));
                }
                if start.offset < item.text.len() {
                    let next = next_boundary(&item.text, start.offset);
                    replace_range(
                        project,
                        start,
                        Position {
                            offset: next,
                            ..start
                        },
                        "",
                    )?;
                } else if start.element + 1 < project.elements.len() {
                    let offset = item.text.len();
                    if project.elements[start.element + 1].text.is_empty() {
                        remove_body_owner(project, start.element + 1);
                    } else {
                        join_at(project, start.element)?;
                    }
                    return Ok(Selection::caret(Position {
                        element: start.element,
                        offset,
                    }));
                }
                Ok(Selection::caret(start))
            },
        )
    }

    pub fn backspace(&mut self) -> Result<(), EditError> {
        let (start, end) = self.selection.ordered();
        let generated_scene_heading_deletions = if start == end {
            deletion_candidate_empty_element(&self.project, start, DeleteDirection::Backward)
                .into_iter()
                .filter_map(|index| self.project.elements.get(index))
                .filter(|element| has_scene_heading_capability(&self.project, element))
                .map(|element| element.id.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        self.mutate_structural_with_scene_heading_deletions(
            MutationValidationScope::SingleParagraph,
            generated_scene_heading_deletions,
            |project, selection, _generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end {
                    replace_range(project, start, end, "")?;
                    return Ok(Selection::caret(start));
                }
                let item = project
                    .elements
                    .get(start.element)
                    .ok_or(EditError::InvalidElement)?;
                if start.offset == 0 && item.text.is_empty() {
                    if project.elements.len() == 1 {
                        return Ok(Selection::caret(start));
                    }
                    return Ok(remove_empty_body_element(
                        project,
                        start.element,
                        DeleteDirection::Backward,
                    ));
                }
                if start.offset > 0 {
                    let previous = previous_boundary(&item.text, start.offset);
                    let from = Position {
                        offset: previous,
                        ..start
                    };
                    replace_range(project, from, start, "")?;
                    return Ok(Selection::caret(from));
                }
                if start.element > 0 {
                    let previous = project
                        .elements
                        .get(start.element - 1)
                        .ok_or(EditError::InvalidElement)?;
                    if previous.text.is_empty() {
                        remove_body_owner(project, start.element - 1);
                        return Ok(Selection::caret(Position {
                            element: start.element - 1,
                            offset: 0,
                        }));
                    }
                    let offset = previous.text.len();
                    join_at(project, start.element - 1)?;
                    return Ok(Selection::caret(Position {
                        element: start.element - 1,
                        offset,
                    }));
                }
                Ok(Selection::caret(start))
            },
        )
    }
}

fn has_scene_heading_capability(project: &Project, element: &BodyOwner) -> bool {
    project
        .element_semantics(&element.element_type())
        .is_some_and(|semantics| semantics.capabilities().scene_boundary)
}

fn unique_dialogue_capability_key(project: &Project) -> Option<ElementTypeKey> {
    let mut dialogue_keys = project
        .element_definitions()
        .iter()
        .filter_map(|definition| {
            project
                .element_semantics(&definition.key)
                .is_some_and(|semantics| semantics.capabilities().dialogue)
                .then(|| definition.key.clone())
        });
    let key = dialogue_keys.next()?;
    dialogue_keys.next().is_none().then_some(key)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeleteDirection {
    Forward,
    Backward,
}

fn deletion_candidate_empty_element(
    project: &Project,
    position: Position,
    direction: DeleteDirection,
) -> Option<usize> {
    let current = project.elements.get(position.element)?;
    if current.text.is_empty() && project.elements.len() > 1 {
        return Some(position.element);
    }
    match direction {
        DeleteDirection::Forward if position.offset == current.text.len() => {
            let next_index = position.element.checked_add(1)?;
            project
                .elements
                .get(next_index)
                .is_some_and(|element| element.text.is_empty())
                .then_some(next_index)
        }
        DeleteDirection::Backward if position.offset == 0 && position.element > 0 => {
            let previous_index = position.element - 1;
            project.elements[previous_index]
                .text
                .is_empty()
                .then_some(previous_index)
        }
        _ => None,
    }
}

fn remove_empty_body_element(
    project: &mut Project,
    index: usize,
    direction: DeleteDirection,
) -> Selection {
    let (caret_element, caret_offset) = match direction {
        DeleteDirection::Forward if index + 1 < project.elements.len() => (index, 0),
        DeleteDirection::Forward => (index - 1, project.elements[index - 1].text.len()),
        DeleteDirection::Backward if index > 0 => {
            (index - 1, project.elements[index - 1].text.len())
        }
        DeleteDirection::Backward => (index, 0),
    };
    remove_body_owner(project, index);
    Selection::caret(Position {
        element: caret_element,
        offset: caret_offset,
    })
}
