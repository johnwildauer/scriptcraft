#![forbid(unsafe_code)]

use scriptcraft_core::fdx::{
    read_v6, write_v6, FdxError, MAX_ATTRIBUTES_PER_ELEMENT, MAX_IMAGE_BYTES, MAX_SOURCE_BYTES,
    MAX_SOURCE_DEPTH, MAX_SOURCE_EVENTS, MAX_SOURCE_TEXT_BYTES,
};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::layout::{LayoutComposition, ScreenplayLayout};
use scriptcraft_core::preservation::{
    BoardEntity, BoardEntityKind, ForeignIdDomain, ForeignIdentity, PreservationError,
    PreservedElement, PreservedNode, SharedBytes, MAX_TOTAL_IMAGE_BYTES,
};
use scriptcraft_core::{LayoutInputs, Project};

const SYNTHETIC: &[u8] = include_bytes!("fixtures/broad_compatibility.fdx");
const ALTS_FIXTURE: &[u8] = include_bytes!("fixtures/dialogue_alts.fdx");
const REFERENCE_V6: &[u8] = include_bytes!("../../../.reference/The Once and Future King(1).fdx");

fn preserved_elements(project: &Project) -> Vec<&PreservedElement> {
    fn visit<'a>(nodes: &'a [PreservedNode], output: &mut Vec<&'a PreservedElement>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            output.push(element);
            visit(&element.children, output);
        }
    }

    let mut output = Vec::new();
    if let Some(preservation) = &project.preservation {
        for section in &preservation.sections {
            visit(&section.children, &mut output);
        }
    }
    output
}

fn normalized_nodes(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
    let mut normalized = Vec::new();
    for node in nodes {
        let node = match node {
            PreservedNode::Element(element) => {
                let mut element = element.clone();
                element.children = normalized_nodes(&element.children).into();
                PreservedNode::Element(element)
            }
            other => other.clone(),
        };
        if matches!(&node, PreservedNode::Text(text) if text.is_empty()) {
            continue;
        }
        if let PreservedNode::Text(text) = &node {
            if let Some(PreservedNode::Text(previous)) = normalized.last_mut() {
                let mut combined = previous.as_str().to_owned();
                combined.push_str(text.as_str());
                *previous = combined.into();
                continue;
            }
        }
        normalized.push(node);
    }
    normalized
}

fn normalized_sections(
    sections: &[scriptcraft_core::preservation::PreservedSection],
) -> Vec<scriptcraft_core::preservation::PreservedSection> {
    sections.iter().cloned().map(|mut section| {
        section.children = normalized_nodes(&section.children).into();
        section
    }).collect()
}

fn attribute<'a>(element: &'a PreservedElement, local_name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case(local_name)
        })
        .map(|attribute| attribute.value.as_str())
}

fn by_id<'a>(elements: &'a [&'a PreservedElement], id: &str) -> &'a PreservedElement {
    elements
        .iter()
        .copied()
        .find(|element| attribute(element, "id") == Some(id))
        .expect("synthetic source owner is retained")
}

fn count_nodes(nodes: &[PreservedNode], predicate: &impl Fn(&PreservedNode) -> bool) -> usize {
    nodes
        .iter()
        .map(|node| {
            usize::from(predicate(node))
                + match node {
                    PreservedNode::Element(element) => count_nodes(&element.children, predicate),
                    _ => 0,
                }
        })
        .sum()
}

fn direct_note_count(element: &PreservedElement) -> usize {
    element
        .children
        .iter()
        .filter(|node| matches!(
            node,
            PreservedNode::Element(child) if child.name.local_name == "ScriptNote"
        ))
        .count()
}

fn direct_note_owner_count(nodes: &[PreservedNode]) -> usize {
    nodes
        .iter()
        .map(|node| {
            let PreservedNode::Element(element) = node else {
                return 0;
            };
            let is_note_owner = if element.name.local_name == "Paragraph" {
                usize::from(direct_note_count(element) > 0)
            } else {
                0
            };
            is_note_owner + direct_note_owner_count(&element.children)
        })
        .sum()
}

fn collect_subtree_text(element: &PreservedElement, output: &mut String) {
    for node in &element.children {
        match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) => output.push_str(text.as_str()),
            PreservedNode::Element(child) => collect_subtree_text(child, output),
            PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {}
        }
    }
}

fn imported_synthetic() -> scriptcraft_core::fdx::FdxDocument {
    read_v6(SYNTHETIC).expect("the bounded synthetic compatibility fixture is importable")
}

fn assert_opaque_tokens_absent_from_semantic_layout_and_exports(
    project: &Project,
    opaque_tokens: &[&str],
) {
    let canonical = project.elements.iter().map(|paragraph| paragraph.text.as_str())
        .chain(project.title_page.iter().map(|paragraph| paragraph.text.as_str()))
        .collect::<String>();
    let source_settings = project.preservation.as_ref().unwrap().source_layout_settings().clone();
    let layout = ScreenplayLayout::build_with_composition(
        project,
        &LayoutInputs::default(),
        &source_settings,
        LayoutComposition::Print { include_title_page: true },
    ).expect("supported semantic projection lays out");
    let mut layout_text = String::new();
    for line in layout.pages.iter().flat_map(|page| page.lines.iter()) {
        layout_text.push_str(&line.display_text);
    }
    let pdf = export_pdf(project).expect("supported semantic projection exports to PDF");
    let pdf_text = std::str::from_utf8(&pdf).expect("generated PDF source is UTF-8");
    let plain = export_text(project).expect("supported semantic projection exports to plain text");

    for opaque in opaque_tokens {
        assert!(!canonical.contains(opaque), "opaque token projected to canonical text: {opaque}");
        assert!(!layout_text.contains(opaque), "opaque token projected to layout: {opaque}");
        assert!(!pdf_text.contains(opaque), "opaque token projected to PDF: {opaque}");
        assert!(!plain.text.contains(opaque), "opaque token projected to plain text: {opaque}");
    }
}

#[test]
fn c01_repeated_opaque_note_ids_remain_owned_and_do_not_become_typed_identities() {
    let imported = imported_synthetic();
    let elements = preserved_elements(&imported.project);
    let notes = elements
        .iter()
        .filter(|element| element.name.local_name == "ScriptNote")
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 2);
    assert!(notes.iter().all(|note| attribute(note, "ID") == Some("shared-note")));

    let preservation = imported.project.preservation.as_ref().expect("source tree is retained");
    assert!(!preservation.identities.iter().any(|identity| {
        matches!(
            identity.value.as_str(),
            "shared-note"
                | "opaque-repeat"
                | "opaque-image-id"
                | "note-payload-one"
                | "note-payload-two"
                | "spoofed-nested-paragraph"
        )
    }));
    assert!(elements.iter().any(|element| {
        element.name.local_name == "ScriptNoteDefinition"
            && attribute(element, "ID") == Some("shared-note")
    }));
    assert_eq!(direct_note_count(by_id(&elements, "body-one")), 2);
    assert!(notes.iter().all(|note| {
        note.children.iter().any(|child| matches!(
            child,
            PreservedNode::Element(paragraph) if paragraph.name.local_name == "Paragraph"
        ))
    }));
}

#[test]
fn c02_and_c03_unknown_paragraph_children_keep_order_and_never_project_note_payload() {
    let imported = imported_synthetic();
    let elements = preserved_elements(&imported.project);
    let body = by_id(&elements, "body-one");
    let child_element_names = body
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) => Some(element.name.local_name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        child_element_names,
        [
            "BeforeText",
            "Text",
            "BetweenText",
            "Text",
            "ScriptNote",
            "ScriptNote",
            "UserParagraphData",
            "AfterText",
        ]
    );
    assert!(imported.project.elements.iter().any(|paragraph| paragraph.text == "Alpha omega."));
    let semantic_text = imported
        .project
        .elements
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .chain(imported.project.title_page.iter().map(|paragraph| paragraph.text.as_str()))
        .collect::<String>();
    for payload in [
        "NOTE_ONLY_SENTINEL_ONE",
        "NOTE_ONLY_SENTINEL_TWO",
        "SPOOFED_ROOT_TEXT",
        "SPOOFED_NESTED_TEXT",
        "INLINE_ONLY_SENTINEL",
    ] {
        assert!(!semantic_text.contains(payload));
    }
    assert_eq!(imported.project.title_page[0].text, "Portable title text");
}

#[test]
fn c04_bookmark_remains_a_raw_attribute_on_its_body_title_and_dual_owners() {
    let imported = imported_synthetic();
    let elements = preserved_elements(&imported.project);
    for (id, expected) in [
        ("body-one", "body-bookmark"),
        ("title-one", "title-bookmark"),
        ("dual-outer", "outer-bookmark"),
        ("dual-left-cue", "member-bookmark"),
    ] {
        assert_eq!(attribute(by_id(&elements, id), "Bookmark"), Some(expected));
    }
}

#[test]
fn c05_c06_opaque_locations_and_ancestry_keep_fd_x_looking_descendants_as_payload() {
    let imported = imported_synthetic();
    let elements = preserved_elements(&imported.project);
    for local_name in [
        "RootExtra",
        "BeforeContent",
        "AfterContent",
        "BeforeTitle",
        "AfterTitle",
        "SettingsOpaque",
        "NestedOpaque",
        "InlineMarkup",
        "DualDialogue",
        "Image",
    ] {
        assert!(elements.iter().any(|element| element.name.local_name == local_name));
    }
    for (local_name, id) in [
        ("Paragraph", "spoofed-root-paragraph"),
        ("Paragraph", "spoofed-nested-paragraph"),
        ("Paragraph", "spoofed-settings-paragraph"),
    ] {
        assert!(elements.iter().any(|element| {
            element.name.local_name == local_name && attribute(element, "id") == Some(id)
        }));
    }
    assert!(elements.iter().any(|element| {
        element.name.local_name == "RootExtra"
            && element.children.iter().any(|child| matches!(
                child,
                PreservedNode::Element(paragraph) if paragraph.name.local_name == "Paragraph"
            ))
    }));
    assert!(imported.project.preservation.as_ref().unwrap().board_graph.entities.is_empty());
    assert!(!imported.project.elements.iter().any(|paragraph| {
        paragraph.text.contains("SPOOFED_") || paragraph.text.contains("SETTINGS_ONLY_SENTINEL")
    }));
}

#[test]
fn c07_namespaces_attributes_and_mixed_nodes_are_retained_in_the_source_tree() {
    let imported = imported_synthetic();
    let elements = preserved_elements(&imported.project);
    let root = elements
        .iter()
        .find(|element| element.name.local_name == "FinalDraft")
        .expect("root source node is retained");
    assert!(!root.namespace_bindings.is_empty());
    let vendor_element = elements
        .iter()
        .find(|element| element.name.local_name == "RootExtra")
        .expect("namespaced source element is retained");
    assert_eq!(vendor_element.name.namespace_uri.as_deref(), Some("urn:scriptcraft:fixture:vendor"));
    assert_eq!(attribute(vendor_element, "ID"), Some("opaque-repeat"));
    assert!(elements.iter().find(|element| element.name.local_name == "FinalDraft")
        .unwrap().attributes.iter().any(|attribute| {
            attribute.name.local_name == "root-flag"
                && attribute.name.namespace_uri.as_deref() == Some("urn:scriptcraft:fixture:vendor")
                && attribute.value == "kept"
        }));
    let body = by_id(&elements, "body-one");
    assert!(body.attributes.iter().any(|attribute| {
        attribute.name.local_name == "known-owner"
            && attribute.name.namespace_uri.as_deref() == Some("urn:scriptcraft:fixture:vendor")
            && attribute.value == "kept"
    }));

    let top_nodes = &imported.project.preservation.as_ref().unwrap().sections[0].children;
    assert!(count_nodes(top_nodes, &|node| matches!(node, PreservedNode::Comment(_))) >= 2);
    assert!(count_nodes(top_nodes, &|node| matches!(node, PreservedNode::CData(_))) >= 1);
    assert!(count_nodes(top_nodes, &|node| matches!(node, PreservedNode::ProcessingInstruction { .. })) >= 1);

    let raw_attributes = br#"<FinalDraft DocumentType="Script" Version="6" xmlns:v="urn:fixture:vendor"><Content><Paragraph Type="Action" id="raw-owner" Bookmark="raw-bookmark" arbitrary="owner-value"><Text hint="run-value" v:run-flag="retained">visible<Inline>not projected</Inline></Text></Paragraph></Content></FinalDraft>"#;
    let imported = read_v6(raw_attributes).expect("unknown attributes and inline source are safe opaque data");
    assert_eq!(imported.project.elements[0].text, "visible");
    let elements = preserved_elements(&imported.project);
    let owner = by_id(&elements, "raw-owner");
    assert_eq!(attribute(owner, "Bookmark"), Some("raw-bookmark"));
    assert_eq!(attribute(owner, "arbitrary"), Some("owner-value"));
    let text = owner.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
        _ => None,
    }).expect("managed Text element remains in source tree");
    assert_eq!(attribute(text, "hint"), Some("run-value"));
    assert!(text.attributes.iter().any(|attribute| {
        attribute.name.local_name == "run-flag"
            && attribute.name.namespace_uri.as_deref() == Some("urn:fixture:vendor")
            && attribute.value == "retained"
    }));
    assert!(text.children.iter().any(|node| matches!(
        node,
        PreservedNode::Element(element) if element.name.local_name == "Inline"
    )));
}

#[test]
fn c08_unknown_inline_text_topology_is_retained_without_semantic_promotion() {
    let imported = imported_synthetic();
    let elements = preserved_elements(&imported.project);
    let inline = elements
        .iter()
        .find(|element| element.name.local_name == "InlineMarkup")
        .expect("unknown inline element is retained");
    assert!(inline.children.iter().any(|child| matches!(
        child,
        PreservedNode::Element(element) if element.name.local_name == "Text"
    )));
    assert!(!imported.project.elements.iter().any(|paragraph| paragraph.text.contains("INLINE_ONLY_SENTINEL")));
}

#[test]
fn c09_recognized_user_paragraph_data_keeps_its_existing_owner_and_source_attributes() {
    let imported = imported_synthetic();
    let elements = preserved_elements(&imported.project);
    let attachments = elements
        .iter()
        .filter(|element| element.name.local_name == "UserParagraphData")
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attribute(attachments[0], "vendor-data"), Some("retained"));
    assert!(by_id(&elements, "body-one").children.iter().any(|node| matches!(
        node,
        PreservedNode::Element(element) if element.name.local_name == "UserParagraphData"
    )));
    let scene_owner = by_id(&elements, "scene-owner");
    assert!(scene_owner.children.iter().any(|node| matches!(
        node,
        PreservedNode::Element(element) if element.name.local_name == "SceneProperties"
    )));
}

#[test]
fn unsupported_title_and_settings_shapes_remain_source_only() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="managed"><Text>managed body</Text></Paragraph></Content><TitlePage><Content><TitleExtension><Paragraph Type="Title" id="opaque-title"><Text>TITLE_ONLY_SENTINEL</Text></Paragraph></TitleExtension></Content></TitlePage><ElementSettings Type="Action"><ParagraphSpec Type="Action" Alignment="Left"/><ParagraphSpec Type="Action" Alignment="Right"/></ElementSettings><ElementSettings Type="Action"><FontSpec Font="Conflicting role"/></ElementSettings></FinalDraft>"#;
    let imported = read_v6(source).expect("unsupported known-feature shapes remain safe source");
    assert_eq!(imported.project.elements[0].text, "managed body");
    assert!(imported.project.title_page.is_empty());
    let elements = preserved_elements(&imported.project);
    assert!(elements.iter().any(|element| {
        element.name.local_name == "TitleExtension"
            && element.children.iter().any(|child| matches!(
                child,
                PreservedNode::Element(paragraph) if attribute(paragraph, "id") == Some("opaque-title")
            ))
    }));
    assert!(!imported.project.preservation.as_ref().unwrap().identities.iter().any(|identity| {
        identity.value == "opaque-title"
    }));
    assert!(!imported.project.title_page.iter().any(|paragraph| {
        paragraph.text.contains("TITLE_ONLY_SENTINEL")
    }));
}

#[test]
fn c10_c11_actual_managed_identity_and_required_reference_ambiguity_remain_errors() {
    let duplicate_managed = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="same"><Text>one</Text></Paragraph><Paragraph Type="Action" id="same"><Text>two</Text></Paragraph></Content></FinalDraft>"#;
    assert_eq!(read_v6(duplicate_managed).err(), Some(FdxError::AmbiguousIdentity));

    let dangling_required_reference = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action"><Text>safe</Text></Paragraph></Content><DisplayBoard ID="board-one"><Picture ID="missing-image"/></DisplayBoard></FinalDraft>"#;
    assert_eq!(read_v6(dangling_required_reference).err(), Some(FdxError::DanglingReference));

    let opaque_board_lookalike = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action"><Text>safe</Text></Paragraph><VendorPayload><DisplayBoard ID="opaque-board"><Picture ID="missing-opaque-image"/></DisplayBoard></VendorPayload></Content></FinalDraft>"#;
    let imported = read_v6(opaque_board_lookalike)
        .expect("board-like descendants beneath opaque source do not create dependencies");
    let preservation = imported.project.preservation.as_ref().expect("opaque source is retained");
    assert!(preservation.board_graph.entities.is_empty());
    assert!(preservation.board_graph.references.is_empty());
}

#[test]
fn c12_malformed_hostile_resource_and_document_association_inputs_remain_bounded_errors() {
    assert_eq!(read_v6(&[0xff, 0xfe]).err(), Some(FdxError::InvalidEncoding));
    assert_eq!(read_v6(b"<FinalDraft><Content></FinalDraft>").err(), Some(FdxError::MalformedXml));
    assert_eq!(
        read_v6(br#"<!DOCTYPE FinalDraft SYSTEM "file:///unavailable"><FinalDraft DocumentType="Script" Version="6"><Content/></FinalDraft>"#).err(),
        Some(FdxError::ForbiddenDoctype)
    );
    assert!(read_v6(br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action"><Text>&unresolved;</Text></Paragraph></Content></FinalDraft>"#).is_err());
    assert!(read_v6(br#"<FinalDraft xmlns:xml="urn:invalid-xml-binding" DocumentType="Script" Version="6"><Content/></FinalDraft>"#).is_err());

    let oversized = vec![b' '; MAX_SOURCE_BYTES + 1];
    assert_eq!(read_v6(&oversized).err(), Some(FdxError::SourceTooLarge));
    let mut too_deep = String::from("<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content>");
    for _ in 0..(MAX_SOURCE_DEPTH + 2) {
        too_deep.push_str("<Opaque>");
    }
    for _ in 0..(MAX_SOURCE_DEPTH + 2) {
        too_deep.push_str("</Opaque>");
    }
    too_deep.push_str("</Content></FinalDraft>");
    assert_eq!(read_v6(too_deep.as_bytes()).err(), Some(FdxError::ResourceLimit));

    let too_many_events = format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content>{}</Content></FinalDraft>",
        "<Opaque/>".repeat(MAX_SOURCE_EVENTS + 1)
    );
    assert_eq!(read_v6(too_many_events.as_bytes()).err(), Some(FdxError::ResourceLimit));
    let too_much_text = format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\"><Text>{}</Text></Paragraph></Content></FinalDraft>",
        "x".repeat(MAX_SOURCE_TEXT_BYTES + 1)
    );
    assert_eq!(read_v6(too_much_text.as_bytes()).err(), Some(FdxError::ResourceLimit));
    let mut too_many_attributes = String::from(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Vendor",
    );
    for index in 0..=MAX_ATTRIBUTES_PER_ELEMENT {
        too_many_attributes.push_str(&format!(" a{index}=\"x\""));
    }
    too_many_attributes.push_str("/></Content></FinalDraft>");
    assert_eq!(read_v6(too_many_attributes.as_bytes()).err(), Some(FdxError::ResourceLimit));
    let oversized_image = format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content/><Image ID=\"image-one\" Length=\"{}\"/></FinalDraft>",
        MAX_IMAGE_BYTES + 1
    );
    assert_eq!(read_v6(oversized_image.as_bytes()).err(), Some(FdxError::ResourceLimit));

    assert_eq!(read_v6(b"<Other DocumentType=\"Script\" Version=\"6\"><Content/></Other>").err(), Some(FdxError::InvalidRoot));
    assert_eq!(read_v6(b"<FinalDraft DocumentType=\"Other\" Version=\"6\"><Content/></FinalDraft>").err(), Some(FdxError::InvalidRoot));
    assert_eq!(read_v6(b"<FinalDraft DocumentType=\"Script\"><Content/></FinalDraft>").err(), Some(FdxError::MissingVersion));
    assert_eq!(read_v6(b"<FinalDraft DocumentType=\"Script\" Version=\"six\"><Content/></FinalDraft>").err(), Some(FdxError::InvalidVersion));
    assert_eq!(read_v6(b"<FinalDraft DocumentType=\"Script\" Version=\"5\"><Content/></FinalDraft>").err(), Some(FdxError::UnsupportedVersion));
    assert_eq!(read_v6(b"<FinalDraft DocumentType=\"Script\" Version=\"6\"></FinalDraft>").err(), Some(FdxError::MissingContent));
    assert_eq!(read_v6(b"<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content/><Content/></FinalDraft>").err(), Some(FdxError::DuplicateSection));
    assert_eq!(read_v6(b"<FinalDraft DocumentType=\"Script\" Version=\"6\" version=\"6\"><Content/></FinalDraft>").err(), Some(FdxError::DuplicateVersion));
}

#[test]
fn b13_aggregate_image_payload_budget_rejects_multiple_individually_permitted_images() {
    assert!(MAX_IMAGE_BYTES * 3 > MAX_TOTAL_IMAGE_BYTES);
    let mut preservation = read_v6(
        br#"<FinalDraft DocumentType="Script" Version="6"><Content/></FinalDraft>"#,
    ).expect("minimal source imports before aggregate image budget construction")
        .project.preservation.expect("source preservation state exists");

    // Three per-image-maximum payloads are each valid in isolation, but their
    // aggregate exceeds MAX_TOTAL_IMAGE_BYTES. This exercises the aggregate
    // payload-byte invariant directly, without tripping the separate XML text
    // byte ceiling first.
    for index in 0..3u32 {
        let value = format!("aggregate-image-{index}");
        preservation.board_graph.entities.push(BoardEntity {
            identity: ForeignIdentity::new(ForeignIdDomain::Image, value)
                .expect("test image identity is valid"),
            kind: BoardEntityKind::Image,
            source_order: index + 1,
            anchor: None,
            image_payload: Some(SharedBytes::image(vec![index as u8; MAX_IMAGE_BYTES])),
            declared_image_length: Some(MAX_IMAGE_BYTES as u64),
            source_path: None,
        });
    }
    assert_eq!(
        preservation.validate(),
        Err(PreservationError::TotalImageBytesTooLarge),
    );
}

#[test]
fn b03_deterministic_unknown_sentinels_survive_each_eligible_source_location() {
    const BASE: &str = r#"<FinalDraft DocumentType="Script" Version="6" xmlns:v="urn:fixture:vendor">
<Content><!--CONTENT_SLOT--><Paragraph Type="Action" id="body-owner"><Text>VISIBLE_BODY<!--TEXT_SLOT--></Text><!--PARAGRAPH_SLOT--></Paragraph><v:ForeignParent><!--UNKNOWN_PARENT_SLOT--></v:ForeignParent></Content>
<!--ROOT_SLOT--><TitlePage><Content><!--TITLE_CONTENT_SLOT--><Paragraph Type="Title" id="title-owner"><Text>VISIBLE_TITLE</Text></Paragraph></Content></TitlePage>
<SpellCheckIgnoreLists><!--SETTINGS_SLOT--></SpellCheckIgnoreLists></FinalDraft>"#;
    let slots = [
        ("ROOT_SLOT", "root"),
        ("CONTENT_SLOT", "content"),
        ("TITLE_CONTENT_SLOT", "title-content"),
        ("PARAGRAPH_SLOT", "paragraph"),
        ("TEXT_SLOT", "text"),
        ("SETTINGS_SLOT", "settings"),
        ("UNKNOWN_PARENT_SLOT", "unknown-parent"),
    ];
    let mut source = BASE.to_owned();
    let mut sentinels = Vec::new();
    for (slot, location) in slots {
        let id = format!("wo005-{location}-sentinel");
        let token = format!("WO005_{}_OPAQUE_SENTINEL", location.to_ascii_uppercase());
        let element = format!("<v:UnknownPayload ID=\"{id}\" v:opaque-flag=\"retained\"><Payload>{token}</Payload></v:UnknownPayload>");
        let marker = format!("<!--{slot}-->");
        assert_eq!(source.matches(&marker).count(), 1, "slot marker is unique: {slot}");
        source = source.replacen(&marker, &element, 1);
        sentinels.push((id, token));
    }

    // Each deterministic injection is imported, directly written, and reread.
    // Assertions apply to each owner/location and to the shared aggregate.
    let imported = read_v6(source.as_bytes()).expect("all bounded source-location variants are safe");
    assert_eq!(imported.project.elements.len(), 1);
    assert_eq!(imported.project.elements[0].text, "VISIBLE_BODY");
    assert_eq!(imported.project.title_page.len(), 1);
    assert_eq!(imported.project.title_page[0].text, "VISIBLE_TITLE");
    let semantic_text = imported.project.elements.iter()
        .map(|paragraph| paragraph.text.as_str())
        .chain(imported.project.title_page.iter().map(|paragraph| paragraph.text.as_str()))
        .collect::<String>();
    let output = write_v6(&imported.project).expect("all deterministic opaque injections serialize");
    let reopened = read_v6(&output).expect("sentinel output passes the ordinary strict reread");
    let elements = preserved_elements(&reopened.project);
    for (id, token) in sentinels {
        assert!(!semantic_text.contains(&token));
        assert!(!reopened.project.elements.iter().any(|paragraph| paragraph.text.contains(&token)));
        assert!(!reopened.project.title_page.iter().any(|paragraph| paragraph.text.contains(&token)));
        let sentinel = elements.iter().find(|element| {
            element.name.local_name == "UnknownPayload" && attribute(element, "ID") == Some(id.as_str())
        }).expect("unique source sentinel remains in the preserved tree");
        assert!(sentinel.attributes.iter().any(|attribute| {
            attribute.name.local_name == "opaque-flag"
                && attribute.name.namespace_uri.as_deref() == Some("urn:fixture:vendor")
                && attribute.value == "retained"
        }));
        let mut subtree = String::new();
        collect_subtree_text(sentinel, &mut subtree);
        assert!(subtree.contains(&token));
    }
    assert_eq!(
        normalized_sections(&imported.project.preservation.as_ref().unwrap().sections),
        normalized_sections(&reopened.project.preservation.as_ref().unwrap().sections),
    );
}

#[test]
fn b04_unknown_paragraph_roles_remain_opaque_without_semantic_fabrication() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6" xmlns:v="urn:fixture:vendor"><Content>
<Paragraph Type="Action" id="known"><Text>SUPPORTED_TEXT</Text></Paragraph>
<Paragraph Type="FutureRole" id="future-role" v:role-metadata="opaque"><Text>FUTURE_ROLE_ONLY_SENTINEL</Text><ScriptNote ID="repeated"><Paragraph Type="Action"><Text>NESTED_ROLE_NOTE_SENTINEL</Text></Paragraph></ScriptNote></Paragraph>
</Content></FinalDraft>"#;
    let imported = read_v6(source).expect("unsupported paragraph role has an unambiguous opaque source owner");
    assert_eq!(imported.project.elements.len(), 1);
    assert_eq!(imported.project.elements[0].text, "SUPPORTED_TEXT");
    let elements = preserved_elements(&imported.project);
    let future = by_id(&elements, "future-role");
    assert_eq!(attribute(future, "Type"), Some("FutureRole"));
    assert!(future.attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.as_deref() == Some("urn:fixture:vendor")
            && attribute.name.local_name == "role-metadata"
            && attribute.value == "opaque"
    }));
    assert!(!imported.project.preservation.as_ref().unwrap().identities.iter().any(|identity| {
        identity.value == "future-role" || identity.value == "repeated"
    }));
    assert!(!imported.project.elements.iter().any(|paragraph| {
        paragraph.text.contains("FUTURE_ROLE_ONLY_SENTINEL")
            || paragraph.text.contains("NESTED_ROLE_NOTE_SENTINEL")
    }));
}

#[test]
fn b10_b11_opaque_payload_never_projects_to_canonical_layout_pdf_or_plain_text() {
    let imported = imported_synthetic();
    let project = &imported.project;
    let expected_visible = ["Alpha omega.", "Portable title text"];
    let opaque_tokens = [
        "NOTE_ONLY_SENTINEL_ONE", "NOTE_ONLY_SENTINEL_TWO", "SPOOFED_ROOT_TEXT",
        "SPOOFED_NESTED_TEXT", "INLINE_ONLY_SENTINEL", "SETTINGS_ONLY_SENTINEL",
        "opaque CDATA sentinel",
    ];
    let canonical = project.elements.iter().map(|paragraph| paragraph.text.as_str())
        .chain(project.title_page.iter().map(|paragraph| paragraph.text.as_str()))
        .collect::<String>();
    for visible in expected_visible {
        assert!(canonical.contains(visible));
    }
    assert_opaque_tokens_absent_from_semantic_layout_and_exports(project, &opaque_tokens);

    let alts = read_v6(ALTS_FIXTURE).expect("alternate fixture remains importable");
    assert_opaque_tokens_absent_from_semantic_layout_and_exports(
        &alts.project,
        &[
            "INLINE_ALT_SENTINEL", "MEMBER_ALT_SENTINEL", "COLLECTION_MEMBER_SENTINEL",
            "ORPHAN_SENTINEL",
        ],
    );

    let attachment_source = String::from_utf8_lossy(SYNTHETIC)
        .replace("<Payload/>", "<Payload>ATTACHMENT_ONLY_SENTINEL</Payload>");
    let attachment = read_v6(attachment_source.as_bytes())
        .expect("recognized opaque attachment payload remains importable");
    assert_opaque_tokens_absent_from_semantic_layout_and_exports(
        &attachment.project,
        &["ATTACHMENT_ONLY_SENTINEL"],
    );
}

#[test]
fn c14_reference_expectations_are_structural_and_do_not_embed_reference_text() {
    let imported = read_v6(REFERENCE_V6).expect("reference-backed expectations are enabled after import support lands");
    let elements = preserved_elements(&imported.project);
    let notes = elements
        .iter()
        .filter(|element| element.name.local_name == "ScriptNote")
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 2);
    assert!(notes.iter().all(|note| !note.children.is_empty()));
    let root_nodes = &imported.project.preservation.as_ref().unwrap().sections[0].children;
    assert_eq!(direct_note_owner_count(root_nodes), 2);
    let first_note_id = attribute(notes[0], "ID");
    assert!(first_note_id.is_some());
    assert!(notes.iter().all(|note| attribute(note, "ID") == first_note_id));

    let bookmarked = elements
        .iter()
        .filter(|element| element.name.local_name == "Paragraph" && attribute(element, "Bookmark").is_some())
        .count();
    assert_eq!(bookmarked, 1);
    let semantic_text = imported
        .project
        .elements
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .chain(imported.project.title_page.iter().map(|paragraph| paragraph.text.as_str()))
        .collect::<String>();
    for note in notes {
        let mut payload_text = String::new();
        collect_subtree_text(note, &mut payload_text);
        if payload_text.trim().len() > 8 {
            assert!(!semantic_text.contains(payload_text.trim()));
        }
    }
}

#[test]
fn r01_r02_r03_r04_direct_noop_write_reread_preserves_opaque_source_tree() {
    for source in [SYNTHETIC, REFERENCE_V6] {
        let imported = read_v6(source).expect("source imports before direct no-op serialization");
        let before = imported.project.preservation.as_ref().expect("source tree is retained");
        let output = write_v6(&imported.project).expect("unchanged source tree serializes");
        let reopened = read_v6(&output).expect("direct output rereads through the strict codec");
        let after = reopened.project.preservation.as_ref().expect("reread source tree is retained");

        assert_eq!(imported.project.title_page, reopened.project.title_page);
        assert_eq!(imported.project.elements, reopened.project.elements);
        assert_eq!(before.identities, after.identities);
        assert_eq!(normalized_sections(&before.sections), normalized_sections(&after.sections));
    }

    let imported = read_v6(SYNTHETIC).expect("synthetic source imports");
    let output = write_v6(&imported.project).expect("synthetic source serializes");
    let reopened = read_v6(&output).expect("synthetic output rereads");
    let elements = preserved_elements(&reopened.project);
    let body = by_id(&elements, "body-one");
    let child_names = body.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) => Some(element.name.local_name.as_str()),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(
        child_names,
        [
            "BeforeText", "Text", "BetweenText", "Text", "ScriptNote", "ScriptNote",
            "UserParagraphData", "AfterText",
        ]
    );
    assert_eq!(direct_note_count(body), 2);
    assert_eq!(attribute(body, "Bookmark"), Some("body-bookmark"));
    assert!(body.children.iter().any(|node| matches!(
        node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.children.iter().any(|child| matches!(
                child,
                PreservedNode::Element(inline) if inline.name.local_name == "InlineMarkup"
            ))
    )));
}

#[test]
fn r05_empty_managed_text_removal_does_not_remove_opaque_text_content_or_attributes() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6" xmlns:v="urn:fixture:vendor"><Content><Paragraph Type="Action" id="empty-owner" Bookmark="keep-bookmark"><Text/><Text hint="keep-empty-attribute"/><Text><Inline v:flag="keep"><Text>not screenplay text</Text></Inline><!-- keep-comment --><?keep data?></Text></Paragraph></Content></FinalDraft>"#;
    let imported = read_v6(source).expect("empty and opaque Text source imports");
    assert_eq!(imported.project.elements[0].text, "");
    let output = write_v6(&imported.project).expect("empty managed text reconciles without opaque loss");
    let reopened = read_v6(&output).expect("empty-text output rereads");
    let elements = preserved_elements(&reopened.project);
    let owner = by_id(&elements, "empty-owner");
    assert_eq!(attribute(owner, "Bookmark"), Some("keep-bookmark"));
    let text_elements = owner.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(text_elements.len(), 3, "an unchanged empty paragraph retains its source Text slots");
    assert_eq!(text_elements[0].children.len(), 0);
    assert_eq!(attribute(text_elements[1], "hint"), Some("keep-empty-attribute"));
    assert!(text_elements[2].children.iter().any(|node| matches!(
        node,
        PreservedNode::Element(element) if element.name.local_name == "Inline"
            && element.attributes.iter().any(|attribute| {
                attribute.name.namespace_uri.as_deref() == Some("urn:fixture:vendor")
                    && attribute.name.local_name == "flag"
                    && attribute.value == "keep"
            })
    )));
    assert!(text_elements[2].children.iter().any(|node| matches!(node, PreservedNode::Comment(_))));
    assert!(text_elements[2].children.iter().any(|node| matches!(node, PreservedNode::ProcessingInstruction { .. })));
}

#[test]
fn r05_clearing_managed_text_keeps_opaque_inline_nodes_and_owner_attributes() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6" xmlns:v="urn:fixture:vendor"><Content><Paragraph Type="Action" id="clear-owner" Bookmark="clear-bookmark"><Text hint="retain-run-attribute">managed-before<v:Inline keep="opaque"><Text>OPAQUE_INLINE_SENTINEL</Text></v:Inline><![CDATA[managed-after]]><!-- retain-comment --><?retain data?></Text><Text>fully managed sibling</Text><v:AfterText keep="slot"/></Paragraph></Content></FinalDraft>"#;
    let mut imported = read_v6(source).expect("mixed managed and opaque text source imports");
    assert_eq!(imported.project.elements[0].text, "managed-beforemanaged-after");
    imported.project.elements[0].text.clear();
    imported.project.elements[0].runs.clear();

    let output = write_v6(&imported.project).expect("managed empty-text reconciliation retains opaque content");
    let reopened = read_v6(&output).expect("cleared output passes strict reread");
    assert_eq!(reopened.project.elements[0].text, "");
    let elements = preserved_elements(&reopened.project);
    let owner = by_id(&elements, "clear-owner");
    assert_eq!(attribute(owner, "Bookmark"), Some("clear-bookmark"));
    let text_elements = owner.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(text_elements.len(), 1);
    assert_eq!(attribute(text_elements[0], "hint"), Some("retain-run-attribute"));
    assert!(text_elements[0].children.iter().any(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "Inline"
            && element.children.iter().any(|child| matches!(
                child,
                PreservedNode::Element(nested) if nested.name.local_name == "Text"
                    && nested.children.iter().any(|text| matches!(
                        text,
                        PreservedNode::Text(value) if value.as_str() == "OPAQUE_INLINE_SENTINEL"
                    ))
            ))
    )));
    assert!(text_elements[0].children.iter().any(|node| matches!(node, PreservedNode::Comment(_))));
    assert!(text_elements[0].children.iter().any(|node| matches!(node, PreservedNode::ProcessingInstruction { .. })));
    assert!(owner.children.iter().any(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "AfterText"
    )));
}
