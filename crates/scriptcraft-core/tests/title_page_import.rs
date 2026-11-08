#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::fdx::{read_v6, FdxError};
use scriptcraft_core::preservation::{PreservedNode, SourceOwnership};
use scriptcraft_core::{TitlePageAlignment, TitlePageParagraph};
use std::collections::BTreeSet;

fn import(source: &str) -> scriptcraft_core::fdx::FdxDocument {
    read_v6(source.as_bytes())
        .unwrap_or_else(|_| unreachable!("test source is a bounded valid Version 6 document"))
}

fn managed_ids(nodes: &[PreservedNode], output: &mut Vec<String>) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            if let SourceOwnership::Managed(id) = &element.ownership {
                output.push(id.as_str().to_owned());
            }
            managed_ids(&element.children, output);
        }
    }
}

fn find_title_attribute<'a>(
    nodes: &'a [PreservedNode],
    element_name: &str,
    attribute_name: &str,
) -> Option<&'a str> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if element.name.local_name == element_name {
            if let Some(attribute) = element
                .attributes
                .iter()
                .find(|attribute| attribute.name.local_name == attribute_name)
            {
                return Some(attribute.value.as_str());
            }
        }
        if let Some(value) = find_title_attribute(&element.children, element_name, attribute_name) {
            return Some(value);
        }
    }
    None
}

fn title_fixture(paragraphs: &str, extra_title_nodes: &str, body: &str) -> String {
    format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content>{body}</Content><TitlePage><HeaderAndFooter><Header><Paragraph Type=\"Action\" id=\"header-p\"><DynamicLabel Type=\"Page #\"/><Text>header only</Text></Paragraph></Header><Footer><Paragraph Type=\"Action\" id=\"footer-p\"><Text>footer only</Text></Paragraph></Footer></HeaderAndFooter><Content>{paragraphs}</Content>{extra_title_nodes}</TitlePage></FinalDraft>"
    )
}

fn reference_like_paragraphs() -> String {
    let mut paragraphs = String::new();
    for index in 0..54 {
        let alignment = match index {
            0..=16 | 42..=43 => "Left",
            17..=41 => "Center",
            _ => "Full",
        };
        let id = format!("title-{index}");
        let paragraph = match index {
            17 => format!(
                "<Paragraph Alignment=\"{alignment}\" id=\"{id}\"><Text Style=\"Underline+AllCaps\">Script Title</Text></Paragraph>"
            ),
            21 => format!(
                "<Paragraph Alignment=\"{alignment}\" id=\"{id}\"><Text Style=\"\">Written by</Text></Paragraph>"
            ),
            24 => format!(
                "<Paragraph Alignment=\"{alignment}\" id=\"{id}\"><Text AdornmentStyle=\"0\" Font=\"Courier Final Draft\" RevisionID=\"0\" Size=\"12\" Style=\"\">Name </Text><Text AdornmentStyle=\"0\" Font=\"Courier Final Draft\" RevisionID=\"0\" Size=\"12\" Style=\"AllCaps\">of</Text><Text AdornmentStyle=\"0\" Font=\"Courier Final Draft\" RevisionID=\"0\" Size=\"12\" Style=\"\"> </Text><Text AdornmentStyle=\"0\" Font=\"Courier Final Draft\" RevisionID=\"0\" Size=\"12\" Style=\"Italic\">First</Text><Text AdornmentStyle=\"0\" Font=\"Courier Final Draft\" RevisionID=\"0\" Size=\"12\" Style=\"\"> </Text><Text AdornmentStyle=\"0\" Font=\"Courier Final Draft\" RevisionID=\"0\" Size=\"12\" Style=\"Underline\">Writer</Text></Paragraph>"
            ),
            29 => format!(
                "<Paragraph Alignment=\"{alignment}\" id=\"{id}\"><Text Style=\"\">Based on, If Any</Text></Paragraph>"
            ),
            52 => format!(
                "<Paragraph Alignment=\"{alignment}\" id=\"{id}\"><Text Style=\"\">Address</Text></Paragraph>"
            ),
            53 => format!(
                "<Paragraph Alignment=\"{alignment}\" id=\"{id}\"><Text Style=\"\">Phone Number</Text></Paragraph>"
            ),
            _ => format!("<Paragraph Alignment=\"{alignment}\" id=\"{id}\"/>")
        };
        paragraphs.push_str(&paragraph);
    }
    paragraphs
}

fn run_style(run: &scriptcraft_core::TextRun) -> Option<&str> {
    run.attributes
        .iter()
        .find(|attribute| attribute.name == "Style")
        .map(|attribute| attribute.value.as_str())
}

#[test]
fn imports_54_nested_title_occurrences_in_order_with_reference_projection() {
    let source = title_fixture(&reference_like_paragraphs(), "<TextState Scaling=\"100\"/>", "");
    let imported = import(&source);
    let title = &imported.project.title_page;

    assert_eq!(title.len(), 54);
    assert_eq!(title[17].text, "Script Title");
    assert_eq!(title[21].text, "Written by");
    assert_eq!(title[24].text, "Name of First Writer");
    assert_eq!(title[29].text, "Based on, If Any");
    assert_eq!(title[52].text, "Address");
    assert_eq!(title[53].text, "Phone Number");
    assert_eq!(title[0].formatting.alignment, Some(TitlePageAlignment::Left));
    assert_eq!(title[17].formatting.alignment, Some(TitlePageAlignment::Center));
    assert_eq!(title[42].formatting.alignment, Some(TitlePageAlignment::Left));
    assert_eq!(title[44].formatting.alignment, Some(TitlePageAlignment::Justified));
    assert_eq!(title[24].runs.len(), 6);
    assert_eq!(
        title[24]
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Name of First Writer"
    );
    assert_eq!(
        title[24].runs.iter().map(run_style).collect::<Vec<_>>(),
        vec![Some(""), Some("AllCaps"), Some(""), Some("Italic"), Some(""), Some("Underline")]
    );
    assert!(title[24].runs.iter().all(|run| {
        run.attributes.len() == 5
            && run.attributes.iter().map(|attribute| attribute.name.as_str()).collect::<Vec<_>>()
                == ["AdornmentStyle", "Font", "RevisionID", "Size", "Style"]
            && run.attributes[0].value == "0"
            && run.attributes[1].value == "Courier Final Draft"
            && run.attributes[2].value == "0"
            && run.attributes[3].value == "12"
    }));
    assert!(title.iter().all(|paragraph| paragraph.role.is_none()));
    assert_eq!(title[24].source_provenance.as_ref().map(|source| source.ordinal), Some(24));
}

#[test]
fn imports_body_action_runs_and_direct_alignment_without_changing_source_attributes() {
    let body = "<Paragraph Type=\"Action\" Alignment=\"Center\" id=\"body-action\"><Text Style=\"\">plain </Text><Text Style=\"Bold\">bold</Text><Text Style=\"Italic\"> italic</Text><Text Style=\"Underline\"> underline</Text></Paragraph>";
    let title = "<Paragraph Alignment=\"Right\" FirstIndent=\"0.25\" LeftIndent=\"1.00\" RightIndent=\"7.50\" Leading=\"Regular\" SpaceBefore=\"12\" Spacing=\"1\" StartsNewPage=\"Yes\" vendor-field=\"untouched\" id=\"tp-1\"><Text Style=\"Bold+CustomToken\" vendor-run=\"kept\">title</Text></Paragraph>";
    let source = title_fixture(title, "", body);
    let imported = import(&source);

    assert_eq!(imported.project.elements[0].text, "plain bold italic underline");
    assert_eq!(imported.project.elements[0].runs.len(), 4);
    assert_eq!(imported.project.title_page[0].formatting.alignment, Some(TitlePageAlignment::Right));
    assert_eq!(imported.project.title_page[0].formatting.starts_new_page, Some(true));
    assert_eq!(imported.project.title_page[0].runs[0].attributes[0].value, "Bold+CustomToken");
    let preservation = imported.project.preservation.as_ref().unwrap_or_else(|| unreachable!("FDX import retains preservation state"));
    let root_nodes = &preservation.sections[0].children;
    assert_eq!(find_title_attribute(root_nodes, "Paragraph", "vendor-field"), Some("untouched"));
    assert_eq!(find_title_attribute(root_nodes, "Text", "vendor-run"), Some("kept"));
    assert_eq!(find_title_attribute(root_nodes, "Paragraph", "Alignment"), Some("Center"));
}

#[test]
fn repeated_nested_title_foreign_ids_get_distinct_transient_ids_and_managed_ownership() {
    let repeated = (0..4)
        .map(|index| format!("<Paragraph Alignment=\"Full\" id=\"same-title-id\"><Text>occurrence {index}</Text></Paragraph>"))
        .collect::<String>();
    let imported = import(&title_fixture(
        &repeated,
        "<ExternalUse target=\"same-title-id\"/>",
        "",
    ));
    let title = &imported.project.title_page;

    assert_eq!(title.len(), 4);
    assert_eq!(
        title.iter().map(|paragraph| paragraph.text.as_str()).collect::<Vec<_>>(),
        vec!["occurrence 0", "occurrence 1", "occurrence 2", "occurrence 3"]
    );
    assert!(title.iter().all(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .is_some_and(|source| source.foreign_id.as_deref() == Some("same-title-id"))
    }));
    let transient_ids = title
        .iter()
        .map(|paragraph| paragraph.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(transient_ids.len(), 4);

    let preservation = imported.project.preservation.as_ref().unwrap_or_else(|| unreachable!("FDX import retains preservation state"));
    assert_eq!(
        preservation
            .identities
            .iter()
            .filter(|identity| identity.value == "same-title-id")
            .count(),
        1,
        "the repeated raw ID remains registered for external-use safety scanning"
    );
    let mut managed = Vec::new();
    for section in preservation.sections.iter() {
        managed_ids(&section.children, &mut managed);
    }
    assert_eq!(managed.len(), 4);
    assert_eq!(managed.into_iter().collect::<BTreeSet<_>>(), transient_ids);
}

#[test]
fn repeated_title_ids_cannot_shadow_body_or_header_identity_definitions() {
    let repeated = "<Paragraph id=\"collision\"><Text>first</Text></Paragraph><Paragraph id=\"collision\"><Text>second</Text></Paragraph>";
    let body_collision = title_fixture(
        repeated,
        "",
        "<Paragraph Type=\"Action\" id=\"collision\"><Text>body</Text></Paragraph>",
    );
    assert_eq!(read_v6(body_collision.as_bytes()).err(), Some(FdxError::AmbiguousIdentity));

    let header_collision = title_fixture(
        repeated,
        "",
        "<Paragraph Type=\"Action\" id=\"body\"><Text>body</Text></Paragraph>",
    )
    .replace("id=\"header-p\"", "id=\"collision\"");
    assert_eq!(read_v6(header_collision.as_bytes()).err(), Some(FdxError::AmbiguousIdentity));
}

#[test]
fn legacy_typed_title_is_projected_but_headers_dynamic_content_and_nested_state_stay_opaque() {
    let legacy = "<Paragraph Type=\"Action\" Alignment=\"Left\" id=\"legacy\"><Text>legacy title</Text></Paragraph>";
    let opaque = "<Tabstops><Tabstop Position=\"1\"/></Tabstops><TextState Scaling=\"88\"/>";
    let imported = import(&title_fixture(legacy, opaque, ""));

    assert_eq!(imported.project.title_page.len(), 1);
    assert_eq!(imported.project.title_page[0].role, Some(scriptcraft_core::Role::Action));
    assert_eq!(imported.project.title_page[0].text, "legacy title");
    let preservation = imported.project.preservation.as_ref().unwrap_or_else(|| unreachable!("FDX import retains preservation state"));
    let root_nodes = &preservation.sections[0].children;
    assert_eq!(find_title_attribute(root_nodes, "Paragraph", "id"), Some("header-p"));
    let mut managed = Vec::new();
    for section in preservation.sections.iter() {
        managed_ids(&section.children, &mut managed);
    }
    assert_eq!(managed.len(), 1);
    assert_eq!(
        imported.project.title_page[0]
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("legacy")
    );
    assert!(find_title_attribute(root_nodes, "TextState", "Scaling").is_some());
    assert!(find_title_attribute(root_nodes, "Tabstop", "Position").is_some());
    assert!(find_title_attribute(root_nodes, "DynamicLabel", "Type").is_some());
    assert!(find_title_attribute(root_nodes, "Paragraph", "id").is_some());
}

#[test]
fn empty_text_is_omitted_whitespace_is_kept_and_unsupported_mixed_children_reject() {
    let paragraphs = "<Paragraph id=\"empty-mix\"><Text Style=\"Bold\"/><Text> </Text><Text><![CDATA[word]]></Text></Paragraph>";
    let imported = import(&title_fixture(paragraphs, "", ""));
    let title = &imported.project.title_page[0];
    assert_eq!(title.text, " word");
    assert_eq!(title.runs.len(), 2);
    assert_eq!(title.runs[0].text, " ");
    assert_eq!(title.runs[1].text, "word");

    let nested_inline = "<Paragraph id=\"mixed-inline\"><Text>before<Bold/>after</Text></Paragraph>";
    assert_eq!(
        read_v6(title_fixture(nested_inline, "", "").as_bytes()).err(),
        Some(FdxError::UnsupportedConstruct)
    );
    let mixed_sibling = "<Paragraph id=\"mixed-sibling\"><Text>before</Text><DynamicLabel Type=\"Page #\"/></Paragraph>";
    assert_eq!(
        read_v6(title_fixture(mixed_sibling, "", "").as_bytes()).err(),
        Some(FdxError::UnsupportedConstruct)
    );
}

#[test]
fn invalid_supported_fields_remain_source_owned_and_unprojected() {
    let paragraph = "<Paragraph Alignment=\"Sideways\" FirstIndent=\"not-a-number\" LeftIndent=\"1.00\" RightIndent=\"7.50\" StartsNewPage=\"Maybe\" vendor=\"stay\" id=\"invalid-format\"><Text>content</Text></Paragraph>";
    let imported = import(&title_fixture(paragraph, "", ""));
    let title: &TitlePageParagraph = &imported.project.title_page[0];

    assert_eq!(title.formatting.alignment, None);
    assert_eq!(title.formatting.first_indent, None);
    assert_eq!(title.formatting.starts_new_page, None);
    let preservation = imported.project.preservation.as_ref().unwrap_or_else(|| unreachable!("FDX import retains preservation state"));
    let root_nodes = &preservation.sections[0].children;
    assert_eq!(find_title_attribute(root_nodes, "Paragraph", "Alignment"), Some("Sideways"));
    assert_eq!(find_title_attribute(root_nodes, "Paragraph", "FirstIndent"), Some("not-a-number"));
    assert_eq!(find_title_attribute(root_nodes, "Paragraph", "StartsNewPage"), Some("Maybe"));
    assert_eq!(find_title_attribute(root_nodes, "Paragraph", "vendor"), Some("stay"));
}
