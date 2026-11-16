//! Bounded scene cast-list derivation and detached save candidates.

use crate::preservation::PreservationError;
use crate::scene_metadata::SceneMetadataError;
use crate::{ModelError, Project, Role, ScreenplayElement, StableId, TextRun, MAX_TEXT_BYTES};
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
/// independent scenes continue. Source projection failures defer only the
/// affected scene; they do not reject screenplay edits or Save.
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
    outcome.redundant_preservation_clones = 0;
    Ok((candidate, outcome, deferred_scenes, completed_scenes))
}

fn correct_cast_lists_candidate(
    candidate: &mut Project,
    requested_scenes: Option<&BTreeSet<String>>,
) -> Result<(CastListCorrection, BTreeSet<String>, BTreeSet<String>), CastListError> {
    let mut all_scenes = BTreeSet::new();
    let mut scene_positions = BTreeMap::new();
    let mut previous_heading = None;
    for (index, element) in candidate.elements.iter().enumerate() {
        if element.role != Role::SceneHeading {
            continue;
        }
        if let Some((heading_id, start)) = previous_heading.take() {
            scene_positions
                .entry(heading_id)
                .or_insert((start, index));
        }
        let heading_id = element.id.as_str().to_owned();
        all_scenes.insert(heading_id.clone());
        previous_heading = Some((heading_id, index));
    }
    if let Some((heading_id, start)) = previous_heading {
        scene_positions
            .entry(heading_id)
            .or_insert((start, candidate.elements.len()));
    }
    let scope = requested_scenes
        .cloned()
        .unwrap_or_else(|| all_scenes.clone());
    let mut deferred_scenes = scope.difference(&all_scenes).cloned().collect::<BTreeSet<_>>();
    let mut changed_scenes = BTreeSet::new();
    let mut attempted_scenes = BTreeSet::new();
    let mut scene_candidate_clones = 0u64;
    let mut project_candidate_clones = 1u64;

    // Current scenes use the read-only classifier. Only scenes that need a
    // projection mutation are given detached project candidates; scene-local
    // source limitations leave their old derived values intact while
    // independent scenes can still refresh and commit.
    for heading_id in scope.intersection(&all_scenes) {
        attempted_scenes.insert(heading_id.clone());
        let Some(&(start, end)) = scene_positions.get(heading_id) else {
            deferred_scenes.insert(heading_id.clone());
            continue;
        };
        let (canonical_names, cast_currency, stable_heading_id) = {
            let scene = &candidate.elements[start + 1..end];
            let canonical_names = crate::scene_metadata::derived_character_projection(scene);
            let cast_currency = classify_cast_list_currency(scene, &canonical_names);
            (
                canonical_names,
                cast_currency,
                candidate.elements[start].id.clone(),
            )
        };
        match classify_scene_currency_after_cast_check(
            candidate,
            &stable_heading_id,
            &canonical_names,
            cast_currency,
        ) {
            crate::scene_metadata::SceneCurrency::Current => continue,
            crate::scene_metadata::SceneCurrency::Deferred => {
                deferred_scenes.insert(heading_id.clone());
                continue;
            }
            crate::scene_metadata::SceneCurrency::NeedsUpdate => {}
        }

        let mut scene_candidate = candidate.clone();
        project_candidate_clones = project_candidate_clones.saturating_add(1);
        let reconciliation_scope = BTreeSet::from([heading_id.clone()]);
        let reconciliation = match crate::scene_metadata::reconcile_project_candidate_with_scope(
            &mut scene_candidate,
            Some(&reconciliation_scope),
        ) {
            Ok(reconciliation) => reconciliation,
            Err(_) => {
                deferred_scenes.insert(heading_id.clone());
                continue;
            }
        };
        scene_candidate_clones = scene_candidate_clones
            .saturating_add(reconciliation.scene_candidates);
        if reconciliation.deferred_scenes.contains(heading_id) {
            deferred_scenes.insert(heading_id.clone());
            continue;
        }

        let names = if scene_candidate.preservation.is_some() {
            match crate::scene_metadata::scene_arc_names(
                &scene_candidate,
                &scene_candidate.elements[start].id,
            ) {
                Ok(names) => names,
                Err(_) => {
                    deferred_scenes.insert(heading_id.clone());
                    continue;
                }
            }
        } else {
            crate::scene_metadata::derived_character_projection(
                &scene_candidate.elements[start + 1..end],
            )
            .into_iter()
            .map(|(_, name)| name)
            .collect()
        };
        let cast = match scene_cast_text(&names) {
            Ok(cast) => cast,
            Err(_) => {
                deferred_scenes.insert(heading_id.clone());
                continue;
            }
        };
        let mut scene_changed = reconciliation.changed;
        let mut failed = false;
        for element_index in start + 1..end {
            if scene_candidate.elements[element_index].role != Role::CastList
                || scene_candidate.elements[element_index].text == cast
            {
                continue;
            }
            if replace_cast_list_text(&mut scene_candidate, element_index, &cast).is_err() {
                failed = true;
                break;
            }
            scene_changed = true;
        }
        if failed || scene_candidate.validate_current_state().is_err() {
            deferred_scenes.insert(heading_id.clone());
            continue;
        }
        if scene_changed {
            changed_scenes.insert(heading_id.clone());
            *candidate = scene_candidate;
        }
    }

    let changed = !changed_scenes.is_empty();
    let completed_scenes = attempted_scenes
        .difference(&deferred_scenes)
        .cloned()
        .collect::<BTreeSet<_>>();
    let unchanged_scenes = completed_scenes
        .difference(&changed_scenes)
        .count();
    let outcome = CastListCorrection {
        changed,
        deferred: !deferred_scenes.is_empty(),
        requested_scenes: scope.len() as u64,
        attempted_scenes: attempted_scenes.len() as u64,
        changed_scenes: changed_scenes.len() as u64,
        unchanged_scenes: unchanged_scenes as u64,
        deferred_scenes: deferred_scenes.len() as u64,
        project_candidate_clones,
        scene_candidate_clones,
        redundant_preservation_clones: 0,
    };
    Ok((outcome, deferred_scenes, completed_scenes))
}

fn cast_list_source_is_replaceable(
    scene: &[crate::ScreenplayElement],
    replacement: &str,
) -> bool {
    for element in scene
        .iter()
        .filter(|element| element.role == Role::CastList)
    {
        if element.text != replacement
            && element.runs.iter().any(|run| !run.attributes.is_empty())
        {
            return false;
        }
    }
    true
}

/// Classifies one scene against already-derived canonical Character names and
/// its current body slice. This is read-only with respect to project content;
/// only the preservation state's lazy lookup caches may be populated.
pub(crate) fn classify_scene_currency(
    project: &mut Project,
    heading_id: &StableId,
    canonical_names: &[(String, String)],
    scene: &[ScreenplayElement],
) -> crate::scene_metadata::SceneCurrency {
    let cast_currency = classify_cast_list_currency(scene, canonical_names);
    classify_scene_currency_after_cast_check(project, heading_id, canonical_names, cast_currency)
}

fn classify_cast_list_currency(
    scene: &[ScreenplayElement],
    canonical_names: &[(String, String)],
) -> crate::scene_metadata::SceneCurrency {
    let names = canonical_names
        .iter()
        .map(|(_, name)| name.clone())
        .collect::<Vec<_>>();
    let expected_cast = match scene_cast_text(&names) {
        Ok(cast) => cast,
        Err(_) => return crate::scene_metadata::SceneCurrency::Deferred,
    };
    if !cast_list_source_is_replaceable(scene, &expected_cast) {
        return crate::scene_metadata::SceneCurrency::Deferred;
    }
    if scene
        .iter()
        .filter(|element| element.role == Role::CastList)
        .all(|element| element.text == expected_cast)
    {
        crate::scene_metadata::SceneCurrency::Current
    } else {
        crate::scene_metadata::SceneCurrency::NeedsUpdate
    }
}

fn classify_scene_currency_after_cast_check(
    project: &mut Project,
    heading_id: &StableId,
    canonical_names: &[(String, String)],
    cast_currency: crate::scene_metadata::SceneCurrency,
) -> crate::scene_metadata::SceneCurrency {
    if cast_currency == crate::scene_metadata::SceneCurrency::Deferred {
        return crate::scene_metadata::SceneCurrency::Deferred;
    }
    // Source-less projects have no preserved scene metadata to reconcile; the
    // Cast List is their complete derived projection.
    if project.preservation.is_none() {
        return cast_currency;
    }
    match crate::scene_metadata::classify_scene_metadata_currency(
        project,
        heading_id,
        canonical_names,
    ) {
        crate::scene_metadata::SceneCurrency::Deferred => {
            crate::scene_metadata::SceneCurrency::Deferred
        }
        crate::scene_metadata::SceneCurrency::NeedsUpdate => {
            crate::scene_metadata::SceneCurrency::NeedsUpdate
        }
        crate::scene_metadata::SceneCurrency::Current => cast_currency,
    }
}

/// Returns a corrected clone suitable for Save or Save As preflight.
/// The caller owns publication and must do so only after successful storage.
pub fn prepare_save_candidate(project: &Project) -> Result<Project, CastListError> {
    corrected_cast_list_candidate(project).map(|(candidate, _)| candidate)
}

pub(crate) fn prepare_save_candidate_with_deferrals(
    project: &Project,
) -> Result<(Project, BTreeSet<String>), CastListError> {
    corrected_cast_list_candidate_with_scope(project, None)
        .map(|(candidate, _, deferred_scenes, _)| (candidate, deferred_scenes))
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
    let provenance = element.source_provenance.clone();
    let original_text = element.text.clone();
    let element = &mut project.elements[element_index];
    element.text = replacement.to_owned();
    element.runs = if replacement.is_empty() {
        Vec::new()
    } else {
        vec![TextRun { text: replacement.to_owned(), attributes: Vec::new() }]
    };
    if let Some(preservation) = project.preservation.as_mut() {
        if let Some(recovered) = preservation.project_managed_text_runs(
            &id,
            &original_text,
            &element.text,
            &element.runs,
            provenance.as_ref(),
            Some("Cast List"),
        ).map_err(CastListError::Preservation)? {
            element.source_provenance = Some(recovered);
        }
    }
    Ok(())
}

#[cfg(test)]
mod scene_currency_tests {
    use super::*;
    use crate::fdx::read_v6;
    use crate::preservation::{PreservedNode, SourceOwnership};
    use crate::scene_metadata::{
        classify_heading_metadata_currency, derived_character_projection,
        SceneCurrency, SceneParagraphIdAllocator,
    };
    use crate::paragraph_id::ParagraphIdAllocationError;
    use crate::{RunAttribute, StableId};

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/user_paragraph_data.fdx");

    fn current_scene() -> (Project, StableId, Vec<ScreenplayElement>, Vec<(String, String)>) {
        let source = std::str::from_utf8(FIXTURE)
            .unwrap_or_else(|_| unreachable!("checked-in fixture is UTF-8"));
        let source = source
            .replacen(
                "Type=\"Action\" id=\"body-001\"",
                "Type=\"Scene Heading\" id=\"body-001\"",
                1,
            )
            .replacen("Opening action.", "INT. LIVING ROOM - DAY", 1)
            .replacen(
                "</DualDialogue>\n    </Paragraph>\n  </Content>",
                "</DualDialogue>\n    </Paragraph>\n    <Paragraph Type=\"Cast List\" id=\"cast-001\"><Text>(old)</Text></Paragraph>\n  </Content>",
                1,
            );
        let mut project = read_v6(source.as_bytes())
            .unwrap_or_else(|_| unreachable!("scene fixture is supported"))
            .project;
        correct_cast_lists(&mut project)
            .unwrap_or_else(|_| unreachable!("scene fixture supports derived projection"));
        let heading_id = StableId::new("body-001")
            .unwrap_or_else(|_| unreachable!("fixture heading id is valid"));
        let start = project
            .elements
            .iter()
            .position(|element| element.id == heading_id)
            .unwrap_or_else(|| unreachable!("fixture heading is imported"));
        let end = project.elements[start + 1..]
            .iter()
            .position(|element| element.role == Role::SceneHeading)
            .map_or(project.elements.len(), |offset| start + 1 + offset);
        let scene = project.elements[start + 1..end].to_vec();
        let names = derived_character_projection(&scene);
        (project, heading_id, scene, names)
    }

    fn classify(
        project: &mut Project,
        heading_id: &StableId,
        scene: &[ScreenplayElement],
        names: &[(String, String)],
    ) -> SceneCurrency {
        classify_scene_currency(project, heading_id, names, scene)
    }

    fn heading_mut<'a>(
        project: &'a mut Project,
        id: &StableId,
    ) -> &'a mut crate::preservation::PreservedElement {
        project
            .preservation
            .as_mut()
            .and_then(|state| state.managed_element_mut_indexed(id).ok())
            .unwrap_or_else(|| unreachable!("fixture heading is managed"))
    }

    #[test]
    fn current_scene_is_read_only_and_repeatable_without_project_candidates() {
        let (mut project, heading_id, scene, names) = current_scene();
        let original_elements = project.elements.clone();
        let original_sections = project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("fixture retains source"))
            .sections
            .clone();
        let original_identities = project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("fixture retains source"))
            .identities
            .clone();

        assert_eq!(classify(&mut project, &heading_id, &scene, &names), SceneCurrency::Current);
        assert_eq!(classify(&mut project, &heading_id, &scene, &names), SceneCurrency::Current);
        assert_eq!(project.elements, original_elements);
        let state = project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("fixture retains source"));
        assert_eq!(state.sections, original_sections);
        assert_eq!(state.identities, original_identities);
    }

    #[test]
    fn canonical_character_order_and_case_changes_need_update() {
        let (mut project, heading_id, scene, names) = current_scene();
        let mut reordered = names.clone();
        reordered.reverse();
        assert_eq!(
            classify(&mut project, &heading_id, &scene, &reordered),
            SceneCurrency::NeedsUpdate
        );

        let (mut project, heading_id, scene, mut names) = current_scene();
        names[0].1.make_ascii_lowercase();
        assert_eq!(
            classify(&mut project, &heading_id, &scene, &names),
            SceneCurrency::NeedsUpdate
        );
    }

    #[test]
    fn changed_arc_metadata_and_stale_cast_list_need_update() {
        let (mut project, heading_id, scene, names) = current_scene();
        let heading = heading_mut(&mut project, &heading_id);
        let properties = heading.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
                Some(element)
            }
            _ => None,
        }).unwrap_or_else(|| unreachable!("current heading has generated properties"));
        let arcs = properties.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats" => {
                Some(element)
            }
            _ => None,
        }).unwrap_or_else(|| unreachable!("current properties have generated arcs"));
        let beat = arcs.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "CharacterArcBeat" => {
                Some(element)
            }
            _ => None,
        }).unwrap_or_else(|| unreachable!("fixture has character arcs"));
        beat.attributes[0].value.push_str(" CHANGED");
        assert_eq!(
            classify(&mut project, &heading_id, &scene, &names),
            SceneCurrency::NeedsUpdate
        );

        let (mut project, heading_id, mut scene, names) = current_scene();
        let cast = scene
            .iter_mut()
            .find(|element| element.role == Role::CastList)
            .unwrap_or_else(|| unreachable!("fixture contains a Cast List"));
        cast.text = "(STALE)".to_owned();
        cast.runs.clear();
        assert_eq!(
            classify(&mut project, &heading_id, &scene, &names),
            SceneCurrency::NeedsUpdate
        );
    }

    #[test]
    fn unsupported_missing_and_formatted_scene_sources_defer() {
        let (mut project, heading_id, scene, names) = current_scene();
        let heading = heading_mut(&mut project, &heading_id);
        let properties = heading.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
                Some(element)
            }
            _ => None,
        }).unwrap_or_else(|| unreachable!("current heading has generated properties"));
        properties.attributes[0].value = "unsupported".to_owned();
        assert_eq!(
            classify(&mut project, &heading_id, &scene, &names),
            SceneCurrency::Deferred
        );

        let (mut project, _, scene, names) = current_scene();
        let missing = StableId::new("missing-heading")
            .unwrap_or_else(|_| unreachable!("test heading id is valid"));
        assert_eq!(
            classify(&mut project, &missing, &scene, &names),
            SceneCurrency::Deferred
        );

        let (mut project, heading_id, mut scene, names) = current_scene();
        let cast_index = scene
            .iter()
            .position(|element| element.role == Role::CastList)
            .unwrap_or_else(|| unreachable!("fixture contains a Cast List"));
        scene[cast_index].text = "(STALE)".to_owned();
        scene[cast_index].runs = vec![TextRun {
            text: "(STALE)".to_owned(),
            attributes: vec![RunAttribute {
                name: "Bold".to_owned(),
                value: "Yes".to_owned(),
            }],
        }];
        assert_eq!(
            classify(&mut project, &heading_id, &scene, &names),
            SceneCurrency::Deferred
        );
    }

    struct AllocationFailure;

    impl SceneParagraphIdAllocator for AllocationFailure {
        fn allocate(&mut self) -> Result<String, ParagraphIdAllocationError> {
            Err(ParagraphIdAllocationError::EntropyUnavailable)
        }
    }

    #[test]
    fn paragraph_identity_allocation_failure_defers_without_mutating_heading() {
        let (mut project, heading_id, _scene, names) = current_scene();
        let heading = heading_mut(&mut project, &heading_id).clone();
        let mut missing_arcs = heading.clone();
        if let Some(PreservedNode::Element(properties)) = missing_arcs
            .children
            .iter_mut()
            .find(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "SceneProperties"))
        {
            if let Some(PreservedNode::Element(arcs)) = properties
                .children
                .iter_mut()
                .find(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats"))
            {
                arcs.children.clear();
            }
        }
        let mut allocator = AllocationFailure;
        assert_eq!(
            classify_heading_metadata_currency(&missing_arcs, 1, false, &names, &mut allocator),
            SceneCurrency::Deferred
        );
        assert_eq!(heading_mut(&mut project, &heading_id), &heading);
        assert!(matches!(heading.ownership, SourceOwnership::Managed(_)));
    }
}
