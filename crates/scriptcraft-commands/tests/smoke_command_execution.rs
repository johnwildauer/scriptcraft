#![forbid(unsafe_code)]

use scriptcraft_commands::{
    execute_editor_command, Command, CommandOutcome, CommandRejection, EditorSurface,
    ElementTypeValue, Position, Selection, SelectionError,
};
use scriptcraft_model::ElementTypeKey;
use scriptcraft_session::editing::{
    EditingSession, Position as CorePosition, Selection as CoreSelection,
};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("command smoke uses valid bounded input"))
}

#[test]
fn open_element_type_values_preserve_exact_labels() {
    let value = valid(ElementTypeValue::new("Lyrics"));
    let encoded = serde_json::to_string(&value)
        .unwrap_or_else(|_| unreachable!("open element value serializes"));
    assert_eq!(encoded, "\"Lyrics\"");
    let decoded: ElementTypeValue = serde_json::from_str(&encoded)
        .unwrap_or_else(|_| unreachable!("open element value deserializes"));
    assert_eq!(decoded.as_str(), "Lyrics");
}

#[test]
fn command_execution_public_api_smoke() {
    const INSERTED_TEXT: &str = "Command café";

    let mut project = valid(scriptcraft_fdx::new_project());
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut lyrics = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("default project defines Action"));
    lyrics.key = valid(ElementTypeKey::new("Lyrics"));
    definitions.push(lyrics);
    valid(project.replace_element_definitions(definitions));
    let mut session = valid(EditingSession::new(project));
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert {
                text: INSERTED_TEXT.to_owned(),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(session.project().elements[0].text, INSERTED_TEXT);
    assert_eq!(
        session.selection(),
        CoreSelection::caret(CorePosition {
            element: 0,
            offset: INSERTED_TEXT.len(),
        })
    );
    assert!(session.dirty());

    let before_rejection = session.clone();
    // The final é occupies two bytes; this points at its continuation byte.
    let invalid_offset = valid(u32::try_from(INSERTED_TEXT.len() - "é".len() + 1));
    let invalid_position = Position {
        element: 0,
        offset: invalid_offset,
    };
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorSelectionSet {
                selection: Selection {
                    anchor: invalid_position,
                    focus: invalid_position,
                },
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Rejected(CommandRejection::InvalidSelection(
            SelectionError::InvalidUtf8Boundary,
        ))
    );

    assert_eq!(session, before_rejection);
    assert_eq!(
        session.project().elements[0].text,
        before_rejection.project().elements[0].text
    );
    assert_eq!(session.selection(), before_rejection.selection());
    assert_eq!(session.revision(), before_rejection.revision());
    assert_eq!(session.dirty(), before_rejection.dirty());
    assert_eq!(session.can_undo(), before_rejection.can_undo());
    assert_eq!(session.can_redo(), before_rejection.can_redo());

    let arbitrary_type = valid(ElementTypeValue::new("Lyrics"));
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorElementChange {
                element_type: arbitrary_type,
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        session.project().elements[0].element_type().as_str(),
        "Lyrics"
    );
}
