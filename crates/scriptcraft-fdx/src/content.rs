//! FDX body content and supported dual-dialogue semantic mapping.
//! The FDX package owns this mapping between source content and model values.

use crate::diagnostics::{map_model_error, map_paragraph_id_error, FdxError, SaveDiagnosticCode};
use crate::identity::raw_identity_attribute as identity_attribute;
use crate::reader::{attribute, validate_xml_chars, RawElement, RawNode};
use crate::settings::{bounded_inches, role_from_fdx};
use crate::tree::{preserve_misc_node, preserve_opaque_element, preserved_element};
use crate::writer::validate_qname;
use scriptcraft_model::dual_dialogue::{
    DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance,
};
use scriptcraft_model::formatting::{
    formatting_for_role, FixedPoint, ParagraphAlignment, SourceLayoutSettings,
    SourceParagraphOverrides,
};
use scriptcraft_model::preservation::{
    is_user_paragraph_data_name, AnchorRelation,
    AnchorTarget, DependencyConfidence, PreservedAttribute, PreservedElement, PreservedNode,
    SectionKind, SourceAnchor, SourceAnchorId, SourceName, SourceOwnership,
};
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::{
    BodyOwner, Project, Role, RunAttribute, ScreenplayElement, SourceFormat,
    SourceProvenance, SourceSection, StableId, TextRun, TitlePageParagraph,
};

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}

pub(crate) use crate::reader::{classify_preserved_element, SourceClass};

pub(crate) fn is_supported_dual_dialogue_source(outer: &RawElement, root_ns: Option<&str>) -> bool {
    if !is_fdx_element(outer, "Paragraph", root_ns)
        || attribute(outer, "Type") != Some("General")
    {
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
    let mut roles = Vec::new();
    for child in &dual.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            RawNode::Element(element)
                if is_fdx_element(element, "SpellCheckIgnoreLists", root_ns) =>
            {
                if spell_blocks != 0
                    || !roles.is_empty()
                    || validate_nested_spell_check(element, root_ns).is_err()
                {
                    return false;
                }
                spell_blocks += 1;
            }
            RawNode::Element(element) if is_fdx_element(element, "Paragraph", root_ns) => {
                let Some(role) = attribute(element, "Type").and_then(role_from_fdx) else {
                    return false;
                };
                if !matches!(role, Role::Character | Role::Dialogue | Role::Parenthetical)
                    || validate_nested_member_topology(element, root_ns, role).is_err()
                {
                    return false;
                }
                roles.push(role);
            }
            _ => return false,
        }
    }
    if spell_blocks != 1 || roles.len() < 2 || roles.first() != Some(&Role::Character) {
        return false;
    }
    let Some(right_cue) = roles
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, role)| (*role == Role::Character).then_some(index))
    else {
        return false;
    };
    !roles[1..right_cue]
        .iter()
        .any(|role| !matches!(role, Role::Dialogue | Role::Parenthetical))
        && !roles[right_cue + 1..]
            .iter()
            .any(|role| !matches!(role, Role::Dialogue | Role::Parenthetical))
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
    let member_start = project.elements.len();
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
    let mut roles = Vec::new();
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
                let role = attribute(element, "Type")
                    .and_then(role_from_fdx)
                    .ok_or(FdxError::UnsupportedConstruct)?;
                if !matches!(
                    role,
                    Role::Character | Role::Dialogue | Role::Parenthetical
                ) {
                    return Err(FdxError::UnsupportedConstruct);
                }
                validate_nested_member_topology(element, root_ns, role)?;
                let semantic = make_semantic_element(
                    element,
                    role,
                    SectionKind::Content,
                    next_id,
                )?;
                let member_id = semantic.id.clone();
                let anchor_id = next_source_anchor(next_anchor)?;
                let member_overrides = direct_paragraph_overrides(element);
                if let Some(overrides) = member_overrides {
                    staged_member_overrides.push((member_id.clone(), overrides));
                }
                staged_anchors.push(SourceAnchor {
                    id: anchor_id.clone(),
                    target: AnchorTarget::ProjectElement(
                        member_id.clone(),
                    ),
                    relation: AnchorRelation::Represents,
                });
                roles.push(role);
                staged_members.push(semantic);
                dual_children.push(PreservedNode::Element(
                    preserved_managed_nested_paragraph(element, member_id, anchor_id)?,
                ));
            }
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    if spell_blocks != 1 {
        return Err(FdxError::UnsupportedConstruct);
    }
    let right_cue = roles
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, role)| (*role == Role::Character).then_some(index))
        .ok_or(FdxError::UnsupportedConstruct)?;
    if roles.first() != Some(&Role::Character)
        || roles[1..right_cue]
            .iter()
            .any(|role| !matches!(role, Role::Dialogue | Role::Parenthetical))
        || roles[right_cue + 1..]
            .iter()
            .any(|role| !matches!(role, Role::Dialogue | Role::Parenthetical))
        || roles[right_cue + 1..]
            .iter()
            .any(|role| *role == Role::Character)
    {
        return Err(FdxError::UnsupportedConstruct);
    }

    // The semantic members were appended in source order. Their IDs are also
    // the group membership authority; the outer General remains invisible.
    if staged_members.len() != roles.len() {
        return Err(FdxError::UnsupportedConstruct);
    }
    let members = staged_members.as_slice();
    let group_id = StableId::new(format!(
        "fdx-dual-group-{}",
        project.dual_dialogue_groups.len().checked_add(1).ok_or(FdxError::ResourceLimit)?
    ))
    .map_err(map_model_error)?;
    let left = DualDialogueSide {
        character: members[0].id.clone(),
        paragraphs: members[1..right_cue]
            .iter()
            .map(|member| member.id.clone())
            .collect(),
    };
    let right = DualDialogueSide {
        character: members[right_cue].id.clone(),
        paragraphs: members[right_cue + 1..]
            .iter()
            .map(|member| member.id.clone())
            .collect(),
    };

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
        left,
        right,
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
    project.elements.extend(staged_members.into_iter().map(BodyOwner::shared));
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

fn validate_nested_spell_check(element: &RawElement, root_ns: Option<&str>) -> Result<(), FdxError> {
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
    role: Role,
) -> Result<(), FdxError> {
    let mut saw_alts = false;
    let mut saw_attachment = false;
    for child in &element.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            RawNode::Element(child)
                if is_fdx_element(child, "Text", root_ns)
                    && child.children.iter().all(|inline| {
                        matches!(inline, RawNode::Text(_) | RawNode::CData(_))
                    }) => {}
            RawNode::Element(child)
                if role == Role::SceneHeading && is_fdx_element(child, "SceneProperties", root_ns) => {}
            RawNode::Element(child)
                if role == Role::Dialogue && is_fdx_element(child, "Alts", root_ns) =>
            {
                if saw_alts {
                    return Err(FdxError::UnsupportedConstruct);
                }
                saw_alts = true;
            }
            RawNode::Element(child)
                if is_user_paragraph_data_name(&child.name, &element.name) && !saw_attachment =>
            {
                saw_attachment = true;
            }
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
    let mut paragraph = preserved_element("Paragraph", vec![("Type", role_to_fdx(semantic.role))]);
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
        push_unqualified_attribute(
            paragraph,
            "StartsNewPage",
            if value { "Yes" } else { "No" },
        );
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
        push_unqualified_attribute(
            paragraph,
            "StartsNewPage",
            if value { "Yes" } else { "No" },
        );
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
    let whole = magnitude / PER_INCH;
    let remainder = magnitude % PER_INCH;
    if remainder == 0 {
        return format!("{}{whole}", if raw < 0 { "-" } else { "" });
    }
    let mut fraction = format!("{remainder:06}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{}{whole}.{fraction}", if raw < 0 { "-" } else { "" })
}

pub(crate) fn role_to_fdx(role: Role) -> &'static str {
    formatting_for_role(role).fdx_label
}

impl ImportedSemanticParagraph {
    pub(crate) fn id(&self) -> &StableId {
        match self {
            Self::Body(element) => &element.id,
            Self::Title(paragraph) => &paragraph.id,
        }
    }
}

pub(crate) fn direct_paragraph_overrides(
    element: &RawElement,
) -> Option<SourceParagraphOverrides> {
    let mut overrides = SourceParagraphOverrides::default();
    let paragraph = &mut overrides.paragraph;
    paragraph.alignment = attribute(element, "Alignment")
        .and_then(ParagraphAlignment::from_fdx);
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
    role: Role,
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
    let runs = semantic_text_runs(
        element,
        section == SectionKind::Content && role == Role::SceneHeading,
        section == SectionKind::Content && role == Role::Dialogue,
    )?;
    ScreenplayElement::with_runs(id, role, runs, Some(source_provenance)).map_err(map_model_error)
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
                RawNode::Element(_) | RawNode::Comment(_) | RawNode::ProcessingInstruction { .. } => {}
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
