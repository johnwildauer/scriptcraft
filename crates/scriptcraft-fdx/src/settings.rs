//! FDX known-role compatibility and page settings mapped onto model values.

use crate::diagnostics::FdxError;
use crate::reader::{RawElement, RawNode};
use scriptcraft_model::element_definition::ElementDefinition;
use scriptcraft_model::element_definitions::DocumentElementDefinitions;
use scriptcraft_model::formatting::{
    role_index, FixedPoint, PrintVisibility, SourceFontSpec, SourceLayoutSettings,
    SourceParagraphSpec,
};
use scriptcraft_model::preservation::PreservedElement;

/// Normalize PageLayout color settings in the retained source graph without
/// changing the imported byte snapshot. Element FontSpec colors are normalized
/// separately from winning open settings drafts.
pub(crate) fn normalize_root_page_layout_colors(root: &mut RawElement, root_ns: Option<&str>) {
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
        }
    }
}

/// Derives the temporary fixed-role layout projection from authoritative
/// completed document definitions. Unknown labels and pagination aliases do
/// not populate role slots.
pub(crate) fn derive_compatibility_layout_settings(
    definitions: &DocumentElementDefinitions,
) -> SourceLayoutSettings {
    let mut settings = SourceLayoutSettings::default();
    for definition in definitions.iter() {
        let Some(role) = definition.key.to_role() else {
            continue;
        };
        let index = role_index(role);
        settings.role_font_specs[index] = SourceFontSpec {
            family: Some(definition.font.family.clone()),
            size: Some(definition.font.size),
            color: definition.font.color.clone(),
            bold: Some(definition.font.bold),
            italic: Some(definition.font.italic),
            underline: Some(definition.font.underline),
            all_caps: Some(definition.font.all_caps),
            hidden_text: Some(definition.font.hidden_text),
        };
        settings.role_print_visibility[index] = Some(if definition.font.hidden_text {
            PrintVisibility::Hidden
        } else {
            PrintVisibility::Visible
        });
        settings.role_paragraph_specs[index] = SourceParagraphSpec {
            alignment: Some(definition.paragraph.alignment),
            first_indent: Some(definition.paragraph.first_indent),
            left_indent: Some(definition.paragraph.left_indent),
            right_indent: Some(definition.paragraph.right_indent),
            leading_regular: Some(definition.paragraph.leading == "Regular"),
            space_before: Some(definition.paragraph.space_before),
            spacing_single: Some(definition.paragraph.spacing == 1),
        };
        settings.role_starts_new_page[index] = Some(definition.paragraph.starts_new_page);
    }
    settings
}

/// Construct a complete new-document settings node from the Project's owned
/// definition using the same bounded serializer as retained-source projection.
pub(crate) fn new_element_settings(
    definition: &ElementDefinition,
    root: &PreservedElement,
) -> Result<PreservedElement, FdxError> {
    crate::element_settings::new_project_element_settings(definition, root)
}

pub(crate) fn parse_bounded_font_size(value: &str) -> Option<u8> {
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
