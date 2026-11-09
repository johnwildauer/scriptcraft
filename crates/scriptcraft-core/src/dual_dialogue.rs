//! Semantic dual-dialogue groups layered over the canonical flat body sequence.

use std::ops::Range;

use crate::editing::{Position, Selection};
use crate::preservation::SourceAnchorId;
use crate::{ModelError, Project, Role, ScreenplayElement, SourceProvenance, StableId, MAX_ELEMENTS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualDialogueSide {
    pub character: StableId,
    pub paragraphs: Vec<StableId>,
}

impl DualDialogueSide {
    pub fn member_ids(&self) -> impl Iterator<Item = &StableId> {
        std::iter::once(&self.character).chain(self.paragraphs.iter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DualDialogueWrapperProvenance {
    /// The foreign UUID on the outer General Paragraph, when imported.
    pub outer_foreign_id: Option<String>,
    /// General Paragraph source coordinates; this wrapper is never a visible element.
    pub source: Option<SourceProvenance>,
    /// Anchor for wrapper-owned unknown attributes in PreservationState.
    pub preservation_anchor: Option<SourceAnchorId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualDialogueGroup {
    /// Transient semantic wrapper identity, distinct from every visible element ID.
    pub id: StableId,
    pub left: DualDialogueSide,
    pub right: DualDialogueSide,
    pub wrapper: DualDialogueWrapperProvenance,
}

impl DualDialogueGroup {
    pub fn member_ids(&self) -> impl Iterator<Item = &StableId> {
        self.left.member_ids().chain(self.right.member_ids())
    }

    pub fn first_member(&self) -> &StableId {
        &self.left.character
    }

    pub fn last_member(&self) -> &StableId {
        self.right.paragraphs.last().unwrap_or(&self.right.character)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualDialogueSideKind {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionSurface {
    Body,
    TitlePage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePattern {
    pub left: DualDialogueSide,
    pub right: DualDialogueSide,
    pub flat_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleTarget {
    Enable(CandidatePattern),
    Disable(StableId),
}

/// Pure result classification for a proposed structural edit against the
/// detached flat Body candidate. No project, source, or history state is
/// changed while these outcomes are calculated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralGroupOutcome {
    UnchangedValid,
    UpdatedValid,
    Removed,
    InvalidRequiresUnwrap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralGroupClassification {
    pub group_id: StableId,
    pub outcome: StructuralGroupOutcome,
    pub touched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualDialogueError {
    TooManyGroups,
    TooManyMembers,
    DuplicateIdentity,
    DuplicateMember,
    MissingMember,
    NonContiguousMembers,
    GroupsOutOfOrder,
    InvalidRolePattern,
    InvalidWrapperProvenance,
    InvalidWrapperOwnership,
    InvalidWrapperOverride,
}

/// Resolve member identities against the current flat body sequence.
pub fn group_at_element_id<'a>(project: &'a Project, id: &StableId) -> Option<&'a DualDialogueGroup> {
    project.dual_dialogue_groups.iter().find(|group| group.member_ids().any(|member| member == id))
}

pub fn group_at_element_index(project: &Project, index: usize) -> Option<&DualDialogueGroup> {
    project.elements.get(index).and_then(|element| group_at_element_id(project, &element.id))
}

pub fn group_order(project: &Project) -> impl Iterator<Item = &DualDialogueGroup> {
    project.dual_dialogue_groups.iter()
}

/// Maps a transient group identity across a validated save/re-read pair.
/// Wrapper UUIDs are preferred; when the source had no wrapper UUID, the
/// ordered member identities must map as a complete group before a candidate
/// identity is returned.
pub fn map_group_identity(
    before: &Project,
    after: &Project,
    group_id: &StableId,
) -> Option<StableId> {
    let old_group = before.dual_dialogue_groups.iter().find(|group| &group.id == group_id)?;
    let old_members = old_group.member_ids().collect::<Vec<_>>();
    let mapped_members = old_members
        .iter()
        .map(|id| map_member_identity(before, after, id))
        .collect::<Option<Vec<_>>>()?;

    let candidates = if let Some(outer_id) = old_group.wrapper.outer_foreign_id.as_deref() {
        if before.dual_dialogue_groups.iter().filter(|group| {
            group.wrapper.outer_foreign_id.as_deref() == Some(outer_id)
        }).count() != 1 || after.dual_dialogue_groups.iter().filter(|group| {
            group.wrapper.outer_foreign_id.as_deref() == Some(outer_id)
        }).count() != 1 {
            return None;
        }
        after.dual_dialogue_groups.iter().filter(|group| {
            group.wrapper.outer_foreign_id.as_deref() == Some(outer_id)
        }).collect::<Vec<_>>()
    } else {
        after.dual_dialogue_groups.iter().filter(|group| {
            group.member_ids().eq(mapped_members.iter())
        }).collect::<Vec<_>>()
    };
    let [candidate] = candidates.as_slice() else { return None };
    if candidate.member_ids().ne(mapped_members.iter()) {
        return None;
    }
    let old_source = old_group.wrapper.source.as_ref();
    let new_source = candidate.wrapper.source.as_ref();
    if old_group.wrapper.outer_foreign_id.is_some()
        && !old_source.zip(new_source).is_some_and(|(old, new)| {
            old.format == new.format
                && old.section == new.section
                && old.foreign_id == new.foreign_id
                && old.foreign_role == new.foreign_role
        })
    {
        return None;
    }
    Some(candidate.id.clone())
}

fn map_member_identity(before: &Project, after: &Project, id: &StableId) -> Option<StableId> {
    let old_element = before.elements.iter().find(|element| &element.id == id)?;
    let old_source = old_element.source_provenance.as_ref();
    let source_id = old_source.and_then(|source| source.foreign_id.as_deref());
    let matches = if let Some(source_id) = source_id {
        if before.elements.iter().filter(|element| element.source_provenance.as_ref()
            .and_then(|source| source.foreign_id.as_deref()) == Some(source_id)).count() != 1
            || after.elements.iter().filter(|element| element.source_provenance.as_ref()
                .and_then(|source| source.foreign_id.as_deref()) == Some(source_id)).count() != 1
        {
            return None;
        }
        after.elements.iter().filter(|element| element.source_provenance.as_ref()
            .is_some_and(|source| {
                source.foreign_id.as_deref() == Some(source_id)
                    && old_source.is_some_and(|old| {
                        source.format == old.format
                            && source.section == old.section
                            && source.foreign_role == old.foreign_role
                    })
            })).collect::<Vec<_>>()
    } else {
        let old_matches = before.elements.iter().filter(|element| {
            element.role == old_element.role && element.text == old_element.text
        }).collect::<Vec<_>>();
        let occurrence = old_matches.iter().position(|element| element.id == old_element.id)?;
        let new_matches = after.elements.iter().filter(|element| {
            element.role == old_element.role && element.text == old_element.text
        }).collect::<Vec<_>>();
        if old_matches.len() != new_matches.len() || occurrence >= new_matches.len() {
            return None;
        }
        vec![new_matches[occurrence]]
    };
    let [element] = matches.as_slice() else { return None };
    Some(element.id.clone())
}

/// Classify all baseline groups against a proposed detached project. A group
/// is valid only when its identities still form the exact two-cue,
/// zero-or-more Dialogue/Parenthetical pattern in flat order.
pub fn classify_structural_result(
    baseline: &Project,
    selection: Selection,
    proposed: &Project,
) -> Vec<StructuralGroupClassification> {
    baseline.dual_dialogue_groups.iter().map(|before| {
        let touched = group_touched_by_selection(baseline, before, selection);
        let Some(after) = proposed.dual_dialogue_groups.iter().find(|group| group.id == before.id) else {
            return StructuralGroupClassification {
                group_id: before.id.clone(),
                outcome: StructuralGroupOutcome::Removed,
                touched,
            };
        };

        let before_members = before.member_ids().cloned().collect::<Vec<_>>();
        let after_members = after.member_ids().cloned().collect::<Vec<_>>();
        let remaining = before_members.iter().filter(|id| {
            proposed.elements.iter().any(|element| &element.id == *id)
        }).count();
        let outcome = if remaining == 0 {
            StructuralGroupOutcome::Removed
        } else if !has_valid_flat_pattern(proposed, after) {
            StructuralGroupOutcome::InvalidRequiresUnwrap
        } else if before_members == after_members
            && before.left.paragraphs == after.left.paragraphs
            && before.right.paragraphs == after.right.paragraphs
            && before.left.character == after.left.character
            && before.right.character == after.right.character
            && before_members.iter().all(|id| {
                let old = baseline.elements.iter().find(|element| &element.id == id);
                let new = proposed.elements.iter().find(|element| &element.id == id);
                old.zip(new).is_some_and(|(old, new)| old.role == new.role)
            })
        {
            StructuralGroupOutcome::UnchangedValid
        } else {
            StructuralGroupOutcome::UpdatedValid
        };
        StructuralGroupClassification { group_id: before.id.clone(), outcome, touched }
    }).collect()
}

fn group_touched_by_selection(project: &Project, group: &DualDialogueGroup, selection: Selection) -> bool {
    let (start, end) = if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    };
    let Some(first) = first_member(project, group) else { return false };
    let Some(last) = last_member(project, group) else { return false };
    if start == end {
        (first..=last).contains(&start.element)
            || start.element.checked_add(1).is_some_and(|next| next == first)
            || start.element.checked_sub(1).is_some_and(|previous| previous == last)
    } else {
        start.element <= last && end.element >= first
    }
}

fn has_valid_flat_pattern(project: &Project, group: &DualDialogueGroup) -> bool {
    let ids = group.member_ids().collect::<Vec<_>>();
    if ids.len() < 2 {
        return false;
    }
    let mut positions = Vec::with_capacity(ids.len());
    let mut roles = Vec::with_capacity(ids.len());
    for id in ids {
        let mut matches = project.elements.iter().enumerate().filter(|(_, item)| &item.id == id);
        let Some((index, element)) = matches.next() else { return false };
        if matches.next().is_some() { return false }
        positions.push(index);
        roles.push(element.role);
    }
    if positions.windows(2).any(|pair| pair[1] != pair[0].saturating_add(1)) {
        return false;
    }
    let right_cue = group.left.paragraphs.len().saturating_add(1);
    roles.first() == Some(&Role::Character)
        && roles.get(right_cue) == Some(&Role::Character)
        && roles.len() == group.left.paragraphs.len() + group.right.paragraphs.len() + 2
        && roles[1..right_cue].iter().all(|role| matches!(role, Role::Dialogue | Role::Parenthetical))
        && roles[right_cue + 1..].iter().all(|role| matches!(role, Role::Dialogue | Role::Parenthetical))
}

/// Remove one staged member identity from either side list. Used only by the
/// final-right Enter exception, where the new Action remains outside the
/// retained wrapper.
pub(crate) fn remove_staged_group_member(project: &mut Project, id: &StableId) {
    for group in &mut project.dual_dialogue_groups {
        group.left.paragraphs.retain(|member| member != id);
        group.right.paragraphs.retain(|member| member != id);
    }
}

pub fn first_member(project: &Project, group: &DualDialogueGroup) -> Option<usize> {
    project.elements.iter().position(|element| element.id == *group.first_member())
}

pub fn last_member(project: &Project, group: &DualDialogueGroup) -> Option<usize> {
    project.elements.iter().position(|element| element.id == *group.last_member())
}

pub fn left_flat_range(project: &Project, group: &DualDialogueGroup) -> Option<Range<usize>> {
    side_flat_range(project, &group.left)
}

pub fn right_flat_range(project: &Project, group: &DualDialogueGroup) -> Option<Range<usize>> {
    side_flat_range(project, &group.right)
}

fn side_flat_range(project: &Project, side: &DualDialogueSide) -> Option<Range<usize>> {
    let start = project.elements.iter().position(|element| element.id == side.character)?;
    let end = match side.paragraphs.last() {
        Some(end_id) => project.elements.iter().position(|element| &element.id == end_id)?,
        None => start,
    };
    (end >= start).then_some(start..end + 1)
}

pub fn side_at_element_index(project: &Project, index: usize) -> Option<DualDialogueSideKind> {
    let id = &project.elements.get(index)?.id;
    let group = group_at_element_id(project, id)?;
    if group.left.member_ids().any(|member| member == id) {
        Some(DualDialogueSideKind::Left)
    } else {
        Some(DualDialogueSideKind::Right)
    }
}

pub fn caret_side(project: &Project, position: Position) -> Option<DualDialogueSideKind> {
    let element = project.elements.get(position.element)?;
    element.validate_boundary(position.offset).ok()?;
    side_at_element_index(project, position.element)
}

/// Adds a newly-created member at a flat insertion point when that point is
/// inside the exact member sequence of the anchored group. The caller inserts
/// the semantic element at the same index immediately after this metadata
/// update; the detached candidate is validated before publication.
pub(crate) fn insert_group_member_at_flat_index(
    project: &mut Project,
    index: usize,
    anchor: &StableId,
    new_id: StableId,
) -> Result<(), ModelError> {
    let Some(group_index) = project.dual_dialogue_groups.iter().position(|group| {
        group.member_ids().any(|member| member == anchor)
    }) else {
        return Ok(());
    };
    let group = &project.dual_dialogue_groups[group_index];
    let left_cue = project.elements.iter().position(|item| item.id == group.left.character)
        .ok_or(ModelError::InvalidDualDialogue(crate::dual_dialogue::DualDialogueError::MissingMember))?;
    let right_cue = project.elements.iter().position(|item| item.id == group.right.character)
        .ok_or(ModelError::InvalidDualDialogue(crate::dual_dialogue::DualDialogueError::MissingMember))?;
    let last = project.elements.iter().position(|item| item.id == *group.last_member())
        .ok_or(ModelError::InvalidDualDialogue(crate::dual_dialogue::DualDialogueError::MissingMember))?;

    let (side, side_index) = if index > left_cue && index <= right_cue {
        let insert_before = group.left.paragraphs.iter().filter(|id| {
            project.elements.iter().position(|item| &item.id == *id).is_some_and(|at| at < index)
        }).count();
        (DualDialogueSideKind::Left, insert_before)
    } else if index > right_cue && index <= last.saturating_add(1) {
        let insert_before = group.right.paragraphs.iter().filter(|id| {
            project.elements.iter().position(|item| &item.id == *id).is_some_and(|at| at < index)
        }).count();
        (DualDialogueSideKind::Right, insert_before)
    } else {
        // Insertion immediately before the first cue is outside the group.
        return Ok(());
    };

    let group = &mut project.dual_dialogue_groups[group_index];
    let paragraphs = match side {
        DualDialogueSideKind::Left => &mut group.left.paragraphs,
        DualDialogueSideKind::Right => &mut group.right.paragraphs,
    };
    if side_index > paragraphs.len() {
        return Err(ModelError::InvalidDualDialogue(
            crate::dual_dialogue::DualDialogueError::NonContiguousMembers,
        ));
    }
    paragraphs.insert(side_index, new_id);
    Ok(())
}

/// Removes deleted Dialogue/Parenthetical identities from membership lists.
/// Cue identities intentionally remain required, so deleting a cue is rejected
/// by complete group validation rather than silently changing group identity.
pub(crate) fn prune_removed_group_paragraphs(before: &Project, after: &mut Project) {
    let retained = after.elements.iter().map(|item| item.id.clone()).collect::<std::collections::BTreeSet<_>>();
    let prior = before.elements.iter().map(|item| item.id.clone()).collect::<std::collections::BTreeSet<_>>();
    for group in &mut after.dual_dialogue_groups {
        group.left.paragraphs.retain(|id| !prior.contains(id) || retained.contains(id));
        group.right.paragraphs.retain(|id| !prior.contains(id) || retained.contains(id));
    }
}

/// Recognize a complete candidate beginning at `start`; this never mutates or repairs the project.
pub fn recognize_candidate(
    project: &Project,
    start: usize,
) -> Result<Option<CandidatePattern>, ModelError> {
    project.validate()?;
    recognize_candidate_unchecked(project, start)
}

fn recognize_candidate_unchecked(
    project: &Project,
    start: usize,
) -> Result<Option<CandidatePattern>, ModelError> {
    let Some(first) = project.elements.get(start) else { return Ok(None) };
    if first.role != Role::Character { return Ok(None) }
    if (0..start).any(|prior| {
        project.elements.get(prior).is_some_and(|element| element.role == Role::Character)
            && candidate_shape_end(&project.elements, prior).is_some_and(|end| start < end)
    }) {
        return Ok(None);
    }
    let left_begin = start.checked_add(1).ok_or(ModelError::InvalidNumber)?;
    let Some(left_end) = run_end(&project.elements, left_begin) else { return Ok(None) };
    let right_cue_index = left_end;
    let Some(right_cue) = project.elements.get(right_cue_index) else { return Ok(None) };
    if right_cue.role != Role::Character { return Ok(None) }
    let right_begin = right_cue_index.checked_add(1).ok_or(ModelError::InvalidNumber)?;
    let Some(end) = run_end(&project.elements, right_begin) else { return Ok(None) };
    if project.elements.get(end).is_some_and(|element| element.role == Role::Character) {
        return Ok(None);
    }
    if project.dual_dialogue_groups.iter().any(|group| {
        let first = first_member(project, group);
        let last = last_member(project, group);
        first.zip(last).is_some_and(|(first, last)| start <= last && end >= first)
    }) {
        return Ok(None);
    }
    let left = DualDialogueSide {
        character: first.id.clone(),
        paragraphs: project.elements[left_begin..left_end].iter().map(|element| element.id.clone()).collect(),
    };
    let right = DualDialogueSide {
        character: right_cue.id.clone(),
        paragraphs: project.elements[right_begin..end].iter().map(|element| element.id.clone()).collect(),
    };
    Ok(Some(CandidatePattern { left, right, flat_range: start..end }))
}

fn run_end(elements: &[ScreenplayElement], start: usize) -> Option<usize> {
    if start > elements.len() { return None }
    let mut end = start;
    while elements.get(end).is_some_and(|element| matches!(element.role, Role::Dialogue | Role::Parenthetical)) {
        end = end.checked_add(1)?;
    }
    Some(end)
}

fn candidate_shape_end(elements: &[ScreenplayElement], start: usize) -> Option<usize> {
    if elements.get(start)?.role != Role::Character { return None }
    let left_end = run_end(elements, start.checked_add(1)?)?;
    if elements.get(left_end)?.role != Role::Character { return None }
    let end = run_end(elements, left_end.checked_add(1)?)?;
    elements.get(end).map_or(true, |element| element.role != Role::Character).then_some(end)
}

/// Resolve a valid toggle target for a collapsed Body caret. TitlePage and ranged selections are ineligible.
pub fn resolve_toggle_target(
    project: &Project,
    surface: SelectionSurface,
    selection: Selection,
) -> Result<Option<ToggleTarget>, ModelError> {
    project.validate()?;
    if surface != SelectionSurface::Body || selection.anchor != selection.focus {
        return Ok(None);
    }
    let Some(element) = project.elements.get(selection.focus.element) else { return Ok(None) };
    element.validate_boundary(selection.focus.offset)?;
    if let Some(group) = group_at_element_id(project, &element.id) {
        return Ok(Some(ToggleTarget::Disable(group.id.clone())));
    }
    if element.role != Role::Character { return Ok(None) }
    Ok(recognize_candidate_unchecked(project, selection.focus.element)?.map(ToggleTarget::Enable))
}

pub(crate) fn validate_groups(project: &Project) -> Result<(), DualDialogueError> {
    if project.dual_dialogue_groups.len() > MAX_ELEMENTS { return Err(DualDialogueError::TooManyGroups) }
    let mut ids = std::collections::BTreeSet::new();
    for item in project.title_page.iter().map(|item| &item.id).chain(project.elements.iter().map(|item| &item.id)) {
        ids.insert(item.clone());
    }
    let mut membership = std::collections::BTreeSet::new();
    let mut total = 0usize;
    let mut previous_end = None;
    for group in &project.dual_dialogue_groups {
        if !ids.insert(group.id.clone()) { return Err(DualDialogueError::DuplicateIdentity) }
        validate_wrapper_provenance(&group.wrapper)?;
        let member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        total = total.checked_add(member_ids.len()).ok_or(DualDialogueError::TooManyMembers)?;
        if total > MAX_ELEMENTS { return Err(DualDialogueError::TooManyMembers) }
        if member_ids.len() < 2 {
            return Err(DualDialogueError::InvalidRolePattern)
        }
        let mut indexes = Vec::with_capacity(member_ids.len());
        for id in member_ids {
            if !membership.insert(id.clone()) { return Err(DualDialogueError::DuplicateMember) }
            let matches = project.elements.iter().enumerate().filter(|(_, item)| item.id == id).map(|(index, _)| index).collect::<Vec<_>>();
            if matches.len() != 1 { return Err(DualDialogueError::MissingMember) }
            indexes.push(matches[0]);
        }
        if indexes.windows(2).any(|pair| pair[1] != pair[0].saturating_add(1)) {
            return Err(DualDialogueError::NonContiguousMembers)
        }
        if previous_end.is_some_and(|end| indexes[0] <= end) { return Err(DualDialogueError::GroupsOutOfOrder) }
        previous_end = indexes.last().copied();
        let roles = indexes.iter().filter_map(|index| project.elements.get(*index).map(|item| item.role)).collect::<Vec<_>>();
        if roles.first() != Some(&Role::Character)
            || roles.get(group.left.paragraphs.len() + 1) != Some(&Role::Character)
            || roles.len() != group.left.paragraphs.len() + group.right.paragraphs.len() + 2
            || !roles[1..group.left.paragraphs.len() + 1].iter().all(|role| matches!(role, Role::Dialogue | Role::Parenthetical))
            || !roles[group.left.paragraphs.len() + 2..].iter().all(|role| matches!(role, Role::Dialogue | Role::Parenthetical))
        {
            return Err(DualDialogueError::InvalidRolePattern);
        }
    }
    Ok(())
}

pub(crate) fn validate_wrapper_overrides(
    project: &Project,
) -> Result<(), DualDialogueError> {
    let group_ids = project.dual_dialogue_groups.iter().map(|group| &group.id).collect::<std::collections::BTreeSet<_>>();
    for (id, overrides) in &project.dual_dialogue_paragraph_overrides {
        if !group_ids.contains(id)
            || overrides.paragraph.alignment.is_some()
            || overrides.paragraph.first_indent.is_some()
            || overrides.paragraph.leading_regular.is_some()
            || overrides.paragraph.spacing_single.is_some()
        {
            return Err(DualDialogueError::InvalidWrapperOverride);
        }
    }
    Ok(())
}

fn validate_wrapper_provenance(wrapper: &DualDialogueWrapperProvenance) -> Result<(), DualDialogueError> {
    if wrapper.outer_foreign_id.as_ref().is_some_and(|value| value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control)) {
        return Err(DualDialogueError::InvalidWrapperProvenance);
    }
    if let Some(source) = &wrapper.source {
        if source.format != crate::SourceFormat::Fdx
            || source.section != crate::SourceSection::Content
            || source.foreign_role.as_deref() != Some("General")
            || source.foreign_id != wrapper.outer_foreign_id
        {
            return Err(DualDialogueError::InvalidWrapperProvenance);
        }
    }
    Ok(())
}
