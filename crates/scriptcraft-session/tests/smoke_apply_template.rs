#![forbid(unsafe_code)]

use scriptcraft_fdx::{
    extract_element_template_settings, instantiate_builtin_template, BuiltinTemplateId,
};
use scriptcraft_model::dual_dialogue::active_dual_dialogue_projection;
use scriptcraft_model::preservation::PreservedNode;
use scriptcraft_model::{ElementTypeKey, Position, Project, ScreenplayElement, Selection};
use scriptcraft_session::editing::{ApplyTemplateOutcome, EditingSession};
use scriptcraft_session::storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use std::fs;
use std::path::PathBuf;

const CURRENT_FIXTURE: &[u8] = include_bytes!("fixtures/apply_template_current.fdx");

#[track_caller]
fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("real Apply regression input failed: {error:?}"))
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
        .unwrap_or_else(|| unreachable!("fixture body paragraph is managed by its foreign ID"))
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
        .unwrap_or_else(|| unreachable!("fixture title paragraph is managed by its foreign ID"))
}

fn unrelated_non_definition_root_nodes(project: &Project) -> Vec<PreservedNode> {
    let preservation = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("the Apply regression uses a source-backed document"));
    let root = preservation
        .sections
        .iter()
        .flat_map(|section| section.children.iter())
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("the retained document has one FinalDraft root"));
    root.children
        .iter()
        .filter(|node| match node {
            PreservedNode::Element(element) => !matches!(
                element.name.local_name.as_str(),
                "ElementSettings" | "Content" | "TitlePage"
            ),
            PreservedNode::Text(_) => false,
            PreservedNode::CData(_)
            | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => true,
        })
        .cloned()
        .collect()
}

fn direct_root_definition_keys(project: &Project) -> Vec<String> {
    let preservation = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("the Apply regression uses a source-backed document"));
    let root = preservation
        .sections
        .iter()
        .flat_map(|section| section.children.iter())
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("the retained document has one FinalDraft root"));
    root.children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "ElementSettings" => {
                element.source_type.clone()
            }
            _ => None,
        })
        .collect()
}

fn expected_body_after_screenplay_apply(
    project: &Project,
    target_keys: &[String],
) -> Vec<ScreenplayElement> {
    project
        .elements
        .iter()
        .map(|owner| {
            let mut element = owner.as_element().clone();
            if !target_keys
                .iter()
                .any(|target_key| target_key == element.element_type.as_str())
            {
                element.set_element_type(key("Action"));
                if let Some(provenance) = &mut element.source_provenance {
                    provenance.foreign_role = Some("Action".to_owned());
                }
            }
            element
        })
        .collect()
}

fn project_body(project: &Project) -> Vec<ScreenplayElement> {
    project
        .elements
        .iter()
        .map(|owner| owner.as_element().clone())
        .collect()
}

fn normalize_settings_anchor(node: &mut scriptcraft_model::preservation::PreservedElement) {
    node.anchor = None;
}

fn assert_applied_definition_source_matches_target(
    project: &Project,
    target: &scriptcraft_fdx::ElementTemplateSettings,
) {
    let preservation = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("Apply preserves source-backed settings"));
    assert_eq!(
        project.element_definitions(),
        target.definitions(),
        "the ordered complete registry must be the normalized Screenplay registry"
    );
    assert_eq!(
        direct_root_definition_keys(project),
        target
            .definitions()
            .iter()
            .map(|definition| definition.key.as_str().to_owned())
            .collect::<Vec<_>>(),
        "all direct-root winners are in exact target order with no obsolete or inert duplicates"
    );
    assert_eq!(target.definitions().len(), target.retained_settings().len());

    for (definition, expected_node) in target.definitions().iter().zip(target.retained_settings()) {
        let actual_node = preservation
            .element_definition_settings(&definition.key)
            .unwrap_or_else(|| {
                unreachable!("every normalized definition has one winning source node")
            });
        assert_eq!(
            actual_node.source_type.as_deref(),
            Some(definition.key.as_str())
        );
        let mut expected_node = expected_node.clone();
        let mut actual_node = actual_node.clone();
        normalize_settings_anchor(&mut expected_node);
        normalize_settings_anchor(&mut actual_node);
        assert_eq!(
            actual_node, expected_node,
            "retained winner payload and values follow the real built-in extraction"
        );
    }
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-apply-template-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path)
            .unwrap_or_else(|_| unreachable!("unique Apply test directory can be created"));
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn real_builtin_apply_is_settings_only_undoable_and_roundtrips_current_document() {
    let directory = TempDirectory::new();
    let current_path = directory.file("current-musical-document.fdx");
    fs::write(&current_path, CURRENT_FIXTURE)
        .unwrap_or_else(|_| unreachable!("sanitized current fixture can be written"));

    let opened = valid(open(&StdFdxStorageFileSystem, &current_path));
    let current_before = opened.document.project;
    let initial_source_bytes = CURRENT_FIXTURE.to_vec();
    let target_baseline = opened.baseline.clone();
    let target_path = current_path.clone();
    let musical_lyrics_definition = current_before
        .element_definitions()
        .get_by_label("Lyrics")
        .unwrap_or_else(|| unreachable!("the musical-style current source defines Lyrics"));
    assert_eq!(musical_lyrics_definition.font.family, "Times New Roman");
    let source_identities_before = current_before
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("the current document retains a preservation graph"))
        .identities
        .clone();
    let source_references_before = current_before
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("the current document retains a preservation graph"))
        .references
        .clone();
    let source_board_graph_before = current_before
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("the current document retains a preservation graph"))
        .board_graph
        .clone();
    let non_definition_settings_before = unrelated_non_definition_root_nodes(&current_before);
    let direct_body_formatting_before = current_before.direct_paragraph_overrides.clone();
    let direct_wrapper_formatting_before = current_before.dual_dialogue_paragraph_overrides.clone();
    let active_left_id = current_before.elements
        [foreign_body_index(&current_before, "apply-active-left-cue")]
    .id
    .clone();
    let inactive_left_id = current_before.elements
        [foreign_body_index(&current_before, "apply-inactive-member-lyrics")]
    .id
    .clone();
    let active_group_before = current_before
        .dual_dialogue_groups
        .iter()
        .find(|group| group.members.iter().any(|id| id == &active_left_id))
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture has an active source dual group"));
    let inactive_group_before = current_before
        .dual_dialogue_groups
        .iter()
        .find(|group| group.members.iter().any(|id| id == &inactive_left_id))
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture has a pre-inactive source dual group"));
    assert!(active_dual_dialogue_projection(&current_before, &active_group_before).is_some());
    assert!(active_dual_dialogue_projection(&current_before, &inactive_group_before).is_none());

    let expected_target_document =
        valid(instantiate_builtin_template(BuiltinTemplateId::Screenplay));
    let expected_target = valid(extract_element_template_settings(&expected_target_document));
    let target_keys = expected_target
        .definitions()
        .iter()
        .map(|definition| definition.key.as_str().to_owned())
        .collect::<Vec<_>>();
    let expected_body = expected_body_after_screenplay_apply(&current_before, &target_keys);
    let mut expected_title = current_before.title_page.clone();
    for paragraph in &mut expected_title {
        if paragraph.element_type.as_ref().is_some_and(|element_type| {
            !target_keys
                .iter()
                .any(|target_key| target_key == element_type.as_str())
        }) {
            paragraph.element_type = Some(key("Action"));
            if let Some(provenance) = &mut paragraph.source_provenance {
                provenance.foreign_role = Some("Action".to_owned());
            }
        }
    }

    let mut session = valid(EditingSession::new(current_before.clone()));
    let redo_seed = foreign_body_index(session.project(), "apply-body-lyrics");
    valid(session.set_selection(Selection::caret(Position {
        element: redo_seed,
        offset: 0,
    })));
    valid(session.insert("temporary redo branch"));
    valid(session.undo());
    assert!(
        session.can_redo(),
        "the baseline includes an ordinary redo branch"
    );
    let body_selection = Selection::caret(Position {
        element: foreign_body_index(session.project(), "apply-body-lyrics"),
        offset: "  First ".len(),
    });
    let title_selection = Selection::caret(Position {
        element: foreign_title_index(session.project(), "apply-title-lyrics"),
        offset: "Musical ".len(),
    });
    valid(session.set_selection(body_selection));
    valid(session.set_title_selection(title_selection));
    let body_typing_style = session.body_typing_style();
    let title_typing_style = session.title_typing_style();
    let revision_before_apply = session.revision();
    let mutation_token_before_apply = session.mutation_token();
    let target = FdxSaveTarget::from_baseline(target_baseline.clone());
    let target_before_apply = target.clone();

    assert_eq!(
        valid(session.apply_builtin_template(BuiltinTemplateId::Screenplay)),
        ApplyTemplateOutcome::Changed
    );

    assert_eq!(session.revision(), revision_before_apply + 1);
    assert_eq!(session.mutation_token(), mutation_token_before_apply + 1);
    assert!(
        session.dirty(),
        "Apply is one ordinary edit against the unchanged clean baseline"
    );
    assert!(session.can_undo());
    assert!(
        !session.can_redo(),
        "the accepted candidate clears an existing redo branch"
    );
    assert_eq!(session.selection(), body_selection);
    assert_eq!(session.title_selection(), title_selection);
    assert_eq!(session.body_typing_style(), body_typing_style);
    assert_eq!(session.title_typing_style(), title_typing_style);
    assert!(session.pending_derived_work_summary().complete);
    assert_eq!(
        target, target_before_apply,
        "Apply cannot change path or conflict baseline"
    );
    assert_eq!(target.path(), Some(target_path.as_path()));
    assert_eq!(target.baseline(), Some(&target_baseline));
    assert_eq!(
        fs::read(&current_path).ok().as_deref(),
        Some(initial_source_bytes.as_slice())
    );

    assert_applied_definition_source_matches_target(session.project(), &expected_target);
    assert_eq!(project_body(session.project()), expected_body);
    assert_eq!(session.project().title_page, expected_title);
    assert_eq!(
        session.project().direct_paragraph_overrides,
        direct_body_formatting_before,
        "Body direct formatting is unchanged"
    );
    assert_eq!(
        session.project().dual_dialogue_paragraph_overrides,
        direct_wrapper_formatting_before,
        "dual wrapper direct formatting is unchanged"
    );
    assert_eq!(
        unrelated_non_definition_root_nodes(session.project()),
        non_definition_settings_before,
        "PageLayout, headers/footers, continuation, revision, view, and opaque root settings remain current-owned"
    );
    let applied_preservation = session
        .project()
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("Apply retains the current source projection"));
    assert_eq!(applied_preservation.identities, source_identities_before);
    assert_eq!(applied_preservation.references, source_references_before);
    assert_eq!(applied_preservation.board_graph, source_board_graph_before);
    assert_eq!(
        applied_preservation.source.original_bytes.as_slice(),
        initial_source_bytes,
        "the immutable pre-save source snapshot remains the current document's original bytes"
    );
    assert_eq!(
        session.project().dual_dialogue_groups[0],
        active_group_before
    );
    assert_eq!(
        session.project().dual_dialogue_groups[1],
        inactive_group_before
    );
    assert!(
        active_dual_dialogue_projection(
            session.project(),
            &session.project().dual_dialogue_groups[0]
        )
        .is_some(),
        "Screenplay's required base types retain the active source projection"
    );
    assert!(
        active_dual_dialogue_projection(
            session.project(),
            &session.project().dual_dialogue_groups[1]
        )
        .is_none(),
        "a pre-inactive source group remains inactive and wrapped"
    );
    assert_eq!(
        session.project().elements[foreign_body_index(session.project(), "apply-body-cast")].text,
        "(OLD CAST)",
        "Apply does not synchronously repair Cast List content"
    );

    let body_lyrics = foreign_body_index(session.project(), "apply-body-lyrics");
    assert_eq!(
        session.project().elements[body_lyrics].element_type(),
        key("Action")
    );
    assert_eq!(
        session.project().elements[body_lyrics].text,
        "  First line — café  "
    );
    assert_eq!(session.project().elements[body_lyrics].runs.len(), 2);
    assert_eq!(
        session.project().elements[body_lyrics]
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("apply-body-lyrics")
    );
    assert_eq!(
        session.project().elements[foreign_body_index(session.project(), "apply-body-action")]
            .element_type(),
        key("Action"),
        "the exact target key is retained"
    );
    for foreign_id in [
        "apply-body-lower-action",
        "apply-body-lower-lyrics",
        "apply-body-custom",
    ] {
        assert_eq!(
            session.project().elements[foreign_body_index(session.project(), foreign_id)]
                .element_type(),
            key("Action"),
            "case-distinct and arbitrary managed body keys map to exact Screenplay Action"
        );
    }
    assert_eq!(
        session.project().title_page[foreign_title_index(session.project(), "apply-title-lyrics")]
            .element_type,
        Some(key("Action"))
    );
    assert_eq!(
        session.project().title_page
            [foreign_title_index(session.project(), "apply-title-character")]
        .element_type,
        Some(key("Character")),
        "an exact typed title key is retained"
    );
    assert_eq!(
        session.project().title_page
            [foreign_title_index(session.project(), "apply-title-arbitrary")]
        .element_type,
        Some(key("Action"))
    );
    assert_eq!(
        session.project().title_page
            [foreign_title_index(session.project(), "apply-title-typeless")],
        current_before.title_page[foreign_title_index(&current_before, "apply-title-typeless")],
        "a type-less title paragraph remains entirely unchanged"
    );

    let serialized_before_save = fs::read(&current_path)
        .unwrap_or_else(|_| unreachable!("current fixture still exists before transactional save"));
    assert_eq!(serialized_before_save, initial_source_bytes);
    let applied_project = session.project().clone();
    valid(session.undo());
    assert_eq!(session.project(), &current_before);
    assert!(!session.dirty());
    assert!(session.can_redo());
    valid(session.redo());
    assert_eq!(session.project(), &applied_project);
    assert_eq!(project_body(session.project()), expected_body);
    assert_eq!(session.project().title_page, expected_title);
    assert_applied_definition_source_matches_target(session.project(), &expected_target);
    assert!(session.dirty());

    let mut target = target;
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    assert!(!session.dirty());
    assert_eq!(target.path(), Some(target_path.as_path()));
    assert!(target.baseline().is_some());

    let saved_bytes = fs::read(&current_path)
        .unwrap_or_else(|_| unreachable!("Apply save publishes the existing native document"));
    let saved_source = String::from_utf8(saved_bytes.clone())
        .unwrap_or_else(|_| unreachable!("native FDX output is UTF-8"));
    for opaque_marker in [
        "retain-body-attachment",
        "retain-body-opaque",
        "retain-title-attachment",
        "retain-title-opaque",
        "retain-active-wrapper",
        "retain-inactive-wrapper",
        "inactive-member-opaque",
        "opaque-current-root",
        "opaque Type-looking descendant",
    ] {
        assert!(
            saved_source.contains(opaque_marker),
            "retained opaque marker: {opaque_marker}"
        );
    }
    assert!(!saved_source.contains("current-only definition payload"));
    assert!(!saved_source.contains("inert source"));
    assert!(saved_source.contains("PageLayout marker=\"current-page-layout\""));
    assert!(saved_source.contains("HeaderAndFooter marker=\"current-header-footer\""));
    assert!(saved_source.contains("current-continuation"));
    assert!(saved_source.contains("current-revision-settings"));
    assert!(saved_source.contains(
        "<WindowState Height=\"874\" Left=\"0\" Mode=\"Normal\" Top=\"75\" Width=\"870\"/>"
    ));
    assert!(saved_source
        .contains("<TextState Scaling=\"100\" Selection=\"8,8\" ShowInvisibles=\"No\"/>"));
    assert!(saved_source.contains("CURRENT-REVISION"));
    let saved_body = project_body(session.project());

    let reopened = valid(open(&StdFdxStorageFileSystem, &current_path));
    assert_eq!(&reopened.document.project, session.project());
    assert_eq!(
        reopened.document.project.element_definitions(),
        expected_target.definitions()
    );
    assert_eq!(project_body(&reopened.document.project), saved_body);
    assert_eq!(reopened.document.project.title_page, expected_title);
    assert_applied_definition_source_matches_target(&reopened.document.project, &expected_target);
    assert_eq!(
        reopened.document.project.dual_dialogue_groups,
        session.project().dual_dialogue_groups
    );
    assert!(active_dual_dialogue_projection(
        &reopened.document.project,
        &reopened.document.project.dual_dialogue_groups[1]
    )
    .is_none());
}

#[test]
fn real_builtin_apply_on_dirty_document_undo_restores_prior_dirty_state() {
    let directory = TempDirectory::new();
    let current_path = directory.file("dirty-current-musical-document.fdx");
    fs::write(&current_path, CURRENT_FIXTURE)
        .unwrap_or_else(|_| unreachable!("sanitized current fixture can be written"));
    let opened = valid(open(&StdFdxStorageFileSystem, &current_path));
    let mut session = valid(EditingSession::new(opened.document.project));

    let lyrics = foreign_body_index(session.project(), "apply-body-lyrics");
    valid(session.set_selection(Selection::caret(Position {
        element: lyrics,
        offset: 0,
    })));
    valid(session.insert("unsaved current edit: "));
    assert!(session.dirty());
    let prior_dirty_project = session.project().clone();
    let revision_before_apply = session.revision();
    let token_before_apply = session.mutation_token();
    let path_before_apply = current_path.clone();
    let baseline_before_apply = opened.baseline.clone();

    assert_eq!(
        valid(session.apply_builtin_template(BuiltinTemplateId::Screenplay)),
        ApplyTemplateOutcome::Changed
    );
    assert!(session.dirty());
    assert_eq!(session.revision(), revision_before_apply + 1);
    assert_eq!(session.mutation_token(), token_before_apply + 1);
    assert_eq!(
        session.project().elements[foreign_body_index(session.project(), "apply-body-lyrics")].text,
        "unsaved current edit:   First line — café  "
    );
    assert_eq!(
        session.project().elements[foreign_body_index(session.project(), "apply-body-lyrics")]
            .element_type(),
        key("Action")
    );
    assert_eq!(baseline_before_apply.path(), path_before_apply.as_path());
    assert_eq!(baseline_before_apply.byte_count(), CURRENT_FIXTURE.len());
    assert_eq!(
        fs::read(&current_path).ok().as_deref(),
        Some(CURRENT_FIXTURE)
    );

    let applied_project = session.project().clone();
    valid(session.undo());
    assert_eq!(session.project(), &prior_dirty_project);
    assert!(
        session.dirty(),
        "Undo restores the dirty state that existed before Apply"
    );
    valid(session.redo());
    assert_eq!(session.project(), &applied_project);
    assert!(session.dirty());
    assert_eq!(
        session.project().elements[foreign_body_index(session.project(), "apply-body-lyrics")]
            .element_type(),
        key("Action")
    );
}
