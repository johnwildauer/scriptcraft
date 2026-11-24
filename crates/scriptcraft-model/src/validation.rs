use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::{
    beat_board::{
        BeatBoard, MAX_BOARD_DIMENSION, MAX_CARD_HEIGHT, MAX_CARD_WIDTH, MIN_CARD_HEIGHT,
        MIN_CARD_WIDTH,
    },
    dual_dialogue::{self, DualDialogueError},
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
    ModelError, MAX_TEXT_BYTES,
};

pub const MAX_ELEMENTS: usize = 10_000;

/// Validates canonical screenplay element text, runs, identity, and provenance.
pub(crate) fn validate_element(element: &ScreenplayElement) -> Result<(), ModelError> {
    StableId::new(element.id.as_str())?;
    validate_text_runs(&element.text, &element.runs, false)?;
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
        ids.extend(
            self.dual_dialogue_groups
                .iter()
                .map(|group| group.id.clone()),
        );
        validate_beat_board(self.beat_board(), &mut ids, self.preservation.as_ref())?;
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
