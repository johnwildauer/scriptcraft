//! FDX title-page semantic paragraph and direct-formatting mapping.
//! The FDX package owns this mapping between source title-page data and model values.

use crate::diagnostics::{
    map_model_error, map_paragraph_id_error, FdxError, SaveDiagnosticCode,
};
use crate::identity::raw_identity_attribute as identity_attribute;
use crate::reader::{attribute, validate_xml_chars, RawElement};
use crate::settings::role_from_fdx;
use crate::tree::{preserved_element, preserved_element_owned};
use crate::writer::validate_qname;
use crate::content::role_to_fdx;
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::preservation::{
    PreservedAttribute, PreservedElement, PreservedNode, SourceName,
};
use scriptcraft_model::{
    SourceFormat, SourceProvenance, SourceSection, TextRun, TitlePageAlignment,
    TitlePageParagraph, TitlePageParagraphFormatting,
};

pub(crate) fn new_title_page_header_and_footer(
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedElement, FdxError> {
    let mut header_and_footer = preserved_element(
        "HeaderAndFooter",
        vec![
            ("FooterFirstPage", "Yes"),
            ("FooterVisible", "No"),
            ("HeaderFirstPage", "No"),
            ("HeaderVisible", "Yes"),
            ("StartingPage", "1"),
        ],
    );

    let mut header = preserved_element("Header", Vec::new());
    let header_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    let mut header_paragraph = preserved_element(
        "Paragraph",
        vec![
            ("Alignment", "Right"),
            ("FirstIndent", "0.00"),
            ("Leading", "Regular"),
            ("LeftIndent", "1.25"),
            ("OutlineLevel", "1"),
            ("RightIndent", "-1.25"),
            ("SpaceBefore", "0"),
            ("Spacing", "1"),
            ("StartsNewPage", "No"),
            ("id", &header_id),
        ],
    );
    header_paragraph
        .children
        .push(PreservedNode::Element(preserved_element(
            "DynamicLabel",
            vec![
                ("AdornmentStyle", "0"),
                ("Font", "Courier Final Draft"),
                ("RevisionID", "0"),
                ("Size", "12"),
                ("Style", ""),
                ("Type", "Page #"),
            ],
        )));
    let mut header_text = preserved_element(
        "Text",
        vec![
            ("AdornmentStyle", "0"),
            ("Font", "Courier Final Draft"),
            ("RevisionID", "0"),
            ("Size", "12"),
            ("Style", ""),
        ],
    );
    header_text.children.push(PreservedNode::Text(".".into()));
    header_paragraph
        .children
        .push(PreservedNode::Element(header_text));
    header
        .children
        .push(PreservedNode::Element(header_paragraph));

    let mut footer = preserved_element("Footer", Vec::new());
    let footer_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    footer.children.push(PreservedNode::Element(preserved_element(
        "Paragraph",
        vec![
            ("Alignment", "Right"),
            ("FirstIndent", "0.00"),
            ("Leading", "Regular"),
            ("LeftIndent", "1.25"),
            ("OutlineLevel", "1"),
            ("RightIndent", "-1.25"),
            ("SpaceBefore", "0"),
            ("Spacing", "1"),
            ("StartsNewPage", "No"),
            ("id", &footer_id),
        ],
    )));
    header_and_footer
        .children
        .push(PreservedNode::Element(header));
    header_and_footer
        .children
        .push(PreservedNode::Element(footer));
    Ok(header_and_footer)
}

pub(crate) fn new_title_page_paragraph(
    semantic: &TitlePageParagraph,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedNode, FdxError> {
    let formatting = semantic.formatting;
    if formatting.leading_regular == Some(false) || formatting.spacing_single == Some(false) {
        return Err(FdxError::UnsupportedConstruct);
    }
    let mut attributes = Vec::with_capacity(12);
    if let Some(alignment) = formatting.alignment {
        let value = match alignment {
            TitlePageAlignment::Left => "Left",
            TitlePageAlignment::Right => "Right",
            TitlePageAlignment::Center => "Center",
            TitlePageAlignment::Justified => "Full",
        };
        attributes.push(("Alignment", value.to_owned()));
    }
    if let Some(value) = formatting.first_indent {
        attributes.push(("FirstIndent", format_inches_attribute(value)));
    }
    if formatting.leading_regular == Some(true) {
        attributes.push(("Leading", "Regular".to_owned()));
    }
    if let Some(value) = formatting.left_indent {
        attributes.push(("LeftIndent", format_inches_attribute(value)));
    }
    attributes.push(("OutlineLevel", "1".to_owned()));
    if let Some(value) = formatting.right_indent {
        attributes.push(("RightIndent", format_inches_attribute(value)));
    }
    if let Some(value) = formatting.space_before {
        attributes.push(("SpaceBefore", value.decimal_string()));
    }
    if formatting.spacing_single == Some(true) {
        attributes.push(("Spacing", "1".to_owned()));
    }
    if let Some(value) = formatting.starts_new_page {
        attributes.push((
            "StartsNewPage",
            if value { "Yes" } else { "No" }.to_owned(),
        ));
    }
    if let Some(role) = semantic.role {
        attributes.push(("Type", role_to_fdx(role).to_owned()));
    }
    validate_xml_chars(&semantic.text)?;
    let paragraph_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    attributes.push(("id", paragraph_id));
    let mut paragraph = preserved_element_owned(
        "Paragraph",
        attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    );
    let runs = if semantic.runs.is_empty() && !semantic.text.is_empty() {
        vec![TextRun {
            text: semantic.text.clone(),
            attributes: Vec::new(),
        }]
    } else {
        semantic.runs.clone()
    };
    if semantic.text.is_empty() {
        if !runs.is_empty() {
            return Err(SaveDiagnosticCode::NewParagraphEmptyRunsMismatch.error());
        }
        return Ok(PreservedNode::Element(paragraph));
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != semantic.text {
        return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
    }
    for run in runs.into_iter().filter(|run| !run.text.is_empty()) {
        validate_xml_chars(&run.text)?;
        let mut text = preserved_element("Text", Vec::new());
        for attribute in run.attributes {
            validate_qname(&attribute.name)?;
            if attribute.name.contains(':') {
                return Err(SaveDiagnosticCode::NewParagraphNamespacedRunAttribute.error());
            }
            validate_xml_chars(&attribute.value)?;
            text.attributes.push(PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: attribute.name.clone(),
                    qualified_name: Some(attribute.name),
                },
                value: attribute.value,
            });
        }
        text.children.push(PreservedNode::Text(run.text.into()));
        paragraph.children.push(PreservedNode::Element(text));
    }
    Ok(PreservedNode::Element(paragraph))
}

fn format_inches_attribute(value: FixedPoint) -> String {
    let hundredths = value.raw().div_euclid(72);
    let negative = hundredths < 0;
    let magnitude = hundredths.unsigned_abs();
    format!(
        "{}{}.{}",
        if negative { "-" } else { "" },
        magnitude / 100,
        magnitude % 100
    )
}

/// Map one supported title-page Paragraph into its semantic model value.
///
/// Project a bounded raw title-page paragraph into the semantic model.
pub(crate) fn make_title_page_paragraph(
    element: &RawElement,
    ordinal: u32,
    next_id: &mut usize,
) -> Result<TitlePageParagraph, FdxError> {
    let id = format!("fdx-element-{}", *next_id);
    *next_id = next_id.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    let source_provenance = SourceProvenance {
        format: SourceFormat::Fdx,
        section: SourceSection::TitlePage,
        ordinal,
        foreign_id: identity_attribute(element).map(str::to_owned),
        foreign_role: attribute(element, "Type").map(str::to_owned),
    };
    let role = attribute(element, "Type").and_then(role_from_fdx);
    TitlePageParagraph::with_runs(
        id,
        role,
        crate::content::semantic_text_runs(element, false, false)?,
        Some(source_provenance),
        direct_title_page_formatting(element),
    )
    .map_err(map_model_error)
}

fn direct_title_page_formatting(element: &RawElement) -> TitlePageParagraphFormatting {
    TitlePageParagraphFormatting {
        alignment: attribute(element, "Alignment").and_then(|alignment| match alignment {
            "Left" => Some(TitlePageAlignment::Left),
            "Center" => Some(TitlePageAlignment::Center),
            "Right" => Some(TitlePageAlignment::Right),
            "Full" => Some(TitlePageAlignment::Justified),
            _ => None,
        }),
        first_indent: attribute(element, "FirstIndent")
            .and_then(|value| crate::settings::bounded_inches(value, true)),
        left_indent: attribute(element, "LeftIndent")
            .and_then(|value| crate::settings::bounded_inches(value, false)),
        right_indent: attribute(element, "RightIndent")
            .and_then(|value| crate::settings::bounded_inches(value, false)),
        leading_regular: (attribute(element, "Leading") == Some("Regular")).then_some(true),
        space_before: attribute(element, "SpaceBefore")
            .and_then(FixedPoint::parse_points)
            .filter(|value| (0..=79_200).contains(&value.raw())),
        spacing_single: (attribute(element, "Spacing") == Some("1")).then_some(true),
        starts_new_page: attribute(element, "StartsNewPage").and_then(|value| match value {
            "Yes" => Some(true),
            "No" => Some(false),
            _ => None,
        }),
    }
}
