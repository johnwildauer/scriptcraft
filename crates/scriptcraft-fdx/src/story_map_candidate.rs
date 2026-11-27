//! Detached durable identity finalization for Story Map output candidates.

use crate::diagnostics::{map_model_error, FdxError};
use scriptcraft_model::preservation::PreservedNode;
use scriptcraft_model::{Project, StoryMapLane};
use std::collections::BTreeSet;

fn reserve_source_ids(nodes: &[PreservedNode], reserved: &mut BTreeSet<String>) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            reserved.extend(element.attributes.iter().filter(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
            }).map(|attribute| attribute.value.clone()));
            reserve_source_ids(&element.children, reserved);
        }
    }
}

fn reserve_project_ids(project: &Project, reserved: &mut BTreeSet<String>) {
    reserved.extend(project.elements.iter().map(|element| element.id.as_str().to_owned()));
    reserved.extend(project.title_page.iter().map(|element| element.id.as_str().to_owned()));
    reserved.extend(project.dual_dialogue_groups.iter().map(|group| group.id.as_str().to_owned()));
    reserved.extend(project.elements.iter().filter_map(|element| {
        element.source_provenance.as_ref().and_then(|source| source.foreign_id.clone())
    }));
    reserved.extend(project.title_page.iter().filter_map(|element| {
        element.source_provenance.as_ref().and_then(|source| source.foreign_id.clone())
    }));
    reserved.extend(project.dual_dialogue_groups.iter().filter_map(|group| {
        group.wrapper.outer_foreign_id.clone()
    }));
    let board = project.beat_board();
    reserved.extend(board.foreign_id.iter().cloned());
    for card in &board.cards {
        reserved.insert(card.id.as_str().to_owned());
        reserved.insert(card.foreign_id.clone());
        reserved.extend(card.paragraphs.iter().map(|paragraph| paragraph.id.as_str().to_owned()));
        reserved.extend(card.paragraphs.iter().filter_map(|paragraph| paragraph.foreign_id.clone()));
    }
    for note in project.script_notes().iter() {
        reserved.insert(note.id.as_str().to_owned());
        reserved.extend(note.ref_id.iter().cloned());
        reserved.extend(note.paragraphs.iter().filter_map(|paragraph| paragraph.durable_id.clone()));
    }
    if let Some(id) = &project.story_map().foreign_id { reserved.insert(id.clone()); }
    reserved.extend(project.story_map().lanes.iter().map(|lane| lane.uuid.clone()));
}

fn allocate_uuid(reserved: &mut BTreeSet<String>) -> Result<String, FdxError> {
    for _ in 0..16 {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| FdxError::InvalidValue)?;
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let candidate = uuid::Uuid::from_bytes(bytes).hyphenated().to_string();
        if candidate.bytes().all(|byte| !byte.is_ascii_uppercase())
            && uuid::Uuid::parse_str(&candidate).is_ok_and(|value| {
                value.hyphenated().to_string() == candidate
                    && value.get_version() == Some(uuid::Version::Random)
                    && value.get_variant() == uuid::Variant::RFC4122
            })
            && reserved.insert(candidate.clone())
        {
            return Ok(candidate);
        }
    }
    Err(FdxError::InvalidValue)
}

/// Finalize the durable Story Map board and required default lanes on one
/// detached Project. The caller's Project and retained source bytes are intact.
pub fn finalize_story_map_candidate(project: &Project) -> Result<Project, FdxError> {
    let mut candidate = project.clone();
    let mut story_map = candidate.story_map().clone();
    if story_map.foreign_id.is_some() && !story_map.lanes.is_empty() {
        candidate.validate().map_err(map_model_error)?;
        return Ok(candidate);
    }

    let mut reserved = BTreeSet::new();
    reserve_project_ids(project, &mut reserved);
    if let Some(state) = &project.preservation {
        state.validate().map_err(crate::diagnostics::map_preservation_error)?;
        reserved.extend(state.identities.iter().map(|identity| identity.value.clone()));
        reserved.extend(state.board_graph.entities.iter().map(|entity| entity.identity.value.clone()));
        for reference in state.references.iter().chain(&state.board_graph.references) {
            reserved.insert(reference.source.value.clone());
            reserved.insert(reference.target.value.clone());
        }
        for section in &state.sections { reserve_source_ids(&section.children, &mut reserved); }
    }

    if story_map.foreign_id.is_none() {
        story_map.foreign_id = Some(allocate_uuid(&mut reserved)?);
    }
    if story_map.lanes.is_empty() {
        if candidate.beat_board().cards.iter().any(|card| card.story_map_placement.is_some()) {
            return Err(FdxError::InvalidValue);
        }
        story_map.lanes = [ ("Outline 1", 7_i64), ("Outline 2", 5_i64) ]
            .into_iter()
            .map(|(label, level)| {
                Ok(StoryMapLane {
                    uuid: allocate_uuid(&mut reserved)?,
                    label: label.to_owned(),
                    level,
                    lane_type: label.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, FdxError>>()?;
    }
    candidate.replace_story_map(story_map).map_err(map_model_error)?;
    candidate.validate().map_err(map_model_error)?;

    Ok(candidate)
}
