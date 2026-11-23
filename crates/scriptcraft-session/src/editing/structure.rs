//! Body structure operations, type mutation, and structural candidate helpers.

use super::candidate::MutationValidationScope;
use super::policy::{resolve_enter, resolve_tab, EnterAction, TabAction};
use super::selection::SelectionOrdering;
use super::text::{effective_runs, normalize_run_sequence, replace_range, slice_runs};
use super::{
    EditError, EditingSession, ParagraphKeyTarget, ParagraphKeyTargets, Position, Selection,
};
use scriptcraft_model::{
    dual_dialogue, preservation::PreservationError, BodyOwner, ElementTypeKey, ModelError, Project,
    ScreenplayElement, StableId, MAX_ELEMENTS, MAX_TEXT_BYTES,
};

impl EditingSession {
    /// Projects the exact Behavior decisions used by [`Self::enter`] and
    /// [`Self::tab`] without executing edits, touching history, or changing
    /// dirty/revision state. The result intentionally does not guarantee that
    /// preservation or capacity checks will accept a later edit.
    pub fn paragraph_key_targets(&self) -> ParagraphKeyTargets {
        let enter = resolve_enter(
            &self.project,
            self.selection,
            &self.generated_parentheticals,
            &self.policy,
        )
        .map_or(ParagraphKeyTarget::Unavailable, |decision| decision.target);
        let tab = resolve_tab(
            &self.project,
            self.selection,
            &self.generated_parentheticals,
            &self.policy,
        )
        .map_or(ParagraphKeyTarget::Unavailable, |decision| decision.target);
        ParagraphKeyTargets { enter, tab }
    }
    /// Splits a semantic paragraph while retaining the original foreign ID on
    /// the left side and allocating a fresh source identity for the right.
    pub fn split(&mut self) -> Result<(), EditError> {
        self.mutate_structural(
            MutationValidationScope::Complete,
            |project, selection, _generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end {
                    return Err(EditError::InvalidSelection);
                }
                let element_type = project
                    .elements
                    .get(start.element)
                    .ok_or(EditError::InvalidElement)?
                    .element_type();
                split_at(project, start, element_type)
            },
        )
    }

    pub fn enter(&mut self) -> Result<(), EditError> {
        self.mutate_structural(
            MutationValidationScope::Complete,
            |project, selection, generated, policy| {
                let (start, end) = selection.ordered();
                if start == end
                    && project.elements.get(start.element).is_some_and(|item| {
                        project
                            .element_semantics(&item.element_type())
                            .is_some_and(|semantics| semantics.capabilities().dialogue)
                            && start.offset == item.text.len()
                    })
                {
                    let is_final_right_dialogue =
                        dual_dialogue::group_at_element_index(project, start.element).is_some_and(
                            |group| {
                                dual_dialogue::active_dual_dialogue_projection(project, group)
                                    .is_some_and(|projection| {
                                        projection.right.paragraphs.last().is_some_and(|id| {
                                            project
                                                .elements
                                                .get(start.element)
                                                .is_some_and(|item| &item.id == id)
                                        })
                                    })
                            },
                        );
                    let final_right_target = is_final_right_dialogue.then(|| {
                        project.resolve_paragraph_behavior_target(
                            &project.elements[start.element].element_type(),
                            scriptcraft_model::ParagraphBehaviorKey::Return,
                        )
                    });
                    if let Some(Some(scriptcraft_model::BehaviorTargetResolution::Resolved(
                        element_type,
                    ))) = final_right_target
                    {
                        let selection = split_at(project, start, element_type)?;
                        let action_id = project
                            .elements
                            .get(start.element + 1)
                            .ok_or(EditError::InvalidElement)?
                            .id
                            .clone();
                        dual_dialogue::remove_staged_group_member(project, &action_id);
                        return Ok(selection);
                    }
                }
                let decision = resolve_enter(project, *selection, generated, policy)?;
                match decision.action {
                    EnterAction::NoOp => Ok(*selection),
                    EnterAction::InsertParentheticalBefore { element, automatic } => {
                        let id = split_id(project)?;
                        let element_type = project
                            .elements
                            .get(element)
                            .ok_or(EditError::InvalidElement)?
                            .element_type();
                        let new = ScreenplayElement::new(
                            id.clone(),
                            element_type,
                            if automatic { "()" } else { "" },
                        )?;
                        if let Some(anchor) =
                            project.elements.get(element).map(|item| item.id.clone())
                        {
                            dual_dialogue::insert_group_member_at_flat_index(
                                project,
                                element,
                                &anchor,
                                new.id.clone(),
                            )?;
                        }
                        project.elements.insert(element, BodyOwner::shared(new));
                        if automatic {
                            generated.insert(id);
                            Ok(Selection::caret(Position { element, offset: 1 }))
                        } else {
                            Ok(Selection::caret(Position { element, offset: 0 }))
                        }
                    }
                    EnterAction::ClearGeneratedAndSplit {
                        position,
                        element_type,
                    } => {
                        BodyOwner::update(&mut project.elements[position.element], |element| {
                            element.text.clear();
                        });
                        generated.remove(project.elements[position.element].id.as_str());
                        split_at(project, position, element_type)
                    }
                    EnterAction::RepairAndInsert {
                        element,
                        element_type,
                    } => {
                        repair_parenthetical_in_project(project, element)?;
                        insert_dialogue_below(project, element, element_type)
                    }
                    EnterAction::Split {
                        position,
                        element_type,
                    } => split_at(project, position, element_type),
                }
            },
        )
    }

    /// Applies the current document's Tab Behavior. Missing or unresolved targets are consumed
    /// without changing the project, selection, or history.
    pub fn tab(&mut self) -> Result<(), EditError> {
        self.mutate_structural(
            MutationValidationScope::Complete,
            |project, selection, generated, policy| {
                let decision = resolve_tab(project, *selection, generated, policy)?;
                match decision.action {
                    TabAction::NoOp => Ok(*selection),
                    TabAction::DelimiterParentheticalToType {
                        element,
                        element_type,
                    } => {
                        let text_len = project.elements[element].text.len();
                        replace_range(
                            project,
                            Position { element, offset: 0 },
                            Position {
                                element,
                                offset: text_len,
                            },
                            "",
                        )?;
                        update_preserved_type(project, element, &element_type)?;
                        BodyOwner::update(&mut project.elements[element], |element| {
                            element.set_element_type(element_type);
                        });
                        generated.remove(project.elements[element].id.as_str());
                        Ok(Selection::caret(Position { element, offset: 0 }))
                    }
                    TabAction::RepairAndInsert {
                        element,
                        element_type,
                    } => {
                        repair_parenthetical_in_project(project, element)?;
                        insert_dialogue_below(project, element, element_type)
                    }
                    TabAction::SetElementType {
                        position,
                        element_type,
                        clear_generated_parenthetical,
                    } => {
                        if clear_generated_parenthetical {
                            BodyOwner::update(&mut project.elements[position.element], |element| {
                                element.text.clear();
                            });
                            generated.remove(project.elements[position.element].id.as_str());
                        }
                        let old_type = project.elements[position.element].element_type();
                        if old_type != element_type {
                            clear_body_paragraph_formatting(project, position.element)?;
                            update_preserved_type(project, position.element, &element_type)?;
                            BodyOwner::update(&mut project.elements[position.element], |element| {
                                element.set_element_type(element_type.clone());
                            });
                        }
                        if policy.auto_parenthetical
                            && project
                                .element_semantics(&element_type)
                                .is_some_and(|semantics| semantics.capabilities().parenthetical)
                            && project.elements[position.element].text.is_empty()
                        {
                            replace_range(
                                project,
                                Position {
                                    element: position.element,
                                    offset: 0,
                                },
                                Position {
                                    element: position.element,
                                    offset: 0,
                                },
                                "()",
                            )?;
                            generated
                                .insert(project.elements[position.element].id.as_str().to_owned());
                            return Ok(Selection::caret(Position {
                                element: position.element,
                                offset: 1,
                            }));
                        }
                        Ok(Selection::caret(position))
                    }
                }
            },
        )
    }

    /// Joins adjacent same-role paragraphs while retaining the left source
    /// identity. The removed owner's source subtree is discarded transactionally.
    pub fn join_with_next(&mut self) -> Result<(), EditError> {
        self.mutate_structural(
            MutationValidationScope::Complete,
            |project, selection, _generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end || start.offset != project.elements[start.element].text.len() {
                    return Err(EditError::InvalidSelection);
                }
                let offset = project.elements[start.element].text.len();
                join_at(project, start.element)?;
                Ok(Selection::caret(Position {
                    element: start.element,
                    offset,
                }))
            },
        )
    }

    /// Changes the selected body paragraph to an exact document-owned type.
    /// A real key change resets direct paragraph formatting but retains inline
    /// runs, text, identity, attachments, and dual-dialogue membership.
    pub fn change_element_type(&mut self, element_type: ElementTypeKey) -> Result<(), EditError> {
        self.change_element_type_candidate(element_type, false)
    }

    fn change_element_type_candidate(
        &mut self,
        element_type: ElementTypeKey,
        role_adapter: bool,
    ) -> Result<(), EditError> {
        if self
            .project
            .element_definitions()
            .get(&element_type)
            .is_none()
        {
            return Err(EditError::Model(ModelError::MissingElementDefinition(
                element_type,
            )));
        }
        let (start, end) = self.selection.ordered();
        if start.element != end.element {
            return Err(EditError::InvalidSelection);
        }
        let current = self
            .project
            .elements
            .get(start.element)
            .ok_or(EditError::InvalidElement)?
            .element_type();
        if !role_adapter && current == element_type {
            return Ok(());
        }

        self.mutate(
            MutationValidationScope::SingleParagraph,
            |project, selection, generated, policy| {
                let (start, end) = selection.ordered();
                if start.element != end.element {
                    return Err(EditError::InvalidSelection);
                }
                if project.elements[start.element].element_type() != element_type {
                    clear_body_paragraph_formatting(project, start.element)?;
                    update_preserved_type(project, start.element, &element_type)?;
                    BodyOwner::update(&mut project.elements[start.element], |element| {
                        element.set_element_type(element_type.clone());
                    });
                }
                let has_parenthetical_capability = project
                    .element_semantics(&element_type)
                    .is_some_and(|semantics| semantics.capabilities().parenthetical);
                if !has_parenthetical_capability {
                    generated.remove(project.elements[start.element].id.as_str());
                } else if role_adapter
                    && policy.auto_parenthetical
                    && project.elements[start.element].text.is_empty()
                {
                    replace_range(
                        project,
                        Position {
                            element: start.element,
                            offset: 0,
                        },
                        Position {
                            element: start.element,
                            offset: 0,
                        },
                        "()",
                    )?;
                    generated.insert(project.elements[start.element].id.as_str().to_owned());
                    return Ok(Selection::caret(Position {
                        element: start.element,
                        offset: 1,
                    }));
                }
                Ok(*selection)
            },
        )
    }
}

fn clear_body_paragraph_formatting(
    project: &mut Project,
    element_index: usize,
) -> Result<(), EditError> {
    let id = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?
        .id
        .clone();
    project.direct_paragraph_overrides.remove(&id);
    if let Some(preservation) = project.preservation.as_mut() {
        if preservation.has_managed_element(&id) {
            preservation
                .clear_managed_paragraph_formatting(&id)
                .map_err(EditError::Preservation)?;
        }
        preservation.clear_direct_paragraph_overrides(&id);
    }
    Ok(())
}

pub(super) fn update_preserved_type(
    project: &mut Project,
    element_index: usize,
    new_type: &ElementTypeKey,
) -> Result<(), EditError> {
    let element = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?;
    let id = element.id.clone();
    let old_type = element.element_type();
    if element
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_role.as_deref())
        .is_some_and(|source_type| source_type != old_type.as_str())
    {
        return Err(EditError::Preservation(
            PreservationError::UnsupportedRoleMapping,
        ));
    }
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ))?
            .change_managed_element_type(&id, &old_type, new_type)
            .map_err(EditError::Preservation)?;
    } else if element.source_provenance.is_some() {
        return Err(EditError::Preservation(
            PreservationError::ManagedAnchorMissing,
        ));
    }
    BodyOwner::update(&mut project.elements[element_index], |element| {
        if let Some(provenance) = &mut element.source_provenance {
            provenance.foreign_role = Some(new_type.as_str().to_owned());
        }
    });
    Ok(())
}

fn insert_dialogue_below(
    project: &mut Project,
    element: usize,
    element_type: ElementTypeKey,
) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let new = ScreenplayElement::new(split_id(project)?, element_type, "")?;
    let anchor = project.elements[element].id.clone();
    dual_dialogue::insert_group_member_at_flat_index(
        project,
        element + 1,
        &anchor,
        new.id.clone(),
    )?;
    project.elements.insert(element + 1, BodyOwner::shared(new));
    Ok(Selection::caret(Position {
        element: element + 1,
        offset: 0,
    }))
}

fn repair_parenthetical_in_project(
    project: &mut Project,
    element_index: usize,
) -> Result<(), EditError> {
    let text = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?
        .text
        .clone();
    let opens = text.starts_with('(');
    let closes = text.ends_with(')');
    if opens && !closes {
        let end = text.len();
        replace_range(
            project,
            Position {
                element: element_index,
                offset: end,
            },
            Position {
                element: element_index,
                offset: end,
            },
            ")",
        )?;
    } else if !opens && closes {
        replace_range(
            project,
            Position {
                element: element_index,
                offset: 0,
            },
            Position {
                element: element_index,
                offset: 0,
            },
            "(",
        )?;
    } else if !opens && !closes {
        replace_range(
            project,
            Position {
                element: element_index,
                offset: 0,
            },
            Position {
                element: element_index,
                offset: 0,
            },
            "(",
        )?;
        let end = text.len().saturating_add(1);
        replace_range(
            project,
            Position {
                element: element_index,
                offset: end,
            },
            Position {
                element: element_index,
                offset: end,
            },
            ")",
        )?;
    }
    Ok(())
}

fn split_at(
    project: &mut Project,
    position: Position,
    element_type: ElementTypeKey,
) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let item = project
        .elements
        .get(position.element)
        .ok_or(EditError::InvalidElement)?
        .clone();
    let left = item.text[..position.offset].to_owned();
    let right = item.text[position.offset..].to_owned();
    let old_runs = effective_runs(&item);
    let mut left_runs = slice_runs(&old_runs, 0, position.offset);
    let mut right_runs = slice_runs(&old_runs, position.offset, item.text.len());
    normalize_run_sequence(&left, &mut left_runs)?;
    normalize_run_sequence(&right, &mut right_runs)?;
    let id = split_id(project)?;
    let new_item = ScreenplayElement::new(id, element_type, right)?;
    dual_dialogue::insert_group_member_at_flat_index(
        project,
        position.element + 1,
        &item.id,
        new_item.id.clone(),
    )?;
    BodyOwner::update(&mut project.elements[position.element], |element| {
        element.text = left;
        element.runs = left_runs;
    });
    let mut new_item = new_item;
    new_item.runs = right_runs;
    project
        .elements
        .insert(position.element + 1, BodyOwner::shared(new_item));
    Ok(Selection::caret(Position {
        element: position.element + 1,
        offset: 0,
    }))
}

pub(super) fn unwrap_dual_dialogue_groups(
    project: &mut Project,
    group_ids: &[StableId],
) -> Result<(), EditError> {
    for group_id in group_ids {
        let index = project
            .dual_dialogue_groups
            .iter()
            .position(|group| &group.id == group_id)
            .ok_or(EditError::InvalidSelection)?;
        let group = project.dual_dialogue_groups[index].clone();
        if let Some(preservation) = project.preservation.as_mut() {
            let members = group.member_ids().cloned().collect::<Vec<_>>();
            preservation
                .unwrap_dual_dialogue_members(&group.id, &members, &group.wrapper)
                .map_err(EditError::Preservation)?;
        }
        project.dual_dialogue_groups.remove(index);
        project.dual_dialogue_paragraph_overrides.remove(group_id);
    }
    Ok(())
}

pub(super) fn remap_selection_by_identity(
    baseline: &Project,
    candidate: &Project,
    selection: Selection,
) -> Result<Selection, EditError> {
    let remap = |position: Position| -> Result<Position, EditError> {
        let id = &baseline
            .elements
            .get(position.element)
            .ok_or(EditError::InvalidElement)?
            .id;
        let element = candidate
            .elements
            .iter()
            .position(|item| &item.id == id)
            .ok_or(EditError::InvalidElement)?;
        Ok(Position {
            element,
            offset: position.offset,
        })
    };
    Ok(Selection {
        anchor: remap(selection.anchor)?,
        focus: remap(selection.focus)?,
    })
}

pub(super) fn groups_touched_by_selection(
    project: &Project,
    selection: Selection,
) -> Vec<StableId> {
    let (start, end) = selection.ordered();
    project
        .dual_dialogue_groups
        .iter()
        .filter_map(|group| {
            let first = dual_dialogue::first_member(project, group)?;
            let last = dual_dialogue::last_member(project, group)?;
            let touched = if start == end {
                (first..=last).contains(&start.element)
                    || start
                        .element
                        .checked_add(1)
                        .is_some_and(|next| next == first)
                    || start
                        .element
                        .checked_sub(1)
                        .is_some_and(|previous| previous == last)
            } else {
                start.element <= last && end.element >= first
            };
            touched.then(|| group.id.clone())
        })
        .collect()
}

fn split_id(project: &Project) -> Result<String, ModelError> {
    Ok(project.allocate_element_id()?.as_str().to_owned())
}

fn ensure_join_types_match(project: &Project, index: usize) -> Result<(), EditError> {
    let left = project
        .elements
        .get(index)
        .ok_or(EditError::InvalidElement)?;
    let right = project
        .elements
        .get(index + 1)
        .ok_or(EditError::InvalidElement)?;
    if left.element_type() != right.element_type() {
        return Err(EditError::InvalidSelection);
    }
    Ok(())
}

/// Removes one semantic owner and its direct/group registrations. Source-tree
/// cleanup remains in the transaction-level preservation reconciliation.
pub(super) fn remove_body_owner(project: &mut Project, index: usize) {
    let Some(removed_id) = project
        .elements
        .get(index)
        .map(|element| element.id.clone())
    else {
        return;
    };
    let removed_cue_group_ids = project
        .dual_dialogue_groups
        .iter()
        .filter(|group| {
            dual_dialogue::active_dual_dialogue_projection(project, group).is_some_and(
                |projection| {
                    projection.left.character == removed_id
                        || projection.right.character == removed_id
                },
            )
        })
        .map(|group| group.id.clone())
        .collect::<Vec<_>>();
    project.elements.remove(index);
    project.direct_paragraph_overrides.remove(&removed_id);

    if removed_cue_group_ids.is_empty() {
        for group in &mut project.dual_dialogue_groups {
            group.members.retain(|id| id != &removed_id);
        }
    } else {
        project
            .dual_dialogue_groups
            .retain(|group| !removed_cue_group_ids.contains(&group.id));
        for group_id in removed_cue_group_ids {
            project.dual_dialogue_paragraph_overrides.remove(&group_id);
        }
    }
    let undersized = project
        .dual_dialogue_groups
        .iter()
        .filter(|group| group.members.len() < 2)
        .map(|group| group.id.clone())
        .collect::<Vec<_>>();
    project
        .dual_dialogue_groups
        .retain(|group| !undersized.contains(&group.id));
    for group_id in undersized {
        project.dual_dialogue_paragraph_overrides.remove(&group_id);
    }
}

pub(super) fn join_at(project: &mut Project, index: usize) -> Result<(), EditError> {
    if index + 1 >= project.elements.len() {
        return Err(EditError::InvalidElement);
    }
    ensure_join_types_match(project, index)?;
    let right = project.elements[index + 1].text.clone();
    if project.elements[index]
        .text
        .len()
        .saturating_add(right.len())
        > MAX_TEXT_BYTES
    {
        return Err(EditError::TextTooLong);
    }
    let joined_text = format!("{}{}", project.elements[index].text, right);
    let mut joined_runs = effective_runs(&project.elements[index]);
    joined_runs.extend(effective_runs(&project.elements[index + 1]));
    normalize_run_sequence(&joined_text, &mut joined_runs)?;
    BodyOwner::update(&mut project.elements[index], |item| {
        item.text = joined_text;
        item.runs = joined_runs;
    });
    remove_body_owner(project, index + 1);
    Ok(())
}
