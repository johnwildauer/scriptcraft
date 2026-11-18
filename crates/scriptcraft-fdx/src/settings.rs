//! FDX role and page settings mapped onto shared model formatting values.

use crate::reader::{attribute, RawElement, RawNode};
use crate::tree::{preserved_element, preserved_element_owned};
use scriptcraft_model::element::Role;
use scriptcraft_model::formatting::{
    formatting_for_role, formatting_in_native_order, role_index, FixedPoint, ParagraphAlignment,
    PrintVisibility, SourceFontSpec, SourceLayoutSettings, SourceParagraphSpec,
};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode, SourceName};
use crate::diagnostics::FdxError;
use crate::reader::RawAttribute;

/// Read the source-owned page and role settings from a bounded root.
pub(crate) fn read_source_layout_settings(
    root: &RawElement,
    root_ns: Option<&str>,
) -> Result<SourceLayoutSettings, FdxError> {
    let mut settings = SourceLayoutSettings::default();
    let mut seen_roles = std::collections::BTreeSet::new();
    for child in &root.children {
        let RawNode::Element(setting) = child else {
            continue;
        };
        if !is_fdx_element(setting, "ElementSettings", root_ns) {
            continue;
        }
        let Some(source_role) = attribute(setting, "Type") else {
            continue;
        };
        let Some(role) = role_from_fdx(source_role) else {
            continue;
        };
        if !seen_roles.insert(role) {
            // Repeated effective roles are ambiguous, but stay fully
            // source-owned and leave the role defaults effective.
            continue;
        }
        let font_specs = setting
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(child)
                    if child.name.namespace_uri.as_deref() == root_ns
                        && child.name.local_name == "FontSpec" =>
                {
                    Some(child)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        // Duplicate FontSpec children are ambiguous as a whole. Leave every
        // projected font field at its role default without modifying source.
        if let [font] = font_specs.as_slice() {
            let style =
                attribute(font, "Style").map(|style| style.split('+').collect::<std::collections::BTreeSet<_>>());
            let projection = SourceFontSpec {
                family: attribute(font, "Font")
                    .filter(|value| !value.is_empty() && value.len() <= 128)
                    .map(str::to_owned),
                size: attribute(font, "Size").and_then(parse_bounded_font_size),
                color: attribute(font, "Color")
                    .filter(|value| value.len() <= 128)
                    .map(str::to_owned),
                bold: style.as_ref().map(|tokens| tokens.contains("Bold")),
                italic: style.as_ref().map(|tokens| tokens.contains("Italic")),
                underline: style.as_ref().map(|tokens| tokens.contains("Underline")),
                all_caps: style.as_ref().map(|tokens| tokens.contains("AllCaps")),
                hidden_text: style.as_ref().map(|tokens| tokens.contains("HiddenText")),
            };
            if let Some(hidden_text) = projection.hidden_text {
                settings.role_print_visibility[role_index(role)] = Some(if hidden_text {
                    PrintVisibility::Hidden
                } else {
                    PrintVisibility::Visible
                });
            }
            settings.role_font_specs[role_index(role)] = projection;
        }
        let paragraph_specs = setting
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(child)
                    if child.name.namespace_uri.as_deref() == root_ns
                        && child.name.local_name == "ParagraphSpec" =>
                {
                    Some(child)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if paragraph_specs.len() > 1 {
            // Unsupported settings multiplicity stays retained; do not guess
            // which paragraph specification should become effective.
            continue;
        }
        let Some(paragraph) = paragraph_specs.first() else {
            continue;
        };
        // A conflicting child Type makes the entire specification ambiguous;
        // the preserved XML remains untouched and shared role defaults apply.
        if attribute(paragraph, "Type").is_some_and(|paragraph_role| paragraph_role != source_role)
        {
            continue;
        }

        let index = role_index(role);
        let projection = SourceParagraphSpec {
            alignment: attribute(paragraph, "Alignment").map(|alignment| match alignment {
                "Right" => ParagraphAlignment::Right,
                "Center" => ParagraphAlignment::Center,
                "Left" => ParagraphAlignment::Left,
                _ => ParagraphAlignment::Left,
            }),
            first_indent: attribute(paragraph, "FirstIndent")
                .and_then(|value| bounded_inches(value, true)),
            left_indent: attribute(paragraph, "LeftIndent")
                .and_then(|value| bounded_inches(value, false)),
            right_indent: attribute(paragraph, "RightIndent")
                .and_then(|value| bounded_inches(value, false)),
            leading_regular: (attribute(paragraph, "Leading") == Some("Regular")).then_some(true),
            space_before: attribute(paragraph, "SpaceBefore")
                .and_then(FixedPoint::parse_points)
                .filter(|value| (0..=79_200).contains(&value.raw())),
            spacing_single: (attribute(paragraph, "Spacing") == Some("1")).then_some(true),
        };
        settings.role_paragraph_specs[index] = projection;
        if let Some(starts_new_page) = attribute(paragraph, "StartsNewPage") {
            settings.role_starts_new_page[index] = match starts_new_page {
                "Yes" => Some(true),
                "No" => Some(false),
                _ => None,
            };
        }
    }
    Ok(settings)
}

/// Normalize role and page-color settings in the retained source graph without
/// changing the imported byte snapshot.
pub(crate) fn normalize_root_role_and_page_colors(root: &mut RawElement, root_ns: Option<&str>) {
    for child in &mut root.children {
        let RawNode::Element(element) = child else {
            continue;
        };
        if is_fdx_element(element, "PageLayout", root_ns) {
            element.attributes.retain(|attribute| {
                attribute.name.namespace_uri.is_some()
                    || !matches!(
                        attribute.name.local_name.as_str(),
                        "BackgroundColor" | "ForegroundColor"
                    )
            });
            continue;
        }
        if !is_fdx_element(element, "ElementSettings", root_ns) {
            continue;
        }

        let role = attribute(element, "Type").and_then(role_from_fdx);
        let default_color = role.and_then(|role| formatting_for_role(role).font.color);
        for child in &mut element.children {
            let RawNode::Element(font_spec) = child else {
                continue;
            };
            if !is_fdx_element(font_spec, "FontSpec", root_ns) {
                continue;
            }
            font_spec.attributes.retain(|attribute| {
                attribute.name.namespace_uri.is_some()
                    || !matches!(attribute.name.local_name.as_str(), "Background" | "Color")
            });
            if let Some(color) = default_color {
                font_spec.attributes.insert(
                    0,
                    RawAttribute {
                        name: SourceName {
                            namespace_uri: None,
                            local_name: "Color".to_owned(),
                            qualified_name: Some("Color".to_owned()),
                        },
                        value: color.to_owned(),
                    },
                );
            }
        }
    }
}

/// Resolve an FDX role label with the model's canonical role map.
pub(crate) fn role_from_fdx(value: &str) -> Option<Role> {
    formatting_in_native_order()
        .iter()
        .find(|formatting| formatting.fdx_label == value)
        .map(|formatting| formatting.role)
}

/// Construct a canonical new-document ElementSettings subtree from shared
/// model formatting defaults.
pub(crate) fn new_element_settings(role: Role) -> PreservedElement {
    let formatting = formatting_for_role(role);
    let mut settings = preserved_element("ElementSettings", vec![("Type", formatting.fdx_label)]);
    let font = formatting.font;
    let mut font_attributes = vec![(
        "AdornmentStyle".to_owned(),
        font.adornment_style.to_string(),
    )];
    if let Some(color) = font.color {
        font_attributes.push(("Color".to_owned(), color.to_owned()));
    }
    font_attributes.extend([
        ("Font".to_owned(), font.family.to_owned()),
        ("RevisionID".to_owned(), font.revision_id.to_string()),
        ("Size".to_owned(), font.size.to_string()),
        ("Style".to_owned(), font.style_tokens.join("+")),
    ]);
    settings.children.push(PreservedNode::Element(preserved_element_owned(
        "FontSpec",
        font_attributes,
    )));

    let paragraph = formatting.paragraph;
    settings.children.push(PreservedNode::Element(preserved_element_owned(
        "ParagraphSpec",
        vec![
            ("Alignment".to_owned(), paragraph.alignment.to_owned()),
            ("FirstIndent".to_owned(), paragraph.first_indent.to_owned()),
            ("Leading".to_owned(), paragraph.leading.to_owned()),
            ("LeftIndent".to_owned(), paragraph.left_indent.to_owned()),
            ("RightIndent".to_owned(), paragraph.right_indent.to_owned()),
            ("SpaceBefore".to_owned(), paragraph.space_before.to_string()),
            ("Spacing".to_owned(), paragraph.spacing.to_string()),
            (
                "StartsNewPage".to_owned(),
                if paragraph.starts_new_page { "Yes" } else { "No" }.to_owned(),
            ),
            ("Type".to_owned(), formatting.fdx_label.to_owned()),
        ],
    )));

    let behavior = formatting.behavior;
    settings.children.push(PreservedNode::Element(preserved_element_owned(
        "Behavior",
        vec![
            ("PaginateAs".to_owned(), behavior.paginate_as.to_owned()),
            ("ReturnKey".to_owned(), behavior.return_key.to_owned()),
            ("Shortcut".to_owned(), behavior.shortcut.to_owned()),
            ("TabKey".to_owned(), behavior.tab_key.to_owned()),
            ("WinShortcut".to_owned(), behavior.win_shortcut.to_owned()),
        ],
    )));

    settings.children.push(PreservedNode::Element(preserved_element_owned(
        "Outline",
        vec![
            (
                "CanHide".to_owned(),
                if formatting.outline.can_hide { "Yes" } else { "No" }.to_owned(),
            ),
            ("Level".to_owned(), formatting.outline.level.to_string()),
        ],
    )));
    settings
}

fn parse_bounded_font_size(value: &str) -> Option<u8> {
    if value.is_empty() || value.len() > 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let size = value.parse::<u8>().ok()?;
    (1..=72).contains(&size).then_some(size)
}

pub(crate) fn bounded_inches(value: &str, signed: bool) -> Option<FixedPoint> {
    let fixed = FixedPoint::parse_inches(value)?;
    // The current runtime page is bounded to US Letter. Values outside that
    // model budget remain source-owned but are not effective projections.
    if fixed.raw().unsigned_abs() > 61_200 || (!signed && fixed.raw() < 0) {
        return None;
    }
    Some(fixed)
}

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}
