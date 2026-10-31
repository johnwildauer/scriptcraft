//! Bounded scene cast-list derivation and detached save candidates.

use crate::preservation::PreservationError;
use crate::{ModelError, Project, Role, MAX_TEXT_BYTES};
use std::collections::BTreeMap;
use std::time::Duration;

/// The desktop refresh cadence for derived scene cast lists.
pub const CAST_LIST_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastListError {
    Model(ModelError),
    Preservation(PreservationError),
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
/// Source-backed formatted lists or lists whose source text cannot be safely
/// replaced are rejected atomically rather than dropping style or opaque data.
pub fn correct_cast_lists(project: &mut Project) -> Result<bool, CastListError> {
    let mut updates = Vec::new();
    let mut scene_start = None;

    for index in 0..=project.elements.len() {
        let is_heading = project
            .elements
            .get(index)
            .is_some_and(|element| element.role == Role::SceneHeading);
        if is_heading || index == project.elements.len() {
            if let Some(start) = scene_start {
                let scene = &project.elements[start + 1..index];
                if scene.iter().any(|element| element.role == Role::CastList) {
                    let cast = scene_cast_text(project, start + 1, index)?;
                    for element_index in start + 1..index {
                        if project.elements[element_index].role == Role::CastList
                            && project.elements[element_index].text != cast
                        {
                            updates.push((element_index, cast.clone()));
                        }
                    }
                }
            }
            scene_start = is_heading.then_some(index);
        }
    }

    if updates.is_empty() {
        return Ok(false);
    }

    project.validate()?;
    let mut candidate = project.clone();
    for (element_index, replacement) in updates {
        replace_cast_list_text(&mut candidate, element_index, &replacement)?;
    }
    candidate.validate()?;
    *project = candidate;
    Ok(true)
}

/// Returns a corrected clone suitable for Save or Save As preflight.
/// The caller owns publication and must do so only after successful storage.
pub fn prepare_save_candidate(project: &Project) -> Result<Project, CastListError> {
    let mut candidate = project.clone();
    correct_cast_lists(&mut candidate)?;
    Ok(candidate)
}

fn scene_cast_text(project: &Project, start: usize, end: usize) -> Result<String, CastListError> {
    let mut names = BTreeMap::<String, String>::new();
    let mut names_bytes = 2usize; // Parentheses.
    for element in &project.elements[start..end] {
        if element.role != Role::Character {
            continue;
        }
        let name = element.text.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_lowercase();
        if names.contains_key(&key) {
            continue;
        }
        let separator_bytes = if names.is_empty() { 0 } else { 2 };
        names_bytes = names_bytes
            .checked_add(separator_bytes)
            .and_then(|length| length.checked_add(name.len()))
            .ok_or(CastListError::Model(ModelError::TextTooLong))?;
        if names_bytes > MAX_TEXT_BYTES {
            return Err(CastListError::Model(ModelError::TextTooLong));
        }
        names.insert(key, name.to_owned());
    }

    let mut output = String::with_capacity(names_bytes);
    output.push('(');
    for (index, name) in names.values().enumerate() {
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
    if let Some((first, remaining)) = element.runs.split_first_mut() {
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
        assert_eq!(project.elements[2].text, "(ada, Zoë)");
        assert_eq!(project.elements[7].text, "(ada, Zoë)");
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
    fn refuses_to_discard_cast_list_run_formatting_atomically() {
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

        assert_eq!(
            correct_cast_lists(&mut project),
            Err(CastListError::UnsupportedFormattedCastList)
        );
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
    fn unsupported_derived_mapping_does_not_block_text_undo() {
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
        assert_eq!(
            session.refresh_cast_lists(),
            Err(crate::editing::EditError::CastList(
                CastListError::UnsupportedFormattedCastList
            ))
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
