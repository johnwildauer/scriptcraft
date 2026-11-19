#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::formatted_text::{parse_style_attribute, RunStyleFlags};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_model::Project;

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

fn child_element_with_attribute<'a>(
    element: &'a PreservedElement,
    local_name: &str,
    attribute_name: &str,
    expected_value: &str,
) -> Option<&'a PreservedElement> {
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child)
            if child.name.local_name == local_name
                && child.name.namespace_uri == element.name.namespace_uri
                && attribute(child, attribute_name) == Some(expected_value) =>
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
    assert_eq!(imported.import_diagnostics, Default::default());
    let project = imported.project;
    assert_eq!(project.validate(), Ok(()));
    assert_eq!(
        project
            .element_definitions()
            .iter()
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Scene Heading",
            "Action",
            "Title",
            "General",
            "Character",
            "Parenthetical",
            "Dialogue",
            "Transition",
            "Shot",
            "Cast List",
        ]
    );
    assert!(project
        .element_definitions()
        .iter()
        .all(|definition| definition.validate().is_ok()));
    assert!(project
        .element_definitions()
        .get_by_label("Title")
        .is_some_and(|definition| definition.capabilities()
            == scriptcraft_model::element_definition::SemanticCapabilities::default()));

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
    assert_eq!(scene.element_type().as_str(), "Scene Heading");
    assert_eq!(scene.text, "INT. STUDIO — DAY");
    assert_eq!(action.id.as_str(), "fdx-element-2");
    assert_eq!(
        action
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("smoke-action")
    );
    assert_eq!(action.element_type().as_str(), "Action");
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
    assert_eq!(
        title.element_type.as_ref().map(|key| key.as_str()),
        Some("Title")
    );

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

#[test]
fn import_normalizes_only_winning_managed_black_white_font_colors() {
    let fixture = std::str::from_utf8(SMOKE_FDX)
        .unwrap_or_else(|_| unreachable!("sanitized import fixture is UTF-8"));
    let settings = concat!(
        "  <ElementSettings Type=\"General\"><FontSpec Color=\"#000000000000\" ",
        "Background=\"keep-background\" Vendor=\"keep-vendor\"/></ElementSettings>\n",
        "  <ElementSettings Type=\"Title\"><FontSpec Color=\"#87878F8FE0E0\"/></ElementSettings>\n",
        "  <ElementSettings Type=\"General\"><FontSpec Color=\"#FfFfFfFfFfFf\"/></ElementSettings>\n",
        "  <ElementSettings Type=\"Ambiguous\"><FontSpec Color=\"#000000000000\"/>",
        "<FontSpec Color=\"#FfFfFfFfFfFf\"/></ElementSettings>\n",
    );
    let source = fixture.replace("  <Content>", &format!("{settings}  <Content>"));
    assert_ne!(source, fixture);

    let imported = valid(read_v6(source.as_bytes()));
    let project = imported.project;
    assert_eq!(
        project
            .element_definitions()
            .get_by_label("General")
            .and_then(|definition| definition.font.color.as_deref()),
        None
    );
    assert_eq!(
        project
            .element_definitions()
            .get_by_label("Title")
            .and_then(|definition| definition.font.color.as_deref()),
        Some("#87878F8FE0E0")
    );
    assert_eq!(
        project
            .element_definitions()
            .get_by_label("Ambiguous")
            .and_then(|definition| definition.font.color.as_deref()),
        None
    );

    let root = source_root(&project)
        .unwrap_or_else(|| unreachable!("retained source contains its FinalDraft root"));
    let settings = root
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "ElementSettings" => {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settings.len(), 4);

    let general_font = child_element(&settings[0], "FontSpec")
        .unwrap_or_else(|| unreachable!("winning General retains its FontSpec"));
    assert_eq!(attribute(general_font, "Color"), None);
    assert_eq!(
        attribute(general_font, "Background"),
        Some("keep-background")
    );
    assert_eq!(attribute(general_font, "Vendor"), Some("keep-vendor"));

    let custom_font = child_element(&settings[1], "FontSpec")
        .unwrap_or_else(|| unreachable!("Title retains its FontSpec"));
    assert_eq!(attribute(custom_font, "Color"), Some("#87878F8FE0E0"));

    let duplicate_font = child_element(&settings[2], "FontSpec")
        .unwrap_or_else(|| unreachable!("inert duplicate retains its FontSpec"));
    assert_eq!(attribute(duplicate_font, "Color"), Some("#FfFfFfFfFfFf"));

    let ambiguous_fonts = settings[3]
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "FontSpec" => {
                Some(attribute(element, "Color"))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ambiguous_fonts,
        vec![Some("#000000000000"), Some("#FfFfFfFfFfFf")]
    );
    assert_eq!(
        project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source.original_bytes.as_slice()),
        Some(source.as_bytes())
    );
}

#[test]
fn arbitrary_typed_title_paragraphs_are_managed_only_inside_title_content() {
    let fixture = std::str::from_utf8(SMOKE_FDX)
        .unwrap_or_else(|_| unreachable!("sanitized import fixture is UTF-8"));
    let source = fixture.replace(
        "<TitlePage>\n    <Content>\n      <Paragraph Type=\"Title\" id=\"smoke-title\">\n        <Text>Smoke Story</Text>\n      </Paragraph>\n    </Content>\n  </TitlePage>",
        "<TitlePage>\n    <Paragraph Type=\"DirectOnlyTitle\" Alignment=\"Center\" id=\"legacy-title\">\n      <Text>Direct </Text><Text Style=\"Bold\">Title</Text>\n      <VendorTitleData marker=\"retained\"/>\n      <UserParagraphData><VendorPayload/></UserParagraphData>\n    </Paragraph>\n    <Paragraph id=\"typeless-direct\"><Text>Opaque direct</Text></Paragraph>\n    <Wrapper><Paragraph Type=\"OutOfBoundary\" id=\"opaque-lookalike\"><Text>Opaque lookalike</Text></Paragraph></Wrapper>\n    <Header><Paragraph Type=\"HeaderOnlyTitle\" id=\"header-title\"><Text>Header data</Text></Paragraph></Header>\n    <Footer><Paragraph Type=\"FooterOnlyTitle\" id=\"footer-title\"><Text>Footer data</Text></Paragraph></Footer>\n    <Content>\n      <Paragraph Type=\"CustomTitle\" id=\"content-title\"><Text>Content title</Text></Paragraph>\n      <Paragraph Type=\"customtitle\" id=\"case-title\"><Text>Case title</Text></Paragraph>\n      <Paragraph id=\"typeless-content\"><Text>Type-less title</Text></Paragraph>\n    </Content>\n  </TitlePage>",
    );
    assert_ne!(source, fixture);

    let imported = valid(read_v6(source.as_bytes()));
    let project = imported.project;
    assert_eq!(project.title_page.len(), 3);
    assert!(project
        .element_definitions()
        .get_by_label("CustomTitle")
        .is_some());
    assert!(project
        .element_definitions()
        .get_by_label("customtitle")
        .is_some());
    assert!(project
        .element_definitions()
        .get_by_label("DirectOnlyTitle")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("HeaderOnlyTitle")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("FooterOnlyTitle")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("OutOfBoundary")
        .is_none());

    assert!(!project.title_page.iter().any(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            == Some("legacy-title")
    }));

    let content_typed = project
        .title_page
        .iter()
        .find(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("content-title")
        })
        .unwrap_or_else(|| unreachable!("typed TitlePage Content paragraph is managed"));
    assert_eq!(
        content_typed.element_type.as_ref().map(|key| key.as_str()),
        Some("CustomTitle")
    );
    let case_variant = project
        .title_page
        .iter()
        .find(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("case-title")
        })
        .unwrap_or_else(|| unreachable!("case-distinct typed title paragraph is managed"));
    assert_eq!(
        case_variant.element_type.as_ref().map(|key| key.as_str()),
        Some("customtitle")
    );
    let typeless_content = project
        .title_page
        .iter()
        .find(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("typeless-content")
        })
        .unwrap_or_else(|| unreachable!("type-less TitlePage Content paragraph remains valid"));
    assert_eq!(typeless_content.element_type, None);
    assert!(!project.title_page.iter().any(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            .is_some_and(|foreign_id| matches!(foreign_id, "typeless-direct" | "opaque-lookalike"))
    }));

    let root = source_root(&project)
        .unwrap_or_else(|| unreachable!("the fixture retains its FinalDraft root"));
    let title_page = child_element(root, "TitlePage")
        .unwrap_or_else(|| unreachable!("the fixture retains its TitlePage"));
    let direct_source = child_element_with_attribute(title_page, "Paragraph", "id", "legacy-title")
        .unwrap_or_else(|| unreachable!("the direct typed title source remains retained"));
    assert!(matches!(&direct_source.ownership, SourceOwnership::Opaque));
    assert!(direct_source.anchor.is_none());
    assert_eq!(attribute(direct_source, "Type"), Some("DirectOnlyTitle"));
    assert_eq!(
        child_element(direct_source, "Text").map(direct_text),
        Some("Direct ".to_owned())
    );
    assert!(child_element(direct_source, "VendorTitleData").is_some());
    assert!(child_element(direct_source, "UserParagraphData").is_some());
    let title_content = child_element(title_page, "Content")
        .unwrap_or_else(|| unreachable!("the fixture retains TitlePage Content"));
    let content_title_source =
        child_element_with_attribute(title_content, "Paragraph", "id", "content-title")
            .unwrap_or_else(|| unreachable!("the typed Content title source remains retained"));
    assert!(matches!(
        &content_title_source.ownership,
        SourceOwnership::Managed(owner) if owner == &content_typed.id
    ));
    assert!(content_title_source.anchor.is_some());
    let opaque_direct =
        child_element_with_attribute(title_page, "Paragraph", "id", "typeless-direct")
            .unwrap_or_else(|| unreachable!("untyped legacy-depth source remains retained"));
    assert!(matches!(&opaque_direct.ownership, SourceOwnership::Opaque));
    let opaque_wrapper = child_element(title_page, "Wrapper")
        .unwrap_or_else(|| unreachable!("the unsupported title wrapper remains retained"));
    assert!(matches!(&opaque_wrapper.ownership, SourceOwnership::Opaque));
    assert!(
        child_element_with_attribute(opaque_wrapper, "Paragraph", "id", "opaque-lookalike")
            .is_some()
    );
    for (container, paragraph_id) in [("Header", "header-title"), ("Footer", "footer-title")] {
        let region = child_element(title_page, container)
            .unwrap_or_else(|| unreachable!("header/footer-like title structure remains retained"));
        assert!(matches!(&region.ownership, SourceOwnership::Opaque));
        let paragraph = child_element_with_attribute(region, "Paragraph", "id", paragraph_id)
            .unwrap_or_else(|| unreachable!("header/footer-like paragraph remains retained"));
        assert!(matches!(&paragraph.ownership, SourceOwnership::Opaque));
        assert!(paragraph.anchor.is_none());
    }
}

#[test]
fn duplicate_element_settings_are_reported_without_source_values() {
    let source = std::str::from_utf8(SMOKE_FDX)
        .unwrap_or_else(|_| unreachable!("sanitized import fixture is UTF-8"))
        .replace(
            "  <Content>",
            "  <ElementSettings Type=\"Private Probe\"/><ElementSettings Type=\"Private Probe\"/>\n  <Content>",
        );
    let imported = valid(read_v6(source.as_bytes()));

    assert_eq!(
        imported.import_diagnostics.entries,
        vec![
            scriptcraft_fdx::FdxImportDiagnostic::DuplicateElementSettings {
                settings_ordinal: 1
            }
        ]
    );
    assert_eq!(imported.import_diagnostics.suppressed_duplicate_count, 0);
}
