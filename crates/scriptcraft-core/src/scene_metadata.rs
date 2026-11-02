//! Preservation-safe derived SceneProperties and scene-cast arc projection.

use crate::preservation::{
    ForeignIdDomain, ForeignIdentity, PreservedAttribute, PreservedElement, PreservedNode,
    AnchorTarget, PreservationError, PreservationState, SourceName, SourceOwnership,
};
use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
use crate::{ModelError, Project, Role, ScreenplayElement, StableId};
use std::collections::BTreeSet;

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

/// Refreshes scene properties and their derived character-arc projection in a
/// detached preservation candidate. Unrecognized or unowned stale data stops
/// the complete update without publishing partial changes.
pub(crate) fn reconcile_project(project: &mut Project) -> Result<bool, SceneMetadataError> {
    if project.preservation.is_none() {
        // New projects acquire their source tree in the FDX writer, where the
        // same projection is emitted directly from semantic content.
        return Ok(false);
    }
    let before = project.clone();
    let mut candidate = project.clone();
    let scenes = scene_casts(&candidate.elements);
    let state = candidate
        .preservation
        .as_mut()
        .ok_or(SceneMetadataError::UnsupportedSceneMetadata)?;
    let removable = safe_generated_removals(state);
    for (heading_id, names) in scenes {
        reconcile_heading(state, &heading_id, &names, &removable)?;
    }
    candidate.validate()?;
    let changed = candidate != before;
    if changed {
        *project = candidate;
    }
    Ok(changed)
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

/// First-spelling, first-occurrence character projection shared by preservation
/// reconciliation and new-document generation.
pub(crate) fn character_names(elements: &[ScreenplayElement]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    for element in elements {
        if element.role != Role::Character {
            continue;
        }
        let name = element.text.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_lowercase();
        if seen.insert(key) {
            names.push(name.to_owned());
        }
    }
    names
}

pub(crate) fn scene_casts(elements: &[ScreenplayElement]) -> Vec<(StableId, Vec<String>)> {
    let mut scenes = Vec::new();
    let mut active: Option<(StableId, Vec<ScreenplayElement>)> = None;
    for element in elements {
        if element.role == Role::SceneHeading {
            if let Some((heading, body)) = active.take() {
                scenes.push((heading, character_names(&body)));
            }
            active = Some((element.id.clone(), Vec::new()));
        } else if let Some((_, body)) = active.as_mut() {
            body.push(element.clone());
        }
    }
    if let Some((heading, body)) = active {
        scenes.push((heading, character_names(&body)));
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

/// Carries only creation proof across the writer's mandatory re-read. For an
/// imported candidate, only IDs explicitly tracked before serialization qualify.
/// A newly-authored candidate may trust exact arc nodes emitted by this writer.
pub(crate) fn carry_generated_ownership(expected: &Project, actual: &mut Project) {
    let Some(actual_state) = actual.preservation.as_mut() else {
        return;
    };
    let allowed = expected
        .preservation
        .as_ref()
        .map(|state| state.generated_scene_arc_paragraph_ids.clone());
    let generated_by_writer = expected.preservation.is_none();
    let mut found = BTreeSet::new();
    collect_arc_paragraph_ids(&actual_state.sections, &mut found);
    actual_state.generated_scene_arc_paragraph_ids = match allowed {
        Some(ids) => ids.intersection(&found).cloned().collect(),
        None if generated_by_writer => found,
        None => BTreeSet::new(),
    };
}

fn collect_arc_paragraph_ids(sections: &[crate::preservation::PreservedSection], out: &mut BTreeSet<String>) {
    fn scan(nodes: &[PreservedNode], inside_arc: bool, out: &mut BTreeSet<String>) {
        for node in nodes {
            let PreservedNode::Element(current) = node else {
                continue;
            };
            let is_arc = current.name.local_name == "CharacterArcBeat";
            if inside_arc && current.name.local_name == "Paragraph" {
                if let Some(id) = id_value(current) {
                    out.insert(id.to_owned());
                }
            }
            scan(&current.children, inside_arc || is_arc, out);
        }
    }
    for section in sections {
        scan(&section.children, false, out);
    }
}

fn reconcile_heading(
    state: &mut PreservationState,
    heading_id: &StableId,
    names: &[String],
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
    let mut retained_owned = state.generated_scene_arc_paragraph_ids.clone();
    let generated_ids = state.generated_scene_arc_paragraph_ids.clone();
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
        .map(|name| (name.to_lowercase(), name.as_str()))
        .collect::<Vec<_>>();
    let desired_keys = desired
        .iter()
        .map(|(key, _)| key.as_str())
        .collect::<BTreeSet<_>>();
    for (key, _, id, _) in &existing {
        if !desired_keys.contains(key.as_str()) {
            if !generated_ids.contains(id) || !safely_removable.contains(id) {
                return Err(SceneMetadataError::UnsupportedSceneMetadata);
            }
            removed.push(id.clone());
            retained_owned.remove(id);
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
        retained_owned.insert(id);
    }
    state.generated_scene_arc_paragraph_ids = retained_owned;
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
    if paragraph.name.namespace_uri != *parent_namespace || !paragraph.children.is_empty() {
        return None;
    }
    let mut expected = [
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
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect::<Vec<_>>();
    let mut id = None;
    for attribute in &paragraph.attributes {
        if attribute.name.namespace_uri.is_some() {
            return None;
        }
        if attribute.name.local_name.eq_ignore_ascii_case("id") {
            if id.replace(attribute.value.clone()).is_some() {
                return None;
            }
        } else if let Some(index) = expected
            .iter()
            .position(|(name, value)| name == &attribute.name.local_name && value == &attribute.value)
        {
            expected.remove(index);
        } else {
            return None;
        }
    }
    if !expected.is_empty() {
        return None;
    }
    let id = id?;
    Some((name.to_lowercase(), name, id))
}

fn safe_generated_removals(state: &PreservationState) -> BTreeSet<String> {
    state
        .generated_scene_arc_paragraph_ids
        .iter()
        .filter(|id| {
            let identity = ForeignIdentity {
                domain: ForeignIdDomain::Paragraph,
                value: (*id).clone(),
            };
            state.identities.iter().filter(|item| **item == identity).count() == 1
                && !state
                    .references
                    .iter()
                    .chain(&state.board_graph.references)
                    .any(|reference| reference.source == identity || reference.target == identity)
                && !state.anchors.iter().any(|anchor| {
                    matches!(&anchor.target, AnchorTarget::Foreign(target) if target == &identity)
                })
                && !external_id_attribute_use(&state.sections, id)
        })
        .cloned()
        .collect()
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
        assert!(text.contains("<CharacterArcBeat Name=\"Zoë\">"));
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
    fn scenes_are_independent_and_names_keep_first_unicode_trimmed_spelling() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ONE</Text></Paragraph><Paragraph Type="Character" id="c1"><Text> Zoë </Text></Paragraph><Paragraph Type="Character" id="c2"><Text>zoë</Text></Paragraph><Paragraph Type="Scene Heading" id="h2"><Text>EXT. TWO</Text></Paragraph><Paragraph Type="Character" id="c3"><Text>JO</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let text = String::from_utf8(output).unwrap_or_else(|_| unreachable!());
        assert_eq!(text.matches("<SceneProperties ").count(), 2);
        assert_eq!(text.matches("<CharacterArcBeat Name=\"Zoë\">").count(), 1);
        assert_eq!(text.matches("<CharacterArcBeat Name=\"JO\">").count(), 1);
        assert!(!text.contains("Name=\"zoë\""));
    }

    #[test]
    fn imported_scene_metadata_adds_and_reorders_while_retaining_matched_ids() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>Alpha</Text></Paragraph><Paragraph Type="Character" id="c2"><Text>Beta</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let state = project.preservation.as_ref().unwrap_or_else(|| unreachable!());
        let ids_before = state.generated_scene_arc_paragraph_ids.clone();
        assert_eq!(ids_before.len(), 2);
        assert!(ids_before.iter().all(|id| {
            uuid::Uuid::parse_str(id)
                .is_ok_and(|value| value.get_version() == Some(uuid::Version::Random))
        }));
        project.elements[1].text = "Beta".to_owned();
        project.elements[2].text = "Alpha".to_owned();
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let reopened = read_v6(&output).unwrap_or_else(|_| unreachable!());
        let mut rebased_project = reopened.project;
        carry_generated_ownership(&project, &mut rebased_project);
        let reopened_state = rebased_project.preservation.as_ref().unwrap_or_else(|| unreachable!());
        assert_eq!(reopened_state.generated_scene_arc_paragraph_ids, ids_before);
        for id in ids_before {
            assert!(reopened_state.identities.iter().any(|identity| identity.value == id));
        }
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
    fn removal_requires_in_session_generated_ownership_and_drops_its_identity() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let id = project.preservation.as_ref().and_then(|state| state.generated_scene_arc_paragraph_ids.iter().next().cloned()).unwrap_or_else(|| unreachable!());
        project.elements[1].text = "BETA".to_owned();
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let state = project.preservation.as_ref().unwrap_or_else(|| unreachable!());
        assert!(!state.generated_scene_arc_paragraph_ids.contains(&id));
        assert!(!state.identities.iter().any(|identity| identity.domain == ForeignIdDomain::Paragraph && identity.value == id));
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        assert!(!String::from_utf8(output).unwrap_or_else(|_| unreachable!()).contains(&id));
    }

    #[test]
    fn possible_source_reference_blocks_owned_entry_removal_atomically() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        assert!(reconcile_project(&mut project).unwrap_or(false));
        let id = project.preservation.as_ref().and_then(|state| state.generated_scene_arc_paragraph_ids.iter().next().cloned()).unwrap_or_else(|| unreachable!());
        let state = project.preservation.as_mut().unwrap_or_else(|| unreachable!());
        let PreservedNode::Element(document) = &mut state.sections[0].children[0] else {
            unreachable!();
        };
        let content = document.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Content" => Some(element),
            _ => None,
        }).unwrap_or_else(|| unreachable!());
        let mut vendor = element("VendorLink", Vec::new());
        vendor.attributes.push(attribute("Ref", &id));
        content.children.push(PreservedNode::Element(vendor));
        project.elements[1].text = "BETA".to_owned();
        let original = project.clone();
        assert_eq!(reconcile_project(&mut project), Err(SceneMetadataError::UnsupportedSceneMetadata));
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
    fn imported_unowned_stale_or_authored_nested_payload_safe_stops_atomically() {
        let source = r#"<FinalDraft Version="6"><Content><Paragraph Type="Scene Heading" id="h1"><SceneProperties Length="3/8" Page="7" Title="kept"><SceneArcBeats><CharacterArcBeat Name="OLD"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="arc-p"/></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" id="c1"><Text>NEW</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        let original = project.clone();
        assert_eq!(reconcile_project(&mut project), Err(SceneMetadataError::UnsupportedSceneMetadata));
        assert_eq!(project, original);

        let authored = source.replace("id=\"arc-p\"/>", "id=\"arc-p\"><Text>authored beat</Text></Paragraph>");
        let mut project = read_v6(authored.as_bytes()).unwrap_or_else(|_| unreachable!()).project;
        let original = project.clone();
        assert_eq!(reconcile_project(&mut project), Err(SceneMetadataError::UnsupportedSceneMetadata));
        assert_eq!(project, original);
    }
}
