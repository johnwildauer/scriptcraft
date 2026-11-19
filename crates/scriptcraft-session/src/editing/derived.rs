//! Derived invalidation classification and safe-stop refresh scheduling.

use super::{EditError, EditingSession};
use scriptcraft_model::cast_list::{self, CastListCorrection};
use scriptcraft_model::{BodyOwner, Project, Role, ScreenplayElement};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Value-free view of session-owned derived invalidation for scheduler policy.
/// Scene identities remain internal to the core handoff.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DerivedWorkSummary {
    /// Whether any derived invalidation remains, including safely deferred scenes.
    pub pending: bool,
    pub complete: bool,
    pub affected_scene_count: usize,
    /// Whether the scheduler can make progress now. Deferred-only work waits for
    /// a later relevant edit instead of being retried on every timer tick.
    pub retryable: bool,
    /// Number of affected scenes currently held by safe-stop deferral.
    pub deferred_scene_count: usize,
    /// Whether a complete refresh failed and is suspended pending invalidation.
    pub failed_complete: bool,
    /// Number of affected scenes whose correction failed and is suspended.
    pub failed_scene_count: usize,
}

/// Pending derived work. Scene-heading identities are deliberately not exposed
/// through the public summary or diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingDerivedInvalidation {
    None,
    Scenes(BTreeSet<String>),
    Complete,
}

impl PendingDerivedInvalidation {
    fn summary(&self) -> DerivedWorkSummary {
        match self {
            Self::None => DerivedWorkSummary::default(),
            Self::Scenes(scenes) => DerivedWorkSummary {
                pending: !scenes.is_empty(),
                complete: false,
                affected_scene_count: scenes.len(),
                retryable: !scenes.is_empty(),
                deferred_scene_count: 0,
                failed_complete: false,
                failed_scene_count: 0,
            },
            Self::Complete => DerivedWorkSummary {
                pending: true,
                complete: true,
                affected_scene_count: 0,
                retryable: true,
                deferred_scene_count: 0,
                failed_complete: false,
                failed_scene_count: 0,
            },
        }
    }

    pub(super) fn merge(&mut self, newer: Self) {
        match (self, newer) {
            (current @ Self::Complete, _) => *current = Self::Complete,
            (current, Self::Complete) => *current = Self::Complete,
            (Self::None, Self::None) => {}
            (current @ Self::None, Self::Scenes(scenes)) => {
                *current = Self::Scenes(scenes);
            }
            (Self::Scenes(_), Self::None) => {}
            (Self::Scenes(current), Self::Scenes(scenes)) => current.extend(scenes),
        }
    }
}

impl EditingSession {
    pub(crate) fn reset_derived_after_saved_rebase(&mut self) {
        self.pending_derived_invalidation = PendingDerivedInvalidation::None;
        self.deferred_derived_scenes.clear();
        self.failed_derived_complete = false;
        self.failed_derived_scenes.clear();
    }

    /// Returns value-free derived-work state without disclosing document or
    /// scene identities. A complete refresh reports zero individually affected
    /// scenes because its scope is the whole document. Deferred-only work stays
    /// pending but is not retryable until a later relevant invalidation.
    pub fn pending_derived_work_summary(&self) -> DerivedWorkSummary {
        let mut summary = self.pending_derived_invalidation.summary();
        match &self.pending_derived_invalidation {
            PendingDerivedInvalidation::Complete => {
                summary.failed_complete = self.failed_derived_complete;
                summary.retryable = !self.failed_derived_complete;
            }
            PendingDerivedInvalidation::Scenes(scenes) => {
                summary.deferred_scene_count =
                    scenes.intersection(&self.deferred_derived_scenes).count();
                summary.failed_scene_count =
                    scenes.intersection(&self.failed_derived_scenes).count();
                summary.retryable = scenes
                    .difference(&self.deferred_derived_scenes)
                    .find(|scene| !self.failed_derived_scenes.contains(*scene))
                    .is_some();
            }
            PendingDerivedInvalidation::None => {}
        }
        summary
    }

    /// Completes only work actually reconciled by a successful correction.
    /// Scoped completion cannot consume a pending complete refresh.
    pub(super) fn complete_derived_invalidation(
        &mut self,
        completed_scenes: &BTreeSet<String>,
        deferred_scenes: &BTreeSet<String>,
        complete_refresh: bool,
    ) {
        if complete_refresh {
            self.pending_derived_invalidation = if deferred_scenes.is_empty() {
                PendingDerivedInvalidation::None
            } else {
                PendingDerivedInvalidation::Scenes(deferred_scenes.clone())
            };
            self.deferred_derived_scenes = deferred_scenes.clone();
            self.failed_derived_complete = false;
            self.failed_derived_scenes.clear();
            return;
        }
        if let PendingDerivedInvalidation::Scenes(pending) = &mut self.pending_derived_invalidation
        {
            pending.retain(|scene| !completed_scenes.contains(scene));
            if pending.is_empty() {
                self.pending_derived_invalidation = PendingDerivedInvalidation::None;
            }
        }
        self.failed_derived_scenes
            .retain(|scene| !completed_scenes.contains(scene));
        if let PendingDerivedInvalidation::Scenes(pending) = &mut self.pending_derived_invalidation
        {
            pending.extend(deferred_scenes.iter().cloned());
        }
        self.deferred_derived_scenes
            .retain(|scene| !completed_scenes.contains(scene));
        self.deferred_derived_scenes
            .extend(deferred_scenes.iter().cloned());
    }

    /// Refreshes derived cast lists without adding a separate undo entry.
    /// Selection is unchanged; dirty state and revision follow the corrected
    /// project so cached layout and the clean baseline remain accurate.
    pub fn refresh_cast_lists(&mut self) -> Result<bool, EditError> {
        self.refresh_cast_lists_with_outcome()
            .map(|outcome| outcome.changed)
    }

    /// Refreshes derived data and reports safe-stop deferrals without exposing
    /// document-owned identities or content to the caller.
    pub fn refresh_cast_lists_with_outcome(&mut self) -> Result<CastListCorrection, EditError> {
        let (scope, complete_refresh) = match &self.pending_derived_invalidation {
            PendingDerivedInvalidation::None => {
                return Ok(CastListCorrection::default());
            }
            PendingDerivedInvalidation::Complete => {
                if self.failed_derived_complete {
                    return Ok(CastListCorrection::default());
                }
                (None, true)
            }
            PendingDerivedInvalidation::Scenes(scenes) => {
                let retryable = scenes
                    .difference(&self.deferred_derived_scenes)
                    .filter(|scene| !self.failed_derived_scenes.contains(*scene))
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if retryable.is_empty() {
                    return Ok(CastListCorrection::default());
                }
                (Some(retryable), false)
            }
        };
        let was_clean = !self.dirty;
        let (mut project, outcome, deferred_scenes, completed_scenes) =
            match cast_list::corrected_cast_list_candidate_with_scope(&self.project, scope.as_ref())
            {
                Ok(candidate) => candidate,
                Err(error) => {
                    if complete_refresh {
                        self.failed_derived_complete = true;
                    } else if let Some(attempted_scenes) = scope {
                        self.failed_derived_scenes.extend(attempted_scenes);
                    }
                    return Err(EditError::CastList(error));
                }
            };
        if outcome.changed {
            if let Some(preservation) = project.preservation.as_mut() {
                preservation.ensure_source_identity_use_index();
            }
            self.project = project;
            self.revision = self.revision.saturating_add(1);
            if was_clean && !outcome.deferred {
                self.clean_project = self.project.clone();
            }
            self.dirty = self.project != self.clean_project;
        }
        self.complete_derived_invalidation(&completed_scenes, &deferred_scenes, complete_refresh);
        Ok(outcome)
    }
}

pub(super) fn can_skip_derived_invalidation(
    before: &Project,
    after: &Project,
    element_index: usize,
    scoped_valid: bool,
) -> bool {
    // Scoped validation is available only for nonstructural single-element
    // candidates with a complete valid baseline and unchanged non-target state.
    // Character text and scene-heading text/topology are the derived inputs.
    scoped_valid
        && before
            .elements
            .get(element_index)
            .zip(after.elements.get(element_index))
            .is_some_and(|(before, after)| {
                before.id == after.id
                    && before.element_type() == after.element_type()
                    && !matches!(
                        before.transitional_role(),
                        Role::Character | Role::SceneHeading
                    )
            })
}

pub(super) fn classify_derived_invalidation(
    before: &Project,
    after: &Project,
) -> PendingDerivedInvalidation {
    // Structural edits are matched by stable identity, not by position. A
    // duplicate identity makes the before/after correspondence ambiguous, so
    // retain the safe complete fallback rather than guessing at ownership.
    let Some(before_by_id) = unique_elements_by_id(&before.elements) else {
        return PendingDerivedInvalidation::Complete;
    };
    let Some(after_by_id) = unique_elements_by_id(&after.elements) else {
        return PendingDerivedInvalidation::Complete;
    };

    let before_heading_order = before
        .elements
        .iter()
        .filter(|element| element.transitional_role() == Role::SceneHeading)
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let after_heading_order = after
        .elements
        .iter()
        .filter(|element| element.transitional_role() == Role::SceneHeading)
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let before_heading_set = before_heading_order
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let after_heading_set = after_heading_order.iter().copied().collect::<BTreeSet<_>>();

    // Existing scene boundaries changing relative order cannot be assigned a
    // reliable local split/join mapping. Insertions and removals remain
    // classifiable through each Character cue's before/after owner.
    let before_common_heading_order = before_heading_order
        .iter()
        .copied()
        .filter(|id| after_heading_set.contains(id))
        .collect::<Vec<_>>();
    let after_common_heading_order = after_heading_order
        .iter()
        .copied()
        .filter(|id| before_heading_set.contains(id))
        .collect::<Vec<_>>();
    if before_common_heading_order != after_common_heading_order {
        return PendingDerivedInvalidation::Complete;
    }

    // The ordered Character identity/text projection is exactly the input
    // whose changes can affect a scene's generated cast and arc data. Building
    // it on both sides also catches cue insertion/deletion, role transitions,
    // cue reorder, and ownership transfer caused by heading topology edits.
    let Some(before_character_inputs) = character_projection_inputs(&before.elements) else {
        return PendingDerivedInvalidation::Complete;
    };
    let Some(after_character_inputs) = character_projection_inputs(&after.elements) else {
        return PendingDerivedInvalidation::Complete;
    };

    let mut affected = after_heading_set
        .difference(&before_heading_set)
        .map(|heading| (*heading).to_owned())
        .collect::<BTreeSet<_>>();
    let owners = before_character_inputs
        .keys()
        .chain(after_character_inputs.keys())
        .collect::<BTreeSet<_>>();
    for owner in owners {
        if before_character_inputs.get(owner) != after_character_inputs.get(owner)
            && after_heading_set.contains(owner.as_str())
        {
            affected.insert(owner.clone());
        }
    }

    // Preserve the established single-element behavior for heading text
    // changes. Newly-created/role-transitioned headings are already included
    // above; removed headings cannot be refreshed because they are no longer
    // live scene owners.
    for (id, old) in &before_by_id {
        if let Some(new) = after_by_id.get(id) {
            if old.transitional_role() == Role::SceneHeading
                && new.transitional_role() == Role::SceneHeading
                && old.text != new.text
            {
                affected.insert(id.clone());
            }
        }
    }

    if affected.is_empty() {
        PendingDerivedInvalidation::None
    } else {
        PendingDerivedInvalidation::Scenes(affected)
    }
}

fn unique_elements_by_id(
    elements: &[Arc<BodyOwner>],
) -> Option<BTreeMap<String, &ScreenplayElement>> {
    let mut by_id = BTreeMap::new();
    for element in elements {
        if by_id
            .insert(element.id.as_str().to_owned(), element.as_element())
            .is_some()
        {
            return None;
        }
    }
    Some(by_id)
}

fn character_projection_inputs(
    elements: &[Arc<BodyOwner>],
) -> Option<BTreeMap<String, Vec<(String, String)>>> {
    let mut by_scene = BTreeMap::<String, Vec<(String, String)>>::new();
    for (index, element) in elements.iter().enumerate() {
        if element.transitional_role() != Role::Character {
            continue;
        }
        let owner = owning_scene_id(elements, index)?;
        by_scene
            .entry(owner)
            .or_default()
            .push((element.id.as_str().to_owned(), element.text.clone()));
    }
    Some(by_scene)
}

fn owning_scene_id(elements: &[Arc<BodyOwner>], element_index: usize) -> Option<String> {
    elements
        .get(..=element_index)?
        .iter()
        .rev()
        .find(|element| element.transitional_role() == Role::SceneHeading)
        .map(|heading| heading.id.as_str().to_owned())
}
