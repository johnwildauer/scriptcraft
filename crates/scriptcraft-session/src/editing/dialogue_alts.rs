//! Session-owned, transactional editing for alternate Dialogue variants.

use super::{candidate::MutationValidationScope, EditError, EditingSession, Position, Selection};
use super::text::body_change;
use scriptcraft_model::{
    body_offsets::{flat_offset_to_position, paragraph_flat_start},
    dialogue_alts::{
        DialogueAltAddIds, DialogueAltDirection, DialogueAltEdit, DialogueAltError,
        DialogueAltId, DialogueAlternatives,
    },
    script_notes::{BodyTextReplacement, ScriptNoteRange},
    BodyOwner, ModelError, Project, StableId,
};
use std::collections::BTreeSet;

/// Read-only Alt state projected for the owner at the current Body selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueAltTarget {
    pub element_index: usize,
    pub owner_id: StableId,
    pub collapsed_dialogue_owner: bool,
    pub inline_source_supported: bool,
    pub total_variant_count: Option<usize>,
    pub active_ordinal: Option<usize>,
    pub can_add: bool,
    pub can_previous: bool,
    pub can_next: bool,
    pub can_delete: bool,
}

struct SelectedDialogue<'a> {
    index: usize,
    owner: &'a scriptcraft_model::ScreenplayElement,
    definition: &'a scriptcraft_model::ElementDefinition,
}

fn selected_dialogue<'a>(
    project: &'a Project,
    selection: Selection,
) -> Option<SelectedDialogue<'a>> {
    if selection.anchor != selection.focus {
        return None;
    }
    let index = selection.focus.element;
    let owner = project.elements.get(index)?;
    let semantics = project.element_semantics(&owner.element_type())?;
    if !semantics.capabilities().dialogue {
        return None;
    }
    let has_inline_alts = project
        .preservation
        .as_ref()
        .is_some_and(|preservation| preservation.has_inline_alts(&owner.id));
    if has_inline_alts && owner.dialogue_alternatives.is_none() {
        return None;
    }
    Some(SelectedDialogue {
        index,
        owner,
        definition: semantics.definition(),
    })
}

fn model_alt_error(error: DialogueAltError) -> EditError {
    match error {
        DialogueAltError::RunProjection(error) => EditError::FormattedText(error),
        DialogueAltError::TooManyVariants => EditError::Model(ModelError::TooManyElements),
        _ => EditError::Model(ModelError::InvalidNumber),
    }
}

fn active_caret(element: usize, offset: usize) -> Selection {
    let position = Position { element, offset };
    Selection {
        anchor: position,
        focus: position,
    }
}

fn document_max_alt_id(project: &Project) -> Option<u64> {
    let semantic_max = project
        .elements
        .iter()
        .filter_map(|owner| owner.dialogue_alternatives.as_ref())
        .flat_map(|alternatives| {
            std::iter::once(alternatives.current.value()).chain(
                alternatives
                    .inactive
                    .iter()
                    .map(|alternative| alternative.id.value()),
            )
        })
        .max();
    let retained_source_max = project
        .preservation
        .as_ref()
        .and_then(|preservation| preservation.max_numeric_dialogue_alt_id());
    semantic_max.into_iter().chain(retained_source_max).max()
}

fn allocate_add_ids(
    project: &Project,
    existing: Option<&DialogueAlternatives>,
) -> Result<DialogueAltAddIds, EditError> {
    let first = match document_max_alt_id(project) {
        Some(maximum) => maximum
            .checked_add(1)
            .ok_or(EditError::Model(ModelError::InvalidNumber))?,
        None => 1,
    };
    let first = DialogueAltId::new(first);
    match existing {
        Some(_) => Ok(DialogueAltAddIds {
            demoted_current: None,
            fresh_current: first,
        }),
        None => {
            let fresh = first
                .checked_increment()
                .ok_or(EditError::Model(ModelError::InvalidNumber))?;
            Ok(DialogueAltAddIds {
                demoted_current: Some(first),
                fresh_current: fresh,
            })
        }
    }
}

fn alt_text_replacement(
    project: &Project,
    index: usize,
    replacement_text: &str,
) -> Result<BodyTextReplacement, EditError> {
    let owner = project
        .elements
        .get(index)
        .ok_or(EditError::InvalidSelection)?;
    let start = paragraph_flat_start(project, index).ok_or(EditError::InvalidSelection)?;
    let removed_len = owner.text.encode_utf16().count();
    let end = start
        .checked_add(removed_len)
        .ok_or(EditError::InvalidSelection)?;
    body_change::flat_replacement(project, start, end, replacement_text.encode_utf16().count())
}

fn alt_note_relocation_plan(
    project: &Project,
    action_target: usize,
) -> Result<Vec<(StableId, usize)>, EditError> {
    let mut eligible = vec![false; project.elements.len()];
    for (index, owner) in project.elements.iter().enumerate() {
        let is_dialogue = project
            .element_semantics(&owner.element_type())
            .is_some_and(|semantics| semantics.capabilities().dialogue);
        if !is_dialogue {
            continue;
        }
        let has_inline_alts = project
            .preservation
            .as_ref()
            .is_some_and(|preservation| preservation.has_inline_alts(&owner.id));
        eligible[index] = owner.dialogue_alternatives.is_some() || has_inline_alts;
    }
    if let Some(target) = eligible.get_mut(action_target) {
        *target = true;
    } else {
        return Err(EditError::InvalidSelection);
    }

    let mut plan = Vec::new();
    for note in project.script_notes().iter() {
        let start = flat_offset_to_position(project, note.range.start())
            .ok_or(EditError::Model(ModelError::InvalidNumber))?;
        let end = flat_offset_to_position(project, note.range.end())
            .ok_or(EditError::Model(ModelError::InvalidNumber))?;
        let start_paragraph = eligible
            .get(start.element)
            .copied()
            .unwrap_or(false)
            .then_some(start.element);
        let end_paragraph = eligible
            .get(end.element)
            .copied()
            .unwrap_or(false)
            .then_some(end.element);
        let selected_paragraph = match (start_paragraph, end_paragraph) {
            (Some(start), Some(end)) => Some(start.min(end)),
            (Some(start), None) => Some(start),
            (None, Some(end)) => Some(end),
            (None, None) => None,
        };
        if let Some(paragraph) = selected_paragraph {
            plan.push((note.id.clone(), paragraph));
        }
    }
    Ok(plan)
}

fn publish_alt_edit(
    project: &mut Project,
    index: usize,
    edit: DialogueAltEdit,
    replacement: BodyTextReplacement,
    relocation_plan: &[(StableId, usize)],
) -> Result<(), EditError> {
    BodyOwner::update(&mut project.elements[index], |element| {
        element.dialogue_alternatives = edit.alternatives;
        element.text = edit.text;
        element.runs = edit.runs;
    });
    body_change::apply(project, replacement)?;

    let mut final_ranges = Vec::with_capacity(relocation_plan.len());
    for (note_id, paragraph_index) in relocation_plan {
        // Recompute from the unchanged paragraph index after replacement: later
        // paragraphs move by the active-text length delta while separators remain.
        let front = paragraph_flat_start(project, *paragraph_index)
            .ok_or(EditError::Model(ModelError::InvalidNumber))?;
        let range = ScriptNoteRange::new(front, front)
            .ok_or(EditError::Model(ModelError::InvalidNumber))?;
        final_ranges.push((note_id.clone(), range));
    }
    project
        .install_script_note_ranges_candidate(&final_ranges)
        .map_err(EditError::Model)
}

impl EditingSession {
    /// Projects the selected Body owner and its current authoring eligibility.
    pub fn dialogue_alt_target(&self) -> Option<DialogueAltTarget> {
        let selection = self.selection;
        let index = selection.focus.element;
        let owner = self.project.elements.get(index)?;
        let is_dialogue = self
            .project
            .element_semantics(&owner.element_type())
            .is_some_and(|semantics| semantics.capabilities().dialogue);
        let has_inline_alts = self
            .project
            .preservation
            .as_ref()
            .is_some_and(|preservation| preservation.has_inline_alts(&owner.id));
        let inline_source_supported = !has_inline_alts
            || (is_dialogue && owner.dialogue_alternatives.is_some());
        let collapsed_dialogue_owner = selection.anchor == selection.focus && is_dialogue;
        let (total_variant_count, active_ordinal) = owner
            .dialogue_alternatives
            .as_ref()
            .map(|alternatives| {
                (
                    Some(alternatives.total_count()),
                    Some(alternatives.active_ordinal()),
                )
            })
            .unwrap_or((None, None));
        let eligible = collapsed_dialogue_owner && inline_source_supported;
        let multiple = total_variant_count.is_some_and(|count| count > 1);
        Some(DialogueAltTarget {
            element_index: index,
            owner_id: owner.id.clone(),
            collapsed_dialogue_owner,
            inline_source_supported,
            total_variant_count,
            active_ordinal,
            can_add: eligible,
            can_previous: eligible && multiple,
            can_next: eligible && multiple,
            can_delete: eligible && multiple,
        })
    }

    /// Adds a fresh empty Current at the active collapsed Dialogue caret.
    /// Returns `false` without publishing when the target is ineligible.
    pub fn add_dialogue_alt(&mut self) -> Result<bool, EditError> {
        let mut applied = false;
        self.mutate_with_scene_heading_deletions(
            MutationValidationScope::Complete,
            BTreeSet::new(),
            false,
            |project, selection, _, _| {
                let Some(selected) = selected_dialogue(project, *selection) else {
                    return Ok(*selection);
                };
                let ids = allocate_add_ids(project, selected.owner.dialogue_alternatives.as_ref())?;
                let edit = DialogueAlternatives::add(
                    selected.owner.dialogue_alternatives.as_ref(),
                    &selected.owner.text,
                    &selected.owner.runs,
                    ids,
                    selected.definition,
                )
                .map_err(model_alt_error)?;
                let index = selected.index;
                let relocation_plan = alt_note_relocation_plan(project, index)?;
                let replacement = alt_text_replacement(project, index, &edit.text)?;
                publish_alt_edit(project, index, edit, replacement, &relocation_plan)?;
                applied = true;
                Ok(active_caret(index, 0))
            },
        )?;
        Ok(applied)
    }

    /// Navigates to the nearest numeric Previous/Next variant, wrapping at the
    /// boundary. Returns `false` without publishing when ineligible.
    pub fn navigate_dialogue_alt(
        &mut self,
        direction: DialogueAltDirection,
    ) -> Result<bool, EditError> {
        let mut applied = false;
        self.mutate_with_scene_heading_deletions(
            MutationValidationScope::Complete,
            BTreeSet::new(),
            false,
            |project, selection, _, _| {
                let Some(selected) = selected_dialogue(project, *selection) else {
                    return Ok(*selection);
                };
                let Some(alternatives) = selected.owner.dialogue_alternatives.as_ref() else {
                    return Ok(*selection);
                };
                if !alternatives.can_navigate() {
                    return Ok(*selection);
                }
                let edit = alternatives
                    .navigate(
                        direction,
                        &selected.owner.text,
                        &selected.owner.runs,
                        selected.definition,
                    )
                    .map_err(model_alt_error)?;
                let index = selected.index;
                let caret = edit.text.len();
                let relocation_plan = alt_note_relocation_plan(project, index)?;
                let replacement = alt_text_replacement(project, index, &edit.text)?;
                publish_alt_edit(project, index, edit, replacement, &relocation_plan)?;
                applied = true;
                Ok(active_caret(index, caret))
            },
        )?;
        Ok(applied)
    }

    /// Deletes Current and promotes greatest-lower, otherwise least-greater.
    /// Returns `false` without publishing when ineligible.
    pub fn delete_dialogue_alt(&mut self) -> Result<bool, EditError> {
        let mut applied = false;
        self.mutate_with_scene_heading_deletions(
            MutationValidationScope::Complete,
            BTreeSet::new(),
            false,
            |project, selection, _, _| {
                let Some(selected) = selected_dialogue(project, *selection) else {
                    return Ok(*selection);
                };
                let Some(alternatives) = selected.owner.dialogue_alternatives.as_ref() else {
                    return Ok(*selection);
                };
                if !alternatives.can_delete_current() {
                    return Ok(*selection);
                }
                let edit = alternatives
                    .delete_current(selected.definition)
                    .map_err(model_alt_error)?;
                let index = selected.index;
                let caret = edit.text.len();
                let relocation_plan = alt_note_relocation_plan(project, index)?;
                let replacement = alt_text_replacement(project, index, &edit.text)?;
                publish_alt_edit(project, index, edit, replacement, &relocation_plan)?;
                applied = true;
                Ok(active_caret(index, caret))
            },
        )?;
        Ok(applied)
    }
}
