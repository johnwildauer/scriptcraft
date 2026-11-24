//! Detached allocation of durable Beat identities for later output transactions.

use crate::diagnostics::{map_model_error, map_paragraph_id_error, map_preservation_error, FdxError};
use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment, ParagraphLeading, ParagraphSpacing};
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::preservation::{ForeignIdDomain, PreservedNode};
use scriptcraft_model::{BeatBoard, Project, RunAttribute};
use std::collections::BTreeSet;

// WO-003's Beat sample uses an explicit empty Style on an unstyled run.
const DEFAULT_BEAT_RUN_ATTRIBUTES: [(&str, &str); 5] = [
    ("AdornmentStyle", "0"), ("Font", "Arial"), ("RevisionID", "0"),
    ("Size", "12"), ("Style", ""),
];

fn default_beat_run_attributes() -> Vec<RunAttribute> {
    DEFAULT_BEAT_RUN_ATTRIBUTES.into_iter().map(|(name, value)| RunAttribute {
        name: name.into(), value: value.into(),
    }).collect()
}

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

/// Return a detached candidate with missing Beat identities allocated once.
/// The caller's Project and original retained bytes are never modified.
///
/// Writer and session paths must use this same enriched candidate for
/// serialization and semantic comparison.
pub fn finalize_beat_board_candidate(project: &Project) -> Result<Project, FdxError> {
    prepare_beat_board_candidate(project, project.beat_board().clone())
}

/// Prepare a complete semantic board edit on a detached Project. Missing card,
/// board and paragraph IDs are assigned before the board is installed, so new
/// owners have durable identities from the moment their candidate is committed.
/// Retained XML is projected later by the writer from this semantic candidate.
pub fn prepare_beat_board_candidate(project: &Project, mut board: BeatBoard) -> Result<Project, FdxError> {
    let paragraph_count = board.cards.iter().try_fold(0usize, |count, card| {
        count.checked_add(card.paragraphs.len()).ok_or(FdxError::ResourceLimit)
    })?;
    if board.cards.len() > scriptcraft_model::validation::MAX_ELEMENTS
        || paragraph_count > scriptcraft_model::validation::MAX_ELEMENTS {
        return Err(FdxError::ResourceLimit);
    }
    let mut reserved = BTreeSet::new();
    for provenance in project.elements.iter().filter_map(|item| item.source_provenance.as_ref())
        .chain(project.title_page.iter().filter_map(|item| item.source_provenance.as_ref())) {
        if let Some(id) = &provenance.foreign_id { reserved.insert(id.clone()); }
    }
    reserved.extend(project.dual_dialogue_groups.iter().filter_map(|group| group.wrapper.outer_foreign_id.clone()));
    if let Some(state) = &project.preservation {
        state.validate().map_err(map_preservation_error)?;
        reserved.extend(state.identities.iter().map(|identity| identity.value.clone()));
        reserved.extend(state.board_graph.entities.iter().map(|entity| entity.identity.value.clone()));
        for reference in state.references.iter().chain(&state.board_graph.references) {
            reserved.insert(reference.source.value.clone());
            reserved.insert(reference.target.value.clone());
        }
        for section in &state.sections { reserve_source_ids(&section.children, &mut reserved); }
    }
    // The live graph can already contain generated semantic Beat owners. They
    // reserve IDs for future allocation, but are not retained XML owners. Do
    // not misclassify them as source collisions during save-time binding.
    let mut non_beat_ids = reserved.clone();
    if let Some(state) = &project.preservation {
        let mut source_ids = BTreeSet::new();
        for section in &state.sections { reserve_source_ids(&section.children, &mut source_ids); }
        let semantic_paragraph_ids = project.beat_board().cards.iter().flat_map(|card| &card.paragraphs)
            .filter_map(|paragraph| paragraph.foreign_id.as_deref()).collect::<BTreeSet<_>>();
        for identity in &state.identities {
            if identity.domain == ForeignIdDomain::Paragraph
                && semantic_paragraph_ids.contains(identity.value.as_str())
                && !source_ids.contains(&identity.value)
                && !state.references.iter().chain(&state.board_graph.references)
                    .any(|reference| reference.source == *identity || reference.target == *identity) {
                non_beat_ids.remove(&identity.value);
            }
        }
        for entity in &state.board_graph.entities {
            let managed = project.beat_board().foreign_id.as_deref() == Some(entity.identity.value.as_str())
                || project.beat_board().cards.iter().any(|card| card.foreign_id == entity.identity.value);
            if managed && entity.anchor.is_none() && !source_ids.contains(&entity.identity.value)
                && !state.identities.iter().any(|id| id.value == entity.identity.value)
                && !state.references.iter().any(|r| r.source.value == entity.identity.value || r.target.value == entity.identity.value) {
                non_beat_ids.remove(&entity.identity.value);
            }
        }
    }
    let before_allocation = board.clone();
    // Reserve the previous semantic board too: a replacement candidate may
    // have removed an owner whose ID must not be allocated to a new owner.
    reserved.extend(project.beat_board().foreign_id.iter().cloned());
    for card in &project.beat_board().cards {
        if !card.foreign_id.is_empty() { reserved.insert(card.foreign_id.clone()); }
        reserved.extend(card.paragraphs.iter().filter_map(|paragraph| paragraph.foreign_id.clone()));
    }
    if let Some(id) = &board.foreign_id { reserved.insert(id.clone()); }
    for card in &board.cards {
        if !card.foreign_id.is_empty() { reserved.insert(card.foreign_id.clone()); }
        for paragraph in &card.paragraphs {
            if let Some(id) = &paragraph.foreign_id { reserved.insert(id.clone()); }
        }
    }
    let mut allocator = ParagraphIdAllocator::new(reserved);
    if board.foreign_id.is_none() {
        board.foreign_id = Some(allocator.allocate().map_err(map_paragraph_id_error)?);
    }
    for card in &mut board.cards {
        if card.foreign_id.is_empty() {
            card.foreign_id = allocator.allocate().map_err(map_paragraph_id_error)?;
        }
        for paragraph in &mut card.paragraphs {
            if paragraph.foreign_id.is_none() {
                paragraph.foreign_id = Some(allocator.allocate().map_err(map_paragraph_id_error)?);
            }
        }
    }
    // Reject duplicate candidate identities even across Beat domains. Existing
    // source identities are allowed only through the ownership mapping below;
    // in a source-free candidate none may collide with screenplay identities.
    let mut beat_ids = BTreeSet::new();
    let ids = board.foreign_id.iter().chain(board.cards.iter().flat_map(|card| {
        std::iter::once(&card.foreign_id).chain(card.paragraphs.iter().filter_map(|p| p.foreign_id.as_ref()))
    }));
    for id in ids {
        if !beat_ids.insert(id.clone()) || (project.preservation.is_none() && non_beat_ids.contains(id)) {
            return Err(FdxError::AmbiguousIdentity);
        }
    }
    let mut candidate = project.clone();
    // Registry edits may have left old definition anchors in retained source.
    // Apply the existing detached settings projection before the model's whole-
    // candidate validation, just as the writer does for its validation copy.
    if candidate.preservation.is_some() {
        crate::element_settings::reconcile_project_element_settings(&mut candidate)?;
    }
    let original = match candidate.preservation.as_ref() {
        Some(state) => Some(crate::reader::read_v6(&state.source.original_bytes)?),
        None => None,
    };
    if let Some(state) = candidate.preservation.as_mut() {
        crate::beat_board_output::bind_candidate_paragraph_identities(
            state, &before_allocation, &board, &non_beat_ids,
            original.as_ref().ok_or(FdxError::AmbiguousIdentity)?.project.beat_board(),
        )?;
    }
    // Binding has registered every attributable retained id-less paragraph.
    // Only identities absent from that registry describe generated shells;
    // retained sparse settings must remain sparse.
    let retained_paragraphs = candidate.preservation.as_ref().map(|state| {
        state.identities.iter().filter(|id| id.domain == ForeignIdDomain::Paragraph)
            .map(|id| id.value.clone()).collect::<BTreeSet<_>>()
    }).unwrap_or_default();
    let paragraphs_with_source_text = candidate.preservation.as_ref()
        .map(crate::beat_board_output::paragraphs_with_source_text).unwrap_or_default();
    for paragraph in board.cards.iter_mut().flat_map(|card| &mut card.paragraphs) {
        let retained = paragraph.foreign_id.as_ref().is_some_and(|id| retained_paragraphs.contains(id));
        if !retained {
            let settings = &mut paragraph.direct_settings;
            let spec = &mut settings.paragraph;
            spec.alignment.get_or_insert(ParagraphAlignment::Left);
            spec.first_indent.get_or_insert(FixedPoint::from_raw(0));
            spec.leading.get_or_insert(ParagraphLeading::Regular);
            spec.left_indent.get_or_insert(FixedPoint::from_raw(0));
            spec.right_indent.get_or_insert(FixedPoint::from_raw(21_672));
            spec.space_before.get_or_insert(FixedPoint::from_raw(0));
            spec.spacing.get_or_insert(ParagraphSpacing::Single);
            settings.starts_new_page.get_or_insert(false);
        }
        // Do not manufacture a run for an invalid Beat projection. Unlike
        // legacy screenplay paragraphs, nonempty Beat text requires runs.
        if (paragraph.text.is_empty() && !paragraph.runs.is_empty())
            || (!paragraph.text.is_empty() && paragraph.runs.is_empty()) {
            return Err(FdxError::InvalidValue);
        }
        // Empty Text occurrences have no semantic run on import. Do not emit
        // empty runs alongside authored text in the finalized projection.
        paragraph.runs.retain(|run| !run.text.is_empty());
        // Generated text, including the first text of an empty retained
        // paragraph, receives default typing attributes.
        for run in &mut paragraph.runs {
            if retained && paragraph.foreign_id.as_ref().is_some_and(|id| paragraphs_with_source_text.contains(id)) { continue; }
            if run.attributes.is_empty() { run.attributes = default_beat_run_attributes(); }
        }
    }
    candidate.replace_beat_board(board).map_err(map_model_error)?;
    Ok(candidate)
}
