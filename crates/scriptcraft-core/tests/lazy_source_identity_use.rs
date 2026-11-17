#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{
    managed_paragraph_index_diagnostics, reset_managed_paragraph_index_diagnostics,
    reset_source_identity_use_index_diagnostics, source_identity_use_index_diagnostics,
    PreservationError, PreservedAttribute, PreservedElement, PreservedNode,
};
use scriptcraft_core::{ParagraphAlignment, Project};

const FIXTURE: &[u8] = include_bytes!("fixtures/user_paragraph_data.fdx");

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("lazy identity-use case failed: {error:?}"))
}

fn fixture_session() -> EditingSession {
    let imported = read_v6(FIXTURE).expect("opaque lifecycle fixture imports");
    valid(EditingSession::new(imported.project))
}

fn attachments_by_paragraph_id(project: &Project) -> BTreeMap<String, PreservedElement> {
    fn visit(nodes: &[PreservedNode], output: &mut BTreeMap<String, PreservedElement>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph" {
                let owner_id = element
                    .attributes
                    .iter()
                    .find(|attribute| {
                        attribute.name.namespace_uri.is_none()
                            && attribute.name.local_name.eq_ignore_ascii_case("id")
                    })
                    .map(|attribute| attribute.value.clone());
                if let Some(owner_id) = owner_id {
                    for child in &element.children {
                        if let PreservedNode::Element(attachment) = child {
                            if attachment.name.local_name == "UserParagraphData"
                                && attachment.name.namespace_uri == element.name.namespace_uri
                            {
                                output.insert(owner_id.clone(), attachment.clone());
                            }
                        }
                    }
                }
            }
            visit(&element.children, output);
        }
    }

    let mut output = BTreeMap::new();
    if let Some(preservation) = &project.preservation {
        for section in &preservation.sections {
            visit(&section.children, &mut output);
        }
    }
    output
}

fn paragraph_by_foreign_id(project: &Project, id: &str) -> Option<PreservedElement> {
    fn visit(nodes: &[PreservedNode], id: &str) -> Option<PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            let found_id = element.attributes.iter().find(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
            }).map(|attribute| attribute.value.as_str());
            if element.name.local_name == "Paragraph" && found_id == Some(id) {
                return Some(element.clone());
            }
            if let Some(found) = visit(&element.children, id) {
                return Some(found);
            }
        }
        None
    }

    project
        .preservation
        .as_ref()?
        .sections
        .iter()
        .find_map(|section| visit(&section.children, id))
}

fn owner_opaque_snapshot(
    paragraph: &PreservedElement,
) -> (Vec<PreservedAttribute>, Vec<PreservedNode>) {
    let attributes = paragraph
        .attributes
        .iter()
        .filter(|attribute| {
            !(attribute.name.namespace_uri.is_none()
                && (attribute.name.local_name.eq_ignore_ascii_case("id")
                    || attribute.name.local_name == "Type"))
        })
        .cloned()
        .collect();
    let children = paragraph
        .children
        .iter()
        .filter(|node| {
            !matches!(node, PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri)
        })
        .cloned()
        .collect();
    (attributes, children)
}

fn raw_source_id_values(project: &Project) -> BTreeSet<String> {
    fn visit(nodes: &[PreservedNode], output: &mut BTreeSet<String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            output.extend(
                element
                    .attributes
                    .iter()
                    .filter(|attribute| {
                        attribute.name.namespace_uri.is_none()
                            && attribute.name.local_name.eq_ignore_ascii_case("id")
                    })
                    .map(|attribute| attribute.value.clone()),
            );
            visit(&element.children, output);
        }
    }

    let mut output = BTreeSet::new();
    if let Some(preservation) = &project.preservation {
        for section in &preservation.sections {
            visit(&section.children, &mut output);
        }
    }
    output
}

fn assert_native_roundtrip(session: &EditingSession, owner_id: &str) {
    let expected_attachments = attachments_by_paragraph_id(session.project());
    let expected_owner = owner_opaque_snapshot(
        &paragraph_by_foreign_id(session.project(), owner_id).expect("source owner exists"),
    );
    let bytes = write_v6(session.project()).expect("edited source writes directly");
    let reopened = read_v6(&bytes).expect("edited source reimports");
    assert_eq!(
        attachments_by_paragraph_id(&reopened.project),
        expected_attachments
    );
    let actual_owner = owner_opaque_snapshot(
        &paragraph_by_foreign_id(&reopened.project, owner_id).expect("reopened owner exists"),
    );
    assert_eq!(actual_owner, expected_owner);
}

/// One test entry point keeps the process-wide value-free counters isolated
/// from other integration tests while the scenarios execute sequentially.
#[test]
fn lazy_source_identity_use_contract() {
    scoped_text_edit_defers_identity_use_rebuild_and_retains_managed_paths();
    newly_introduced_wrapper_reference_refreshes_before_unwrap_safe_stop();
    full_validation_fallback_refreshes_lazy_identity_use_without_changing_opaque_data();
    structural_split_after_lazy_edit_refreshes_before_fresh_id_allocation();
    dual_dialogue_rewrap_after_lazy_edit_refreshes_before_wrapper_id_allocation();
}

fn scoped_text_edit_defers_identity_use_rebuild_and_retains_managed_paths() {
    let mut session = fixture_session();
    let first_text = session.project().elements[0].text.clone();
    let original_payloads = attachments_by_paragraph_id(session.project());
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: first_text.len(),
    })));
    reset_source_identity_use_index_diagnostics();
    reset_managed_paragraph_index_diagnostics();

    valid(session.insert(" without eager identity scanning"));

    assert!(session.last_edit_used_scoped_validation());
    let identity = source_identity_use_index_diagnostics();
    assert_eq!(identity.rebuilds, 0);
    assert_eq!(identity.source_nodes_indexed, 0);
    assert_eq!(identity.identity_queries, 0);
    assert_eq!(identity.reserved_id_queries, 0);
    let managed = managed_paragraph_index_diagnostics();
    assert_eq!(managed.rebuilds, 0);
    assert_eq!(managed.source_nodes_indexed, 0);
    assert!(managed.lookups > 0);
    assert_eq!(managed.fallback_tree_nodes_examined, 0);
    assert_eq!(
        session.project().elements[0].text,
        format!("{first_text} without eager identity scanning")
    );
    assert_eq!(
        attachments_by_paragraph_id(session.project()),
        original_payloads
    );
    assert_native_roundtrip(&session, "body-001");

    let edited = session.project().clone();
    valid(session.undo());
    assert_eq!(session.project().elements[0].text, first_text);
    valid(session.redo());
    assert_eq!(session.project(), &edited);
    assert_native_roundtrip(&session, "body-001");
}

fn newly_introduced_wrapper_reference_refreshes_before_unwrap_safe_stop() {
    let mut session = fixture_session();
    let first_text = session.project().elements[0].text.clone();
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: first_text.len(),
    })));
    valid(session.insert(" outer-dual-001"));
    assert!(session.last_edit_used_scoped_validation());
    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    let before = session.clone();
    reset_source_identity_use_index_diagnostics();

    let error = session
        .toggle_dual_dialogue()
        .expect_err("new source text must veto wrapper removal");

    assert!(matches!(
        error,
        EditError::Preservation(PreservationError::PotentialReferenceConflict)
    ));
    let identity = source_identity_use_index_diagnostics();
    assert_eq!(identity.rebuilds, 1);
    assert!(identity.source_nodes_indexed > 0);
    assert_eq!(identity.identity_queries, 1);
    assert_eq!(session, before);
    assert!(paragraph_by_foreign_id(session.project(), "outer-dual-001").is_some());
    assert_native_roundtrip(&session, "body-001");
}

fn full_validation_fallback_refreshes_lazy_identity_use_without_changing_opaque_data() {
    let mut session = fixture_session();
    let first_text = session.project().elements[0].text.clone();
    let original_payloads = attachments_by_paragraph_id(session.project());
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: first_text.len(),
    })));
    valid(session.insert("!"));
    assert!(session.last_edit_used_scoped_validation());
    reset_source_identity_use_index_diagnostics();

    valid(session.set_body_alignment(ParagraphAlignment::Right));

    assert!(!session.last_edit_used_scoped_validation());
    let identity = source_identity_use_index_diagnostics();
    assert!(identity.rebuilds > 0);
    assert!(identity.source_nodes_indexed > 0);
    assert_eq!(
        attachments_by_paragraph_id(session.project()),
        original_payloads
    );
    assert_native_roundtrip(&session, "body-001");
}

fn structural_split_after_lazy_edit_refreshes_before_fresh_id_allocation() {
    let mut session = fixture_session();
    let source_ids_before = raw_source_id_values(session.project());
    let original_payloads = attachments_by_paragraph_id(session.project());
    let first_text = session.project().elements[0].text.clone();
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: first_text.len(),
    })));
    valid(session.insert("!"));
    assert!(session.last_edit_used_scoped_validation());
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: "Opening ".len(),
    })));
    reset_source_identity_use_index_diagnostics();

    valid(session.split());

    assert!(!session.last_edit_used_scoped_validation());
    let identity = source_identity_use_index_diagnostics();
    assert!(identity.rebuilds > 0);
    assert!(identity.source_nodes_indexed > 0);
    assert!(identity.reserved_id_queries > 0);
    let right_id = session.project().elements[1]
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("split right owner receives a fresh source identity");
    assert!(!source_ids_before.contains(right_id));
    assert_eq!(
        attachments_by_paragraph_id(session.project()).get("body-001"),
        original_payloads.get("body-001")
    );
    assert_native_roundtrip(&session, "body-001");

    let split = session.project().clone();
    valid(session.undo());
    valid(session.redo());
    assert_eq!(session.project(), &split);
}

fn dual_dialogue_rewrap_after_lazy_edit_refreshes_before_wrapper_id_allocation() {
    let mut session = fixture_session();
    let source_ids_before = raw_source_id_values(session.project());
    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    valid(session.toggle_dual_dialogue());
    assert!(session.project().dual_dialogue_groups.is_empty());
    let member_text_len = session.project().elements[2].text.len();
    valid(session.set_selection(Selection::caret(Position {
        element: 2,
        offset: member_text_len,
    })));
    valid(session.insert("?"));
    assert!(session.last_edit_used_scoped_validation());
    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    reset_source_identity_use_index_diagnostics();

    valid(session.toggle_dual_dialogue());

    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    let wrapper_id = session.project().dual_dialogue_groups[0]
        .wrapper
        .outer_foreign_id
        .as_deref()
        .expect("rewrap allocates a wrapper identity");
    assert!(!source_ids_before.contains(wrapper_id));
    assert_ne!(wrapper_id, "outer-dual-001");
    let identity = source_identity_use_index_diagnostics();
    assert!(identity.rebuilds > 0);
    assert!(identity.source_nodes_indexed > 0);
    assert!(identity.reserved_id_queries > 0);
    assert_native_roundtrip(&session, "left-dialogue-001");

    let wrapped = session.project().clone();
    valid(session.undo());
    assert!(session.project().dual_dialogue_groups.is_empty());
    valid(session.redo());
    assert_eq!(session.project(), &wrapped);
    assert_native_roundtrip(&session, "left-dialogue-001");
}
