use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::{
    beat_board::{
        BeatBoard, MAX_BOARD_DIMENSION, MAX_CARD_HEIGHT, MAX_CARD_WIDTH, MIN_CARD_HEIGHT,
        MIN_CARD_WIDTH,
    },
    dual_dialogue::{self, DualDialogueError},
    dialogue_alts::{
        DialogueAlternatives, MAX_DIALOGUE_ALT_RUN_VALUES_PER_PROJECT,
        MAX_DIALOGUE_ALT_ATTRIBUTE_BYTES_PER_PROJECT, MAX_DIALOGUE_ALT_TEXT_BYTES_PER_PROJECT,
        MAX_DIALOGUE_ALT_VARIANTS_PER_OWNER, MAX_DIALOGUE_ALT_VARIANTS_PER_PROJECT,
    },
    element::{
        ScreenplayElement, TextRun, MAX_PROVENANCE_STRING_BYTES, MAX_RUNS_PER_ELEMENT,
        MAX_RUN_ATTRIBUTES, MAX_RUN_ATTRIBUTE_BYTES,
    },
    identity::StableId,
    preservation::{
        BoardEntityKind, ForeignIdDomain, ForeignIdentity, PreservationError, PreservationState,
        ReferencePolicy,
    },
    project::Project,
    script_notes::{ScriptNoteIdentity, ScriptNotes},
    story_map::{
        StoryMap, StoryMapPlacement, MAX_STORY_MAP_LANES, MAX_STORY_MAP_STRING_BYTES,
        MAX_STORY_MAP_TARGET_PAGES,
    },
    ModelError, MAX_TEXT_BYTES,
};

pub const MAX_ELEMENTS: usize = 10_000;

/// Validates canonical screenplay element text, runs, identity, and provenance.
pub(crate) fn validate_element(element: &ScreenplayElement) -> Result<(), ModelError> {
    StableId::new(element.id.as_str())?;
    validate_text_runs(&element.text, &element.runs, false)?;
    if let Some(alternatives) = &element.dialogue_alternatives {
        validate_dialogue_alternatives(alternatives)?;
    }
    if element.source_provenance.as_ref().is_some_and(|source| {
        source
            .foreign_id
            .as_ref()
            .is_some_and(|value| value.len() > MAX_PROVENANCE_STRING_BYTES)
            || source
                .foreign_role
                .as_ref()
                .is_some_and(|value| value.len() > MAX_PROVENANCE_STRING_BYTES)
    }) {
        return Err(ModelError::StringTooLong);
    }
    Ok(())
}

fn validate_dialogue_alternatives(alternatives: &DialogueAlternatives) -> Result<(), ModelError> {
    if alternatives.total_count() > MAX_DIALOGUE_ALT_VARIANTS_PER_OWNER {
        return Err(ModelError::TooManyElements);
    }
    let mut ids = BTreeSet::new();
    ids.insert(alternatives.current);
    for alternative in &alternatives.inactive {
        if !ids.insert(alternative.id) {
            return Err(ModelError::DuplicateId);
        }
        validate_text_runs(&alternative.text, &alternative.runs, true)?;
    }
    Ok(())
}

fn validate_text_runs(text: &str, runs: &[TextRun], exact: bool) -> Result<(), ModelError> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(ModelError::TextTooLong);
    }
    if runs.len() > MAX_RUNS_PER_ELEMENT {
        return Err(ModelError::TooManyRuns);
    }
    let mut run_text = String::new();
    let mut attribute_count = 0usize;
    for run in runs {
        if run_text.len().saturating_add(run.text.len()) > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        run_text.push_str(&run.text);
        attribute_count = attribute_count.saturating_add(run.attributes.len());
        if run.attributes.iter().any(|attribute| {
            attribute.name.is_empty()
                || attribute.name.len() > MAX_RUN_ATTRIBUTE_BYTES
                || attribute.value.len() > MAX_RUN_ATTRIBUTE_BYTES
        }) {
            return Err(ModelError::StringTooLong);
        }
    }
    if attribute_count > MAX_RUN_ATTRIBUTES {
        return Err(ModelError::TooManyRunAttributes);
    }
    if (exact || !runs.is_empty()) && run_text != text {
        return Err(ModelError::InvalidRunProjection);
    }
    Ok(())
}

fn foreign_identity(domain: ForeignIdDomain, value: &str) -> Result<ForeignIdentity, ModelError> {
    ForeignIdentity::new(domain, value).map_err(ModelError::InvalidPreservationState)
}

fn validate_beat_board(
    board: &BeatBoard,
    story_map: &StoryMap,
    ids: &mut BTreeSet<StableId>,
    preservation: Option<&PreservationState>,
) -> Result<(), ModelError> {
    if board.width.raw() <= 0
        || board.height.raw() <= 0
        || board.width > MAX_BOARD_DIMENSION
        || board.height > MAX_BOARD_DIMENSION
    {
        return Err(ModelError::InvalidNumber);
    }
    if board.cards.len() > MAX_ELEMENTS {
        return Err(ModelError::TooManyElements);
    }
    let card_stable_ids = board.cards.iter().map(|card| &card.id).collect::<BTreeSet<_>>();
    let creation_ids = board.card_creation_order.iter().collect::<BTreeSet<_>>();
    if card_stable_ids.len() != board.cards.len()
        || creation_ids.len() != board.card_creation_order.len()
        || card_stable_ids != creation_ids
    {
        return Err(ModelError::DuplicateId);
    }
    let board_id = board
        .foreign_id
        .as_deref()
        .map(|value| foreign_identity(ForeignIdDomain::Board, value))
        .transpose()?;
    let mut card_ids = BTreeSet::new();
    let mut paragraph_count = 0usize;
    for card in &board.cards {
        StableId::new(card.id.as_str())?;
        if !ids.insert(card.id.clone()) {
            return Err(ModelError::DuplicateId);
        }
        let card_id = foreign_identity(ForeignIdDomain::ListItem, &card.foreign_id)?;
        if !card_ids.insert(card_id) {
            return Err(ModelError::DuplicateId);
        }
        if card.title.len() > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        if card.title.contains(['\r', '\n']) {
            return Err(ModelError::InvalidRunProjection);
        }
        if let Some(placement) = &card.story_map_placement {
            validate_story_map_placement(placement, story_map)?;
        }
        if card.rect.width < MIN_CARD_WIDTH
            || card.rect.width > MAX_CARD_WIDTH
            || card.rect.height < MIN_CARD_HEIGHT
            || card.rect.height > MAX_CARD_HEIGHT
            || !card.rect.contained_in(board.width, board.height)
        {
            return Err(ModelError::InvalidNumber);
        }
        if card.paragraphs.is_empty() {
            return Err(ModelError::InvalidRunProjection);
        }
        paragraph_count = paragraph_count.saturating_add(card.paragraphs.len());
        if paragraph_count > MAX_ELEMENTS {
            return Err(ModelError::TooManyElements);
        }
        for paragraph in &card.paragraphs {
            StableId::new(paragraph.id.as_str())?;
            if !ids.insert(paragraph.id.clone()) {
                return Err(ModelError::DuplicateId);
            }
            if let Some(value) = &paragraph.foreign_id {
                foreign_identity(ForeignIdDomain::Paragraph, value)?;
            }
            validate_text_runs(&paragraph.text, &paragraph.runs, true)?;
            if paragraph.text.is_empty() && !paragraph.runs.is_empty() {
                return Err(ModelError::InvalidRunProjection);
            }
        }
    }
    if let Some(preservation) = preservation {
        validate_beat_graph_coherence(board_id.as_ref(), &card_ids, preservation)?;
    }
    Ok(())
}

fn validate_story_map_placement(
    placement: &StoryMapPlacement,
    story_map: &StoryMap,
) -> Result<(), ModelError> {
    validate_bounded_xml_string(&placement.lane_uuid, MAX_STORY_MAP_STRING_BYTES)?;
    foreign_identity(ForeignIdDomain::Other("StoryMapLane".to_owned()), &placement.lane_uuid)?;
    let lane_matches = story_map
        .lanes
        .iter()
        .filter(|lane| lane.uuid == placement.lane_uuid)
        .count();
    let target_eighths = story_map
        .target_eighths()
        .ok_or(ModelError::InvalidNumber)?;
    let end = placement
        .checked_end_eighths()
        .filter(|_| placement.length_eighths > 0)
        .ok_or(ModelError::InvalidNumber)?;
    if lane_matches != 1 || end > target_eighths {
        return Err(ModelError::InvalidNumber);
    }
    Ok(())
}

fn validate_story_map(story_map: &StoryMap, beat_board: &BeatBoard) -> Result<(), ModelError> {
    if story_map.target_pages == 0
        || story_map.target_pages > MAX_STORY_MAP_TARGET_PAGES
        || story_map.target_eighths().is_none()
    {
        return Err(ModelError::InvalidNumber);
    }
    if story_map.lanes.len() > MAX_STORY_MAP_LANES {
        return Err(ModelError::TooManyElements);
    }

    let mut owned_foreign_values = BTreeSet::new();
    if let Some(board_id) = &story_map.foreign_id {
        validate_bounded_xml_string(board_id, MAX_STORY_MAP_STRING_BYTES)?;
        foreign_identity(ForeignIdDomain::Board, board_id)?;
        owned_foreign_values.insert(board_id.as_str());
        if beat_board.foreign_id.as_deref() == Some(board_id.as_str()) {
            return Err(ModelError::DuplicateId);
        }
    }

    let mut aggregate_text_bytes = story_map.foreign_id.as_ref().map_or(0usize, String::len);
    for lane in &story_map.lanes {
        validate_bounded_xml_string(&lane.uuid, MAX_STORY_MAP_STRING_BYTES)?;
        foreign_identity(
            ForeignIdDomain::Other("StoryMapLane".to_owned()),
            &lane.uuid,
        )?;
        validate_bounded_xml_string(&lane.label, MAX_STORY_MAP_STRING_BYTES)?;
        validate_bounded_xml_string(&lane.lane_type, MAX_STORY_MAP_STRING_BYTES)?;
        if lane.lane_type.is_empty() {
            return Err(ModelError::InvalidRunProjection);
        }
        if !owned_foreign_values.insert(lane.uuid.as_str()) {
            return Err(ModelError::DuplicateId);
        }
        aggregate_text_bytes = aggregate_text_bytes
            .checked_add(lane.uuid.len())
            .and_then(|bytes| bytes.checked_add(lane.label.len()))
            .and_then(|bytes| bytes.checked_add(lane.lane_type.len()))
            .filter(|bytes| *bytes <= MAX_TEXT_BYTES)
            .ok_or(ModelError::TextTooLong)?;
    }

    // Native identities are source-owned strings. Reject overlap with the
    // existing semantic Beat board/ListItems so an owner cannot be ambiguous.
    let mut beat_foreign_values = BTreeSet::new();
    if let Some(id) = &beat_board.foreign_id {
        beat_foreign_values.insert(id.as_str());
    }
    for card in &beat_board.cards {
        beat_foreign_values.insert(card.foreign_id.as_str());
        for paragraph in &card.paragraphs {
            if let Some(id) = paragraph.foreign_id.as_deref() {
                beat_foreign_values.insert(id);
            }
        }
    }
    if owned_foreign_values
        .iter()
        .any(|identity| beat_foreign_values.contains(identity))
    {
        return Err(ModelError::DuplicateId);
    }
    Ok(())
}

fn is_xml_compatible(value: &str) -> bool {
    value.chars().all(|character| {
        matches!(character, '\u{9}' | '\u{A}' | '\u{D}')
            || ('\u{20}'..='\u{D7FF}').contains(&character)
            || ('\u{E000}'..='\u{FFFD}').contains(&character)
            || ('\u{10000}'..='\u{10FFFF}').contains(&character)
    })
}

fn validate_bounded_xml_string(value: &str, max_bytes: usize) -> Result<(), ModelError> {
    if value.len() > max_bytes {
        return Err(ModelError::StringTooLong);
    }
    if !is_xml_compatible(value) {
        return Err(ModelError::InvalidRunProjection);
    }
    Ok(())
}

fn validate_script_notes(
    notes: &ScriptNotes,
    project: &Project,
    ids: &mut BTreeSet<StableId>,
) -> Result<(), ModelError> {
    if notes.len() > MAX_ELEMENTS {
        return Err(ModelError::TooManyElements);
    }
    let body_length = crate::body_offsets::flat_document_len(project).ok_or(ModelError::InvalidNumber)?;
    let mut total_values = notes.len();
    let mut source_note_ordinals = BTreeSet::new();
    let mut durable_note_ids = BTreeSet::new();
    let mut durable_paragraph_ids = BTreeSet::new();

    for note in notes.iter() {
        StableId::new(note.id.as_str())?;
        if !ids.insert(note.id.clone()) {
            return Err(ModelError::DuplicateId);
        }
        if note.source_note_ordinal.is_some_and(|ordinal| !source_note_ordinals.insert(ordinal)) {
            return Err(ModelError::DuplicateId);
        }
        if let Some(identity) = note.durable_identity() {
            if !durable_note_ids.insert(identity) {
                return Err(ModelError::DuplicateId);
            }
        }
        if note.numeric_id.as_deref().is_some_and(|value| {
            !value.is_empty() && !value.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            return Err(ModelError::InvalidNumber);
        }
        for value in [
            note.numeric_id.as_deref(),
            note.ref_id.as_deref(),
            Some(note.color.as_str()),
            note.note_type.as_deref(),
            Some(note.writer_id.as_str()),
            Some(note.writer_name.as_str()),
            Some(note.date_time.as_str()),
            Some(note.date_modified.as_str()),
        ]
        .into_iter()
        .flatten()
        {
            validate_bounded_xml_string(value, crate::element::MAX_PROVENANCE_STRING_BYTES)?;
        }
        if note.title.len() > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        if !is_xml_compatible(&note.title) {
            return Err(ModelError::InvalidRunProjection);
        }
        if note.title.contains(['\r', '\n']) {
            return Err(ModelError::InvalidRunProjection);
        }
        if note.note_type.as_deref().is_some_and(str::is_empty) {
            return Err(ModelError::InvalidRunProjection);
        }
        if note.paragraphs.is_empty() {
            return Err(ModelError::InvalidRunProjection);
        }
        if note.range.start() > note.range.end()
            || note.range.start() > body_length
            || note.range.end() > body_length
            || crate::body_offsets::flat_offset_to_position(project, note.range.start()).is_none()
            || crate::body_offsets::flat_offset_to_position(project, note.range.end()).is_none()
        {
            return Err(ModelError::InvalidUtf8Boundary);
        }

        total_values = total_values
            .checked_add(note.paragraphs.len())
            .ok_or(ModelError::TooManyElements)?;
        if total_values > MAX_ELEMENTS {
            return Err(ModelError::TooManyElements);
        }
        let mut source_paragraph_ordinals = BTreeSet::new();
        for paragraph in &note.paragraphs {
            StableId::new(paragraph.id.as_str())?;
            if !ids.insert(paragraph.id.clone()) {
                return Err(ModelError::DuplicateId);
            }
            if paragraph.source_paragraph_ordinal.is_some_and(|ordinal| {
                !source_paragraph_ordinals.insert(ordinal)
            }) {
                return Err(ModelError::DuplicateId);
            }
            if let Some(durable_id) = paragraph.durable_id.as_deref() {
                validate_bounded_xml_string(
                    durable_id,
                    crate::element::MAX_PROVENANCE_STRING_BYTES,
                )?;
                if !durable_id.is_empty() && !durable_paragraph_ids.insert(durable_id) {
                    return Err(ModelError::DuplicateId);
                }
            }
            validate_text_runs(&paragraph.text, &paragraph.runs, true)?;
            if paragraph.text.is_empty() && !paragraph.runs.is_empty() {
                return Err(ModelError::InvalidRunProjection);
            }
        }
    }
    Ok(())
}

/// Checks graph facts already known without assuming source-node ownership.
/// WO-002 must bind the winning supported nodes and paragraph identities; the
/// later output projection must account for new or removed graph registrations.
fn validate_beat_graph_coherence(
    board_id: Option<&ForeignIdentity>,
    card_ids: &BTreeSet<ForeignIdentity>,
    preservation: &PreservationState,
) -> Result<(), ModelError> {
    if board_id.is_none() && card_ids.is_empty() {
        return Ok(());
    }
    let entities = preservation
        .board_graph
        .entities
        .iter()
        .map(|entity| (&entity.identity, entity.kind))
        .collect::<BTreeMap<_, _>>();
    let graph_error = || ModelError::InvalidPreservationState(PreservationError::DanglingReference);
    if board_id
        .and_then(|id| entities.get(id))
        .is_some_and(|kind| *kind != BoardEntityKind::DisplayBoard)
        || card_ids.iter().any(|id| {
            entities
                .get(id)
                .is_some_and(|kind| *kind != BoardEntityKind::ListItem)
        })
    {
        return Err(graph_error());
    }
    let references = preservation
        .board_graph
        .references
        .iter()
        .filter(|reference| reference.policy == ReferencePolicy::RequiredInternal)
        .map(|reference| (&reference.source, &reference.target))
        .collect::<BTreeSet<_>>();
    if let Some(board_id) = board_id.filter(|id| entities.contains_key(*id)) {
        for card_id in card_ids.iter().filter(|id| entities.contains_key(*id)) {
            if !references.contains(&(board_id, card_id)) {
                return Err(graph_error());
            }
        }
    }
    Ok(())
}

pub fn validate_layout_inputs(
    inputs: &crate::layout_inputs::LayoutInputs,
) -> Result<(), ModelError> {
    if inputs.name.len() > crate::layout_inputs::MAX_LAYOUT_NAME_BYTES
        || inputs.format_version == 0
        || inputs.paper_width == 0
        || inputs.paper_height == 0
        || inputs.boundary_gap > 1_000
        || inputs.font_size == 0
        || inputs.font_size > 72
    {
        return Err(ModelError::InvalidNumber);
    }
    Ok(())
}

pub(crate) fn validate_document_page_layout(
    page_layout: &crate::layout_inputs::DocumentPageLayout,
) -> Result<(), ModelError> {
    let fallback_inputs = crate::layout_inputs::LayoutInputs::default();
    let fallback_horizontal_margins = u64::from(fallback_inputs.margin_left)
        .saturating_add(u64::from(fallback_inputs.margin_right));
    if page_layout.paper_width == 0
        || page_layout.paper_height == 0
        || fallback_horizontal_margins >= u64::from(page_layout.paper_width)
        || u64::from(page_layout.margin_top).saturating_add(u64::from(page_layout.margin_bottom))
            >= u64::from(page_layout.paper_height)
        || page_layout
            .header_margin
            .is_some_and(|margin| margin.raw() < 0)
        || page_layout
            .footer_margin
            .is_some_and(|margin| margin.raw() < 0)
    {
        return Err(ModelError::InvalidNumber);
    }
    Ok(())
}

impl Project {
    pub fn validate(&self) -> Result<(), ModelError> {
        validate_document_page_layout(self.page_layout())?;
        if self.elements.len() > MAX_ELEMENTS || self.title_page.len() > MAX_ELEMENTS {
            return Err(ModelError::TooManyElements);
        }
        self.element_definitions().validate()?;
        let mut ids = BTreeSet::new();
        let mut element_ids = BTreeSet::new();
        let mut dialogue_alt_variant_count = 0usize;
        let mut dialogue_alt_text_bytes = 0usize;
        let mut dialogue_alt_run_values = 0usize;
        let mut dialogue_alt_attribute_bytes = 0usize;
        for item in &self.title_page {
            item.validate()?;
            if let Some(key) = item.element_type() {
                if self.element_definitions().get(&key).is_none() {
                    return Err(ModelError::MissingElementDefinition(key));
                }
            }
            if !ids.insert(item.id.clone()) {
                return Err(ModelError::DuplicateId);
            }
            element_ids.insert(item.id.clone());
        }
        for item in &self.elements {
            validate_element(item)?;
            if let Some(alternatives) = &item.dialogue_alternatives {
                dialogue_alt_variant_count = dialogue_alt_variant_count
                    .checked_add(alternatives.total_count())
                    .filter(|count| *count <= MAX_DIALOGUE_ALT_VARIANTS_PER_PROJECT)
                    .ok_or(ModelError::TooManyElements)?;
                dialogue_alt_text_bytes = dialogue_alt_text_bytes
                    .checked_add(item.text.len())
                    .filter(|count| *count <= MAX_DIALOGUE_ALT_TEXT_BYTES_PER_PROJECT)
                    .ok_or(ModelError::TextTooLong)?;
                dialogue_alt_run_values = dialogue_alt_run_values
                    .checked_add(item.runs.len())
                    .filter(|count| *count <= MAX_DIALOGUE_ALT_RUN_VALUES_PER_PROJECT)
                    .ok_or(ModelError::TooManyRuns)?;
                let active_attribute_count = item.runs.iter().try_fold(0usize, |count, run| {
                    count.checked_add(run.attributes.len())
                }).ok_or(ModelError::TooManyRunAttributes)?;
                dialogue_alt_run_values = dialogue_alt_run_values
                    .checked_add(active_attribute_count)
                    .filter(|count| *count <= MAX_DIALOGUE_ALT_RUN_VALUES_PER_PROJECT)
                    .ok_or(ModelError::TooManyRunAttributes)?;
                for run in &item.runs {
                    for attribute in &run.attributes {
                        dialogue_alt_attribute_bytes = dialogue_alt_attribute_bytes
                            .checked_add(attribute.name.len())
                            .and_then(|count| count.checked_add(attribute.value.len()))
                            .filter(|count| *count <= MAX_DIALOGUE_ALT_ATTRIBUTE_BYTES_PER_PROJECT)
                            .ok_or(ModelError::StringTooLong)?;
                    }
                }
                for alternative in &alternatives.inactive {
                    dialogue_alt_text_bytes = dialogue_alt_text_bytes
                        .checked_add(alternative.text.len())
                        .filter(|count| *count <= MAX_DIALOGUE_ALT_TEXT_BYTES_PER_PROJECT)
                        .ok_or(ModelError::TextTooLong)?;
                    dialogue_alt_run_values = dialogue_alt_run_values
                        .checked_add(alternative.runs.len())
                        .filter(|count| *count <= MAX_DIALOGUE_ALT_RUN_VALUES_PER_PROJECT)
                        .ok_or(ModelError::TooManyRuns)?;
                    let attribute_count = alternative.runs.iter().try_fold(0usize, |count, run| {
                        count.checked_add(run.attributes.len())
                    }).ok_or(ModelError::TooManyRunAttributes)?;
                    dialogue_alt_run_values = dialogue_alt_run_values
                        .checked_add(attribute_count)
                        .filter(|count| *count <= MAX_DIALOGUE_ALT_RUN_VALUES_PER_PROJECT)
                        .ok_or(ModelError::TooManyRunAttributes)?;
                    for run in &alternative.runs {
                        for attribute in &run.attributes {
                            dialogue_alt_attribute_bytes = dialogue_alt_attribute_bytes
                                .checked_add(attribute.name.len())
                                .and_then(|count| count.checked_add(attribute.value.len()))
                                .filter(|count| *count <= MAX_DIALOGUE_ALT_ATTRIBUTE_BYTES_PER_PROJECT)
                                .ok_or(ModelError::StringTooLong)?;
                        }
                    }
                }
            }
            let key = item.element_type();
            if self.element_definitions().get(&key).is_none() {
                return Err(ModelError::MissingElementDefinition(key));
            }
            if !ids.insert(item.id.clone()) {
                return Err(ModelError::DuplicateId);
            }
            element_ids.insert(item.id.clone());
        }
        dual_dialogue::validate_groups(self).map_err(ModelError::InvalidDualDialogue)?;
        for group in &self.dual_dialogue_groups {
            if !ids.insert(group.id.clone()) {
                return Err(ModelError::DuplicateId);
            }
        }
        validate_story_map(self.story_map(), self.beat_board())?;
        validate_beat_board(
            self.beat_board(),
            self.story_map(),
            &mut ids,
            self.preservation.as_ref(),
        )?;
        validate_script_notes(self.script_notes(), self, &mut ids)?;
        if self
            .direct_paragraph_overrides
            .keys()
            .any(|id| !self.elements.iter().any(|element| &element.id == id))
        {
            return Err(ModelError::InvalidParagraphOverride);
        }
        let group_ids = self
            .dual_dialogue_groups
            .iter()
            .map(|group| group.id.clone())
            .collect::<BTreeSet<_>>();
        dual_dialogue::validate_wrapper_overrides(self).map_err(ModelError::InvalidDualDialogue)?;
        if let Some(preservation) = &self.preservation {
            preservation
                .validate_project_elements(&element_ids, &group_ids)
                .map_err(ModelError::InvalidPreservationState)?;
            let definition_keys = self
                .element_definitions()
                .iter()
                .map(|definition| definition.key.clone())
                .collect::<BTreeSet<_>>();
            preservation
                .validate_element_definition_anchors(&definition_keys)
                .map_err(ModelError::InvalidPreservationState)?;
            for group in &self.dual_dialogue_groups {
                let wrapper_anchor = preservation.dual_dialogue_wrapper_anchor(&group.id);
                if wrapper_anchor.as_ref() != group.wrapper.preservation_anchor.as_ref() {
                    return Err(ModelError::InvalidDualDialogue(
                        DualDialogueError::InvalidWrapperOwnership,
                    ));
                }
            }
        } else if self
            .dual_dialogue_groups
            .iter()
            .any(|group| group.wrapper.preservation_anchor.is_some())
        {
            return Err(ModelError::InvalidDualDialogue(
                DualDialogueError::InvalidWrapperOwnership,
            ));
        }
        Ok(())
    }

    /// Validates one ordinary-edit candidate against a complete valid baseline.
    /// `Ok(false)` means scope preconditions were not established and the caller
    /// must perform complete validation.
    pub fn validate_scoped_single_element_edit(
        &mut self,
        baseline: &Project,
        element_index: usize,
    ) -> Result<bool, ModelError> {
        if self.elements.len() != baseline.elements.len()
            || self.title_page.len() != baseline.title_page.len()
            || self.dual_dialogue_groups != baseline.dual_dialogue_groups
            || self.dual_dialogue_paragraph_overrides != baseline.dual_dialogue_paragraph_overrides
            || self.direct_paragraph_overrides != baseline.direct_paragraph_overrides
            || self.element_definitions() != baseline.element_definitions()
            || self.page_layout() != baseline.page_layout()
            || self.beat_board() != baseline.beat_board()
            || self.script_notes() != baseline.script_notes()
            || !Arc::ptr_eq(&self.layout_identity, &baseline.layout_identity)
        {
            return Ok(false);
        }
        let Some(before) = baseline.elements.get(element_index) else {
            return Ok(false);
        };
        let Some(after) = self.elements.get(element_index) else {
            return Ok(false);
        };
        if before.id != after.id {
            return Ok(false);
        }
        if before.dialogue_alternatives != after.dialogue_alternatives {
            return Ok(false);
        }
        if self.title_page != baseline.title_page
            || self
                .elements
                .iter()
                .zip(&baseline.elements)
                .enumerate()
                .any(|(index, (candidate, original))| {
                    index != element_index
                        && !Arc::ptr_eq(candidate, original)
                        && candidate != original
                })
        {
            return Ok(false);
        }
        validate_element(after)?;
        let updated_source_text_bytes = match (&self.preservation, &baseline.preservation) {
            (Some(candidate), Some(valid_baseline)) => {
                if !candidate.has_managed_paragraph_index()
                    || !valid_baseline.has_managed_paragraph_index()
                    || !candidate.scoped_global_invariants_match(valid_baseline)
                {
                    return Ok(false);
                }
                Some(
                    candidate
                        .validate_scoped_managed_edit(valid_baseline, after)
                        .map_err(ModelError::InvalidPreservationState)?,
                )
            }
            (None, None) => None,
            _ => return Ok(false),
        };
        if let Some(byte_count) = updated_source_text_bytes {
            if let Some(preservation) = self.preservation.as_mut() {
                preservation.set_validated_source_text_bytes(byte_count);
            }
        }
        Ok(true)
    }
}
