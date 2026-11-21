//! FDX page-layout and settings values mapped onto model values.

use crate::reader::{RawElement, RawNode};
use scriptcraft_model::formatting::FixedPoint;

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
