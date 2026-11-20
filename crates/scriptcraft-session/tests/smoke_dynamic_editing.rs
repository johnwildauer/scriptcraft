#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::dual_dialogue::{
    active_dual_dialogue_projection, map_group_identity, SelectionSurface,
};
use scriptcraft_model::{
    ElementDefinition, ElementTypeKey, Position, Project, Selection, SemanticCapabilities,
};
use scriptcraft_session::editing::{
    EditingSession, ElementDefinitionCandidate, ElementTypeRemapping,
};
use scriptcraft_session::storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use std::fs;
use std::path::PathBuf;

const DYNAMIC_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/dynamic_editing.fdx");
const ACTIVE_DUAL_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/active_dual_dialogue.fdx");
const INACTIVE_DUAL_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/inactive_dual_dialogue.fdx");

#[track_caller]
fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!(
            "dynamic editing fixture operation failed at {}: {error:?}",
            std::panic::Location::caller()
        ),
    }
}

fn key(label: &str) -> ElementTypeKey {
    valid(ElementTypeKey::new(label))
}

fn foreign_body_index(project: &Project, foreign_id: &str) -> usize {
    project
        .elements
        .iter()
        .position(|owner| {
            owner
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some(foreign_id)
        })
        .unwrap_or_else(|| unreachable!("fixture body owner is managed"))
}

fn foreign_title_index(project: &Project, foreign_id: &str) -> usize {
    project
        .title_page
        .iter()
        .position(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some(foreign_id)
        })
        .unwrap_or_else(|| unreachable!("fixture title owner is managed"))
}

fn caret(element: usize, offset: usize) -> Selection {
    Selection::caret(Position { element, offset })
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-dynamic-editing-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path)
            .unwrap_or_else(|_| unreachable!("unique temporary directory can be created"));
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn dynamic_fixture_drives_exact_behavior_capabilities_and_body_operations() {
    let project = valid(read_v6(DYNAMIC_FIXTURE)).project;
    let ordered_keys = project
        .element_definitions()
        .iter()
        .map(|definition| definition.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        &ordered_keys[..4],
        ["Verse", "aside", "Lyrics", "scene heading"]
    );

    let verse = project
        .element_semantics(&key("Verse"))
        .unwrap_or_else(|| unreachable!("custom definition is registered"));
    assert_eq!(verse.capabilities(), SemanticCapabilities::default());
    assert_eq!(verse.definition().behavior.shortcut.as_deref(), Some(""));
    assert_eq!(
        verse.definition().behavior.win_shortcut.as_deref(),
        Some("Alt+V")
    );
    let aside = project
        .element_semantics(&key("aside"))
        .unwrap_or_else(|| unreachable!("case-distinct definition is registered"));
    assert_eq!(aside.capabilities(), SemanticCapabilities::default());
    assert_eq!(
        aside.definition().behavior.paginate_as.as_deref(),
        Some("Scene Heading")
    );
    assert!(project
        .element_semantics(&key("scene heading"))
        .is_some_and(|semantics| semantics.capabilities() == SemanticCapabilities::default()));
    assert_eq!(
        project.resolve_paragraph_behavior_target(
            &key("Verse"),
            scriptcraft_model::ParagraphBehaviorKey::Return
        ),
        Some(scriptcraft_model::BehaviorTargetResolution::Resolved(key(
            "Lyrics"
        )))
    );
    assert_eq!(
        project.resolve_paragraph_behavior_target(
            &key("Verse"),
            scriptcraft_model::ParagraphBehaviorKey::Tab
        ),
        Some(scriptcraft_model::BehaviorTargetResolution::Resolved(key(
            "aside"
        )))
    );
    assert_eq!(
        project.resolve_paragraph_behavior_target(
            &key("aside"),
            scriptcraft_model::ParagraphBehaviorKey::Return
        ),
        Some(scriptcraft_model::BehaviorTargetResolution::Unresolved(
            key("MissingTarget")
        ))
    );
    assert_eq!(
        project.resolve_paragraph_behavior_target(
            &key("aside"),
            scriptcraft_model::ParagraphBehaviorKey::Tab
        ),
        Some(scriptcraft_model::BehaviorTargetResolution::Resolved(key(
            "General"
        )))
    );
    assert_eq!(
        project
            .elements
            .iter()
            .filter(|owner| {
                project
                    .element_semantics(&owner.element_type())
                    .is_some_and(|semantics| semantics.capabilities().scene_boundary)
            })
            .count(),
        1,
        "PaginateAs and case-distinct labels do not create a second scene boundary"
    );
    assert_eq!(project.dual_dialogue_groups.len(), 1);
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_some());

    let mut toggle_session = valid(EditingSession::new(project.clone()));
    let left_cue = foreign_body_index(toggle_session.project(), "dynamic-dual-left-cue");
    valid(toggle_session.set_selection(caret(left_cue, 0)));
    assert!(valid(toggle_session.dual_dialogue_toggle_target(SelectionSurface::Body)).is_some());
    valid(toggle_session.toggle_dual_dialogue());
    assert!(toggle_session.project().dual_dialogue_groups.is_empty());
    valid(toggle_session.undo());
    assert!(active_dual_dialogue_projection(
        toggle_session.project(),
        &toggle_session.project().dual_dialogue_groups[0]
    )
    .is_some());

    let mut session = valid(EditingSession::new(project));
    let enter_index = foreign_body_index(session.project(), "dynamic-verse-enter");
    let original_id = session.project().elements[enter_index].id.clone();
    let original_text = session.project().elements[enter_index].text.clone();
    let original_runs = session.project().elements[enter_index].runs.clone();
    valid(session.set_selection(caret(enter_index, "Multi ".len())));
    valid(session.enter());
    assert_eq!(
        session.project().elements[enter_index].element_type(),
        key("Verse")
    );
    assert_eq!(
        session.project().elements[enter_index + 1].element_type(),
        key("Lyrics")
    );
    assert_eq!(session.project().elements[enter_index].text, "Multi ");
    assert_eq!(
        session.project().elements[enter_index + 1].text,
        "run — café"
    );
    valid(session.change_element_type(key("Verse")));
    let left_length = session.project().elements[enter_index].text.len();
    valid(session.set_selection(caret(enter_index, left_length)));
    valid(session.join_with_next());
    assert_eq!(session.project().elements[enter_index].id, original_id);
    assert_eq!(session.project().elements[enter_index].text, original_text);
    assert_eq!(session.project().elements[enter_index].runs, original_runs);

    let tab_index = foreign_body_index(session.project(), "dynamic-verse-tab");
    let tab_owner_id = session.project().elements[tab_index].id.clone();
    let tab_text = session.project().elements[tab_index].text.clone();
    let tab_runs = session.project().elements[tab_index].runs.clone();
    valid(session.set_selection(caret(tab_index, 0)));
    valid(session.tab());
    assert_eq!(
        session.project().elements[tab_index].element_type(),
        key("aside")
    );
    assert_eq!(session.project().elements[tab_index].id, tab_owner_id);
    assert_eq!(session.project().elements[tab_index].text, tab_text);
    assert_eq!(session.project().elements[tab_index].runs, tab_runs);

    let generated_index = foreign_body_index(session.project(), "dynamic-generated-parenthetical");
    valid(session.set_selection(caret(generated_index, 0)));
    valid(session.change_element_type(key("Character")));
    valid(session.tab());
    assert_eq!(session.project().elements[generated_index].text, "()");
    assert_eq!(
        session.project().elements[generated_index].element_type(),
        key("Parenthetical")
    );
}

#[test]
fn dynamic_save_rebase_keeps_eligible_generation_and_inactive_dual_source() {
    let directory = TestDirectory::new();
    let destination = directory.path("dynamic-editing.fdx");
    let project = valid(read_v6(DYNAMIC_FIXTURE)).project;
    let mut session = valid(EditingSession::new(project));
    let original_definitions = session
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let generated_index = foreign_body_index(session.project(), "dynamic-generated-parenthetical");
    valid(session.set_selection(caret(generated_index, 0)));
    valid(session.change_element_type(key("Character")));
    valid(session.tab());
    let generated_id = session.project().elements[generated_index].id.clone();

    let scene_cue = foreign_body_index(session.project(), "dynamic-scene-cue");
    valid(session.set_selection(Selection {
        anchor: Position {
            element: scene_cue,
            offset: 0,
        },
        focus: Position {
            element: scene_cue,
            offset: "ALICE".len(),
        },
    }));
    valid(session.insert("BOB"));
    assert!(session.pending_derived_work_summary().pending);

    let selected_body = foreign_body_index(session.project(), "dynamic-verse-enter");
    let body_selection = caret(selected_body, 3);
    let title_index = foreign_title_index(session.project(), "dynamic-title-verse");
    let title_selection = caret(title_index, 4);
    valid(session.set_selection(body_selection));
    valid(session.set_title_selection(title_selection));

    let initial_group_id = session.project().dual_dialogue_groups[0].id.clone();
    let initial_group_members = session.project().dual_dialogue_groups[0].members.clone();
    let revision_before_first_save = session.revision();
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    ));
    assert!(!session.dirty());
    assert_eq!(session.revision(), revision_before_first_save + 1);
    assert_eq!(session.selection(), body_selection);
    assert_eq!(session.title_selection(), title_selection);
    assert_eq!(
        session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        original_definitions
    );
    assert_eq!(
        session.project().elements[foreign_body_index(session.project(), "dynamic-cast-list")].text,
        "(BOB, LEFT, RIGHT)"
    );
    assert_eq!(
        session.project().dual_dialogue_groups[0].members,
        initial_group_members
    );
    assert_eq!(
        map_group_identity(
            &valid(read_v6(DYNAMIC_FIXTURE)).project,
            session.project(),
            &initial_group_id
        ),
        Some(session.project().dual_dialogue_groups[0].id.clone())
    );

    let saved_verse = foreign_body_index(session.project(), "dynamic-verse-enter");
    assert_eq!(
        session.project().elements[saved_verse].text,
        "Multi run — café"
    );
    assert_eq!(session.project().elements[saved_verse].runs.len(), 2);
    assert!(session
        .project()
        .direct_paragraph_overrides
        .contains_key(&session.project().elements[saved_verse].id));
    let written = fs::read(&destination)
        .unwrap_or_else(|_| unreachable!("the user-owned save scenario reads its own output"));
    let written =
        String::from_utf8(written).unwrap_or_else(|_| unreachable!("native FDX output is UTF-8"));
    assert!(written.contains("BodyAttachment"));
    assert!(written.contains("BodyOpaque"));
    assert!(written.contains("WrapperAttachment"));
    assert!(written.contains("MemberAttachment"));

    // The generated mark is session-only but survives an eligible save rebase.
    let generated_index = session
        .project()
        .elements
        .iter()
        .position(|owner| owner.id == generated_id)
        .unwrap_or_else(|| unreachable!("the generated owner keeps its stable identity"));
    valid(session.set_selection(caret(generated_index, 1)));
    valid(session.enter());
    assert_eq!(session.project().elements[generated_index].text, "");
    valid(session.undo());
    assert_eq!(session.project().elements[generated_index].text, "()");
    valid(session.set_selection(caret(generated_index, 0)));
    valid(session.change_element_type(key("Action")));
    assert_eq!(session.project().elements[generated_index].text, "()");
    valid(session.change_element_type(key("Parenthetical")));
    assert_eq!(session.project().elements[generated_index].text, "()");

    // Simulate a future registry remap through the generic candidate: source
    // membership remains, while losing the exact Character capability makes
    // the group inactive. This is not Apply Template policy.
    let mut definitions = session
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<ElementDefinition>>();
    let mut speaker_cue = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Character")
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture has the exact base Character definition"));
    speaker_cue.key = key("SpeakerCue");
    definitions.push(speaker_cue);
    let mut remap = ElementDefinitionCandidate::new(definitions);
    remap.body_remappings.push(ElementTypeRemapping::new(
        key("Character"),
        key("SpeakerCue"),
    ));
    valid(session.replace_element_definitions_and_remap_types(remap));
    let inactive_before_save = session.project().clone();
    let selection_before_inactive_save = session.selection();
    let title_selection_before_inactive_save = session.title_selection();
    let revision_before_inactive_save = session.revision();
    assert_eq!(
        session.project().dual_dialogue_groups[0].members,
        initial_group_members
    );
    assert!(active_dual_dialogue_projection(
        session.project(),
        &session.project().dual_dialogue_groups[0]
    )
    .is_none());

    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    assert!(!session.dirty());
    assert_eq!(session.revision(), revision_before_inactive_save + 1);
    assert_eq!(session.selection(), selection_before_inactive_save);
    assert_eq!(
        session.title_selection(),
        title_selection_before_inactive_save
    );
    assert!(active_dual_dialogue_projection(
        session.project(),
        &session.project().dual_dialogue_groups[0]
    )
    .is_none());
    assert_eq!(
        map_group_identity(
            &inactive_before_save,
            session.project(),
            &inactive_before_save.dual_dialogue_groups[0].id
        ),
        Some(session.project().dual_dialogue_groups[0].id.clone())
    );

    let reopened = valid(open(&StdFdxStorageFileSystem, &destination));
    let reopened_project = reopened.document.project;
    assert_eq!(&reopened_project, session.project());
    assert_eq!(
        reopened_project.element_definitions(),
        session.project().element_definitions()
    );
    assert_eq!(
        reopened_project.dual_dialogue_groups[0].members,
        initial_group_members
    );
    assert!(active_dual_dialogue_projection(
        &reopened_project,
        &reopened_project.dual_dialogue_groups[0]
    )
    .is_none());
    let mut reopened_session = valid(EditingSession::new(reopened_project));
    assert!(!reopened_session.dirty());
    let reopened_generated = foreign_body_index(
        reopened_session.project(),
        "dynamic-generated-parenthetical",
    );
    valid(reopened_session.set_selection(caret(reopened_generated, 1)));
    valid(reopened_session.enter());
    assert_eq!(
        reopened_session.project().elements[reopened_generated].text,
        "()"
    );

    let mut restored_definitions = reopened_session
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    restored_definitions.retain(|definition| definition.key.as_str() != "SpeakerCue");
    let mut restore = ElementDefinitionCandidate::new(restored_definitions);
    restore.body_remappings.push(ElementTypeRemapping::new(
        key("SpeakerCue"),
        key("Character"),
    ));
    valid(reopened_session.replace_element_definitions_and_remap_types(restore));
    assert!(active_dual_dialogue_projection(
        reopened_session.project(),
        &reopened_session.project().dual_dialogue_groups[0]
    )
    .is_some());
    valid(reopened_session.undo());
    assert!(active_dual_dialogue_projection(
        reopened_session.project(),
        &reopened_session.project().dual_dialogue_groups[0]
    )
    .is_none());
    valid(reopened_session.redo());
    assert!(active_dual_dialogue_projection(
        reopened_session.project(),
        &reopened_session.project().dual_dialogue_groups[0]
    )
    .is_some());
}

#[test]
fn real_template_apply_preserves_inactive_and_active_dual_projections_through_history() {
    let target_document = valid(scriptcraft_fdx::instantiate_builtin_template(
        scriptcraft_fdx::BuiltinTemplateId::Screenplay,
    ));
    let target = valid(scriptcraft_fdx::extract_element_template_settings(
        &target_document,
    ));

    let inactive_project = valid(read_v6(INACTIVE_DUAL_FIXTURE)).project;
    let inactive_group = inactive_project.dual_dialogue_groups[0].clone();
    let inactive_original = inactive_project.clone();
    let inactive_members = inactive_group
        .members
        .iter()
        .map(|id| {
            let owner = inactive_project
                .elements
                .iter()
                .find(|owner| &owner.id == id)
                .unwrap_or_else(|| unreachable!("every source member has one managed owner"));
            (
                owner.id.clone(),
                owner.text.clone(),
                owner.as_element().runs.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert!(active_dual_dialogue_projection(&inactive_project, &inactive_group).is_none());

    let mut inactive = valid(EditingSession::new(inactive_project));
    assert_eq!(
        valid(inactive.apply_element_template_settings(&target)),
        scriptcraft_session::editing::ApplyTemplateOutcome::Changed
    );
    let inactive_after = &inactive.project().dual_dialogue_groups[0];
    assert_eq!(
        inactive_after, &inactive_group,
        "wrapper provenance and ordered source membership are retained"
    );
    assert!(active_dual_dialogue_projection(inactive.project(), inactive_after).is_none());
    for (index, member_id) in inactive_after.members.iter().enumerate() {
        let owner = inactive
            .project()
            .elements
            .iter()
            .find(|owner| &owner.id == member_id)
            .unwrap_or_else(|| unreachable!("Apply does not remove a source member"));
        assert_eq!(owner.element_type(), key("Action"));
        assert_eq!(owner.id, inactive_members[index].0);
        assert_eq!(owner.text, inactive_members[index].1);
        assert_eq!(owner.as_element().runs, inactive_members[index].2);
    }
    valid(inactive.undo());
    assert_eq!(inactive.project(), &inactive_original);
    assert!(active_dual_dialogue_projection(
        inactive.project(),
        &inactive.project().dual_dialogue_groups[0]
    )
    .is_none());
    valid(inactive.redo());
    assert_eq!(&inactive.project().dual_dialogue_groups[0], &inactive_group);
    assert!(active_dual_dialogue_projection(
        inactive.project(),
        &inactive.project().dual_dialogue_groups[0]
    )
    .is_none());

    let active_project = valid(read_v6(ACTIVE_DUAL_FIXTURE)).project;
    let active_group = active_project.dual_dialogue_groups[0].clone();
    let active_original = active_project.clone();
    assert!(active_dual_dialogue_projection(&active_project, &active_group).is_some());
    let mut active = valid(EditingSession::new(active_project));
    assert_eq!(
        valid(active.apply_element_template_settings(&target)),
        scriptcraft_session::editing::ApplyTemplateOutcome::Changed
    );
    assert_eq!(active.project().dual_dialogue_groups[0], active_group);
    assert!(
        active_dual_dialogue_projection(
            active.project(),
            &active.project().dual_dialogue_groups[0]
        )
        .is_some(),
        "the built-in retains exact base-type dual capabilities"
    );
    valid(active.undo());
    assert_eq!(active.project(), &active_original);
    assert!(active_dual_dialogue_projection(
        active.project(),
        &active.project().dual_dialogue_groups[0]
    )
    .is_some());
    valid(active.redo());
    assert!(active_dual_dialogue_projection(
        active.project(),
        &active.project().dual_dialogue_groups[0]
    )
    .is_some());
}
