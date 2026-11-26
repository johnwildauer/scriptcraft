//! Detached FDX projection for inline Dialogue alternatives and AltCollection.

use crate::diagnostics::FdxError;
use crate::tree::{new_lineage_element, plain_source_name};
use scriptcraft_model::formatted_text::{complete_alt_run_projection, dialogue_default_run_attributes};
use scriptcraft_model::preservation::{
    PreservedAttribute, PreservedElement, PreservedNode, PreservedSection,
    SourceOwnership,
};
use scriptcraft_model::{DialogueAltId, DialogueAlternatives, Project, ScreenplayElement, StableId};
use std::collections::{BTreeMap, BTreeSet};

type OwnerKey = (String, String);

pub(crate) fn complete_alt_output_runs(
    definition: &scriptcraft_model::ElementDefinition,
    text: &str,
    runs: &[scriptcraft_model::TextRun],
) -> Result<Vec<scriptcraft_model::TextRun>, FdxError> {
    let mut complete = complete_alt_run_projection(definition, text, runs)
        .map_err(|_| FdxError::InvalidValue)?;
    let empty_style = dialogue_default_run_attributes(definition).into_iter()
        .find(|attribute| attribute.name == "Style" && attribute.value.is_empty());
    if let Some(style) = empty_style {
        for run in &mut complete {
            if run.attributes.iter().all(|attribute| attribute.name != "Style") {
                let index = run.attributes.iter().position(|attribute| attribute.name == "Size")
                    .map_or(run.attributes.len(), |index| index + 1);
                run.attributes.insert(index, style.clone());
            }
        }
    }
    Ok(complete)
}

fn is_dialogue(project: &Project, semantic: &ScreenplayElement) -> bool {
    project.element_definitions().get(&semantic.element_type())
        .is_some_and(|definition| definition.capabilities().dialogue)
}

fn attr<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    let mut values = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    });
    let value = values.next()?.value.as_str();
    values.next().is_none().then_some(value)
}

fn direct_child_indices(parent: &PreservedElement, name: &str) -> Vec<usize> {
    parent.children.iter().enumerate().filter_map(|(index, node)| match node {
        PreservedNode::Element(child) if child.name.local_name == name
            && child.name.namespace_uri == parent.name.namespace_uri => Some(index),
        _ => None,
    }).collect()
}

fn has_direct_local_name(parent: &PreservedElement, name: &str) -> bool {
    parent.children.iter().any(|node| matches!(node,
        PreservedNode::Element(child) if child.name.local_name == name))
}

fn generated_inline_alts(parent: &PreservedElement, alternatives: &DialogueAlternatives) -> PreservedElement {
    let mut alts = new_lineage_element(&parent.name, "Alts", vec![("Current".into(), alternatives.current.to_string())]);
    for alternative in &alternatives.inactive {
        let mut child = new_lineage_element(&alts.name, "AltId", Vec::new());
        child.children.push(PreservedNode::Text(alternative.id.to_string().into()));
        alts.children.push(PreservedNode::Element(child));
    }
    alts
}

fn set_inline_alts(paragraph: &mut PreservedElement, alternatives: &DialogueAlternatives) -> Result<(), FdxError> {
    let indices = direct_child_indices(paragraph, "Alts");
    if indices.len() > 1 { return Ok(()); }
    let mut generated = generated_inline_alts(paragraph, alternatives);
    if let Some(index) = indices.first().copied() {
        let Some(PreservedNode::Element(existing)) = paragraph.children.get_mut(index) else {
            return Err(FdxError::InvalidValue);
        };
        let current_indexes = existing.attributes.iter().enumerate().filter_map(|(attribute_index, attribute)| {
            (attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Current")
                .then_some(attribute_index)
        }).collect::<Vec<_>>();
        match current_indexes.as_slice() {
            [current_index] => existing.attributes[*current_index].value = alternatives.current.to_string(),
            [] => existing.attributes.push(PreservedAttribute {
                name: plain_source_name("Current"), value: alternatives.current.to_string(),
            }),
            _ => return Err(FdxError::AmbiguousIdentity),
        }
        let generated_ids = std::mem::take(&mut generated.children);
        let mut children = existing.children.iter().filter(|node| !matches!(node,
            PreservedNode::Element(child) if child.name.local_name == "AltId"
                && child.name.namespace_uri == existing.name.namespace_uri))
            .cloned().collect::<Vec<_>>();
        children.extend(generated_ids);
        existing.children = children.into();
    } else {
        paragraph.children.push(PreservedNode::Element(generated));
    }
    Ok(())
}

fn durable_id(paragraph: &PreservedElement) -> Option<String> {
    let mut values = paragraph.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id")
    });
    let value = values.next()?.value.as_str();
    values.next().is_none().then(|| value.to_owned())
}

fn gather_managed_paragraphs<'a>(element: &'a PreservedElement, output: &mut Vec<(&'a StableId, &'a PreservedElement)>) {
    if element.name.local_name == "Paragraph" {
        if let SourceOwnership::Managed(id) = &element.ownership { output.push((id, element)); }
    }
    for child in &element.children {
        if let PreservedNode::Element(child) = child { gather_managed_paragraphs(child, output); }
    }
}

fn first_root_mut(sections: &mut [PreservedSection]) -> Option<&mut PreservedElement> {
    sections.iter_mut().flat_map(|section| &mut section.children).find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => Some(element),
        _ => None,
    })
}

fn first_root_collection(root: &PreservedElement) -> Option<usize> {
    direct_child_indices(root, "AltCollection").first().copied()
}

fn source_alt_key(alt: &PreservedElement) -> Option<OwnerKey> {
    let id = DialogueAltId::parse_decimal(attr(alt, "Id")?)?.to_string();
    let paragraphs = direct_child_indices(alt, "Paragraph");
    let [index] = paragraphs.as_slice() else { return None };
    let PreservedNode::Element(paragraph) = &alt.children[*index] else { return None };
    Some((id, durable_id(paragraph)?))
}

fn preserve_alt_shell(existing: &PreservedElement, generated: PreservedElement) -> PreservedElement {
    let paragraphs = direct_child_indices(&generated, "Paragraph");
    let [generated_index] = paragraphs.as_slice() else { return generated };
    let Some(PreservedNode::Element(generated_paragraph)) = generated.children.get(*generated_index) else {
        return generated;
    };
    let mut result = existing.clone();
    let existing_paragraphs = direct_child_indices(&result, "Paragraph");
    let [existing_index] = existing_paragraphs.as_slice() else { return generated };
    result.children[*existing_index] = PreservedNode::Element(generated_paragraph.clone());
    result
}

fn generated_alt(
    collection: &PreservedElement,
    id: DialogueAltId,
    owner_id: &str,
    semantic: &ScreenplayElement,
    project: &Project,
) -> Result<PreservedElement, FdxError> {
    let definition = project.element_definitions().get(&semantic.element_type())
        .filter(|definition| definition.capabilities().dialogue)
        .or_else(|| project.element_definitions().iter().find(|definition| definition.key.as_str() == "Dialogue"))
        .ok_or(FdxError::InvalidValue)?;
    let mut alt = new_lineage_element(&collection.name, "Alt", vec![("Id".into(), id.to_string())]);
    let mut paragraph = new_lineage_element(&alt.name, "Paragraph", vec![
        ("Alignment".into(), "Left".into()),
        ("FirstIndent".into(), "0.00".into()),
        ("Leading".into(), "Regular".into()),
        ("LeftIndent".into(), "2.50".into()),
        ("OutlineLevel".into(), "1".into()),
        ("RightIndent".into(), "-1.11".into()),
        ("SpaceBefore".into(), "0".into()),
        ("Spacing".into(), "1".into()),
        ("StartsNewPage".into(), "No".into()),
        ("id".into(), owner_id.to_owned()),
    ]);
    let alternative = semantic.dialogue_alternatives.as_ref().and_then(|state| state.variant(id))
        .ok_or(FdxError::InvalidValue)?;
    if !alternative.text.is_empty() {
        let runs = complete_alt_output_runs(definition, &alternative.text, &alternative.runs)?;
        for run in runs {
            let mut text = new_lineage_element(&paragraph.name, "Text", Vec::new());
            text.attributes = run.attributes.into_iter().map(|attribute| PreservedAttribute {
                name: plain_source_name(&attribute.name), value: attribute.value,
            }).collect();
            if !run.text.is_empty() { text.children.push(PreservedNode::Text(run.text.into())); }
            paragraph.children.push(PreservedNode::Element(text));
        }
    }
    alt.children.push(PreservedNode::Element(paragraph));
    Ok(alt)
}

fn baseline_alt<'a>(original: &'a Project, key: &OwnerKey) -> Option<&'a scriptcraft_model::DialogueAlternative> {
    let id = DialogueAltId::parse_decimal(&key.0)?;
    original.elements.iter().find_map(|owner| {
        let semantic = owner.as_element();
        (semantic.source_provenance.as_ref()?.foreign_id.as_deref()? == key.1)
            .then(|| semantic.dialogue_alternatives.as_ref()?.variant(id))?
    })
}

fn make_desired(
    project: &Project,
    ids: &BTreeMap<StableId, String>,
    dialogue_only: bool,
) -> BTreeMap<OwnerKey, StableId> {
    let mut desired = BTreeMap::new();
    for owner in &project.elements {
        let semantic = owner.as_element();
        if dialogue_only && !is_dialogue(project, semantic) { continue; }
        let Some(owner_id) = ids.get(&semantic.id) else { continue };
        if let Some(alternatives) = &semantic.dialogue_alternatives {
            for alternative in &alternatives.inactive {
                desired.insert((alternative.id.to_string(), owner_id.clone()), semantic.id.clone());
            }
        }
    }
    desired
}

fn make_source_desired(
    project: &Project,
    original: &Project,
    ids: &BTreeMap<StableId, String>,
) -> BTreeMap<OwnerKey, StableId> {
    make_desired(project, ids, false).into_iter().filter(|(_, owner_id)| {
        let Some(owner) = project.elements.iter().find(|owner| &owner.id == owner_id) else { return false };
        is_dialogue(project, owner.as_element())
            || !(owner.dialogue_alternatives.is_some()
                && original.elements.iter().any(|baseline| baseline.id == *owner_id
                    && baseline.dialogue_alternatives.is_some()))
    }).collect()
}

/// Reconciles inline state and AltCollection on detached source-backed output.
pub(crate) fn project_source_backed(
    sections: &mut [PreservedSection],
    project: &Project,
    original: &Project,
) -> Result<(), FdxError> {
    let root = first_root_mut(sections).ok_or(FdxError::InvalidRoot)?;
    let content_index = direct_child_indices(root, "Content").first().copied().ok_or(FdxError::MissingContent)?;
    let PreservedNode::Element(content) = &mut root.children[content_index] else { return Err(FdxError::InvalidValue) };
    let mut managed = Vec::new();
    gather_managed_paragraphs(content, &mut managed);
    let mut ids = BTreeMap::new();
    for (owner_id, paragraph) in managed {
        if let Some(durable) = durable_id(paragraph) { ids.insert(owner_id.clone(), durable); }
    }
    for owner in &project.elements {
        let semantic = owner.as_element();
        let dialogue_capable = is_dialogue(project, semantic);
        let was_supported = original.elements.iter().any(|baseline| baseline.id == semantic.id
            && baseline.dialogue_alternatives.is_some());
        if !dialogue_capable && was_supported && semantic.dialogue_alternatives.is_some() { continue; }
        let Some(_durable) = ids.get(&semantic.id) else {
            if semantic.dialogue_alternatives.as_ref().is_some_and(|state| !state.inactive.is_empty()) {
                return Err(FdxError::AmbiguousIdentity);
            }
            continue;
        };
        let Some(paragraph) = find_managed_mut(content, &semantic.id) else { continue };
        match &semantic.dialogue_alternatives {
            Some(alternatives) => {
                let existed_before = original.elements.iter().any(|baseline| baseline.id == semantic.id);
                if existed_before && !was_supported && has_direct_local_name(paragraph, "Alts") {
                    return Err(FdxError::AmbiguousIdentity);
                }
                if !has_direct_local_name(paragraph, "Alts") || was_supported || !existed_before {
                    set_inline_alts(paragraph, alternatives)?;
                }
            }
            None => {
                if was_supported {
                    for index in direct_child_indices(paragraph, "Alts").into_iter().rev() {
                        paragraph.children.remove(index);
                    }
                }
            }
        }
    }
    let desired = make_source_desired(project, original, &ids);
    let original_supported = original.elements.iter().filter_map(|owner| {
        let semantic = owner.as_element();
        let owner_id = semantic.source_provenance.as_ref()?.foreign_id.as_ref()?;
        let alternatives = semantic.dialogue_alternatives.as_ref()?;
        Some(alternatives.inactive.iter().map(move |alternative| (alternative.id.to_string(), owner_id.clone())))
    }).flatten().collect::<BTreeSet<_>>();
    let collection_index = first_root_collection(root);
    if collection_index.is_none() && desired.is_empty() { return Ok(()); }
    if collection_index.is_none() {
        let mut collection = new_lineage_element(&root.name, "AltCollection", Vec::new());
        reconcile_collection(&mut collection, &desired, &original_supported, project, original, &ids)?;
        let insertion = root.children.iter().position(|node| matches!(node,
            PreservedNode::Element(element) if matches!(element.name.local_name.as_str(), "ListItems" | "DisplayBoards" | "ScriptNotes")))
            .unwrap_or(root.children.len());
        root.children.insert(insertion, PreservedNode::Element(collection));
        return Ok(());
    }
    let index = collection_index.ok_or(FdxError::InvalidValue)?;
    let PreservedNode::Element(collection) = &mut root.children[index] else { return Err(FdxError::InvalidValue) };
    reconcile_collection(collection, &desired, &original_supported, project, original, &ids)
}

fn find_managed_mut<'a>(element: &'a mut PreservedElement, id: &StableId) -> Option<&'a mut PreservedElement> {
    if element.name.local_name == "Paragraph"
        && matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id)
    { return Some(element); }
    for child in &mut element.children {
        if let PreservedNode::Element(child) = child {
            if let Some(found) = find_managed_mut(child, id) { return Some(found); }
        }
    }
    None
}

fn reconcile_collection(
    collection: &mut PreservedElement,
    desired: &BTreeMap<OwnerKey, StableId>,
    original_supported: &BTreeSet<OwnerKey>,
    project: &Project,
    original: &Project,
    ids: &BTreeMap<StableId, String>,
) -> Result<(), FdxError> {
    let namespace = collection.name.namespace_uri.clone();
    let mut current_semantic_by_id = BTreeMap::new();
    for owner in &project.elements { current_semantic_by_id.insert(owner.id.clone(), owner.as_element()); }
    let mut desired_keys = desired.keys().cloned().collect::<BTreeSet<_>>();
    let mut existing_by_key = BTreeMap::<OwnerKey, Vec<usize>>::new();
    for (index, node) in collection.children.iter().enumerate() {
        let PreservedNode::Element(alt) = node else { continue };
        if alt.name.local_name == "Alt" && alt.name.namespace_uri == namespace {
            if let Some(key) = source_alt_key(alt) { existing_by_key.entry(key).or_default().push(index); }
        }
    }
    let mut removals = BTreeSet::new();
    let mut replacements = BTreeMap::<usize, PreservedElement>::new();
    for (key, positions) in &existing_by_key {
        let supported_before = original_supported.contains(key);
        let owner_allows_managed_removal = project.elements.iter().any(|owner| {
            if ids.get(&owner.id) != Some(&key.1) { return false; }
            is_dialogue(project, owner.as_element())
                || (owner.dialogue_alternatives.is_none()
                    && original.elements.iter().any(|baseline| baseline.id == owner.id
                        && baseline.dialogue_alternatives.is_some()))
        });
        if supported_before && owner_allows_managed_removal && !desired.contains_key(key) {
            removals.extend(positions.iter().copied());
        }
        if let Some(owner_id) = desired.get(key) {
            if !supported_before {
                // A same-key entry not represented by the original semantic
                // import is opaque source, not an editable managed copy.
                return Err(FdxError::AmbiguousIdentity);
            }
            let semantic = current_semantic_by_id.get(owner_id).copied().ok_or(FdxError::InvalidValue)?;
            let baseline_unchanged = baseline_alt(original, key).is_some_and(|baseline| {
                semantic.dialogue_alternatives.as_ref().and_then(|state| state.variant(
                    DialogueAltId::parse_decimal(&key.0).unwrap_or(DialogueAltId::new(u64::MAX))
                )).is_some_and(|current| current == baseline)
            });
            if !baseline_unchanged {
                let Some(index) = positions.first().copied() else { continue };
                let alt_id = DialogueAltId::parse_decimal(&key.0).ok_or(FdxError::InvalidValue)?;
                let generated = generated_alt(collection, alt_id, &key.1, semantic, project)?;
                let PreservedNode::Element(existing) = &collection.children[index] else { return Err(FdxError::InvalidValue) };
                replacements.insert(index, preserve_alt_shell(existing, generated));
                for duplicate in positions.iter().skip(1) { removals.insert(*duplicate); }
            }
            desired_keys.remove(key);
        }
    }
    let old_children = std::mem::take(&mut collection.children);
    let mut projected = Vec::with_capacity(old_children.len());
    for (index, node) in old_children.into_iter().enumerate() {
        if removals.contains(&index) { continue; }
        if let Some(replacement) = replacements.get(&index) {
            projected.push(PreservedNode::Element(replacement.clone()));
        } else {
            projected.push(node);
        }
    }
    collection.children = projected.into();
    let mut additions = desired_keys.into_iter().collect::<Vec<_>>();
    additions.sort_by(|left, right| {
        DialogueAltId::parse_decimal(&left.0).cmp(&DialogueAltId::parse_decimal(&right.0))
            .then_with(|| left.1.cmp(&right.1))
    });
    let mut ordered_managed_keys = original_supported.clone();
    ordered_managed_keys.extend(desired.keys().cloned());
    for key in additions {
        let owner_id = desired.get(&key).ok_or(FdxError::InvalidValue)?;
        let semantic = current_semantic_by_id.get(owner_id).copied().ok_or(FdxError::InvalidValue)?;
        let id = DialogueAltId::parse_decimal(&key.0).ok_or(FdxError::InvalidValue)?;
        let node = generated_alt(collection, id, &key.1, semantic, project)?;
        let insertion = insertion_index(collection, id, &ordered_managed_keys);
        collection.children.insert(insertion, PreservedNode::Element(node));
        ordered_managed_keys.insert(key);
    }
    Ok(())
}

fn insertion_index(collection: &PreservedElement, id: DialogueAltId, managed: &BTreeSet<OwnerKey>) -> usize {
    let mut last_managed = None;
    for (index, node) in collection.children.iter().enumerate() {
        let PreservedNode::Element(alt) = node else { continue };
        if alt.name.local_name != "Alt" || alt.name.namespace_uri != collection.name.namespace_uri { continue; }
        if let Some(key) = source_alt_key(alt).filter(|key| managed.contains(key)) {
            let Some(existing) = DialogueAltId::parse_decimal(&key.0) else { continue };
            if existing > id { return index; }
            last_managed = Some(index + 1);
        }
    }
    last_managed.unwrap_or(collection.children.len())
}

/// Appends a generated root collection before generated Beat and ScriptNotes
/// regions, using durable IDs already assigned to the Content owner nodes.
pub(crate) fn append_generated_collection(
    root: &mut PreservedElement,
    project: &Project,
    paragraph_ids: &BTreeMap<StableId, String>,
) -> Result<(), FdxError> {
    let desired = make_desired(project, paragraph_ids, false);
    if desired.is_empty() { return Ok(()); }
    let mut collection = new_lineage_element(&root.name, "AltCollection", Vec::new());
    let mut ordered = desired.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|(left, _), (right, _)| {
        DialogueAltId::parse_decimal(&left.0).cmp(&DialogueAltId::parse_decimal(&right.0))
            .then_with(|| left.1.cmp(&right.1))
    });
    for (key, owner_id) in ordered {
        let semantic = project.elements.iter().find(|owner| owner.id == owner_id).ok_or(FdxError::InvalidValue)?;
        let id = DialogueAltId::parse_decimal(&key.0).ok_or(FdxError::InvalidValue)?;
        let alt = generated_alt(&collection, id, &key.1, semantic.as_element(), project)?;
        collection.children.push(PreservedNode::Element(alt));
    }
    let insertion = root.children.iter().position(|node| matches!(node,
        PreservedNode::Element(element) if matches!(element.name.local_name.as_str(), "ListItems" | "DisplayBoards" | "ScriptNotes")))
        .unwrap_or(root.children.len());
    root.children.insert(insertion, PreservedNode::Element(collection));
    Ok(())
}

/// Attaches semantic inline IDs to a generated Dialogue paragraph.
pub(crate) fn attach_generated_inline(
    paragraph: &mut PreservedElement,
    semantic: &ScreenplayElement,
) -> Result<(), FdxError> {
    if let Some(alternatives) = &semantic.dialogue_alternatives {
        set_inline_alts(paragraph, alternatives)?;
    }
    Ok(())
}

/// Extracts the already allocated paragraph ID from one generated Paragraph.
pub(crate) fn generated_paragraph_id(paragraph: &PreservedElement) -> Result<String, FdxError> {
    durable_id(paragraph).ok_or(FdxError::AmbiguousIdentity)
}
