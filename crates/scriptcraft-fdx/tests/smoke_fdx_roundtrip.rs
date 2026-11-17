#![forbid(unsafe_code)]

use scriptcraft_fdx::{read_v6, write_v6};
use scriptcraft_model::formatted_text::{parse_style_attribute, RunStyleFlags};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{Project, Role};

const SMOKE_FDX: &[u8] = include_bytes!("fixtures/smoke_screenplay.fdx");
const ROOT_OPAQUE_TEXT: &str = "retained root opaque data";
const ATTACHMENT_OPAQUE_TEXT: &str = "retained attachment opaque data";

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("FDX round-trip smoke uses valid bounded input"))
}

fn attribute<'a>(element: &'a PreservedElement, local_name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == local_name
        })
        .map(|attribute| attribute.value.as_str())
}

fn child_element<'a>(
    element: &'a PreservedElement,
    local_name: &str,
) -> Option<&'a PreservedElement> {
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child)
            if child.name.local_name == local_name
                && child.name.namespace_uri == element.name.namespace_uri =>
        {
            Some(child)
        }
        _ => None,
    })
}

fn direct_text(element: &PreservedElement) -> String {
    element
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn source_root(project: &Project) -> Option<&PreservedElement> {
    project
        .preservation
        .as_ref()?
        .sections
        .iter()
        .flat_map(|section| section.children.iter())
        .find_map(|node| match node {
            PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
            _ => None,
        })
}

#[test]
fn fdx_roundtrip_public_api_smoke() {
    let imported = valid(read_v6(SMOKE_FDX)).project;
    assert_eq!(imported.elements.len(), 2);
    let imported_scene = imported.elements[0].as_element();
    let imported_action = imported.elements[1].as_element();
    assert_eq!(imported_scene.id.as_str(), "fdx-element-1");
    assert_eq!(
        imported_scene
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-scene")
    );
    assert_eq!(imported_scene.role, Role::SceneHeading);
    assert_eq!(imported_scene.text, "INT. STUDIO — DAY");
    assert_eq!(imported_action.id.as_str(), "fdx-element-2");
    assert_eq!(
        imported_action
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-action")
    );
    assert_eq!(imported_action.role, Role::Action);
    assert_eq!(imported_action.text, "Café lights");
    assert_eq!(imported_action.runs.len(), 2);
    assert_eq!(imported_action.runs[0].text, "Café ");
    assert_eq!(imported_action.runs[1].text, "lights");
    let first_style = valid(parse_style_attribute(&imported_action.runs[0].attributes));
    let second_style = valid(parse_style_attribute(&imported_action.runs[1].attributes));
    assert_eq!(first_style.flags, RunStyleFlags::default());
    assert_eq!(
        second_style.flags,
        RunStyleFlags {
            bold: true,
            italics: true,
            ..RunStyleFlags::default()
        }
    );
    assert_eq!(imported.title_page.len(), 1);
    assert_eq!(imported.title_page[0].id.as_str(), "fdx-element-3");
    assert_eq!(
        imported.title_page[0]
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-title")
    );
    assert_eq!(imported.title_page[0].text, "Smoke Story");
    assert!(imported.direct_paragraph_overrides.is_empty());
    assert!(imported.dual_dialogue_paragraph_overrides.is_empty());
    assert!(imported.dual_dialogue_groups.is_empty());

    let serialized = valid(write_v6(&imported));
    let reopened = valid(read_v6(&serialized)).project;
    assert_eq!(reopened.validate(), Ok(()));

    assert_eq!(reopened.elements.len(), imported.elements.len());
    for (before, after) in imported.elements.iter().zip(&reopened.elements) {
        let before = before.as_element();
        let after = after.as_element();
        assert_eq!(after.id, before.id);
        assert_eq!(after.role, before.role);
        assert_eq!(after.text, before.text);
        assert_eq!(after.runs, before.runs);
        assert_eq!(after.source_provenance, before.source_provenance);
    }
    assert_eq!(reopened.title_page, imported.title_page);
    assert_eq!(
        reopened.direct_paragraph_overrides,
        imported.direct_paragraph_overrides
    );
    assert_eq!(
        reopened.dual_dialogue_paragraph_overrides,
        imported.dual_dialogue_paragraph_overrides
    );
    assert_eq!(
        reopened.dual_dialogue_groups,
        imported.dual_dialogue_groups
    );

    assert_eq!(reopened.elements.len(), 2);
    let reopened_scene = reopened.elements[0].as_element();
    let reopened_action = reopened.elements[1].as_element();
    assert_eq!(reopened_scene.id.as_str(), "fdx-element-1");
    assert_eq!(reopened_scene.role, Role::SceneHeading);
    assert_eq!(reopened_scene.text, "INT. STUDIO — DAY");
    assert_eq!(reopened_action.id.as_str(), "fdx-element-2");
    assert_eq!(reopened_action.role, Role::Action);
    assert_eq!(reopened_action.text, "Café lights");
    assert_eq!(reopened_action.runs.len(), 2);
    assert_eq!(reopened_action.runs[0].text, "Café ");
    assert_eq!(reopened_action.runs[1].text, "lights");
    assert_eq!(reopened.title_page.len(), 1);
    assert_eq!(reopened.title_page[0].id.as_str(), "fdx-element-3");
    assert_eq!(reopened.title_page[0].text, "Smoke Story");
    assert!(reopened.direct_paragraph_overrides.is_empty());
    assert!(reopened.dual_dialogue_paragraph_overrides.is_empty());
    assert!(reopened.dual_dialogue_groups.is_empty());

    for opaque_text in [ROOT_OPAQUE_TEXT, ATTACHMENT_OPAQUE_TEXT] {
        assert!(reopened.elements.iter().all(|owner| {
            let element = owner.as_element();
            !element.text.contains(opaque_text)
                && element
                    .runs
                    .iter()
                    .all(|run| !run.text.contains(opaque_text))
        }));
        assert!(reopened.title_page.iter().all(|paragraph| {
            !paragraph.text.contains(opaque_text)
                && paragraph
                    .runs
                    .iter()
                    .all(|run| !run.text.contains(opaque_text))
        }));
    }

    let root = source_root(&reopened)
        .unwrap_or_else(|| unreachable!("round-trip retains its FinalDraft root"));
    let future_data = child_element(root, "FutureData")
        .unwrap_or_else(|| unreachable!("round-trip retains root FutureData"));
    assert_eq!(attribute(future_data, "marker"), Some("smoke-root"));
    let payload = child_element(future_data, "Payload")
        .unwrap_or_else(|| unreachable!("root FutureData retains its Payload"));
    assert_eq!(direct_text(payload), ROOT_OPAQUE_TEXT);

    let content = child_element(root, "Content")
        .unwrap_or_else(|| unreachable!("round-trip retains its Content section"));
    let source_action = content
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(paragraph)
                if paragraph.name.local_name == "Paragraph"
                    && paragraph.name.namespace_uri == content.name.namespace_uri
                    && attribute(paragraph, "id") == Some("smoke-action") =>
            {
                Some(paragraph)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("round-trip retains the Action owner"));
    assert_eq!(attribute(source_action, "vendor-owner"), Some("retained"));
    let attachment = child_element(source_action, "UserParagraphData")
        .unwrap_or_else(|| unreachable!("Action retains its direct UserParagraphData"));
    let vendor_payload = child_element(attachment, "VendorPayload")
        .unwrap_or_else(|| unreachable!("UserParagraphData retains its VendorPayload"));
    assert_eq!(
        attribute(vendor_payload, "marker"),
        Some("smoke-attachment")
    );
    assert_eq!(direct_text(vendor_payload), ATTACHMENT_OPAQUE_TEXT);
}
