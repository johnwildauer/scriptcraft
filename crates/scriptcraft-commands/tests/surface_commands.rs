#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_commands::{
    execute_editor_command, execute_editor_command_on_surface, validate_descriptor_catalog,
    Command, CommandEnvelope, CommandId, CommandOrigin, CommandOutcome, CommandRejection,
    EditorSurface, EnvelopeError, HostEffectDescription, ParagraphAlignmentValue, PdfExportOptions,
    Position, Selection, TitleTextStyle, UnavailableReason,
    COMMAND_DESCRIPTORS,
};
use scriptcraft_core::editing::{EditingSession, Position as CorePosition, Selection as CoreSelection};
use scriptcraft_core::{Project, Role};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("surface command fixture is valid"))
}

fn session() -> EditingSession {
    valid(EditingSession::new(valid(Project::new())))
}

fn position(element: u32, offset: u32) -> Position {
    Position { element, offset }
}

#[test]
fn catalog_has_stable_additive_surface_and_host_effect_ids() {
    assert_eq!(validate_descriptor_catalog(), Ok(()));
    assert_eq!(CommandId::ALL.len(), 50);
    assert_eq!(COMMAND_DESCRIPTORS.len(), 50);
    for (id, spelling) in [
        (CommandId::WindowTitlePageOpen, "window.title-page.open"),
        (CommandId::WindowTitlePageClose, "window.title-page.close"),
        (CommandId::EditorStyleToggle, "editor.style.toggle"),
        (CommandId::EditorAlignmentSet, "editor.alignment.set"),
        (CommandId::EditorDualDialogueToggle, "editor.dual-dialogue.toggle"),
        (CommandId::EditorStyleClear, "editor.style.clear"),
        (CommandId::PdfIncludeTitlePageResolve, "pdf.include-title-page.resolve"),
    ] {
        assert_eq!(id.as_str(), spelling);
        assert!(COMMAND_DESCRIPTORS.iter().any(|descriptor| descriptor.id == id));
    }
    assert!(PdfExportOptions::default().include_title_page);
    assert_eq!(
        Command::FileSaveAsPdf.host_effect_description(),
        Some(HostEffectDescription::OpenPdfDestinationPicker)
    );
    assert_eq!(
        Command::WindowTitlePageOpen.host_effect_description(),
        Some(HostEffectDescription::OpenTitlePageWindow)
    );
    assert_eq!(
        Command::EditorFocus.host_effect_description(),
        Some(HostEffectDescription::FocusActiveEditor)
    );
}

#[test]
fn plain_text_surface_command_is_additive_and_keeps_wire_version_one() {
    let command = Command::EditorStyleClear {
        surface: EditorSurface::TitlePage,
    };
    let envelope = CommandEnvelope::new(CommandOrigin::Test, command.clone());
    assert_eq!(CommandId::EditorStyleClear.as_str(), "editor.style.clear");
    assert_eq!(command.id(), CommandId::EditorStyleClear);
    assert_eq!(envelope.version, 1);
    assert!(envelope.validate().is_ok());

    let mut editing = session();
    valid(editing.insert_title("Title"));
    valid(editing.set_title_selection(CoreSelection {
        anchor: CorePosition { element: 0, offset: 0 },
        focus: CorePosition { element: 0, offset: 5 },
    }));
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorStyleToggle {
                surface: EditorSurface::TitlePage,
                style: TitleTextStyle::Bold,
            },
        ),
        CommandOutcome::Changed,
    );
    assert_eq!(
        execute_editor_command(&mut editing, &command),
        CommandOutcome::Changed,
    );
    assert!(editing.project().title_page[0].runs.iter().all(|run| {
        !run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && !attribute.value.is_empty()
        })
    }));
}

#[test]
fn typed_text_style_alignment_and_history_delegate_to_both_surface_session_apis() {
    let mut editing = session();
    let body_before = editing.project().elements[0].text.clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert {
                text: "Title-side".into(),
                surface: EditorSurface::TitlePage,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(editing.project().elements[0].text, body_before);
    assert!(editing.project().title_page[0].text.starts_with("Title-side"));

    let title_text = editing.project().title_page[17].text.clone();
    valid(editing.set_title_selection(CoreSelection {
        anchor: CorePosition { element: 17, offset: 0 },
        focus: CorePosition { element: 17, offset: title_text.len() },
    }));
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorStyleToggle {
                surface: EditorSurface::TitlePage,
                style: TitleTextStyle::Bold,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorAlignmentSet {
                surface: EditorSurface::TitlePage,
                alignment: ParagraphAlignmentValue::Right,
            },
        ),
        CommandOutcome::Changed
    );
    assert!(editing.can_undo());
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);

    let mut body = session();
    assert_eq!(
        execute_editor_command(
            &mut body,
            &Command::EditorElementChange {
                role: scriptcraft_commands::RoleValue::new(Role::Character),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(body.project().elements[0].role, Role::Character);
}

#[test]
fn parameterless_edit_commands_use_host_surface_but_keep_one_global_history() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command_on_surface(
            &mut editing,
            &Command::EditorTextInsert { text: "body".into(), surface: EditorSurface::Body },
            EditorSurface::Body,
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command_on_surface(
            &mut editing,
            &Command::EditorTextInsert { text: " title".into(), surface: EditorSurface::TitlePage },
            EditorSurface::TitlePage,
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditSelectAll, EditorSurface::TitlePage),
        CommandOutcome::Changed
    );
    assert_eq!(
        editing.title_selection(),
        CoreSelection {
            anchor: CorePosition { element: 0, offset: 0 },
            focus: CorePosition { element: 53, offset: editing.project().title_page[53].text.len() },
        }
    );
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_eq!(editing.project().title_page[0].text, "");
}

#[test]
fn surface_validation_rejects_wrong_vector_indexes_without_changing_session() {
    let mut editing = session();
    let before = editing.clone();
    let invalid = Command::EditorSelectionSet {
        selection: Selection {
            anchor: position(u32::MAX, 0),
            focus: position(u32::MAX, 0),
        },
        surface: EditorSurface::TitlePage,
    };
    assert!(matches!(
        execute_editor_command(&mut editing, &invalid),
        CommandOutcome::Rejected(CommandRejection::InvalidSelection(_))
    ));
    assert_eq!(editing, before);
}

#[test]
fn envelope_document_validation_checks_the_declared_surface_vector() {
    let project = valid(Project::new());
    let title_selection = Selection {
        anchor: position(17, 0),
        focus: position(17, 5),
    };
    let title_envelope = CommandEnvelope::new(
        CommandOrigin::Test,
        Command::EditorSelectionSet {
            selection: title_selection,
            surface: EditorSurface::TitlePage,
        },
    );
    assert_eq!(title_envelope.validate_for_project(&project), Ok(()));

    let body_envelope = CommandEnvelope::new(
        CommandOrigin::Test,
        Command::EditorSelectionSet {
            selection: title_selection,
            surface: EditorSurface::Body,
        },
    );
    assert!(matches!(
        body_envelope.validate_for_project(&project),
        Err(EnvelopeError::InvalidSelection(_))
    ));
}

#[test]
fn pointer_selection_is_validated_and_applied_to_only_its_declared_surface() {
    let mut editing = session();
    let body_before = editing.selection();
    let pointer = Command::EditorSelectionPointer {
        position: position(17, 2),
        phase: scriptcraft_commands::PointerPhase::Press,
        surface: EditorSurface::TitlePage,
    };
    assert_eq!(execute_editor_command(&mut editing, &pointer), CommandOutcome::Changed);
    assert_eq!(editing.selection(), body_before);
    assert_eq!(editing.title_selection().focus, CorePosition { element: 17, offset: 2 });
}

#[test]
fn title_role_command_and_host_effects_are_rejected_headlessly_without_mutation() {
    let mut editing = session();
    let before = editing.clone();
    for command in [
        Command::EditorElementChange {
            role: scriptcraft_commands::RoleValue::new(Role::Dialogue),
            surface: EditorSurface::TitlePage,
        },
        Command::WindowTitlePageOpen,
        Command::WindowTitlePageClose,
        Command::EditorFocus,
        Command::FileSaveAsPdf,
        Command::ViewFocusModeToggle,
        Command::PdfIncludeTitlePageResolve { options: PdfExportOptions::default() },
    ] {
        let id = command.id();
        assert_eq!(
            execute_editor_command(&mut editing, &command),
            CommandOutcome::Rejected(CommandRejection::UnsupportedCommand(id))
        );
        assert_eq!(editing, before);
    }
}

#[test]
fn title_paragraph_enter_tab_and_undo_share_body_history_semantics() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command_on_surface(
            &mut editing,
            &Command::EditorTextInsert { text: "title line".into(), surface: EditorSurface::TitlePage },
            EditorSurface::TitlePage,
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorParagraphTab, EditorSurface::TitlePage),
        CommandOutcome::NoOp
    );
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorParagraphEnter, EditorSurface::TitlePage),
        CommandOutcome::Changed
    );
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_eq!(editing.project().title_page.len(), 54);
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditSelectAll, EditorSurface::Body),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditUndo),
        CommandOutcome::Changed
    );
    assert_eq!(editing.project().title_page[0].text, "");
    let mut fresh = session();
    assert_eq!(
        execute_editor_command_on_surface(&mut fresh, &Command::EditUndo, EditorSurface::TitlePage),
        CommandOutcome::Unavailable(UnavailableReason::UndoHistory)
    );
}
