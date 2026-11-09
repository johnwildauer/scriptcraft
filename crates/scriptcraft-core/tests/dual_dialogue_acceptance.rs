#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, FdxDocument};
use scriptcraft_core::fdx_storage::{
    save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::formatting::SourceLayoutSettings;
use scriptcraft_core::layout::{LayoutComposition, ScreenplayLayout};
use scriptcraft_core::{LayoutInputs, Project, Role};
use std::fs;
use std::path::PathBuf;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("bounded dual-dialogue acceptance journey is valid"))
}

fn import_reference() -> FdxDocument {
    valid(read_v6(
        include_str!("fixtures/dual_dialogue_basic.fdx").as_bytes(),
    ))
}

struct TemporaryFdx(PathBuf);

impl TemporaryFdx {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "scriptcraft-dual-dialogue-acceptance-{}.fdx",
            uuid::Uuid::new_v4()
        )))
    }
}

impl Drop for TemporaryFdx {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn serialized(project: &Project) -> String {
    String::from_utf8(valid(scriptcraft_core::fdx::write_v6(project)))
        .unwrap_or_else(|_| unreachable!("FDX serialization is UTF-8"))
}

fn assert_flat_members(project: &Project, member_ids: &[scriptcraft_core::StableId]) {
    assert_eq!(project.elements[1..7].iter().map(|item| item.id.clone()).collect::<Vec<_>>(), member_ids);
    assert_eq!(project.dual_dialogue_groups.len(), 1);
    assert_eq!(project.dual_dialogue_groups[0].member_ids().cloned().collect::<Vec<_>>(), member_ids);
}

#[test]
fn reference_import_edit_structural_retention_unwrap_history_save_and_spell_scopes() {
    let imported = import_reference();
    let initial = imported.project;
    assert_eq!(initial.dual_dialogue_groups.len(), 1);
    let member_ids = initial.dual_dialogue_groups[0]
        .member_ids()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(initial.elements[1..7].iter().map(|item| item.id.clone()).collect::<Vec<_>>(), member_ids);

    let mut session = valid(EditingSession::new(initial.clone()));
    let imported_xml = serialized(session.project());
    assert_eq!(imported_xml.matches("SpellCheckIgnoreLists").count(), 4);
    assert!(imported_xml.contains("vendor-scope=\"group\""));
    assert!(imported_xml.contains("vendor-scope=\"document\""));
    assert!(imported_xml.contains("<Word>colour</Word>"));
    assert_flat_members(session.project(), &member_ids);

    // Ordinary text editing is nested semantically but remains flat in TextState.
    valid(session.set_selection(Selection::caret(Position { element: 2, offset: 0 })));
    valid(session.insert("Revised: "));
    assert!(session.project().elements[2].text.starts_with("Revised: First half"));
    assert_flat_members(session.project(), &member_ids);

    // A valid Tab structural edit must retain the imported group and wrapper data.
    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: session.project().elements[1].text.len(),
    })));
    valid(session.tab());
    assert_eq!(session.project().elements[1].role, Role::Parenthetical);
    assert_flat_members(session.project(), &member_ids);
    let retained = session.project().clone();
    let retained_xml = serialized(&retained);
    assert_eq!(retained_xml.matches("SpellCheckIgnoreLists").count(), 4);
    assert!(retained_xml.contains("vendor-scope=\"group\""));

    // A role edit that breaks the side pattern unwraps atomically with that edit.
    valid(session.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(session.change_role(Role::Action));
    let unwrapped = session.project().clone();
    assert!(unwrapped.dual_dialogue_groups.is_empty());
    assert_eq!(unwrapped.elements[1].role, Role::Action);
    assert_eq!(unwrapped.elements[1..7].iter().map(|item| item.id.clone()).collect::<Vec<_>>(), member_ids);
    let unwrapped_xml = serialized(&unwrapped);
    assert_eq!(unwrapped_xml.matches("SpellCheckIgnoreLists").count(), 2);
    assert!(!unwrapped_xml.contains("vendor-scope=\"group\""));
    assert!(unwrapped_xml.contains("vendor-scope=\"document\""));
    assert!(unwrapped_xml.contains("<Word>colour</Word>"));

    // The composite unwrap/edit is one transaction; Undo restores nested scope,
    // and Redo removes only that scope again.
    valid(session.undo());
    assert_eq!(session.project(), &retained);
    assert_flat_members(session.project(), &member_ids);
    assert!(serialized(session.project()).contains("vendor-scope=\"group\""));
    valid(session.redo());
    assert_eq!(session.project(), &unwrapped);
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert!(serialized(session.project()).contains("vendor-scope=\"document\""));

    // Exercise the storage Save path, then reopen the exact published FDX.
    let path = TemporaryFdx::new();
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(path.0.clone()),
    ));
    let saved_bytes = valid(fs::read(&path.0));
    let reopened = valid(read_v6(&saved_bytes));
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 0);
    assert_eq!(reopened.project.elements[1..7].iter().map(|item| item.id.clone()).collect::<Vec<_>>(), member_ids);
    assert_eq!(reopened.project.elements[2].text, unwrapped.elements[2].text);
    let reopened_xml = String::from_utf8(saved_bytes)
        .unwrap_or_else(|_| unreachable!("saved FDX is UTF-8"));
    assert_eq!(reopened_xml.matches("SpellCheckIgnoreLists").count(), 2);
    assert!(reopened_xml.contains("vendor-scope=\"document\""));
    assert!(!reopened_xml.contains("vendor-scope=\"group\""));
    assert!(reopened_xml.contains("<Word>colour</Word>"));
}

#[test]
fn deleting_alts_owned_last_member_keeps_cue_only_layout_and_saved_group() {
    let imported = valid(read_v6(include_bytes!("fixtures/dialogue_alts.fdx")));
    let mut session = valid(EditingSession::new(imported.project));
    let initial_group = session.project().dual_dialogue_groups[0].clone();
    let right_cue = session.project().elements.iter().position(|element| element.id == initial_group.right.character);
    let right_dialogue = session.project().elements.iter().position(|element| initial_group.right.paragraphs.iter().any(|id| id == &element.id));
    let (Some(right_cue), Some(right_dialogue)) = (right_cue, right_dialogue) else {
        unreachable!("fixture has actual right cue and Alts-owning Dialogue")
    };
    valid(session.set_selection(Selection {
        anchor: Position { element: right_cue, offset: session.project().elements[right_cue].text.len() },
        focus: Position { element: right_dialogue, offset: session.project().elements[right_dialogue].text.len() },
    }));
    valid(session.delete());

    let project = session.project();
    assert_eq!(project.elements.len(), 3);
    assert!(project.dual_dialogue_groups.first().is_some_and(|group| group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()));
    let layout = valid(ScreenplayLayout::build_with_composition(
        project,
        &LayoutInputs::default(),
        &SourceLayoutSettings::default(),
        LayoutComposition::Print { include_title_page: false },
    ));
    assert_eq!(layout.active_group_id, None);
    assert!(layout.editor_group_boundaries.is_empty());
    let cue_lines = layout.pages.iter().flat_map(|page| &page.lines).filter(|line| matches!(line.element, 1 | 2)).collect::<Vec<_>>();
    assert_eq!(cue_lines.len(), 2);
    assert!(cue_lines.iter().all(|line| line.role == Role::Character));

    let path = TemporaryFdx::new();
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(path.0.clone()),
    ));
    let reopened = valid(read_v6(&valid(fs::read(&path.0))));
    assert_eq!(reopened.project.elements.len(), 3);
    assert!(reopened.project.dual_dialogue_groups.first().is_some_and(|group| group.member_ids().count() == 2 && group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()));
}
