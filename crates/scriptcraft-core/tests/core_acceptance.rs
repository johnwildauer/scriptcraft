#![forbid(unsafe_code)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::layout::ScreenplayLayout;
use scriptcraft_core::{Project, Role};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("acceptance path uses valid bounded input"))
}

#[test]
fn headless_edit_layout_and_exports_form_one_non_mutating_journey() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("A quiet room — café."));
    let before = session.clone();

    let first = valid(ScreenplayLayout::build(session.project()));
    let second = valid(ScreenplayLayout::build(session.project()));
    assert_eq!(first, second);
    assert!(!first.pages.is_empty());
    let pdf = valid(export_pdf(session.project()));
    let text = valid(export_text(session.project()));
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(text.text.contains("A QUIET ROOM"));
    assert!(text.loss.page_layout_lost);
    assert_eq!(session, before);
    assert!(session.dirty());
}

#[test]
fn native_fdx_edit_save_reopen_preserves_semantics_opaque_source_and_ids() {
    let source = br#"<?xml version="1.0" encoding="UTF-8" standalone="no" ?>
<FinalDraft DocumentType="Script" Version="6"><FutureData marker="keep"><Payload>opaque-source</Payload></FutureData><Content><Paragraph Type="Scene Heading" id="scene-id"><Text>INT. ROOM - DAY</Text></Paragraph><Paragraph Type="Action" id="action-id"><Text>Original action</Text></Paragraph></Content></FinalDraft>"#;
    let imported = valid(read_v6(source));
    let mut session = valid(EditingSession::new(imported.project));
    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: "Original action".len(),
    })));
    valid(session.insert(" — déjà vu"));

    let saved = valid(write_v6(session.project()));
    let reopened = valid(read_v6(&saved));
    assert_eq!(reopened.project.elements.len(), 2);
    assert_eq!(reopened.project.elements[0].role, Role::SceneHeading);
    assert_eq!(reopened.project.elements[1].text, "Original action — déjà vu");
    let xml = String::from_utf8(saved).unwrap_or_else(|_| unreachable!("FDX output is UTF-8"));
    for retained in ["marker=\"keep\"", "opaque-source", "id=\"scene-id\"", "id=\"action-id\""] {
        assert!(xml.contains(retained));
    }
}

#[test]
fn dual_dialogue_fixture_survives_edit_save_reopen_as_flat_semantics() {
    let imported = valid(read_v6(include_bytes!("fixtures/dual_dialogue_basic.fdx")));
    let original_ids = imported
        .project
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let mut session = valid(EditingSession::new(imported.project));
    let member = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Dialogue)
        .unwrap_or_else(|| unreachable!("fixture includes dialogue"));
    valid(session.set_selection(Selection::caret(Position {
        element: member,
        offset: session.project().elements[member].text.len(),
    })));
    valid(session.insert(" updated"));

    let saved = valid(write_v6(session.project()));
    let reopened = valid(read_v6(&saved));
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 1);
    assert_eq!(reopened.project.elements.len(), original_ids.len());
    assert_eq!(reopened.project.elements[member].id, original_ids[member]);
    assert!(reopened.project.elements[member].text.ends_with(" updated"));
}
