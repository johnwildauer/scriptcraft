#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::formatted_text::{parse_style_attribute, RunStyleFlags};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{Project, Role};

const SMOKE_FDX: &[u8] = include_bytes!("fixtures/smoke_screenplay.fdx");
const ROOT_OPAQUE_TEXT: &str = "retained root opaque data";
const ATTACHMENT_OPAQUE_TEXT: &str = "retained attachment opaque data";

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("FDX import smoke uses valid bounded input"))
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
fn fdx_import_public_api_smoke() {
    let imported = valid(read_v6(SMOKE_FDX));
    let project = imported.project;
    assert_eq!(project.validate(), Ok(()));

    assert_eq!(project.elements.len(), 2);
    let scene = project.elements[0].as_element();
    let action = project.elements[1].as_element();
    assert_eq!(scene.id.as_str(), "fdx-element-1");
    assert_eq!(
        scene
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-scene")
    );
    assert_eq!(scene.role, Role::SceneHeading);
    assert_eq!(scene.text, "INT. STUDIO — DAY");
    assert_eq!(action.id.as_str(), "fdx-element-2");
    assert_eq!(
        action
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-action")
    );
    assert_eq!(action.role, Role::Action);
    assert_eq!(action.text, "Café lights");

    assert_eq!(action.runs.len(), 2);
    assert_eq!(action.runs[0].text, "Café ");
    assert_eq!(action.runs[1].text, "lights");
    let run_text = action
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    assert_eq!(run_text, "Café lights");
    assert_eq!(action.text, run_text);
    let first_style = valid(parse_style_attribute(&action.runs[0].attributes));
    let second_style = valid(parse_style_attribute(&action.runs[1].attributes));
    assert_eq!(first_style.flags, RunStyleFlags::default());
    assert_eq!(
        second_style.flags,
        RunStyleFlags {
            bold: true,
            italics: true,
            ..RunStyleFlags::default()
        }
    );
    assert!(project.elements.iter().all(|owner| {
        let element = owner.as_element();
        !element.text.contains(ATTACHMENT_OPAQUE_TEXT)
            && element
                .runs
                .iter()
                .all(|run| !run.text.contains(ATTACHMENT_OPAQUE_TEXT))
    }));

    assert_eq!(project.title_page.len(), 1);
    let title = &project.title_page[0];
    assert_eq!(title.id.as_str(), "fdx-element-3");
    assert_eq!(
        title
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-title")
    );
    assert_eq!(title.text, "Smoke Story");

    let root = source_root(&project)
        .unwrap_or_else(|| unreachable!("sanitized fixture retains its FinalDraft root"));
    let future_data = child_element(root, "FutureData")
        .unwrap_or_else(|| unreachable!("sanitized fixture retains root FutureData"));
    assert_eq!(attribute(future_data, "marker"), Some("smoke-root"));
    let payload = child_element(future_data, "Payload")
        .unwrap_or_else(|| unreachable!("root FutureData retains its Payload"));
    assert_eq!(direct_text(payload), ROOT_OPAQUE_TEXT);

    let content = child_element(root, "Content")
        .unwrap_or_else(|| unreachable!("sanitized fixture retains its Content section"));
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
        .unwrap_or_else(|| unreachable!("sanitized fixture retains the Action owner"));
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
