//! Bounded scene cast-list derivation and detached save candidates.

use crate::preservation::PreservationError;
use crate::scene_metadata::SceneMetadataError;
use crate::{ModelError, Project, Role, MAX_TEXT_BYTES};
use std::collections::BTreeMap;
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
    let original = project.clone();
    let mut candidate = project.clone();
    let reconciliation = crate::scene_metadata::reconcile_project_with_outcome(&mut candidate)
        .map_err(CastListError::SceneMetadata)?;
    let mut updates = Vec::new();
    let mut scene_start: Option<usize> = None;

    for index in 0..=candidate.elements.len() {
        let is_heading = candidate
            .elements
            .get(index)
            .is_some_and(|element| element.role == Role::SceneHeading);
        if is_heading || index == candidate.elements.len() {
            if let Some(start) = scene_start {
                let scene = &candidate.elements[start + 1..index];
                if scene.iter().any(|element| element.role == Role::CastList) {
                    let heading_id = &candidate.elements[start].id;
                    if reconciliation
                        .deferred_scenes
                        .contains(heading_id.as_str())
                    {
                        scene_start = is_heading.then_some(index);
                        continue;
                    }
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
                            updates.push((element_index, cast.clone()));
                        }
                    }
                }
            }
            scene_start = is_heading.then_some(index);
        }
    }

    for (element_index, replacement) in updates {
        match replace_cast_list_text(&mut candidate, element_index, &replacement) {
            Ok(()) | Err(CastListError::UnsupportedFormattedCastList) => {}
            Err(error) => return Err(error),
        }
    }
    candidate.validate()?;
    let changed = candidate != original;
    if changed {
        *project = candidate;
    }
    Ok(changed)
}

/// Returns a corrected clone suitable for Save or Save As preflight.
/// The caller owns publication and must do so only after successful storage.
pub fn prepare_save_candidate(project: &Project) -> Result<Project, CastListError> {
    let mut candidate = project.clone();
    correct_cast_lists(&mut candidate)?;
    Ok(candidate)
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
}
