#![forbid(unsafe_code)]

use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::formatting::formatting_in_native_order;
use scriptcraft_core::preservation::{PreservedElement, PreservedNode};
use scriptcraft_core::{FixedPoint, ParagraphAlignment, Role};

const ROOT_FDX_NS: &str = "urn:fixture:fdx";
const VENDOR_NS: &str = "urn:fixture:vendor";

fn preserved_root(project: &scriptcraft_core::Project) -> &PreservedElement {
    project
        .preservation
        .as_ref()
        .expect("source-backed project has preservation")
        .sections[0]
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => {
                Some(element)
            }
            _ => None,
        })
        .expect("retained FDX root exists")
}

fn direct_child<'a>(
    parent: &'a PreservedElement,
    local_name: &str,
    namespace: &str,
) -> Option<&'a PreservedElement> {
    parent.children.iter().find_map(|node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == local_name
                && element.name.namespace_uri.as_deref() == Some(namespace) =>
        {
            Some(element)
        }
        _ => None,
    })
}

fn direct_children<'a>(
    parent: &'a PreservedElement,
    local_name: &str,
    namespace: &str,
) -> Vec<&'a PreservedElement> {
    parent
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == local_name
                    && element.name.namespace_uri.as_deref() == Some(namespace) =>
            {
                Some(element)
            }
            _ => None,
        })
        .collect()
}

fn source_attribute<'a>(
    element: &'a PreservedElement,
    local_name: &str,
    namespace: Option<&str>,
) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.local_name == local_name
                && attribute.name.namespace_uri.as_deref() == namespace
        })
        .map(|attribute| attribute.value.as_str())
}

fn find_descendant<'a>(
    element: &'a PreservedElement,
    local_name: &str,
    namespace: &str,
) -> Option<&'a PreservedElement> {
    for child in element.children.iter() {
        let PreservedNode::Element(child) = child else {
            continue;
        };
        if child.name.local_name == local_name
            && child.name.namespace_uri.as_deref() == Some(namespace)
        {
            return Some(child);
        }
        if let Some(found) = find_descendant(child, local_name, namespace) {
            return Some(found);
        }
    }
    None
}

#[test]
fn imported_role_colors_do_not_override_effective_style_or_role_defaults() {
    let mut source = String::from(
        r#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action"><Text>body text</Text></Paragraph></Content>"#,
    );
    for formatting in formatting_in_native_order() {
        let mut style_tokens = vec!["Bold", "Italic", "Underline", "AllCaps"];
        if formatting.font.style_tokens.contains(&"HiddenText") {
            style_tokens.push("HiddenText");
        }
        let style = style_tokens.join("+");
        source.push_str(&format!(
            r##"<ElementSettings Type="{}"><FontSpec AdornmentStyle="0" Background="#FFFFFFFFFFFF" Color="#000000000000" Font="Imported Role Face" RevisionID="0" Size="17" Style="{}"/><ParagraphSpec Alignment="Right" FirstIndent="0.25" Leading="Regular" LeftIndent="2.00" RightIndent="6.50" SpaceBefore="3" Spacing="1" StartsNewPage="Yes"/></ElementSettings>"##,
            formatting.fdx_label, style
        ));
    }
    source.push_str("</FinalDraft>");

    let imported = read_v6(source.as_bytes()).expect("all recognized role settings import");
    let preservation = imported
        .project
        .preservation
        .as_ref()
        .expect("source settings are retained");
    let settings = preservation.source_layout_settings();

    assert_eq!(imported.project.elements[0].role, Role::Action);
    assert_eq!(imported.project.elements[0].text, "body text");
    for formatting in formatting_in_native_order() {
        let role = formatting.role;
        let source_font = settings.font_spec(role);
        assert_eq!(source_font.color.as_deref(), Some("#000000000000"));
        assert_eq!(source_font.family.as_deref(), Some("Imported Role Face"));
        assert_eq!(source_font.size, Some(17));

        let effective = settings.text_style(role);
        assert_eq!(effective.declared_family, "Imported Role Face");
        assert_eq!(effective.size, 17);
        assert!(effective.bold);
        assert!(effective.italic);
        assert!(effective.underline);
        assert!(effective.all_caps);
        assert_eq!(
            effective.hidden_text,
            formatting.font.style_tokens.contains(&"HiddenText")
        );
        assert_eq!(
            effective.source_color,
            formatting.font.color.map(str::to_owned),
            "role {} must keep its app-owned default color",
            formatting.fdx_label
        );
        assert_eq!(
            settings.paragraph_spec(role).alignment,
            Some(ParagraphAlignment::Right)
        );
        let paragraph = settings.paragraph_spec(role);
        assert_eq!(paragraph.first_indent, Some(FixedPoint::from_raw(1_800)));
        assert_eq!(paragraph.left_indent, Some(FixedPoint::from_raw(14_400)));
        assert_eq!(paragraph.right_indent, Some(FixedPoint::from_raw(46_800)));
        assert_eq!(paragraph.leading_regular, Some(true));
        assert_eq!(paragraph.space_before, Some(FixedPoint::from_raw(300)));
        assert_eq!(paragraph.spacing_single, Some(true));
        assert!(settings.starts_new_page(role));
    }
}

#[test]
fn source_backed_fdx_normalizes_only_root_role_and_page_colors_stably() {
    const SOURCE: &[u8] = br##"<FinalDraft xmlns="urn:fixture:fdx" xmlns:v="urn:fixture:vendor" DocumentType="Script" Version="6">
  <Content>
    <Paragraph Type="Action" id="body-owner"><Text Color="body-color" Background="body-background">body sentinel</Text><ScriptNote Color="note-color" Background="note-background"/><Revision Color="revision-color" Background="revision-background"/></Paragraph>
    <v:Vendor Color="vendor-color" Background="vendor-background"><v:FontSpec Color="vendor-font-color" Background="vendor-font-background"/></v:Vendor>
  </Content>
  <PageLayout BackgroundColor="page-background" ForegroundColor="page-foreground" v:ForegroundColor="vendor-page-foreground"><v:PageLayout BackgroundColor="vendor-nested-page-background"/></PageLayout>
  <ElementSettings Type="Action"><FontSpec Font="Imported Face" Size="12" Style="" Color="#000000000000" Background="#FFFFFFFFFFFF" Keep="font-sentinel" v:Color="vendor-font-color" v:Background="vendor-font-background"/><v:FontSpec Color="vendor-child-font-color" Background="vendor-child-font-background"/><Group><FontSpec Color="nested-font-color" Background="nested-font-background"/></Group></ElementSettings>
  <ElementSettings Type="Unknown Custom Role"><FontSpec Color="unknown-role-color" Background="unknown-role-background"/></ElementSettings>
  <ElementSettings Type="Summary"><FontSpec Font="Arial" Size="12" Style="HiddenText" Color="#000000000000" Background="#FFFFFFFFFFFF"/></ElementSettings>
  <v:ElementSettings Type="Action"><v:FontSpec Color="vendor-role-color" Background="vendor-role-background"/></v:ElementSettings>
</FinalDraft>"##;

    let imported = read_v6(SOURCE).expect("color normalization fixture imports");
    let preservation = imported.project.preservation.as_ref().unwrap();
    assert_eq!(&preservation.source.original_bytes[..], SOURCE);

    let root = preserved_root(&imported.project);
    let root_settings = direct_children(root, "ElementSettings", ROOT_FDX_NS);
    let action = root_settings
        .iter()
        .copied()
        .find(|settings| source_attribute(settings, "Type", None) == Some("Action"))
        .expect("direct root Action settings remain");
    let action_font = direct_child(action, "FontSpec", ROOT_FDX_NS).unwrap();
    assert_eq!(source_attribute(action_font, "Color", None), None);
    assert_eq!(source_attribute(action_font, "Background", None), None);
    assert_eq!(
        source_attribute(action_font, "Keep", None),
        Some("font-sentinel")
    );
    assert_eq!(
        source_attribute(action_font, "Color", Some(VENDOR_NS)),
        Some("vendor-font-color")
    );
    assert_eq!(
        source_attribute(action_font, "Background", Some(VENDOR_NS)),
        Some("vendor-font-background")
    );
    let vendor_child_font = direct_child(action, "FontSpec", VENDOR_NS).unwrap();
    assert_eq!(
        source_attribute(vendor_child_font, "Color", None),
        Some("vendor-child-font-color")
    );
    assert_eq!(
        source_attribute(vendor_child_font, "Background", None),
        Some("vendor-child-font-background")
    );
    let nested_group = direct_child(action, "Group", ROOT_FDX_NS).unwrap();
    let nested_font = direct_child(nested_group, "FontSpec", ROOT_FDX_NS).unwrap();
    assert_eq!(
        source_attribute(nested_font, "Color", None),
        Some("nested-font-color")
    );
    assert_eq!(
        source_attribute(nested_font, "Background", None),
        Some("nested-font-background")
    );

    let unknown = root_settings
        .iter()
        .copied()
        .find(|settings| source_attribute(settings, "Type", None) == Some("Unknown Custom Role"))
        .expect("unknown role settings remain");
    let unknown_font = direct_child(unknown, "FontSpec", ROOT_FDX_NS).unwrap();
    assert_eq!(source_attribute(unknown_font, "Color", None), None);
    assert_eq!(source_attribute(unknown_font, "Background", None), None);

    let summary = root_settings
        .iter()
        .copied()
        .find(|settings| source_attribute(settings, "Type", None) == Some("Summary"))
        .expect("present hidden role settings remain");
    let summary_font = direct_child(summary, "FontSpec", ROOT_FDX_NS).unwrap();
    assert_eq!(
        source_attribute(summary_font, "Color", None),
        Some("#87878F8FE0E0")
    );
    assert_eq!(source_attribute(summary_font, "Background", None), None);
    assert!(
        root_settings
            .iter()
            .all(|settings| { source_attribute(settings, "Type", None) != Some("Outline 1") }),
        "normalization must not synthesize absent hidden-role settings"
    );

    let page_layout = direct_child(root, "PageLayout", ROOT_FDX_NS).unwrap();
    assert_eq!(source_attribute(page_layout, "BackgroundColor", None), None);
    assert_eq!(source_attribute(page_layout, "ForegroundColor", None), None);
    assert_eq!(
        source_attribute(page_layout, "ForegroundColor", Some(VENDOR_NS)),
        Some("vendor-page-foreground")
    );
    let nested_page = direct_child(page_layout, "PageLayout", VENDOR_NS).unwrap();
    assert_eq!(
        source_attribute(nested_page, "BackgroundColor", None),
        Some("vendor-nested-page-background")
    );
    let vendor_settings = direct_child(root, "ElementSettings", VENDOR_NS).unwrap();
    assert_eq!(
        source_attribute(vendor_settings, "Type", None),
        Some("Action")
    );
    let vendor_font = direct_child(vendor_settings, "FontSpec", VENDOR_NS).unwrap();
    assert_eq!(
        source_attribute(vendor_font, "Color", None),
        Some("vendor-role-color")
    );
    assert_eq!(
        source_attribute(vendor_font, "Background", None),
        Some("vendor-role-background")
    );

    let text = find_descendant(root, "Text", ROOT_FDX_NS).expect("body Text is retained");
    assert_eq!(source_attribute(text, "Color", None), Some("body-color"));
    assert_eq!(
        source_attribute(text, "Background", None),
        Some("body-background")
    );
    for (name, color, background) in [
        ("ScriptNote", "note-color", "note-background"),
        ("Revision", "revision-color", "revision-background"),
        ("Vendor", "vendor-color", "vendor-background"),
    ] {
        let element = find_descendant(
            root,
            name,
            if name == "Vendor" {
                VENDOR_NS
            } else {
                ROOT_FDX_NS
            },
        )
        .unwrap_or_else(|| panic!("{name} sentinel is retained"));
        assert_eq!(source_attribute(element, "Color", None), Some(color));
        assert_eq!(
            source_attribute(element, "Background", None),
            Some(background)
        );
    }

    let first_write = write_v6(&imported.project).expect("normalized source graph serializes");
    let reopened = read_v6(&first_write).expect("normalized source output rereads");
    let second_write = write_v6(&reopened.project).expect("second normalized write succeeds");
    assert_eq!(first_write, second_write, "normalized writes are stable");
    let output_root = preserved_root(&reopened.project);
    let output_settings = direct_children(output_root, "ElementSettings", ROOT_FDX_NS);
    let output_action = output_settings
        .iter()
        .copied()
        .find(|settings| source_attribute(settings, "Type", None) == Some("Action"))
        .unwrap();
    let output_action_font = direct_child(output_action, "FontSpec", ROOT_FDX_NS).unwrap();
    assert_eq!(source_attribute(output_action_font, "Color", None), None);
    assert_eq!(
        source_attribute(output_action_font, "Background", None),
        None
    );
    let output_summary = output_settings
        .iter()
        .copied()
        .find(|settings| source_attribute(settings, "Type", None) == Some("Summary"))
        .unwrap();
    let output_summary_font = direct_child(output_summary, "FontSpec", ROOT_FDX_NS).unwrap();
    assert_eq!(
        source_attribute(output_summary_font, "Color", None),
        Some("#87878F8FE0E0")
    );
    let output_page = direct_child(output_root, "PageLayout", ROOT_FDX_NS).unwrap();
    assert_eq!(source_attribute(output_page, "BackgroundColor", None), None);
    assert_eq!(source_attribute(output_page, "ForegroundColor", None), None);
}

const ORIGINAL_BLACK_REFERENCE: &[u8] = include_bytes!("../../../.reference/black text.fdx");
const FINAL_DRAFT_BLACK_REFERENCE: &[u8] =
    include_bytes!("../../../.reference/black text from final draft.fdx");
const WHITE_REFERENCE: &[u8] = include_bytes!("../../../.reference/white text.fdx");

fn unqualified_root(project: &scriptcraft_core::Project) -> &PreservedElement {
    project
        .preservation
        .as_ref()
        .expect("reference project retains source graph")
        .sections[0]
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "FinalDraft"
                    && element.name.namespace_uri.is_none() =>
            {
                Some(element)
            }
            _ => None,
        })
        .expect("unqualified FinalDraft root exists")
}

fn unqualified_child<'a>(
    parent: &'a PreservedElement,
    local_name: &str,
) -> Option<&'a PreservedElement> {
    parent.children.iter().find_map(|node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == local_name && element.name.namespace_uri.is_none() =>
        {
            Some(element)
        }
        _ => None,
    })
}

fn unqualified_children<'a>(
    parent: &'a PreservedElement,
    local_name: &str,
) -> Vec<&'a PreservedElement> {
    parent
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == local_name
                    && element.name.namespace_uri.is_none() =>
            {
                Some(element)
            }
            _ => None,
        })
        .collect()
}

fn find_unqualified_descendant<'a>(
    element: &'a PreservedElement,
    local_name: &str,
) -> Option<&'a PreservedElement> {
    for child in element.children.iter() {
        let PreservedNode::Element(child) = child else {
            continue;
        };
        if child.name.local_name == local_name && child.name.namespace_uri.is_none() {
            return Some(child);
        }
        if let Some(found) = find_unqualified_descendant(child, local_name) {
            return Some(found);
        }
    }
    None
}

fn collect_unqualified_descendants<'a>(
    element: &'a PreservedElement,
    local_name: &str,
    found: &mut Vec<&'a PreservedElement>,
) {
    for child in element.children.iter() {
        let PreservedNode::Element(child) = child else {
            continue;
        };
        if child.name.local_name == local_name && child.name.namespace_uri.is_none() {
            found.push(child);
        }
        collect_unqualified_descendants(child, local_name, found);
    }
}

fn attributes_except_color_normalization(
    element: &PreservedElement,
) -> Vec<(Option<String>, String, String)> {
    element
        .attributes
        .iter()
        .filter(|attribute| {
            !(attribute.name.namespace_uri.is_none()
                && matches!(
                    attribute.name.local_name.as_str(),
                    "Color" | "Background" | "BackgroundColor" | "ForegroundColor"
                ))
        })
        .map(|attribute| {
            (
                attribute.name.namespace_uri.clone(),
                attribute.name.local_name.clone(),
                attribute.value.clone(),
            )
        })
        .collect()
}

fn assert_reference_role_and_page_colors_are_normalized(project: &scriptcraft_core::Project) {
    let root = unqualified_root(project);
    for settings in unqualified_children(root, "ElementSettings") {
        let Some(font) = unqualified_child(settings, "FontSpec") else {
            continue;
        };
        let role_name =
            source_attribute(settings, "Type", None).expect("ElementSettings has a role label");
        let default_color = formatting_in_native_order()
            .iter()
            .find(|formatting| formatting.fdx_label == role_name)
            .and_then(|formatting| formatting.font.color);
        assert_eq!(
            source_attribute(font, "Color", None),
            default_color,
            "{role_name} should contain only its canonical app default color"
        );
        assert_eq!(
            source_attribute(font, "Background", None),
            None,
            "{role_name}"
        );
    }
    if let Some(page_layout) = unqualified_child(root, "PageLayout") {
        assert_eq!(source_attribute(page_layout, "BackgroundColor", None), None);
        assert_eq!(source_attribute(page_layout, "ForegroundColor", None), None);
    }
}

fn assert_non_color_layout_settings_unchanged(
    before: &scriptcraft_core::formatting::SourceLayoutSettings,
    after: &scriptcraft_core::formatting::SourceLayoutSettings,
) {
    for formatting in formatting_in_native_order() {
        let mut before_font = before.font_spec(formatting.role).clone();
        let mut after_font = after.font_spec(formatting.role).clone();
        before_font.color = None;
        after_font.color = None;
        assert_eq!(
            &before_font, &after_font,
            "non-color FontSpec projection for {} remains stable",
            formatting.fdx_label
        );
        assert_eq!(
            before.paragraph_spec(formatting.role),
            after.paragraph_spec(formatting.role),
            "ParagraphSpec for {} remains stable",
            formatting.fdx_label
        );
        assert_eq!(
            before.starts_new_page(formatting.role),
            after.starts_new_page(formatting.role),
            "pagination setting for {} remains stable",
            formatting.fdx_label
        );
    }
}

#[test]
fn supplied_color_references_cover_import_save_reopen_and_preservation_boundaries() {
    for (fixture_name, source) in [
        ("original black", ORIGINAL_BLACK_REFERENCE),
        ("Final Draft-saved black", FINAL_DRAFT_BLACK_REFERENCE),
        ("white control", WHITE_REFERENCE),
    ] {
        let imported = read_v6(source)
            .unwrap_or_else(|error| panic!("{fixture_name} reference imports: {error}"));
        let before_preservation = imported.project.preservation.as_ref().unwrap();
        assert_eq!(
            &before_preservation.source.original_bytes[..],
            source,
            "{fixture_name} input bytes remain available as provenance"
        );
        let before_settings = before_preservation.source_layout_settings().clone();
        let before_elements = imported.project.elements.clone();
        let before_title_page = imported.project.title_page.clone();
        let before_ids = imported
            .project
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<Vec<_>>();
        let before_identities = before_preservation.identities.clone();
        let before_root = unqualified_root(&imported.project);

        assert_reference_role_and_page_colors_are_normalized(&imported.project);
        let page_before = unqualified_child(before_root, "PageLayout");
        let page_size_attrs_before = page_before
            .and_then(|page| unqualified_child(page, "PageSize"))
            .map(attributes_except_color_normalization);
        let page_attrs_before = page_before.map(attributes_except_color_normalization);
        let mut scene_properties_before = Vec::new();
        collect_unqualified_descendants(
            before_root,
            "SceneProperties",
            &mut scene_properties_before,
        );
        let scene_property_attrs_before = scene_properties_before
            .into_iter()
            .map(attributes_except_color_normalization)
            .collect::<Vec<_>>();

        let output = write_v6(&imported.project)
            .unwrap_or_else(|error| panic!("{fixture_name} saves: {error}"));
        let reopened = read_v6(&output)
            .unwrap_or_else(|error| panic!("{fixture_name} saved output reopens: {error}"));
        assert_reference_role_and_page_colors_are_normalized(&reopened.project);
        assert_eq!(
            output,
            write_v6(&reopened.project).expect("second normalized save succeeds"),
            "{fixture_name} output reaches stable bytes after reopen"
        );

        assert_eq!(
            reopened.project.elements, before_elements,
            "{fixture_name} body text, roles, IDs, and runs remain unchanged"
        );
        assert_eq!(
            reopened.project.title_page, before_title_page,
            "{fixture_name} title-page content remains unchanged"
        );
        assert_eq!(
            reopened
                .project
                .elements
                .iter()
                .map(|element| element.id.clone())
                .collect::<Vec<_>>(),
            before_ids,
            "{fixture_name} paragraph identities remain unchanged"
        );
        let after_preservation = reopened.project.preservation.as_ref().unwrap();
        assert_eq!(
            after_preservation.identities, before_identities,
            "{fixture_name} opaque identities remain unchanged"
        );
        assert_non_color_layout_settings_unchanged(
            &before_settings,
            after_preservation.source_layout_settings(),
        );

        let after_root = unqualified_root(&reopened.project);
        assert_eq!(
            unqualified_child(after_root, "PageLayout").map(attributes_except_color_normalization),
            page_attrs_before,
            "{fixture_name} page-layout inputs remain unchanged"
        );
        assert_eq!(
            unqualified_child(after_root, "PageLayout")
                .and_then(|page| unqualified_child(page, "PageSize"))
                .map(attributes_except_color_normalization),
            page_size_attrs_before,
            "{fixture_name} page dimensions remain unchanged"
        );
        let mut scene_properties_after = Vec::new();
        collect_unqualified_descendants(after_root, "SceneProperties", &mut scene_properties_after);
        assert_eq!(
            scene_properties_after
                .into_iter()
                .map(attributes_except_color_normalization)
                .collect::<Vec<_>>(),
            scene_property_attrs_before,
            "{fixture_name} scene pagination metadata remains unchanged"
        );
    }

    // These are deliberately color-bearing constructs outside the normalization
    // boundary. Both survive the source-backed rewrite of the original sample.
    let imported = read_v6(ORIGINAL_BLACK_REFERENCE).expect("original black control imports");
    let output = write_v6(&imported.project).expect("original black control saves");
    let reopened = read_v6(&output).expect("original black output reopens");
    for project in [&imported.project, &reopened.project] {
        let root = unqualified_root(project);
        let note =
            find_unqualified_descendant(root, "ScriptNote").expect("script-note sentinel remains");
        assert_eq!(source_attribute(note, "Color", None), Some("#000000000000"));
        let note_text = find_unqualified_descendant(note, "Text").expect("annotation text remains");
        assert_eq!(
            source_attribute(note_text, "Color", None),
            Some("#000000000000")
        );
        assert_eq!(
            source_attribute(note_text, "Background", None),
            Some("#FFFFFFFFFFFF")
        );
        let revision =
            find_unqualified_descendant(root, "Revision").expect("revision sentinel remains");
        assert_eq!(
            source_attribute(revision, "Color", None),
            Some("#00000000FFFF")
        );
        assert_eq!(
            source_attribute(revision, "PageColor", None),
            Some("#00000000FFFF")
        );
    }
}
