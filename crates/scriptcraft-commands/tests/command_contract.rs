#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_commands::{
    descriptor_for, validate_descriptor_catalog, AvailabilityPolicy, Command, CommandDescriptor,
    CommandEnvelope, CommandId, CommandOrigin, DiagnosticMessage, DocumentPath,
    EditorSurface, EnvelopeError, HostEffectDescription, ParagraphAlignmentValue, PdfExportOptions,
    Position, Selection, TitleTextStyle, ZoomFactor, ZoomPercent, COMMAND_DESCRIPTORS,
    COMMAND_WIRE_VERSION, MAX_DIAGNOSTIC_MESSAGE_BYTES,
};
use serde::de::value::{Error as ValueError, MapDeserializer};
use serde::de::{IntoDeserializer, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone)]
enum WireValue {
    Null,
    String(&'static str),
    Bool(bool),
    U64(u64),
    Map(Vec<(&'static str, WireValue)>),
}

impl<'de> IntoDeserializer<'de, ValueError> for WireValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> Deserializer<'de> for WireValue {
    type Error = ValueError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Null => visitor.visit_unit(),
            Self::String(value) => visitor.visit_string(value.to_owned()),
            Self::Bool(value) => visitor.visit_bool(value),
            Self::U64(value) => visitor.visit_u64(value),
            Self::Map(entries) => visitor.visit_map(MapDeserializer::new(
                entries
                    .into_iter()
                    .map(|(key, value)| (Self::String(key), value)),
            )),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Null => visitor.visit_none(),
            value => visitor.visit_some(value),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::String(value) => serde::de::value::StringDeserializer::<ValueError>::new(
                value.to_owned(),
            )
            .deserialize_enum(name, variants, visitor),
            _ => Err(serde::de::Error::custom("expected enum string")),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple tuple_struct map struct
        identifier ignored_any
    }
}

fn command_wire(id: &'static str, params: Option<Vec<(&'static str, WireValue)>>) -> WireValue {
    let mut entries = vec![("id", WireValue::String(id))];
    if let Some(params) = params {
        entries.push(("params", WireValue::Map(params)));
    }
    WireValue::Map(entries)
}

fn parameterless(id: &'static str) -> WireValue {
    command_wire(id, None)
}

fn params(entries: Vec<(&'static str, WireValue)>) -> Option<Vec<(&'static str, WireValue)>> {
    Some(entries)
}

fn assert_serializable<T: Serialize>() {}

#[test]
fn command_catalog_has_complete_unique_stable_dotted_identities() {
    assert_eq!(validate_descriptor_catalog(), Ok(()));
    assert_eq!(CommandId::ALL.len(), 50);
    assert_eq!(COMMAND_DESCRIPTORS.len(), 50);

    let ids = CommandId::ALL.map(CommandId::as_str);
    for (index, id) in ids.iter().enumerate() {
        assert!(id.contains('.'), "stable command identity must be dotted: {id}");
        assert!(!ids[..index].contains(id), "duplicate stable identity: {id}");
    }
    for (index, descriptor) in COMMAND_DESCRIPTORS.iter().enumerate() {
        assert!(!descriptor.parameters.is_empty());
        assert_eq!(descriptor.availability, AvailabilityPolicy::RecheckAtExecution);
        assert!(!COMMAND_DESCRIPTORS[..index]
            .iter()
            .any(|earlier| earlier.id == descriptor.id));
        assert!(CommandId::ALL.contains(&descriptor.id));
        assert_eq!(descriptor_for(descriptor.id), Some(descriptor));
    }
    for id in CommandId::ALL {
        assert_eq!(COMMAND_DESCRIPTORS.iter().filter(|entry| entry.id == id).count(), 1);
    }
}

#[test]
fn introduction_order_is_independent_of_catalog_discovery_order() {
    // This is the compatibility sequence from the public contract, not the
    // implementation's enum or descriptor source order (where file.print is
    // grouped with the other file descriptors).
    let introduced = [
        "file.new", "file.open.request", "file.open", "file.close", "file.save",
        "file.save-as.request", "file.save-as", "file.save-as-pdf", "file.import.resolve",
        "edit.undo", "edit.redo", "edit.cut", "edit.copy", "edit.paste", "edit.delete",
        "edit.select-all", "editor.text.insert", "editor.text.paste", "editor.delete.backward",
        "editor.paragraph.enter", "editor.paragraph.tab", "editor.element.change",
        "editor.selection.set", "editor.selection.pointer", "editor.navigation.horizontal",
        "editor.navigation.vertical", "editor.navigation.line-edge", "editor.focus",
        "window.title-page.open", "window.title-page.close", "editor.style.toggle",
        "editor.alignment.set", "pdf.include-title-page.resolve", "view.zoom.set",
        "view.zoom.pinch", "view.appearance.set", "view.focus-mode.toggle",
        "tools.sprint-timer.toggle-popup", "tools.sprint-timer.close-popup",
        "tools.sprint-timer.set-mode", "tools.sprint-timer.set-minutes", "tools.sprint-timer.start",
        "tools.sprint-timer.pause", "tools.sprint-timer.resume", "tools.sprint-timer.reset",
        "tools.sprint-timer.end", "ui.status.set", "editor.dual-dialogue.toggle", "file.print",
        "editor.style.clear",
    ];
    assert_eq!(introduced.len(), 50);
    for expected in introduced {
        let id = expected
            .parse::<CommandId>()
            .unwrap_or_else(|_| unreachable!("compatibility ID is present in the typed catalog"));
        assert_eq!(id.as_str(), expected);
        assert!(descriptor_for(id).is_some());
    }
    assert_eq!(introduced[47], "editor.dual-dialogue.toggle");
    assert_eq!(introduced[48], "file.print");
    assert_eq!(introduced[49], "editor.style.clear");
    assert_eq!(CommandId::ALL[8].as_str(), "file.print");
    assert_ne!(CommandId::ALL[48].as_str(), introduced[48]);
}

#[test]
fn wire_v1_envelope_and_adjacent_command_shape_are_stable() {
    assert_eq!(COMMAND_WIRE_VERSION, 1);
    assert_serializable::<Command>();
    assert_serializable::<CommandEnvelope>();
    let envelope = CommandEnvelope::new(CommandOrigin::Test, Command::EditorDualDialogueToggle);
    assert_eq!(envelope.version, 1);
    assert_eq!(envelope.validate(), Ok(()));
    let decoded_envelope = CommandEnvelope::deserialize(WireValue::Map(vec![
        ("version", WireValue::U64(1)),
        ("origin", WireValue::String("test")),
        ("command", parameterless("editor.dual-dialogue.toggle")),
    ]))
    .unwrap_or_else(|_| unreachable!("version-one envelope shape decodes"));
    assert_eq!(decoded_envelope, envelope);
    assert!(CommandEnvelope::deserialize(WireValue::Map(vec![
        ("version", WireValue::U64(1)),
        ("origin", WireValue::String("test")),
        ("command", parameterless("file.new")),
        ("extra", WireValue::Bool(true)),
    ]))
    .is_err());
    let decoded = Command::deserialize(parameterless("editor.dual-dialogue.toggle"))
        .unwrap_or_else(|_| unreachable!("known parameterless command wire payload"));
    assert_eq!(decoded, Command::EditorDualDialogueToggle);
    assert_eq!(decoded.id(), CommandId::EditorDualDialogueToggle);

    let unsupported = CommandEnvelope { version: 2, ..envelope };
    assert_eq!(unsupported.validate(), Err(EnvelopeError::UnsupportedVersion(2)));
    assert!(Command::deserialize(parameterless("editor.not-a-command")).is_err());
    assert!(Command::deserialize(WireValue::Map(vec![("id", WireValue::String("file.new")),
        ("unexpected", WireValue::Bool(true))])).is_err());
}

#[test]
fn compatibility_defaults_and_typed_path_selection_parameters_are_enforced() {
    let legacy_save = Command::deserialize(command_wire(
        "file.save-as",
        params(vec![("path", WireValue::String("/tmp/legacy.fdx"))]),
    ))
    .unwrap_or_else(|_| unreachable!("legacy save-as payload remains decodable"));
    assert!(matches!(legacy_save, Command::FileSaveAs { overwrite: false, .. }));
    let confirmed_save = Command::deserialize(command_wire(
        "file.save-as",
        params(vec![
            ("path", WireValue::String("/tmp/confirmed.fdx")),
            ("overwrite", WireValue::Bool(true)),
        ]),
    ))
    .unwrap_or_else(|_| unreachable!("explicit overwrite consent decodes"));
    assert!(matches!(confirmed_save, Command::FileSaveAs { overwrite: true, .. }));
    assert!(DocumentPath::new("  ").is_err());
    assert!(DocumentPath::new("bad\0path").is_err());
    assert!(Command::deserialize(command_wire(
        "file.open",
        params(vec![("path", WireValue::String("\0"))]),
    ))
    .is_err());

    let legacy_text = Command::deserialize(command_wire(
        "editor.text.insert",
        params(vec![("text", WireValue::String("legacy"))]),
    ))
    .unwrap_or_else(|_| unreachable!("legacy body payload remains decodable"));
    assert!(matches!(legacy_text, Command::EditorTextInsert { surface: EditorSurface::Body, .. }));
    let title_text = Command::deserialize(command_wire(
        "editor.text.insert",
        params(vec![
            ("text", WireValue::String("title")),
            ("surface", WireValue::String("title_page")),
        ]),
    ))
    .unwrap_or_else(|_| unreachable!("explicit title-page surface decodes"));
    assert!(matches!(title_text, Command::EditorTextInsert { surface: EditorSurface::TitlePage, .. }));
    assert!(Command::deserialize(command_wire(
        "editor.text.insert",
        params(vec![
            ("text", WireValue::String("bad")),
            ("surface", WireValue::String("canvas")),
        ]),
    ))
    .is_err());
    assert!(Command::deserialize(command_wire(
        "editor.text.insert",
        params(vec![
            ("text", WireValue::String("bad")),
            ("extra", WireValue::Bool(true)),
        ]),
    ))
    .is_err());
    assert!(Command::deserialize(parameterless("editor.text.insert")).is_err());

    assert!(ZoomPercent::new(74).is_err());
    assert!(ZoomPercent::new(301).is_err());
    assert!(ZoomFactor::new(0.0).is_err());
    assert!(ZoomFactor::new(f32::INFINITY).is_err());
    let project = scriptcraft_core::Project::new().unwrap_or_else(|_| unreachable!("empty project"));
    let invalid_selection = Selection {
        anchor: Position { element: u32::MAX, offset: 0 },
        focus: Position { element: u32::MAX, offset: 0 },
    };
    assert!(invalid_selection.validate_for_surface(&project, EditorSurface::Body).is_err());
    let invalid_envelope = CommandEnvelope::new(
        CommandOrigin::Test,
        Command::EditorSelectionSet { selection: invalid_selection, surface: EditorSurface::Body },
    );
    assert!(matches!(
        invalid_envelope.validate_for_project(&project),
        Err(EnvelopeError::InvalidSelection(_))
    ));
    assert!(Command::deserialize(command_wire(
        "view.zoom.set",
        params(vec![("percent", WireValue::U64(74))]),
    ))
    .is_err());
}

#[test]
fn formatting_pdf_and_diagnostic_payloads_decode_with_bounds_and_reject_unknowns() {
    let clear = Command::deserialize(command_wire(
        "editor.style.clear",
        params(vec![("surface", WireValue::String("title_page"))]),
    ))
    .unwrap_or_else(|_| unreachable!("style-clear surface payload decodes"));
    assert_eq!(clear, Command::EditorStyleClear { surface: EditorSurface::TitlePage });
    let style = Command::deserialize(command_wire(
        "editor.style.toggle",
        params(vec![
            ("surface", WireValue::String("body")),
            ("style", WireValue::String("strikethrough")),
        ]),
    ))
    .unwrap_or_else(|_| unreachable!("bounded style payload decodes"));
    assert!(matches!(style, Command::EditorStyleToggle { style: TitleTextStyle::Strikethrough, .. }));
    let alignment = Command::deserialize(command_wire(
        "editor.alignment.set",
        params(vec![
            ("surface", WireValue::String("body")),
            ("alignment", WireValue::String("justified")),
        ]),
    ))
    .unwrap_or_else(|_| unreachable!("bounded alignment payload decodes"));
    assert!(matches!(alignment, Command::EditorAlignmentSet { alignment: ParagraphAlignmentValue::Justified, .. }));
    assert!(Command::deserialize(command_wire(
        "editor.alignment.set",
        params(vec![
            ("surface", WireValue::String("body")),
            ("alignment", WireValue::String("diagonal")),
        ]),
    ))
    .is_err());

    let pdf_default = PdfExportOptions::default();
    assert!(pdf_default.include_title_page);
    let pdf = Command::deserialize(command_wire(
        "pdf.include-title-page.resolve",
        params(vec![("options", WireValue::Map(vec![(
            "include_title_page", WireValue::Bool(false),
        )]))]),
    ))
    .unwrap_or_else(|_| unreachable!("typed PDF options decode"));
    assert!(matches!(pdf, Command::PdfIncludeTitlePageResolve { options: PdfExportOptions { include_title_page: false } }));
    assert!(Command::deserialize(command_wire(
        "pdf.include-title-page.resolve",
        params(vec![("options", WireValue::Map(vec![
            ("include_title_page", WireValue::Bool(false)),
            ("unexpected", WireValue::Bool(true)),
        ]))]),
    ))
    .is_err());

    let at_limit = DiagnosticMessage::new("x".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES));
    assert!(at_limit.is_ok());
    assert!(DiagnosticMessage::new("x".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES + 1)).is_err());
    assert!(DiagnosticMessage::new("two\nlines").is_err());
    let status = Command::deserialize(command_wire(
        "ui.status.set",
        params(vec![("message", WireValue::String("READY"))]),
    ))
    .unwrap_or_else(|_| unreachable!("bounded diagnostic payload decodes"));
    assert!(matches!(status, Command::UiStatusSet { message: Some(_ ) }));
    let clear_status = Command::deserialize(command_wire(
        "ui.status.set",
        params(vec![("message", WireValue::Null)]),
    ))
    .unwrap_or_else(|_| unreachable!("nullable diagnostic status decodes"));
    assert!(matches!(clear_status, Command::UiStatusSet { message: None }));
}

#[test]
fn descriptors_declare_host_effects_separately_from_semantic_commands() {
    let host_commands = [
        (Command::FileSaveAsPdf, HostEffectDescription::OpenPdfDestinationPicker),
        (Command::FilePrint, HostEffectDescription::OpenPrintPanel),
        (Command::EditorFocus, HostEffectDescription::FocusActiveEditor),
        (Command::WindowTitlePageOpen, HostEffectDescription::OpenTitlePageWindow),
        (Command::WindowTitlePageClose, HostEffectDescription::CloseTitlePageWindow),
        (Command::ViewFocusModeToggle, HostEffectDescription::ToggleFocusModeViewport),
    ];
    for (command, effect) in host_commands {
        assert_eq!(command.host_effect_description(), Some(effect));
    }
    assert_eq!(Command::EditUndo.host_effect_description(), None);
    assert_eq!(Command::EditorDualDialogueToggle.host_effect_description(), None);
    let print: Option<&CommandDescriptor> = descriptor_for(CommandId::FilePrint);
    assert!(print.is_some_and(|entry| entry.family == "file" && entry.parameters.contains("print host effect")));
    assert!(descriptor_for(CommandId::EditorDualDialogueToggle)
        .is_some_and(|entry| entry.parameters.contains("Body only")));
}
