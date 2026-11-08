#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::fdx::{read_v6, write_v6, FdxDocument};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode};
use scriptcraft_core::{Project, RunAttribute, TextRun, TitlePageAlignment};
use std::collections::BTreeSet;

fn write_and_read(project: &Project) -> FdxDocument {
    let bytes = write_v6(project)
        .unwrap_or_else(|_| unreachable!("the test title-page project should serialize"));
    read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("the writer output should be readable"))
}

fn find_element<'a>(nodes: &'a [PreservedNode], name: &str) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if element.name.local_name == name {
            return Some(element);
        }
        if let Some(found) = find_element(&element.children, name) {
            return Some(found);
        }
    }
    None
}

fn find_element_by_id<'a>(
    nodes: &'a [PreservedNode],
    name: &str,
    id: &str,
) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if element.name.local_name == name
            && element
                .attributes
                .iter()
                .any(|attribute| attribute.name.local_name == "id" && attribute.value == id)
        {
            return Some(element);
        }
        if let Some(found) = find_element_by_id(&element.children, name, id) {
            return Some(found);
        }
    }
    None
}

fn paragraph_ids(element: &PreservedElement, output: &mut Vec<String>) {
    if element.name.local_name == "Paragraph" {
        if let Some(id) = element
            .attributes
            .iter()
            .find(|attribute| attribute.name.local_name == "id")
        {
            output.push(id.value.clone());
        }
    }
    for child in &element.children {
        if let PreservedNode::Element(child) = child {
            paragraph_ids(child, output);
        }
    }
}

fn count_foreign_id(nodes: &[PreservedNode], id: &str) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            PreservedNode::Element(element) => {
                usize::from(element.attributes.iter().any(|attribute| {
                    attribute.name.local_name == "id" && attribute.value == id
                })) + count_foreign_id(&element.children, id)
            }
            _ => 0,
        })
        .sum()
}

fn title_fixture() -> String {
    "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" Alignment=\"Center\" id=\"body-id\"><Text>body</Text><Text Style=\"Bold\"> run</Text></Paragraph></Content><TitlePage><HeaderAndFooter vendor-hf=\"keep\"><Header><OpaqueHeader value=\"stay\"/></Header><Footer/></HeaderAndFooter><Content><Paragraph Alignment=\"Center\" vendor-p=\"keep\" id=\"same-title-id\"><Text Style=\"Underline+AllCaps\" vendor-run=\"keep\"><![CDATA[Title]]></Text><Text Style=\"Bold\"> tail</Text></Paragraph><Paragraph id=\"same-title-id\"/><Paragraph id=\"remove-id\"><Text Style=\"Italic\">remove me</Text></Paragraph></Content><TextState Scaling=\"80\" Selection=\"1,2\" ShowInvisibles=\"Yes\" vendor-state=\"keep\"/><Paragraph Type=\"Action\" id=\"legacy-title-id\"><Text>legacy</Text></Paragraph></TitlePage><VendorSection vendor-root=\"keep\"><Opaque/></VendorSection></FinalDraft>".to_owned()
}

#[test]
fn preservation_free_default_project_scene_properties_are_readable_and_preserved() {
    let project = Project::new().unwrap_or_else(|_| unreachable!("default project is valid"));
    let bytes = write_v6(&project)
        .unwrap_or_else(|_| unreachable!("default generated FDX must pass its output re-read"));
    let xml = String::from_utf8(bytes.clone())
        .unwrap_or_else(|_| unreachable!("FDX writer emits UTF-8"));
    assert!(xml.contains("<SceneProperties"));

    let reread = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("generated SceneProperties is bounded and readable"));
    assert_eq!(reread.project.elements[0].role, scriptcraft_core::Role::SceneHeading);
    assert!(reread.project.elements[0].text.is_empty());
    let preservation = reread
        .project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("direct output re-read retains its source tree"));
    assert!(find_element(&preservation.sections[0].children, "SceneProperties").is_some());
}

#[test]
fn title_page_generated_shapes_are_independently_bounded_importable_and_preserved() {
    let fragments = [
        (
            "header and footer",
            "<HeaderAndFooter FooterFirstPage=\"Yes\" HeaderVisible=\"Yes\"><Header><Paragraph id=\"header-p\"><Text>header</Text></Paragraph></Header><Footer><Paragraph id=\"footer-p\"/></Footer></HeaderAndFooter>",
        ),
        (
            "nested Content and a Type-less Paragraph",
            "<Content><Paragraph Alignment=\"Center\" OutlineLevel=\"1\" id=\"title-p\"><Text>title</Text></Paragraph></Content>",
        ),
        (
            "nested TextState",
            "<Content/><TextState Scaling=\"100\" Selection=\"0,0\" ShowInvisibles=\"No\"/>",
        ),
        (
            "direct paragraph attributes",
            "<Content><Paragraph Alignment=\"Right\" FirstIndent=\"0.00\" LeftIndent=\"1.00\" OutlineLevel=\"1\" RightIndent=\"0.75\" SpaceBefore=\"0\" Spacing=\"1\" StartsNewPage=\"No\" id=\"formatted-p\"><Text>formatted</Text></Paragraph></Content>",
        ),
        (
            "empty Text",
            "<Content><Paragraph id=\"empty-p\"><Text Style=\"Bold\"/></Paragraph></Content>",
        ),
        (
            "nonempty Text with ordered run attributes",
            "<Content><Paragraph id=\"run-p\"><Text Font=\"Courier Final Draft\" RevisionID=\"0\" Size=\"12\" Style=\"Underline+AllCaps\">run</Text></Paragraph></Content>",
        ),
    ];

    for (label, title_page) in fragments {
        let source = format!(
            "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content/><TitlePage>{title_page}</TitlePage></FinalDraft>"
        );
        let imported = read_v6(source.as_bytes())
            .unwrap_or_else(|_| unreachable!("partitioned {label} fixture is supported"));
        let output = write_v6(&imported.project)
            .unwrap_or_else(|_| unreachable!("partitioned {label} source is retained on write"));
        let reread = read_v6(&output)
            .unwrap_or_else(|_| unreachable!("partitioned {label} writer output is readable"));
        assert_eq!(
            reread.project.title_page,
            imported.project.title_page,
            "bounded TitlePage semantics remain stable for {label}"
        );
        assert!(reread.project.preservation.is_some());
    }
}

#[test]
fn new_document_generates_reference_shaped_title_page_with_fresh_paragraph_ids() {
    let project = Project::new().unwrap_or_else(|_| unreachable!("default project is valid"));
    let written = write_and_read(&project);
    let preservation = written
        .project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("generated FDX is source-backed after re-read"));
    let title_page = find_element(&preservation.sections[0].children, "TitlePage")
        .unwrap_or_else(|| unreachable!("new FDX contains TitlePage"));
    let header_footer = find_element(&title_page.children, "HeaderAndFooter")
        .unwrap_or_else(|| unreachable!("new TitlePage contains HeaderAndFooter"));
    let content = find_element(&title_page.children, "Content")
        .unwrap_or_else(|| unreachable!("new TitlePage contains Content"));
    let nested_state = title_page
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "TextState" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("new TitlePage has nested TextState"));

    assert_eq!(written.project.title_page.len(), 54);
    assert_eq!(content.children.len(), 54);
    assert_eq!(written.project.title_page[17].text, "Script Title");
    assert_eq!(written.project.title_page[17].formatting.alignment, Some(TitlePageAlignment::Center));
    assert_eq!(written.project.title_page[17].runs[0].attributes[4].value, "Underline+AllCaps");
    assert_eq!(written.project.title_page[21].text, "Written by");
    assert_eq!(written.project.title_page[24].text, "Name of First Writer");
    assert_eq!(written.project.title_page[29].text, "Based on, If Any");
    assert_eq!(written.project.title_page[52].text, "Address");
    assert_eq!(written.project.title_page[53].text, "Phone Number");
    assert_eq!(
        header_footer
            .attributes
            .iter()
            .map(|attribute| (attribute.name.local_name.as_str(), attribute.value.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("FooterFirstPage", "Yes"),
            ("FooterVisible", "No"),
            ("HeaderFirstPage", "No"),
            ("HeaderVisible", "Yes"),
            ("StartingPage", "1"),
        ]
    );
    assert_eq!(nested_state.attributes[0].value, "100");
    assert_eq!(nested_state.attributes[1].value, "0,0");
    assert_eq!(nested_state.attributes[2].value, "No");

    let mut generated_ids = Vec::new();
    paragraph_ids(title_page, &mut generated_ids);
    let distinct = generated_ids.iter().collect::<BTreeSet<_>>();
    assert_eq!(generated_ids.len(), 56);
    assert_eq!(distinct.len(), generated_ids.len());
    assert!(generated_ids.iter().all(|id| {
        id.len() == 36
            && id.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
            })
    }));
    assert!(!generated_ids.iter().any(|id| {
        id == "6b076af3-b05b-461b-9679-7959dbe3ed1d"
            || id == "a359adb8-0574-45ea-abc4-381ca624a326"
    }));
}

#[test]
fn source_backed_title_reconciliation_keeps_type_runs_repeated_ids_and_opaque_source() {
    let mut project = read_v6(title_fixture().as_bytes())
        .unwrap_or_else(|_| unreachable!("fixture is bounded valid FDX"))
        .project;
    assert_eq!(project.title_page.len(), 4);
    assert_eq!(project.title_page[0].role, None);
    assert_eq!(project.title_page[3].role, Some(scriptcraft_core::Role::Action));
    assert_eq!(project.title_page[0].runs.len(), 2);
    project.title_page[0].text = "Renamed title tail".to_owned();
    project.title_page[0].runs[0].text = "Renamed title".to_owned();
    project.title_page[1].text = "Typed into childless paragraph".to_owned();
    project.title_page[2].text.clear();
    project.title_page[2].runs.clear();
    project.title_page[3].text = "legacy edited".to_owned();
    project.title_page[3].runs[0].text = "legacy edited".to_owned();

    let written = write_and_read(&project);
    assert_eq!(written.project.title_page[0].text, "Renamed title tail");
    assert_eq!(written.project.title_page[1].text, "Typed into childless paragraph");
    assert!(written.project.title_page[2].text.is_empty());
    assert_eq!(written.project.title_page[3].text, "legacy edited");
    assert_eq!(written.project.title_page[0].runs.len(), 2);
    assert_eq!(written.project.title_page[0].runs[0].attributes[0].value, "Underline+AllCaps");
    assert_eq!(written.project.title_page[0].runs[0].attributes[1].value, "keep");
    assert_eq!(written.project.elements[0].role, scriptcraft_core::Role::Action);
    assert_eq!(written.project.elements[0].runs.len(), 2);
    assert_eq!(written.project.elements[0].runs[1].attributes[0].value, "Bold");

    let preservation = written
        .project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("source-backed output has preservation state"));
    let root = &preservation.sections[0].children;
    assert_eq!(count_foreign_id(root, "same-title-id"), 2);
    let first = find_element_by_id(root, "Paragraph", "same-title-id")
        .unwrap_or_else(|| unreachable!("repeated title ID remains in source"));
    assert!(first.attributes.iter().any(|attribute| {
        attribute.name.local_name == "vendor-p" && attribute.value == "keep"
    }));
    let header_footer = find_element(root, "HeaderAndFooter")
        .unwrap_or_else(|| unreachable!("source HeaderAndFooter remains"));
    assert!(header_footer
        .attributes
        .iter()
        .any(|attribute| attribute.name.local_name == "vendor-hf"));
    let nested_state = find_element(root, "TextState")
        .unwrap_or_else(|| unreachable!("source nested TextState remains"));
    assert!(nested_state
        .attributes
        .iter()
        .any(|attribute| attribute.name.local_name == "vendor-state"));
    let vendor = find_element(root, "VendorSection")
        .unwrap_or_else(|| unreachable!("unrelated root section remains"));
    assert!(vendor
        .attributes
        .iter()
        .any(|attribute| attribute.name.local_name == "vendor-root"));
    assert!(written.project.preservation.as_ref().is_some_and(|state| {
        state
            .sections
            .iter()
            .flat_map(|section| section.children.iter())
            .any(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "TitlePage"))
    }));
    let output_bytes = write_v6(&written.project)
        .unwrap_or_else(|_| unreachable!("the preserved output serializes again"));
    let output_text = String::from_utf8(output_bytes)
        .unwrap_or_else(|_| unreachable!("FDX output uses UTF-8"));
    assert!(output_text.contains("<![CDATA[Renamed title]]>"));
}

#[test]
fn empty_and_childless_titles_reconcile_text_children_in_the_paragraph_namespace() {
    let source = "<FinalDraft xmlns=\"urn:scriptcraft:test\" DocumentType=\"Script\" Version=\"6\"><Content/><TitlePage><Content><Paragraph id=\"empty-child\"/><Paragraph id=\"remove-child\"><Text Style=\"Italic\">remove</Text></Paragraph></Content></TitlePage></FinalDraft>";
    let mut project = read_v6(source.as_bytes())
        .unwrap_or_else(|_| unreachable!("namespaced fixture is bounded valid FDX"))
        .project;
    project.title_page[0].text = "entered".to_owned();
    project.title_page[1].text.clear();
    project.title_page[1].runs.clear();

    let written = write_and_read(&project);
    assert_eq!(written.project.title_page[0].text, "entered");
    assert!(written.project.title_page[1].text.is_empty());
    let preservation = written
        .project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("re-read output has preservation state"));
    let entered = find_element_by_id(&preservation.sections[0].children, "Paragraph", "empty-child")
        .unwrap_or_else(|| unreachable!("childless title Paragraph remains"));
    let text = entered
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("nonempty title has one Text child"));
    assert_eq!(text.name.namespace_uri.as_deref(), Some("urn:scriptcraft:test"));
    let removed = find_element_by_id(
        &preservation.sections[0].children,
        "Paragraph",
        "remove-child",
    )
    .unwrap_or_else(|| unreachable!("emptied Paragraph remains"));
    assert!(!removed.children.iter().any(|node| {
        matches!(node, PreservedNode::Element(element) if element.name.local_name == "Text")
    }));
}

#[test]
fn source_run_attributes_are_retained_when_only_canonical_run_text_changes() {
    let mut project = read_v6(title_fixture().as_bytes())
        .unwrap_or_else(|_| unreachable!("fixture is bounded valid FDX"))
        .project;
    project.title_page[0].runs[0].attributes = vec![
        RunAttribute {
            name: "Style".to_owned(),
            value: "Bold+VendorToken".to_owned(),
        },
        RunAttribute {
            name: "vendor-run".to_owned(),
            value: "keep".to_owned(),
        },
    ];
    project.title_page[0].runs[0].text = "Restyled".to_owned();
    project.title_page[0].text = "Restyled tail".to_owned();

    let written = write_and_read(&project);
    assert_eq!(written.project.title_page[0].runs[0].text, "Restyled");
    assert_eq!(written.project.title_page[0].runs[0].attributes[0].value, "Bold+VendorToken");
    assert_eq!(written.project.title_page[0].runs[0].attributes[1].value, "keep");
}
