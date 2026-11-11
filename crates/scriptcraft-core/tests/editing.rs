#![forbid(unsafe_code)]

use scriptcraft_core::editing::{EditingSession, ParagraphKeyTarget, Position, Selection};
use scriptcraft_core::{ParagraphAlignment, Project, Role, RunStyle, ScreenplayElement};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("editing case uses valid input"))
}

fn two_sided_project() -> Project {
    let mut project = valid(Project::new());
    project.elements = [
        ("left-cue", Role::Character, "LEFT"),
        ("left-line", Role::Dialogue, "café"),
        ("right-cue", Role::Character, "RIGHT"),
        ("right-line", Role::Dialogue, "你好"),
    ]
    .into_iter()
    .map(|(id, role, text)| valid(ScreenplayElement::new(id, role, text)))
    .collect();
    project
}

#[test]
fn body_and_title_history_share_one_timeline_and_restore_dirty_state() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.set_title_selection(Selection::caret(Position { element: 17, offset: 0 })));
    valid(session.insert_title("Feature"));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert("INT. CAFE — DAY"));
    assert!(session.dirty());

    valid(session.undo());
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(session.project().title_page[17].text, "FeatureScript Title");
    valid(session.undo());
    assert_eq!(session.project().title_page[17].text, "Script Title");
    assert!(!session.dirty());
    valid(session.redo());
    valid(session.redo());
    assert_eq!(session.project().elements[0].text, "INT. CAFE — DAY");
}

#[test]
fn utf8_editing_formatting_and_navigation_keep_byte_offsets_and_roles() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("café"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: "café".len(),
    })));
    let keys = session.paragraph_key_targets();
    assert!(matches!(keys.enter, ParagraphKeyTarget::Target(_)));
    let before_invalid = session.clone();
    assert!(session
        .set_selection(Selection::caret(Position { element: 0, offset: 4 }))
        .is_err());
    assert_eq!(session, before_invalid);

    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: "café".len() },
    }));
    let role = session.project().elements[0].role;
    valid(session.toggle_body_run_style(RunStyle::Bold));
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    assert_eq!(session.project().elements[0].role, role);
    assert_eq!(
        session.project().direct_paragraph_overrides[&session.project().elements[0].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Right)
    );
    assert!(session.project().elements[0].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value.contains("Bold")
        })
    }));
}

#[test]
fn paragraph_key_projection_is_read_only_and_matches_character_boundary_rules() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    let before = session.clone();
    let projected = session.paragraph_key_targets();
    assert_eq!(session.paragraph_key_targets(), projected);
    assert_eq!(session, before);
    assert_eq!(projected.tab, ParagraphKeyTarget::NoOp);
    assert!(matches!(projected.enter, ParagraphKeyTarget::Target(_)));

    valid(session.enter());
    assert_ne!(session, before);
    assert_eq!(session.project().elements.len(), 5);
}

#[test]
fn dual_dialogue_toggle_is_atomic_reversible_and_ineligible_toggle_is_a_noop() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    let members = session.project().elements.clone();
    valid(session.toggle_dual_dialogue());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(session.project().elements, members);
    valid(session.undo());
    assert!(session.project().dual_dialogue_groups.is_empty());
    valid(session.redo());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);

    let mut ordinary = valid(EditingSession::new(valid(Project::new())));
    let before = ordinary.clone();
    valid(ordinary.toggle_dual_dialogue());
    assert_eq!(ordinary, before);
}

#[test]
fn group_unwrap_preserves_visible_members_and_invalid_selection_rolls_back() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.toggle_dual_dialogue());
    let before_invalid_selection = session.clone();
    assert!(session
        .set_selection(Selection::caret(Position { element: 1, offset: usize::MAX }))
        .is_err());
    assert_eq!(session, before_invalid_selection);

    valid(session.set_selection(Selection::caret(Position { element: 3, offset: 0 })));
    valid(session.toggle_dual_dialogue());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert_eq!(
        session.project().elements.iter().map(|item| item.text.as_str()).collect::<Vec<_>>(),
        ["LEFT", "café", "RIGHT", "你好"]
    );
    assert!(session.can_undo());
}
