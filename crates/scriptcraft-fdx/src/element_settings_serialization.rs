//! Shared ElementSettings source synthesis and serialization formatting helpers.

use crate::diagnostics::FdxError;
use scriptcraft_model::element_definition::ElementDefinition;
use scriptcraft_model::element_type::ElementTypeKey;
use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
use scriptcraft_model::preservation::{
    PreservedAttribute, PreservedElement, PreservedNode, SourceName, SourceOwnership,
};
use std::collections::BTreeMap;

const MAX_FONT_FAMILY_BYTES: usize = 128;

fn is_preserved_fdx_element(
    element: &PreservedElement,
    name: &str,
    root_ns: Option<&str>,
) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}

pub(crate) fn reconcile_owned_element_settings(
    setting: &mut PreservedElement,
    definition: &ElementDefinition,
    root_ns: Option<&str>,
) -> Result<(), FdxError> {
    if setting.name.local_name != "ElementSettings"
        || setting.name.namespace_uri.as_deref() != root_ns
        || setting.ownership != SourceOwnership::Opaque
        || definition.font.family.len() > MAX_FONT_FAMILY_BYTES
        || !(1..=72).contains(&definition.font.size)
        || definition.paragraph.leading.is_empty()
    {
        return Err(FdxError::InvalidValue);
    }
    set_unique_unqualified_attribute(setting, "Type", definition.key.as_str())?;
    setting.source_type = Some(definition.key.as_str().to_owned());

    let font = ensure_unique_child(setting, "FontSpec", root_ns)?;
    set_unique_unqualified_attribute(font, "Font", &definition.font.family)?;
    set_unique_unqualified_attribute(font, "Size", &definition.font.size.to_string())?;
    let style = [
        (definition.font.bold, "Bold"),
        (definition.font.italic, "Italic"),
        (definition.font.underline, "Underline"),
        (definition.font.all_caps, "AllCaps"),
        (definition.font.hidden_text, "HiddenText"),
    ]
    .into_iter()
    .filter_map(|(enabled, token)| enabled.then_some(token))
    .collect::<Vec<_>>()
    .join("+");
    set_unique_unqualified_attribute(font, "Style", &style)?;
    set_optional_unqualified_attribute(font, "Color", definition.font.color.as_deref())?;
    set_unique_unqualified_attribute(
        font,
        "AdornmentStyle",
        &definition.font.adornment_style.to_string(),
    )?;
    set_unique_unqualified_attribute(font, "RevisionID", &definition.font.revision_id.to_string())?;

    let paragraph = ensure_unique_child(setting, "ParagraphSpec", root_ns)?;
    set_unique_unqualified_attribute(
        paragraph,
        "Alignment",
        alignment_label(definition.paragraph.alignment),
    )?;
    set_unique_unqualified_attribute(
        paragraph,
        "FirstIndent",
        &format_inches(definition.paragraph.first_indent, true)?,
    )?;
    set_unique_unqualified_attribute(
        paragraph,
        "LeftIndent",
        &format_inches(definition.paragraph.left_indent, false)?,
    )?;
    set_unique_unqualified_attribute(
        paragraph,
        "RightIndent",
        &format_inches(definition.paragraph.right_indent, false)?,
    )?;
    set_unique_unqualified_attribute(paragraph, "Leading", &definition.paragraph.leading)?;
    set_unique_unqualified_attribute(
        paragraph,
        "SpaceBefore",
        &definition.paragraph.space_before.decimal_string(),
    )?;
    if !(0..=79_200).contains(&definition.paragraph.space_before.raw()) {
        return Err(FdxError::InvalidValue);
    }
    set_unique_unqualified_attribute(
        paragraph,
        "Spacing",
        &definition.paragraph.spacing.to_string(),
    )?;
    set_unique_unqualified_attribute(
        paragraph,
        "StartsNewPage",
        yes_no(definition.paragraph.starts_new_page),
    )?;
    set_unique_unqualified_attribute(paragraph, "Type", definition.key.as_str())?;
    paragraph.source_type = Some(definition.key.as_str().to_owned());

    let behavior = ensure_unique_child(setting, "Behavior", root_ns)?;
    set_optional_unqualified_attribute(
        behavior,
        "PaginateAs",
        definition.behavior.paginate_as.as_deref(),
    )?;
    set_optional_unqualified_attribute(
        behavior,
        "ReturnKey",
        definition
            .behavior
            .return_key
            .as_ref()
            .map(ElementTypeKey::as_str),
    )?;
    set_optional_unqualified_attribute(
        behavior,
        "TabKey",
        definition
            .behavior
            .tab_key
            .as_ref()
            .map(ElementTypeKey::as_str),
    )?;
    set_optional_unqualified_attribute(
        behavior,
        "Shortcut",
        definition.behavior.shortcut.as_deref(),
    )?;
    set_optional_unqualified_attribute(
        behavior,
        "WinShortcut",
        definition.behavior.win_shortcut.as_deref(),
    )?;

    let outline = ensure_unique_child(setting, "Outline", root_ns)?;
    set_unique_unqualified_attribute(outline, "CanHide", yes_no(definition.outline.can_hide))?;
    set_unique_unqualified_attribute(outline, "Level", &definition.outline.level.to_string())?;
    Ok(())
}

fn ensure_unique_child<'a>(
    setting: &'a mut PreservedElement,
    name: &str,
    root_ns: Option<&str>,
) -> Result<&'a mut PreservedElement, FdxError> {
    let mut found = None;
    for (index, child) in setting.children.iter().enumerate() {
        if matches!(child, PreservedNode::Element(element) if is_preserved_fdx_element(element, name, root_ns))
        {
            if found.is_some() {
                return Err(FdxError::InvalidValue);
            }
            found = Some(index);
        }
    }
    let index = if let Some(index) = found {
        index
    } else {
        let child = new_empty_child(name, setting);
        setting.children.push(PreservedNode::Element(child));
        setting.children.len() - 1
    };
    match setting.children.get_mut(index) {
        Some(PreservedNode::Element(element)) => Ok(element),
        _ => Err(FdxError::InvalidValue),
    }
}

fn new_empty_child(name: &str, parent: &PreservedElement) -> PreservedElement {
    PreservedElement {
        name: derived_name(name, &parent.name),
        attributes: Vec::new(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: parent.dependency_confidence,
        anchor: None,
        children: Vec::new().into(),
    }
}

fn set_unique_unqualified_attribute(
    element: &mut PreservedElement,
    name: &str,
    value: &str,
) -> Result<(), FdxError> {
    let mut found = None;
    for (index, attribute) in element.attributes.iter().enumerate() {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == name {
            if found.is_some() {
                return Err(FdxError::InvalidValue);
            }
            found = Some(index);
        }
    }
    if let Some(index) = found {
        element.attributes[index].value = value.to_owned();
    } else {
        element.attributes.push(PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: name.to_owned(),
                qualified_name: Some(name.to_owned()),
            },
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn set_optional_unqualified_attribute(
    element: &mut PreservedElement,
    name: &str,
    value: Option<&str>,
) -> Result<(), FdxError> {
    if let Some(value) = value {
        return set_unique_unqualified_attribute(element, name, value);
    }
    let mut count = 0usize;
    for attribute in &element.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == name {
            count += 1;
        }
    }
    if count > 1 {
        return Err(FdxError::InvalidValue);
    }
    element.attributes.retain(|attribute| {
        attribute.name.namespace_uri.is_some() || attribute.name.local_name != name
    });
    Ok(())
}

fn alignment_label(alignment: ParagraphAlignment) -> &'static str {
    match alignment {
        ParagraphAlignment::Left => "Left",
        ParagraphAlignment::Right => "Right",
        ParagraphAlignment::Center => "Center",
        ParagraphAlignment::Justified => "Full",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}

/// Six decimal inch places preserve every hundredth-point value when parsed by
/// the Phase 2 fixed-point reader, using integer-only rounded arithmetic.
fn format_inches(value: FixedPoint, signed: bool) -> Result<String, FdxError> {
    let raw = value.raw();
    if raw.unsigned_abs() > 61_200 || (!signed && raw < 0) {
        return Err(FdxError::InvalidValue);
    }
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let scaled = magnitude.saturating_mul(1_000_000).saturating_add(3_600) / 7_200;
    let sign = if negative && scaled != 0 { "-" } else { "" };
    Ok(format!(
        "{sign}{}.{:06}",
        scaled / 1_000_000,
        scaled % 1_000_000
    ))
}

fn derived_name(local_name: &str, parent: &SourceName) -> SourceName {
    let prefix = parent
        .qualified_name
        .as_deref()
        .and_then(|qualified| qualified.rsplit_once(':').map(|(prefix, _)| prefix));
    let qualified_name = prefix
        .map(|prefix| format!("{prefix}:{local_name}"))
        .unwrap_or_else(|| local_name.to_owned());
    SourceName {
        namespace_uri: parent.namespace_uri.clone(),
        local_name: local_name.to_owned(),
        qualified_name: Some(qualified_name),
    }
}

pub(crate) fn make_owned_element_settings(
    definition: &ElementDefinition,
    root: &PreservedElement,
) -> Result<PreservedElement, FdxError> {
    let mut setting = PreservedElement {
        name: derived_name("ElementSettings", &root.name),
        attributes: Vec::new(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: root.dependency_confidence,
        anchor: None,
        children: Vec::new().into(),
    };
    reconcile_owned_element_settings(&mut setting, definition, root.name.namespace_uri.as_deref())?;
    Ok(setting)
}

/// Serializes one fully specified settings node from the authoritative owned
/// definition using the same serializer as source-backed projection.
pub(crate) fn serialize_element_settings(
    definition: &ElementDefinition,
    root: &PreservedElement,
) -> Result<PreservedElement, FdxError> {
    make_owned_element_settings(definition, root)
}
