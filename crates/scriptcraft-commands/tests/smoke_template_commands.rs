#![forbid(unsafe_code)]

use scriptcraft_commands::{
    builtin_template_id, builtin_template_value, execute_editor_command, BuiltinTemplateValue,
    Command, CommandEnvelope, CommandId, CommandOrigin, CommandOutcome, EditorSurface,
    HostEffectDescription, COMMAND_WIRE_VERSION,
};
use scriptcraft_fdx::{instantiate_builtin_template, BuiltinTemplateId};
use scriptcraft_session::editing::EditingSession;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("built-in template command source uses valid input"))
}

#[test]
fn new_from_template_commands_are_typed_host_effects_and_catalog_ids() {
    let open = Command::FileNewFromTemplateOpen;
    assert_eq!(open.id(), CommandId::FileNewFromTemplateOpen);
    assert_eq!(open.id().as_str(), "file.new-from-template.open");
    assert_eq!(
        open.host_effect_description(),
        Some(HostEffectDescription::OpenNewTemplateSelector)
    );
    assert_eq!(
        serde_json::to_value(open)
            .unwrap_or_else(|_| unreachable!("New selector command serializes")),
        serde_json::json!({ "id": "file.new-from-template.open" })
    );

    let launch = Command::FileNewFromTemplate {
        template: BuiltinTemplateValue::DramatistsGuildMusical,
    };
    assert_eq!(launch.id(), CommandId::FileNewFromTemplate);
    assert_eq!(launch.id().as_str(), "file.new-from-template");
    assert_eq!(
        launch.host_effect_description(),
        Some(HostEffectDescription::LaunchIndependentTemplateChild(
            BuiltinTemplateValue::DramatistsGuildMusical
        ))
    );
    assert_eq!(
        serde_json::to_value(launch)
            .unwrap_or_else(|_| unreachable!("typed New command serializes")),
        serde_json::json!({
            "id": "file.new-from-template",
            "params": { "template": "dramatists-guild-musical" }
        })
    );
    assert!(scriptcraft_commands::descriptor_for(CommandId::FileNewFromTemplateOpen).is_some());
    assert!(scriptcraft_commands::descriptor_for(CommandId::FileNewFromTemplate).is_some());
}

#[test]
fn command_values_convert_exactly_to_and_from_the_builtin_catalog() {
    for (value, id, wire_id) in [
        (
            BuiltinTemplateValue::Screenplay,
            BuiltinTemplateId::Screenplay,
            "screenplay",
        ),
        (
            BuiltinTemplateValue::Stageplay,
            BuiltinTemplateId::Stageplay,
            "stageplay",
        ),
        (
            BuiltinTemplateValue::DramatistsGuildMusical,
            BuiltinTemplateId::DramatistsGuildMusical,
            "dramatists-guild-musical",
        ),
    ] {
        assert_eq!(value.as_str(), wire_id);
        assert_eq!(builtin_template_id(value), id);
        assert_eq!(builtin_template_value(id), value);
        let command = Command::FormatTemplateApply { template: value };
        assert_eq!(command.id().as_str(), "format.template.apply");
        assert_eq!(
            serde_json::to_value(command)
                .unwrap_or_else(|_| unreachable!("typed catalog command serializes")),
            serde_json::json!({
                "id": "format.template.apply",
                "params": { "template": wire_id }
            })
        );

        let new_command = Command::FileNewFromTemplate { template: value };
        assert_eq!(new_command.id(), CommandId::FileNewFromTemplate);
        assert_eq!(
            new_command.host_effect_description(),
            Some(HostEffectDescription::LaunchIndependentTemplateChild(value))
        );
        assert_eq!(
            serde_json::to_value(new_command)
                .unwrap_or_else(|_| unreachable!("typed New catalog command serializes")),
            serde_json::json!({
                "id": "file.new-from-template",
                "params": { "template": wire_id }
            })
        );
    }
    assert!(serde_json::from_str::<BuiltinTemplateValue>("\"Screenplay\"").is_err());
    assert!(serde_json::from_str::<BuiltinTemplateValue>("\"arbitrary\"").is_err());
}

#[test]
fn phase_nine_template_envelopes_keep_v2_and_host_commands_out_of_the_executor() {
    assert_eq!(COMMAND_WIRE_VERSION, 2);
    let host_commands = [
        (Command::FormatElementsOpen, "format.elements.open"),
        (Command::FormatElementsClose, "format.elements.close"),
        (
            Command::FormatTemplateSelectorOpen,
            "format.template-selector.open",
        ),
        (
            Command::FormatTemplateSelectorCancel,
            "format.template-selector.cancel",
        ),
    ];
    let document = valid(instantiate_builtin_template(BuiltinTemplateId::Screenplay));
    let mut session = valid(EditingSession::new(document.project));

    for (command, stable_id) in host_commands {
        assert_eq!(command.id().as_str(), stable_id);
        assert!(scriptcraft_commands::descriptor_for(command.id()).is_some());
        assert!(command.host_effect_description().is_some());
        let envelope = CommandEnvelope::new(CommandOrigin::NativeMenu, command.clone());
        let encoded = serde_json::to_value(&envelope)
            .unwrap_or_else(|_| unreachable!("Phase 9 host command envelope serializes"));
        assert_eq!(encoded["version"], serde_json::json!(2));
        assert_eq!(encoded["origin"], serde_json::json!("native_menu"));
        assert_eq!(encoded["command"]["id"], serde_json::json!(stable_id));
        assert_eq!(
            serde_json::from_value::<CommandEnvelope>(encoded)
                .unwrap_or_else(|_| unreachable!("Phase 9 host command envelope round-trips")),
            envelope
        );
        let before = session.clone();
        assert_eq!(
            execute_editor_command(&mut session, &command),
            CommandOutcome::Rejected(scriptcraft_commands::CommandRejection::UnsupportedCommand(
                command.id()
            ))
        );
        assert_eq!(session, before);
    }

    let apply = Command::FormatTemplateApply {
        template: BuiltinTemplateValue::DramatistsGuildMusical,
    };
    let envelope = CommandEnvelope::new(CommandOrigin::Test, apply.clone());
    let encoded = serde_json::to_value(&envelope)
        .unwrap_or_else(|_| unreachable!("typed Apply envelope serializes"));
    assert_eq!(encoded["version"], serde_json::json!(2));
    assert_eq!(
        encoded["command"]["params"]["template"],
        serde_json::json!("dramatists-guild-musical")
    );
    assert_eq!(
        serde_json::from_value::<CommandEnvelope>(encoded)
            .unwrap_or_else(|_| unreachable!("typed Apply envelope round-trips")),
        envelope
    );
}

#[test]
fn shared_executor_applies_through_the_phase_eight_service_as_one_candidate() {
    let current = valid(instantiate_builtin_template(
        BuiltinTemplateId::DramatistsGuildMusical,
    ));
    let target = valid(instantiate_builtin_template(BuiltinTemplateId::Screenplay));
    let current_body = current
        .project
        .elements
        .iter()
        .map(|owner| owner.as_element().clone())
        .collect::<Vec<_>>();
    let current_title_text = current
        .project
        .title_page
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect::<Vec<_>>();
    let before = current.project;
    let mut session = valid(EditingSession::new(before.clone()));
    let revision = session.revision();
    let mutation_token = session.mutation_token();

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::FormatTemplateApply {
                template: BuiltinTemplateValue::Screenplay,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(session.revision(), revision + 1);
    assert_eq!(session.mutation_token(), mutation_token + 1);
    assert!(session.dirty());
    assert_eq!(
        session.project().element_definitions(),
        target.project.element_definitions()
    );
    assert_eq!(
        session
            .project()
            .elements
            .iter()
            .map(|owner| owner.as_element().text.as_str())
            .collect::<Vec<_>>(),
        current_body
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        session
            .project()
            .title_page
            .iter()
            .map(|paragraph| paragraph.text.clone())
            .collect::<Vec<_>>(),
        current_title_text
    );
    assert_ne!(session.project(), &target.project);

    valid(session.undo());
    assert_eq!(session.project(), &before);
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::FormatTemplateApply {
                template: BuiltinTemplateValue::Screenplay,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(session.revision(), revision + 3);
    assert_eq!(session.mutation_token(), mutation_token + 3);
    assert!(session.can_undo());
    assert!(!session.can_redo());
}

#[test]
fn executor_reports_true_noop_without_a_second_mutation() {
    let document = valid(instantiate_builtin_template(BuiltinTemplateId::Screenplay));
    let mut session = valid(EditingSession::new(document.project));
    let before = session.clone();

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::FormatTemplateApply {
                template: BuiltinTemplateValue::Screenplay,
            },
        ),
        CommandOutcome::NoOp
    );
    assert_eq!(session, before);
}

#[test]
fn apply_command_is_not_a_surface_specific_edit() {
    let document = valid(instantiate_builtin_template(BuiltinTemplateId::Screenplay));
    let mut session = valid(EditingSession::new(document.project));
    let command = Command::FormatTemplateApply {
        template: BuiltinTemplateValue::Stageplay,
    };

    let outcome = scriptcraft_commands::execute_editor_command_on_surface(
        &mut session,
        &command,
        EditorSurface::TitlePage,
    );
    assert_eq!(outcome, CommandOutcome::Changed);
}
