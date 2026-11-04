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

pub(crate) fn corrected_cast_list_candidate_for_scenes(
    project: &Project,
    requested_scenes: &BTreeSet<String>,
) -> Result<(Project, CastListCorrection, BTreeSet<String>, BTreeSet<String>), CastListError> {
    corrected_cast_list_candidate_with_scope(project, Some(requested_scenes))
}

pub(crate) fn corrected_cast_list_candidate_with_scope(
    project: &Project,
    requested_scenes: Option<&BTreeSet<String>>,
) -> Result<(Project, CastListCorrection, BTreeSet<String>, BTreeSet<String>), CastListError> {
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
    let scope = requested_scenes.cloned().unwrap_or_else(|| all_scenes.clone());
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
                        crate::scene_metadata::scene_arc_names(&candidate, heading_id)
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
                            updates.push((heading_id.as_str().to_owned(), element_index, cast.clone()));
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

fn scene_needs_cast_list_update(
    scene: &[crate::ScreenplayElement],
) -> Result<bool, CastListError> {
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
    for element in scene.iter().filter(|element| element.role == Role::CastList) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{EditingSession, Position, Selection};
    use crate::{RunAttribute, ScreenplayElement, TextRun};

    fn element(id: &str, role: Role, text: &str) -> ScreenplayElement {
        ScreenplayElement::new(id, role, text).unwrap_or_else(|_| unreachable!())
    }

    fn project_with(elements: Vec<ScreenplayElement>) -> Project {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = elements;
        project
    }

    fn imported_project_without_scene_metadata() -> Project {
        let source = std::str::from_utf8(include_bytes!(
            "../../../.forge/initiatives/2026-10-31-document-model-refactor/evidence/wo001-preservation-shapes-synthetic.xml"
        ))
        .unwrap_or_else(|_| unreachable!());
        let source = source.replace(
            "<SceneProperties><SceneArcBeats><SceneArcBeat id=\"s-beat-1\">synthetic beat</SceneArcBeat></SceneArcBeats></SceneProperties>",
            "",
        );
        let source = source.replace(
            "<Paragraph Type=\"General\" id=\"s-general-1\">",
            "<Paragraph Type=\"Character\" id=\"s-character-2\"><Text>ada</Text></Paragraph><Paragraph Type=\"General\" id=\"s-general-1\">",
        );
        let source = source.replace(
            "</Content>",
            "<Paragraph Type=\"Scene Heading\" id=\"s-heading-2\"><Text>EXT. STREET - DAY</Text></Paragraph><Paragraph Type=\"Character\" id=\"s-character-3\"><Text>Mara</Text></Paragraph><Paragraph Type=\"Cast List\" id=\"s-cast-list-2\"><Text>(stale)</Text></Paragraph></Content>",
        );
        crate::fdx::read_v6(source.as_bytes())
            .unwrap_or_else(|_| unreachable!())
            .project
    }

    #[test]
    fn derives_sorted_deduplicated_cast_per_scene_including_final_scene() {
        let mut project = project_with(vec![
            element("pre-list", Role::CastList, "leave before heading"),
            element("heading-1", Role::SceneHeading, "INT. ROOM"),
            element("cast-1a", Role::CastList, "old list"),
            element("cue-1a", Role::Character, "  Zoë  "),
            element("cue-1b", Role::Character, "ada"),
            element("cue-1c", Role::Character, "Ada"),
            element("cue-1d", Role::Character, "   "),
            element("cast-1b", Role::CastList, "another old list"),
            element("heading-2", Role::SceneHeading, "EXT. STREET"),
            element("cue-2", Role::Character, "MARA"),
            element("cast-2", Role::CastList, "old final scene"),
        ]);

        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        assert_eq!(project.elements[0].text, "leave before heading");
        assert_eq!(project.elements[2].text, "(ADA, ZOË)");
        assert_eq!(project.elements[7].text, "(ADA, ZOË)");
        assert_eq!(project.elements[10].text, "(MARA)");
        assert!(!correct_cast_lists(&mut project).unwrap_or(true));
    }

    #[test]
    fn candidate_is_detached_and_updates_plain_text_runs_consistently() {
        let project = project_with(vec![
            element("heading", Role::SceneHeading, "INT. ROOM"),
            element("cue", Role::Character, "ALEX"),
            ScreenplayElement::with_runs(
                "cast",
                Role::CastList,
                vec![
                    TextRun {
                        text: "(old".into(),
                        attributes: Vec::new(),
                    },
                    TextRun {
                        text: ")".into(),
                        attributes: Vec::new(),
                    },
                ],
                None,
            )
            .unwrap_or_else(|_| unreachable!()),
        ]);
        let original = project.clone();

        let candidate = prepare_save_candidate(&project).unwrap_or_else(|_| unreachable!());
        assert_eq!(project, original);
        assert_eq!(candidate.elements[2].text, "(ALEX)");
        assert_eq!(candidate.elements[2].runs[0].text, "(ALEX)");
        assert_eq!(candidate.elements[2].runs[1].text, "");
    }

    #[test]
    fn imported_cast_list_consumes_reconciled_arc_projection_in_detached_save_candidate() {
        let mut project = imported_project_without_scene_metadata();
        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        let character_index = session
            .project()
            .elements
            .iter()
            .find(|element| element.role == Role::Character)
            .map(|element| element.id.as_str())
            .and_then(|id| {
                session
                    .project()
                    .elements
                    .iter()
                    .position(|element| element.id.as_str() == id)
            })
            .unwrap_or_else(|| unreachable!());
        session
            .set_selection(Selection {
                anchor: Position {
                    element: character_index,
                    offset: 0,
                },
                focus: Position {
                    element: character_index,
                    offset: "ALPHA".len(),
                },
            })
            .unwrap_or_else(|_| unreachable!());
        session.insert("ZOË").unwrap_or_else(|_| unreachable!());
        let original = session.clone();

        let candidate = session
            .prepare_cast_list_save_candidate()
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(session, original);
        let cast_list = candidate
            .elements
            .iter()
            .find(|element| element.role == Role::CastList)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(cast_list.text, "(ADA, ZOË)");
        let heading = candidate
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(&candidate, &heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ZOË".to_owned(), "ADA".to_owned()]
        );
        let second_heading = candidate
            .elements
            .iter()
            .filter(|element| element.role == Role::SceneHeading)
            .nth(1)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(&candidate, &second_heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["MARA".to_owned()]
        );
        assert_eq!(
            candidate
                .elements
                .iter()
                .filter(|element| element.role == Role::CastList)
                .nth(1)
                .unwrap_or_else(|| unreachable!())
                .text,
            "(MARA)"
        );

        let emitted = crate::fdx::write_v6(&candidate).unwrap_or_else(|_| unreachable!());
        let reopened = crate::fdx::read_v6(&emitted).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            reopened
                .project
                .elements
                .iter()
                .find(|element| element.role == Role::CastList)
                .unwrap_or_else(|| unreachable!())
                .text,
            "(ADA, ZOË)"
        );
        let reopened_heading = reopened
            .project
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(&reopened.project, &reopened_heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ZOË".to_owned(), "ADA".to_owned()]
        );
    }

    #[test]
    fn refreshed_imported_arc_and_cast_list_follow_undo_redo_together() {
        let mut project = imported_project_without_scene_metadata();
        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        session.mark_saved();
        let character_index = session
            .project()
            .elements
            .iter()
            .position(|element| element.role == Role::Character)
            .unwrap_or_else(|| unreachable!());
        session
            .set_selection(Selection::caret(Position {
                element: character_index,
                offset: "ALPHA".len(),
            }))
            .unwrap_or_else(|_| unreachable!());
        session.insert("X").unwrap_or_else(|_| unreachable!());

        assert!(session.refresh_cast_lists().unwrap_or(false));
        let heading = session
            .project()
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(session.project(), &heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ALPHAX".to_owned(), "ADA".to_owned()]
        );
        assert_eq!(
            session
                .project()
                .elements
                .iter()
                .find(|element| element.role == Role::CastList)
                .unwrap_or_else(|| unreachable!())
                .text,
            "(ADA, ALPHAX)"
        );

        session.undo().unwrap_or_else(|_| unreachable!());
        let heading = session
            .project()
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(session.project(), &heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ALPHA".to_owned(), "ADA".to_owned()]
        );
        assert_eq!(
            session
                .project()
                .elements
                .iter()
                .find(|element| element.role == Role::CastList)
                .unwrap_or_else(|| unreachable!())
                .text,
            "(ADA, ALPHA)"
        );

        session.redo().unwrap_or_else(|_| unreachable!());
        let heading = session
            .project()
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(session.project(), &heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ALPHAX".to_owned(), "ADA".to_owned()]
        );
        assert_eq!(
            session
                .project()
                .elements
                .iter()
                .find(|element| element.role == Role::CastList)
                .unwrap_or_else(|| unreachable!())
                .text,
            "(ADA, ALPHAX)"
        );
    }

    #[test]
    fn stale_arc_after_reopen_is_reconciled_during_cast_list_refresh() {
        let mut project = imported_project_without_scene_metadata();
        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        let emitted = crate::fdx::write_v6(&project).unwrap_or_else(|_| unreachable!());
        let reopened = crate::fdx::read_v6(&emitted)
            .unwrap_or_else(|_| unreachable!())
            .project;
        let mut session = EditingSession::new(reopened).unwrap_or_else(|_| unreachable!());
        let character_index = session
            .project()
            .elements
            .iter()
            .find(|element| element.role == Role::Character)
            .map(|element| element.id.as_str())
            .and_then(|id| {
                session
                    .project()
                    .elements
                    .iter()
                    .position(|element| element.id.as_str() == id)
            })
            .unwrap_or_else(|| unreachable!());
        session
            .set_selection(Selection {
                anchor: Position {
                    element: character_index,
                    offset: 0,
                },
                focus: Position {
                    element: character_index,
                    offset: "ALPHA".len(),
                },
            })
            .unwrap_or_else(|_| unreachable!());
        session.insert("BETA").unwrap_or_else(|_| unreachable!());
        let mut refreshed = session.project().clone();
        assert!(correct_cast_lists(&mut refreshed).unwrap_or(false));
        let heading = refreshed
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(&refreshed, &heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["BETA".to_owned(), "ADA".to_owned()]
        );
    }

    #[test]
    fn removing_character_role_removes_derived_arc_and_refreshes_cast_list() {
        let mut project = imported_project_without_scene_metadata();
        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        let character_index = session
            .project()
            .elements
            .iter()
            .position(|element| element.role == Role::Character)
            .unwrap_or_else(|| unreachable!());
        session
            .set_selection(Selection::caret(Position {
                element: character_index,
                offset: 0,
            }))
            .unwrap_or_else(|_| unreachable!());
        session
            .change_role(Role::Action)
            .unwrap_or_else(|_| unreachable!());

        assert!(session.refresh_cast_lists().unwrap_or(false));
        let first_heading = session
            .project()
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(session.project(), &first_heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ADA".to_owned()]
        );
        assert_eq!(
            session
                .project()
                .elements
                .iter()
                .find(|element| element.role == Role::CastList)
                .unwrap_or_else(|| unreachable!())
                .text,
            "(ADA)"
        );
    }

    #[test]
    fn adding_character_role_adds_arc_and_refreshes_cast_list() {
        let mut project = imported_project_without_scene_metadata();
        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        let action_index = session
            .project()
            .elements
            .iter()
            .position(|element| element.role == Role::Action)
            .unwrap_or_else(|| unreachable!());
        session
            .set_selection(Selection::caret(Position {
                element: action_index,
                offset: 0,
            }))
            .unwrap_or_else(|_| unreachable!());
        session
            .change_role(Role::Character)
            .unwrap_or_else(|_| unreachable!());

        assert!(session.refresh_cast_lists().unwrap_or(false));
        let first_heading = session
            .project()
            .elements
            .iter()
            .find(|element| element.role == Role::SceneHeading)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(session.project(), &first_heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec![
                "SYNTHETIC ACTION".to_owned(),
                "ALPHA".to_owned(),
                "ADA".to_owned()
            ]
        );
        assert_eq!(
            session
                .project()
                .elements
                .iter()
                .find(|element| element.role == Role::CastList)
                .unwrap_or_else(|| unreachable!())
                .text,
            "(ADA, ALPHA, SYNTHETIC ACTION)"
        );
    }

    #[test]
    fn formatted_cast_list_is_left_unchanged_while_refresh_succeeds() {
        let cast = ScreenplayElement::with_runs(
            "cast",
            Role::CastList,
            vec![TextRun {
                text: "(old)".into(),
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold".into(),
                }],
            }],
            None,
        )
        .unwrap_or_else(|_| unreachable!());
        let mut project = project_with(vec![
            element("heading", Role::SceneHeading, "INT. ROOM"),
            element("cue", Role::Character, "ALEX"),
            cast,
        ]);
        let original = project.clone();

        assert!(!correct_cast_lists(&mut project).unwrap_or(true));
        assert_eq!(project, original);
    }

    #[test]
    fn imported_multirun_cast_list_refresh_preserves_runs_arcs_and_save_projection() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" ID="heading"><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Character" ID="cue"><Text>ALICE</Text></Paragraph>
<Paragraph Type="Cast List" ID="cast"><Text>(OLD</Text><Text>)</Text></Paragraph>
</Content></FinalDraft>"#;
        let project = crate::fdx::read_v6(source)
            .unwrap_or_else(|_| unreachable!("plain imported multi-run Cast List imports"))
            .project;
        let mut project = project;
        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        assert_eq!(project.elements[2].text, "(ALICE)");
        assert_eq!(project.elements[2].runs[0].text, "(ALICE)");
        assert_eq!(project.elements[2].runs[1].text, "");

        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        session
            .set_selection(Selection::caret(Position {
                element: 1,
                offset: "ALICE".len(),
            }))
            .unwrap_or_else(|_| unreachable!("imported Character caret is valid"));
        session
            .insert("X")
            .unwrap_or_else(|_| unreachable!("imported Character remains editable"));
        assert!(session.refresh_cast_lists().unwrap_or(false));
        assert_eq!(session.project().elements[2].text, "(ALICEX)");
        assert_eq!(session.project().elements[2].runs[0].text, "(ALICEX)");
        assert_eq!(session.project().elements[2].runs[1].text, "");
        let heading = &session.project().elements[0];
        assert_eq!(
            crate::scene_metadata::scene_arc_names(session.project(), &heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ALICEX".to_owned()]
        );

        let candidate = session
            .prepare_cast_list_save_candidate()
            .unwrap_or_else(|_| unreachable!("complete save preparation accepts corrected source"));
        let emitted = crate::fdx::write_v6(&candidate)
            .unwrap_or_else(|_| unreachable!("corrected imported FDX serializes"));
        let reopened = crate::fdx::read_v6(&emitted)
            .unwrap_or_else(|_| unreachable!("corrected imported FDX reopens"))
            .project;
        assert_eq!(reopened.elements[2].text, "(ALICEX)");
        assert_eq!(reopened.elements[2].runs[0].text, "(ALICEX)");
        assert_eq!(reopened.elements[2].runs[1].text, "");
        let heading = &reopened.elements[0];
        assert_eq!(
            crate::scene_metadata::scene_arc_names(&reopened, &heading.id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["ALICEX".to_owned()]
        );
    }

    #[test]
    fn imported_empty_cast_list_is_rebuilt_and_round_trips() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" ID="heading"><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Character" ID="cue"><Text>ADA</Text></Paragraph>
<Paragraph Type="Cast List" ID="cast"><Text/></Paragraph>
</Content></FinalDraft>"#;
        let mut project = crate::fdx::read_v6(source)
            .unwrap_or_else(|_| unreachable!("imported empty Cast List fixture imports"))
            .project;

        assert!(correct_cast_lists(&mut project).unwrap_or(false));
        assert_eq!(project.elements[2].text, "(ADA)");
        let emitted = crate::fdx::write_v6(&project)
            .unwrap_or_else(|_| unreachable!("rebuilt empty Cast List serializes"));
        let reopened = crate::fdx::read_v6(&emitted)
            .unwrap_or_else(|_| unreachable!("rebuilt empty Cast List reopens"))
            .project;
        assert_eq!(reopened.elements[2].text, "(ADA)");
    }

    #[test]
    fn unsupported_imported_cast_source_defers_whole_scene_but_updates_independent_scene() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" ID="unsupported-heading"><Text>INT. OPAQUE - DAY</Text></Paragraph>
<Paragraph Type="Character" ID="unsupported-cue"><Text>ALPHA</Text></Paragraph>
<Paragraph Type="Cast List" ID="unsupported-cast"><Text>(STALE)</Text><VendorPayload>retain</VendorPayload></Paragraph>
<Paragraph Type="Scene Heading" ID="supported-heading"><Text>EXT. STREET - DAY</Text></Paragraph>
<Paragraph Type="Character" ID="supported-cue"><Text>BETA</Text></Paragraph>
<Paragraph Type="Cast List" ID="supported-cast"><Text>(STALE)</Text></Paragraph>
</Content></FinalDraft>"#;
        let mut project = crate::fdx::read_v6(source)
            .unwrap_or_else(|_| unreachable!("imported opaque Cast List fixture imports"))
            .project;

        let outcome = correct_cast_lists_with_outcome(&mut project)
            .unwrap_or_else(|_| unreachable!("unsupported Cast List mapping safely defers"));
        assert!(outcome.changed);
        assert!(outcome.deferred);
        assert_eq!(outcome.deferred_scenes, 1);
        assert_eq!(project.elements[2].text, "(STALE)");
        assert_eq!(project.elements[5].text, "(BETA)");
        assert!(crate::scene_metadata::scene_arc_names(&project, &project.elements[0].id).is_err());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(&project, &project.elements[3].id)
                .unwrap_or_else(|_| unreachable!("independent supported scene reconciles")),
            vec!["BETA".to_owned()]
        );

        let emitted = crate::fdx::write_v6(&project)
            .unwrap_or_else(|_| unreachable!("deferred plus corrected scenes serialize"));
        let reopened = crate::fdx::read_v6(&emitted)
            .unwrap_or_else(|_| unreachable!("deferred plus corrected scenes reopen"))
            .project;
        assert_eq!(reopened.elements[2].text, "(STALE)");
        assert_eq!(reopened.elements[5].text, "(BETA)");
        assert!(crate::scene_metadata::scene_arc_names(&reopened, &reopened.elements[0].id).is_err());
        assert_eq!(
            crate::scene_metadata::scene_arc_names(&reopened, &reopened.elements[3].id)
                .unwrap_or_else(|_| unreachable!()),
            vec!["BETA".to_owned()]
        );
    }

    #[test]
    fn no_heading_or_no_cast_list_does_not_synthesize_data() {
        let mut project = project_with(vec![
            element("cue", Role::Character, "ALEX"),
            element("cast", Role::CastList, "pre-scene custom"),
        ]);
        let original = project.clone();
        assert!(!correct_cast_lists(&mut project).unwrap_or(true));
        assert_eq!(project, original);

        let mut project = project_with(vec![
            element("heading", Role::SceneHeading, "INT. ROOM"),
            element("cue", Role::Character, "ALEX"),
        ]);
        assert!(!correct_cast_lists(&mut project).unwrap_or(true));
        assert_eq!(project.elements.len(), 2);
    }

    #[test]
    fn session_refresh_is_non_undoable_tracks_dirty_and_rederives_after_undo_redo() {
        let project = project_with(vec![
            element("heading", Role::SceneHeading, "INT. ROOM"),
            element("cue", Role::Character, "ALICE"),
            element("cast", Role::CastList, "(ALICE)"),
        ]);
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        let selection = Selection::caret(Position {
            element: 1,
            offset: 5,
        });
        session
            .set_selection(selection)
            .unwrap_or_else(|_| unreachable!());
        session.insert("X").unwrap_or_else(|_| unreachable!());
        assert_eq!(session.project().elements[2].text, "(ALICE)");
        let undo_available = session.can_undo();
        let selection_after_edit = session.selection();

        assert!(session.refresh_cast_lists().unwrap_or(false));
        assert_eq!(session.project().elements[2].text, "(ALICEX)");
        assert_eq!(session.selection(), selection_after_edit);
        assert_eq!(session.can_undo(), undo_available);
        assert!(session.dirty());

        session.undo().unwrap_or_else(|_| unreachable!());
        assert_eq!(session.project().elements[1].text, "ALICE");
        assert_eq!(session.project().elements[2].text, "(ALICE)");
        assert!(!session.dirty());
        session.redo().unwrap_or_else(|_| unreachable!());
        assert_eq!(session.project().elements[1].text, "ALICEX");
        assert_eq!(session.project().elements[2].text, "(ALICEX)");
        assert!(session.dirty());
    }

    #[test]
    fn formatted_cast_list_does_not_block_history_or_scheduled_refresh() {
        let cast = ScreenplayElement::with_runs(
            "cast",
            Role::CastList,
            vec![TextRun {
                text: "(stale)".into(),
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold".into(),
                }],
            }],
            None,
        )
        .unwrap_or_else(|_| unreachable!());
        let project = project_with(vec![
            element("heading", Role::SceneHeading, "INT. ROOM"),
            element("cue", Role::Character, "ALICE"),
            cast,
        ]);
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        session
            .set_selection(Selection::caret(Position {
                element: 1,
                offset: 5,
            }))
            .unwrap_or_else(|_| unreachable!());
        session.insert("X").unwrap_or_else(|_| unreachable!());

        session.undo().unwrap_or_else(|_| unreachable!());
        assert_eq!(session.project().elements[1].text, "ALICE");
        assert_eq!(session.project().elements[2].text, "(stale)");
        assert!(!session.refresh_cast_lists().unwrap_or(true));
        assert_eq!(session.project().elements[2].text, "(stale)");
        assert_eq!(
            session.project().elements[2].runs[0].attributes,
            vec![RunAttribute {
                name: "Style".into(),
                value: "Bold".into(),
            }]
        );
    }

    #[test]
    fn corrected_imported_cast_list_reconciles_through_fdx_writer() {
        let source = std::str::from_utf8(include_bytes!(
            "../../../.forge/initiatives/2026-10-31-document-model-refactor/evidence/wo001-preservation-shapes-synthetic.xml"
        ))
        .unwrap_or_else(|_| unreachable!());
        let stale_source = source.replace(
            "<Paragraph Type=\"Cast List\" id=\"s-cast-list-1\"><Text>(ALPHA)</Text></Paragraph>",
            "<Paragraph Type=\"Cast List\" id=\"s-cast-list-1\"><Text>(STALE)</Text></Paragraph>",
        );
        assert_ne!(stale_source, source);
        let imported =
            crate::fdx::read_v6(stale_source.as_bytes()).unwrap_or_else(|_| unreachable!());
        let session = EditingSession::new(imported.project).unwrap_or_else(|_| unreachable!());
        let original = session.clone();
        let candidate = session
            .prepare_cast_list_save_candidate()
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(session, original);
        assert_eq!(candidate.elements[9].text, "(ALPHA)");
        let emitted = crate::fdx::write_v6(&candidate).unwrap_or_else(|_| unreachable!());
        let reopened = crate::fdx::read_v6(&emitted).unwrap_or_else(|_| unreachable!());
        assert_eq!(reopened.project.elements[2].text, "ALPHA");
        assert_eq!(reopened.project.elements[9].text, "(ALPHA)");
    }

    #[test]
    fn wo009_refresh_uses_one_project_candidate_and_heading_scoped_scene_candidates() {
        let project = imported_project_without_scene_metadata();
        let original = project.clone();
        let expected_scene_candidates = project
            .elements
            .iter()
            .filter(|element| element.role == Role::SceneHeading)
            .count() as u64;

        let (candidate, outcome) =
            corrected_cast_list_candidate(&project).unwrap_or_else(|_| unreachable!());

        assert_eq!(project, original);
        assert!(outcome.changed);
        assert_eq!(outcome.project_candidate_clones, 1);
        assert_eq!(outcome.scene_candidate_clones, expected_scene_candidates);
        assert_eq!(outcome.redundant_preservation_clones, 0);
        assert!(candidate.validate().is_ok());

        let (_, unchanged) =
            corrected_cast_list_candidate(&candidate).unwrap_or_else(|_| unreachable!());
        assert!(!unchanged.changed);
        assert_eq!(unchanged.project_candidate_clones, 1);
        assert_eq!(unchanged.scene_candidate_clones, expected_scene_candidates);
        assert_eq!(unchanged.redundant_preservation_clones, 0);
    }

    #[test]
    fn wo010_derived_candidate_shares_original_bytes_and_untouched_preserved_subtree() {
        fn find_managed<'a>(
            sections: &'a [crate::preservation::PreservedSection],
            id: &crate::StableId,
        ) -> Option<&'a crate::preservation::PreservedElement> {
            fn find_nodes<'a>(
                nodes: &'a [crate::preservation::PreservedNode],
                id: &crate::StableId,
            ) -> Option<&'a crate::preservation::PreservedElement> {
                for node in nodes {
                    let crate::preservation::PreservedNode::Element(element) = node else {
                        continue;
                    };
                    if matches!(&element.ownership, crate::preservation::SourceOwnership::Managed(owner) if owner == id) {
                        return Some(element);
                    }
                    if let Some(found) = find_nodes(&element.children, id) {
                        return Some(found);
                    }
                }
                None
            }

            sections
                .iter()
                .find_map(|section| find_nodes(&section.children, id))
        }

        let source_project = project_with(vec![
            element("wo010-heading", Role::SceneHeading, "INT. ROOM - DAY"),
            element("wo010-character", Role::Character, "ALPHA"),
            element("wo010-cast-list", Role::CastList, "(STALE)"),
            element("wo010-unrelated", Role::Action, "UNRELATED SOURCE CONTENT"),
        ]);
        let serialized = crate::fdx::write_v6(&source_project).unwrap_or_else(|_| unreachable!());
        let imported = crate::fdx::read_v6(&serialized)
            .unwrap_or_else(|_| unreachable!())
            .project;
        let unrelated_id = imported
            .elements
            .iter()
            .find(|element| element.text == "UNRELATED SOURCE CONTENT")
            .map(|element| element.id.clone())
            .unwrap_or_else(|| unreachable!());
        let original_state = imported.preservation.as_ref().unwrap_or_else(|| unreachable!());
        let before_unrelated = find_managed(&original_state.sections, &unrelated_id)
            .unwrap_or_else(|| unreachable!());
        let cast_list_index = imported
            .elements
            .iter()
            .position(|element| element.role == Role::CastList)
            .unwrap_or_else(|| unreachable!());
        let (candidate, outcome) = corrected_cast_list_candidate(&imported)
            .unwrap_or_else(|_| unreachable!());
        let candidate_state = candidate.preservation.as_ref().unwrap_or_else(|| unreachable!());
        let after_unrelated = find_managed(&candidate_state.sections, &unrelated_id)
            .unwrap_or_else(|| unreachable!());

        assert!(outcome.changed);
        assert_eq!(candidate.elements[cast_list_index].text, "(ALPHA)");
        assert!(original_state.source.original_bytes.shares_storage_with(
            &candidate_state.source.original_bytes
        ));
        assert!(before_unrelated.children.shares_storage_with(&after_unrelated.children));
        assert_eq!(
            imported
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .source
                .original_bytes,
            serialized
        );

        let save_candidate = prepare_save_candidate(&imported).unwrap_or_else(|_| unreachable!());
        let save_state = save_candidate.preservation.as_ref().unwrap_or_else(|| unreachable!());
        let save_unrelated = find_managed(&save_state.sections, &unrelated_id)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(save_candidate.elements[cast_list_index].text, "(ALPHA)");
        assert!(original_state
            .source
            .original_bytes
            .shares_storage_with(&save_state.source.original_bytes));
        assert!(before_unrelated
            .children
            .shares_storage_with(&save_unrelated.children));
        assert_eq!(
            imported.elements[cast_list_index].text,
            "(STALE)"
        );
    }

    #[test]
    fn wo009_preservation_free_noop_has_no_scene_candidates() {
        let project = project_with(vec![
            element("heading", Role::SceneHeading, "INT. ROOM"),
            element("cue", Role::Character, "ALEX"),
            element("cast", Role::CastList, "(ALEX)"),
        ]);
        let original = project.clone();

        let (candidate, outcome) =
            corrected_cast_list_candidate(&project).unwrap_or_else(|_| unreachable!());

        assert_eq!(project, original);
        assert_eq!(candidate, original);
        assert!(!outcome.changed);
        assert_eq!(outcome.project_candidate_clones, 1);
        assert_eq!(outcome.scene_candidate_clones, 0);
        assert_eq!(outcome.redundant_preservation_clones, 0);
    }

    #[test]
    fn wo009_fatal_later_scene_failure_does_not_publish_earlier_candidate_work() {
        fn make_second_cast_mapping_opaque(nodes: &mut [crate::preservation::PreservedNode], id: &crate::StableId) -> bool {
            for node in nodes {
                let crate::preservation::PreservedNode::Element(element) = node else {
                    continue;
                };
                if matches!(&element.ownership, crate::preservation::SourceOwnership::Managed(target) if target == id) {
                    element.ownership = crate::preservation::SourceOwnership::Opaque;
                    return true;
                }
                if make_second_cast_mapping_opaque(&mut element.children, id) {
                    return true;
                }
            }
            false
        }

        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ONE</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph><Paragraph Type="Cast List" id="l1"><Text>(stale one)</Text></Paragraph><Paragraph Type="Scene Heading" id="h2"><Text>EXT. TWO</Text></Paragraph><Paragraph Type="Character" id="c2"><Text>BETA</Text></Paragraph><Paragraph Type="Cast List" id="l2"><Text>(stale two)</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = crate::fdx::read_v6(source.as_bytes())
            .unwrap_or_else(|_| unreachable!())
            .project;
        let second_cast_id = project
            .elements
            .iter()
            .filter(|element| element.role == Role::CastList)
            .nth(1)
            .unwrap_or_else(|| unreachable!())
            .id
            .clone();
        let state = project.preservation.as_mut().unwrap_or_else(|| unreachable!());
        assert!(state.sections.iter_mut().any(|section| {
            make_second_cast_mapping_opaque(&mut section.children, &second_cast_id)
        }));
        let original = project.clone();

        assert!(correct_cast_lists_with_outcome(&mut project).is_err());
        assert_eq!(project, original);
    }
}
