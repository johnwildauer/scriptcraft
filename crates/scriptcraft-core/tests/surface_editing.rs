use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{PreservedNode, SourceOwnership};
use scriptcraft_core::{ParagraphAlignment, Project, RunAttribute, RunStyle, RunStyleFlags, StableId, TextRun};

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("unexpected error: {error:?}"))
}

fn source_backed_project() -> Project {
    let source = valid(write_v6(&valid(Project::new())));
    valid(read_v6(&source)).project
}

fn mutate_managed_paragraph(
    nodes: &mut [PreservedNode],
    id: &StableId,
    action: &mut impl FnMut(&mut scriptcraft_core::preservation::PreservedElement),
) -> bool {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
            action(element);
            return true;
        }
        if mutate_managed_paragraph(&mut element.children, id, action) {
            return true;
        }
    }
    false
}

fn insert_comment_after_managed(
    nodes: &mut scriptcraft_core::preservation::SharedVec<PreservedNode>,
    id: &StableId,
) -> bool {
    for index in 0..nodes.len() {
        let is_target = matches!(nodes.get(index), Some(PreservedNode::Element(element))
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id));
        if is_target {
            nodes.insert(index + 1, PreservedNode::Comment("title-sibling-attachment".into()));
            return true;
        }
        let nested_match = match nodes.get_mut(index) {
            Some(PreservedNode::Element(element)) => insert_comment_after_managed(&mut element.children, id),
            _ => false,
        };
        if nested_match {
            return true;
        }
    }
    false
}

#[test]
fn body_and_title_edits_share_one_undo_redo_timeline() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert_title("Title"));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert("Body"));

    assert_eq!(session.project().title_page[0].text, "Title");
    assert_eq!(session.project().elements[0].text, "Body");
    valid(session.undo());
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(session.project().title_page[0].text, "Title");
    valid(session.undo());
    assert_eq!(session.project().title_page[0].text, "");
    assert!(!session.dirty());

    valid(session.redo());
    valid(session.redo());
    assert_eq!(session.project().elements[0].text, "Body");
    assert_eq!(session.project().title_page[0].text, "Title");
}

#[test]
fn title_split_backspace_join_and_unicode_delete_are_surface_local() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert_title("αβ"));
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 2 })));
    valid(session.split_title());
    assert_eq!(session.project().title_page[0].text, "α");
    assert_eq!(session.project().title_page[1].text, "β");

    valid(session.backspace_title());
    assert_eq!(session.project().title_page.len(), 54);
    assert_eq!(session.project().title_page[0].text, "αβ");

    valid(session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: 2 },
    }));
    valid(session.delete_title());
    assert_eq!(session.project().title_page[0].text, "β");
}

#[test]
fn title_typing_style_is_transient_and_formatting_uses_shared_history() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    let initial_revision = session.revision();
    valid(session.set_title_selection(Selection::caret(Position { element: 17, offset: 0 })));
    valid(session.toggle_title_run_style(RunStyle::Bold));
    assert!(!session.dirty());
    assert_eq!(session.revision(), initial_revision);
    assert!(session.title_typing_style().bold);

    valid(session.insert_title("X"));
    assert!(session.dirty());
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 17, offset: 0 },
        focus: Position { element: 17, offset: 1 },
    }));
    valid(session.toggle_title_run_style(RunStyle::Strikethrough));
    valid(session.set_title_alignment(ParagraphAlignment::Right));
    assert_eq!(session.project().title_page[17].text, "XScript Title");
    assert_eq!(session.project().title_page[17].formatting.alignment, Some(scriptcraft_core::TitlePageAlignment::Right));

    valid(session.undo());
    assert_eq!(session.project().title_page[17].formatting.alignment, Some(scriptcraft_core::TitlePageAlignment::Center));
    valid(session.undo());
    assert_eq!(session.project().title_page[17].text, "XScript Title");
    assert!(!session.project().title_page[17].runs[0].attributes.iter().any(|attribute| {
        attribute.name == "Style" && attribute.value.contains("Strikethrough")
    }));
    valid(session.undo());
    assert_eq!(session.project().title_page[17].text, "Script Title");
}

#[test]
fn body_formatting_changes_semantics_without_changing_the_screenplay_role() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("Action"));
    let role = session.project().elements[0].role;
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: 6 },
    }));
    valid(session.toggle_body_run_style(RunStyle::Underline));
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    assert_eq!(session.project().elements[0].role, role);
    assert_eq!(session.project().direct_paragraph_overrides[&session.project().elements[0].id].paragraph.alignment, Some(ParagraphAlignment::Right));
}

#[test]
fn plain_text_is_one_undoable_body_edit_and_preserves_alignment_and_canonical_text() {
    let mut project = valid(Project::new());
    project.elements[0].text = "Body".to_owned();
    project.elements[0].runs = vec![TextRun {
        text: "Body".to_owned(),
        attributes: vec![RunAttribute {
            name: "Style".to_owned(),
            value: "Bold+Italic+Underline+AllCaps+Strikethrough+VendorToken".to_owned(),
        }],
    }];
    let mut session = valid(EditingSession::new(project));
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: 4 },
    }));
    let before_clear = session.project().clone();

    valid(session.clear_body_run_styles());

    assert_eq!(session.project().elements[0].text, "Body");
    assert_eq!(session.project().elements[0].runs[0].attributes[0].value, "VendorToken");
    assert_eq!(
        session.project().direct_paragraph_overrides[&session.project().elements[0].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Right),
    );
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &before_clear);
}

#[test]
fn plain_text_at_a_collapsed_caret_resets_typing_style_without_document_history() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.toggle_body_run_style(RunStyle::Bold));
    valid(session.toggle_body_run_style(RunStyle::AllCaps));
    assert!(session.body_typing_style().bold && session.body_typing_style().all_caps);
    let revision = session.revision();

    valid(session.clear_body_run_styles());

    assert_eq!(session.body_typing_style(), RunStyleFlags::default());
    assert_eq!(session.revision(), revision);
    assert!(!session.can_undo());
    valid(session.insert("plain"));
    assert!(session.project().elements[0].runs.iter().all(|run| {
        !run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && !attribute.value.is_empty()
        })
    }));
}

#[test]
fn source_backed_title_text_and_run_styles_publish_and_undo_together() {
    let source = valid(write_v6(&valid(Project::new())));
    let project = valid(read_v6(&source)).project;
    let mut session = valid(EditingSession::new(project));
    valid(session.set_title_selection(Selection::caret(Position { element: 17, offset: 0 })));
    valid(session.insert_title("X"));
    assert_eq!(session.project().title_page[17].text, "XScript Title");
    valid(session.undo());
    assert_eq!(session.project().title_page[17].text, "Script Title");

    valid(session.set_title_selection(Selection {
        anchor: Position { element: 17, offset: 0 },
        focus: Position { element: 17, offset: 6 },
    }));
    valid(session.toggle_title_run_style(RunStyle::Bold));
    assert!(session.project().title_page[17].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value.split('+').any(|token| token == "Bold")
        })
    }));
    valid(session.undo());
    assert_eq!(session.project().title_page[17].text, "Script Title");
}

#[test]
fn source_backed_title_split_join_undo_redo_and_re_read_keep_source_in_sync() {
    let mut project = source_backed_project();
    let target = project.title_page[17].id.clone();
    let preservation = project.preservation.as_mut().expect("imported FDX preservation");
    let mut inserted_comment = false;
    for section in &mut preservation.sections {
        inserted_comment |= insert_comment_after_managed(&mut section.children, &target);
    }
    assert!(inserted_comment);
    preservation.rebuild_indexes();
    let mut session = valid(EditingSession::new(project));
    let original_id = session.project().title_page[17].id.clone();
    let original_foreign_id = session.project().title_page[17]
        .source_provenance.as_ref().and_then(|source| source.foreign_id.clone());
    valid(session.set_title_selection(Selection::caret(Position { element: 17, offset: 6 })));
    let before_split = session.project().clone();
    valid(session.split_title());

    assert_eq!(session.project().title_page.len(), 55);
    assert_eq!(session.project().title_page[17].text, "Script");
    assert_eq!(session.project().title_page[18].text, " Title");
    assert_eq!(session.project().title_page[17].id, original_id);
    let split_foreign_id = session.project().title_page[18]
        .source_provenance.as_ref().and_then(|source| source.foreign_id.as_deref());
    assert!(split_foreign_id.is_some());
    assert_ne!(split_foreign_id, original_foreign_id.as_deref());
    let encoded_split = valid(write_v6(session.project()));
    assert!(String::from_utf8_lossy(&encoded_split).contains("title-sibling-attachment"));
    let reread_split = valid(read_v6(&encoded_split));
    assert_eq!(reread_split.project.title_page[17].text, "Script");
    assert_eq!(reread_split.project.title_page[18].text, " Title");

    valid(session.undo());
    assert_eq!(session.project(), &before_split);
    valid(session.redo());
    assert_eq!(session.project().title_page.len(), 55);
    valid(session.set_title_selection(Selection::caret(Position { element: 18, offset: 0 })));
    valid(session.join_title_with_previous());
    assert_eq!(session.project().title_page.len(), 54);
    assert_eq!(session.project().title_page[17].text, "Script Title");
    assert_eq!(session.project().title_page[17].id, original_id);
    let encoded_join = valid(write_v6(session.project()));
    let reread_join = valid(read_v6(&encoded_join));
    assert_eq!(reread_join.project.title_page[17].text, "Script Title");
    assert_eq!(reread_join.project.title_page.len(), 54);
}

#[test]
fn source_backed_cross_paragraph_deletion_preserves_runs_and_history() {
    let mut session = valid(EditingSession::new(source_backed_project()));
    valid(session.set_title_selection(Selection::caret(Position { element: 18, offset: 0 })));
    valid(session.insert_title("Finish"));
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 17, offset: 6 },
        focus: Position { element: 18, offset: 3 },
    }));
    let left_id = session.project().title_page[17].id.clone();
    valid(session.delete_title());

    assert_eq!(session.project().title_page.len(), 54);
    assert_eq!(session.project().title_page[17].text, "Scriptish");
    assert_eq!(session.project().title_page[17].id, left_id);
    assert_eq!(session.title_selection(), Selection::caret(Position { element: 17, offset: 6 }));
    let round_trip = valid(read_v6(&valid(write_v6(session.project()))));
    assert_eq!(round_trip.project.title_page[17].text, "Scriptish");
    valid(session.undo());
    assert_eq!(session.project().title_page[17].text, "Script Title");
    assert_eq!(session.project().title_page[18].text, "Finish");
    valid(session.redo());
    assert_eq!(session.project().title_page[17].text, "Scriptish");
    assert_eq!(session.project().title_page.len(), 54);
}

#[test]
fn source_backed_structural_edits_reject_opaque_attachments_transactionally() {
    let mut project = source_backed_project();
    let target = project.title_page[17].id.clone();
    let preservation = project.preservation.as_mut().expect("imported FDX preservation");
    let mut attached = false;
    for section in &mut preservation.sections {
        attached |= mutate_managed_paragraph(&mut section.children, &target, &mut |paragraph| {
            paragraph.children.push(PreservedNode::Comment("authored attachment".into()));
        });
    }
    assert!(attached);
    preservation.rebuild_indexes();

    let mut session = valid(EditingSession::new(project));
    valid(session.set_title_selection(Selection::caret(Position { element: 17, offset: 6 })));
    let before = session.project().clone();
    let selection = session.title_selection();
    let revision = session.revision();
    assert!(session.split_title().is_err());
    assert_eq!(session.project(), &before);
    assert_eq!(session.title_selection(), selection);
    assert_eq!(session.revision(), revision);
    assert!(!session.can_undo());
}

#[test]
fn source_backed_join_rejects_repeated_raw_paragraph_ids_transactionally() {
    let mut project = source_backed_project();
    let repeated_id = project.title_page[17].source_provenance.as_ref()
        .and_then(|source| source.foreign_id.clone()).expect("source paragraph ID");
    let target = project.title_page[18].id.clone();
    let preservation = project.preservation.as_mut().expect("imported FDX preservation");
    let mut changed = false;
    for section in &mut preservation.sections {
        changed |= mutate_managed_paragraph(&mut section.children, &target, &mut |paragraph| {
            let id_attribute = paragraph.attributes.iter_mut().find(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
            }).expect("source paragraph ID attribute");
            id_attribute.value = repeated_id.clone();
        });
    }
    assert!(changed);
    project.title_page[18].source_provenance.as_mut()
        .expect("source paragraph provenance").foreign_id = Some(repeated_id);
    preservation.rebuild_indexes();

    let mut session = valid(EditingSession::new(project));
    valid(session.set_title_selection(Selection::caret(Position { element: 18, offset: 0 })));
    let before = session.project().clone();
    let selection = session.title_selection();
    let revision = session.revision();
    assert!(session.join_title_with_previous().is_err());
    assert_eq!(session.project(), &before);
    assert_eq!(session.title_selection(), selection);
    assert_eq!(session.revision(), revision);
    assert!(!session.can_undo());
}
