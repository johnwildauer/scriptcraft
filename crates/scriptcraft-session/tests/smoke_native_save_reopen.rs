#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::dual_dialogue::{active_dual_dialogue_projection, map_group_identity};
use scriptcraft_model::preservation::{
    AnchorRelation, AnchorTarget, BoardEntity, BoardEntityKind, DependencyConfidence,
    ForeignIdDomain, ForeignIdentity, ForeignReference, PreservedElement, PreservedNode,
    PreservedSection, ReferencePolicy, SourceAnchor, SourceAnchorId, SourceName, SourceOwnership,
};
use scriptcraft_model::{ElementDefinition, ElementTypeKey, Position, Project, Selection};
use scriptcraft_session::editing::EditingSession;
use scriptcraft_session::editing::{ElementDefinitionCandidate, ElementTypeRemapping};
use scriptcraft_session::storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use std::fs;
use std::path::PathBuf;

#[track_caller]
fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| {
        let caller = std::panic::Location::caller();
        panic!(
            "native save/reopen smoke failed at {}:{}: {error:?}",
            caller.file(),
            caller.line()
        )
    })
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("scriptcraft-native-smoke-{}", uuid::Uuid::new_v4()));
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
fn native_save_reopen_public_api_smoke() {
    const BODY_TEXT: &str = "Native smoke — café";

    let directory = TestDirectory::new();
    let destination = directory.path("smoke-native.fdx");
    assert!(!destination.exists());

    let mut session = valid(EditingSession::new(valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    )));
    valid(session.insert(BODY_TEXT));
    assert_eq!(session.project().elements[0].text, BODY_TEXT);
    let expected_body = session
        .project()
        .elements
        .iter()
        .map(|owner| (owner.element_type(), owner.text.clone()))
        .collect::<Vec<_>>();
    let expected_title_page = session
        .project()
        .title_page
        .iter()
        .map(|paragraph| (paragraph.element_type(), paragraph.text.clone()))
        .collect::<Vec<_>>();
    let expected_direct_paragraph_overrides = session.project().direct_paragraph_overrides.clone();
    let expected_dual_dialogue_paragraph_overrides =
        session.project().dual_dialogue_paragraph_overrides.clone();
    let expected_dual_dialogue_groups = session.project().dual_dialogue_groups.clone();
    let mut target = FdxSaveTarget::new_unsaved();

    let outcome = valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    ));

    assert!(!outcome.temporary_cleanup_warning);
    assert_eq!(target.path(), Some(destination.as_path()));
    assert!(target.baseline().is_some());
    assert!(!session.dirty());
    let saved_body = session
        .project()
        .elements
        .iter()
        .map(|owner| (owner.element_type(), owner.text.clone()))
        .collect::<Vec<_>>();
    let saved_title_page = session
        .project()
        .title_page
        .iter()
        .map(|paragraph| (paragraph.element_type(), paragraph.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(saved_body, expected_body);
    assert_eq!(saved_title_page, expected_title_page);
    assert_eq!(
        session.project().direct_paragraph_overrides,
        expected_direct_paragraph_overrides
    );
    assert_eq!(
        session.project().dual_dialogue_paragraph_overrides,
        expected_dual_dialogue_paragraph_overrides
    );
    assert_eq!(
        session.project().dual_dialogue_groups,
        expected_dual_dialogue_groups
    );

    let reopened = valid(open(&StdFdxStorageFileSystem, &destination));
    let reopened_project = reopened.document.project;
    assert_eq!(reopened_project.validate(), Ok(()));
    let saved_project = session.project();
    assert_eq!(reopened_project.elements, saved_project.elements);
    assert_eq!(reopened_project.title_page, saved_project.title_page);
    assert_eq!(
        reopened_project.direct_paragraph_overrides,
        saved_project.direct_paragraph_overrides
    );
    assert_eq!(
        reopened_project.dual_dialogue_paragraph_overrides,
        saved_project.dual_dialogue_paragraph_overrides
    );
    assert_eq!(
        reopened_project.dual_dialogue_groups,
        saved_project.dual_dialogue_groups
    );
    assert_eq!(
        reopened_project.element_definitions(),
        saved_project.element_definitions()
    );

    let reopened_session = valid(EditingSession::new(reopened_project));
    assert!(!reopened_session.dirty());
}

#[test]
fn save_rebases_selection_and_history_across_body_title_type_and_definition_changes() {
    const OPEN_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/open_element_settings.fdx");

    fn key(label: &str) -> ElementTypeKey {
        ElementTypeKey::new(label).unwrap_or_else(|_| unreachable!("test type key is valid"))
    }
    fn rename(definitions: &mut [ElementDefinition], from: &str, to: &str) {
        let definition = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == from)
            .unwrap_or_else(|| unreachable!("fixture includes the remapped definition"));
        definition.key = key(to);
    }

    let project = valid(read_v6(OPEN_FIXTURE)).project;
    let body_index = project
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-body-lyrics")
        })
        .unwrap_or_else(|| unreachable!("fixture Lyrics paragraph is managed"));
    let title_index = project
        .title_page
        .iter()
        .position(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-title-content")
        })
        .unwrap_or_else(|| unreachable!("fixture TitlePage Content paragraph is managed"));
    let mut session = valid(EditingSession::new(project));
    let body_selection = Selection::caret(Position {
        element: body_index,
        offset: 3,
    });
    let title_selection = Selection::caret(Position {
        element: title_index,
        offset: 2,
    });
    valid(session.set_selection(body_selection));
    valid(session.set_title_selection(title_selection));

    let mut definitions = session
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    rename(&mut definitions, "Lyrics", "Song");
    rename(&mut definitions, "TitleContent", "Credit");
    definitions.reverse();
    let expected_definitions = definitions.clone();
    let mut candidate = ElementDefinitionCandidate::new(definitions);
    candidate
        .body_remappings
        .push(ElementTypeRemapping::new(key("Lyrics"), key("Song")));
    candidate.title_remappings.push(ElementTypeRemapping::new(
        key("TitleContent"),
        key("Credit"),
    ));
    valid(session.replace_element_definitions_and_remap_types(candidate));
    let candidate_body_id = session.project().elements[body_index].id.clone();
    let candidate_title_id = session.project().title_page[title_index].id.clone();
    assert!(session.dirty());
    assert!(session.can_undo());

    let directory = TestDirectory::new();
    let destination = directory.path("changed-types.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    ));

    assert_eq!(session.project().elements[body_index].id, candidate_body_id);
    assert_eq!(
        session.project().elements[body_index].element_type(),
        key("Song")
    );
    assert_eq!(
        session.project().title_page[title_index].id,
        candidate_title_id
    );
    assert_eq!(
        session.project().title_page[title_index].element_type(),
        Some(key("Credit"))
    );
    assert_eq!(session.selection(), body_selection);
    assert_eq!(session.title_selection(), title_selection);
    assert_eq!(
        session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        expected_definitions
    );
    assert!(!session.dirty());

    valid(session.undo());
    assert_eq!(
        session.project().elements[body_index].element_type(),
        key("Lyrics")
    );
    assert_eq!(
        session.project().title_page[title_index].element_type(),
        Some(key("TitleContent"))
    );
    assert!(session.dirty());
    valid(session.redo());
    assert_eq!(
        session.project().elements[body_index].element_type(),
        key("Song")
    );
    assert_eq!(
        session.project().title_page[title_index].element_type(),
        Some(key("Credit"))
    );
    assert!(!session.dirty());
}

#[test]
fn save_rebases_inactive_dual_identity_after_a_member_type_change() {
    const ACTIVE_DUAL_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/active_dual_dialogue.fdx");

    fn key(label: &str) -> ElementTypeKey {
        ElementTypeKey::new(label).unwrap_or_else(|_| unreachable!("test type key is valid"))
    }

    let project = valid(read_v6(ACTIVE_DUAL_FIXTURE)).project;
    let mut session = valid(EditingSession::new(project));
    let group = session
        .project()
        .dual_dialogue_groups
        .first()
        .unwrap_or_else(|| unreachable!("fixture contains a dual dialogue group"));
    assert!(active_dual_dialogue_projection(session.project(), group).is_some());
    let left_cue = session
        .project()
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("active-left-cue")
        })
        .unwrap_or_else(|| unreachable!("fixture left cue is managed"));
    valid(session.set_selection(Selection::caret(Position {
        element: left_cue,
        offset: 2,
    })));
    valid(session.change_element_type(key("Action")));
    let changed_project = session.project().clone();
    let changed_group_id = changed_project.dual_dialogue_groups[0].id.clone();
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert!(active_dual_dialogue_projection(
        session.project(),
        &session.project().dual_dialogue_groups[0]
    )
    .is_none());

    let directory = TestDirectory::new();
    let destination = directory.path("inactive-dual.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination),
    ));

    let saved_group = session
        .project()
        .dual_dialogue_groups
        .first()
        .unwrap_or_else(|| unreachable!("save preserves the dual source container"));
    assert_eq!(
        map_group_identity(&changed_project, session.project(), &changed_group_id),
        Some(saved_group.id.clone())
    );
    assert_eq!(
        saved_group.members,
        changed_project.dual_dialogue_groups[0].members
    );
    assert!(active_dual_dialogue_projection(session.project(), saved_group).is_none());
    assert!(!session.dirty());
}

#[test]
fn explicit_source_backed_disable_unwraps_active_and_inactive_groups_reversibly() {
    const ACTIVE_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/active_dual_dialogue.fdx");
    const INACTIVE_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/inactive_dual_dialogue.fdx");

    for (fixture, wrapper_id, member_attachment) in [
        (ACTIVE_FIXTURE, "active-dual-wrapper", "MemberPayload"),
        (
            INACTIVE_FIXTURE,
            "inactive-dual-wrapper",
            "VendorMemberPayload",
        ),
    ] {
        let directory = TestDirectory::new();
        let destination = directory.path("disabled-dual.fdx");
        let project = valid(read_v6(fixture)).project;
        let mut session = valid(EditingSession::new(project));
        let before = session.project().clone();
        let group = before
            .dual_dialogue_groups
            .first()
            .unwrap_or_else(|| unreachable!("fixture has a dual-dialogue group"));
        let group_id = group.id.clone();
        let member_ids = group.members.clone();
        let expected_members = member_ids
            .iter()
            .map(|id| {
                before
                    .elements
                    .iter()
                    .find(|element| &element.id == id)
                    .unwrap_or_else(|| unreachable!("group members are managed"))
                    .clone()
            })
            .collect::<Vec<_>>();
        let expected_member_overrides = before.direct_paragraph_overrides.clone();
        let selected = before
            .elements
            .iter()
            .position(|element| element.id == member_ids[0])
            .unwrap_or_else(|| unreachable!("first member has a flat body position"));
        valid(session.set_selection(Selection::caret(Position {
            element: selected,
            offset: 0,
        })));
        valid(session.toggle_dual_dialogue());

        assert!(session.project().dual_dialogue_groups.is_empty());
        for expected in &expected_members {
            assert!(session
                .project()
                .elements
                .iter()
                .any(|element| element == expected));
        }
        assert_eq!(
            session.project().direct_paragraph_overrides,
            expected_member_overrides
        );
        let preservation = session
            .project()
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("source-backed disable retains preservation"));
        for member in &expected_members {
            if let Some(foreign_id) = member
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
            {
                assert!(preservation.identities.iter().any(|identity| {
                    identity.domain == ForeignIdDomain::Paragraph && identity.value == foreign_id
                }));
            }
        }
        assert!(!session
            .project()
            .dual_dialogue_paragraph_overrides
            .contains_key(&group_id));
        valid(session.undo());
        assert_eq!(session.project(), &before);
        assert!(
            session.undo().is_err(),
            "one toggle creates one history entry"
        );
        valid(session.redo());
        assert!(session.project().dual_dialogue_groups.is_empty());
        assert!(session.redo().is_err(), "one toggle creates one redo entry");

        let mut target = FdxSaveTarget::new_unsaved();
        valid(save_session(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
        ));
        let written = fs::read_to_string(&destination)
            .unwrap_or_else(|_| unreachable!("the saved FDX is UTF-8 source"));
        assert!(!written.contains(wrapper_id));
        assert!(!written.contains("WrapperPayload"));
        assert!(!written.contains("SpellCheckIgnoreLists"));
        assert!(written.contains(member_attachment));

        let reopened = valid(open(&StdFdxStorageFileSystem, &destination));
        assert!(reopened.document.project.dual_dialogue_groups.is_empty());
        assert_eq!(reopened.document.project.elements, expected_members);
    }
}

#[test]
fn explicit_unwrap_keeps_required_board_reference_failure_atomic() {
    const ACTIVE_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/active_dual_dialogue.fdx");
    let mut project = valid(read_v6(ACTIVE_FIXTURE)).project;
    let group = project.dual_dialogue_groups[0].clone();
    let member_index = project
        .elements
        .iter()
        .position(|element| element.id == group.members[0])
        .unwrap_or_else(|| unreachable!("first group member has a body position"));
    let wrapper_identity = ForeignIdentity::new(
        ForeignIdDomain::Paragraph,
        group
            .wrapper
            .outer_foreign_id
            .clone()
            .unwrap_or_else(|| unreachable!("fixture wrapper has a foreign ID")),
    )
    .unwrap_or_else(|_| unreachable!("fixture wrapper identity is bounded"));
    let board_identity = ForeignIdentity::new(ForeignIdDomain::Board, "required-board")
        .unwrap_or_else(|_| unreachable!("test board identity is bounded"));
    let preservation = project
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("FDX fixture retains preservation state"));
    let source_order = preservation
        .board_graph
        .entities
        .iter()
        .map(|entity| entity.source_order)
        .max()
        .map_or(0, |order| order.saturating_add(1));
    preservation.board_graph.entities.push(BoardEntity {
        identity: board_identity.clone(),
        kind: BoardEntityKind::DisplayBoard,
        source_order,
        anchor: None,
        image_payload: None,
        declared_image_length: None,
        source_path: None,
    });
    preservation.board_graph.references.push(ForeignReference {
        source: board_identity,
        target: wrapper_identity,
        policy: ReferencePolicy::RequiredInternal,
    });
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position {
        element: member_index,
        offset: 0,
    })));
    let before = session.project().clone();
    let revision = session.revision();
    assert!(session.toggle_dual_dialogue().is_err());
    assert_eq!(session.project(), &before);
    assert_eq!(session.revision(), revision);
    assert!(
        session.undo().is_err(),
        "failed unwrap does not create history"
    );
}

#[test]
fn explicit_unwrap_cleans_wrapper_typed_uses_but_keeps_raw_id_orphans() {
    const ACTIVE_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/active_dual_dialogue.fdx");
    let mut project = valid(read_v6(ACTIVE_FIXTURE)).project;
    let group = project.dual_dialogue_groups[0].clone();
    let wrapper_id = group
        .wrapper
        .outer_foreign_id
        .as_deref()
        .unwrap_or_else(|| unreachable!("fixture wrapper has a foreign ID"));
    let wrapper_identity = ForeignIdentity::new(ForeignIdDomain::Paragraph, wrapper_id)
        .unwrap_or_else(|_| unreachable!("fixture wrapper identity is bounded"));
    let attached_anchor = SourceAnchorId::new("test-wrapper-attached-anchor")
        .unwrap_or_else(|_| unreachable!("test anchor is bounded"));
    let preservation = project
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("FDX fixture retains preservation state"));
    preservation.references.push(ForeignReference {
        source: wrapper_identity.clone(),
        target: ForeignIdentity::new(ForeignIdDomain::Board, "typed-orphan-target")
            .unwrap_or_else(|_| unreachable!("test board identity is bounded")),
        policy: ReferencePolicy::CrossDomainMayOrphan,
    });
    preservation.board_graph.references.push(ForeignReference {
        source: wrapper_identity.clone(),
        target: ForeignIdentity::new(ForeignIdDomain::Board, "typed-orphan-target")
            .unwrap_or_else(|_| unreachable!("test board identity is bounded")),
        policy: ReferencePolicy::CrossDomainMayOrphan,
    });
    preservation.anchors.push(SourceAnchor {
        id: attached_anchor.clone(),
        target: AnchorTarget::Foreign(wrapper_identity.clone()),
        relation: AnchorRelation::Attached,
    });
    let outer = find_preserved_wrapper_mut(&mut preservation.sections, &group.id)
        .unwrap_or_else(|| unreachable!("fixture has its preserved wrapper"));
    let dual = outer
        .children
        .iter_mut()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "DualDialogue" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("wrapper contains its dual source node"));
    let member = find_preserved_managed_mut(&mut dual.children, &group.members[0])
        .unwrap_or_else(|| unreachable!("first group member has preserved source"));
    member
        .children
        .push(PreservedNode::Comment(wrapper_id.to_owned().into()));
    member
        .children
        .push(PreservedNode::Element(PreservedElement {
            name: SourceName {
                namespace_uri: None,
                local_name: "VendorAttachedOrphan".to_owned(),
                qualified_name: Some("VendorAttachedOrphan".to_owned()),
            },
            attributes: vec![scriptcraft_model::preservation::PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: "id".to_owned(),
                    qualified_name: Some("id".to_owned()),
                },
                value: wrapper_id.to_owned(),
            }],
            namespace_bindings: Vec::new(),
            source_type: None,
            ownership: SourceOwnership::Attached(wrapper_identity.clone()),
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: Some(attached_anchor),
            children: Vec::new().into(),
        }));

    let member_index = project
        .elements
        .iter()
        .position(|element| element.id == group.members[0])
        .unwrap_or_else(|| unreachable!("first group member has a body position"));
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position {
        element: member_index,
        offset: 0,
    })));
    valid(session.toggle_dual_dialogue());

    let preservation = session
        .project()
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("source-backed disable retains preservation"));
    assert!(!preservation.references.iter().any(|reference| {
        reference.source == wrapper_identity || reference.target == wrapper_identity
    }));
    assert!(
        !preservation.board_graph.references.iter().any(|reference| {
            reference.source == wrapper_identity || reference.target == wrapper_identity
        })
    );
    assert!(!preservation.anchors.iter().any(|anchor| {
        matches!(&anchor.target, AnchorTarget::Foreign(identity) if identity == &wrapper_identity)
    }));
    assert!(!preservation.anchors.iter().any(|anchor| {
        matches!(&anchor.target, AnchorTarget::DualDialogueWrapper(id) if id == &group.id)
    }));
    assert!(!preservation.identities.contains(&wrapper_identity));
    let attached = find_preserved_element(&preservation.sections, "VendorAttachedOrphan")
        .unwrap_or_else(|| unreachable!("raw source occurrence remains as an orphan"));
    assert_eq!(attached.ownership, SourceOwnership::Opaque);
    assert!(attached.anchor.is_none());
    assert!(attached
        .attributes
        .iter()
        .any(|attribute| { attribute.name.local_name == "id" && attribute.value == wrapper_id }));
}

fn find_preserved_wrapper_mut<'a>(
    sections: &'a mut [PreservedSection],
    group_id: &scriptcraft_model::StableId,
) -> Option<&'a mut PreservedElement> {
    fn find<'a>(
        nodes: &'a mut [PreservedNode],
        group_id: &scriptcraft_model::StableId,
    ) -> Option<&'a mut PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
            {
                return Some(element);
            }
            if let Some(found) = find(&mut element.children, group_id) {
                return Some(found);
            }
        }
        None
    }
    for section in sections {
        if let Some(found) = find(&mut section.children, group_id) {
            return Some(found);
        }
    }
    None
}

fn find_preserved_managed_mut<'a>(
    nodes: &'a mut [PreservedNode],
    id: &scriptcraft_model::StableId,
) -> Option<&'a mut PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id) {
            return Some(element);
        }
        if let Some(found) = find_preserved_managed_mut(&mut element.children, id) {
            return Some(found);
        }
    }
    None
}

fn find_preserved_element<'a>(
    sections: &'a [PreservedSection],
    name: &str,
) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], name: &str) -> Option<&'a PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == name {
                return Some(element);
            }
            if let Some(found) = find(&element.children, name) {
                return Some(found);
            }
        }
        None
    }
    sections
        .iter()
        .find_map(|section| find(&section.children, name))
}
