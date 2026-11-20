#![forbid(unsafe_code)]

use scriptcraft_commands::{
    Command, CommandEnvelope, CommandId, CommandOrigin, EditorSurface, ElementTypeValue,
    COMMAND_DESCRIPTORS, COMMAND_WIRE_VERSION, COMMAND_WIRE_VERSION_1,
};

const LEGACY_ROLE_LABELS: [(&str, &str); 17] = [
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

fn parse_envelope(json: &str) -> Result<CommandEnvelope, serde_json::Error> {
    serde_json::from_str(json)
}

#[test]
fn every_version_1_role_maps_to_its_exact_fdx_label_at_the_envelope_boundary() {
    for (role, label) in LEGACY_ROLE_LABELS {
        let wire = format!(
            r#"{{"version":1,"origin":"test","command":{{"id":"editor.element.change","params":{{"role":"{role}"}}}}}}"#
        );
        let envelope = parse_envelope(&wire).unwrap();
        assert_eq!(envelope.version, COMMAND_WIRE_VERSION_1);
        assert_eq!(
            envelope.command,
            Command::EditorElementChange {
                element_type: ElementTypeValue::new(label).unwrap(),
                surface: EditorSurface::Body,
            }
        );
        let canonical = serde_json::to_string(&envelope).unwrap();
        assert!(canonical.starts_with(r#"{"version":2,"origin":"test""#));
        assert!(canonical.contains("\"element_type\""));
        assert!(!canonical.contains("\"role\""));
    }
}

#[test]
fn version_2_preserves_arbitrary_and_case_distinct_labels_in_round_trips() {
    for label in ["Lyrics", "lyrics", "Notations"] {
        let envelope = CommandEnvelope::new(
            CommandOrigin::Test,
            Command::EditorElementChange {
                element_type: ElementTypeValue::new(label).unwrap(),
                surface: EditorSurface::Body,
            },
        );
        let wire = serde_json::to_string(&envelope).unwrap();
        assert!(wire.contains("\"element_type\""));
        assert!(!wire.contains("\"role\""));
        let decoded = parse_envelope(&wire).unwrap();
        assert_eq!(decoded, envelope);
    }
    assert_eq!(COMMAND_WIRE_VERSION, 2);
}

#[test]
fn command_22_identity_and_final_appended_command_order_remain_stable() {
    assert_eq!(
        CommandId::EditorElementChange.as_str(),
        "editor.element.change"
    );
    assert_eq!(CommandId::ALL[22], CommandId::EditorElementChange);
    assert_eq!(COMMAND_DESCRIPTORS.len(), CommandId::ALL.len());
    assert_eq!(
        COMMAND_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>(),
        CommandId::ALL.to_vec()
    );
    assert_eq!(COMMAND_DESCRIPTORS[22].id, CommandId::EditorElementChange);
    assert!(COMMAND_DESCRIPTORS[22].parameters.contains("element_type"));

    // IDs 1–52 retain their established positions; the remaining suffix is
    // derived from the final landed Elements/selector/New catalog, not a
    // planned command-count estimate.
    assert_eq!(
        &CommandId::ALL[52..],
        &[
            CommandId::FormatElementsOpen,
            CommandId::FormatElementsClose,
            CommandId::FormatTemplateSelectorOpen,
            CommandId::FormatTemplateSelectorCancel,
            CommandId::FormatTemplateApply,
            CommandId::FileNewFromTemplateOpen,
            CommandId::FileNewFromTemplate,
        ]
    );
}

#[test]
fn element_change_payload_rejects_mixed_missing_duplicate_and_unknown_forms() {
    for wire in [
        r#"{"version":1,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Lyrics"}}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"role":"action"}}}"#,
        r#"{"version":1,"origin":"test","command":{"id":"editor.element.change","params":{"role":"action","element_type":"Action"}}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Action","role":"action"}}}"#,
        r#"{"version":1,"origin":"test","command":{"id":"editor.element.change","params":{}}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{}}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Action","unknown":true}}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Action","element_type":"Dialogue"}}}"#,
        r#"{"version":1,"origin":"test","command":{"id":"editor.element.change","params":{"role":"action","role":"dialogue"}}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Action","surface":"body","surface":"title_page"}}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"file.new","extra":true}}"#,
        r#"{"version":2,"origin":"test","command":{"id":"pdf.include-title-page.resolve","params":{"options":{"include_title_page":true,"include_title_page":false}}}}"#,
    ] {
        assert!(
            parse_envelope(wire).is_err(),
            "accepted invalid wire: {wire}"
        );
    }
}

#[test]
fn old_unknown_roles_unknown_fields_and_unsupported_versions_fail() {
    let unknown_role = r#"{"version":1,"origin":"test","command":{"id":"editor.element.change","params":{"role":"lyrics"}}}"#;
    assert!(parse_envelope(unknown_role).is_err());

    let unknown_envelope_field =
        r#"{"version":2,"origin":"test","command":{"id":"file.new"},"extra":true}"#;
    assert!(parse_envelope(unknown_envelope_field).is_err());

    let unsupported_element_version = r#"{"version":3,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Action"}}}"#;
    assert!(parse_envelope(unsupported_element_version).is_err());

    let unsupported_other_command = r#"{"version":3,"origin":"test","command":{"id":"file.new"}}"#;
    let envelope = parse_envelope(unsupported_other_command).unwrap();
    assert!(envelope.validate().is_err());
}

#[test]
fn element_keys_remain_bounded_and_default_surface_remains_body() {
    assert!(ElementTypeValue::new("").is_err());
    assert!(ElementTypeValue::new("x".repeat(4097)).is_err());
    assert!(ElementTypeValue::new("x".repeat(4096)).is_ok());
    let empty_key_wire = r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":""}}}"#;
    assert!(parse_envelope(empty_key_wire).is_err());
    let overlong_key_wire = format!(
        r#"{{"version":2,"origin":"test","command":{{"id":"editor.element.change","params":{{"element_type":"{}"}}}}}}"#,
        "x".repeat(4097)
    );
    assert!(parse_envelope(&overlong_key_wire).is_err());

    let wire = r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Lyrics"}}}"#;
    assert!(matches!(
        parse_envelope(wire).unwrap().command,
        Command::EditorElementChange {
            surface: EditorSurface::Body,
            ..
        }
    ));

    let title_wire = r#"{"version":2,"origin":"test","command":{"id":"editor.element.change","params":{"element_type":"Lyrics","surface":"title_page"}}}"#;
    assert!(matches!(
        parse_envelope(title_wire).unwrap().command,
        Command::EditorElementChange {
            surface: EditorSurface::TitlePage,
            ..
        }
    ));
}

#[test]
fn other_command_payload_shapes_and_command_only_json_boundary_are_unchanged() {
    let envelope = CommandEnvelope::new(
        CommandOrigin::Test,
        Command::EditorTextInsert {
            text: "unchanged".to_owned(),
            surface: EditorSurface::Body,
        },
    );
    let wire = serde_json::to_string(&envelope).unwrap();
    assert_eq!(
        wire,
        r#"{"version":2,"origin":"test","command":{"id":"editor.text.insert","params":{"text":"unchanged","surface":"body"}}}"#
    );
    assert!(matches!(
        parse_envelope(&wire).unwrap().command,
        Command::EditorTextInsert { .. }
    ));

    // This source exercises JSON command envelopes only. It does not call or
    // introduce a document JSON persistence, import, or migration API.
}
