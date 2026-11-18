//! Body structure operations, role mutation, and structural candidate helpers.

use super::candidate::MutationValidationScope;
use super::policy::{repair_parenthetical, resolve_enter, resolve_tab, EnterAction, TabAction};
use super::selection::SelectionOrdering;
use super::text::{coalesce_runs, effective_runs, replace_range, slice_runs};
use super::{EditError, EditingSession, ParagraphKeyTarget, ParagraphKeyTargets, Position, Selection};
use scriptcraft_model::{
    dual_dialogue, preservation::PreservationError, BodyOwner, ModelError, Project, Role,
    ScreenplayElement, StableId, MAX_ELEMENTS, MAX_TEXT_BYTES,
};

impl EditingSession {
    /// Projects the exact role decisions used by [`Self::enter`] and
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
                split_at(project, start, project.elements[start.element].role)
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
                        item.role == Role::Dialogue && start.offset == item.text.len()
                    })
                {
                    let is_final_right_dialogue = dual_dialogue::group_at_element_index(
                        project,
                        start.element,
                    ).is_some_and(|group| {
                        dual_dialogue::last_member(project, group) == Some(start.element)
                            && group.right.paragraphs.last().is_some_and(|id| {
                                project.elements.get(start.element).is_some_and(|item| &item.id == id)
                            })
                    });
                    if is_final_right_dialogue {
                        let selection = split_at(project, start, Role::Action)?;
                        let action_id = project.elements.get(start.element + 1)
                            .ok_or(EditError::InvalidElement)?.id.clone();
                        dual_dialogue::remove_staged_group_member(project, &action_id);
                        return Ok(selection);
                    }
                }
                let decision = resolve_enter(project, *selection, generated, policy)?;
                match decision.action {
                    EnterAction::NoOp => Ok(*selection),
                    EnterAction::InsertParentheticalBefore { element, automatic } => {
                        let id = split_id(project)?;
                        let new = ScreenplayElement::new(
                            id.clone(),
                            Role::Parenthetical,
                            if automatic { "()" } else { "" },
                        )?;
                        if let Some(anchor) = project.elements.get(element).map(|item| item.id.clone()) {
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
                    EnterAction::ClearGeneratedAndSplit { position, role } => {
                        BodyOwner::update(&mut project.elements[position.element], |element| {
                            element.text.clear();
                        });
                        generated.remove(project.elements[position.element].id.as_str());
                        split_at(project, position, role)
                    }
                    EnterAction::RepairAndInsertDialogue { element } => {
                        let text = project.elements[element].text.clone();
                        BodyOwner::update(&mut project.elements[element], |element| {
                            element.text = repair_parenthetical(text.as_str());
                        });
                        insert_dialogue_below(project, element)
                    }
                    EnterAction::Split { position, role } => split_at(project, position, role),
                }
            },
        )
    }

    /// Applies the explicit Tab policy. Unlisted roles are intentionally consumed
    /// without changing the project, selection, or history.
    pub fn tab(&mut self) -> Result<(), EditError> {
        self.mutate_structural(
            MutationValidationScope::Complete,
            |project, selection, generated, policy| {
                let decision = resolve_tab(project, *selection, generated, policy)?;
                match decision.action {
                    TabAction::NoOp => Ok(*selection),
                    TabAction::DelimiterParentheticalToDialogue { element } => {
                        let text_len = project.elements[element].text.len();
                        replace_range(
                            project,
                            Position { element, offset: 0 },
                            Position { element, offset: text_len },
                            "",
                        )?;
                        update_preserved_role(
                            project,
                            element,
                            Role::Parenthetical,
                            Role::Dialogue,
                        )?;
                        BodyOwner::update(&mut project.elements[element], |element| {
                            element.role = Role::Dialogue;
                        });
                        generated.remove(project.elements[element].id.as_str());
                        Ok(Selection::caret(Position { element, offset: 0 }))
                    }
                    TabAction::RepairAndInsertDialogue { element } => {
                        let text = project.elements[element].text.clone();
                        BodyOwner::update(&mut project.elements[element], |element| {
                            element.text = repair_parenthetical(text.as_str());
                        });
                        insert_dialogue_below(project, element)
                    }
                    TabAction::SetRole {
                        position,
                        role,
                        clear_generated_parenthetical,
                    } => {
                        if clear_generated_parenthetical {
                            BodyOwner::update(&mut project.elements[position.element], |element| {
                                element.text.clear();
                            });
                            generated.remove(project.elements[position.element].id.as_str());
                        }
                        let old_role = project.elements[position.element].role;
                        if old_role != role {
                            update_preserved_role(project, position.element, old_role, role)?;
                            BodyOwner::update(&mut project.elements[position.element], |element| {
                                element.role = role;
                            });
                        }
                        if policy.auto_parenthetical
                            && role == Role::Parenthetical
                            && project.elements[position.element].text.is_empty()
                        {
                            replace_range(
                                project,
                                Position { element: position.element, offset: 0 },
                                Position { element: position.element, offset: 0 },
                                "()",
                            )?;
                            generated.insert(
                                project.elements[position.element].id.as_str().to_owned(),
                            );
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

    /// Changes a known role and its exact mapped source Type attribute together.
    /// A role change that would invalidate attached scene data is rejected.
    pub fn change_role(&mut self, role: Role) -> Result<(), EditError> {
        let (start, end) = self.selection.ordered();
        if start.element == end.element
            && self.project.elements.get(start.element).is_some_and(|element| {
                element.role == Role::Dialogue
                    && role != Role::Dialogue
                    && self.project.preservation.as_ref()
                        .is_some_and(|state| state.has_inline_alts(&element.id))
            })
        {
            // Reject before entering the structural planner: role changes are
            // not repaired by unwrapping an otherwise-valid dual group.
            return Err(EditError::InlineAltsRoleChange);
        }
        self.mutate_structural(
            MutationValidationScope::SingleParagraph,
            |project, selection, generated, policy| {
                let (start, end) = selection.ordered();
                if start.element != end.element {
                    return Err(EditError::InvalidSelection);
                }
                let old_role = project.elements[start.element].role;
                if old_role != role {
                    update_preserved_role(project, start.element, old_role, role)?;
                }
                BodyOwner::update(&mut project.elements[start.element], |element| {
                    element.role = role;
                });
                if role != Role::Parenthetical {
                    generated.remove(project.elements[start.element].id.as_str());
                } else if policy.auto_parenthetical
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

pub(super) fn update_preserved_role(
    project: &mut Project,
    element_index: usize,
    old_role: Role,
    new_role: Role,
) -> Result<(), EditError> {
    let element = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?;
    let id = element.id.clone();
    let old_fdx_role = fdx_role_name(old_role);
    if element
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_role.as_deref())
        .is_some_and(|source_role| source_role != old_fdx_role)
    {
        return Err(EditError::Preservation(
            PreservationError::UnsupportedRoleMapping,
        ));
    }
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if new_role != Role::Dialogue
        && project.preservation.as_ref().is_some_and(|state| state.has_inline_alts(&id))
    {
        return Err(EditError::InlineAltsRoleChange);
    }
    if has_source_mapping {
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ))?
            .change_managed_role(&id, old_fdx_role, fdx_role_name(new_role))
            .map_err(EditError::Preservation)?;
    } else if element.source_provenance.is_some() {
        return Err(EditError::Preservation(
            PreservationError::ManagedAnchorMissing,
        ));
    }
    BodyOwner::update(&mut project.elements[element_index], |element| {
        if let Some(provenance) = &mut element.source_provenance {
            provenance.foreign_role = Some(fdx_role_name(new_role).to_owned());
        }
    });
    Ok(())
}

fn fdx_role_name(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "Scene Heading",
        Role::Action => "Action",
        Role::Character => "Character",
        Role::Dialogue => "Dialogue",
        Role::Parenthetical => "Parenthetical",
        Role::Transition => "Transition",
        Role::Note => "Note",
        Role::NewAct => "New Act",
        Role::EndOfAct => "End of Act",
        Role::CastList => "Cast List",
        Role::General => "General",
        Role::Shot => "Shot",
        Role::Sequence => "Sequence",
        Role::Outline1 => "Outline 1",
        Role::Outline2 => "Outline 2",
        Role::Outline3 => "Outline 3",
        Role::Summary => "Summary",
    }
}

fn insert_dialogue_below(project: &mut Project, element: usize) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let new = ScreenplayElement::new(split_id(project)?, Role::Dialogue, "")?;
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

fn split_at(project: &mut Project, position: Position, role: Role) -> Result<Selection, EditError> {
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
    let left_runs = slice_runs(&old_runs, 0, position.offset);
    let right_runs = slice_runs(&old_runs, position.offset, item.text.len());
    let id = split_id(project)?;
    let new_item = ScreenplayElement::new(id, role, right)?;
    dual_dialogue::insert_group_member_at_flat_index(
        project,
        position.element + 1,
        &item.id,
        new_item.id.clone(),
    )?;
    BodyOwner::update(&mut project.elements[position.element], |element| {
        element.text = left;
        if !item.runs.is_empty() {
            element.runs = left_runs;
        }
    });
    let mut new_item = new_item;
    if !item.runs.is_empty() {
        new_item.runs = right_runs;
    }
    project.elements.insert(position.element + 1, BodyOwner::shared(new_item));
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
        let index = project.dual_dialogue_groups.iter()
            .position(|group| &group.id == group_id)
            .ok_or(EditError::InvalidSelection)?;
        let group = project.dual_dialogue_groups[index].clone();
        if let Some(preservation) = project.preservation.as_mut() {
            let anchor = group.wrapper.preservation_anchor.as_ref().ok_or(
                EditError::Preservation(PreservationError::ManagedAnchorMissing),
            )?;
            let members = group.member_ids().cloned().collect::<Vec<_>>();
            preservation.unwrap_dual_dialogue_members(
                &group.id,
                &members,
                anchor,
                group.wrapper.outer_foreign_id.as_deref(),
            ).map_err(EditError::Preservation)?;
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
        let id = &baseline.elements.get(position.element).ok_or(EditError::InvalidElement)?.id;
        let element = candidate.elements.iter().position(|item| &item.id == id)
            .ok_or(EditError::InvalidElement)?;
        Ok(Position { element, offset: position.offset })
    };
    Ok(Selection { anchor: remap(selection.anchor)?, focus: remap(selection.focus)? })
}

pub(super) fn groups_touched_by_selection(project: &Project, selection: Selection) -> Vec<StableId> {
    let (start, end) = selection.ordered();
    project.dual_dialogue_groups.iter().filter_map(|group| {
        let first = dual_dialogue::first_member(project, group)?;
        let last = dual_dialogue::last_member(project, group)?;
        let touched = if start == end {
            (first..=last).contains(&start.element)
                || start.element.checked_add(1).is_some_and(|next| next == first)
                || start.element.checked_sub(1).is_some_and(|previous| previous == last)
        } else {
            start.element <= last && end.element >= first
        };
        touched.then(|| group.id.clone())
    }).collect()
}

fn split_id(project: &Project) -> Result<String, ModelError> {
    Ok(project.allocate_element_id()?.as_str().to_owned())
}

fn ensure_join_roles_match(project: &Project, index: usize) -> Result<(), EditError> {
    let left = project.elements.get(index).ok_or(EditError::InvalidElement)?;
    let right = project
        .elements
        .get(index + 1)
        .ok_or(EditError::InvalidElement)?;
    if left.role != right.role {
        return Err(EditError::InvalidSelection);
    }
    Ok(())
}

/// Removes one semantic owner and its direct/group registrations. Source-tree
/// cleanup remains in the transaction-level preservation reconciliation.
pub(super) fn remove_body_owner(project: &mut Project, index: usize) {
    let removed_id = project.elements.remove(index).id.clone();
    project.direct_paragraph_overrides.remove(&removed_id);

    let removed_cue_group_ids = project
        .dual_dialogue_groups
        .iter()
        .filter(|group| group.left.character == removed_id || group.right.character == removed_id)
        .map(|group| group.id.clone())
        .collect::<Vec<_>>();
    if removed_cue_group_ids.is_empty() {
        for group in &mut project.dual_dialogue_groups {
            group.left.paragraphs.retain(|id| id != &removed_id);
            group.right.paragraphs.retain(|id| id != &removed_id);
        }
    } else {
        project
            .dual_dialogue_groups
            .retain(|group| !removed_cue_group_ids.contains(&group.id));
        for group_id in removed_cue_group_ids {
            project.dual_dialogue_paragraph_overrides.remove(&group_id);
        }
    }
}

pub(super) fn join_at(project: &mut Project, index: usize) -> Result<(), EditError> {
    if index + 1 >= project.elements.len() {
        return Err(EditError::InvalidElement);
    }
    ensure_join_roles_match(project, index)?;
    let right = project.elements[index + 1].text.clone();
    let joined_runs =
        if project.elements[index].runs.is_empty() && project.elements[index + 1].runs.is_empty() {
            Vec::new()
        } else {
            let mut runs = effective_runs(&project.elements[index]);
            runs.extend(effective_runs(&project.elements[index + 1]));
            coalesce_runs(&mut runs);
            runs
        };
    if project.elements[index].text.len().saturating_add(right.len()) > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    BodyOwner::update(&mut project.elements[index], |item| {
        item.text.push_str(&right);
        item.runs = joined_runs;
    });
    remove_body_owner(project, index + 1);
    Ok(())
}
