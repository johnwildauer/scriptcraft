//! Preservation-safe derived SceneProperties and scene-cast arc projection.

use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
use crate::preservation::{
    is_user_paragraph_data_lookalike, is_user_paragraph_data_name, ForeignIdDomain,
    ForeignIdentity, PreservationError, PreservationState, PreservedAttribute, PreservedElement,
    PreservedNode, SourceName, SourceOwnership,
};
use crate::{ModelError, Project, Role, ScreenplayElement, StableId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub(crate) struct ReconcileOutcome {
    pub changed: bool,
    pub deferred_scenes: BTreeSet<String>,
    pub changed_scenes: BTreeSet<String>,
    pub scene_candidates: u64,
    pub preservation_state_clones: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneMetadataError {
    Model(ModelError),
    Preservation(PreservationError),
    UnsupportedSceneMetadata,
    ParagraphIdentityEntropyUnavailable,
    ParagraphIdentityCollisionLimit,
}

impl From<ModelError> for SceneMetadataError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

pub(crate) fn reconcile_project_candidate_with_scope(
    project: &mut Project,
    requested_scenes: Option<&BTreeSet<String>>,
) -> Result<ReconcileOutcome, SceneMetadataError> {
    if project.preservation.is_none() {
        // New projects acquire their source tree in the FDX writer, where the
        // same projection is emitted directly from semantic content.
        return Ok(ReconcileOutcome::default());
    }
    let scenes = scene_casts_with_scope(&project.elements, requested_scenes);
    let state = project
        .preservation
        .as_mut()
        .ok_or(SceneMetadataError::UnsupportedSceneMetadata)?;
    state.ensure_managed_paragraph_index();
    state.ensure_source_identity_use_index();
    let removable = safe_paragraph_removals(state);
    let mut reserved_ids = state.indexed_source_id_values();
    reserved_ids.extend(
        state
            .identities
            .iter()
            .map(|identity| identity.value.clone()),
    );
    reserved_ids.extend(
        state
            .board_graph
            .entities
            .iter()
            .map(|entity| entity.identity.value.clone()),
    );
    reserved_ids.extend(state.references.iter().flat_map(|reference| {
        [
            reference.source.value.clone(),
            reference.target.value.clone(),
        ]
    }));
    reserved_ids.extend(state.board_graph.references.iter().flat_map(|reference| {
        [
            reference.source.value.clone(),
            reference.target.value.clone(),
        ]
    }));
    let mut paragraph_ids = ParagraphIdAllocator::new(reserved_ids);
    let known_paragraph_ids = state
        .identities
        .iter()
        .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
        .map(|identity| identity.value.clone())
        .collect::<BTreeSet<_>>();
    let mut deferred_scenes = BTreeSet::new();
    let mut changed_scenes = BTreeSet::new();
    let mut changed = false;
    let mut scene_candidates = 0u64;
    for (heading_id, names) in scenes {
        let Some(original_heading) = state
            .managed_element(&heading_id)
            .map_err(SceneMetadataError::Preservation)?
        else {
            return Err(SceneMetadataError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ));
        };
        let mut scene_candidate = original_heading.clone();
        scene_candidates = scene_candidates.saturating_add(1);
        let (heading_depth, inherited_space_preserve) = state
            .managed_element_context(&heading_id)
            .map_err(SceneMetadataError::Preservation)?
            .ok_or(SceneMetadataError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ))?;
        match reconcile_heading(
            &mut scene_candidate,
            heading_depth,
            inherited_space_preserve,
            &names,
            &removable,
            &known_paragraph_ids,
            &mut paragraph_ids,
        ) {
            Ok((removed, allocated)) => {
                let scene_changed = scene_candidate != *original_heading;
                if scene_changed {
                    let target = state
                        .managed_element_mut_indexed(&heading_id)
                        .map_err(SceneMetadataError::Preservation)?;
                    *target = scene_candidate;
                    state.identities.retain(|identity| {
                        !(identity.domain == ForeignIdDomain::Paragraph
                            && removed.contains(&identity.value))
                    });
                    for id in allocated {
                        state.identities.push(
                            ForeignIdentity::new(ForeignIdDomain::Paragraph, id)
                                .map_err(SceneMetadataError::Preservation)?,
                        );
                    }
                    changed = true;
                    changed_scenes.insert(heading_id.as_str().to_owned());
                }
            }
            Err(SceneMetadataError::UnsupportedSceneMetadata) => {
                deferred_scenes.insert(heading_id.as_str().to_owned());
            }
            Err(error) => return Err(error),
        }
    }
    if changed {
        state.rebuild_managed_paragraph_index();
    }
    Ok(ReconcileOutcome {
        changed,
        deferred_scenes,
        changed_scenes,
        scene_candidates,
        preservation_state_clones: 0,
    })
}

/// Reads the recognized CharacterArcBeat projection for a managed heading.
/// Callers reconcile first so this is the single preserved source for all
/// scene-cast presentation derived from an imported or saved project.
pub(crate) fn scene_arc_names(
    project: &Project,
    heading_id: &StableId,
) -> Result<Vec<String>, SceneMetadataError> {
    let state = project
        .preservation
        .as_ref()
        .ok_or(SceneMetadataError::UnsupportedSceneMetadata)?;
    let heading = state
        .managed_element(heading_id)
        .map_err(SceneMetadataError::Preservation)?
        .ok_or(SceneMetadataError::Preservation(
            PreservationError::ManagedAnchorMissing,
        ))?;
    let namespace = heading.name.namespace_uri.clone();
    let mut properties = heading.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == "SceneProperties"
                && element.name.namespace_uri == namespace =>
        {
            Some(element)
        }
        _ => None,
    });
    let properties = properties
        .next()
        .ok_or(SceneMetadataError::UnsupportedSceneMetadata)?;
    if properties.name.namespace_uri != namespace
        || properties.children.iter().any(|node| {
            !matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "SceneArcBeats"
                    && element.name.namespace_uri == namespace)
                && !matches!(node,
                PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty())
        })
    {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    }
    let mut arcs = properties.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats" => {
            Some(element)
        }
        _ => None,
    });
    let arcs = arcs
        .next()
        .ok_or(SceneMetadataError::UnsupportedSceneMetadata)?;
    if arcs.name.namespace_uri != namespace {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    }
    let mut names = Vec::new();
    for child in &arcs.children {
        match child {
            PreservedNode::Element(beat) if beat.name.local_name == "CharacterArcBeat" => {
                let (_, name, _) = recognized_beat(beat, &namespace)
                    .ok_or(SceneMetadataError::UnsupportedSceneMetadata)?;
                names.push(name);
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            _ => return Err(SceneMetadataError::UnsupportedSceneMetadata),
        }
    }
    Ok(names)
}

/// Recognizes only the generated SceneProperties/SceneArcBeats shape that this
/// module emits and reconciles. The heading's direct Text runs are validated as
/// managed paragraph content; every other child must be formatting whitespace
/// or the one generated container. Returned IDs are the nested arc Paragraphs
/// whose complete subtrees are app-owned by the supported arc contract.
pub(crate) fn recognized_generated_scene_heading_subtree(
    heading: &PreservedElement,
) -> Option<Vec<String>> {
    if heading.name.local_name != "Paragraph" {
        return None;
    }
    let namespace = heading.name.namespace_uri.clone();
    let mut properties = None;
    let mut user_paragraph_data_seen = false;
    for (index, child) in heading.children.iter().enumerate() {
        match child {
            PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
                if element.name.namespace_uri != namespace || properties.replace(index).is_some() {
                    return None;
                }
            }
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == namespace
                    && element.children.iter().all(|node| {
                        matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
                    }) => {}
            PreservedNode::Element(element)
                if is_user_paragraph_data_lookalike(&element.name) =>
            {
                if !is_user_paragraph_data_name(&element.name, &heading.name)
                    || user_paragraph_data_seen
                {
                    return None;
                }
                user_paragraph_data_seen = true;
            }
            PreservedNode::Text(text) | PreservedNode::CData(text)
                if text.chars().all(char::is_whitespace) => {}
            _ => return None,
        }
    }
    let properties_index = properties?;
    let PreservedNode::Element(properties) = &heading.children[properties_index] else {
        return None;
    };
    let generated = generated_scene_properties(&heading.name);
    if !generated_scene_properties_attributes(properties, &generated) {
        return None;
    }
    let generated_arcs = generated.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats" => {
            Some(element)
        }
        _ => None,
    })?;

    let mut arcs = None;
    for (index, child) in properties.children.iter().enumerate() {
        match child {
            PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats" => {
                if element.name.local_name != generated_arcs.name.local_name
                    || element.name.namespace_uri != generated_arcs.name.namespace_uri
                    || arcs.replace(index).is_some()
                {
                    return None;
                }
            }
            PreservedNode::Text(text) | PreservedNode::CData(text)
                if text.chars().all(char::is_whitespace) => {}
            _ => return None,
        }
    }
    let arcs_index = arcs?;
    let PreservedNode::Element(arcs) = &properties.children[arcs_index] else {
        return None;
    };
    if !arcs.attributes.is_empty() {
        return None;
    }

    let mut association_keys = BTreeSet::new();
    let mut paragraph_ids = BTreeSet::new();
    let mut ids = Vec::new();
    for child in &arcs.children {
        match child {
            PreservedNode::Element(beat) if beat.name.local_name == "CharacterArcBeat" => {
                let (key, _, id) = recognized_beat(beat, &namespace)?;
                if !association_keys.insert(key) || !paragraph_ids.insert(id.clone()) {
                    return None;
                }
                ids.push(id);
            }
            PreservedNode::Text(text) | PreservedNode::CData(text)
                if text.chars().all(char::is_whitespace) => {}
            _ => return None,
        }
    }
    Some(ids)
}

fn generated_scene_properties_attributes(
    properties: &PreservedElement,
    generated: &PreservedElement,
) -> bool {
    if properties.name.local_name != generated.name.local_name
        || properties.name.namespace_uri != generated.name.namespace_uri
        || properties.attributes.len() != generated.attributes.len()
    {
        return false;
    }
    generated.attributes.iter().all(|expected| {
        let mut matching = properties.attributes.iter().filter(|attribute| {
            attribute.name.namespace_uri == expected.name.namespace_uri
                && attribute.name.local_name == expected.name.local_name
        });
        matching
            .next()
            .is_some_and(|attribute| attribute.value == expected.value)
            && matching.next().is_none()
    })
}

/// Projects canonical Character cues into normalized association keys and
/// uppercase names for generated and imported scene metadata.
pub(crate) fn derived_character_projection(
    elements: &[ScreenplayElement],
) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut projection = Vec::new();
    for element in elements {
        if element.role != Role::Character {
            continue;
        }
        let Some((key, name)) = projected_character_name(&element.text) else {
            continue;
        };
        if seen.insert(key.clone()) {
            projection.push((key, name.to_uppercase()));
        }
    }
    projection
}

type SceneProjection = (StableId, Vec<(String, String)>);
type ActiveSceneProjection = (StableId, Vec<(String, String)>, BTreeSet<String>);

fn scene_casts_with_scope(
    elements: &[ScreenplayElement],
    requested_scenes: Option<&BTreeSet<String>>,
) -> Vec<SceneProjection> {
    let mut scenes = Vec::new();
    let mut active: Option<ActiveSceneProjection> = None;
    for element in elements {
        if element.role == Role::SceneHeading {
            if let Some((heading, projection, _)) = active.take() {
                scenes.push((heading, projection));
            }
            if requested_scenes.is_none_or(|requested| requested.contains(element.id.as_str())) {
                active = Some((element.id.clone(), Vec::new(), BTreeSet::new()));
            }
        } else if element.role == Role::Character {
            if let Some((_, projection, seen)) = active.as_mut() {
                if let Some((key, name)) = projected_character_name(&element.text) {
                    if seen.insert(key.clone()) {
                        projection.push((key, name.to_uppercase()));
                    }
                }
            }
        }
    }
    if let Some((heading, projection, _)) = active {
        scenes.push((heading, projection));
    }
    scenes
}

/// Constructs native metadata for a new document using the shared paragraph-ID allocator.
pub(crate) fn new_scene_properties(
    names: &[String],
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedElement, SceneMetadataError> {
    let parent = SourceName {
        namespace_uri: None,
        local_name: "Paragraph".to_owned(),
        qualified_name: None,
    };
    let mut properties = generated_scene_properties(&parent);
    let PreservedNode::Element(arcs) = &mut properties.children[0] else {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    };
    for name in names {
        arcs.children.push(PreservedNode::Element(new_character_arc(
            name,
            paragraph_ids,
        )?));
    }
    Ok(properties)
}

fn reconcile_heading(
    heading: &mut PreservedElement,
    heading_depth: usize,
    inherited_space_preserve: bool,
    names: &[(String, String)],
    safely_removable: &BTreeSet<String>,
    known_paragraph_ids: &BTreeSet<String>,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<(Vec<String>, Vec<String>), SceneMetadataError> {
    let mut allocated = Vec::new();
    let mut removed = Vec::new();
    let namespace = heading.name.namespace_uri.clone();
    let mut user_paragraph_data_seen = false;
    for child in &heading.children {
        match child {
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == namespace
                    && element.children.iter().all(|node| {
                        matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
                    }) => {}
            PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
                if element.name.namespace_uri != namespace {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                }
            }
            PreservedNode::Element(element)
                if is_user_paragraph_data_lookalike(&element.name) =>
            {
                if !is_user_paragraph_data_name(&element.name, &heading.name)
                    || user_paragraph_data_seen
                {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                }
                user_paragraph_data_seen = true;
            }
            PreservedNode::Text(text) | PreservedNode::CData(text)
                if text.chars().all(char::is_whitespace) => {}
            _ => return Err(SceneMetadataError::UnsupportedSceneMetadata),
        }
    }
    let props_indexes = heading
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(child) if child.name.local_name == "SceneProperties" => {
                Some((index, child.name.namespace_uri == namespace))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if props_indexes.iter().any(|(_, same_ns)| !same_ns) || props_indexes.len() > 1 {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    }
    let props_index = props_indexes.first().map(|(index, _)| *index);
    let original_properties_preserve = props_index.is_some_and(|index| {
        matches!(&heading.children[index], PreservedNode::Element(properties)
            if has_effective_xml_space_preserve(properties, inherited_space_preserve))
    });
    let mut arcs_index = None;
    if let Some(properties_index) = props_index {
        let PreservedNode::Element(properties) = &heading.children[properties_index] else {
            return Err(SceneMetadataError::UnsupportedSceneMetadata);
        };
        if properties.name.namespace_uri != namespace {
            return Err(SceneMetadataError::UnsupportedSceneMetadata);
        }
        for (index, child) in properties.children.iter().enumerate() {
            if let PreservedNode::Element(element) = child {
                if element.name.local_name == "SceneArcBeats" {
                    if element.name.namespace_uri != namespace {
                        return Err(SceneMetadataError::UnsupportedSceneMetadata);
                    }
                    if arcs_index.replace(index).is_some() {
                        return Err(SceneMetadataError::UnsupportedSceneMetadata);
                    }
                }
            }
        }
    }
    // This heading is already a detached rollback candidate. Move its generated
    // arc children into the reconciliation workspace so opaque nested payloads
    // are not copied again; any safe-stop discards the complete heading candidate.
    let original_arc_children = match (props_index, arcs_index) {
        (Some(properties_index), Some(arcs_index)) => {
            let PreservedNode::Element(properties) = &mut heading.children[properties_index] else {
                return Err(SceneMetadataError::UnsupportedSceneMetadata);
            };
            let PreservedNode::Element(arcs) = &mut properties.children[arcs_index] else {
                return Err(SceneMetadataError::UnsupportedSceneMetadata);
            };
            Some(std::mem::take(&mut arcs.children))
        }
        _ => None,
    };

    let mut existing = Vec::<(String, String)>::new();
    let mut seen = BTreeSet::new();
    for child in original_arc_children
        .as_ref()
        .map_or(&[][..], |children| children.as_slice())
    {
        match child {
            PreservedNode::Element(beat) if beat.name.local_name == "CharacterArcBeat" => {
                let Some((key, _, id)) = recognized_beat(beat, &namespace) else {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                };
                if !seen.insert(key.clone()) || !known_paragraph_ids.contains(&id) {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                }
                existing.push((key, id));
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            _ => return Err(SceneMetadataError::UnsupportedSceneMetadata),
        }
    }
    let desired = names
        .iter()
        .map(|(key, name)| (key.as_str(), name.as_str()))
        .collect::<Vec<_>>();
    let desired_keys = desired.iter().map(|(key, _)| *key).collect::<BTreeSet<_>>();
    for (key, id) in &existing {
        if !desired_keys.contains(key.as_str()) {
            if !safely_removable.contains(id) {
                return Err(SceneMetadataError::UnsupportedSceneMetadata);
            }
            removed.push(id.clone());
        }
    }
    let mut existing_beats = BTreeMap::new();
    let mut old_children_template = Vec::new();
    let mut existing_metadata = existing.iter();
    for child in original_arc_children.unwrap_or_default() {
        match child {
            PreservedNode::Element(beat) if beat.name.local_name == "CharacterArcBeat" => {
                let Some((key, _)) = existing_metadata.next() else {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                };
                existing_beats.insert(key.clone(), beat);
                old_children_template.push(None);
            }
            other => old_children_template.push(Some(other)),
        }
    }
    let mut ordered_beats = Vec::with_capacity(names.len());
    for (key, name) in &desired {
        if let Some(mut beat) = existing_beats.remove(*key) {
            set_attribute(&mut beat, "Name", name);
            ordered_beats.push(PreservedNode::Element(beat));
        } else {
            let beat = new_character_arc_for(&heading.name, name, paragraph_ids, &mut allocated)?;
            ordered_beats.push(PreservedNode::Element(beat));
        }
    }
    let mut next_beat = ordered_beats.into_iter();
    let mut replacement = Vec::with_capacity(old_children_template.len().max(names.len()));
    for child in old_children_template {
        match child {
            Some(node) => replacement.push(node),
            None => {
                if let Some(beat) = next_beat.next() {
                    replacement.push(beat);
                }
            }
        }
    }
    let trailing_whitespace = replacement
        .iter()
        .rev()
        .take_while(|node| matches!(node, PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty()))
        .count();
    let insert_at = replacement.len().saturating_sub(trailing_whitespace);
    replacement.splice(insert_at..insert_at, next_beat);

    // SceneProperties and SceneArcBeats are generated containers for supported
    // headings. Only validated CharacterArcBeat subtrees cross this boundary.
    let mut properties = scene_properties_for(heading);
    let PreservedNode::Element(arcs) = &mut properties.children[0] else {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    };
    arcs.children = replacement.into();
    if let Some(index) = props_index {
        heading.children[index] = PreservedNode::Element(properties);
    } else {
        let insert_at = heading
            .children
            .iter()
            .position(|node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text" && child.name.namespace_uri == namespace))
            .unwrap_or(heading.children.len());
        heading
            .children
            .insert(insert_at, PreservedNode::Element(properties));
    }
    if !inherited_space_preserve && !original_properties_preserve {
        format_supported_scene_properties(heading, heading_depth);
    }
    Ok((removed, allocated))
}

fn effective_xml_space_preserve(element: &PreservedElement, inherited: bool) -> bool {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.local_name == "space"
                && (attribute.name.namespace_uri.as_deref()
                    == Some("http://www.w3.org/XML/1998/namespace")
                    || attribute.name.qualified_name.as_deref() == Some("xml:space"))
        })
        .map_or(inherited, |attribute| match attribute.value.as_str() {
            "default" => false,
            "preserve" => true,
            _ => true,
        })
}

/// Formats only the already-validated container generated for this supported
/// managed scene. Arc Paragraph children are deliberately opaque: only their
/// containing CharacterArcBeat gets element-only indentation.
fn format_supported_scene_properties(heading: &mut PreservedElement, heading_depth: usize) {
    let namespace = heading.name.namespace_uri.clone();
    let mut properties_indexes = heading
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "SceneProperties"
                    && element.name.namespace_uri == namespace =>
            {
                Some(index)
            }
            _ => None,
        });
    let Some(properties_index) = properties_indexes.next() else {
        return;
    };
    if properties_indexes.next().is_some() {
        return;
    }

    // A mixed or opaque heading child list is not a formatting context. Keep
    // its existing whitespace and content exactly as supplied.
    if !heading.children.iter().all(|node| match node {
        PreservedNode::Element(_) => true,
        PreservedNode::Text(text) => text.chars().all(char::is_whitespace),
        _ => false,
    }) {
        return;
    }
    let mut properties = match heading.children.get(properties_index) {
        Some(PreservedNode::Element(element)) => element.clone(),
        _ => return,
    };
    if has_effective_xml_space_preserve(&properties, false) {
        return;
    }
    let arcs_index = properties.children.iter().position(|node| {
        matches!(node, PreservedNode::Element(element)
            if element.name.local_name == "SceneArcBeats"
                && element.name.namespace_uri == namespace)
    });
    let Some(arcs_index) = arcs_index else {
        return;
    };
    if properties.children.iter().any(|node| match node {
        PreservedNode::Element(element) => {
            element.name.local_name != "SceneArcBeats" || element.name.namespace_uri != namespace
        }
        PreservedNode::Text(text) => !text.chars().all(char::is_whitespace),
        _ => true,
    }) {
        return;
    }

    let mut arcs = match properties.children.get(arcs_index) {
        Some(PreservedNode::Element(element)) => element.clone(),
        _ => return,
    };
    if arcs.children.iter().any(|node| match node {
        PreservedNode::Element(element) => {
            element.name.local_name != "CharacterArcBeat" || element.name.namespace_uri != namespace
        }
        PreservedNode::Text(text) => !text.chars().all(char::is_whitespace),
        _ => true,
    }) {
        return;
    }

    let properties_depth = heading_depth.saturating_add(1);
    let arcs_depth = properties_depth.saturating_add(1);
    if !format_element_only_children(&mut properties, properties_depth) {
        return;
    }
    if !arcs.children.is_empty() {
        if !format_element_only_children(&mut arcs, arcs_depth) {
            return;
        }
        for node in &mut arcs.children {
            let PreservedNode::Element(beat) = node else {
                continue;
            };
            if beat.name.local_name == "CharacterArcBeat"
                && beat.name.namespace_uri == namespace
                && !format_element_only_children(beat, arcs_depth.saturating_add(1))
            {
                return;
            }
        }
    }

    let Some(formatted_arcs_index) = properties.children.iter().position(|node| {
        matches!(node, PreservedNode::Element(element)
            if element.name.local_name == "SceneArcBeats"
                && element.name.namespace_uri == namespace)
    }) else {
        return;
    };
    properties.children[formatted_arcs_index] = PreservedNode::Element(arcs);
    heading.children[properties_index] = PreservedNode::Element(properties);
    format_scene_properties_sibling_gaps(heading, properties_index, heading_depth);
}

fn has_effective_xml_space_preserve(element: &PreservedElement, inherited: bool) -> bool {
    let current = effective_xml_space_preserve(element, inherited);
    current
        || element.children.iter().any(|node| match node {
            PreservedNode::Element(child) => has_effective_xml_space_preserve(child, current),
            _ => false,
        })
}

/// Replaces whitespace only in element-only generated containers, using the
/// element's absolute source-tree depth. Empty containers stay self-closing.
fn format_element_only_children(element: &mut PreservedElement, depth: usize) -> bool {
    if element.children.is_empty() {
        return true;
    }
    if element.children.iter().any(|node| match node {
        PreservedNode::Element(_) => false,
        PreservedNode::Text(text) => !text.chars().all(char::is_whitespace),
        _ => true,
    }) {
        return false;
    }

    let child_indent = format!("\n{}", "  ".repeat(depth.saturating_add(1)));
    let closing_indent = format!("\n{}", "  ".repeat(depth));
    let mut formatted = Vec::with_capacity(element.children.len().saturating_mul(2) + 1);
    for child in std::mem::take(&mut element.children) {
        if matches!(child, PreservedNode::Element(_)) {
            formatted.push(PreservedNode::Text(child_indent.clone().into()));
            formatted.push(child);
        }
    }
    formatted.push(PreservedNode::Text(closing_indent.into()));
    element.children = formatted.into();
    true
}

fn format_scene_properties_sibling_gaps(
    heading: &mut PreservedElement,
    properties_index: usize,
    heading_depth: usize,
) {
    let child_indent = format!("\n{}", "  ".repeat(heading_depth.saturating_add(1)));
    let closing_indent = format!("\n{}", "  ".repeat(heading_depth));
    let mut formatted = Vec::with_capacity(heading.children.len().saturating_add(2));
    let mut pending_whitespace = Vec::new();
    let mut previous_was_properties = false;
    for (index, child) in std::mem::take(&mut heading.children)
        .into_iter()
        .enumerate()
    {
        if matches!(&child, PreservedNode::Text(text) if text.chars().all(char::is_whitespace)) {
            pending_whitespace.push(child);
            continue;
        }
        let is_properties = index == properties_index;
        if previous_was_properties || is_properties {
            formatted.push(PreservedNode::Text(child_indent.clone().into()));
        } else {
            formatted.append(&mut pending_whitespace);
        }
        pending_whitespace.clear();
        formatted.push(child);
        previous_was_properties = is_properties;
    }
    if previous_was_properties {
        formatted.push(PreservedNode::Text(closing_indent.into()));
    } else {
        formatted.append(&mut pending_whitespace);
    }
    heading.children = formatted.into();
}

fn scene_properties_for(heading: &PreservedElement) -> PreservedElement {
    generated_scene_properties(&heading.name)
}

fn generated_scene_properties(parent: &SourceName) -> PreservedElement {
    let mut properties = namespaced_element_from_name(parent, "SceneProperties");
    properties.attributes = [("Length", "1/8"), ("Page", "1"), ("Title", "")]
        .into_iter()
        .map(|(name, value)| attribute(name, value))
        .collect();
    properties
        .children
        .push(PreservedNode::Element(namespaced_element_from_name(
            parent,
            "SceneArcBeats",
        )));
    properties
}

fn new_character_arc(
    name: &str,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedElement, SceneMetadataError> {
    let id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    Ok(character_arc_element(name, id, None))
}

fn new_character_arc_for(
    parent: &SourceName,
    name: &str,
    paragraph_ids: &mut ParagraphIdAllocator,
    allocated: &mut Vec<String>,
) -> Result<PreservedElement, SceneMetadataError> {
    let id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    allocated.push(id.clone());
    Ok(character_arc_element(name, id, Some(parent)))
}

fn map_paragraph_id_error(error: ParagraphIdAllocationError) -> SceneMetadataError {
    match error {
        ParagraphIdAllocationError::EntropyUnavailable => {
            SceneMetadataError::ParagraphIdentityEntropyUnavailable
        }
        ParagraphIdAllocationError::CollisionLimit => {
            SceneMetadataError::ParagraphIdentityCollisionLimit
        }
    }
}

fn character_arc_element(name: &str, id: String, parent: Option<&SourceName>) -> PreservedElement {
    let make = |local: &str| match parent {
        Some(parent) => namespaced_element_from_name(parent, local),
        None => element(local, Vec::new()),
    };
    let mut beat = make("CharacterArcBeat");
    beat.attributes.push(attribute("Name", name));
    let mut paragraph = make("Paragraph");
    paragraph.attributes = [
        ("Alignment", "Left"),
        ("FirstIndent", "0.00"),
        ("Leading", "Regular"),
        ("LeftIndent", "0.00"),
        ("OutlineLevel", "0"),
        ("RightIndent", "1.39"),
        ("SpaceBefore", "0"),
        ("Spacing", "1"),
        ("StartsNewPage", "No"),
    ]
    .into_iter()
    .map(|(key, value)| attribute(key, value))
    .collect();
    paragraph.attributes.push(attribute("id", &id));
    beat.children.push(PreservedNode::Element(paragraph));
    beat
}

fn recognized_beat(
    beat: &PreservedElement,
    parent_namespace: &Option<String>,
) -> Option<(String, String, String)> {
    if beat.name.namespace_uri != *parent_namespace || beat.attributes.len() != 1 {
        return None;
    }
    let name_attribute = beat.attributes.first()?;
    if name_attribute.name.namespace_uri.is_some() || name_attribute.name.local_name != "Name" {
        return None;
    }
    let name = name_attribute.value.clone();
    let mut paragraph = None;
    for child in &beat.children {
        match child {
            PreservedNode::Element(child) if child.name.local_name == "Paragraph" => {
                if paragraph.replace(child).is_some() {
                    return None;
                }
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            _ => return None,
        }
    }
    let paragraph = paragraph?;
    if paragraph.name.namespace_uri != *parent_namespace
        || !recognized_arc_paragraph_content(paragraph, parent_namespace)
    {
        return None;
    }
    let mut id = None;
    for attribute in &paragraph.attributes {
        if attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
            && id.replace(attribute.value.clone()).is_some()
        {
            return None;
        }
    }
    let id = id?;
    Some((character_association_key(&name)?, name, id))
}

/// Arc Paragraphs remain opaque source nodes. Their only interpreted content is
/// an ordered sequence of same-namespace Text runs; all attributes and run
/// formatting stay attached to the original nodes when a Character is retained.
fn recognized_arc_paragraph_content(
    paragraph: &PreservedElement,
    parent_namespace: &Option<String>,
) -> bool {
    let mut user_paragraph_data_seen = false;
    paragraph.children.iter().all(|child| match child {
        PreservedNode::Element(run)
            if run.name.local_name == "Text" && run.name.namespace_uri == *parent_namespace =>
        {
            run.children
                .iter()
                .all(|part| matches!(part, PreservedNode::Text(_) | PreservedNode::CData(_)))
        }
        PreservedNode::Element(element) if is_user_paragraph_data_lookalike(&element.name) => {
            if !is_user_paragraph_data_name(&element.name, &paragraph.name)
                || user_paragraph_data_seen
            {
                return false;
            }
            user_paragraph_data_seen = true;
            true
        }
        PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => true,
        _ => false,
    })
}

fn safe_paragraph_removals(state: &PreservationState) -> BTreeSet<String> {
    state
        .identities
        .iter()
        .filter_map(|identity| {
            if identity.domain != ForeignIdDomain::Paragraph {
                return None;
            }
            state
                .paragraph_identity_safely_removable(&identity.value)
                .then(|| identity.value.clone())
        })
        .collect()
}

fn projected_character_name(cue: &str) -> Option<(String, &str)> {
    const DELIVERY_EXTENSIONS: [&str; 7] = [
        "(V.O.)",
        "(O.S.)",
        "(O.C.)",
        "(CONT'D)",
        "(SUBTITLE)",
        "(TEXT)",
        "(V.O)",
    ];

    let cue = cue.trim();
    if cue.is_empty() {
        return None;
    }
    let name = DELIVERY_EXTENSIONS
        .iter()
        .find_map(|extension| {
            let suffix_start = cue.len().checked_sub(extension.len())?;
            cue.get(suffix_start..)
                .filter(|suffix| suffix.eq_ignore_ascii_case(extension))
                .map(|_| cue[..suffix_start].trim_end())
        })
        .unwrap_or(cue)
        .trim();
    (!name.is_empty()).then(|| (name.to_lowercase(), name))
}

fn character_association_key(name: &str) -> Option<String> {
    projected_character_name(name).map(|(key, _)| key)
}

fn set_attribute(element: &mut PreservedElement, name: &str, value: &str) {
    if let Some(attribute) = element.attributes.iter_mut().find(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    }) {
        attribute.value = value.to_owned();
    }
}

fn namespaced_element_from_name(parent: &SourceName, local: &str) -> PreservedElement {
    let prefix = parent
        .qualified_name
        .as_deref()
        .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
    let qualified_name =
        prefix.map_or_else(|| local.to_owned(), |prefix| format!("{prefix}:{local}"));
    PreservedElement {
        name: SourceName {
            namespace_uri: parent.namespace_uri.clone(),
            local_name: local.to_owned(),
            qualified_name: Some(qualified_name),
        },
        attributes: Vec::new(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: crate::preservation::DependencyConfidence::Unknown,
        anchor: None,
        children: Default::default(),
    }
}

fn element(name: &str, attrs: Vec<(&str, &str)>) -> PreservedElement {
    let mut element = namespaced_element_from_name(
        &SourceName {
            namespace_uri: None,
            local_name: String::new(),
            qualified_name: None,
        },
        name,
    );
    element.attributes = attrs
        .into_iter()
        .map(|(key, value)| attribute(key, value))
        .collect();
    element
}

fn attribute(name: &str, value: &str) -> PreservedAttribute {
    let (namespace_uri, local_name) = if name == "xml:space" {
        (
            Some("http://www.w3.org/XML/1998/namespace".to_owned()),
            "space",
        )
    } else {
        (None, name)
    };
    PreservedAttribute {
        name: SourceName {
            namespace_uri,
            local_name: local_name.to_owned(),
            qualified_name: Some(name.to_owned()),
        },
        value: value.to_owned(),
    }
}
