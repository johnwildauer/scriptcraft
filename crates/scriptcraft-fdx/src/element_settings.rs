//! Bounded, open parsing of direct-root FDX ElementSettings.
//!
//! These drafts preserve only supported semantic fields. The raw ElementSettings
//! subtree remains authoritative for every unrecognized or malformed value.

use crate::diagnostics::FdxError;
use crate::reader::{attribute, RawElement, RawNode};
use crate::settings::bounded_inches;
use scriptcraft_model::element_definition::{
    ElementBehavior, ElementDefinition, ElementFontSpec, ElementOutline, ElementParagraphSpec,
    MAX_ELEMENT_DEFINITION_STRING_BYTES,
};
use scriptcraft_model::element_definitions::{
    DocumentElementDefinitions, MAX_DOCUMENT_ELEMENT_DEFINITIONS,
};
use scriptcraft_model::element_type::ElementTypeKey;
use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
use scriptcraft_model::preservation::{
    AnchorRelation, AnchorTarget, PreservationState, PreservedAttribute, PreservedElement,
    PreservedNode, PreservedSection, SourceAnchor, SourceAnchorId, SourceName, SourceOwnership,
    MAX_SOURCE_EVENTS,
};
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

/// Binds only the first-wins direct-root settings ordinals captured by the
/// Phase 2 importer. Later duplicates, foreign-namespace settings, and nodes
/// without one exact unqualified Type remain unanchored.
pub(crate) fn anchor_document_element_settings(
    preservation: &mut PreservationState,
    imported: &ImportedElementSettings,
) -> Result<(), FdxError> {
    let mut staged = preservation.clone();
    let mut used = staged
        .anchors
        .iter()
        .map(|anchor| anchor.id.clone())
        .collect::<BTreeSet<_>>();
    let mut new_anchors = Vec::new();
    with_direct_root_mut(&mut staged.sections, |root| {
        let root_ns = root.name.namespace_uri.as_deref();
        let winners = imported
            .winners
            .iter()
            .map(|winner| (winner.settings_ordinal, &winner.key))
            .collect::<BTreeMap<_, _>>();
        if winners.len() != imported.winners.len() {
            return Err(FdxError::InvalidValue);
        }
        let mut ordinal = 0usize;
        let mut next_anchor_ordinal = 0usize;
        for child in &mut root.children {
            let PreservedNode::Element(setting) = child else {
                continue;
            };
            if !is_preserved_fdx_element(setting, "ElementSettings", root_ns) {
                continue;
            }
            let setting_ordinal = ordinal;
            ordinal = ordinal.checked_add(1).ok_or(FdxError::ResourceLimit)?;
            let Some(key) = winners.get(&setting_ordinal) else {
                continue;
            };
            if setting.anchor.is_some()
                || setting.ownership != SourceOwnership::Opaque
                || setting.source_type.as_deref() != Some(key.as_str())
                || exact_unqualified_attribute(setting, "Type") != Some(key.as_str())
            {
                return Err(FdxError::InvalidValue);
            }
            let anchor_id = allocate_definition_anchor(&used, &mut next_anchor_ordinal)?;
            used.insert(anchor_id.clone());
            setting.anchor = Some(anchor_id.clone());
            new_anchors.push(SourceAnchor {
                id: anchor_id,
                target: AnchorTarget::ElementDefinition((*key).clone()),
                relation: AnchorRelation::Represents,
            });
        }
        if winners.keys().any(|winner| *winner >= ordinal) {
            return Err(FdxError::InvalidValue);
        }
        if new_anchors.len() != imported.winners.len() {
            return Err(FdxError::InvalidValue);
        }
        Ok(())
    })?;
    staged.anchors.extend(new_anchors);
    staged
        .validate()
        .map_err(crate::diagnostics::map_preservation_error)?;
    preservation.sections = staged.sections;
    preservation.anchors = staged.anchors;
    Ok(())
}

fn allocate_definition_anchor(
    used: &BTreeSet<SourceAnchorId>,
    next_ordinal: &mut usize,
) -> Result<SourceAnchorId, FdxError> {
    while *next_ordinal < MAX_SOURCE_EVENTS {
        let ordinal = *next_ordinal;
        *next_ordinal = next_ordinal.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        let anchor = SourceAnchorId::for_element_definition(ordinal)
            .map_err(crate::diagnostics::map_preservation_error)?;
        if !used.contains(&anchor) {
            return Ok(anchor);
        }
    }
    Err(FdxError::ResourceLimit)
}

fn with_direct_root_mut<T>(
    sections: &mut [PreservedSection],
    mut operation: impl FnMut(&mut PreservedElement) -> Result<T, FdxError>,
) -> Result<T, FdxError> {
    let mut result = None;
    for section in sections {
        for node in &mut section.children {
            let PreservedNode::Element(root) = node else {
                continue;
            };
            if root.name.local_name != crate::constants::FDX_ROOT {
                continue;
            }
            if result.is_some() {
                return Err(FdxError::InvalidValue);
            }
            result = Some(operation(root)?);
        }
    }
    result.ok_or(FdxError::InvalidValue)
}

fn is_preserved_fdx_element(element: &PreservedElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}

fn exact_unqualified_attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    let mut found = None;
    for attribute in &element.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == name {
            if found.is_some() {
                return None;
            }
            found = Some(attribute.value.as_str());
        }
    }
    found
}

/// Reconciles the retained direct-root ElementSettings projection from the
/// authoritative ordered registry. The operation stages the complete source
/// projection and anchors before publishing any mutation.
pub fn reconcile_project_element_settings(project: &mut Project) -> Result<(), FdxError> {
    let definitions = project.element_definitions().clone();
    let preservation = project
        .preservation
        .as_mut()
        .ok_or(FdxError::InvalidValue)?;
    reconcile_retained_project_element_settings(preservation, &definitions)
}

pub(crate) fn reconcile_retained_project_element_settings(
    preservation: &mut PreservationState,
    definitions: &DocumentElementDefinitions,
) -> Result<(), FdxError> {
    definitions
        .validate()
        .map_err(crate::diagnostics::map_model_error)?;
    let mut staged = preservation.clone();
    let mut staged_anchors = staged.anchors.clone();
    let used_anchor_ids = staged_anchors
        .iter()
        .map(|anchor| anchor.id.clone())
        .collect::<BTreeSet<_>>();
    let definition_keys = definitions
        .iter()
        .map(|definition| definition.key.clone())
        .collect::<BTreeSet<_>>();

    with_direct_root_mut(&mut staged.sections, |root| {
        reconcile_direct_root_settings(
            root,
            definitions,
            &definition_keys,
            &mut staged_anchors,
            &used_anchor_ids,
        )
    })?;

    staged.anchors = staged_anchors;
    staged
        .validate()
        .map_err(crate::diagnostics::map_preservation_error)?;
    staged
        .validate_element_definition_anchors(&definition_keys)
        .map_err(crate::diagnostics::map_preservation_error)?;
    preservation.sections = staged.sections;
    preservation.anchors = staged.anchors;
    Ok(())
}

fn reconcile_direct_root_settings(
    root: &mut PreservedElement,
    definitions: &DocumentElementDefinitions,
    definition_keys: &BTreeSet<ElementTypeKey>,
    anchors: &mut Vec<SourceAnchor>,
    used_anchor_ids: &BTreeSet<SourceAnchorId>,
) -> Result<(), FdxError> {
    let root_ns = root.name.namespace_uri.as_deref();
    let original = std::mem::take(&mut root.children)
        .into_iter()
        .collect::<Vec<_>>();
    let mut source_settings = Vec::<(usize, Option<ElementTypeKey>, Option<SourceAnchorId>)>::new();
    let mut anchored_keys = BTreeSet::new();
    let mut obsolete_keys = BTreeSet::new();
    let mut anchor_targets = BTreeMap::<SourceAnchorId, (&ElementTypeKey, AnchorRelation)>::new();
    for anchor in anchors.iter() {
        if let AnchorTarget::ElementDefinition(key) = &anchor.target {
            if anchor_targets
                .insert(anchor.id.clone(), (key, anchor.relation))
                .is_some()
            {
                return Err(FdxError::InvalidValue);
            }
            if !definition_keys.contains(key) {
                obsolete_keys.insert(key.clone());
            }
        }
    }

    for (index, node) in original.iter().enumerate() {
        let PreservedNode::Element(setting) = node else {
            continue;
        };
        if !is_preserved_fdx_element(setting, "ElementSettings", root_ns) {
            continue;
        }
        let type_label = exact_unqualified_attribute(setting, "Type");
        let key = type_label.and_then(|label| ElementTypeKey::new(label.to_owned()).ok());
        let anchor = setting.anchor.clone();
        if let Some(anchor_id) = &anchor {
            let Some((target_key, relation)) = anchor_targets.get(anchor_id) else {
                return Err(FdxError::InvalidValue);
            };
            if *relation != AnchorRelation::Represents
                || key.as_ref() != Some(*target_key)
                || setting.source_type.as_deref() != Some(target_key.as_str())
            {
                return Err(FdxError::InvalidValue);
            }
            if definition_keys.contains(*target_key) && !anchored_keys.insert((*target_key).clone())
            {
                return Err(FdxError::InvalidValue);
            }
        }
        source_settings.push((index, key, anchor));
    }

    let mut selected = BTreeMap::<ElementTypeKey, usize>::new();
    for (index, key, anchor) in &source_settings {
        let Some(key) = key else {
            continue;
        };
        if !definition_keys.contains(key) || obsolete_keys.contains(key) {
            continue;
        }
        if anchor_targets.values().any(|(target, _)| *target == key) {
            if anchor.as_ref().is_some_and(
                |id| matches!(anchor_targets.get(id), Some((target, _)) if *target == key),
            ) {
                selected.insert(key.clone(), *index);
            }
        } else if !selected.contains_key(key) {
            // A source settings family with no prior winner anchor can only be
            // adopted by its first exact-label node; following duplicates stay
            // inert and unanchored.
            selected.insert(key.clone(), *index);
        }
    }

    let mut next_anchor_ordinal = 0usize;
    let mut winner_nodes = BTreeMap::<ElementTypeKey, PreservedNode>::new();
    for definition in definitions.iter() {
        let key = definition.key.clone();
        let mut setting = if let Some(index) = selected.get(&key) {
            match original.get(*index) {
                Some(PreservedNode::Element(setting)) => setting.clone(),
                _ => return Err(FdxError::InvalidValue),
            }
        } else {
            make_owned_element_settings(definition, root)?
        };
        reconcile_owned_element_settings(&mut setting, definition, root_ns)?;
        if setting.anchor.is_none() {
            let anchor_id = allocate_definition_anchor(used_anchor_ids, &mut next_anchor_ordinal)?;
            setting.anchor = Some(anchor_id.clone());
            anchors.push(SourceAnchor {
                id: anchor_id,
                target: AnchorTarget::ElementDefinition(key.clone()),
                relation: AnchorRelation::Represents,
            });
        } else {
            let anchor_id = setting.anchor.as_ref().ok_or(FdxError::InvalidValue)?;
            if !anchors.iter().any(|anchor| {
                &anchor.id == anchor_id
                    && matches!(&anchor.target, AnchorTarget::ElementDefinition(target) if target == &key)
            }) {
                return Err(FdxError::InvalidValue);
            }
        }
        winner_nodes.insert(key, PreservedNode::Element(setting));
    }

    // Drop complete source families only for keys that previously had a
    // winning anchor and have now left the registry. Case-distinct and
    // unanchored settings families remain retained.
    anchors.retain(|anchor| {
        !matches!(&anchor.target, AnchorTarget::ElementDefinition(key) if obsolete_keys.contains(key))
    });

    let winner_indices = selected.values().copied().collect::<BTreeSet<_>>();
    let mut inert_settings = Vec::new();
    let mut first_settings_index = None;
    let mut source_index_by_index = BTreeMap::new();
    for (index, key, _) in &source_settings {
        first_settings_index.get_or_insert(*index);
        source_index_by_index.insert(*index, key.clone());
    }
    for (index, key) in &source_index_by_index {
        if winner_indices.contains(index) {
            continue;
        }
        if key.as_ref().is_some_and(|key| obsolete_keys.contains(key)) {
            continue;
        }
        if let Some(node) = original.get(*index) {
            inert_settings.push(node.clone());
        }
    }

    let mut output = Vec::with_capacity(original.len() + winner_nodes.len());
    let insertion_index = first_settings_index.unwrap_or_else(|| {
        original
            .iter()
            .position(|node| match node {
                PreservedNode::Element(element) => {
                    is_preserved_fdx_element(element, "Content", root_ns)
                        || is_preserved_fdx_element(element, "TitlePage", root_ns)
                }
                _ => false,
            })
            .unwrap_or(original.len())
    });
    let mut inserted = false;
    for (index, node) in original.into_iter().enumerate() {
        if index == insertion_index {
            append_ordered_settings(&mut output, &mut winner_nodes, definitions);
            output.append(&mut inert_settings);
            inserted = true;
        }
        if source_index_by_index.contains_key(&index) {
            continue;
        }
        output.push(node);
    }
    if !inserted {
        append_ordered_settings(&mut output, &mut winner_nodes, definitions);
        output.append(&mut inert_settings);
    }
    root.children = output.into();
    Ok(())
}

/// Replaces the complete direct-root settings region with caller-provided
/// ordered nodes, validating each node against the supplied owned definitions
/// before assigning fresh winning anchors. All unrelated root children remain
/// in their original relative order.
pub fn replace_project_element_settings(
    preservation: &mut PreservationState,
    definitions: &DocumentElementDefinitions,
    replacement_nodes: Vec<PreservedElement>,
) -> Result<(), FdxError> {
    definitions
        .validate()
        .map_err(crate::diagnostics::map_model_error)?;
    if replacement_nodes.len() != definitions.len()
        || replacement_nodes.len() > MAX_DOCUMENT_ELEMENT_DEFINITIONS
    {
        return Err(FdxError::InvalidValue);
    }
    let mut validated_nodes = Vec::with_capacity(replacement_nodes.len());
    for (node, definition) in replacement_nodes.into_iter().zip(definitions.iter()) {
        if node.anchor.is_some()
            || node.source_type.as_deref() != Some(definition.key.as_str())
            || exact_unqualified_attribute(&node, "Type") != Some(definition.key.as_str())
        {
            return Err(FdxError::InvalidValue);
        }
        let mut projected = node.clone();
        reconcile_owned_element_settings(
            &mut projected,
            definition,
            node.name.namespace_uri.as_deref(),
        )?;
        if projected != node {
            return Err(FdxError::InvalidValue);
        }
        validated_nodes.push(node);
    }

    let mut staged = preservation.clone();
    let mut staged_anchors = staged
        .anchors
        .iter()
        .filter(|anchor| !matches!(anchor.target, AnchorTarget::ElementDefinition(_)))
        .cloned()
        .collect::<Vec<_>>();
    let used_anchor_ids = staged
        .anchors
        .iter()
        .map(|anchor| anchor.id.clone())
        .collect::<BTreeSet<_>>();
    let definition_keys = definitions
        .iter()
        .map(|definition| definition.key.clone())
        .collect::<BTreeSet<_>>();
    let mut next_anchor_ordinal = 0usize;

    with_direct_root_mut(&mut staged.sections, |root| {
        let root_ns = root.name.namespace_uri.as_deref();
        let original = std::mem::take(&mut root.children)
            .into_iter()
            .collect::<Vec<_>>();
        let first_setting_index = original.iter().position(|node| {
            matches!(node, PreservedNode::Element(element) if is_preserved_fdx_element(element, "ElementSettings", root_ns))
        });
        let insertion_index = first_setting_index.unwrap_or_else(|| {
            original
                .iter()
                .position(|node| match node {
                    PreservedNode::Element(element) => {
                        is_preserved_fdx_element(element, "Content", root_ns)
                            || is_preserved_fdx_element(element, "TitlePage", root_ns)
                    }
                    _ => false,
                })
                .unwrap_or(original.len())
        });
        let mut ordered_nodes = Vec::with_capacity(validated_nodes.len());
        for (mut node, definition) in validated_nodes.clone().into_iter().zip(definitions.iter()) {
            if node.name.namespace_uri.as_deref() != root_ns {
                return Err(FdxError::InvalidValue);
            }
            let anchor_id = allocate_definition_anchor(&used_anchor_ids, &mut next_anchor_ordinal)?;
            node.anchor = Some(anchor_id.clone());
            staged_anchors.push(SourceAnchor {
                id: anchor_id,
                target: AnchorTarget::ElementDefinition(definition.key.clone()),
                relation: AnchorRelation::Represents,
            });
            ordered_nodes.push(PreservedNode::Element(node));
        }
        let mut output = Vec::with_capacity(original.len() + ordered_nodes.len());
        let mut inserted = false;
        for (index, node) in original.into_iter().enumerate() {
            if index == insertion_index {
                output.append(&mut ordered_nodes);
                inserted = true;
            }
            if matches!(&node, PreservedNode::Element(element) if is_preserved_fdx_element(element, "ElementSettings", root_ns))
            {
                continue;
            }
            output.push(node);
        }
        if !inserted {
            output.append(&mut ordered_nodes);
        }
        root.children = output.into();
        Ok(())
    })?;

    staged.anchors = staged_anchors;
    staged
        .validate()
        .map_err(crate::diagnostics::map_preservation_error)?;
    staged
        .validate_element_definition_anchors(&definition_keys)
        .map_err(crate::diagnostics::map_preservation_error)?;
    preservation.sections = staged.sections;
    preservation.anchors = staged.anchors;
    Ok(())
}

fn append_ordered_settings(
    output: &mut Vec<PreservedNode>,
    winner_nodes: &mut BTreeMap<ElementTypeKey, PreservedNode>,
    definitions: &DocumentElementDefinitions,
) {
    for definition in definitions.iter() {
        if let Some(node) = winner_nodes.remove(&definition.key) {
            output.push(node);
        }
    }
}

fn reconcile_owned_element_settings(
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

fn make_owned_element_settings(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::RawAttribute;
    use scriptcraft_model::preservation::SourceName;

    fn raw_element(name: &str, attributes: &[(&str, &str)], children: Vec<RawNode>) -> RawElement {
        RawElement {
            name: SourceName {
                namespace_uri: None,
                local_name: name.to_owned(),
                qualified_name: Some(name.to_owned()),
            },
            attributes: attributes
                .iter()
                .map(|(name, value)| RawAttribute {
                    name: SourceName {
                        namespace_uri: None,
                        local_name: (*name).to_owned(),
                        qualified_name: Some((*name).to_owned()),
                    },
                    value: (*value).to_owned(),
                })
                .collect(),
            namespace_bindings: Vec::new(),
            children,
        }
    }

    fn setting(label: Option<&str>, children: Vec<RawNode>) -> RawNode {
        let attributes = label.map(|value| vec![("Type", value)]).unwrap_or_default();
        RawNode::Element(raw_element("ElementSettings", &attributes, children))
    }

    fn child(name: &str, attributes: &[(&str, &str)]) -> RawNode {
        RawNode::Element(raw_element(name, attributes, Vec::new()))
    }

    #[test]
    fn parses_arbitrary_case_distinct_labels_and_first_exact_wins_ordinals() {
        let root = raw_element(
            "FinalDraft",
            &[],
            vec![
                setting(None, vec![]),
                setting(Some(""), vec![]),
                setting(
                    Some("Lyrics"),
                    vec![child("Behavior", &[("Shortcut", "first")])],
                ),
                setting(Some("lyrics"), vec![]),
                setting(
                    Some("Lyrics"),
                    vec![child("Behavior", &[("Shortcut", "inert")])],
                ),
            ],
        );
        let imported = read_element_settings(&root, None).unwrap_or_default();

        assert_eq!(imported.winners.len(), 2);
        assert_eq!(imported.winners[0].key.as_str(), "Lyrics");
        assert_eq!(imported.winners[0].settings_ordinal, 2);
        assert_eq!(imported.winners[1].key.as_str(), "lyrics");
        assert_eq!(imported.winners[1].settings_ordinal, 3);
        assert_eq!(
            imported.winners[0]
                .behavior
                .as_ref()
                .and_then(|behavior| behavior.shortcut.as_deref()),
            Some("first")
        );
        assert_eq!(
            imported.diagnostics.entries,
            vec![
                crate::document::FdxImportDiagnostic::DuplicateElementSettings {
                    settings_ordinal: 4
                }
            ]
        );
    }

    #[test]
    fn parses_all_supported_definition_fields_without_resolving_targets() {
        let root = raw_element(
            "FinalDraft",
            &[],
            vec![setting(
                Some("Arbitrary"),
                vec![
                    child(
                        "FontSpec",
                        &[
                            ("Font", "A Custom Family"),
                            ("Size", "16"),
                            ("Style", "Bold+Italic+Underline+AllCaps+HiddenText"),
                            ("Color", "#87878F8FE0E0"),
                            ("AdornmentStyle", "2"),
                            ("RevisionID", "7"),
                        ],
                    ),
                    child(
                        "ParagraphSpec",
                        &[
                            ("Alignment", "Full"),
                            ("FirstIndent", "-0.10"),
                            ("LeftIndent", "1.25"),
                            ("RightIndent", "6.50"),
                            ("Leading", "Regular"),
                            ("SpaceBefore", "12.5"),
                            ("Spacing", "2"),
                            ("StartsNewPage", "Yes"),
                        ],
                    ),
                    child(
                        "Behavior",
                        &[
                            ("PaginateAs", "Custom Pagination"),
                            ("ReturnKey", "Not Yet Defined"),
                            ("TabKey", "Another Missing Type"),
                            ("Shortcut", ""),
                            ("WinShortcut", "Ctrl+7"),
                        ],
                    ),
                    child("Outline", &[("CanHide", "Yes"), ("Level", "6")]),
                ],
            )],
        );
        let imported = read_element_settings(&root, None).unwrap_or_default();
        let draft = &imported.winners[0];
        let font = draft
            .font
            .as_ref()
            .unwrap_or(&FontSpecDraft::default())
            .clone();
        assert_eq!(font.family.as_deref(), Some("A Custom Family"));
        assert_eq!(font.size, Some(16));
        assert_eq!(font.bold, Some(true));
        assert_eq!(font.italic, Some(true));
        assert_eq!(font.underline, Some(true));
        assert_eq!(font.all_caps, Some(true));
        assert_eq!(font.hidden_text, Some(true));
        assert_eq!(font.color.as_deref(), Some("#87878F8FE0E0"));
        assert_eq!(font.adornment_style, Some(2));
        assert_eq!(font.revision_id, Some(7));

        let paragraph = draft
            .paragraph
            .as_ref()
            .unwrap_or(&ParagraphSpecDraft::default())
            .clone();
        assert_eq!(paragraph.alignment, Some(ParagraphAlignment::Justified));
        assert_eq!(paragraph.first_indent.map(FixedPoint::raw), Some(-720));
        assert_eq!(paragraph.left_indent.map(FixedPoint::raw), Some(9_000));
        assert_eq!(paragraph.right_indent.map(FixedPoint::raw), Some(46_800));
        assert_eq!(paragraph.leading.as_deref(), Some("Regular"));
        assert_eq!(paragraph.space_before.map(FixedPoint::raw), Some(1_250));
        assert_eq!(paragraph.spacing, Some(2));
        assert_eq!(paragraph.starts_new_page, Some(true));

        let behavior = draft
            .behavior
            .as_ref()
            .unwrap_or(&BehaviorDraft::default())
            .clone();
        assert_eq!(behavior.paginate_as.as_deref(), Some("Custom Pagination"));
        assert_eq!(behavior.return_key.as_deref(), Some("Not Yet Defined"));
        assert_eq!(behavior.tab_key.as_deref(), Some("Another Missing Type"));
        assert_eq!(behavior.shortcut.as_deref(), Some(""));
        assert_eq!(behavior.win_shortcut.as_deref(), Some("Ctrl+7"));
        assert_eq!(
            draft.outline,
            Some(OutlineDraft {
                can_hide: Some(true),
                level: Some(6)
            })
        );
    }

    #[test]
    fn malformed_fields_and_ambiguous_children_remain_absent() {
        let root = raw_element(
            "FinalDraft",
            &[],
            vec![
                setting(
                    Some("Ambiguous"),
                    vec![
                        child("FontSpec", &[("Font", "first")]),
                        child("FontSpec", &[("Font", "second")]),
                        child("ParagraphSpec", &[("Alignment", "Left")]),
                        child("ParagraphSpec", &[("Alignment", "Full")]),
                    ],
                ),
                setting(
                    Some("Malformed"),
                    vec![
                        child(
                            "FontSpec",
                            &[("Font", ""), ("Size", "999"), ("Color", "custom")],
                        ),
                        child(
                            "ParagraphSpec",
                            &[
                                ("Alignment", "Diagonal"),
                                ("FirstIndent", "-bad"),
                                ("LeftIndent", "-0.10"),
                                ("RightIndent", "2.00"),
                                ("SpaceBefore", "-1"),
                                ("Spacing", "bad"),
                                ("StartsNewPage", "Maybe"),
                            ],
                        ),
                        child("Behavior", &[("Shortcut", "")]),
                        child("Outline", &[("CanHide", "Maybe"), ("Level", "x")]),
                    ],
                ),
            ],
        );
        let imported = read_element_settings(&root, None).unwrap_or_default();
        let ambiguous = &imported.winners[0];
        assert!(ambiguous.font.is_none());
        assert!(ambiguous.paragraph.is_none());

        let draft = &imported.winners[1];
        let font = draft
            .font
            .as_ref()
            .unwrap_or(&FontSpecDraft::default())
            .clone();
        assert_eq!(font.family, None);
        assert_eq!(font.size, None);
        assert_eq!(font.color.as_deref(), Some("custom"));
        let paragraph = draft
            .paragraph
            .as_ref()
            .unwrap_or(&ParagraphSpecDraft::default())
            .clone();
        assert_eq!(paragraph.alignment, None);
        assert_eq!(paragraph.first_indent, None);
        assert_eq!(paragraph.left_indent, None);
        assert_eq!(paragraph.right_indent.map(FixedPoint::raw), Some(14_400));
        assert_eq!(paragraph.space_before, None);
        assert_eq!(paragraph.spacing, None);
        assert_eq!(paragraph.starts_new_page, None);
        assert_eq!(
            draft
                .behavior
                .as_ref()
                .and_then(|value| value.shortcut.as_deref()),
            Some("")
        );
        assert_eq!(draft.outline, Some(OutlineDraft::default()));
    }

    #[test]
    fn duplicate_diagnostics_are_capped_and_suppressed_count_is_bounded() {
        let mut children = vec![setting(Some("Duplicate"), vec![])];
        children.extend((0..40).map(|_| setting(Some("Duplicate"), vec![])));
        let root = raw_element("FinalDraft", &[], children);
        let imported = read_element_settings(&root, None).unwrap_or_default();

        assert_eq!(imported.winners.len(), 1);
        assert_eq!(
            imported.diagnostics.entries.len(),
            crate::document::MAX_FDX_IMPORT_DIAGNOSTICS
        );
        assert_eq!(imported.diagnostics.suppressed_duplicate_count, 8);
        assert_eq!(
            imported.diagnostics.entries[0],
            crate::document::FdxImportDiagnostic::DuplicateElementSettings {
                settings_ordinal: 1
            }
        );
        assert_eq!(
            imported.diagnostics.entries[crate::document::MAX_FDX_IMPORT_DIAGNOSTICS - 1],
            crate::document::FdxImportDiagnostic::DuplicateElementSettings {
                settings_ordinal: 32
            }
        );
    }

    #[test]
    fn normalizes_only_exact_extended_black_and_white_spellings() {
        assert_eq!(normalize_import_color("#000000000000"), None);
        assert_eq!(normalize_import_color("#fFfFfFfFfFfF"), None);
        for custom in [
            "#87878F8FE0E0",
            "#000000000001",
            "#FFFFFFFFFFFF0",
            "black",
            "#00000000000G",
        ] {
            assert_eq!(normalize_import_color(custom).as_deref(), Some(custom));
        }
    }

    #[test]
    fn retained_normalization_changes_only_winning_unambiguous_managed_colors() {
        fn setting_at(root: &RawElement, index: usize) -> &RawElement {
            match &root.children[index] {
                RawNode::Element(element) => element,
                _ => unreachable!("test settings are elements"),
            }
        }

        let mut root = raw_element(
            "FinalDraft",
            &[],
            vec![
                setting(
                    Some("General"),
                    vec![child(
                        "FontSpec",
                        &[
                            ("Color", "#000000000000"),
                            ("Background", "preserve-background"),
                            ("Vendor", "preserve-vendor"),
                        ],
                    )],
                ),
                setting(
                    Some("Custom"),
                    vec![child("FontSpec", &[("Color", "#87878F8FE0E0")])],
                ),
                setting(
                    Some("General"),
                    vec![child("FontSpec", &[("Color", "#FfFfFfFfFfFf")])],
                ),
                setting(
                    Some("Ambiguous"),
                    vec![
                        child("FontSpec", &[("Color", "#000000000000")]),
                        child("FontSpec", &[("Color", "#FfFfFfFfFfFf")]),
                    ],
                ),
            ],
        );
        let imported = read_element_settings(&root, None).unwrap_or_default();

        assert_eq!(
            imported.winners[0]
                .font
                .as_ref()
                .and_then(|font| font.color.as_deref()),
            None
        );
        assert_eq!(
            imported.winners[1]
                .font
                .as_ref()
                .and_then(|font| font.color.as_deref()),
            Some("#87878F8FE0E0")
        );
        assert!(normalize_winning_font_colors(&mut root, None, &imported).is_ok());

        let general_font = unique_child(setting_at(&root, 0), "FontSpec", None)
            .unwrap_or_else(|| unreachable!("General has one FontSpec"));
        assert_eq!(attribute(general_font, "Color"), None);
        assert_eq!(
            attribute(general_font, "Background"),
            Some("preserve-background")
        );
        assert_eq!(attribute(general_font, "Vendor"), Some("preserve-vendor"));

        let custom_font = unique_child(setting_at(&root, 1), "FontSpec", None)
            .unwrap_or_else(|| unreachable!("Custom has one FontSpec"));
        assert_eq!(attribute(custom_font, "Color"), Some("#87878F8FE0E0"));

        let duplicate_font = unique_child(setting_at(&root, 2), "FontSpec", None)
            .unwrap_or_else(|| unreachable!("duplicate General has one FontSpec"));
        assert_eq!(attribute(duplicate_font, "Color"), Some("#FfFfFfFfFfFf"));

        let ambiguous = setting_at(&root, 3);
        let ambiguous_colors = ambiguous
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element) if element.name.local_name == "FontSpec" => {
                    attribute(element, "Color")
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(ambiguous_colors, vec!["#000000000000", "#FfFfFfFfFfFf"]);
    }

    #[test]
    fn completes_source_first_use_and_packaged_base_definitions_in_order() {
        let direct_title = child("Paragraph", &[("Type", "Opaque Direct Title")]);
        let content_title = child("Paragraph", &[("Type", "Title Content First")]);
        let title_page = raw_element(
            "TitlePage",
            &[],
            vec![
                direct_title,
                RawNode::Element(raw_element(
                    "Header",
                    &[],
                    vec![child("Paragraph", &[("Type", "Opaque Header Title")])],
                )),
                RawNode::Element(raw_element("Content", &[], vec![content_title])),
                RawNode::Element(raw_element(
                    "Footer",
                    &[],
                    vec![child("Paragraph", &[("Type", "Opaque Footer Title")])],
                )),
                RawNode::Element(raw_element(
                    "Wrapper",
                    &[],
                    vec![child("Paragraph", &[("Type", "Opaque Wrapped Title")])],
                )),
            ],
        );
        let unsupported_dual = raw_element(
            "Paragraph",
            &[("Type", "General")],
            vec![RawNode::Element(raw_element(
                "DualDialogue",
                &[],
                vec![child("Paragraph", &[("Type", "Opaque Dual Type")])],
            ))],
        );
        let content = raw_element(
            "Content",
            &[],
            vec![
                child("Paragraph", &[("Type", "Action")]),
                child("Paragraph", &[("Type", "action")]),
                child("Paragraph", &[("Type", "Body Used")]),
                RawNode::Element(raw_element(
                    "Opaque",
                    &[],
                    vec![child("Paragraph", &[("Type", "Opaque Descendant")])],
                )),
                RawNode::Element(unsupported_dual),
            ],
        );
        let root = raw_element(
            "FinalDraft",
            &[],
            vec![
                setting(
                    Some("General"),
                    vec![
                        child(
                            "FontSpec",
                            &[("Font", "Source General"), ("Color", "#112233445566")],
                        ),
                        child("ParagraphSpec", &[("Alignment", "Center")]),
                    ],
                ),
                setting(
                    Some("Configured"),
                    vec![
                        child("FontSpec", &[("Color", "#000000000000")]),
                        child("Behavior", &[("ReturnKey", "Not Yet Defined")]),
                    ],
                ),
                RawNode::Element(title_page),
                RawNode::Element(content),
            ],
        );

        let used = collect_paragraph_used_types(&root, None).unwrap_or_default();
        assert_eq!(
            used.iter().map(ElementTypeKey::as_str).collect::<Vec<_>>(),
            vec!["Title Content First", "Action", "action", "Body Used"]
        );

        let imported = read_element_settings(&root, None).unwrap_or_default();
        let mut project = match Project::empty_projection() {
            Ok(project) => project,
            Err(_) => {
                assert!(
                    false,
                    "the transitional empty project should be constructible"
                );
                return;
            }
        };
        assert!(complete_document_definitions(&root, &imported, &mut project).is_ok());
        let definitions = project.element_definitions();
        assert_eq!(
            definitions
                .iter()
                .map(|definition| definition.key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "General",
                "Configured",
                "Title Content First",
                "Action",
                "action",
                "Body Used",
                "Scene Heading",
                "Character",
                "Parenthetical",
                "Dialogue",
                "Transition",
                "Shot",
                "Cast List",
            ]
        );

        let general = definitions.get_by_label("General");
        assert_eq!(
            general.map(|definition| definition.font.family.as_str()),
            Some("Source General")
        );
        assert_eq!(
            general.map(|definition| definition.font.color.as_deref()),
            Some(Some("#112233445566"))
        );
        assert_eq!(
            general.map(|definition| definition.paragraph.alignment),
            Some(ParagraphAlignment::Center)
        );
        let configured = definitions.get_by_label("Configured");
        assert_eq!(
            configured.map(|definition| definition.font.family.as_str()),
            Some("Source General")
        );
        assert_eq!(
            configured.map(|definition| definition.font.color.as_ref()),
            Some(None)
        );
        assert_eq!(
            configured
                .and_then(|definition| definition.behavior.return_key.as_ref())
                .map(ElementTypeKey::as_str),
            Some("Not Yet Defined")
        );
        assert_eq!(
            definitions.get_by_label("Action"),
            crate::builtin_screenplay::screenplay_defaults()
                .ok()
                .and_then(|defaults| defaults.definition("Action"))
                .as_ref()
        );
        assert!(definitions
            .get_by_label("action")
            .is_some_and(|definition| definition.capabilities()
                == scriptcraft_model::element_definition::SemanticCapabilities::default()));
    }

    #[test]
    fn registry_completion_fails_atomically_when_source_plus_bases_exceed_limit() {
        let mut children = Vec::with_capacity(MAX_DOCUMENT_ELEMENT_DEFINITIONS + 1);
        for index in 0..=MAX_DOCUMENT_ELEMENT_DEFINITIONS - 9 {
            children.push(setting(Some(&format!("Custom {index}")), Vec::new()));
        }
        let root = raw_element("FinalDraft", &[], children);
        let imported = read_element_settings(&root, None).unwrap_or_default();
        let mut project = match Project::empty_projection() {
            Ok(project) => project,
            Err(_) => {
                assert!(
                    false,
                    "the transitional empty project should be constructible"
                );
                return;
            }
        };
        let original = project.element_definitions().len();

        assert_eq!(
            complete_document_definitions(&root, &imported, &mut project),
            Err(FdxError::ResourceLimit)
        );
        assert_eq!(project.element_definitions().len(), original);
    }

    #[test]
    fn over_bound_type_fails_without_truncation() {
        let label = "x".repeat(scriptcraft_model::element_type::MAX_ELEMENT_TYPE_KEY_BYTES + 1);
        let root = raw_element("FinalDraft", &[], vec![setting(Some(&label), vec![])]);
        assert_eq!(
            read_element_settings(&root, None),
            Err(FdxError::ResourceLimit)
        );
    }

    fn imported_open_fixture() -> Option<crate::document::FdxDocument> {
        crate::reader::read_v6(include_bytes!(
            "../tests/fixtures/open_element_settings.fdx"
        ))
        .ok()
    }

    fn preserved_root(preservation: &PreservationState) -> Option<&PreservedElement> {
        preservation.sections.iter().find_map(|section| {
            section.children.iter().find_map(|node| match node {
                PreservedNode::Element(element)
                    if element.name.local_name == crate::constants::FDX_ROOT =>
                {
                    Some(element)
                }
                _ => None,
            })
        })
    }

    fn direct_settings(preservation: &PreservationState) -> Vec<&PreservedElement> {
        let Some(root) = preserved_root(preservation) else {
            return Vec::new();
        };
        let root_ns = root.name.namespace_uri.as_deref();
        root.children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element)
                    if element.name.local_name == "ElementSettings"
                        && element.name.namespace_uri.as_deref() == root_ns =>
                {
                    Some(element)
                }
                _ => None,
            })
            .collect()
    }

    fn setting_child<'a>(
        setting: &'a PreservedElement,
        name: &str,
    ) -> Option<&'a PreservedElement> {
        setting.children.iter().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == name => Some(element),
            _ => None,
        })
    }

    fn attr_value<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
        exact_unqualified_attribute(element, name)
    }

    fn raw_from_preserved(element: &PreservedElement) -> RawElement {
        fn raw_node(node: &PreservedNode) -> RawNode {
            match node {
                PreservedNode::Element(element) => RawNode::Element(raw_from_preserved(element)),
                PreservedNode::Text(text) => RawNode::Text(text.to_string()),
                PreservedNode::CData(text) => RawNode::CData(text.to_string()),
                PreservedNode::Comment(text) => RawNode::Comment(text.to_string()),
                PreservedNode::ProcessingInstruction { target, data } => {
                    RawNode::ProcessingInstruction {
                        target: target.to_string(),
                        data: data.to_string(),
                    }
                }
            }
        }
        RawElement {
            name: element.name.clone(),
            attributes: element
                .attributes
                .iter()
                .map(|attribute| RawAttribute {
                    name: attribute.name.clone(),
                    value: attribute.value.clone(),
                })
                .collect(),
            namespace_bindings: element.namespace_bindings.clone(),
            children: element.children.iter().map(raw_node).collect(),
        }
    }

    #[test]
    fn reader_anchors_import_winners_and_reconciliation_synthesizes_in_registry_order() {
        let Some(document) = imported_open_fixture() else {
            assert!(false, "the Phase 2 open-settings fixture should import");
            return;
        };
        let mut project = document.project;
        let Some(preservation) = project.preservation.as_ref() else {
            assert!(false, "the imported project should retain source");
            return;
        };
        let original_bytes = preservation.source.original_bytes.clone();
        let initial_settings = direct_settings(preservation);
        assert_eq!(initial_settings.len(), 6);
        let anchored = initial_settings
            .iter()
            .filter_map(|setting| {
                let anchor = setting.anchor.as_ref()?;
                preservation.anchors.iter().find_map(|source_anchor| {
                    (&source_anchor.id == anchor).then(|| match &source_anchor.target {
                        AnchorTarget::ElementDefinition(key) => Some(key.as_str()),
                        _ => None,
                    })?
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            anchored,
            vec!["Lyrics", "Notations", "action", "Action", "lyrics"]
        );
        assert_eq!(
            anchored.len(),
            preservation
                .anchors
                .iter()
                .filter(|anchor| { matches!(&anchor.target, AnchorTarget::ElementDefinition(_)) })
                .count()
        );

        let Some(lyrics_definition) = project.element_definitions().get_by_label("Lyrics") else {
            assert!(false, "the Lyrics definition should be imported");
            return;
        };
        let source_lyrics = preservation.element_definition_settings(&lyrics_definition.key);
        assert_eq!(
            source_lyrics.and_then(|setting| attr_value(setting, "vendor-setting")),
            None
        );
        assert_eq!(direct_settings(preservation).len(), 6);

        assert!(reconcile_project_element_settings(&mut project).is_ok());
        let Some(preservation) = project.preservation.as_ref() else {
            assert!(false, "the imported project should retain source");
            return;
        };
        let expected = project
            .element_definitions()
            .iter()
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>();
        let anchored = direct_settings(preservation)
            .iter()
            .filter_map(|setting| {
                let anchor = setting.anchor.as_ref()?;
                preservation.anchors.iter().find_map(|source_anchor| {
                    (&source_anchor.id == anchor).then(|| match &source_anchor.target {
                        AnchorTarget::ElementDefinition(key) => Some(key.as_str()),
                        _ => None,
                    })?
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(anchored, expected);
        assert_eq!(preservation.source.original_bytes, original_bytes);

        let lyrics = project
            .element_definitions()
            .get_by_label("Lyrics")
            .and_then(|definition| preservation.element_definition_settings(&definition.key));
        assert!(lyrics.is_some());
        let Some(lyrics) = lyrics else { return };
        assert_eq!(attr_value(lyrics, "Type"), Some("Lyrics"));
        let font = setting_child(lyrics, "FontSpec");
        assert_eq!(
            font.and_then(|font| attr_value(font, "Font")),
            Some("Avenir Next")
        );
        assert_eq!(
            font.and_then(|font| attr_value(font, "Color")),
            Some("#87878F8FE0E0")
        );
        let paragraph = setting_child(lyrics, "ParagraphSpec");
        assert_eq!(
            paragraph.and_then(|p| attr_value(p, "FirstIndent")),
            Some("-0.250000")
        );
        assert_eq!(
            paragraph.and_then(|p| attr_value(p, "LeftIndent")),
            Some("1.250000")
        );
        assert_eq!(
            paragraph.and_then(|p| attr_value(p, "Type")),
            Some("Lyrics")
        );

        let lyrics_family = direct_settings(preservation)
            .into_iter()
            .filter(|setting| attr_value(setting, "Type") == Some("Lyrics"))
            .collect::<Vec<_>>();
        assert_eq!(lyrics_family.len(), 2);
        assert_eq!(
            lyrics_family
                .iter()
                .filter(|setting| setting.anchor.is_some())
                .count(),
            1
        );
        assert!(lyrics_family.iter().any(|setting| setting.anchor.is_none()
            && setting_child(setting, "FontSpec").and_then(|font| attr_value(font, "Font"))
                == Some("Duplicate Must Not Win")));
    }

    #[test]
    fn ambiguous_recognized_children_fail_reconciliation_without_mutation() {
        let Some(document) = imported_open_fixture() else {
            assert!(false, "the Phase 2 open-settings fixture should import");
            return;
        };
        let Some(mut preservation) = document.project.preservation.as_ref().cloned() else {
            assert!(false, "the imported project should retain source");
            return;
        };
        let Some(root_index) = preservation.sections[0]
            .children
            .iter()
            .position(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == crate::constants::FDX_ROOT))
        else {
            assert!(false, "the source root should be retained");
            return;
        };
        let PreservedNode::Element(root) = &mut preservation.sections[0].children[root_index]
        else {
            assert!(false, "the located source root should be an element");
            return;
        };
        let Some(setting) = root.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(setting) if attr_value(setting, "Type") == Some("Lyrics") => {
                Some(setting)
            }
            _ => None,
        }) else {
            assert!(false, "the Lyrics winner should be retained");
            return;
        };
        let Some(font) = setting_child(setting, "FontSpec").cloned() else {
            assert!(false, "the winning FontSpec should be retained");
            return;
        };
        setting.children.push(PreservedNode::Element(font));
        let baseline = preservation.clone();
        let definitions = document_definitions(&document.project);
        assert!(
            reconcile_retained_project_element_settings(&mut preservation, &definitions).is_err()
        );
        assert_eq!(preservation, baseline);
    }

    fn document_definitions(project: &Project) -> DocumentElementDefinitions {
        DocumentElementDefinitions::new(project.element_definitions().iter().cloned())
            .unwrap_or_default()
    }

    #[test]
    fn obsolete_exact_label_family_is_removed_without_touching_case_variant() {
        let Some(mut document) = imported_open_fixture() else {
            assert!(false, "the Phase 2 open-settings fixture should import");
            return;
        };
        let definitions = document
            .project
            .element_definitions()
            .iter()
            .filter(|definition| definition.key.as_str() != "Lyrics")
            .cloned()
            .collect::<Vec<_>>();
        assert!(document
            .project
            .replace_element_definitions(definitions)
            .is_ok());
        assert!(reconcile_project_element_settings(&mut document.project).is_ok());
        let Some(preservation) = document.project.preservation.as_ref() else {
            assert!(false, "the imported project should retain source");
            return;
        };
        assert!(!direct_settings(preservation)
            .iter()
            .any(|setting| attr_value(setting, "Type") == Some("Lyrics")));
        assert!(direct_settings(preservation)
            .iter()
            .any(|setting| attr_value(setting, "Type") == Some("lyrics")));
    }

    #[test]
    fn whole_settings_replacement_validates_and_publishes_atomically() {
        let Some(document) = imported_open_fixture() else {
            assert!(false, "the Phase 2 open-settings fixture should import");
            return;
        };
        let Some(original_preservation) = document.project.preservation.as_ref() else {
            assert!(false, "the imported project should retain source");
            return;
        };
        let definitions = document_definitions(&document.project);
        let Some(root) = preserved_root(original_preservation) else {
            assert!(false, "the source root should be retained");
            return;
        };
        let non_settings_before = root
            .children
            .iter()
            .filter(|node| {
                !matches!(node, PreservedNode::Element(element) if is_preserved_fdx_element(element, "ElementSettings", root.name.namespace_uri.as_deref()))
            })
            .cloned()
            .collect::<Vec<_>>();
        let previous_definition_anchor_ids = original_preservation
            .anchors
            .iter()
            .filter(|anchor| matches!(&anchor.target, AnchorTarget::ElementDefinition(_)))
            .map(|anchor| anchor.id.clone())
            .collect::<BTreeSet<_>>();
        let mut invalid_nodes = definitions
            .iter()
            .map(|definition| make_owned_element_settings(definition, root))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default();
        if let Some(node) = invalid_nodes.first_mut() {
            assert!(set_unique_unqualified_attribute(node, "Type", "wrong-key").is_ok());
            node.source_type = Some("wrong-key".to_owned());
        }
        let mut preservation = original_preservation.clone();
        let baseline = preservation.clone();
        assert!(
            replace_project_element_settings(&mut preservation, &definitions, invalid_nodes,)
                .is_err()
        );
        assert_eq!(preservation, baseline);

        let nodes = definitions
            .iter()
            .map(|definition| make_owned_element_settings(definition, root))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default();
        assert!(replace_project_element_settings(&mut preservation, &definitions, nodes).is_ok());
        assert!(preservation
            .validate_element_definition_anchors(
                &definitions
                    .iter()
                    .map(|definition| definition.key.clone())
                    .collect()
            )
            .is_ok());
        assert_eq!(
            preservation
                .anchors
                .iter()
                .filter(|anchor| matches!(&anchor.target, AnchorTarget::ElementDefinition(_)))
                .count(),
            definitions.len()
        );
        let Some(replaced_root) = preserved_root(&preservation) else {
            assert!(false, "replacement should retain the source root");
            return;
        };
        let non_settings_after = replaced_root
            .children
            .iter()
            .filter(|node| {
                !matches!(node, PreservedNode::Element(element) if is_preserved_fdx_element(element, "ElementSettings", replaced_root.name.namespace_uri.as_deref()))
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(non_settings_after, non_settings_before);
        assert!(preservation.anchors.iter().all(|anchor| {
            !matches!(&anchor.target, AnchorTarget::ElementDefinition(_))
                || !previous_definition_anchor_ids.contains(&anchor.id)
        }));
    }

    #[test]
    fn serializer_formats_signed_inches_with_parser_exactness() {
        for raw in [-61_200, -1_800, -1, 0, 1, 1_800, 61_200] {
            let value = FixedPoint::from_raw(raw);
            let spelling = format_inches(value, true);
            assert!(spelling.is_ok());
            let Ok(spelling) = spelling else { return };
            assert_eq!(FixedPoint::parse_inches(&spelling), Some(value));
        }
    }

    #[test]
    fn complete_owned_serializer_round_trips_through_phase_two_field_parser() {
        let Some(document) = imported_open_fixture() else {
            assert!(false, "the Phase 2 open-settings fixture should import");
            return;
        };
        let Some(definition) = document
            .project
            .element_definitions()
            .get_by_label("Lyrics")
        else {
            assert!(false, "the Lyrics definition should be imported");
            return;
        };
        let Some(root) = document
            .project
            .preservation
            .as_ref()
            .and_then(preserved_root)
        else {
            assert!(false, "the FDX root should be retained");
            return;
        };
        let setting = make_owned_element_settings(definition, root);
        assert!(setting.is_ok());
        let Ok(setting) = setting else { return };
        let raw_setting = raw_from_preserved(&setting);
        let raw_root = raw_element("FinalDraft", &[], vec![RawNode::Element(raw_setting)]);
        let imported = read_element_settings(&raw_root, None);
        assert!(imported.is_ok());
        let Ok(imported) = imported else { return };
        let Some(draft) = imported.winners.first() else {
            assert!(
                false,
                "the serialized node should parse as a winning definition"
            );
            return;
        };
        let Some(font) = draft.font.as_ref() else {
            assert!(false, "the serialized FontSpec should parse");
            return;
        };
        assert_eq!(
            font.family.as_deref(),
            Some(definition.font.family.as_str())
        );
        assert_eq!(font.size, Some(definition.font.size));
        assert_eq!(font.bold, Some(definition.font.bold));
        assert_eq!(font.italic, Some(definition.font.italic));
        assert_eq!(font.underline, Some(definition.font.underline));
        assert_eq!(font.all_caps, Some(definition.font.all_caps));
        assert_eq!(font.hidden_text, Some(definition.font.hidden_text));
        assert_eq!(font.color.as_deref(), definition.font.color.as_deref());
        assert_eq!(font.adornment_style, Some(definition.font.adornment_style));
        assert_eq!(font.revision_id, Some(definition.font.revision_id));

        let Some(paragraph) = draft.paragraph.as_ref() else {
            assert!(false, "the serialized ParagraphSpec should parse");
            return;
        };
        assert_eq!(paragraph.alignment, Some(definition.paragraph.alignment));
        assert_eq!(
            paragraph.first_indent,
            Some(definition.paragraph.first_indent)
        );
        assert_eq!(
            paragraph.left_indent,
            Some(definition.paragraph.left_indent)
        );
        assert_eq!(
            paragraph.right_indent,
            Some(definition.paragraph.right_indent)
        );
        assert_eq!(
            paragraph.leading.as_deref(),
            Some(definition.paragraph.leading.as_str())
        );
        assert_eq!(
            paragraph.space_before,
            Some(definition.paragraph.space_before)
        );
        assert_eq!(paragraph.spacing, Some(definition.paragraph.spacing));
        assert_eq!(
            paragraph.starts_new_page,
            Some(definition.paragraph.starts_new_page)
        );

        let Some(behavior) = draft.behavior.as_ref() else {
            assert!(false, "the serialized Behavior should parse");
            return;
        };
        assert_eq!(
            behavior.paginate_as.as_deref(),
            definition.behavior.paginate_as.as_deref()
        );
        assert_eq!(
            behavior.return_key.as_deref(),
            definition
                .behavior
                .return_key
                .as_ref()
                .map(ElementTypeKey::as_str)
        );
        assert_eq!(
            behavior.tab_key.as_deref(),
            definition
                .behavior
                .tab_key
                .as_ref()
                .map(ElementTypeKey::as_str)
        );
        assert_eq!(
            behavior.shortcut.as_deref(),
            definition.behavior.shortcut.as_deref()
        );
        assert_eq!(
            behavior.win_shortcut.as_deref(),
            definition.behavior.win_shortcut.as_deref()
        );
        assert_eq!(
            draft.outline,
            Some(OutlineDraft {
                can_hide: Some(definition.outline.can_hide),
                level: Some(definition.outline.level),
            })
        );
    }

    #[test]
    fn in_place_reconciliation_preserves_unknown_attributes_children_and_comments() {
        let Some(mut document) = imported_open_fixture() else {
            assert!(false, "the Phase 2 open-settings fixture should import");
            return;
        };
        let Some(preservation) = document.project.preservation.as_mut() else {
            assert!(false, "the imported project should retain source");
            return;
        };
        let Some(root) = preservation.sections.iter_mut().find_map(|section| {
            section.children.iter_mut().find_map(|node| match node {
                PreservedNode::Element(root)
                    if root.name.local_name == crate::constants::FDX_ROOT =>
                {
                    Some(root)
                }
                _ => None,
            })
        }) else {
            assert!(false, "the source root should be retained");
            return;
        };
        let Some(setting) = root.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(setting) if attr_value(setting, "Type") == Some("Lyrics") => {
                Some(setting)
            }
            _ => None,
        }) else {
            assert!(false, "the Lyrics winner should be retained");
            return;
        };
        setting.attributes.push(PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "vendor-setting".to_owned(),
                qualified_name: Some("vendor-setting".to_owned()),
            },
            value: "retain-me".to_owned(),
        });
        setting
            .children
            .push(PreservedNode::Comment("settings note".into()));
        setting
            .children
            .push(PreservedNode::Element(PreservedElement {
                name: derived_name("VendorSettingsPayload", &setting.name),
                attributes: Vec::new(),
                namespace_bindings: Vec::new(),
                source_type: None,
                ownership: SourceOwnership::Opaque,
                dependency_confidence: setting.dependency_confidence,
                anchor: None,
                children: Vec::new().into(),
            }));

        let mut definitions = document
            .project
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let Some(lyrics) = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == "Lyrics")
        else {
            assert!(false, "the Lyrics definition should be imported");
            return;
        };
        lyrics.font.family = "Reconciled Family".to_owned();
        lyrics.font.color = None;
        lyrics.behavior.return_key = None;
        assert!(document
            .project
            .replace_element_definitions(definitions)
            .is_ok());
        assert!(reconcile_project_element_settings(&mut document.project).is_ok());
        let Some(preservation) = document.project.preservation.as_ref() else {
            assert!(false, "the imported project should retain source");
            return;
        };
        let lyrics = project_setting_by_type(preservation, "Lyrics");
        assert_eq!(
            lyrics.and_then(|setting| attr_value(setting, "vendor-setting")),
            Some("retain-me")
        );
        assert!(lyrics.is_some_and(|setting| {
            setting.children.iter().any(|node| {
            matches!(node, PreservedNode::Comment(comment) if comment.as_str() == "settings note")
        })
        }));
        assert!(lyrics.is_some_and(|setting| setting.children.iter().any(|node| {
            matches!(node, PreservedNode::Element(element) if element.name.local_name == "VendorSettingsPayload")
        })));
        assert_eq!(
            lyrics
                .and_then(|setting| setting_child(setting, "FontSpec"))
                .and_then(|font| attr_value(font, "Font")),
            Some("Reconciled Family")
        );
        assert_eq!(
            lyrics
                .and_then(|setting| setting_child(setting, "FontSpec"))
                .and_then(|font| attr_value(font, "Color")),
            None
        );
        assert_eq!(
            lyrics
                .and_then(|setting| setting_child(setting, "Behavior"))
                .and_then(|behavior| attr_value(behavior, "ReturnKey")),
            None
        );
    }

    fn project_setting_by_type<'a>(
        preservation: &'a PreservationState,
        key: &str,
    ) -> Option<&'a PreservedElement> {
        direct_settings(preservation)
            .into_iter()
            .find(|setting| attr_value(setting, "Type") == Some(key) && setting.anchor.is_some())
    }
}
