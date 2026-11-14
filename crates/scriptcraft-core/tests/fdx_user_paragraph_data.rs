#![forbid(unsafe_code)]

use scriptcraft_core::fdx::{
    import_fdx, read_v6, write_v6, FdxImportOutcome, FdxMigrationConsent, FdxMigrationKind,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SectionKind};
use scriptcraft_core::Project;

const SYNTHETIC: &[u8] = include_bytes!("fixtures/user_paragraph_data.fdx");
const REFERENCE_V2: &[u8] = include_bytes!("../../../.reference/The Once and Future King.fdx");

fn collect_attachments(nodes: &[PreservedNode], output: &mut Vec<PreservedElement>) {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if element.name.local_name == "UserParagraphData" {
            continue;
        }
        if element.name.local_name == "Paragraph" {
            for child in &element.children {
                if let PreservedNode::Element(attachment) = child {
                    if attachment.name.local_name == "UserParagraphData"
                        && attachment.name.namespace_uri == element.name.namespace_uri
                    {
                        output.push(attachment.clone());
                    }
                }
            }
        }
        collect_attachments(&element.children, output);
    }
}

fn attachments_by_section(project: &Project, kind: SectionKind) -> Vec<PreservedElement> {
    let mut output = Vec::new();
    for section in &project.preservation.as_ref().expect("source tree exists").sections {
        if section.kind == kind {
            collect_attachments(&section.children, &mut output);
        }
    }
    output
}

fn all_attachments(project: &Project) -> Vec<PreservedElement> {
    let mut output = Vec::new();
    for section in &project.preservation.as_ref().expect("source tree exists").sections {
        collect_attachments(&section.children, &mut output);
    }
    output
}

#[test]
fn opaque_attachments_roundtrip_without_entering_semantic_projection_or_identity_graph() {
    let imported = read_v6(SYNTHETIC).expect("synthetic attachment fixture imports");
    assert_eq!(imported.project.title_page.len(), 1);
    assert_eq!(imported.project.title_page[0].text, "A Small Story");
    assert_eq!(
        imported
            .project
            .elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>(),
        ["Opening action.", "LINA", "Hello there.", "OMAR", "Goodbye."]
    );
    let before = all_attachments(&imported.project);
    assert_eq!(before.len(), 7);
    assert_eq!(attachments_by_section(&imported.project, SectionKind::TitlePage).len(), 1);
    assert_eq!(attachments_by_section(&imported.project, SectionKind::Content).len(), 6);

    let preservation = imported.project.preservation.as_ref().expect("source tree exists");
    let identities_before = preservation.identities.clone();
    assert!(!preservation.identities.iter().any(|identity| {
        matches!(identity.value.as_str(), "payload-body-id" | "not-a-group")
    }));
    assert!(preservation.board_graph.entities.is_empty());
    assert!(preservation.board_graph.references.is_empty());

    let output = write_v6(&imported.project).expect("opaque source attachment serializes");
    let reopened = read_v6(&output).expect("serialized attachment source reimports");
    assert_eq!(all_attachments(&reopened.project), before);
    assert_eq!(
        reopened
            .project
            .preservation
            .as_ref()
            .expect("reopened source tree exists")
            .identities,
        identities_before
    );
    assert_eq!(reopened.project.elements, imported.project.elements);
    assert_eq!(
        reopened.project.dual_dialogue_groups,
        imported.project.dual_dialogue_groups
    );
}

#[test]
fn unsupported_attachment_and_unknown_child_shapes_remain_opaque_on_import() {
    let source = std::str::from_utf8(SYNTHETIC).expect("synthetic fixture is UTF-8");
    let duplicate = source.replacen(
        "</UserParagraphData>",
        "</UserParagraphData><UserParagraphData/>",
        1,
    );
    assert!(read_v6(duplicate.as_bytes()).is_ok());

    let wrong_namespace = source.replacen(
        "<UserParagraphData vendor=\"body\"",
        "<UserParagraphData xmlns=\"urn:wrong\" vendor=\"body\"",
        1,
    );
    assert!(read_v6(wrong_namespace.as_bytes()).is_ok());

    let nested_in_text = source.replacen(
        "<Text>Opening action.</Text>",
        "<Text>Opening <UserParagraphData/>action.</Text>",
        1,
    );
    assert!(read_v6(nested_in_text.as_bytes()).is_ok());

    let misplaced = source.replacen(
        "<Content>",
        "<Content><UserParagraphData/>",
        1,
    );
    assert!(read_v6(misplaced.as_bytes()).is_ok());

    let unrelated_child = source.replacen(
        "<Text>Opening action.</Text>",
        "<Text>Opening action.</Text><UnrecognizedDirectChild/>",
        1,
    );
    assert!(read_v6(unrelated_child.as_bytes()).is_ok());
}

#[test]
fn version_two_continue_retains_all_reference_paragraph_attachments_and_existing_loss_scope() {
    let outcome = import_fdx(REFERENCE_V2, FdxMigrationConsent::Continue)
        .expect("Version 2 Continue imports the untouched reference");
    let FdxImportOutcome::Candidate(candidate) = outcome else {
        panic!("Continue returns an import candidate");
    };
    assert_eq!(
        candidate.migration,
        Some(FdxMigrationKind::OlderLossy {
            declared: "2".to_owned()
        })
    );
    assert_eq!(candidate.document.project.elements.len(), 22);
    assert_eq!(candidate.document.project.title_page.len(), 48);
    let before = all_attachments(&candidate.document.project);
    let body_before = candidate.document.project.elements.clone();
    let title_before = candidate.document.project.title_page.clone();
    let groups_before = candidate.document.project.dual_dialogue_groups.clone();
    let identities_before = candidate
        .document
        .project
        .preservation
        .as_ref()
        .expect("migration retains a source tree")
        .identities
        .clone();
    assert_eq!(before.len(), 70);
    assert_eq!(
        attachments_by_section(&candidate.document.project, SectionKind::Content).len(),
        22
    );
    assert_eq!(
        attachments_by_section(&candidate.document.project, SectionKind::TitlePage).len(),
        48
    );
    assert!(!candidate
        .document
        .project
        .preservation
        .as_ref()
        .expect("migration retains a source tree")
        .sections
        .iter()
        .any(|section| section.name.local_name == "UserDocumentData"));

    let output = write_v6(&candidate.document.project)
        .expect("migrated paragraph payloads serialize through the direct writer");
    let reopened = read_v6(&output).expect("direct-writer output reimports");
    assert_eq!(all_attachments(&reopened.project), before);
    assert_eq!(reopened.project.elements, body_before);
    assert_eq!(reopened.project.title_page, title_before);
    assert_eq!(reopened.project.dual_dialogue_groups, groups_before);
    assert_eq!(
        reopened
            .project
            .preservation
            .as_ref()
            .expect("reopened source tree exists")
            .identities,
        identities_before
    );
}

#[test]
fn new_preservation_free_document_does_not_generate_paragraph_attachments() {
    let project = Project::new().expect("new project initializes");
    let output = write_v6(&project).expect("new project serializes");
    assert!(!String::from_utf8(output)
        .expect("writer output is UTF-8")
        .contains("UserParagraphData"));
}
