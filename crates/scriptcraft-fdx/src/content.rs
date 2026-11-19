//! FDX body content and supported dual-dialogue semantic mapping.
//! The FDX package owns this mapping between source content and model values.

use crate::diagnostics::{map_model_error, map_paragraph_id_error, FdxError, SaveDiagnosticCode};
use crate::identity::raw_identity_attribute as identity_attribute;
use crate::reader::{attribute, validate_xml_chars, RawElement, RawNode};
use crate::settings::bounded_inches;
use crate::tree::{preserve_misc_node, preserve_opaque_element, preserved_element};
use crate::writer::validate_qname;
use scriptcraft_model::dual_dialogue::{DualDialogueGroup, DualDialogueWrapperProvenance};
use scriptcraft_model::element_definition::SemanticCapabilities;
use scriptcraft_model::element_type::ElementTypeKey;
use scriptcraft_model::formatting::{
    FixedPoint, ParagraphAlignment, SourceLayoutSettings, SourceParagraphOverrides,
};
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::preservation::{
    is_user_paragraph_data_name, AnchorRelation, AnchorTarget, DependencyConfidence,
    PreservedAttribute, PreservedElement, PreservedNode, SectionKind, SourceAnchor, SourceAnchorId,
    SourceName, SourceOwnership,
};
use scriptcraft_model::{
    BodyOwner, Project, RunAttribute, ScreenplayElement, SourceFormat, SourceProvenance,
    SourceSection, StableId, TextRun, TitlePageParagraph,
};

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}

pub(crate) use crate::reader::{classify_preserved_element, SourceClass};

/// Returns a direct, unqualified, unique, nonempty paragraph Type as its exact
/// open key. Oversized labels fail through the bounded import error mapping.
pub(crate) fn paragraph_type_key(element: &RawElement) -> Result<Option<ElementTypeKey>, FdxError> {
    let Some(label) = attribute(element, "Type").filter(|label| !label.is_empty()) else {
        return Ok(None);
    };
    if label.len() > scriptcraft_model::element_type::MAX_ELEMENT_TYPE_KEY_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    ElementTypeKey::new(label.to_owned())
        .map(Some)
        .map_err(|_| FdxError::InvalidValue)
}

/// Whether a direct body Paragraph is a managed owner under the supported
/// dual-dialogue source topology.
pub(crate) fn is_managed_direct_body_paragraph(
    element: &RawElement,
    root_ns: Option<&str>,
) -> Result<bool, FdxError> {
    if paragraph_type_key(element)?.is_none() {
        return Ok(false);
    }
    let has_dual_name = element.children.iter().any(
        |child| matches!(child, RawNode::Element(child) if child.name.local_name == "DualDialogue"),
    );
    if !has_dual_name {
        return Ok(true);
    }
    Ok(attribute(element, "Type") == Some("General")
        && is_supported_dual_dialogue_source(element, root_ns))
}

pub(crate) fn is_supported_dual_dialogue_source(outer: &RawElement, root_ns: Option<&str>) -> bool {
    if !is_fdx_element(outer, "Paragraph", root_ns) || attribute(outer, "Type") != Some("General") {
        return false;
    }
    let duals = outer
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element) if is_fdx_element(element, "DualDialogue", root_ns) => {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let [dual] = duals.as_slice() else {
        return false;
    };

    let mut outer_attachment = false;
    for child in &outer.children {
        match child {
            RawNode::Element(element) if std::ptr::eq(*dual, element) => {}
            RawNode::Element(element)
                if is_user_paragraph_data_name(&element.name, &outer.name) && !outer_attachment =>
            {
                outer_attachment = true;
            }
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            _ => return false,
        }
    }

    let mut spell_blocks = 0usize;
    let mut members = 0usize;
    for child in &dual.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            RawNode::Element(element)
                if is_fdx_element(element, "SpellCheckIgnoreLists", root_ns) =>
            {
                if spell_blocks != 0
                    || members != 0
                    || validate_nested_spell_check(element, root_ns).is_err()
                {
                    return false;
                }
                spell_blocks += 1;
            }
            RawNode::Element(element) if is_fdx_element(element, "Paragraph", root_ns) => {
                let Ok(Some(element_type)) = paragraph_type_key(element) else {
                    return false;
                };
                if validate_nested_member_topology(element, root_ns, &element_type).is_err() {
                    return false;
                }
                members += 1;
            }
            _ => return false,
        }
    }
    spell_blocks == 1 && members >= 2
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn preserve_dual_dialogue_wrapper(
    outer: &RawElement,
    root_ns: Option<&str>,
    project: &mut Project,
    source_layout_settings: &mut SourceLayoutSettings,
    anchors: &mut Vec<SourceAnchor>,
    next_id: &mut usize,
    next_anchor: &mut usize,
) -> Result<PreservedElement, FdxError> {
    let wrapper_ordinal = next_id.checked_sub(1).ok_or(FdxError::ResourceLimit)?;
    let mut staged_members = Vec::new();
    let mut staged_anchors = Vec::new();
    let mut staged_member_overrides = Vec::new();
    let duals = outer
        .children
        .iter()
        .filter_map(|child| match child {
            RawNode::Element(child) if child.name.local_name == "DualDialogue" => Some(child),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [dual] = duals.as_slice() else {
        return Err(FdxError::UnsupportedConstruct);
    };
    let dual = *dual;
    if !is_fdx_element(outer, "Paragraph", root_ns)
        || !is_fdx_element(dual, "DualDialogue", root_ns)
        || attribute(outer, "Type") != Some("General")
    {
        return Err(FdxError::UnsupportedConstruct);
    }

    // The wrapper's direct mixed content is deliberately stricter than generic
    // preservation: only indentation whitespace, one opaque attachment, and its
    // one DualDialogue child are accepted.
    let mut outer_attachment = false;
    for child in &outer.children {
        match child {
            RawNode::Element(child) if child.name.local_name == "DualDialogue" => {}
            RawNode::Element(child)
                if is_user_paragraph_data_name(&child.name, &outer.name) && !outer_attachment =>
            {
                outer_attachment = true;
            }
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }

    let mut spell_blocks = 0usize;
    let mut seen_member = false;
    let mut member_count = 0usize;
    let mut dual_children = Vec::with_capacity(dual.children.len());
    for child in &dual.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {
                dual_children.push(preserve_misc_node(child.clone())?);
            }
            RawNode::Element(element) if element.name.local_name == "SpellCheckIgnoreLists" => {
                if seen_member || spell_blocks != 0 {
                    return Err(FdxError::UnsupportedConstruct);
                }
                validate_nested_spell_check(element, root_ns)?;
                spell_blocks += 1;
                dual_children.push(preserve_opaque_element(element.clone())?);
            }
            RawNode::Element(element) if element.name.local_name == "Paragraph" => {
                seen_member = true;
                if !is_fdx_element(element, "Paragraph", root_ns) {
                    return Err(FdxError::UnsupportedConstruct);
                }
                let element_type =
                    paragraph_type_key(element)?.ok_or(FdxError::UnsupportedConstruct)?;
                validate_nested_member_topology(element, root_ns, &element_type)?;
                let semantic =
                    make_semantic_element(element, element_type, SectionKind::Content, next_id)?;
                let member_id = semantic.id.clone();
                let anchor_id = next_source_anchor(next_anchor)?;
                let member_overrides = direct_paragraph_overrides(element);
                if let Some(overrides) = member_overrides {
                    staged_member_overrides.push((member_id.clone(), overrides));
                }
                staged_anchors.push(SourceAnchor {
                    id: anchor_id.clone(),
                    target: AnchorTarget::ProjectElement(member_id.clone()),
                    relation: AnchorRelation::Represents,
                });
                member_count = member_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
                staged_members.push(semantic);
                dual_children.push(PreservedNode::Element(preserved_managed_nested_paragraph(
                    element, member_id, anchor_id,
                )?));
            }
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    if spell_blocks != 1 || member_count < 2 {
        return Err(FdxError::UnsupportedConstruct);
    }

    // The semantic members were appended in source order. Their IDs are also
    // the group membership authority; the outer General remains invisible.
    if staged_members.len() != member_count {
        return Err(FdxError::UnsupportedConstruct);
    }
    let members = staged_members.as_slice();
    let group_id = StableId::new(format!(
        "fdx-dual-group-{}",
        project
            .dual_dialogue_groups
            .len()
            .checked_add(1)
            .ok_or(FdxError::ResourceLimit)?
    ))
    .map_err(map_model_error)?;
    let member_ids = members.iter().map(|member| member.id.clone()).collect();

    let wrapper_anchor = next_source_anchor(next_anchor)?;
    let outer_id = Some(
        identity_attribute(outer)
            .ok_or(FdxError::UnsupportedConstruct)?
            .to_owned(),
    );
    let source = SourceProvenance {
        format: SourceFormat::Fdx,
        section: SourceSection::Content,
        ordinal: u32::try_from(wrapper_ordinal).map_err(|_| FdxError::ResourceLimit)?,
        foreign_id: outer_id.clone(),
        foreign_role: attribute(outer, "Type").map(str::to_owned),
    };
    let wrapper_overrides = dual_wrapper_direct_overrides(outer)?;
    if let Some(overrides) = &wrapper_overrides {
        staged_member_overrides.push((group_id.clone(), overrides.clone()));
    }
    staged_anchors.push(SourceAnchor {
        id: wrapper_anchor.clone(),
        target: AnchorTarget::DualDialogueWrapper(group_id.clone()),
        relation: AnchorRelation::Represents,
    });
    let group = DualDialogueGroup {
        id: group_id.clone(),
        members: member_ids,
        wrapper: DualDialogueWrapperProvenance {
            outer_foreign_id: outer_id,
            source: Some(source),
            preservation_anchor: Some(wrapper_anchor.clone()),
        },
    };

    let dual_preserved = PreservedElement {
        name: dual.name.clone(),
        attributes: preserved_attributes(dual),
        namespace_bindings: dual.namespace_bindings.clone(),
        source_type: attribute(dual, "Type").map(str::to_owned),
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: dual_children.into(),
    };
    let wrapper_children = outer
        .children
        .iter()
        .map(|child| match child {
            RawNode::Element(child) if child.name.local_name == "DualDialogue" => {
                Ok(PreservedNode::Element(dual_preserved.clone()))
            }
            RawNode::Element(child) if is_user_paragraph_data_name(&child.name, &outer.name) => {
                Ok(preserve_opaque_element(child.clone())?)
            }
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {
                preserve_misc_node(child.clone())
            }
            _ => Err(FdxError::UnsupportedConstruct),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let preserved_outer = PreservedElement {
        name: outer.name.clone(),
        attributes: preserved_attributes(outer),
        namespace_bindings: outer.namespace_bindings.clone(),
        source_type: attribute(outer, "Type").map(str::to_owned),
        ownership: SourceOwnership::DualDialogueWrapper(group_id.clone()),
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: Some(wrapper_anchor),
        children: wrapper_children.into(),
    };
    project
        .elements
        .extend(staged_members.into_iter().map(BodyOwner::shared));
    project.dual_dialogue_groups.push(group);
    for (id, overrides) in staged_member_overrides {
        source_layout_settings
            .direct_paragraph_overrides
            .insert(id, overrides);
    }
    anchors.extend(staged_anchors);
    Ok(preserved_outer)
}

fn next_source_anchor(next_anchor: &mut usize) -> Result<SourceAnchorId, FdxError> {
    let value = format!("fdx-anchor-{}", *next_anchor);
    *next_anchor = next_anchor.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    SourceAnchorId::new(value).map_err(|_| FdxError::InvalidValue)
}

fn preserved_attributes(element: &RawElement) -> Vec<PreservedAttribute> {
    element
        .attributes
        .iter()
        .map(|attribute| PreservedAttribute {
            name: attribute.name.clone(),
            value: attribute.value.clone(),
        })
        .collect()
}

fn preserved_managed_nested_paragraph(
    element: &RawElement,
    id: StableId,
    anchor: SourceAnchorId,
) -> Result<PreservedElement, FdxError> {
    Ok(PreservedElement {
        name: element.name.clone(),
        attributes: preserved_attributes(element),
        namespace_bindings: element.namespace_bindings.clone(),
        source_type: attribute(element, "Type").map(str::to_owned),
        ownership: SourceOwnership::Managed(id),
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: Some(anchor),
        children: element
            .children
            .iter()
            .cloned()
            .map(preserve_misc_node)
            .collect::<Result<Vec<_>, _>>()?
            .into(),
    })
}

fn validate_nested_spell_check(
    element: &RawElement,
    root_ns: Option<&str>,
) -> Result<(), FdxError> {
    if !is_fdx_element(element, "SpellCheckIgnoreLists", root_ns) {
        return Err(FdxError::UnsupportedConstruct);
    }
    reject_nested_dual_dialogue(element)?;
    let mut ranges = 0usize;
    let mut words = 0usize;
    for child in &element.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            RawNode::Element(child) if is_fdx_element(child, "IgnoredRanges", root_ns) => {
                ranges += 1;
            }
            RawNode::Element(child) if is_fdx_element(child, "IgnoredWords", root_ns) => {
                words += 1;
            }
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    if ranges != 1 || words != 1 {
        return Err(FdxError::UnsupportedConstruct);
    }
    Ok(())
}

fn reject_nested_dual_dialogue(element: &RawElement) -> Result<(), FdxError> {
    for child in &element.children {
        if let RawNode::Element(child) = child {
            if child.name.local_name == "DualDialogue" {
                return Err(FdxError::UnsupportedConstruct);
            }
            reject_nested_dual_dialogue(child)?;
        }
    }
    Ok(())
}

fn validate_nested_member_topology(
    element: &RawElement,
    root_ns: Option<&str>,
    element_type: &ElementTypeKey,
) -> Result<(), FdxError> {
    let capabilities = SemanticCapabilities::for_key(element_type);
    let mut saw_alts = false;
    let mut saw_attachment = false;
    for child in &element.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            RawNode::Element(child)
                if is_fdx_element(child, "Text", root_ns)
                    && child
                        .children
                        .iter()
                        .all(|inline| matches!(inline, RawNode::Text(_) | RawNode::CData(_))) => {}
            RawNode::Element(child)
                if capabilities.scene_boundary
                    && is_fdx_element(child, "SceneProperties", root_ns) => {}
            RawNode::Element(child)
                if capabilities.dialogue && is_fdx_element(child, "Alts", root_ns) =>
            {
                if saw_alts {
                    return Err(FdxError::UnsupportedConstruct);
                }
                saw_alts = true;
            }
            RawNode::Element(child) if is_user_paragraph_data_name(&child.name, &element.name) => {
                if saw_attachment {
                    return Err(FdxError::UnsupportedConstruct);
                }
                saw_attachment = true;
            }
            RawNode::Element(child) => reject_nested_dual_dialogue(child)?,
            RawNode::Comment(_) | RawNode::ProcessingInstruction { .. } | RawNode::CData(_) => {}
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    Ok(())
}

fn dual_wrapper_direct_overrides(
    element: &RawElement,
) -> Result<Option<SourceParagraphOverrides>, FdxError> {
    let mut overrides = SourceParagraphOverrides::default();
    let paragraph = &mut overrides.paragraph;
    if let Some(value) = attribute(element, "LeftIndent") {
        paragraph.left_indent = Some(bounded_inches(value, false).ok_or(FdxError::InvalidValue)?);
    }
    if let Some(value) = attribute(element, "RightIndent") {
        paragraph.right_indent = Some(bounded_inches(value, false).ok_or(FdxError::InvalidValue)?);
    }
    if let Some(value) = attribute(element, "SpaceBefore") {
        paragraph.space_before = Some(
            FixedPoint::parse_points(value)
                .filter(|value| (0..=79_200).contains(&value.raw()))
                .ok_or(FdxError::InvalidValue)?,
        );
    }
    if let Some(value) = attribute(element, "StartsNewPage") {
        overrides.starts_new_page = Some(match value {
            "Yes" => true,
            "No" => false,
            _ => return Err(FdxError::InvalidValue),
        });
    }
    Ok((paragraph.left_indent.is_some()
        || paragraph.right_indent.is_some()
        || paragraph.space_before.is_some()
        || overrides.starts_new_page.is_some())
    .then_some(overrides))
}

pub(crate) enum ImportedSemanticParagraph {
    Body(ScreenplayElement),
    Title(TitlePageParagraph),
}

pub(crate) fn new_paragraph(
    semantic: &ScreenplayElement,
    direct_overrides: Option<SourceParagraphOverrides>,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedNode, FdxError> {
    let element_type = semantic.element_type();
    let mut paragraph = preserved_element("Paragraph", vec![("Type", element_type.as_str())]);
    if let Some(overrides) = direct_overrides {
        add_member_direct_attributes(&mut paragraph, overrides)?;
    }
    paragraph.attributes.push(PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: "id".to_owned(),
            qualified_name: Some("id".to_owned()),
        },
        value: String::new(),
    });
    let id_attribute = paragraph
        .attributes
        .iter_mut()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "id"
        })
        .ok_or(FdxError::UnsupportedConstruct)?;
    id_attribute.value = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    if semantic.text.is_empty() {
        if !semantic.runs.is_empty() {
            return Err(SaveDiagnosticCode::NewParagraphEmptyRunsMismatch.error());
        }
        return Ok(PreservedNode::Element(paragraph));
    }
    let mut runs = semantic.runs.clone();
    if runs.is_empty() {
        runs.push(TextRun {
            text: semantic.text.clone(),
            attributes: Vec::new(),
        });
    }
    for run in runs {
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
        if !run.text.is_empty() {
            text.children.push(PreservedNode::Text(run.text.into()));
        }
        paragraph.children.push(PreservedNode::Element(text));
    }
    Ok(PreservedNode::Element(paragraph))
}

pub(crate) fn add_member_direct_attributes(
    paragraph: &mut PreservedElement,
    overrides: SourceParagraphOverrides,
) -> Result<(), FdxError> {
    let spec = overrides.paragraph;
    if let Some(alignment) = spec.alignment {
        push_unqualified_attribute(paragraph, "Alignment", alignment.fdx_value());
    }
    for (name, value) in [
        ("FirstIndent", spec.first_indent),
        ("LeftIndent", spec.left_indent),
        ("RightIndent", spec.right_indent),
    ] {
        if let Some(value) = value {
            push_unqualified_attribute(paragraph, name, &fixed_inches_string(value));
        }
    }
    if spec.leading_regular == Some(true) {
        push_unqualified_attribute(paragraph, "Leading", "Regular");
    }
    if let Some(value) = spec.space_before {
        push_unqualified_attribute(paragraph, "SpaceBefore", &value.decimal_string());
    }
    if spec.spacing_single == Some(true) {
        push_unqualified_attribute(paragraph, "Spacing", "1");
    }
    if let Some(value) = overrides.starts_new_page {
        push_unqualified_attribute(paragraph, "StartsNewPage", if value { "Yes" } else { "No" });
    }
    Ok(())
}

pub(crate) fn add_wrapper_direct_attributes(
    paragraph: &mut PreservedElement,
    overrides: SourceParagraphOverrides,
) -> Result<(), FdxError> {
    if let Some(value) = overrides.paragraph.left_indent {
        push_unqualified_attribute(paragraph, "LeftIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.right_indent {
        push_unqualified_attribute(paragraph, "RightIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.space_before {
        push_unqualified_attribute(paragraph, "SpaceBefore", &value.decimal_string());
    }
    if let Some(value) = overrides.starts_new_page {
        push_unqualified_attribute(paragraph, "StartsNewPage", if value { "Yes" } else { "No" });
    }
    Ok(())
}

pub(crate) fn set_unqualified_attribute(
    element: &mut PreservedElement,
    name: &str,
    value: &str,
) -> Result<(), FdxError> {
    let mut matching = element.attributes.iter_mut().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    });
    let Some(attribute) = matching.next() else {
        return Err(FdxError::UnsupportedConstruct);
    };
    if matching.next().is_some() {
        return Err(FdxError::UnsupportedConstruct);
    }
    attribute.value = value.to_owned();
    Ok(())
}

fn push_unqualified_attribute(element: &mut PreservedElement, name: &str, value: &str) {
    element.attributes.push(PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: name.to_owned(),
            qualified_name: Some(name.to_owned()),
        },
        value: value.to_owned(),
    });
}

fn fixed_inches_string(value: FixedPoint) -> String {
    let raw = value.raw();
    const PER_INCH: u64 = 7_200;
    let magnitude = raw.unsigned_abs();
    let mut whole = magnitude / PER_INCH;
    let remainder = magnitude % PER_INCH;
    let mut fractional_millionths = (remainder * 1_000_000 + PER_INCH / 2) / PER_INCH;
    if fractional_millionths == 1_000_000 {
        whole += 1;
        fractional_millionths = 0;
    }
    let sign = if raw < 0 { "-" } else { "" };
    if fractional_millionths == 0 {
        return format!("{sign}{whole}");
    }
    let mut fraction = format!("{fractional_millionths:06}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{sign}{whole}.{fraction}")
}

impl ImportedSemanticParagraph {
    pub(crate) fn id(&self) -> &StableId {
        match self {
            Self::Body(element) => &element.id,
            Self::Title(paragraph) => &paragraph.id,
        }
    }
}

pub(crate) fn direct_paragraph_overrides(element: &RawElement) -> Option<SourceParagraphOverrides> {
    let mut overrides = SourceParagraphOverrides::default();
    let paragraph = &mut overrides.paragraph;
    paragraph.alignment = attribute(element, "Alignment").and_then(ParagraphAlignment::from_fdx);
    paragraph.first_indent =
        attribute(element, "FirstIndent").and_then(|value| bounded_inches(value, true));
    paragraph.left_indent =
        attribute(element, "LeftIndent").and_then(|value| bounded_inches(value, false));
    paragraph.right_indent =
        attribute(element, "RightIndent").and_then(|value| bounded_inches(value, false));
    paragraph.leading_regular = (attribute(element, "Leading") == Some("Regular")).then_some(true);
    paragraph.space_before = attribute(element, "SpaceBefore")
        .and_then(FixedPoint::parse_points)
        .filter(|value| (0..=79_200).contains(&value.raw()));
    paragraph.spacing_single = (attribute(element, "Spacing") == Some("1")).then_some(true);
    overrides.starts_new_page = attribute(element, "StartsNewPage").and_then(|value| match value {
        "Yes" => Some(true),
        "No" => Some(false),
        _ => None,
    });

    (paragraph.alignment.is_some()
        || paragraph.first_indent.is_some()
        || paragraph.left_indent.is_some()
        || paragraph.right_indent.is_some()
        || paragraph.leading_regular.is_some()
        || paragraph.space_before.is_some()
        || paragraph.spacing_single.is_some()
        || overrides.starts_new_page.is_some())
    .then_some(overrides)
}

pub(crate) fn make_semantic_element(
    element: &RawElement,
    element_type: ElementTypeKey,
    section: SectionKind,
    next_id: &mut usize,
) -> Result<ScreenplayElement, FdxError> {
    let id = format!("fdx-element-{}", *next_id);
    *next_id = next_id.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    let source_provenance = SourceProvenance {
        format: SourceFormat::Fdx,
        section: if section == SectionKind::TitlePage {
            SourceSection::TitlePage
        } else {
            SourceSection::Content
        },
        ordinal: u32::try_from(*next_id - 2).map_err(|_| FdxError::ResourceLimit)?,
        foreign_id: identity_attribute(element).map(str::to_owned),
        foreign_role: attribute(element, "Type").map(str::to_owned),
    };
    let capabilities = SemanticCapabilities::for_key(&element_type);
    let runs = semantic_text_runs(
        element,
        section == SectionKind::Content && capabilities.scene_boundary,
        section == SectionKind::Content && capabilities.dialogue,
    )?;
    ScreenplayElement::with_runs(id, element_type, runs, Some(source_provenance))
        .map_err(map_model_error)
}

fn classify_paragraph_child(
    owner: &RawElement,
    child: &RawElement,
    allow_scene_properties: bool,
    allow_inline_alts: bool,
) -> SourceClass {
    let namespace = owner.name.namespace_uri.as_deref();
    if is_fdx_element(child, "Text", namespace) {
        SourceClass::Managed
    } else if is_user_paragraph_data_name(&child.name, &owner.name)
        || (allow_scene_properties && is_fdx_element(child, "SceneProperties", namespace))
        || (allow_inline_alts && is_fdx_element(child, "Alts", namespace))
    {
        SourceClass::RecognizedAttachment
    } else {
        SourceClass::GenericOpaque
    }
}

pub(crate) fn semantic_text_runs(
    element: &RawElement,
    allow_scene_properties: bool,
    allow_inline_alts: bool,
) -> Result<Vec<TextRun>, FdxError> {
    let mut runs = Vec::new();
    for child in &element.children {
        let RawNode::Element(text_element) = child else {
            continue;
        };
        match classify_paragraph_child(
            element,
            text_element,
            allow_scene_properties,
            allow_inline_alts,
        ) {
            SourceClass::Managed => {}
            SourceClass::RecognizedAttachment
            | SourceClass::KnownIdentity
            | SourceClass::Structural
            | SourceClass::GenericOpaque
            | SourceClass::InvalidUnsafe => {
                // Opaque children, including recognized owner attachments,
                // remain at their source positions and are never text runs.
                continue;
            }
        }
        if text_element.name.local_name != "Text"
            || text_element.name.namespace_uri != element.name.namespace_uri
        {
            // Kept defensively in case a future classifier adds a managed
            // child class other than Text.
            continue;
        }
        let mut text = String::new();
        for inline in &text_element.children {
            match inline {
                RawNode::Text(value) | RawNode::CData(value) => text.push_str(value),
                // Inline elements and miscellaneous nodes remain in the raw
                // source tree. Their text is intentionally not flattened.
                RawNode::Element(_)
                | RawNode::Comment(_)
                | RawNode::ProcessingInstruction { .. } => {}
            }
        }
        if text.is_empty() {
            continue;
        }
        let attributes = text_element
            .attributes
            .iter()
            .map(|attribute| RunAttribute {
                name: attribute
                    .name
                    .qualified_name
                    .clone()
                    .unwrap_or_else(|| attribute.name.local_name.clone()),
                value: attribute.value.clone(),
            })
            .collect();
        runs.push(TextRun { text, attributes });
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptcraft_model::preservation::{PreservedElement, PreservedNode, SourceOwnership};

    fn valid<T, E>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|_| unreachable!("focused arbitrary-body FDX source is valid"))
    }

    fn attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
        element
            .attributes
            .iter()
            .find(|attribute| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
            })
            .map(|attribute| attribute.value.as_str())
    }

    fn child<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a PreservedElement> {
        element.children.iter().find_map(|node| match node {
            PreservedNode::Element(child)
                if child.name.local_name == name
                    && child.name.namespace_uri == element.name.namespace_uri =>
            {
                Some(child)
            }
            _ => None,
        })
    }

    fn body_source(project: &scriptcraft_model::Project) -> Option<&PreservedElement> {
        let root = project
            .preservation
            .as_ref()?
            .sections
            .iter()
            .flat_map(|section| section.children.iter())
            .find_map(|node| match node {
                PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
                _ => None,
            })?;
        child(root, "Content")
    }

    #[test]
    fn arbitrary_direct_body_types_are_managed_without_guessed_semantics() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6" xmlns:x="urn:foreign">
  <Content>
    <Paragraph Type="Lyrics" id="lyrics-foreign" LeftIndent="0.25">
      <Text Style="Bold">Some </Text><Text>lyrics</Text>
      <VendorPayload marker="retained">not canonical text</VendorPayload>
      <SceneProperties id="opaque-scene"><SceneArcBeats><SceneArcBeat id="opaque-beat"/></SceneArcBeats></SceneProperties>
      <Alts><AltCollection><Alt><Paragraph Type="Dialogue"><Text>alternate</Text></Paragraph></Alt></AltCollection></Alts>
    </Paragraph>
    <Paragraph Type="lyrics" id="lowercase-foreign"><Text>lowercase</Text></Paragraph>
    <Paragraph id="missing-type"><Text>missing</Text></Paragraph>
    <Paragraph Type="" id="empty-type"><Text>empty</Text></Paragraph>
    <Paragraph x:Type="Foreign" id="namespaced-type"><Text>foreign</Text></Paragraph>
  </Content>
</FinalDraft>"#;
        let imported = valid(crate::reader::read_v6(source));
        let project = imported.project;

        assert_eq!(project.elements.len(), 2);
        let lyrics = project.elements[0].as_element();
        let lowercase = project.elements[1].as_element();
        assert_eq!(lyrics.id.as_str(), "fdx-element-1");
        assert_eq!(lowercase.id.as_str(), "fdx-element-2");
        assert_eq!(lyrics.element_type().as_str(), "Lyrics");
        assert_eq!(lowercase.element_type().as_str(), "lyrics");
        assert_eq!(lyrics.text, "Some lyrics");
        assert_eq!(lyrics.runs.len(), 2);
        assert_eq!(lyrics.runs[0].text, "Some ");
        assert_eq!(lyrics.runs[0].attributes[0].name, "Style");
        assert_eq!(lyrics.runs[0].attributes[0].value, "Bold");
        assert_eq!(lyrics.runs[1].text, "lyrics");
        assert!(project.direct_paragraph_overrides.contains_key(&lyrics.id));
        assert_eq!(
            lyrics
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref()),
            Some("lyrics-foreign")
        );
        assert_eq!(
            project
                .element_definitions()
                .get_by_label("Lyrics")
                .map(|definition| definition.capabilities()),
            Some(SemanticCapabilities::default())
        );
        assert_eq!(
            project
                .element_definitions()
                .get_by_label("lyrics")
                .map(|definition| definition.capabilities()),
            Some(SemanticCapabilities::default())
        );

        let content = body_source(&project)
            .unwrap_or_else(|| unreachable!("the source Content section is retained"));
        let lyrics_source = content
            .children
            .iter()
            .find_map(|node| match node {
                PreservedNode::Element(paragraph)
                    if paragraph.name.local_name == "Paragraph"
                        && attribute(paragraph, "id") == Some("lyrics-foreign") =>
                {
                    Some(paragraph)
                }
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("the arbitrary body paragraph remains retained"));
        assert_eq!(
            lyrics_source.ownership,
            SourceOwnership::Managed(lyrics.id.clone())
        );
        assert!(lyrics_source.anchor.is_some());
        assert_eq!(lyrics_source.source_type.as_deref(), Some("Lyrics"));
        let payload = child(lyrics_source, "VendorPayload")
            .unwrap_or_else(|| unreachable!("unknown direct children remain in their owner"));
        assert_eq!(payload.ownership, SourceOwnership::Opaque);
        assert_eq!(attribute(payload, "marker"), Some("retained"));
        assert_eq!(
            payload
                .children
                .iter()
                .filter_map(|node| match node {
                    PreservedNode::Text(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>(),
            "not canonical text"
        );
        assert_eq!(
            child(lyrics_source, "SceneProperties").map(|node| node.ownership.clone()),
            Some(SourceOwnership::Opaque)
        );
        assert_eq!(
            child(lyrics_source, "Alts").map(|node| node.ownership.clone()),
            Some(SourceOwnership::Opaque)
        );

        for id in ["missing-type", "empty-type", "namespaced-type"] {
            let opaque = content.children.iter().find_map(|node| match node {
                PreservedNode::Element(paragraph)
                    if paragraph.name.local_name == "Paragraph"
                        && attribute(paragraph, "id") == Some(id) =>
                {
                    Some(paragraph)
                }
                _ => None,
            });
            assert!(opaque.is_some_and(|paragraph| {
                paragraph.ownership == SourceOwnership::Opaque && paragraph.anchor.is_none()
            }));
        }
    }

    #[test]
    fn preservation_free_body_construction_emits_exact_case_sensitive_type_keys() {
        for label in ["Lyrics", "lyrics"] {
            let key = ElementTypeKey::new(label).unwrap_or_else(|_| unreachable!());
            let semantic = ScreenplayElement::with_runs(
                format!("new-{label}"),
                key,
                vec![TextRun {
                    text: "sung words".to_owned(),
                    attributes: vec![RunAttribute {
                        name: "Style".to_owned(),
                        value: "Italic".to_owned(),
                    }],
                }],
                None,
            )
            .unwrap_or_else(|_| unreachable!());
            let mut paragraph_ids = ParagraphIdAllocator::new(Vec::<String>::new());
            let PreservedNode::Element(paragraph) = new_paragraph(
                &semantic,
                Some(SourceParagraphOverrides::default()),
                &mut paragraph_ids,
            )
            .unwrap_or_else(|_| unreachable!()) else {
                unreachable!("new paragraph construction returns an element")
            };

            assert_eq!(attribute(&paragraph, "Type"), Some(label));
            assert!(paragraph
                .attributes
                .iter()
                .any(|attribute| attribute.name.local_name == "id" && !attribute.value.is_empty()));
            let text = child(&paragraph, "Text")
                .unwrap_or_else(|| unreachable!("canonical runs construct Text children"));
            assert_eq!(
                text.children
                    .iter()
                    .filter_map(|node| match node {
                        PreservedNode::Text(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>(),
                "sung words"
            );
            assert_eq!(attribute(text, "Style"), Some("Italic"));
        }
    }
}
