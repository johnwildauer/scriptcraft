#![forbid(unsafe_code)]

use scriptcraft_fdx::{read_v6, write_v6};
use scriptcraft_model::formatted_text::{parse_style_attribute, RunStyleFlags};
use scriptcraft_model::preservation::{
    DependencyConfidence, NamespaceBinding, PreservedAttribute, PreservedElement, PreservedNode,
    SourceName, SourceOwnership,
};
use scriptcraft_model::{BodyOwner, ElementDefinition, ElementTypeKey, Project};

const SMOKE_FDX: &[u8] = include_bytes!("fixtures/smoke_screenplay.fdx");
const OPEN_SCOPE_FDX: &[u8] = include_bytes!("fixtures/open_element_settings.fdx");
const ACTIVE_DUAL_FDX: &[u8] = include_bytes!("fixtures/active_dual_dialogue.fdx");
const INACTIVE_DUAL_FDX: &[u8] = include_bytes!("fixtures/inactive_dual_dialogue.fdx");
const ROOT_OPAQUE_TEXT: &str = "retained root opaque data";
const ATTACHMENT_OPAQUE_TEXT: &str = "retained attachment opaque data";

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result
        .unwrap_or_else(|error| panic!("FDX round-trip smoke uses valid bounded input: {error:?}"))
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

fn source_root_mut(project: &mut Project) -> Option<&mut PreservedElement> {
    project
        .preservation
        .as_mut()?
        .sections
        .iter_mut()
        .flat_map(|section| section.children.iter_mut())
        .find_map(|node| match node {
            PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
            _ => None,
        })
}

fn source_element_by_id<'a>(
    element: &'a PreservedElement,
    id: &str,
) -> Option<&'a PreservedElement> {
    if attribute(element, "id") == Some(id) {
        return Some(element);
    }
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child) => source_element_by_id(child, id),
        _ => None,
    })
}

fn settings_type_labels(root: &PreservedElement) -> Vec<&str> {
    root.children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && element.name.namespace_uri == root.name.namespace_uri =>
            {
                attribute(element, "Type")
            }
            _ => None,
        })
        .collect()
}

fn derived_definition(project: &Project, key: &str) -> ElementDefinition {
    let mut definition = project
        .element_definitions()
        .get_by_label("Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("test project defines Action"));
    definition.key = ElementTypeKey::new(key).unwrap_or_else(|_| unreachable!());
    definition
}

#[test]
fn fdx_roundtrip_public_api_smoke() {
    let imported = valid(read_v6(SMOKE_FDX)).project;
    let original_bytes = imported
        .preservation
        .as_ref()
        .map(|state| state.source.original_bytes.clone());
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
    assert_eq!(imported_scene.element_type().as_str(), "Scene Heading");
    assert_eq!(imported_scene.text, "INT. STUDIO — DAY");
    assert_eq!(imported_action.id.as_str(), "fdx-element-2");
    assert_eq!(
        imported_action
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-action")
    );
    assert_eq!(imported_action.element_type().as_str(), "Action");
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
    assert_eq!(
        imported.title_page[0]
            .element_type
            .as_ref()
            .map(|key| key.as_str()),
        Some("Title")
    );
    assert!(imported.direct_paragraph_overrides.is_empty());
    assert!(imported.dual_dialogue_paragraph_overrides.is_empty());
    assert!(imported.dual_dialogue_groups.is_empty());

    let serialized = valid(write_v6(&imported));
    assert_eq!(
        imported
            .preservation
            .as_ref()
            .map(|state| state.source.original_bytes.clone()),
        original_bytes
    );
    let reopened = valid(read_v6(&serialized)).project;
    assert_eq!(reopened.validate(), Ok(()));

    assert_eq!(reopened.elements.len(), imported.elements.len());
    for (before, after) in imported.elements.iter().zip(&reopened.elements) {
        let before = before.as_element();
        let after = after.as_element();
        assert_eq!(after.id, before.id);
        assert_eq!(after.element_type(), before.element_type());
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
    assert_eq!(reopened.dual_dialogue_groups, imported.dual_dialogue_groups);
    assert_eq!(
        reopened.element_definitions(),
        imported.element_definitions()
    );

    assert_eq!(reopened.elements.len(), 2);
    let reopened_scene = reopened.elements[0].as_element();
    let reopened_action = reopened.elements[1].as_element();
    assert_eq!(reopened_scene.id.as_str(), "fdx-element-1");
    assert_eq!(reopened_scene.element_type().as_str(), "Scene Heading");
    assert_eq!(reopened_scene.text, "INT. STUDIO — DAY");
    assert_eq!(reopened_action.id.as_str(), "fdx-element-2");
    assert_eq!(reopened_action.element_type().as_str(), "Action");
    assert_eq!(reopened_action.text, "Café lights");
    assert_eq!(reopened_action.runs.len(), 2);
    assert_eq!(reopened_action.runs[0].text, "Café ");
    assert_eq!(reopened_action.runs[1].text, "lights");
    assert_eq!(reopened.title_page.len(), 1);
    assert_eq!(reopened.title_page[0].id.as_str(), "fdx-element-3");
    assert_eq!(reopened.title_page[0].text, "Smoke Story");
    assert_eq!(
        reopened.title_page[0]
            .element_type
            .as_ref()
            .map(|key| key.as_str()),
        Some("Title")
    );
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

#[test]
fn source_backed_writer_projects_ordered_definitions_and_arbitrary_types() {
    let mut project = valid(read_v6(SMOKE_FDX)).project;
    let original_bytes = project
        .preservation
        .as_ref()
        .map(|state| state.source.original_bytes.clone());
    let lyrics_key = ElementTypeKey::new("Lyrics").unwrap_or_else(|_| unreachable!());
    let title_key = ElementTypeKey::new("TitleCustom").unwrap_or_else(|_| unreachable!());
    let mut lyrics = derived_definition(&project, "Lyrics");
    lyrics.font.family = "Dynamic Writer Font".to_owned();
    lyrics.font.color = Some("#87878F8FE0E0".to_owned());
    lyrics.paragraph.starts_new_page = true;
    lyrics.behavior.paginate_as = Some("Lyrics".to_owned());
    lyrics.outline.level = 4;
    let title = derived_definition(&project, "TitleCustom");
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .rev()
        .collect::<Vec<_>>();
    if let Some(action) = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Action")
    {
        action.font.family = "Reconciled Existing Font".to_owned();
    }
    definitions.insert(0, title);
    definitions.insert(0, lyrics);
    assert!(project.replace_element_definitions(definitions).is_ok());
    BodyOwner::update(&mut project.elements[1], |element| {
        element.set_element_type(lyrics_key.clone());
    });
    project.title_page[0].element_type = Some(title_key.clone());

    let output = valid(write_v6(&project));
    assert_eq!(
        project
            .preservation
            .as_ref()
            .map(|state| state.source.original_bytes.clone()),
        original_bytes
    );
    let reopened = valid(read_v6(&output)).project;
    assert_eq!(
        reopened.element_definitions(),
        project.element_definitions()
    );
    assert_eq!(reopened.elements[1].element_type(), lyrics_key);
    assert_eq!(reopened.elements[1].text, project.elements[1].text);
    assert_eq!(reopened.elements[1].id, project.elements[1].id);
    assert_eq!(reopened.elements[1].runs, project.elements[1].runs);
    assert_eq!(
        reopened.elements[1]
            .as_element()
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        project.elements[1]
            .as_element()
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
    );
    assert_eq!(
        reopened.direct_paragraph_overrides,
        project.direct_paragraph_overrides
    );
    assert_eq!(reopened.title_page[0].element_type, Some(title_key.clone()));

    let root = source_root(&reopened)
        .unwrap_or_else(|| unreachable!("source-backed output retains its root"));
    let expected_order = project
        .element_definitions()
        .iter()
        .map(|definition| definition.key.as_str())
        .collect::<Vec<_>>();
    let actual_order = settings_type_labels(root);
    assert!(actual_order.len() >= expected_order.len());
    assert_eq!(
        &actual_order[..expected_order.len()],
        expected_order.as_slice()
    );
    assert_eq!(
        attribute(
            source_element_by_id(root, "smoke-title")
                .unwrap_or_else(|| unreachable!("TitlePage Content paragraph remains managed")),
            "Type"
        ),
        Some("TitleCustom")
    );
    let edited_action = source_element_by_id(root, "smoke-action")
        .unwrap_or_else(|| unreachable!("edited owner retains its foreign source identity"));
    assert_eq!(attribute(edited_action, "vendor-owner"), Some("retained"));
    assert!(child_element(edited_action, "UserParagraphData").is_some());

    let mut type_less_title = reopened;
    type_less_title.title_page[0].element_type = None;
    let type_less_output = valid(write_v6(&type_less_title));
    let type_less_reopened = valid(read_v6(&type_less_output)).project;
    assert_eq!(type_less_reopened.title_page[0].element_type, None);
    let root = source_root(&type_less_reopened)
        .unwrap_or_else(|| unreachable!("type-less title remains source-backed"));
    assert_eq!(
        attribute(
            source_element_by_id(root, "smoke-title")
                .unwrap_or_else(|| unreachable!("Content-only title source remains present")),
            "Type"
        ),
        None
    );

    let mut typed_again = type_less_reopened;
    typed_again.title_page[0].element_type = Some(title_key.clone());
    let typed_again_output = valid(write_v6(&typed_again));
    let typed_again_reopened = valid(read_v6(&typed_again_output)).project;
    assert_eq!(
        typed_again_reopened.title_page[0].element_type,
        Some(title_key)
    );
    let root = source_root(&typed_again_reopened)
        .unwrap_or_else(|| unreachable!("newly typed Content title remains source-backed"));
    assert_eq!(
        attribute(
            source_element_by_id(root, "smoke-title")
                .unwrap_or_else(|| unreachable!("Content-only title source remains present")),
            "Type"
        ),
        Some("TitleCustom")
    );
}

#[test]
fn source_backed_writer_keeps_inert_duplicates_and_cannot_resurrect_removed_definitions() {
    let mut project = valid(read_v6(OPEN_SCOPE_FDX)).project;
    let root = source_root_mut(&mut project)
        .unwrap_or_else(|| unreachable!("open fixture keeps its root"));
    let setting = root
        .children
        .iter_mut()
        .find_map(|node| match node {
            PreservedNode::Element(setting)
                if setting.name.local_name == "ElementSettings"
                    && attribute(setting, "Type") == Some("Lyrics")
                    && setting.anchor.is_some() =>
            {
                Some(setting)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("Lyrics winning setting remains anchored"));
    setting.namespace_bindings.push(NamespaceBinding {
        prefix: "vendor".to_owned(),
        namespace_uri: "urn:scriptcraft:verification".to_owned(),
    });
    setting.attributes.push(PreservedAttribute {
        name: SourceName {
            namespace_uri: Some("urn:scriptcraft:verification".to_owned()),
            local_name: "Flag".to_owned(),
            qualified_name: Some("vendor:Flag".to_owned()),
        },
        value: "keep-this-value".to_owned(),
    });
    setting
        .children
        .push(PreservedNode::Element(PreservedElement {
            name: SourceName {
                namespace_uri: Some("urn:scriptcraft:verification".to_owned()),
                local_name: "VendorSettingsPayload".to_owned(),
                qualified_name: Some("vendor:VendorSettingsPayload".to_owned()),
            },
            attributes: Vec::new(),
            namespace_bindings: Vec::new(),
            source_type: None,
            ownership: SourceOwnership::Opaque,
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: None,
            children: Vec::new().into(),
        }));
    let original_bytes = project
        .preservation
        .as_ref()
        .map(|state| state.source.original_bytes.clone());

    let no_op_output = valid(write_v6(&project));
    assert_eq!(
        project
            .preservation
            .as_ref()
            .map(|state| state.source.original_bytes.clone()),
        original_bytes
    );
    let no_op_reopened = valid(read_v6(&no_op_output)).project;
    assert_eq!(
        no_op_reopened.element_definitions(),
        project.element_definitions()
    );
    let no_op_root = source_root(&no_op_reopened)
        .unwrap_or_else(|| unreachable!("no-op output retains the source root"));
    assert_eq!(
        settings_type_labels(no_op_root)
            .iter()
            .filter(|label| **label == "Lyrics")
            .count(),
        2,
        "the winning Lyrics setting and its inert duplicate remain retained"
    );
    let lyrics_settings = no_op_root
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(setting)
                if setting.name.local_name == "ElementSettings"
                    && attribute(setting, "Type") == Some("Lyrics") =>
            {
                Some(setting)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lyrics_settings
            .iter()
            .filter(|setting| setting.anchor.is_some())
            .count(),
        1
    );
    assert_eq!(
        child_element(lyrics_settings[0], "FontSpec").and_then(|font| attribute(font, "Color")),
        Some("#87878F8FE0E0")
    );
    assert_eq!(
        child_element(lyrics_settings[1], "FontSpec").and_then(|font| attribute(font, "Color")),
        Some("#123456789ABC")
    );
    assert!(lyrics_settings[0]
        .namespace_bindings
        .iter()
        .any(|binding| binding.prefix == "vendor"
            && binding.namespace_uri == "urn:scriptcraft:verification"));
    assert!(lyrics_settings[0].attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.as_deref() == Some("urn:scriptcraft:verification")
            && attribute.name.local_name == "Flag"
            && attribute.value == "keep-this-value"
    }));
    assert!(lyrics_settings[0].children.iter().any(|node| {
        matches!(node, PreservedNode::Element(element)
            if element.name.namespace_uri.as_deref() == Some("urn:scriptcraft:verification")
                && element.name.local_name == "VendorSettingsPayload")
    }));
    let body = source_element_by_id(no_op_root, "open-body-general-derived")
        .unwrap_or_else(|| unreachable!("unknown source paragraph identity remains present"));
    assert_eq!(attribute(body, "vendor-owner"), Some("preserve"));
    assert_eq!(attribute(body, "Alignment"), Some("Center"));
    assert!(child_element(body, "VendorBodyPayload").is_some());
    assert!(child_element(body, "UserParagraphData").is_some());
    for (id, type_label) in [
        ("open-title-direct", "TitleFirst"),
        ("open-header-title", "HeaderOnlyTitle"),
        ("open-footer-title", "FooterOnlyTitle"),
    ] {
        assert_eq!(
            attribute(
                source_element_by_id(no_op_root, id)
                    .unwrap_or_else(|| unreachable!("out-of-Content title remains opaque")),
                "Type"
            ),
            Some(type_label)
        );
    }

    let notations = ElementTypeKey::new("Notations").unwrap_or_else(|_| unreachable!());
    for owner in &mut project.elements {
        if owner.element_type().as_str() == "Lyrics" {
            BodyOwner::update(owner, |element| element.set_element_type(notations.clone()));
        }
    }
    let remaining_definitions = project
        .element_definitions()
        .iter()
        .filter(|definition| definition.key.as_str() != "Lyrics")
        .cloned()
        .collect::<Vec<_>>();
    assert!(project
        .replace_element_definitions(remaining_definitions)
        .is_ok());

    let edited_output = valid(write_v6(&project));
    assert_eq!(
        project
            .preservation
            .as_ref()
            .map(|state| state.source.original_bytes.clone()),
        original_bytes,
        "source projection never mutates the original byte snapshot"
    );
    let edited_reopened = valid(read_v6(&edited_output)).project;
    assert!(!edited_reopened
        .element_definitions()
        .iter()
        .any(|definition| definition.key.as_str() == "Lyrics"));
    assert!(edited_reopened
        .elements
        .iter()
        .all(|owner| owner.element_type().as_str() != "Lyrics"));
    let edited_root = source_root(&edited_reopened)
        .unwrap_or_else(|| unreachable!("edited output retains its source root"));
    assert!(!settings_type_labels(edited_root).contains(&"Lyrics"));
}

#[test]
fn preservation_free_writer_emits_only_current_registry_and_open_types() {
    let mut project =
        scriptcraft_fdx::new_project().unwrap_or_else(|_| unreachable!("new project is valid"));
    let lyrics_key = ElementTypeKey::new("Lyrics").unwrap_or_else(|_| unreachable!());
    let title_key = ElementTypeKey::new("TitleCustom").unwrap_or_else(|_| unreachable!());
    let lyrics = derived_definition(&project, "Lyrics");
    let title_definition = derived_definition(&project, "TitleCustom");
    let required_base_labels = [
        "General",
        "Scene Heading",
        "Action",
        "Character",
        "Parenthetical",
        "Dialogue",
        "Transition",
        "Shot",
        "Cast List",
    ];
    let mut definitions = project
        .element_definitions()
        .iter()
        .filter(|definition| required_base_labels.contains(&definition.key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    definitions.push(lyrics);
    definitions.push(title_definition);
    assert!(project.replace_element_definitions(definitions).is_ok());
    BodyOwner::update(&mut project.elements[0], |element| {
        element.set_element_type(lyrics_key.clone());
    });
    for title in &mut project.title_page {
        title.element_type = None;
    }
    project.title_page[0].element_type = Some(title_key.clone());

    let output = valid(write_v6(&project));
    let reopened = valid(read_v6(&output)).project;
    assert_eq!(
        reopened.element_definitions(),
        project.element_definitions()
    );
    assert_eq!(
        reopened.element_definitions().len(),
        required_base_labels.len() + 2
    );
    assert_eq!(reopened.elements[0].element_type(), lyrics_key);
    assert_eq!(reopened.title_page[0].element_type, Some(title_key));
    let root = source_root(&reopened)
        .unwrap_or_else(|| unreachable!("new-document reread retains its generated root"));
    let expected_order = project
        .element_definitions()
        .iter()
        .map(|definition| definition.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(settings_type_labels(root), expected_order);
}

#[test]
fn source_backed_writer_keeps_out_of_content_title_types_opaque() {
    let project = valid(read_v6(OPEN_SCOPE_FDX)).project;
    let definitions = project
        .element_definitions()
        .iter()
        .map(|definition| definition.key.as_str())
        .collect::<Vec<_>>();
    for opaque_type in [
        "TitleFirst",
        "OpaqueTitle",
        "HeaderOnlyTitle",
        "FooterOnlyTitle",
    ] {
        assert!(!definitions.contains(&opaque_type));
    }
    assert!(project.title_page.iter().all(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            != Some("open-title-direct")
    }));

    let output = valid(write_v6(&project));
    let reopened = valid(read_v6(&output)).project;
    assert_eq!(
        reopened.element_definitions(),
        project.element_definitions()
    );
    let root = source_root(&reopened)
        .unwrap_or_else(|| unreachable!("source-backed opaque-title data remains present"));
    for (id, expected_type) in [
        ("open-title-direct", "TitleFirst"),
        ("open-title-opaque", "OpaqueTitle"),
        ("open-header-title", "HeaderOnlyTitle"),
        ("open-footer-title", "FooterOnlyTitle"),
    ] {
        assert_eq!(
            attribute(
                source_element_by_id(root, id)
                    .unwrap_or_else(|| unreachable!("opaque title source occurrence is retained")),
                "Type"
            ),
            Some(expected_type)
        );
    }
}

#[test]
fn fdx_dual_source_roundtrips_active_and_inactive_membership() {
    use scriptcraft_model::dual_dialogue::active_dual_dialogue_projection;

    let mut active = valid(read_v6(ACTIVE_DUAL_FDX)).project;
    assert_eq!(active.elements.len(), 4);
    let initial_group = active.dual_dialogue_groups[0].clone();
    assert_eq!(initial_group.members.len(), 4);
    assert!(active_dual_dialogue_projection(&active, &initial_group).is_some());
    let initial_member_ids = initial_group.members.clone();
    let original_source = active
        .preservation
        .as_ref()
        .map(|state| state.source.original_bytes.clone());

    let active_output = valid(write_v6(&active));
    assert_eq!(
        active
            .preservation
            .as_ref()
            .map(|state| state.source.original_bytes.clone()),
        original_source
    );
    let active_reread = valid(read_v6(&active_output)).project;
    assert!(scriptcraft_fdx::same_dual_dialogue_projection(
        &active,
        &active_reread
    ));
    assert_eq!(
        active_reread.dual_dialogue_groups[0].members,
        initial_member_ids
    );
    assert!(active_dual_dialogue_projection(
        &active_reread,
        &active_reread.dual_dialogue_groups[0]
    )
    .is_some());
    assert_eq!(active_reread.elements[1].runs.len(), 2);
    assert_eq!(
        active_reread.direct_paragraph_overrides,
        active.direct_paragraph_overrides
    );
    let active_root = source_root(&active_reread)
        .unwrap_or_else(|| unreachable!("active dual source retains its FDX root"));
    let active_wrapper = source_element_by_id(active_root, "active-dual-wrapper")
        .unwrap_or_else(|| unreachable!("active wrapper identity is retained"));
    assert_eq!(attribute(active_wrapper, "LeftIndent"), Some("0.25"));
    let active_wrapper_payload = child_element(
        child_element(active_wrapper, "UserParagraphData")
            .unwrap_or_else(|| unreachable!("wrapper attachment remains retained")),
        "VendorWrapperPayload",
    )
    .unwrap_or_else(|| unreachable!("wrapper opaque payload remains retained"));
    assert_eq!(direct_text(active_wrapper_payload), "wrapper payload");
    assert!(
        source_element_by_id(active_root, "active-left-cue").is_some_and(|member| attribute(
            member,
            "Alignment"
        ) == Some(
            "Center"
        ))
    );
    assert!(source_element_by_id(active_root, "active-left-line").is_some());
    let active_member = source_element_by_id(active_root, "active-left-cue")
        .unwrap_or_else(|| unreachable!("active member foreign identity is retained"));
    assert_eq!(
        direct_text(
            child_element(active_member, "VendorMemberPayload")
                .unwrap_or_else(|| unreachable!("member opaque descendant is retained"))
        ),
        "left cue payload"
    );
    let active_attachment = child_element(
        source_element_by_id(active_root, "active-left-line")
            .unwrap_or_else(|| unreachable!("active Dialogue identity is retained")),
        "UserParagraphData",
    )
    .unwrap_or_else(|| unreachable!("member attachment is retained"));
    assert_eq!(
        attribute(
            child_element(active_attachment, "VendorAttachment")
                .unwrap_or_else(|| unreachable!("member attachment payload is retained")),
            "marker"
        ),
        Some("left-attachment")
    );

    let inactive_type = ElementTypeKey::new("voiceCaseDistinct")
        .unwrap_or_else(|_| unreachable!("test element key is valid"));
    let mut inactive_definition = derived_definition(&active, inactive_type.as_str());
    inactive_definition.font.family = "Dual Member Font".to_owned();
    let mut definitions = active
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions.push(inactive_definition);
    assert!(active.replace_element_definitions(definitions).is_ok());
    BodyOwner::update(&mut active.elements[0], |element| {
        element.set_element_type(inactive_type.clone());
    });
    assert!(active_dual_dialogue_projection(&active, &active.dual_dialogue_groups[0]).is_none());

    let deactivated_output = valid(write_v6(&active));
    let deactivated = valid(read_v6(&deactivated_output)).project;
    assert!(scriptcraft_fdx::same_dual_dialogue_projection(
        &active,
        &deactivated
    ));
    assert_eq!(
        deactivated.dual_dialogue_groups[0].members,
        initial_member_ids
    );
    assert!(
        active_dual_dialogue_projection(&deactivated, &deactivated.dual_dialogue_groups[0])
            .is_none()
    );
    assert_eq!(deactivated.elements[0].element_type(), inactive_type);
    assert_eq!(deactivated.elements[0].text, active.elements[0].text);
    assert_eq!(deactivated.elements[0].runs, active.elements[0].runs);
    assert_eq!(
        deactivated.direct_paragraph_overrides,
        active.direct_paragraph_overrides
    );
    let deactivated_root = source_root(&deactivated)
        .unwrap_or_else(|| unreachable!("inactive writer output keeps the source wrapper"));
    let deactivated_wrapper = source_element_by_id(deactivated_root, "active-dual-wrapper")
        .unwrap_or_else(|| unreachable!("inactive wrapper foreign identity remains"));
    assert!(source_element_by_id(deactivated_root, "active-right-line").is_some());
    assert_eq!(attribute(deactivated_wrapper, "Type"), Some("General"));

    BodyOwner::update(&mut active.elements[0], |element| {
        element.set_element_type(ElementTypeKey::new("Character").expect("valid fixture label"));
    });
    assert!(active_dual_dialogue_projection(&active, &active.dual_dialogue_groups[0]).is_some());
    let reactivated = valid(read_v6(&valid(write_v6(&active)))).project;
    assert!(
        active_dual_dialogue_projection(&reactivated, &reactivated.dual_dialogue_groups[0])
            .is_some()
    );

    let imported_inactive = valid(read_v6(INACTIVE_DUAL_FDX)).project;
    let inactive_group = imported_inactive.dual_dialogue_groups[0].clone();
    assert_eq!(inactive_group.members.len(), 3);
    assert!(active_dual_dialogue_projection(&imported_inactive, &inactive_group).is_none());
    assert_eq!(
        imported_inactive.elements[0].element_type().as_str(),
        "Voice"
    );
    assert_eq!(
        imported_inactive.elements[1].element_type().as_str(),
        "voice"
    );
    let directly_inactive = valid(read_v6(&valid(write_v6(&imported_inactive)))).project;
    assert!(scriptcraft_fdx::same_dual_dialogue_projection(
        &imported_inactive,
        &directly_inactive
    ));
    assert_eq!(
        directly_inactive.dual_dialogue_groups[0].members,
        inactive_group.members
    );
    assert!(active_dual_dialogue_projection(
        &directly_inactive,
        &directly_inactive.dual_dialogue_groups[0]
    )
    .is_none());
    let inactive_root = source_root(&directly_inactive)
        .unwrap_or_else(|| unreachable!("directly imported inactive wrapper is retained"));
    assert!(source_element_by_id(inactive_root, "inactive-member-three").is_some());
    let inactive_wrapper = source_element_by_id(inactive_root, "inactive-dual-wrapper")
        .unwrap_or_else(|| unreachable!("inactive wrapper identity is retained"));
    assert!(child_element(inactive_wrapper, "UserParagraphData").is_some());
    assert!(
        source_element_by_id(inactive_root, "inactive-member-one").is_some_and(|member| {
            child_element(member, "VendorMemberPayload")
                .is_some_and(|payload| direct_text(payload) == "first opaque payload")
        })
    );

    // The preservation-free writer must emit source-preserving groups in flat
    // body order even if their current member keys have no active split.
    let mut preservation_free = imported_inactive;
    preservation_free.preservation = None;
    preservation_free.dual_dialogue_groups[0].wrapper = Default::default();
    for member in &mut preservation_free.elements {
        BodyOwner::update(member, |element| element.source_provenance = None);
    }
    let generated = valid(read_v6(&valid(write_v6(&preservation_free)))).project;
    assert_eq!(generated.elements.len(), preservation_free.elements.len());
    assert_eq!(generated.dual_dialogue_groups.len(), 1);
    assert_eq!(generated.dual_dialogue_groups[0].members.len(), 3);
    for (generated, expected) in generated.elements.iter().zip(&preservation_free.elements) {
        assert_eq!(generated.element_type(), expected.element_type());
        assert_eq!(generated.text, expected.text);
        assert_eq!(generated.runs, expected.runs);
    }
    assert!(
        active_dual_dialogue_projection(&generated, &generated.dual_dialogue_groups[0]).is_none()
    );
}
