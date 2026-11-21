//! Raw ElementSettings parsing, import normalization, and definition completion.

use crate::diagnostics::FdxError;
use crate::reader::{attribute, RawElement, RawNode};
use crate::settings::bounded_inches;
use scriptcraft_model::element_definition::{
    ElementBehavior, ElementDefinition, ElementFontSpec, ElementOutline, ElementParagraphSpec,
    MAX_ELEMENT_DEFINITION_STRING_BYTES,
};
use scriptcraft_model::element_definitions::MAX_DOCUMENT_ELEMENT_DEFINITIONS;
use scriptcraft_model::element_type::ElementTypeKey;
use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
use scriptcraft_model::Project;
use std::collections::{BTreeMap, BTreeSet};

const MAX_FONT_FAMILY_BYTES: usize = 128;

/// Source-ordered winning ElementSettings drafts and their duplicate report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ImportedElementSettings {
    pub(crate) winners: Vec<ElementSettingsDraft>,
    pub(crate) diagnostics: crate::document::FdxImportDiagnostics,
}

/// Partial semantic projection of one winning direct-root ElementSettings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElementSettingsDraft {
    pub(crate) key: ElementTypeKey,
    /// Zero-based among direct-root, same-namespace ElementSettings elements.
    pub(crate) settings_ordinal: usize,
    pub(crate) font: Option<FontSpecDraft>,
    pub(crate) paragraph: Option<ParagraphSpecDraft>,
    pub(crate) behavior: Option<BehaviorDraft>,
    pub(crate) outline: Option<OutlineDraft>,
}

impl ElementSettingsDraft {
    /// Completes a packaged required-base draft without borrowing or consulting
    /// the transitional static-role formatting table.
    pub(crate) fn complete_packaged_definition(&self) -> Result<ElementDefinition, FdxError> {
        let font = self.font.as_ref().ok_or(FdxError::InvalidValue)?;
        let paragraph = self.paragraph.as_ref().ok_or(FdxError::InvalidValue)?;
        let behavior = self.behavior.as_ref().ok_or(FdxError::InvalidValue)?;
        let outline = self.outline.ok_or(FdxError::InvalidValue)?;

        let font = ElementFontSpec {
            family: required(font.family.clone())?,
            size: required(font.size)?,
            bold: required(font.bold)?,
            italic: required(font.italic)?,
            underline: required(font.underline)?,
            all_caps: required(font.all_caps)?,
            hidden_text: required(font.hidden_text)?,
            color: font.color.clone(),
            adornment_style: required(font.adornment_style)?,
            revision_id: required(font.revision_id)?,
        };
        let paragraph = ElementParagraphSpec {
            alignment: required(paragraph.alignment)?,
            first_indent: required(paragraph.first_indent)?,
            left_indent: required(paragraph.left_indent)?,
            right_indent: required(paragraph.right_indent)?,
            leading: required(paragraph.leading.clone())?,
            space_before: required(paragraph.space_before)?,
            spacing: required(paragraph.spacing)?,
            starts_new_page: required(paragraph.starts_new_page)?,
        };
        let behavior = ElementBehavior {
            paginate_as: Some(required(behavior.paginate_as.clone())?),
            return_key: Some(parse_required_target(behavior.return_key.as_deref())?),
            shortcut: Some(required(behavior.shortcut.clone())?),
            tab_key: Some(parse_required_target(behavior.tab_key.as_deref())?),
            win_shortcut: Some(required(behavior.win_shortcut.clone())?),
        };
        ElementDefinition::new(
            self.key.clone(),
            font,
            paragraph,
            behavior,
            ElementOutline {
                can_hide: required(outline.can_hide)?,
                level: required(outline.level)?,
            },
        )
        .map_err(crate::diagnostics::map_model_error)
    }
}

fn required<T>(value: Option<T>) -> Result<T, FdxError> {
    value.ok_or(FdxError::InvalidValue)
}

fn parse_required_target(value: Option<&str>) -> Result<ElementTypeKey, FdxError> {
    let value = value
        .filter(|value| !value.is_empty())
        .ok_or(FdxError::InvalidValue)?;
    ElementTypeKey::new(value.to_owned()).map_err(|_| FdxError::InvalidValue)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FontSpecDraft {
    pub(crate) family: Option<String>,
    pub(crate) size: Option<u8>,
    pub(crate) bold: Option<bool>,
    pub(crate) italic: Option<bool>,
    pub(crate) underline: Option<bool>,
    pub(crate) all_caps: Option<bool>,
    pub(crate) hidden_text: Option<bool>,
    pub(crate) color: Option<String>,
    /// Distinguishes an explicitly normalized black/white Color from an absent
    /// or malformed field when applying this draft over General-derived values.
    pub(crate) normalized_color: bool,
    pub(crate) adornment_style: Option<u8>,
    pub(crate) revision_id: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParagraphSpecDraft {
    pub(crate) alignment: Option<ParagraphAlignment>,
    pub(crate) first_indent: Option<FixedPoint>,
    pub(crate) left_indent: Option<FixedPoint>,
    pub(crate) right_indent: Option<FixedPoint>,
    pub(crate) leading: Option<String>,
    pub(crate) space_before: Option<FixedPoint>,
    pub(crate) spacing: Option<u8>,
    pub(crate) starts_new_page: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BehaviorDraft {
    pub(crate) paginate_as: Option<String>,
    pub(crate) return_key: Option<String>,
    pub(crate) tab_key: Option<String>,
    pub(crate) shortcut: Option<String>,
    pub(crate) win_shortcut: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OutlineDraft {
    pub(crate) can_hide: Option<bool>,
    pub(crate) level: Option<u8>,
}

/// Reads direct-root settings in source order. Later exact duplicates remain in
/// retained XML but are omitted from the semantic winner list.
pub(crate) fn read_element_settings(
    root: &RawElement,
    root_ns: Option<&str>,
) -> Result<ImportedElementSettings, FdxError> {
    let mut imported = ImportedElementSettings::default();
    let mut seen = std::collections::BTreeSet::new();
    let mut ordinal = 0usize;

    for node in &root.children {
        let RawNode::Element(setting) = node else {
            continue;
        };
        if !is_fdx_element(setting, "ElementSettings", root_ns) {
            continue;
        }
        let settings_ordinal = ordinal;
        ordinal = ordinal.checked_add(1).ok_or(FdxError::ResourceLimit)?;

        let Some(label) = attribute(setting, "Type") else {
            continue;
        };
        if label.is_empty() {
            continue;
        }
        if label.len() > scriptcraft_model::element_type::MAX_ELEMENT_TYPE_KEY_BYTES {
            return Err(FdxError::ResourceLimit);
        }
        let key = ElementTypeKey::new(label.to_owned()).map_err(|_| FdxError::InvalidValue)?;
        if !seen.insert(key.clone()) {
            if imported.diagnostics.entries.len() < crate::document::MAX_FDX_IMPORT_DIAGNOSTICS {
                imported.diagnostics.entries.push(
                    crate::document::FdxImportDiagnostic::DuplicateElementSettings {
                        settings_ordinal,
                    },
                );
            } else {
                imported.diagnostics.suppressed_duplicate_count = imported
                    .diagnostics
                    .suppressed_duplicate_count
                    .saturating_add(1);
            }
            continue;
        }

        imported.winners.push(ElementSettingsDraft {
            key,
            settings_ordinal,
            font: unique_child(setting, "FontSpec", root_ns).map(read_font_spec),
            paragraph: unique_child(setting, "ParagraphSpec", root_ns)
                .filter(|paragraph| {
                    attribute(paragraph, "Type")
                        .map_or(true, |paragraph_type| paragraph_type == label)
                })
                .map(read_paragraph_spec),
            behavior: unique_child(setting, "Behavior", root_ns).map(read_behavior),
            outline: unique_child(setting, "Outline", root_ns).map(read_outline),
        });
    }

    Ok(imported)
}

/// Removes only normalized black/white Color attributes from unambiguous
/// FontSpec children belonging to first-exact-label winning settings. All
/// other retained FontSpec data, including later duplicate settings, stays
/// byte-for-byte equivalent in the source projection.
pub(crate) fn normalize_winning_font_colors(
    root: &mut RawElement,
    root_ns: Option<&str>,
    imported: &ImportedElementSettings,
) -> Result<(), FdxError> {
    let mut ordinal = 0usize;

    for node in &mut root.children {
        let RawNode::Element(setting) = node else {
            continue;
        };
        if !is_fdx_element(setting, "ElementSettings", root_ns) {
            continue;
        }

        let settings_ordinal = ordinal;
        ordinal = ordinal.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        if !imported
            .winners
            .iter()
            .any(|winner| winner.settings_ordinal == settings_ordinal)
        {
            continue;
        }

        let Some(font_spec) = unique_child_mut(setting, "FontSpec", root_ns) else {
            continue;
        };
        let Some(color) =
            bounded_attribute(font_spec, "Color", MAX_ELEMENT_DEFINITION_STRING_BYTES)
        else {
            continue;
        };
        if normalize_import_color(color).is_some() {
            continue;
        }

        font_spec.attributes.retain(|attribute| {
            attribute.name.namespace_uri.is_some() || attribute.name.local_name != "Color"
        });
    }

    Ok(())
}

/// Completes the ordered definitions derived from source settings, first-use
/// paragraph types, and packaged required bases before import publication.
pub(crate) fn complete_document_definitions(
    root: &RawElement,
    imported: &ImportedElementSettings,
    project: &mut Project,
) -> Result<(), FdxError> {
    let defaults = crate::builtin_screenplay::screenplay_defaults()?;
    let packaged_general = defaults.general();
    let general_draft = imported
        .winners
        .iter()
        .find(|draft| draft.key.as_str() == "General");
    let mut effective_general = packaged_general;
    if let Some(draft) = general_draft {
        overlay_draft(draft, &mut effective_general);
    }
    let mut general_targets = DeferredTargets::from_definition(&effective_general);
    if let Some(draft) = general_draft {
        overlay_targets(draft, &mut general_targets);
    }

    let mut definitions = Vec::new();
    let mut deferred_targets = Vec::new();
    let mut defined = BTreeSet::new();

    for draft in &imported.winners {
        let definition = if draft.key.as_str() == "General" {
            effective_general.clone()
        } else {
            let mut definition = effective_general.clone();
            definition.key = draft.key.clone();
            overlay_draft(draft, &mut definition);
            definition
        };
        let targets = if draft.key.as_str() == "General" {
            general_targets.clone()
        } else {
            let mut targets = general_targets.clone();
            overlay_targets(draft, &mut targets);
            targets
        };
        if !defined.insert(definition.key.clone()) {
            // The parser has already applied first-exact-label-wins. Keep this
            // guard local so a future caller cannot publish a duplicate list.
            continue;
        }
        push_definition(&mut definitions, &mut deferred_targets, definition, targets)?;
    }

    for key in collect_paragraph_used_types(root, root.name.namespace_uri.as_deref())? {
        if defined.contains(&key) {
            continue;
        }
        let (definition, targets) = if let Some(base) =
            crate::builtin_screenplay::REQUIRED_BASE_LABELS
                .iter()
                .find(|label| **label == key.as_str())
                .copied()
        {
            let definition = defaults.definition(base).ok_or(FdxError::InvalidValue)?;
            let targets = DeferredTargets::from_definition(&definition);
            (definition, targets)
        } else {
            let mut definition = effective_general.clone();
            definition.key = key.clone();
            (definition, general_targets.clone())
        };
        defined.insert(key);
        push_definition(&mut definitions, &mut deferred_targets, definition, targets)?;
    }

    for label in crate::builtin_screenplay::REQUIRED_BASE_LABELS {
        if defined.iter().any(|key| key.as_str() == label) {
            continue;
        }
        let definition = defaults.definition(label).ok_or(FdxError::InvalidValue)?;
        let targets = DeferredTargets::from_definition(&definition);
        defined.insert(definition.key.clone());
        push_definition(&mut definitions, &mut deferred_targets, definition, targets)?;
    }

    // Behavior references are intentionally converted only after every exact
    // key is known. Missing targets remain their original open keys.
    let complete_keys = definitions
        .iter()
        .map(|definition| definition.key.clone())
        .collect::<Vec<_>>();
    for (definition, targets) in definitions.iter_mut().zip(deferred_targets) {
        definition.behavior.return_key =
            resolve_deferred_target(targets.return_key, &complete_keys)?;
        definition.behavior.tab_key = resolve_deferred_target(targets.tab_key, &complete_keys)?;
    }

    project
        .replace_element_definitions(definitions)
        .map_err(crate::diagnostics::map_model_error)
}

/// Collects typed paragraphs only at the managed body/title Content boundaries.
/// Title structures outside TitlePage/Content, opaque ancestry, and unsupported
/// dual wrappers never contribute a key.
pub(crate) fn collect_paragraph_used_types(
    root: &RawElement,
    root_ns: Option<&str>,
) -> Result<Vec<ElementTypeKey>, FdxError> {
    let mut keys = Vec::new();
    let mut seen = BTreeSet::new();
    for node in &root.children {
        let RawNode::Element(section) = node else {
            continue;
        };
        if !is_fdx_element(section, "Content", root_ns) {
            if is_fdx_element(section, "TitlePage", root_ns) {
                collect_title_page_content_types(section, root_ns, &mut seen, &mut keys)?;
            }
            continue;
        }
        for child in &section.children {
            let RawNode::Element(paragraph) = child else {
                continue;
            };
            if !is_fdx_element(paragraph, "Paragraph", root_ns) {
                continue;
            }
            let has_dual_name = paragraph.children.iter().any(|child| {
                matches!(child, RawNode::Element(element) if element.name.local_name == "DualDialogue")
            });
            if has_dual_name {
                if crate::content::is_supported_dual_dialogue_source(paragraph, root_ns) {
                    collect_supported_dual_member_types(paragraph, root_ns, &mut seen, &mut keys)?;
                }
                continue;
            }
            collect_type_key(paragraph, root_ns, &mut seen, &mut keys)?;
        }
    }
    Ok(keys)
}

fn collect_title_page_content_types(
    title_page: &RawElement,
    root_ns: Option<&str>,
    seen: &mut BTreeSet<ElementTypeKey>,
    keys: &mut Vec<ElementTypeKey>,
) -> Result<(), FdxError> {
    for child in &title_page.children {
        let RawNode::Element(element) = child else {
            continue;
        };
        if is_fdx_element(element, "Content", root_ns) {
            for content_child in &element.children {
                let RawNode::Element(paragraph) = content_child else {
                    continue;
                };
                if is_fdx_element(paragraph, "Paragraph", root_ns) {
                    collect_type_key(paragraph, root_ns, seen, keys)?;
                }
            }
        }
    }
    Ok(())
}

fn collect_supported_dual_member_types(
    wrapper: &RawElement,
    root_ns: Option<&str>,
    seen: &mut BTreeSet<ElementTypeKey>,
    keys: &mut Vec<ElementTypeKey>,
) -> Result<(), FdxError> {
    for child in &wrapper.children {
        let RawNode::Element(dual) = child else {
            continue;
        };
        if !is_fdx_element(dual, "DualDialogue", root_ns) {
            continue;
        }
        for member in &dual.children {
            let RawNode::Element(paragraph) = member else {
                continue;
            };
            if is_fdx_element(paragraph, "Paragraph", root_ns) {
                collect_type_key(paragraph, root_ns, seen, keys)?;
            }
        }
    }
    Ok(())
}

fn collect_type_key(
    element: &RawElement,
    root_ns: Option<&str>,
    seen: &mut BTreeSet<ElementTypeKey>,
    keys: &mut Vec<ElementTypeKey>,
) -> Result<(), FdxError> {
    if element.name.namespace_uri.as_deref() != root_ns {
        return Ok(());
    }
    let Some(label) = attribute(element, "Type").filter(|label| !label.is_empty()) else {
        return Ok(());
    };
    let key = ElementTypeKey::new(label.to_owned()).map_err(|_| {
        if label.len() > scriptcraft_model::element_type::MAX_ELEMENT_TYPE_KEY_BYTES {
            FdxError::ResourceLimit
        } else {
            FdxError::InvalidValue
        }
    })?;
    if seen.insert(key.clone()) {
        keys.push(key);
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct DeferredTargets {
    return_key: Option<String>,
    tab_key: Option<String>,
}

impl DeferredTargets {
    fn from_definition(definition: &ElementDefinition) -> Self {
        Self {
            return_key: definition
                .behavior
                .return_key
                .as_ref()
                .map(|key| key.as_str().to_owned()),
            tab_key: definition
                .behavior
                .tab_key
                .as_ref()
                .map(|key| key.as_str().to_owned()),
        }
    }
}

fn overlay_targets(draft: &ElementSettingsDraft, targets: &mut DeferredTargets) {
    if let Some(behavior) = &draft.behavior {
        if let Some(value) = &behavior.return_key {
            targets.return_key = (!value.is_empty()).then(|| value.clone());
        }
        if let Some(value) = &behavior.tab_key {
            targets.tab_key = (!value.is_empty()).then(|| value.clone());
        }
    }
}

fn resolve_deferred_target(
    value: Option<String>,
    keys: &[ElementTypeKey],
) -> Result<Option<ElementTypeKey>, FdxError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let key = ElementTypeKey::new(value).map_err(crate::diagnostics::map_model_error)?;
    Ok(Some(
        keys.iter()
            .find(|known| known.as_str() == key.as_str())
            .cloned()
            .unwrap_or(key),
    ))
}

fn push_definition(
    definitions: &mut Vec<ElementDefinition>,
    deferred_targets: &mut Vec<DeferredTargets>,
    definition: ElementDefinition,
    targets: DeferredTargets,
) -> Result<(), FdxError> {
    if definitions.len() >= MAX_DOCUMENT_ELEMENT_DEFINITIONS {
        return Err(FdxError::ResourceLimit);
    }
    definition
        .validate()
        .map_err(crate::diagnostics::map_model_error)?;
    definitions.push(definition);
    deferred_targets.push(targets);
    Ok(())
}

fn overlay_draft(draft: &ElementSettingsDraft, definition: &mut ElementDefinition) {
    if let Some(font) = &draft.font {
        if let Some(value) = &font.family {
            definition.font.family.clone_from(value);
        }
        if let Some(value) = font.size {
            definition.font.size = value;
        }
        if let Some(value) = font.bold {
            definition.font.bold = value;
        }
        if let Some(value) = font.italic {
            definition.font.italic = value;
        }
        if let Some(value) = font.underline {
            definition.font.underline = value;
        }
        if let Some(value) = font.all_caps {
            definition.font.all_caps = value;
        }
        if let Some(value) = font.hidden_text {
            definition.font.hidden_text = value;
        }
        if let Some(value) = &font.color {
            definition.font.color = Some(value.clone());
        } else if font.normalized_color {
            definition.font.color = None;
        }
        if let Some(value) = font.adornment_style {
            definition.font.adornment_style = value;
        }
        if let Some(value) = font.revision_id {
            definition.font.revision_id = value;
        }
    }
    if let Some(paragraph) = &draft.paragraph {
        if let Some(value) = paragraph.alignment {
            definition.paragraph.alignment = value;
        }
        if let Some(value) = paragraph.first_indent {
            definition.paragraph.first_indent = value;
        }
        if let Some(value) = paragraph.left_indent {
            definition.paragraph.left_indent = value;
        }
        if let Some(value) = paragraph.right_indent {
            definition.paragraph.right_indent = value;
        }
        if let Some(value) = &paragraph.leading {
            definition.paragraph.leading.clone_from(value);
        }
        if let Some(value) = paragraph.space_before {
            definition.paragraph.space_before = value;
        }
        if let Some(value) = paragraph.spacing {
            definition.paragraph.spacing = value;
        }
        if let Some(value) = paragraph.starts_new_page {
            definition.paragraph.starts_new_page = value;
        }
    }
    if let Some(behavior) = &draft.behavior {
        if let Some(value) = &behavior.paginate_as {
            definition.behavior.paginate_as = Some(value.clone());
        }
        if let Some(value) = &behavior.shortcut {
            definition.behavior.shortcut = Some(value.clone());
        }
        if let Some(value) = &behavior.win_shortcut {
            definition.behavior.win_shortcut = Some(value.clone());
        }
    }
    if let Some(outline) = draft.outline {
        if let Some(value) = outline.can_hide {
            definition.outline.can_hide = value;
        }
        if let Some(value) = outline.level {
            definition.outline.level = value;
        }
    }
}

/// Returns no semantic color only for exact 12-digit hexadecimal black/white.
/// Other spellings are returned verbatim by the caller when within its bound.
pub(crate) fn normalize_import_color(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let exact_black =
        bytes.len() == 13 && bytes[0] == b'#' && bytes[1..].iter().all(|b| *b == b'0');
    let exact_white = bytes.len() == 13
        && bytes[0] == b'#'
        && bytes[1..].iter().all(|b| b.eq_ignore_ascii_case(&b'F'));
    (!exact_black && !exact_white).then(|| value.to_owned())
}

fn read_font_spec(element: &RawElement) -> FontSpecDraft {
    let style = bounded_attribute(element, "Style", MAX_ELEMENT_DEFINITION_STRING_BYTES)
        .and_then(parse_style_tokens);
    let raw_color = bounded_attribute(element, "Color", MAX_ELEMENT_DEFINITION_STRING_BYTES);
    let color = raw_color.and_then(normalize_import_color);
    let normalized_color = raw_color.is_some() && color.is_none();
    FontSpecDraft {
        family: bounded_attribute(element, "Font", MAX_FONT_FAMILY_BYTES)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        size: bounded_attribute(element, "Size", 2)
            .and_then(crate::settings::parse_bounded_font_size),
        bold: style.as_ref().map(|tokens| tokens.contains("Bold")),
        italic: style.as_ref().map(|tokens| tokens.contains("Italic")),
        underline: style.as_ref().map(|tokens| tokens.contains("Underline")),
        all_caps: style.as_ref().map(|tokens| tokens.contains("AllCaps")),
        hidden_text: style.as_ref().map(|tokens| tokens.contains("HiddenText")),
        color,
        normalized_color,
        adornment_style: bounded_attribute(element, "AdornmentStyle", 3).and_then(parse_u8),
        revision_id: bounded_attribute(element, "RevisionID", 3).and_then(parse_u8),
    }
}

fn read_paragraph_spec(element: &RawElement) -> ParagraphSpecDraft {
    ParagraphSpecDraft {
        alignment: bounded_attribute(element, "Alignment", 16).and_then(|value| match value {
            "Left" => Some(ParagraphAlignment::Left),
            "Right" => Some(ParagraphAlignment::Right),
            "Center" => Some(ParagraphAlignment::Center),
            "Full" => Some(ParagraphAlignment::Justified),
            _ => None,
        }),
        first_indent: bounded_attribute(element, "FirstIndent", 32)
            .and_then(|value| bounded_inches(value, true)),
        left_indent: bounded_attribute(element, "LeftIndent", 32)
            .and_then(|value| bounded_inches(value, false)),
        right_indent: bounded_attribute(element, "RightIndent", 32)
            .and_then(|value| bounded_inches(value, false)),
        leading: bounded_attribute(element, "Leading", MAX_ELEMENT_DEFINITION_STRING_BYTES)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        space_before: bounded_attribute(element, "SpaceBefore", 32)
            .and_then(FixedPoint::parse_points)
            .filter(|value| (0..=79_200).contains(&value.raw())),
        spacing: bounded_attribute(element, "Spacing", 3).and_then(parse_u8),
        starts_new_page: bounded_attribute(element, "StartsNewPage", 3).and_then(parse_yes_no),
    }
}

fn read_behavior(element: &RawElement) -> BehaviorDraft {
    BehaviorDraft {
        paginate_as: bounded_attribute(element, "PaginateAs", MAX_ELEMENT_DEFINITION_STRING_BYTES)
            .map(str::to_owned),
        return_key: bounded_attribute(element, "ReturnKey", MAX_ELEMENT_DEFINITION_STRING_BYTES)
            .map(str::to_owned),
        tab_key: bounded_attribute(element, "TabKey", MAX_ELEMENT_DEFINITION_STRING_BYTES)
            .map(str::to_owned),
        shortcut: bounded_attribute(element, "Shortcut", MAX_ELEMENT_DEFINITION_STRING_BYTES)
            .map(str::to_owned),
        win_shortcut: bounded_attribute(
            element,
            "WinShortcut",
            MAX_ELEMENT_DEFINITION_STRING_BYTES,
        )
        .map(str::to_owned),
    }
}

fn read_outline(element: &RawElement) -> OutlineDraft {
    OutlineDraft {
        can_hide: bounded_attribute(element, "CanHide", 3).and_then(parse_yes_no),
        level: bounded_attribute(element, "Level", 3).and_then(parse_u8),
    }
}

fn unique_child<'a>(
    setting: &'a RawElement,
    name: &str,
    root_ns: Option<&str>,
) -> Option<&'a RawElement> {
    let mut found = None;
    for child in &setting.children {
        let RawNode::Element(element) = child else {
            continue;
        };
        if !is_fdx_element(element, name, root_ns) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(element);
    }
    found
}

fn unique_child_mut<'a>(
    setting: &'a mut RawElement,
    name: &str,
    root_ns: Option<&str>,
) -> Option<&'a mut RawElement> {
    let mut found_index = None;
    for (index, child) in setting.children.iter().enumerate() {
        let RawNode::Element(element) = child else {
            continue;
        };
        if !is_fdx_element(element, name, root_ns) {
            continue;
        }
        if found_index.is_some() {
            return None;
        }
        found_index = Some(index);
    }
    match found_index {
        Some(index) => match &mut setting.children[index] {
            RawNode::Element(element) => Some(element),
            _ => None,
        },
        None => None,
    }
}

fn bounded_attribute<'a>(element: &'a RawElement, name: &str, max_bytes: usize) -> Option<&'a str> {
    attribute(element, name).filter(|value| value.len() <= max_bytes)
}

fn parse_style_tokens(value: &str) -> Option<std::collections::BTreeSet<&str>> {
    if value.is_empty() {
        return Some(std::collections::BTreeSet::new());
    }
    let tokens = value.split('+').collect::<std::collections::BTreeSet<_>>();
    (!tokens.iter().any(|token| token.is_empty())).then_some(tokens)
}

fn parse_u8(value: &str) -> Option<u8> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

fn parse_yes_no(value: &str) -> Option<bool> {
    match value {
        "Yes" => Some(true),
        "No" => Some(false),
        _ => None,
    }
}

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}
