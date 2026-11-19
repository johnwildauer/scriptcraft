use std::ops::Range;

use crate::{
    ModelError, Position, Project, Selection, SemanticCapabilities, SourceProvenance, StableId,
    MAX_ELEMENTS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualDialogueError {
    TooManyGroups,
    TooManyMembers,
    DuplicateIdentity,
    DuplicateMember,
    MissingMember,
    NonContiguousMembers,
    GroupsOutOfOrder,
    InvalidMembership,
    /// Retained for callers that used the earlier active-pattern validation error.
    InvalidRolePattern,
    InvalidWrapperProvenance,
    InvalidWrapperOwnership,
    InvalidWrapperOverride,
}

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
    pub preservation_anchor: Option<crate::preservation::SourceAnchorId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualDialogueGroup {
    /// Transient semantic wrapper identity, distinct from every visible element ID.
    pub id: StableId,
    /// Ordered source membership, independent of whether current types support
    /// a two-column interpretation.
    pub members: Vec<StableId>,
    pub wrapper: DualDialogueWrapperProvenance,
}

impl DualDialogueGroup {
    pub fn member_ids(&self) -> impl Iterator<Item = &StableId> {
        self.members.iter()
    }

    pub fn first_member(&self) -> &StableId {
        &self.members[0]
    }

    pub fn last_member(&self) -> &StableId {
        self.members.last().unwrap_or(self.first_member())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveDualDialogueProjection {
    pub left: DualDialogueSide,
    pub right: DualDialogueSide,
}

/// Derives the currently supported two-column split from ordered source
/// membership and the members' current exact-key capabilities.
pub fn active_dual_dialogue_projection(
    project: &Project,
    group: &DualDialogueGroup,
) -> Option<ActiveDualDialogueProjection> {
    let members = group.members.as_slice();
    if members.len() < 2 {
        return None;
    }
    let mut capabilities = Vec::with_capacity(members.len());
    let mut positions = Vec::with_capacity(members.len());
    for id in members {
        let mut matches = project
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| &element.id == id);
        let (position, element) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        positions.push(position);
        capabilities.push(element_capabilities(project, element)?);
    }
    if positions
        .windows(2)
        .any(|pair| pair[1] != pair[0].saturating_add(1))
        || !capabilities.first()?.character_cue
    {
        return None;
    }
    let right_cue = capabilities
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, capability)| capability.character_cue.then_some(index))?;
    if capabilities[1..right_cue]
        .iter()
        .any(|capability| !is_dual_body_member(*capability))
        || capabilities[right_cue + 1..]
            .iter()
            .any(|capability| !is_dual_body_member(*capability))
    {
        return None;
    }
    Some(ActiveDualDialogueProjection {
        left: DualDialogueSide {
            character: members[0].clone(),
            paragraphs: members[1..right_cue].to_vec(),
        },
        right: DualDialogueSide {
            character: members[right_cue].clone(),
            paragraphs: members[right_cue + 1..].to_vec(),
        },
    })
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

/// Validates source membership, wrapper ownership, and flat Body placement.
/// Active two-column type semantics are derived separately and are not a
/// project-validity requirement.
pub(crate) fn validate_groups(project: &Project) -> Result<(), DualDialogueError> {
    if project.dual_dialogue_groups.len() > MAX_ELEMENTS {
        return Err(DualDialogueError::TooManyGroups);
    }
    let mut ids = std::collections::BTreeSet::new();
    for item in project
        .title_page
        .iter()
        .map(|item| &item.id)
        .chain(project.elements.iter().map(|item| &item.id))
    {
        ids.insert(item.clone());
    }
    let mut membership = std::collections::BTreeSet::new();
    let mut total = 0usize;
    let mut previous_end = None;
    for group in &project.dual_dialogue_groups {
        if !ids.insert(group.id.clone()) {
            return Err(DualDialogueError::DuplicateIdentity);
        }
        validate_wrapper_provenance(&group.wrapper)?;
        let member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        total = total
            .checked_add(member_ids.len())
            .ok_or(DualDialogueError::TooManyMembers)?;
        if total > MAX_ELEMENTS {
            return Err(DualDialogueError::TooManyMembers);
        }
        if member_ids.len() < 2 {
            return Err(DualDialogueError::InvalidMembership);
        }
        let mut indexes = Vec::with_capacity(member_ids.len());
        for id in member_ids {
            if !membership.insert(id.clone()) {
                return Err(DualDialogueError::DuplicateMember);
            }
            let matches = project
                .elements
                .iter()
                .enumerate()
                .filter(|(_, item)| item.id == id)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(DualDialogueError::MissingMember);
            }
            indexes.push(matches[0]);
        }
        if indexes
            .windows(2)
            .any(|pair| pair[1] != pair[0].saturating_add(1))
        {
            return Err(DualDialogueError::NonContiguousMembers);
        }
        if previous_end.is_some_and(|end| indexes[0] <= end) {
            return Err(DualDialogueError::GroupsOutOfOrder);
        }
        previous_end = indexes.last().copied();
    }
    Ok(())
}

pub(crate) fn validate_wrapper_overrides(project: &Project) -> Result<(), DualDialogueError> {
    let group_ids = project
        .dual_dialogue_groups
        .iter()
        .map(|group| &group.id)
        .collect::<std::collections::BTreeSet<_>>();
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

fn validate_wrapper_provenance(
    wrapper: &DualDialogueWrapperProvenance,
) -> Result<(), DualDialogueError> {
    if wrapper.outer_foreign_id.as_ref().is_some_and(|value| {
        value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control)
    }) {
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

/// Resolve member identities against the current flat body sequence.
pub fn group_at_element_id<'a>(
    project: &'a Project,
    id: &StableId,
) -> Option<&'a DualDialogueGroup> {
    project
        .dual_dialogue_groups
        .iter()
        .find(|group| group.member_ids().any(|member| member == id))
}

pub fn group_at_element_index(project: &Project, index: usize) -> Option<&DualDialogueGroup> {
    project
        .elements
        .get(index)
        .and_then(|element| group_at_element_id(project, &element.id))
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
    let old_group = before
        .dual_dialogue_groups
        .iter()
        .find(|group| &group.id == group_id)?;
    let old_members = old_group.member_ids().collect::<Vec<_>>();
    let mapped_members = old_members
        .iter()
        .map(|id| map_member_identity(before, after, id))
        .collect::<Option<Vec<_>>>()?;

    let candidates = if let Some(outer_id) = old_group.wrapper.outer_foreign_id.as_deref() {
        if before
            .dual_dialogue_groups
            .iter()
            .filter(|group| group.wrapper.outer_foreign_id.as_deref() == Some(outer_id))
            .count()
            != 1
            || after
                .dual_dialogue_groups
                .iter()
                .filter(|group| group.wrapper.outer_foreign_id.as_deref() == Some(outer_id))
                .count()
                != 1
        {
            return None;
        }
        after
            .dual_dialogue_groups
            .iter()
            .filter(|group| group.wrapper.outer_foreign_id.as_deref() == Some(outer_id))
            .collect::<Vec<_>>()
    } else {
        after
            .dual_dialogue_groups
            .iter()
            .filter(|group| group.member_ids().eq(mapped_members.iter()))
            .collect::<Vec<_>>()
    };
    let [candidate] = candidates.as_slice() else {
        return None;
    };
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
        if before
            .elements
            .iter()
            .filter(|element| {
                element
                    .source_provenance
                    .as_ref()
                    .and_then(|source| source.foreign_id.as_deref())
                    == Some(source_id)
            })
            .count()
            != 1
            || after
                .elements
                .iter()
                .filter(|element| {
                    element
                        .source_provenance
                        .as_ref()
                        .and_then(|source| source.foreign_id.as_deref())
                        == Some(source_id)
                })
                .count()
                != 1
        {
            return None;
        }
        after
            .elements
            .iter()
            .filter(|element| {
                element.source_provenance.as_ref().is_some_and(|source| {
                    source.foreign_id.as_deref() == Some(source_id)
                        && old_source.is_some_and(|old| {
                            source.format == old.format && source.section == old.section
                        })
                })
            })
            .collect::<Vec<_>>()
    } else {
        let stable_matches = after
            .elements
            .iter()
            .filter(|element| element.id == old_element.id)
            .collect::<Vec<_>>();
        if let [element] = stable_matches.as_slice() {
            return Some(element.id.clone());
        }
        let old_matches = before
            .elements
            .iter()
            .filter(|element| element.text == old_element.text)
            .collect::<Vec<_>>();
        let occurrence = old_matches
            .iter()
            .position(|element| element.id == old_element.id)?;
        let new_matches = after
            .elements
            .iter()
            .filter(|element| element.text == old_element.text)
            .collect::<Vec<_>>();
        if old_matches.len() != new_matches.len() || occurrence >= new_matches.len() {
            return None;
        }
        vec![new_matches[occurrence]]
    };
    let [element] = matches.as_slice() else {
        return None;
    };
    Some(element.id.clone())
}

/// Classify all baseline groups against a proposed detached project. A changed
/// or displaced member sequence that no longer has an active projection retains
/// the established structural unwrap retry. Type-only projection changes do
/// not unwrap an otherwise unchanged ordered source container.
pub fn classify_structural_result(
    baseline: &Project,
    selection: Selection,
    proposed: &Project,
) -> Vec<StructuralGroupClassification> {
    baseline
        .dual_dialogue_groups
        .iter()
        .map(|before| {
            let touched = group_touched_by_selection(baseline, before, selection);
            let Some(after) = proposed
                .dual_dialogue_groups
                .iter()
                .find(|group| group.id == before.id)
            else {
                return StructuralGroupClassification {
                    group_id: before.id.clone(),
                    outcome: StructuralGroupOutcome::Removed,
                    touched,
                };
            };

            let before_members = before.member_ids().cloned().collect::<Vec<_>>();
            let after_members = after.member_ids().cloned().collect::<Vec<_>>();
            let remaining = before_members
                .iter()
                .filter(|id| proposed.elements.iter().any(|element| &element.id == *id))
                .count();
            let outcome = if remaining == 0 {
                StructuralGroupOutcome::Removed
            } else if before_members == after_members
                && source_members_are_contiguous(proposed, after)
            {
                let types_unchanged = before_members.iter().all(|id| {
                    let old = baseline.elements.iter().find(|element| &element.id == id);
                    let new = proposed.elements.iter().find(|element| &element.id == id);
                    old.zip(new)
                        .is_some_and(|(old, new)| old.element_type() == new.element_type())
                });
                if types_unchanged {
                    StructuralGroupOutcome::UnchangedValid
                } else {
                    StructuralGroupOutcome::UpdatedValid
                }
            } else if active_dual_dialogue_projection(proposed, after).is_none() {
                StructuralGroupOutcome::InvalidRequiresUnwrap
            } else {
                StructuralGroupOutcome::UpdatedValid
            };
            StructuralGroupClassification {
                group_id: before.id.clone(),
                outcome,
                touched,
            }
        })
        .collect()
}

fn source_members_are_contiguous(project: &Project, group: &DualDialogueGroup) -> bool {
    let Some(start) = first_member(project, group) else {
        return false;
    };
    group.member_ids().enumerate().all(|(offset, id)| {
        start
            .checked_add(offset)
            .and_then(|index| project.elements.get(index))
            .is_some_and(|element| &element.id == id)
    })
}

fn group_touched_by_selection(
    project: &Project,
    group: &DualDialogueGroup,
    selection: Selection,
) -> bool {
    let (start, end) = if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    };
    let Some(first) = first_member(project, group) else {
        return false;
    };
    let Some(last) = last_member(project, group) else {
        return false;
    };
    if start == end {
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
    }
}

/// Remove one staged member identity from ordered source membership. Used only
/// by the final-right Enter exception, where the Behavior-target paragraph
/// remains outside the retained wrapper.
pub fn remove_staged_group_member(project: &mut Project, id: &StableId) {
    for group in &mut project.dual_dialogue_groups {
        group.members.retain(|member| member != id);
    }
}

pub fn first_member(project: &Project, group: &DualDialogueGroup) -> Option<usize> {
    project
        .elements
        .iter()
        .position(|element| element.id == *group.first_member())
}

pub fn last_member(project: &Project, group: &DualDialogueGroup) -> Option<usize> {
    project
        .elements
        .iter()
        .position(|element| element.id == *group.last_member())
}

pub fn left_flat_range(project: &Project, group: &DualDialogueGroup) -> Option<Range<usize>> {
    side_flat_range(
        project,
        &active_dual_dialogue_projection(project, group)?.left,
    )
}

pub fn right_flat_range(project: &Project, group: &DualDialogueGroup) -> Option<Range<usize>> {
    side_flat_range(
        project,
        &active_dual_dialogue_projection(project, group)?.right,
    )
}

fn side_flat_range(project: &Project, side: &DualDialogueSide) -> Option<Range<usize>> {
    let start = project
        .elements
        .iter()
        .position(|element| element.id == side.character)?;
    let end = match side.paragraphs.last() {
        Some(end_id) => project
            .elements
            .iter()
            .position(|element| &element.id == end_id)?,
        None => start,
    };
    (end >= start).then_some(start..end + 1)
}

pub fn side_at_element_index(project: &Project, index: usize) -> Option<DualDialogueSideKind> {
    let id = &project.elements.get(index)?.id;
    let group = group_at_element_id(project, id)?;
    let projection = active_dual_dialogue_projection(project, group)?;
    if projection.left.member_ids().any(|member| member == id) {
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
pub fn insert_group_member_at_flat_index(
    project: &mut Project,
    index: usize,
    anchor: &StableId,
    new_id: StableId,
) -> Result<(), ModelError> {
    let Some(group_index) = project
        .dual_dialogue_groups
        .iter()
        .position(|group| group.member_ids().any(|member| member == anchor))
    else {
        return Ok(());
    };
    let group = &project.dual_dialogue_groups[group_index];
    let positions = group
        .member_ids()
        .map(|id| project.elements.iter().position(|item| &item.id == id))
        .collect::<Option<Vec<_>>>()
        .ok_or(ModelError::InvalidDualDialogue(
            DualDialogueError::MissingMember,
        ))?;
    let first = *positions.first().ok_or(ModelError::InvalidDualDialogue(
        DualDialogueError::MissingMember,
    ))?;
    let last = *positions.last().ok_or(ModelError::InvalidDualDialogue(
        DualDialogueError::MissingMember,
    ))?;
    if index < first || index > last.saturating_add(1) {
        return Ok(());
    }
    let member_index = positions.partition_point(|position| *position < index);
    project.dual_dialogue_groups[group_index]
        .members
        .insert(member_index, new_id);
    Ok(())
}

/// Removes deleted Dialogue/Parenthetical identities from membership lists.
/// Cue identities intentionally remain required, so deleting a cue is rejected
/// by complete group validation rather than silently changing group identity.
pub fn prune_removed_group_paragraphs(before: &Project, after: &mut Project) {
    let retained = after
        .elements
        .iter()
        .map(|item| item.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let prior = before
        .elements
        .iter()
        .map(|item| item.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for group in &mut after.dual_dialogue_groups {
        group
            .members
            .retain(|id| !prior.contains(id) || retained.contains(id));
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
    let Some(first) = project.elements.get(start) else {
        return Ok(None);
    };
    if !has_capability(project, first, |capabilities| capabilities.character_cue) {
        return Ok(None);
    }
    if (0..start).any(|prior| {
        project.elements.get(prior).is_some_and(|element| {
            has_capability(project, element, |capabilities| capabilities.character_cue)
        }) && candidate_shape_end(project, prior).is_some_and(|end| start < end)
    }) {
        return Ok(None);
    }
    let left_begin = start.checked_add(1).ok_or(ModelError::InvalidNumber)?;
    let Some(left_end) = run_end(project, left_begin) else {
        return Ok(None);
    };
    let right_cue_index = left_end;
    let Some(right_cue) = project.elements.get(right_cue_index) else {
        return Ok(None);
    };
    if !has_capability(project, right_cue, |capabilities| {
        capabilities.character_cue
    }) {
        return Ok(None);
    }
    let right_begin = right_cue_index
        .checked_add(1)
        .ok_or(ModelError::InvalidNumber)?;
    let Some(end) = run_end(project, right_begin) else {
        return Ok(None);
    };
    if project.elements.get(end).is_some_and(|element| {
        has_capability(project, element, |capabilities| capabilities.character_cue)
    }) {
        return Ok(None);
    }
    if project.dual_dialogue_groups.iter().any(|group| {
        let first = first_member(project, group);
        let last = last_member(project, group);
        first
            .zip(last)
            .is_some_and(|(first, last)| start <= last && end >= first)
    }) {
        return Ok(None);
    }
    let left = DualDialogueSide {
        character: first.id.clone(),
        paragraphs: project.elements[left_begin..left_end]
            .iter()
            .map(|element| element.id.clone())
            .collect(),
    };
    let right = DualDialogueSide {
        character: right_cue.id.clone(),
        paragraphs: project.elements[right_begin..end]
            .iter()
            .map(|element| element.id.clone())
            .collect(),
    };
    Ok(Some(CandidatePattern {
        left,
        right,
        flat_range: start..end,
    }))
}

fn run_end(project: &Project, start: usize) -> Option<usize> {
    if start > project.elements.len() {
        return None;
    }
    let mut end = start;
    while project
        .elements
        .get(end)
        .is_some_and(|element| has_capability(project, element, is_dual_body_member))
    {
        end = end.checked_add(1)?;
    }
    Some(end)
}

fn candidate_shape_end(project: &Project, start: usize) -> Option<usize> {
    if !project.elements.get(start).is_some_and(|element| {
        has_capability(project, element, |capabilities| capabilities.character_cue)
    }) {
        return None;
    }
    let left_end = run_end(project, start.checked_add(1)?)?;
    if !project.elements.get(left_end).is_some_and(|element| {
        has_capability(project, element, |capabilities| capabilities.character_cue)
    }) {
        return None;
    }
    let end = run_end(project, left_end.checked_add(1)?)?;
    project
        .elements
        .get(end)
        .map_or(true, |element| {
            !has_capability(project, element, |capabilities| capabilities.character_cue)
        })
        .then_some(end)
}

fn element_capabilities(
    project: &Project,
    element: &crate::BodyOwner,
) -> Option<SemanticCapabilities> {
    project
        .element_semantics(&element.element_type())
        .map(|semantics| semantics.capabilities())
}

fn has_capability(
    project: &Project,
    element: &crate::BodyOwner,
    predicate: impl FnOnce(SemanticCapabilities) -> bool,
) -> bool {
    element_capabilities(project, element).is_some_and(predicate)
}

fn is_dual_body_member(capabilities: SemanticCapabilities) -> bool {
    capabilities.dialogue || capabilities.parenthetical
}

/// Resolve a valid toggle target for a collapsed Body caret. TitlePage and ranged selections are ineligible.
pub fn resolve_toggle_target(
    project: &Project,
    surface: SelectionSurface,
    selection: Selection,
) -> Result<Option<ToggleTarget>, ModelError> {
    project.validate()?;
    resolve_toggle_target_for_validated_session(project, surface, selection)
}

/// Post-validation entry point for the scriptcraft-session editing-session consumer.
/// Callers must maintain complete project validity at their construction and
/// accepted-candidate publication boundaries; this is not an arbitrary-project API.
#[doc(hidden)]
pub fn resolve_toggle_target_for_validated_session(
    project: &Project,
    surface: SelectionSurface,
    selection: Selection,
) -> Result<Option<ToggleTarget>, ModelError> {
    if surface != SelectionSurface::Body || selection.anchor != selection.focus {
        return Ok(None);
    }
    let Some(element) = project.elements.get(selection.focus.element) else {
        return Ok(None);
    };
    element.validate_boundary(selection.focus.offset)?;
    if let Some(group) = group_at_element_id(project, &element.id) {
        return Ok(Some(ToggleTarget::Disable(group.id.clone())));
    }
    if !has_capability(project, element, |capabilities| capabilities.character_cue) {
        return Ok(None);
    }
    Ok(recognize_candidate_unchecked(project, selection.focus.element)?.map(ToggleTarget::Enable))
}
