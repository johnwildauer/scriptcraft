#![forbid(unsafe_code)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, FdxDocument};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{
    AnchorTarget, PreservedElement, PreservedNode, SourceOwnership,
};

const FIXTURE: &str = include_str!("fixtures/dual_dialogue_basic.fdx");
const EMPTY_SIDES_FIXTURE: &str = include_str!("fixtures/dual_dialogue_empty_sides.fdx");

fn import_fixture() -> Option<FdxDocument> {
    let result = read_v6(FIXTURE.as_bytes());
    assert!(result.is_ok(), "the authored dual-dialogue fixture is importable");
    result.ok()
}

fn count_named(nodes: &[PreservedNode], name: &str) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            PreservedNode::Element(element) => {
                usize::from(element.name.local_name == name) + count_named(&element.children, name)
            }
            _ => 0,
        })
        .sum()
}

fn find_scope<'a>(nodes: &'a [PreservedNode], scope: &str) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if element.name.local_name == "SpellCheckIgnoreLists"
            && element.attributes.iter().any(|attribute| {
                attribute.name.local_name == "vendor-scope" && attribute.value == scope
            })
        {
            return Some(element);
        }
        if let Some(found) = find_scope(&element.children, scope) {
            return Some(found);
        }
    }
    None
}

fn find_managed<'a>(nodes: &'a [PreservedNode], id: &scriptcraft_core::StableId) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
            return Some(element);
        }
        if let Some(found) = find_managed(&element.children, id) {
            return Some(found);
        }
    }
    None
}

fn unique_directory() -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("scriptcraft-dual-dialogue-{nonce}"))
}

#[test]
fn source_backed_save_save_as_repeated_saves_and_failed_publish_preserve_groups() {
    let Some(imported) = import_fixture() else {
        return;
    };
    let session_result = EditingSession::new(imported.project);
    assert!(session_result.is_ok(), "the imported dual group forms an editing session");
    let Ok(mut session) = session_result else {
        return;
    };
    let source_before = session.project().preservation.as_ref();
    assert!(source_before.is_some(), "fixture project retains its preservation tree");
    let Some(source_before) = source_before else {
        return;
    };
    let unchanged_action_ids = session
        .project()
        .elements
        .iter()
        .filter(|element| element.role == scriptcraft_core::Role::Action)
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let unchanged_actions = unchanged_action_ids
        .iter()
        .filter_map(|id| find_managed(&source_before.sections[0].children, id).cloned())
        .collect::<Vec<_>>();
    let original_document_spell = find_scope(&source_before.sections[0].children, "document").cloned();
    let original_group_spell = find_scope(&source_before.sections[0].children, "group").cloned();
    let directory = unique_directory();
    let created = std::fs::create_dir(&directory);
    assert!(created.is_ok(), "the isolated storage directory is created");
    if created.is_err() {
        return;
    }
    let first_path = directory.join("dual-first.fdx");
    let second_path = directory.join("dual-second.fdx");
    let conflict_path = directory.join("existing.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    let first_save = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(first_path.clone()),
    );
    assert!(first_save.is_ok(), "Save As preserves and rebases an imported group");
    if first_save.is_err() {
        let _ = std::fs::remove_dir_all(&directory);
        return;
    }
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(target.path(), Some(first_path.as_path()));
    let saved_group = session.project().dual_dialogue_groups[0].clone();
    let saved_member_ids = session.project().elements[1..7]
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();

    let text_end = session.project().elements[2].text.len();
    let selected = session.set_selection(Selection::caret(Position {
        element: 2,
        offset: text_end,
    }));
    assert!(selected.is_ok(), "a nested member caret is valid");
    let inserted = session.insert(" 再見✨");
    assert!(inserted.is_ok(), "ordinary text edits remain scoped inside a nested member");
    let first_update = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    );
    assert!(first_update.is_ok(), "Save reconciles the nested member and wrapper");
    let repeated_save = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    );
    assert!(repeated_save.is_ok(), "repeated Save does not duplicate wrapper-owned nodes");
    let save_as_again = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(second_path.clone()),
    );
    assert!(save_as_again.is_ok(), "repeated Save As re-reads the grouped source");
    if first_update.is_err() || repeated_save.is_err() || save_as_again.is_err() {
        let _ = std::fs::remove_dir_all(&directory);
        return;
    }

    let reopened_result = open(&StdFdxStorageFileSystem, &second_path);
    assert!(reopened_result.is_ok(), "Save As output opens through the strict FDX reader");
    let Ok(reopened) = reopened_result else {
        let _ = std::fs::remove_dir_all(&directory);
        return;
    };
    let project = &reopened.document.project;
    assert_eq!(project.elements[2].text, "First half—then the rest. 再見✨");
    assert_eq!(project.dual_dialogue_groups.len(), 1);
    assert_eq!(project.elements[1..7].iter().map(|element| element.id.clone()).collect::<Vec<_>>(), saved_member_ids);
    assert_eq!(project.dual_dialogue_groups[0].wrapper.outer_foreign_id, saved_group.wrapper.outer_foreign_id);
    let Some(preservation) = project.preservation.as_ref() else {
        assert!(false, "reopened Save As output is source-backed");
        let _ = std::fs::remove_dir_all(&directory);
        return;
    };
    assert_eq!(count_named(&preservation.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&preservation.sections[0].children, "SpellCheckIgnoreLists"), 2);
    assert!(find_scope(&preservation.sections[0].children, "group").is_some());
    assert!(find_scope(&preservation.sections[0].children, "document").is_some());
    assert_eq!(
        find_scope(&preservation.sections[0].children, "document"),
        original_document_spell.as_ref()
    );
    assert_eq!(
        find_scope(&preservation.sections[0].children, "group"),
        original_group_spell.as_ref()
    );
    let reopened_actions = unchanged_action_ids
        .iter()
        .filter_map(|id| find_managed(&preservation.sections[0].children, id))
        .collect::<Vec<_>>();
    assert_eq!(reopened_actions, unchanged_actions.iter().collect::<Vec<_>>());
    let group = &project.dual_dialogue_groups[0];
    for member_id in group.member_ids() {
        assert!(preservation.anchors.iter().any(|anchor| {
            anchor.target == AnchorTarget::ProjectElement(member_id.clone())
        }));
    }
    assert!(preservation.anchors.iter().any(|anchor| {
        anchor.target == AnchorTarget::DualDialogueWrapper(group.id.clone())
    }));
    let paragraph_ids = preservation
        .identities
        .iter()
        .filter(|identity| identity.domain == scriptcraft_core::preservation::ForeignIdDomain::Paragraph)
        .map(|identity| identity.value.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(paragraph_ids.len(), preservation.identities.iter().filter(|identity| {
        identity.domain == scriptcraft_core::preservation::ForeignIdDomain::Paragraph
    }).count());

    let sentinel = std::fs::write(&conflict_path, b"do not replace");
    assert!(sentinel.is_ok(), "the conflict fixture is installed");
    let before_session = session.clone();
    let before_target = target.clone();
    let failed_save_as = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(conflict_path.clone()),
    );
    assert!(failed_save_as.is_err(), "Save As refuses an existing destination without consent");
    assert_eq!(session, before_session);
    assert_eq!(target, before_target);
    let destination_bytes = std::fs::read(&conflict_path);
    assert_eq!(destination_bytes.ok().as_deref(), Some(&b"do not replace"[..]));
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn save_reopen_after_disable_keeps_document_spell_scope_and_flat_member_ids() {
    let Some(imported) = import_fixture() else { return };
    let member_ids = imported.project.elements[1..7]
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    assert!(session.set_selection(Selection::caret(Position { element: 3, offset: 4 })).is_ok());
    assert!(session.toggle_dual_dialogue().is_ok());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert_eq!(session.project().elements[1..7].iter().map(|element| element.id.clone()).collect::<Vec<_>>(), member_ids);

    let directory = unique_directory();
    if std::fs::create_dir(&directory).is_err() { return; }
    let path = directory.join("disabled.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    let saved = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(path.clone()),
    );
    assert!(saved.is_ok());
    let reopened = open(&StdFdxStorageFileSystem, &path);
    assert!(reopened.is_ok());
    if let Ok(reopened) = reopened {
        assert!(reopened.document.project.dual_dialogue_groups.is_empty());
        assert_eq!(reopened.document.project.elements[1..7].iter().map(|element| element.id.clone()).collect::<Vec<_>>(), member_ids);
        let Some(preservation) = reopened.document.project.preservation.as_ref() else {
            assert!(false, "disabled source-backed output retains preservation state");
            let _ = std::fs::remove_dir_all(&directory);
            return;
        };
        assert_eq!(count_named(&preservation.sections[0].children, "DualDialogue"), 0);
        assert_eq!(count_named(&preservation.sections[0].children, "SpellCheckIgnoreLists"), 1);
        assert!(find_scope(&preservation.sections[0].children, "document").is_some());
        assert!(find_scope(&preservation.sections[0].children, "group").is_none());
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn repeated_save_and_save_as_rebase_group_identity_overrides_and_each_member_caret() {
    let Some(imported) = import_fixture() else { return };
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    let original = session.project().clone();
    let original_group_id = original.dual_dialogue_groups[0].id.clone();
    let directory = unique_directory();
    if std::fs::create_dir(&directory).is_err() { return; }
    let first_path = directory.join("session-first.fdx");
    let second_path = directory.join("session-second.fdx");
    let mut target = FdxSaveTarget::new_unsaved();

    for (ordinal, index) in (1..7).enumerate() {
        let text = &session.project().elements[index].text;
        let offset = if ordinal % 2 == 0 { 0 } else { text.len() };
        let selection = Selection::caret(Position { element: index, offset });
        assert!(session.set_selection(selection).is_ok());
        let before_rebase = session.project().clone();
        let old_group = session.project().dual_dialogue_groups[0].id.clone();
        let mode = if ordinal == 0 {
            FdxSaveMode::SaveAs(first_path.clone())
        } else {
            FdxSaveMode::Save
        };
        let saved = save_session(&StdFdxStorageFileSystem, &mut session, &mut target, mode);
        assert!(saved.is_ok(), "member selection {index} can be saved and rebased");
        if saved.is_err() {
            let _ = std::fs::remove_dir_all(&directory);
            return;
        }
        assert_eq!(session.selection(), selection);
        let rebased_group = &session.project().dual_dialogue_groups[0];
        assert_eq!(
            scriptcraft_core::dual_dialogue::map_group_identity(
                &before_rebase,
                session.project(),
                &old_group,
            ),
            Some(rebased_group.id.clone()),
        );
        assert_eq!(
            rebased_group.wrapper.outer_foreign_id,
            original.dual_dialogue_groups[0].wrapper.outer_foreign_id,
        );
        let _ = &original_group_id;
    }

    let group = &session.project().dual_dialogue_groups[0];
    assert_eq!(
        session.project().dual_dialogue_paragraph_overrides.get(&group.id),
        original.dual_dialogue_paragraph_overrides.get(&original_group_id),
    );
    for (before, after) in original.elements[1..7].iter().zip(&session.project().elements[1..7]) {
        assert_eq!(
            original.direct_paragraph_overrides.get(&before.id),
            session.project().direct_paragraph_overrides.get(&after.id),
        );
    }

    let save_as = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(second_path.clone()),
    );
    assert!(save_as.is_ok(), "Save As can rebase a previously saved group again");
    if save_as.is_ok() {
        let reopened = open(&StdFdxStorageFileSystem, &second_path);
        assert!(reopened.is_ok(), "the rebased Save As candidate reopens");
        if let Ok(reopened) = reopened {
            assert_eq!(reopened.document.text_state.selection, session.selection());
            assert_eq!(reopened.document.project.dual_dialogue_groups.len(), 1);
        }
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn cue_only_group_survives_save_rebase_reopen_save_as_and_document_replacement() {
    let imported = read_v6(EMPTY_SIDES_FIXTURE.as_bytes());
    assert!(imported.is_ok(), "the source-backed empty-sided fixture imports");
    let Some(imported) = imported.ok() else { return };
    let original = imported.project;
    let group = original.dual_dialogue_groups.iter().find(|group| {
        group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()
    }).cloned();
    assert!(group.is_some(), "a C,C source group is available");
    let Some(group) = group else { return };
    let members = group.member_ids().cloned().collect::<Vec<_>>();
    let session_result = EditingSession::new(original.clone());
    assert!(session_result.is_ok());
    let Ok(mut session) = session_result else { return };

    let directory = unique_directory();
    let created = std::fs::create_dir(&directory);
    assert!(created.is_ok());
    if created.is_err() { return; }
    let first_path = directory.join("cue-only-save.fdx");
    let second_path = directory.join("cue-only-save-as.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    let save_as = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(first_path.clone()),
    );
    assert!(save_as.is_ok(), "Save As rebases a two-cue semantic group");
    if save_as.is_err() {
        let _ = std::fs::remove_dir_all(&directory);
        return;
    }
    assert_eq!(session.project().dual_dialogue_groups.len(), original.dual_dialogue_groups.len());
    let rebased = session.project().dual_dialogue_groups.iter().find(|candidate| {
        candidate.member_ids().map(|id| {
            session.project().elements.iter().find(|element| &element.id == id)
                .and_then(|element| element.source_provenance.as_ref())
                .and_then(|source| source.foreign_id.as_deref())
        }).collect::<Vec<_>>() == members.iter().map(|id| {
            original.elements.iter().find(|element| &element.id == id)
                .and_then(|element| element.source_provenance.as_ref())
                .and_then(|source| source.foreign_id.as_deref())
        }).collect::<Vec<_>>()
    });
    assert!(rebased.is_some());
    if let Some(rebased) = rebased {
        assert_eq!(scriptcraft_core::dual_dialogue::map_group_identity(
            &original, session.project(), &group.id,
        ), Some(rebased.id.clone()));
        assert!(rebased.left.paragraphs.is_empty() && rebased.right.paragraphs.is_empty());
    }

    let saved = save_session(&StdFdxStorageFileSystem, &mut session, &mut target, FdxSaveMode::Save);
    assert!(saved.is_ok(), "ordinary Save retains the rebased cue-only group");
    let reopened = open(&StdFdxStorageFileSystem, &first_path);
    assert!(reopened.is_ok());
    if let Ok(reopened) = reopened {
        let saved_project = reopened.document.project;
        let saved_group = saved_project.dual_dialogue_groups.iter().find(|candidate| {
            candidate.member_ids().count() == 2
                && candidate.member_ids().all(|id| saved_project.elements.iter().any(|element| &element.id == id))
                && candidate.left.paragraphs.is_empty()
                && candidate.right.paragraphs.is_empty()
        });
        assert!(saved_group.is_some());
        assert!(session.replace_with_saved(saved_project.clone()).is_ok());
        assert_eq!(session.project().dual_dialogue_groups.len(), saved_project.dual_dialogue_groups.len());
        assert!(session.project().dual_dialogue_groups.iter().any(|candidate| {
            candidate.member_ids().count() == 2
                && candidate.member_ids().all(|id| saved_project.elements.iter().any(|element| &element.id == id))
                && candidate.left.paragraphs.is_empty()
                && candidate.right.paragraphs.is_empty()
        }));
    }

    let save_as_again = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(second_path.clone()),
    );
    assert!(save_as_again.is_ok(), "document replacement does not stale the empty group before Save As");
    let reopened_as = open(&StdFdxStorageFileSystem, &second_path);
    assert!(reopened_as.is_ok());
    if let Ok(reopened_as) = reopened_as {
        assert!(reopened_as.document.project.dual_dialogue_groups.iter().any(|candidate| {
            candidate.member_ids().count() == 2
                && candidate.left.paragraphs.is_empty()
                && candidate.right.paragraphs.is_empty()
        }));
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn derived_refresh_counts_flat_dual_cues_once_and_keeps_both_spell_scopes() {
    let source = FIXTURE.replace(
        "  <Content>",
        "  <Content>\n    <Paragraph Type=\"Scene Heading\" id=\"scene-derived-001\"><Text>INT. ROOM - DAY</Text><SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\"><SceneArcBeats/></SceneProperties></Paragraph>\n    <Paragraph Type=\"Cast List\" id=\"cast-derived-001\"><Text>(STALE)</Text></Paragraph>",
    );
    let imported = read_v6(source.as_bytes());
    assert!(imported.is_ok(), "the scene-plus-dual source is valid for derived refresh");
    let Ok(imported) = imported else { return };
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    let before_project = session.project().clone();
    let group_before = before_project.dual_dialogue_groups[0].clone();
    let preservation_before = before_project.preservation.as_ref();
    assert!(preservation_before.is_some());
    let Some(preservation_before) = preservation_before else { return };
    let document_spell_before = find_scope(&preservation_before.sections[0].children, "document").cloned();
    let group_spell_before = find_scope(&preservation_before.sections[0].children, "group").cloned();
    let revision_before = session.revision();

    let correction = session.refresh_cast_lists_with_outcome();
    assert!(correction.is_ok(), "the pending complete derived refresh is transactional");
    let Ok(correction) = correction else { return };
    assert!(correction.changed);
    assert_eq!(session.revision(), revision_before + 1);
    assert!(!session.can_undo(), "derived correction does not create an edit-history entry");
    assert_eq!(session.project().dual_dialogue_groups, vec![group_before]);
    assert_eq!(
        session.project().elements.iter().filter(|element| element.role == scriptcraft_core::Role::Character).count(),
        2,
        "both flat dual Character cues participate once and no outer General enters the body",
    );
    let cast_list = session.project().elements.iter()
        .find(|element| element.role == scriptcraft_core::Role::CastList);
    assert_eq!(cast_list.map(|element| element.text.as_str()), Some("(MARA, NOAH)"));

    let Some(preservation) = session.project().preservation.as_ref() else {
        assert!(false, "derived refresh retains the source tree");
        return;
    };
    assert_eq!(find_scope(&preservation.sections[0].children, "document"), document_spell_before.as_ref());
    assert_eq!(find_scope(&preservation.sections[0].children, "group"), group_spell_before.as_ref());
    assert_eq!(count_named(&preservation.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&preservation.sections[0].children, "CharacterArcBeat"), 2);
}

#[test]
fn undo_redo_restore_the_complete_group_wrapper_source_and_selection_snapshot() {
    let Some(imported) = import_fixture() else { return };
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    let selection = Selection::caret(Position { element: 4, offset: 0 });
    assert!(session.set_selection(selection).is_ok());
    let complete_grouped_project = session.project().clone();

    assert!(session.toggle_dual_dialogue().is_ok());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &complete_grouped_project);
    assert_eq!(session.selection(), selection);

    assert!(session.redo().is_ok());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &complete_grouped_project);
    assert_eq!(session.selection(), selection);
}

#[test]
fn deferred_scene_refresh_leaves_group_source_selection_revision_and_history_unchanged() {
    let source = FIXTURE.replace(
        "  <Content>",
        "  <Content>\n    <Paragraph Type=\"Scene Heading\" id=\"deferred-scene-001\"><Text>INT. ROOM - DAY</Text><SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\" vendor-scene=\"preserve\"><SceneArcBeats/></SceneProperties></Paragraph>\n    <Paragraph Type=\"Cast List\" id=\"deferred-cast-001\"><Text>(STALE)</Text></Paragraph>",
    );
    let imported = read_v6(source.as_bytes());
    assert!(imported.is_ok(), "unsupported derived metadata remains importable source");
    let Ok(imported) = imported else { return };
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    let selection = Selection::caret(Position { element: 4, offset: 0 });
    assert!(session.set_selection(selection).is_ok());
    let project_before = session.project().clone();
    let revision_before = session.revision();
    let dirty_before = session.dirty();
    let history_before = (session.can_undo(), session.can_redo());

    let outcome = session.refresh_cast_lists_with_outcome();

    assert!(outcome.is_ok());
    if let Ok(outcome) = outcome {
        assert!(outcome.deferred);
        assert_eq!(session.project(), &project_before);
        assert_eq!(session.selection(), selection);
        assert_eq!(session.revision(), revision_before);
        assert_eq!(session.dirty(), dirty_before);
        assert_eq!((session.can_undo(), session.can_redo()), history_before);
    }
}
