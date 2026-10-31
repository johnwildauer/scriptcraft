#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::persistence::deserialize;
use scriptcraft_core::{Project, Role};

fn project() -> Project {
    Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"))
}

#[test]
fn representative_headless_writing_loop_survives_every_boundary() {
    let mut session = EditingSession::new(project())
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));

    // Create and edit a scene heading, then use the explicit scene-heading → action transition.
    session
        .change_role(Role::SceneHeading)
        .unwrap_or_else(|_| unreachable!("role change must be valid"));
    session
        .insert("INT. CAFÉ — NIGHT")
        .unwrap_or_else(|_| unreachable!("fixture insert must be valid"));
    session
        .enter()
        .unwrap_or_else(|_| unreachable!("scene transition must be valid"));
    assert_eq!(session.project().elements[1].role, Role::Action);

    // Exercise character → dialogue and a Unicode edit through the same session boundary.
    session
        .change_role(Role::Character)
        .unwrap_or_else(|_| unreachable!("role change must be valid"));
    session
        .insert("MARA")
        .unwrap_or_else(|_| unreachable!("fixture insert must be valid"));
    session
        .enter()
        .unwrap_or_else(|_| unreachable!("character transition must be valid"));
    assert_eq!(session.project().elements[2].role, Role::Dialogue);
    session
        .insert("こんにちは、世界 🌙")
        .unwrap_or_else(|_| unreachable!("Unicode insert must be valid"));
    assert!(session.dirty());

    let edited = session.project().clone();
    session
        .undo()
        .unwrap_or_else(|_| unreachable!("undo must restore Unicode edit"));
    assert_ne!(session.project(), &edited);
    session
        .redo()
        .unwrap_or_else(|_| unreachable!("redo must restore Unicode edit"));
    assert_eq!(session.project(), &edited);

    // Pagination is structural and repeatable, independent of any desktop window.
    let first_layout = scriptcraft_core::layout::ScreenplayLayout::build(session.project())
        .unwrap_or_else(|_| unreachable!("valid project must lay out"));
    let second_layout = scriptcraft_core::layout::ScreenplayLayout::build(session.project())
        .unwrap_or_else(|_| unreachable!("valid project must lay out"));
    assert_eq!(first_layout, second_layout);
    assert!(!first_layout.pages.is_empty());

    // Exports consume the project but must not falsely clean or otherwise mutate the session.
    let before_exports = session.clone();
    let pdf = export_pdf(session.project()).unwrap_or_else(|_| unreachable!("PDF must export"));
    let text = export_text(session.project()).unwrap_or_else(|_| unreachable!("TXT must export"));
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(text.text.contains("LOSSY EXPORT"));
    assert_eq!(session, before_exports);
    assert!(session.dirty(), "exports must not clear dirty state");
}

#[test]
fn bounded_hostile_sequences_return_errors_without_state_corruption() {
    let mut session = EditingSession::new(project())
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    session
        .change_role(Role::Dialogue)
        .unwrap_or_else(|_| unreachable!("role change must be valid"));
    session
        .insert("é🙂")
        .unwrap_or_else(|_| unreachable!("fixture insert must be valid"));

    for step in 0..64 {
        let before = session.clone();
        let result = match step % 4 {
            0 => session.set_selection(Selection::caret(Position {
                element: 0,
                offset: 1,
            })),
            1 => session.set_selection(Selection::caret(Position {
                element: usize::MAX,
                offset: 0,
            })),
            2 => session.insert(&"x".repeat(scriptcraft_core::MAX_TEXT_BYTES + 1)),
            _ => {
                assert!(deserialize(b"{not-json").is_err());
                continue;
            }
        };
        assert!(result.is_err());
        assert_eq!(session, before);
    }
    assert_eq!(session.project().elements[0].text, "é🙂");
}
