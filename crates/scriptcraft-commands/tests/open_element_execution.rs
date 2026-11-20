#![forbid(unsafe_code)]

use scriptcraft_commands::{
    execute_editor_command, Command, CommandEnvelope, CommandOrigin, CommandOutcome,
    CommandRejection, EditorSurface, ElementTypeValue, EnvelopeError, SelectionError,
};
use scriptcraft_model::{ElementDefinition, ElementTypeKey, Project, Selection as CoreSelection};
use scriptcraft_session::editing::{
    EditingSession, ElementDefinitionCandidate, Position as CorePosition,
};

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| {
        unreachable!("execution source uses valid bounded inputs: {error:?}")
    })
}

fn definition(label: &str) -> ElementDefinition {
    let project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    let mut definition = project
        .element_definitions()
        .get_by_label("Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("default project defines Action"));
    definition.key = valid(ElementTypeKey::new(label));
    definition
}

fn project_with_arbitrary_types() -> Project {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions.push(definition("Lyrics"));
    definitions.push(definition("lyrics"));
    valid(project.replace_element_definitions(definitions));
    project
}

fn change(label: &str, surface: EditorSurface) -> Command {
    Command::EditorElementChange {
        element_type: valid(ElementTypeValue::new(label)),
        surface,
    }
}

#[test]
fn arbitrary_body_keys_are_project_validated_exact_and_undoable() {
    let mut session = valid(EditingSession::new(project_with_arbitrary_types()));
    for label in ["Lyrics", "lyrics"] {
        let command = change(label, EditorSurface::Body);
        let envelope = CommandEnvelope::new(CommandOrigin::Test, command.clone());
        assert_eq!(envelope.validate_for_project(session.project()), Ok(()));
        assert_eq!(
            execute_editor_command(&mut session, &command),
            CommandOutcome::Changed
        );
        assert_eq!(session.project().elements[0].element_type().as_str(), label);
    }

    let same_key = change("lyrics", EditorSurface::Body);
    let unchanged = session.clone();
    assert_eq!(
        execute_editor_command(&mut session, &same_key),
        CommandOutcome::NoOp
    );
    assert_eq!(session, unchanged);

    assert!(session.undo().is_ok());
    assert_eq!(
        session.project().elements[0].element_type().as_str(),
        "Lyrics"
    );
    assert!(session.redo().is_ok());
    assert_eq!(
        session.project().elements[0].element_type().as_str(),
        "lyrics"
    );

    let bytes = valid(scriptcraft_fdx::write_v6(session.project()));
    let reopened = valid(scriptcraft_fdx::read_v6(&bytes));
    assert_eq!(
        reopened.project.elements[0].element_type().as_str(),
        "lyrics"
    );
    assert!(reopened
        .project
        .element_definitions()
        .get_by_label("lyrics")
        .is_some());
}

#[test]
fn title_page_supports_typed_type_less_and_arbitrary_type_transitions() {
    let mut session = valid(EditingSession::new(project_with_arbitrary_types()));
    let lyrics = valid(ElementTypeKey::new("Lyrics"));
    let lower_lyrics = valid(ElementTypeKey::new("lyrics"));

    assert!(session
        .change_title_element_type(Some(lyrics.clone()))
        .is_ok());
    assert!(session
        .change_title_element_type(Some(lower_lyrics.clone()))
        .is_ok());
    assert_eq!(
        session.project().title_page[0]
            .element_type()
            .as_ref()
            .map(ElementTypeKey::as_str),
        Some("lyrics")
    );

    assert!(session.change_title_element_type(None).is_ok());
    assert_eq!(session.project().title_page[0].element_type(), None);
    let type_less_to_typed = change("Lyrics", EditorSurface::TitlePage);
    assert_eq!(
        execute_editor_command(&mut session, &type_less_to_typed),
        CommandOutcome::Changed
    );
    assert_eq!(
        session.project().title_page[0]
            .element_type()
            .as_ref()
            .map(ElementTypeKey::as_str),
        Some("Lyrics")
    );
    assert_eq!(
        execute_editor_command(&mut session, &type_less_to_typed),
        CommandOutcome::NoOp
    );
}

#[test]
fn every_version_one_legacy_spelling_decodes_to_its_exact_project_key() {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let legacy_roles = [
        ("scene_heading", "Scene Heading"),
        ("action", "Action"),
        ("character", "Character"),
        ("dialogue", "Dialogue"),
        ("parenthetical", "Parenthetical"),
        ("transition", "Transition"),
        ("note", "Note"),
        ("new_act", "New Act"),
        ("end_of_act", "End of Act"),
        ("cast_list", "Cast List"),
        ("general", "General"),
        ("shot", "Shot"),
        ("sequence", "Sequence"),
        ("outline_1", "Outline 1"),
        ("outline_2", "Outline 2"),
        ("outline_3", "Outline 3"),
        ("summary", "Summary"),
    ];
    for (_, label) in legacy_roles {
        if project.element_definitions().get_by_label(label).is_none() {
            definitions.push(definition(label));
        }
    }
    valid(project.replace_element_definitions(definitions));
    let mut session = valid(EditingSession::new(project));
    for (wire_role, exact_label) in legacy_roles {
        let payload = serde_json::json!({
            "version": 1,
            "origin": "test",
            "command": {
                "id": "editor.element.change",
                "params": { "role": wire_role, "surface": "body" }
            }
        });
        let envelope: CommandEnvelope = valid(serde_json::from_value(payload));
        assert_eq!(envelope.validate_for_project(session.project()), Ok(()));
        assert!(matches!(
            execute_editor_command(&mut session, &envelope.command),
            CommandOutcome::Changed | CommandOutcome::NoOp
        ));
        assert_eq!(
            session.project().elements[0].element_type().as_str(),
            exact_label
        );
    }
}

#[test]
fn unknown_and_stale_targets_reject_without_changing_session_state() {
    let mut session = valid(EditingSession::new(project_with_arbitrary_types()));
    let unknown = change("Unregistered", EditorSurface::Body);
    let envelope = CommandEnvelope::new(CommandOrigin::Test, unknown.clone());
    assert_eq!(
        envelope.validate_for_project(session.project()),
        Err(EnvelopeError::InvalidElementTarget)
    );
    let before_unknown = session.clone();
    assert_eq!(
        execute_editor_command(&mut session, &unknown),
        CommandOutcome::Rejected(CommandRejection::InvalidElementTarget)
    );
    assert_eq!(session, before_unknown);

    let base_definitions = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    )
    .element_definitions()
    .iter()
    .cloned()
    .collect();
    assert!(session
        .replace_element_definitions_and_remap_types(ElementDefinitionCandidate::new(
            base_definitions,
        ))
        .is_ok());
    let stale = change("Lyrics", EditorSurface::Body);
    let stale_envelope = CommandEnvelope::new(CommandOrigin::Test, stale.clone());
    assert_eq!(
        stale_envelope.validate_for_project(session.project()),
        Err(EnvelopeError::InvalidElementTarget)
    );
    let before_stale = session.clone();
    assert_eq!(
        execute_editor_command(&mut session, &stale),
        CommandOutcome::Rejected(CommandRejection::InvalidElementTarget)
    );
    assert_eq!(session, before_stale);
}

#[test]
fn invalid_cross_paragraph_selection_is_rejected_atomically() {
    let mut session = valid(EditingSession::new(valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    )));
    assert!(session.enter().is_ok());
    assert!(session
        .set_selection(CoreSelection {
            anchor: CorePosition {
                element: 0,
                offset: 0,
            },
            focus: CorePosition {
                element: 1,
                offset: 0,
            },
        })
        .is_ok());
    let before = session.clone();
    let outcome = execute_editor_command(&mut session, &change("Action", EditorSurface::Body));
    assert_eq!(
        outcome,
        CommandOutcome::Rejected(CommandRejection::InvalidSelection(
            SelectionError::CrossElementRange
        ))
    );
    assert_eq!(session, before);
}

#[test]
fn body_and_title_wire_surfaces_validate_current_targets() {
    let project = project_with_arbitrary_types();
    for surface in [EditorSurface::Body, EditorSurface::TitlePage] {
        assert_eq!(
            CommandEnvelope::new(CommandOrigin::Test, change("Lyrics", surface))
                .validate_for_project(&project),
            Ok(())
        );
    }
}
