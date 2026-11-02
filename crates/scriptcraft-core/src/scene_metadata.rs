//! Preservation-safe derived SceneProperties and scene-cast arc projection.

use crate::preservation::{
    AnchorTarget, ForeignIdDomain, ForeignIdentity, PreservedAttribute, PreservedElement,
    PreservedNode, PreservationError, PreservationState, SourceName, SourceOwnership,
};
use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
use crate::{ModelError, Project, Role, ScreenplayElement, StableId};
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub(crate) struct ReconcileOutcome {
    pub changed: bool,
    pub deferred_scenes: BTreeSet<String>,
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

/// Refreshes supported scene properties and their derived character-arc
/// projection in a detached preservation candidate. Unsupported scenes are
/// independently deferred without publishing partial changes to their subtree.
pub(crate) fn reconcile_project(project: &mut Project) -> Result<bool, SceneMetadataError> {
    reconcile_project_with_outcome(project).map(|outcome| outcome.changed)
}

pub(crate) fn reconcile_project_with_outcome(
    project: &mut Project,
) -> Result<ReconcileOutcome, SceneMetadataError> {
    if project.preservation.is_none() {
        // New projects acquire their source tree in the FDX writer, where the
        // same projection is emitted directly from semantic content.
        return Ok(ReconcileOutcome::default());
    }
    let before = project.clone();
    let mut candidate = project.clone();
    let scenes = scene_casts(&candidate.elements);
    let mut state = candidate
        .preservation
        .clone()
        .ok_or(SceneMetadataError::UnsupportedSceneMetadata)?;
    let removable = safe_paragraph_removals(&state);
    let mut deferred_scenes = BTreeSet::new();
    for (heading_id, names) in scenes {
        let mut scene_candidate = state.clone();
        match reconcile_heading(&mut scene_candidate, &heading_id, &names, &removable) {
            Ok(()) => state = scene_candidate,
            Err(SceneMetadataError::UnsupportedSceneMetadata) => {
                deferred_scenes.insert(heading_id.as_str().to_owned());
            }
            Err(error) => return Err(error),
        }
    }
    candidate.preservation = Some(state);
    candidate.validate()?;
    let changed = candidate != before;
    if changed {
        *project = candidate;
    }
    Ok(ReconcileOutcome {
        changed,
        deferred_scenes,
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
    let heading = find_managed(&state.sections, heading_id)
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
    if properties.name.namespace_uri != namespace || properties.children.iter().any(|node| {
        !matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "SceneArcBeats"
                    && element.name.namespace_uri == namespace)
            && !matches!(node,
                PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty())
    }) {
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

/// Projects canonical Character cues into normalized association keys and
/// uppercase names for generated and imported scene metadata.
pub(crate) fn derived_character_projection(elements: &[ScreenplayElement]) -> Vec<(String, String)> {
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

pub(crate) fn scene_casts(
    elements: &[ScreenplayElement],
) -> Vec<(StableId, Vec<(String, String)>)> {
    let mut scenes = Vec::new();
    let mut active: Option<(StableId, Vec<ScreenplayElement>)> = None;
    for element in elements {
        if element.role == Role::SceneHeading {
            if let Some((heading, body)) = active.take() {
                scenes.push((heading, derived_character_projection(&body)));
            }
            active = Some((element.id.clone(), Vec::new()));
        } else if let Some((_, body)) = active.as_mut() {
            body.push(element.clone());
        }
    }
    if let Some((heading, body)) = active {
        scenes.push((heading, derived_character_projection(&body)));
    }
    scenes
}

/// Constructs native metadata for a new document using the shared paragraph-ID allocator.
pub(crate) fn new_scene_properties(
    names: &[String],
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedElement, SceneMetadataError> {
    let mut properties = element(
        "SceneProperties",
        vec![("Length", "1/8"), ("Page", "1"), ("Title", "")],
    );
    let mut arcs = element("SceneArcBeats", Vec::new());
    for name in names {
        arcs.children.push(PreservedNode::Element(new_character_arc(
            name,
            paragraph_ids,
        )?));
    }
    properties.children.push(PreservedNode::Element(arcs));
    Ok(properties)
}

fn reconcile_heading(
    state: &mut PreservationState,
    heading_id: &StableId,
    names: &[(String, String)],
    safely_removable: &BTreeSet<String>,
) -> Result<(), SceneMetadataError> {
    let reserved_ids = source_id_values(&state.sections)
        .into_iter()
        .chain(state.identities.iter().map(|identity| identity.value.clone()))
        .chain(
            state
                .board_graph
                .entities
                .iter()
                .map(|entity| entity.identity.value.clone()),
        )
        .chain(state.references.iter().flat_map(|reference| {
            [
                reference.source.value.clone(),
                reference.target.value.clone(),
            ]
        }))
        .chain(state.board_graph.references.iter().flat_map(|reference| {
            [
                reference.source.value.clone(),
                reference.target.value.clone(),
            ]
        }));
    let mut paragraph_ids = ParagraphIdAllocator::new(reserved_ids);
    let mut allocated = Vec::new();
    let mut removed = Vec::new();
    let known_paragraph_ids = state
        .identities
        .iter()
        .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
        .map(|identity| identity.value.clone())
        .collect::<BTreeSet<_>>();
    let heading = find_managed_mut(&mut state.sections, heading_id)
        .ok_or(SceneMetadataError::Preservation(
            PreservationError::ManagedAnchorMissing,
        ))?;
    let namespace = heading.name.namespace_uri.clone();
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
    let props_index = if let Some((index, _)) = props_indexes.first() {
        *index
    } else {
        let properties = scene_properties_for(heading);
        let insert_at = heading
            .children
            .iter()
            .position(|node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text" && child.name.namespace_uri == namespace))
            .unwrap_or(heading.children.len());
        heading
            .children
            .insert(insert_at, PreservedNode::Element(properties));
        insert_at
    };
    let PreservedNode::Element(properties) = &mut heading.children[props_index] else {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    };
    if properties.name.namespace_uri != namespace {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    }
    let mut arc_indexes = Vec::new();
    for (index, child) in properties.children.iter().enumerate() {
        match child {
            PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats" => {
                if element.name.namespace_uri != namespace {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                }
                arc_indexes.push(index);
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            _ => return Err(SceneMetadataError::UnsupportedSceneMetadata),
        }
    }
    if arc_indexes.len() > 1 {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    }
    let arc_index = if let Some(index) = arc_indexes.first() {
        *index
    } else {
        properties
            .children
            .push(PreservedNode::Element(element("SceneArcBeats", Vec::new())));
        properties.children.len().saturating_sub(1)
    };
    let PreservedNode::Element(arcs) = &mut properties.children[arc_index] else {
        return Err(SceneMetadataError::UnsupportedSceneMetadata);
    };
    let mut existing = Vec::<(String, String, String, PreservedElement)>::new();
    let mut seen = BTreeSet::new();
    for child in &arcs.children {
        match child {
            PreservedNode::Element(beat) if beat.name.local_name == "CharacterArcBeat" => {
                let Some((key, name, id)) = recognized_beat(beat, &namespace) else {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                };
                if !seen.insert(key.clone()) || !known_paragraph_ids.contains(&id)
                {
                    return Err(SceneMetadataError::UnsupportedSceneMetadata);
                }
                existing.push((key, name, id, beat.clone()));
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
    for (key, _, id, _) in &existing {
        if !desired_keys.contains(key.as_str()) {
            if !safely_removable.contains(id) {
                return Err(SceneMetadataError::UnsupportedSceneMetadata);
            }
            removed.push(id.clone());
        }
    }
    let mut ordered_beats = Vec::with_capacity(desired.len());
    for (key, name) in desired {
        if let Some((_, _, _, original)) = existing.iter().find(|entry| entry.0 == key) {
            let mut beat = original.clone();
            set_attribute(&mut beat, "Name", name);
            ordered_beats.push(PreservedNode::Element(beat));
        } else {
            let beat = new_character_arc_for(&arcs.name, name, &mut paragraph_ids, &mut allocated)?;
            ordered_beats.push(PreservedNode::Element(beat));
        }
    }
    let old_children = std::mem::take(&mut arcs.children);
    let mut replacement = Vec::with_capacity(old_children.len().max(ordered_beats.len()));
    let mut next_beat = ordered_beats.into_iter();
    for child in old_children {
        if matches!(&child, PreservedNode::Element(element) if element.name.local_name == "CharacterArcBeat") {
            if let Some(beat) = next_beat.next() {
                replacement.push(beat);
            }
        } else {
            replacement.push(child);
        }
    }
    let trailing_whitespace = replacement
        .iter()
        .rev()
        .take_while(|node| matches!(node, PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty()))
        .count();
    let insert_at = replacement.len().saturating_sub(trailing_whitespace);
    replacement.splice(insert_at..insert_at, next_beat);
    arcs.children = replacement;
    for id in removed {
        state.identities.retain(|identity| {
            !(identity.domain == ForeignIdDomain::Paragraph && identity.value == id)
        });
    }
    for id in allocated {
        state.identities.push(
            ForeignIdentity::new(ForeignIdDomain::Paragraph, id.clone())
                .map_err(SceneMetadataError::Preservation)?,
        );
    }
    Ok(())
}

fn scene_properties_for(heading: &PreservedElement) -> PreservedElement {
    let mut properties = namespaced_element(heading, "SceneProperties", Vec::new());
    properties.attributes = [("Length", "1/8"), ("Page", "1"), ("Title", "")]
        .into_iter()
        .map(|(name, value)| attribute(name, value))
        .collect();
    properties
        .children
        .push(PreservedNode::Element(namespaced_element(
            heading,
            "SceneArcBeats",
            Vec::new(),
        )));
    properties
}

fn new_character_arc(
    name: &str,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedElement, SceneMetadataError> {
    let id = paragraph_ids
        .allocate()
        .map_err(map_paragraph_id_error)?;
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

fn recognized_beat(beat: &PreservedElement, parent_namespace: &Option<String>) -> Option<(String, String, String)> {
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
        {
            if id.replace(attribute.value.clone()).is_some() {
                return None;
            }
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
    paragraph.children.iter().all(|child| match child {
        PreservedNode::Element(run)
            if run.name.local_name == "Text" && run.name.namespace_uri == *parent_namespace =>
        {
            run.children
                .iter()
                .all(|part| matches!(part, PreservedNode::Text(_) | PreservedNode::CData(_)))
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
            let identity = ForeignIdentity {
                domain: ForeignIdDomain::Paragraph,
                value: identity.value.clone(),
            };
            let safe = state.identities.iter().filter(|item| **item == identity).count() == 1
                && !state
                    .references
                    .iter()
                    .chain(&state.board_graph.references)
                    .any(|reference| reference.source == identity || reference.target == identity)
                && !state.anchors.iter().any(|anchor| {
                    matches!(&anchor.target, AnchorTarget::Foreign(target) if target == &identity)
                })
                && !external_id_attribute_use(&state.sections, &identity.value);
            safe.then_some(identity.value)
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

fn external_id_attribute_use(sections: &[crate::preservation::PreservedSection], id: &str) -> bool {
    fn scan(nodes: &[PreservedNode], id: &str) -> bool {
        nodes.iter().any(|node| match node {
            PreservedNode::Element(element) => {
                let is_target_paragraph = element.name.local_name == "Paragraph" && id_value(element) == Some(id);
                element.attributes.iter().any(|attribute| {
                    attribute.value.contains(id)
                        && !(is_target_paragraph
                            && attribute.name.namespace_uri.is_none()
                            && attribute.name.local_name.eq_ignore_ascii_case("id"))
                }) || scan(&element.children, id)
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) | PreservedNode::Comment(text) => text.contains(id),
            PreservedNode::ProcessingInstruction { target, data } => target.contains(id) || data.contains(id),
        })
    }
    sections.iter().any(|section| scan(&section.children, id))
}

fn source_id_values(sections: &[crate::preservation::PreservedSection]) -> BTreeSet<String> {
    fn scan(nodes: &[PreservedNode], ids: &mut BTreeSet<String>) {
        for node in nodes {
            if let PreservedNode::Element(element) = node {
                for attribute in &element.attributes {
                    if attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id")
                    {
                        ids.insert(attribute.value.clone());
                    }
                }
                scan(&element.children, ids);
            }
        }
    }
    let mut ids = BTreeSet::new();
    for section in sections {
        scan(&section.children, &mut ids);
    }
    ids
}

fn find_managed_mut<'a>(
    sections: &'a mut [crate::preservation::PreservedSection],
    id: &StableId,
) -> Option<&'a mut PreservedElement> {
    fn find<'a>(nodes: &'a mut [PreservedNode], id: &StableId) -> Option<&'a mut PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some(element);
            }
            if let Some(found) = find(&mut element.children, id) {
                return Some(found);
            }
        }
        None
    }
    sections
        .iter_mut()
        .find_map(|section| find(&mut section.children, id))
}

fn find_managed<'a>(
    sections: &'a [crate::preservation::PreservedSection],
    id: &StableId,
) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &StableId) -> Option<&'a PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some(element);
            }
            if let Some(found) = find(&element.children, id) {
                return Some(found);
            }
        }
        None
    }
    sections
        .iter()
        .find_map(|section| find(&section.children, id))
}

fn id_value(element: &PreservedElement) -> Option<&str> {
    let mut ids = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    });
    let value = ids.next()?.value.as_str();
    ids.next().is_none().then_some(value)
}

fn set_attribute(element: &mut PreservedElement, name: &str, value: &str) {
    if let Some(attribute) = element.attributes.iter_mut().find(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    }) {
        attribute.value = value.to_owned();
    }
}

fn namespaced_element(parent: &PreservedElement, local: &str, _attrs: Vec<(&str, &str)>) -> PreservedElement {
    namespaced_element_from_name(&parent.name, local)
}

fn namespaced_element_from_name(parent: &SourceName, local: &str) -> PreservedElement {
    let prefix = parent
        .qualified_name
        .as_deref()
        .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
    let qualified_name = prefix.map_or_else(|| local.to_owned(), |prefix| format!("{prefix}:{local}"));
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
        children: Vec::new(),
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
    element.attributes = attrs.into_iter().map(|(key, value)| attribute(key, value)).collect();
    element
}

fn attribute(name: &str, value: &str) -> PreservedAttribute {
    PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: name.to_owned(),
            qualified_name: Some(name.to_owned()),
        },
        value: value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{EditingSession, Position, Selection};
    use crate::fdx::{read_v6, write_v6};

    #[test]
    fn new_document_scene_metadata_is_created_and_round_trips() {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = vec![
            ScreenplayElement::new("heading", Role::SceneHeading, "INT. ROOM").unwrap_or_else(|_| unreachable!()),
            ScreenplayElement::new("cue-1", Role::Character, "  Zoë\u{00a0}").unwrap_or_else(|_| unreachable!()),
            ScreenplayElement::new("cue-2", Role::Character, "zoë").unwrap_or_else(|_| unreachable!()),
        ];
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let text = String::from_utf8(output).unwrap_or_else(|_| unreachable!());
        assert!(text.contains("<SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\">"));
        assert!(text.contains("<CharacterArcBeat Name=\"ZOË\">"));
        assert!(text.contains("<SceneArcBeats>"));
        assert!(text.find("<SceneProperties ").unwrap_or(usize::MAX) < text.find("<Text>INT. ROOM</Text>").unwrap_or(0));
        assert!(text.contains("Alignment=\"Left\" FirstIndent=\"0.00\" Leading=\"Regular\""));
        let reopened = read_v6(text.as_bytes()).unwrap_or_else(|_| unreachable!());
        let state = reopened.project.preservation.as_ref().unwrap_or_else(|| unreachable!());
        assert!(state.identities.iter().any(|identity| {
            identity.domain == ForeignIdDomain::Paragraph
                && uuid::Uuid::parse_str(&identity.value)
                    .is_ok_and(|value| value.get_version() == Some(uuid::Version::Random))
        }));
    }

    #[test]
    fn imported_scenes_are_independent_and_store_projected_uppercase_names() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ONE</Text></Paragraph><Paragraph Type="Character" id="c1"><Text> Zoë </Text></Paragraph><Paragraph Type="Character" id="c2"><Text>zoë</Text></Paragraph><Paragraph Type="Scene Heading" id="h2"><Text>EXT. TWO</Text></Paragraph><Paragraph Type="Character" id="c3"><Text>JO</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let text = String::from_utf8(output).unwrap_or_else(|_| unreachable!());
        assert_eq!(text.matches("<SceneProperties ").count(), 2);
        assert_eq!(text.matches("<CharacterArcBeat Name=\"ZOË\">").count(), 1);
        assert_eq!(text.matches("<CharacterArcBeat Name=\"JO\">").count(), 1);
        assert!(text.contains("<Text> Zoë </Text>"));
    }

    #[test]
    fn imported_scene_metadata_adds_and_matches_case_insensitively() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>Alpha (V.O.)</Text></Paragraph><Paragraph Type="Character" id="c2"><Text>Beta</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let output = String::from_utf8(write_v6(&project).unwrap_or_else(|_| unreachable!()))
            .unwrap_or_else(|_| unreachable!());
        assert!(output.contains("<CharacterArcBeat Name=\"ALPHA\">"));
        assert!(output.contains("<CharacterArcBeat Name=\"BETA\">"));
        assert!(output.contains("<Text>Alpha (V.O.)</Text>"));
    }

    #[test]
    fn existing_scene_properties_values_and_matching_arc_identity_are_retained() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><SceneProperties Length="3/8" Page="7" Title="kept" Vendor="untouched"><SceneArcBeats><CharacterArcBeat Name="ALPHA"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="authored-arc-id"/></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(!reconcile_project(&mut project).unwrap_or(true));
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let text = String::from_utf8(output).unwrap_or_else(|_| unreachable!());
        assert!(text.contains("Length=\"3/8\" Page=\"7\" Title=\"kept\" Vendor=\"untouched\""));
        assert!(text.contains("id=\"authored-arc-id\""));
    }

    #[test]
    fn app_owned_reconciliation_removes_stale_arc_after_reopen() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph></Content></FinalDraft>"#;
        let mut imported = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(reconcile_project(&mut imported).unwrap_or(false));
        let saved = write_v6(&imported).unwrap_or_else(|_| unreachable!());
        let mut project = read_v6(&saved).unwrap_or_else(|_| unreachable!()).project;
        project.elements[1].text = "BETA".to_owned();
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let text = String::from_utf8(output).unwrap_or_else(|_| unreachable!());
        assert!(!text.contains("Name=\"ALPHA\""));
        assert!(text.contains("Name=\"BETA\""));
    }

    #[test]
    fn possible_source_reference_defers_only_the_unsafe_scene() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><SceneProperties><SceneArcBeats><CharacterArcBeat Name="ALPHA"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="arc-p"/></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        let state = project.preservation.as_mut().unwrap_or_else(|| unreachable!());
        let PreservedNode::Element(document) = &mut state.sections[0].children[0] else {
            unreachable!();
        };
        let content = document.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Content" => Some(element),
            _ => None,
        }).unwrap_or_else(|| unreachable!());
        let mut vendor = element("VendorLink", Vec::new());
        vendor.attributes.push(attribute("Ref", "arc-p"));
        content.children.push(PreservedNode::Element(vendor));
        project.elements[1].text = "BETA".to_owned();
        let original = project.clone();
        let heading_id = project.elements[0].id.as_str().to_owned();
        let outcome = reconcile_project_with_outcome(&mut project).unwrap_or_default();
        assert!(outcome.deferred_scenes.contains(&heading_id));
        assert_eq!(project, original);
    }

    #[test]
    fn refresh_and_history_reconcile_scene_arc_projection() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph></Content></FinalDraft>"#;
        let project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        session
            .set_selection(Selection::caret(Position {
                element: 1,
                offset: 5,
            }))
            .unwrap_or_else(|_| unreachable!());
        session.insert("X").unwrap_or_else(|_| unreachable!());
        assert!(session.refresh_cast_lists().unwrap_or(false));
        let output = write_v6(session.project()).unwrap_or_else(|_| unreachable!());
        assert!(String::from_utf8(output).unwrap_or_else(|_| unreachable!()).contains("Name=\"ALPHAX\""));

        session.undo().unwrap_or_else(|_| unreachable!());
        let output = write_v6(session.project()).unwrap_or_else(|_| unreachable!());
        assert!(String::from_utf8(output).unwrap_or_else(|_| unreachable!()).contains("Name=\"ALPHA\""));
        session.redo().unwrap_or_else(|_| unreachable!());
        let output = write_v6(session.project()).unwrap_or_else(|_| unreachable!());
        assert!(String::from_utf8(output).unwrap_or_else(|_| unreachable!()).contains("Name=\"ALPHAX\""));
    }

    #[test]
    fn unsupported_non_text_arc_child_defers_scene_without_mutation() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><SceneProperties Length="3/8" Page="7" Title="kept"><SceneArcBeats><CharacterArcBeat Name="OLD"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="arc-p"/></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>NEW</Text></Paragraph></Content></FinalDraft>"#;
        let authored = source.replace(
            "id=\"arc-p\"/>",
            "id=\"arc-p\"><DynamicLabel Type=\"unsupported\"/></Paragraph>",
        );
        let mut project = read_v6(authored.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        let original = project.clone();
        let heading_id = project.elements[0].id.as_str().to_owned();
        let outcome = reconcile_project_with_outcome(&mut project).unwrap_or_default();
        assert!(outcome.deferred_scenes.contains(&heading_id));
        assert_eq!(project, original);
    }
}
