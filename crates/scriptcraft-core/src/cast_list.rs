//! Bounded scene cast-list derivation and detached save candidates.

use crate::preservation::PreservationError;
use crate::scene_metadata::SceneMetadataError;
use crate::{ModelError, Project, Role, TextRun, MAX_TEXT_BYTES};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// The desktop refresh cadence for derived scene cast lists.
pub const CAST_LIST_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastListError {
    Model(ModelError),
    Preservation(PreservationError),
    SceneMetadata(SceneMetadataError),
    UnsupportedFormattedCastList,
}

/// Value-free summary of one derived cast/scene correction pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CastListCorrection {
    /// Whether the candidate project differs from the input project.
    pub changed: bool,
    /// Whether any scene or formatted Cast List was safely left untouched.
    pub deferred: bool,
    /// Number of scenes in the requested complete or affected-scene scope.
    pub requested_scenes: u64,
    /// Requested scenes whose projection/managed-heading scope was visited.
    pub attempted_scenes: u64,
    /// Attempted scenes with at least one derived value changed.
    pub changed_scenes: u64,
    /// Attempted scenes with no derived value changed and no deferral.
    pub unchanged_scenes: u64,
    /// Requested scenes safely left pending for a later relevant revision.
    pub deferred_scenes: u64,
    /// Detached whole-project candidates created by this correction call.
    pub project_candidate_clones: u64,
    /// Managed-heading candidates cloned for scene-safe reconciliation.
    pub scene_candidate_clones: u64,
    /// Complete preservation-state clones beyond the top-level candidate.
    pub redundant_preservation_clones: u64,
}

impl From<ModelError> for CastListError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

/// Recomputes every Cast List paragraph within a headed scene range.
///
/// The bounded default is Unicode lowercase-key ordering (Rust's locale-free
/// Unicode lowercase mapping), with outer Unicode whitespace trimmed, empty
/// cues ignored, and case-insensitive duplicates represented by their first
/// encountered spelling. Every Cast List paragraph in a scene receives the
/// same parenthesized list. Lists before the first heading and documents with
/// no heading are left unchanged; no list paragraph is synthesized.
///
/// Unsupported scene metadata and formatted Cast Lists are preserved while
/// independent, safely representable derived updates continue.
pub fn correct_cast_lists(project: &mut Project) -> Result<bool, CastListError> {
    correct_cast_lists_with_outcome(project).map(|outcome| outcome.changed)
}

/// Recomputes derived cast/scene data and reports whether bounded safe-stop
/// behavior deferred any part of the correction. Deferred scene identity is
/// intentionally not exposed by this value-free result.
pub fn correct_cast_lists_with_outcome(
    project: &mut Project,
) -> Result<CastListCorrection, CastListError> {
    let (candidate, outcome) = corrected_cast_list_candidate(project)?;
    if outcome.changed {
        *project = candidate;
    }
    Ok(outcome)
}

pub(crate) fn corrected_cast_list_candidate(
    project: &Project,
) -> Result<(Project, CastListCorrection), CastListError> {
    corrected_cast_list_candidate_with_scope(project, None)
        .map(|(candidate, outcome, _, _)| (candidate, outcome))
}

pub(crate) fn corrected_cast_list_candidate_with_scope(
    project: &Project,
    requested_scenes: Option<&BTreeSet<String>>,
) -> Result<
    (
        Project,
        CastListCorrection,
        BTreeSet<String>,
        BTreeSet<String>,
    ),
    CastListError,
> {
    let mut candidate = project.clone();
    let (mut outcome, deferred_scenes, completed_scenes) =
        correct_cast_lists_candidate(&mut candidate, requested_scenes)?;
    outcome.project_candidate_clones = 1;
    outcome.redundant_preservation_clones = 0;
    Ok((candidate, outcome, deferred_scenes, completed_scenes))
}

fn correct_cast_lists_candidate(
    candidate: &mut Project,
    requested_scenes: Option<&BTreeSet<String>>,
) -> Result<(CastListCorrection, BTreeSet<String>, BTreeSet<String>), CastListError> {
    let all_scenes = candidate
        .elements
        .iter()
        .filter(|element| element.role == Role::SceneHeading)
        .map(|element| element.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let scope = requested_scenes
        .cloned()
        .unwrap_or_else(|| all_scenes.clone());
    let mut source_deferred_scenes = BTreeSet::new();
    let mut scene_start: Option<usize> = None;
    for index in 0..=candidate.elements.len() {
        let is_heading = candidate
            .elements
            .get(index)
            .is_some_and(|element| element.role == Role::SceneHeading);
        if is_heading || index == candidate.elements.len() {
            if let Some(start) = scene_start {
                let scene = &candidate.elements[start + 1..index];
                let heading_id = &candidate.elements[start].id;
                if scope.contains(heading_id.as_str())
                    && scene_needs_cast_list_update(scene)?
                    && !cast_list_source_is_replaceable(candidate, scene)?
                {
                    source_deferred_scenes.insert(heading_id.as_str().to_owned());
                }
            }
            scene_start = is_heading.then_some(index);
        }
    }
    let reconciliation_scope = scope
        .difference(&source_deferred_scenes)
        .cloned()
        .collect::<BTreeSet<_>>();
    let reconciliation = crate::scene_metadata::reconcile_project_candidate_with_scope(
        candidate,
        Some(&reconciliation_scope),
    )
    .map_err(CastListError::SceneMetadata)?;
    let mut deferred_scenes = source_deferred_scenes;
    deferred_scenes.extend(reconciliation.deferred_scenes.iter().cloned());
    deferred_scenes.extend(scope.difference(&all_scenes).cloned());
    let mut changed = reconciliation.changed;
    let mut changed_scenes = reconciliation.changed_scenes.clone();
    let mut updates = Vec::new();
    let mut scene_start: Option<usize> = None;
    let mut attempted_scenes = BTreeSet::new();

    for index in 0..=candidate.elements.len() {
        let is_heading = candidate
            .elements
            .get(index)
            .is_some_and(|element| element.role == Role::SceneHeading);
        if is_heading || index == candidate.elements.len() {
            if let Some(start) = scene_start {
                let scene = &candidate.elements[start + 1..index];
                let heading_id = &candidate.elements[start].id;
                if scope.contains(heading_id.as_str()) {
                    attempted_scenes.insert(heading_id.as_str().to_owned());
                    if deferred_scenes.contains(heading_id.as_str()) {
                        scene_start = is_heading.then_some(index);
                        continue;
                    }
                }
                if scope.contains(heading_id.as_str())
                    && scene.iter().any(|element| element.role == Role::CastList)
                {
                    let names = if candidate.preservation.is_some() {
                        crate::scene_metadata::scene_arc_names(candidate, heading_id)
                            .map_err(CastListError::SceneMetadata)?
                    } else {
                        crate::scene_metadata::derived_character_projection(scene)
                            .into_iter()
                            .map(|(_, name)| name)
                            .collect()
                    };
                    let cast = scene_cast_text(&names)?;
                    for element_index in start + 1..index {
                        if candidate.elements[element_index].role == Role::CastList
                            && candidate.elements[element_index].text != cast
                        {
                            updates.push((
                                heading_id.as_str().to_owned(),
                                element_index,
                                cast.clone(),
                            ));
                        }
                    }
                }
            }
            scene_start = is_heading.then_some(index);
        }
    }

    for (heading_id, element_index, replacement) in updates {
        match replace_cast_list_text(candidate, element_index, &replacement) {
            Ok(()) => {
                changed = true;
                changed_scenes.insert(heading_id);
            }
            Err(CastListError::UnsupportedFormattedCastList) => {
                deferred_scenes.insert(heading_id);
            }
            Err(error) => return Err(error),
        }
    }
    attempted_scenes.extend(reconciliation.deferred_scenes.iter().cloned());
    if changed {
        candidate.validate()?;
    }
    let completed_scenes = attempted_scenes
        .difference(&deferred_scenes)
        .cloned()
        .collect::<BTreeSet<_>>();
    let unchanged_scenes = attempted_scenes
        .difference(&changed_scenes)
        .filter(|scene| !deferred_scenes.contains(*scene))
        .count();
    let outcome = CastListCorrection {
        changed,
        deferred: !deferred_scenes.is_empty(),
        requested_scenes: scope.len() as u64,
        attempted_scenes: attempted_scenes.len() as u64,
        changed_scenes: changed_scenes.len() as u64,
        unchanged_scenes: unchanged_scenes as u64,
        deferred_scenes: deferred_scenes.len() as u64,
        project_candidate_clones: 0,
        scene_candidate_clones: reconciliation.scene_candidates,
        redundant_preservation_clones: reconciliation.preservation_state_clones,
    };
    Ok((outcome, deferred_scenes, completed_scenes))
}

fn scene_needs_cast_list_update(scene: &[crate::ScreenplayElement]) -> Result<bool, CastListError> {
    if !scene.iter().any(|element| element.role == Role::CastList) {
        return Ok(false);
    }
    let names = crate::scene_metadata::derived_character_projection(scene)
        .into_iter()
        .map(|(_, name)| name)
        .collect::<Vec<_>>();
    let replacement = scene_cast_text(&names)?;
    Ok(scene
        .iter()
        .any(|element| element.role == Role::CastList && element.text != replacement))
}

fn cast_list_source_is_replaceable(
    project: &Project,
    scene: &[crate::ScreenplayElement],
) -> Result<bool, CastListError> {
    let names = crate::scene_metadata::derived_character_projection(scene)
        .into_iter()
        .map(|(_, name)| name)
        .collect::<Vec<_>>();
    let replacement = scene_cast_text(&names)?;
    let mut supported = true;
    for element in scene
        .iter()
        .filter(|element| element.role == Role::CastList)
    {
        if element.text == replacement {
            continue;
        }
        if element.runs.iter().any(|run| !run.attributes.is_empty()) {
            supported = false;
            continue;
        }
        if let Some(preservation) = &project.preservation {
            if preservation.has_managed_element(&element.id) {
                let expected_runs = element
                    .runs
                    .iter()
                    .map(|run| run.text.clone())
                    .collect::<Vec<_>>();
                match preservation
                    .validate_managed_text_elements_replacement(&element.id, &expected_runs)
                {
                    Ok(()) => {}
                    Err(PreservationError::UnsupportedTextMapping) => supported = false,
                    Err(error) => return Err(CastListError::Preservation(error)),
                }
            } else if element.source_provenance.is_some() {
                return Err(CastListError::Preservation(
                    PreservationError::ManagedAnchorMissing,
                ));
            }
        } else if element.source_provenance.is_some() {
            return Err(CastListError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ));
        }
    }
    Ok(supported)
}

/// Returns a corrected clone suitable for Save or Save As preflight.
/// The caller owns publication and must do so only after successful storage.
pub fn prepare_save_candidate(project: &Project) -> Result<Project, CastListError> {
    corrected_cast_list_candidate(project).map(|(candidate, _)| candidate)
}

fn scene_cast_text(names: &[String]) -> Result<String, CastListError> {
    let mut sorted_names = BTreeMap::<String, String>::new();
    let mut names_bytes = 2usize; // Parentheses.
    for source_name in names {
        let name = source_name.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_lowercase();
        if sorted_names.contains_key(&key) {
            continue;
        }
        let separator_bytes = if sorted_names.is_empty() { 0 } else { 2 };
        names_bytes = names_bytes
            .checked_add(separator_bytes)
            .and_then(|length| length.checked_add(name.len()))
            .ok_or(CastListError::Model(ModelError::TextTooLong))?;
        if names_bytes > MAX_TEXT_BYTES {
            return Err(CastListError::Model(ModelError::TextTooLong));
        }
        sorted_names.insert(key, name.to_owned());
    }

    let mut output = String::with_capacity(names_bytes);
    output.push('(');
    for (index, name) in sorted_names.values().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(name);
    }
    output.push(')');
    Ok(output)
}

fn replace_cast_list_text(
    project: &mut Project,
    element_index: usize,
    replacement: &str,
) -> Result<(), CastListError> {
    let element = project
        .elements
        .get(element_index)
        .ok_or(CastListError::Model(ModelError::InvalidNumber))?;
    if replacement.len() > MAX_TEXT_BYTES {
        return Err(CastListError::Model(ModelError::TextTooLong));
    }
    if element.runs.iter().any(|run| !run.attributes.is_empty()) {
        return Err(CastListError::UnsupportedFormattedCastList);
    }

    let id = element.id.clone();
    let old_runs = element.runs.clone();
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        let expected_runs = old_runs
            .iter()
            .map(|run| run.text.clone())
            .collect::<Vec<_>>();
        project
            .preservation
            .as_mut()
            .ok_or(CastListError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ))?
            .replace_managed_text_elements(&id, &expected_runs, replacement)
            .map_err(CastListError::Preservation)?;
    } else if element.source_provenance.is_some() {
        return Err(CastListError::Preservation(
            PreservationError::ManagedAnchorMissing,
        ));
    }

    let element = &mut project.elements[element_index];
    element.text = replacement.to_owned();
    if element.text.is_empty() {
        element.runs.clear();
    } else if let Some((first, remaining)) = element.runs.split_first_mut() {
        first.text = replacement.to_owned();
        for run in remaining {
            run.text.clear();
        }
    } else {
        // A newly inserted managed Cast List can have no semantic runs yet.
        // Source replacement creates its first `<Text>` run, so publish the
        // matching semantic run in the same candidate rather than leaving the
        // next preflight to compare an empty projection with nonempty source.
        element.runs.push(TextRun {
            text: replacement.to_owned(),
            attributes: Vec::new(),
        });
    }
    Ok(())
}
