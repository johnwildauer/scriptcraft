//! FDX page-layout and settings values mapped onto model values.

use crate::reader::{attribute, RawElement, RawNode};
use crate::tree::new_lineage_element;
use crate::FdxError;
use crate::constants::FDX_ROOT;
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::layout_inputs::{DocumentLeading, DocumentPageLayout};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode, PreservedSection, SourceName};

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

/// Project only an unambiguous direct-root PageLayout and its direct PageSize.
/// Unusable source values remain untouched in the retained tree and resolve
/// field-locally to the established standard geometry.
pub(crate) fn raw_root_page_layout(root: &RawElement) -> DocumentPageLayout {
    let root_namespace = root.name.namespace_uri.as_deref();
    let page_layouts = root
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element) if is_fdx_element(element, "PageLayout", root_namespace) => {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let [page_layout] = page_layouts.as_slice() else {
        let mut default = DocumentPageLayout::default();
        if !page_layouts.is_empty() {
            default.document_leading = DocumentLeading::Unsupported;
        }
        return default;
    };

    let mut projection = DocumentPageLayout::default();
    projection.document_leading = match attribute(page_layout, "DocumentLeading") {
        None => DocumentLeading::Normal,
        Some("Normal") => DocumentLeading::Normal,
        Some("Loose") => DocumentLeading::Loose,
        Some("Tight") => DocumentLeading::Tight,
        Some("Very Tight") => DocumentLeading::VeryTight,
        Some(_) => DocumentLeading::Unsupported,
    };
    projection.header_margin = parsed_optional_margin(attribute(page_layout, "HeaderMargin"));
    projection.footer_margin = parsed_optional_margin(attribute(page_layout, "FooterMargin"));

    let page_sizes = page_layout
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element) if is_fdx_element(element, "PageSize", root_namespace) => {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if let [page_size] = page_sizes.as_slice() {
        projection.paper_width = rounded_page_value(attribute(page_size, "Width"), true, false)
            .unwrap_or(612);
        projection.paper_height =
            rounded_page_value(attribute(page_size, "Height"), true, false).unwrap_or(792);
    }
    projection.margin_top =
        rounded_page_value(attribute(page_layout, "TopMargin"), false, true).unwrap_or(72);
    projection.margin_bottom = rounded_page_value(
        attribute(page_layout, "BottomMargin"),
        false,
        true,
    )
    .unwrap_or(72);

    let standard_horizontal_margins = 108u64 + 72;
    if standard_horizontal_margins >= u64::from(projection.paper_width)
        || u64::from(projection.margin_top) + u64::from(projection.margin_bottom)
            >= u64::from(projection.paper_height)
    {
        projection.paper_width = 612;
        projection.paper_height = 792;
        projection.margin_top = 72;
        projection.margin_bottom = 72;
    }
    projection
}

fn rounded_page_value(value: Option<&str>, inches: bool, allow_zero: bool) -> Option<u32> {
    let fixed = if inches {
        FixedPoint::parse_inches(value?)?
    } else {
        FixedPoint::parse_points(value?)?
    };
    if fixed.raw() < 0 {
        return None;
    }
    let points = fixed.rounded_points();
    if points < 0 || (!allow_zero && points == 0) {
        return None;
    }
    u32::try_from(points).ok()
}

fn parsed_optional_margin(value: Option<&str>) -> Option<FixedPoint> {
    let margin = FixedPoint::parse_points(value?)?;
    (margin.raw() >= 0).then_some(margin)
}

/// Shared deterministic PageLayout generator for new and source-backed output.
pub(crate) fn page_layout_element(
    parent: &SourceName,
    page_layout: &DocumentPageLayout,
) -> Result<PreservedElement, FdxError> {
    let document_leading = match page_layout.document_leading {
        DocumentLeading::Normal => "Normal",
        DocumentLeading::Loose => "Loose",
        DocumentLeading::Tight => "Tight",
        DocumentLeading::VeryTight => "Very Tight",
        DocumentLeading::Unsupported => return Err(FdxError::InvalidValue),
    };
    let mut attributes = vec![
        ("BottomMargin".to_owned(), page_layout.margin_bottom.to_string()),
        ("DocumentLeading".to_owned(), document_leading.to_owned()),
    ];
    if let Some(margin) = page_layout.footer_margin {
        if margin.raw() < 0 {
            return Err(FdxError::InvalidValue);
        }
        attributes.push(("FooterMargin".to_owned(), margin.decimal_string()));
    }
    if let Some(margin) = page_layout.header_margin {
        if margin.raw() < 0 {
            return Err(FdxError::InvalidValue);
        }
        attributes.push(("HeaderMargin".to_owned(), margin.decimal_string()));
    }
    attributes.push(("TopMargin".to_owned(), page_layout.margin_top.to_string()));

    let mut page_layout_node = new_lineage_element(parent, "PageLayout", attributes);
    let mut page_size = new_lineage_element(
        &page_layout_node.name,
        "PageSize",
        vec![
            ("Height".to_owned(), points_as_inches(page_layout.paper_height)?),
            ("Width".to_owned(), points_as_inches(page_layout.paper_width)?),
        ],
    );
    page_size.children = Default::default();
    page_layout_node
        .children
        .push(PreservedNode::Element(page_size));
    Ok(page_layout_node)
}

fn points_as_inches(points: u32) -> Result<String, FdxError> {
    let scaled = u64::from(points)
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_add(36))
        .ok_or(FdxError::InvalidValue)?
        / 72;
    let whole = scaled / 1_000_000;
    let mut fraction = format!("{:06}", scaled % 1_000_000);
    while fraction.len() > 2 && fraction.ends_with('0') {
        fraction.pop();
    }
    Ok(format!("{whole}.{fraction}"))
}

/// Adds a PageLayout only when the detached source-backed root has none.
pub(crate) fn reconcile_root_page_layout(
    sections: &mut [PreservedSection],
    page_layout: &DocumentPageLayout,
) -> Result<(), FdxError> {
    let mut roots = sections
        .iter_mut()
        .flat_map(|section| &mut section.children)
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => {
                Some(element)
            }
            _ => None,
        });
    let root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() {
        return Err(FdxError::InvalidRoot);
    }
    let root_namespace = root.name.namespace_uri.clone();
    if root.children.iter().any(|node| {
        matches!(node, PreservedNode::Element(element)
            if element.name.local_name == "PageLayout"
                && element.name.namespace_uri.as_deref() == root_namespace.as_deref())
    }) {
        return Ok(());
    }

    let anchor = root
        .children
        .iter()
        .position(|node| direct_root_element(node, "WindowState", &root_namespace))
        .or_else(|| {
            root.children
                .iter()
                .position(|node| direct_root_element(node, "ElementSettings", &root_namespace))
        })
        .or_else(|| {
            root.children
                .iter()
                .position(|node| direct_root_element(node, "TitlePage", &root_namespace))
        })
        .unwrap_or(root.children.len());
    let mut insertion = anchor;
    while insertion > 0
        && matches!(root.children.get(insertion - 1), Some(PreservedNode::Text(text)) if text.trim().is_empty())
    {
        insertion -= 1;
    }

    let generated = page_layout_element(&root.name, page_layout)?;
    if insertion == 0
        || matches!(insertion.checked_sub(1).and_then(|index| root.children.get(index)), Some(PreservedNode::Element(_)))
    {
        root.children
            .insert(insertion, PreservedNode::Text("\n\n  ".into()));
        insertion += 1;
    }
    root.children
        .insert(insertion, PreservedNode::Element(generated));
    let after = insertion + 1;
    if after == root.children.len()
        || matches!(root.children.get(after), Some(PreservedNode::Element(_)))
    {
        root.children
            .insert(after, PreservedNode::Text("\n\n  ".into()));
    }
    Ok(())
}

fn direct_root_element(node: &PreservedNode, name: &str, root_namespace: &Option<String>) -> bool {
    matches!(node, PreservedNode::Element(element)
        if element.name.local_name == name
            && element.name.namespace_uri.as_deref() == root_namespace.as_deref())
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
