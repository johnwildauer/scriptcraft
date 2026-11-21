//! Retained-source projection, reconciliation, and replacement for FDX settings.

use crate::diagnostics::FdxError;
use crate::element_settings_import::ImportedElementSettings;
use crate::element_settings_serialization::{
    make_owned_element_settings, reconcile_owned_element_settings,
};
use scriptcraft_model::element_definitions::{
    DocumentElementDefinitions, MAX_DOCUMENT_ELEMENT_DEFINITIONS,
};
use scriptcraft_model::element_type::ElementTypeKey;
use scriptcraft_model::preservation::{
    AnchorRelation, AnchorTarget, PreservationState, PreservedElement, PreservedNode,
    PreservedSection, SourceAnchor, SourceAnchorId, SourceName, SourceOwnership, MAX_SOURCE_EVENTS,
};
use scriptcraft_model::Project;
use std::collections::{BTreeMap, BTreeSet};

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

/// Returns the winning direct-root settings nodes in registry order, detached
/// from the source document's anchor and ownership bookkeeping. Inert duplicate
/// settings, non-winning nodes, and other root children are never returned.
pub(crate) fn ordered_detached_element_settings_winners(
    preservation: &PreservationState,
    definitions: &DocumentElementDefinitions,
) -> Result<Vec<PreservedElement>, FdxError> {
    definitions
        .validate()
        .map_err(crate::diagnostics::map_model_error)?;
    let definition_keys = definitions
        .iter()
        .map(|definition| definition.key.clone())
        .collect::<BTreeSet<_>>();
    preservation
        .validate_element_definition_anchors(&definition_keys)
        .map_err(crate::diagnostics::map_preservation_error)?;

    let mut root = None;
    for section in &preservation.sections {
        for node in &section.children {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name != crate::constants::FDX_ROOT {
                continue;
            }
            if root.replace(element).is_some() {
                return Err(FdxError::InvalidValue);
            }
        }
    }
    let root = root.ok_or(FdxError::InvalidValue)?;
    let root_ns = root.name.namespace_uri.as_deref();

    let mut targets = BTreeMap::<SourceAnchorId, (&ElementTypeKey, AnchorRelation)>::new();
    for anchor in &preservation.anchors {
        if let AnchorTarget::ElementDefinition(key) = &anchor.target {
            if targets
                .insert(anchor.id.clone(), (key, anchor.relation))
                .is_some()
            {
                return Err(FdxError::InvalidValue);
            }
        }
    }

    let mut winners = BTreeMap::<ElementTypeKey, PreservedElement>::new();
    for child in &root.children {
        let PreservedNode::Element(setting) = child else {
            continue;
        };
        if !is_preserved_fdx_element(setting, "ElementSettings", root_ns) {
            continue;
        }
        let Some(anchor_id) = setting.anchor.as_ref() else {
            continue;
        };
        let (key, relation) = targets.get(anchor_id).ok_or(FdxError::InvalidValue)?;
        if *relation != AnchorRelation::Represents
            || setting.ownership != SourceOwnership::Opaque
            || setting.source_type.as_deref() != Some(key.as_str())
            || exact_unqualified_attribute(setting, "Type") != Some(key.as_str())
            || !definition_keys.contains(*key)
        {
            return Err(FdxError::InvalidValue);
        }
        let mut detached = setting.clone();
        detach_target_source_authority(&mut detached);
        if winners.insert((**key).clone(), detached).is_some() {
            return Err(FdxError::InvalidValue);
        }
    }

    let mut ordered = Vec::with_capacity(definitions.len());
    for definition in definitions.iter() {
        let winner = winners
            .remove(&definition.key)
            .ok_or(FdxError::InvalidValue)?;
        ordered.push(winner);
    }
    if !winners.is_empty() {
        return Err(FdxError::InvalidValue);
    }
    Ok(ordered)
}

fn detach_target_source_authority(element: &mut PreservedElement) {
    element.anchor = None;
    element.ownership = SourceOwnership::Opaque;
    element.dependency_confidence = scriptcraft_model::preservation::DependencyConfidence::Unknown;
    for child in &mut element.children {
        if let PreservedNode::Element(child) = child {
            detach_target_source_authority(child);
        }
    }
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
        if projected != node || !is_detached_opaque_subtree(&node) {
            return Err(FdxError::InvalidValue);
        }
        validated_nodes.push(node);
    }

    // Replacing an equal retained settings projection would allocate fresh
    // source anchors and make an otherwise equal session candidate appear
    // changed. Preserve the existing source graph when current winners match
    // after projecting definition-owned fields; changed opaque settings and
    // definition sets still take the normal staged replacement path below.
    let definition_keys = definitions
        .iter()
        .map(|definition| definition.key.clone())
        .collect::<BTreeSet<_>>();
    let current_definition_keys = preservation
        .anchors
        .iter()
        .filter_map(|anchor| match &anchor.target {
            AnchorTarget::ElementDefinition(key) => Some(key.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if current_definition_keys == definition_keys
        && preservation.validate().is_ok()
        && ordered_detached_element_settings_winners(preservation, definitions).is_ok_and(
            |mut current_nodes| {
                current_nodes
                    .iter_mut()
                    .zip(validated_nodes.iter())
                    .zip(definitions.iter())
                    .all(|((current, replacement), definition)| {
                        let namespace = current.name.namespace_uri.clone();
                        reconcile_owned_element_settings(current, definition, namespace.as_deref())
                            .is_ok()
                            && current == replacement
                    })
            },
        )
    {
        return Ok(());
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
    let mut next_anchor_ordinal = 0usize;

    with_direct_root_mut(&mut staged.sections, |root| {
        let root_ns = root.name.namespace_uri.as_deref();
        let root_namespaces = namespace_scope(&BTreeMap::new(), &root.namespace_bindings)?;
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
            if node.name.namespace_uri.as_deref() != root_ns
                || !validate_detached_node_namespaces(&node, &root_namespaces)
            {
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
    // ElementSettings contributes to the retained source identity-use and text
    // indexes. Rebuild them on the staged tree before the atomic publication.
    staged.rebuild_indexes();
    *preservation = staged;
    Ok(())
}

const XML_NAMESPACE_URI: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_NAMESPACE_URI: &str = "http://www.w3.org/2000/xmlns/";

fn is_detached_opaque_subtree(element: &PreservedElement) -> bool {
    element.anchor.is_none()
        && element.ownership == SourceOwnership::Opaque
        && element.children.iter().all(|node| match node {
            PreservedNode::Element(child) => is_detached_opaque_subtree(child),
            _ => true,
        })
}

fn namespace_scope(
    inherited: &BTreeMap<String, Option<String>>,
    declarations: &[scriptcraft_model::preservation::NamespaceBinding],
) -> Result<BTreeMap<String, Option<String>>, FdxError> {
    let mut scope = inherited.clone();
    let mut declared = BTreeSet::new();
    for binding in declarations {
        if !declared.insert(binding.prefix.as_str())
            || binding.prefix == "xmlns"
            || binding.namespace_uri == XMLNS_NAMESPACE_URI
            || (binding.prefix == "xml" && binding.namespace_uri != XML_NAMESPACE_URI)
            || (binding.prefix != "xml" && binding.namespace_uri == XML_NAMESPACE_URI)
            || (!binding.prefix.is_empty() && binding.namespace_uri.is_empty())
        {
            return Err(FdxError::InvalidValue);
        }
        scope.insert(
            binding.prefix.clone(),
            (!binding.namespace_uri.is_empty()).then(|| binding.namespace_uri.clone()),
        );
    }
    scope.insert("xml".to_owned(), Some(XML_NAMESPACE_URI.to_owned()));
    Ok(scope)
}

fn source_name_resolves(
    name: &SourceName,
    is_attribute: bool,
    scope: &BTreeMap<String, Option<String>>,
) -> bool {
    let Some(qualified_name) = name.qualified_name.as_deref() else {
        return false;
    };
    let mut parts = qualified_name.split(':');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    if parts.next().is_some() {
        return false;
    }
    let (prefix, local_name) = match second {
        Some(local_name) if !first.is_empty() && !local_name.is_empty() => {
            (Some(first), local_name)
        }
        Some(_) => return false,
        None => (None, first),
    };
    if local_name != name.local_name
        || local_name.is_empty()
        || name.local_name == "xmlns"
        || name.namespace_uri.as_deref() == Some(XMLNS_NAMESPACE_URI)
    {
        return false;
    }
    let resolved = if let Some(prefix) = prefix {
        if prefix == "xmlns" {
            return false;
        }
        match scope.get(prefix) {
            Some(Some(namespace_uri)) => Some(namespace_uri.clone()),
            _ => return false,
        }
    } else if is_attribute {
        None
    } else {
        scope.get("").cloned().flatten()
    };
    resolved == name.namespace_uri
}

fn validate_detached_node_namespaces(
    element: &PreservedElement,
    inherited: &BTreeMap<String, Option<String>>,
) -> bool {
    let Ok(scope) = namespace_scope(inherited, &element.namespace_bindings) else {
        return false;
    };
    if !source_name_resolves(&element.name, false, &scope) {
        return false;
    }
    let mut attribute_names = BTreeSet::new();
    for attribute in &element.attributes {
        if !source_name_resolves(&attribute.name, true, &scope)
            || !attribute_names.insert((
                attribute.name.namespace_uri.as_deref(),
                attribute.name.local_name.as_str(),
            ))
        {
            return false;
        }
    }
    element.children.iter().all(|node| match node {
        PreservedNode::Element(child) => validate_detached_node_namespaces(child, &scope),
        _ => true,
    })
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
