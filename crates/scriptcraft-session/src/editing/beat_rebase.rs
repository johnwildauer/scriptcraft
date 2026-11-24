//! Durable Beat identity mapping for a verified save candidate and its reread.

use super::EditError;
use scriptcraft_model::preservation::MAX_SOURCE_BYTES;
use scriptcraft_model::validation::MAX_ELEMENTS;
use scriptcraft_model::{BeatBoard, BeatCard, BeatParagraph, Project, StableId};
use std::collections::{BTreeMap, BTreeSet};

type Cards<'a> = BTreeMap<&'a str, &'a BeatCard>;
type Paragraphs<'a> = BTreeMap<&'a str, (&'a str, &'a BeatParagraph)>;

fn index_board(board: &BeatBoard) -> Result<(Cards<'_>, Paragraphs<'_>), EditError> {
    let board_id = board.foreign_id.as_deref().filter(|id| !id.is_empty())
        .ok_or(EditError::InvalidSelection)?;
    if board.cards.len() > MAX_ELEMENTS { return Err(EditError::InvalidSelection); }
    let mut ids = BTreeSet::new();
    ids.insert(board_id);
    let mut cards = BTreeMap::new();
    let mut paragraphs = BTreeMap::new();
    for card in &board.cards {
        let card_id = card.foreign_id.as_str();
        if card_id.is_empty() || !ids.insert(card_id) || cards.insert(card_id, card).is_some() {
            return Err(EditError::InvalidSelection);
        }
        for paragraph in &card.paragraphs {
            if let Some(id) = paragraph.foreign_id.as_deref() {
                if id.is_empty() || !ids.insert(id) || paragraphs.insert(id, (card_id, paragraph)).is_some()
                    || paragraphs.len() > MAX_ELEMENTS {
                    return Err(EditError::InvalidSelection);
                }
            }
        }
    }
    Ok((cards, paragraphs))
}

/// Identity correspondence between the enriched candidate and reread. Construct
/// only after the caller's semantic and source verification gates have passed.
/// Missing, duplicate, cross-card, and changed durable identities are rejected.
pub struct BeatIdentityRebase<'a> {
    cards: Cards<'a>,
    paragraphs: Paragraphs<'a>,
    candidate_cards: BTreeMap<StableId, &'a BeatCard>,
    candidate_paragraphs: BTreeMap<StableId, (&'a BeatCard, &'a BeatParagraph)>,
}

impl<'a> BeatIdentityRebase<'a> {
    pub fn new(candidate: &Project, reread: &'a Project) -> Result<Self, EditError> {
        let old = candidate.beat_board();
        let new = reread.beat_board();
        if old.foreign_id.is_none() || old.foreign_id != new.foreign_id {
            return Err(EditError::InvalidSelection);
        }
        let (old_cards, old_paragraphs) = index_board(old)?;
        let (cards, paragraphs) = index_board(new)?;
        if old_cards.len() != cards.len() || old_paragraphs.len() != paragraphs.len()
            || old_cards.keys().any(|id| !cards.contains_key(id))
            || old_paragraphs.iter().any(|(id, (owner, _))| {
                paragraphs.get(id).is_none_or(|(new_owner, _)| owner != new_owner)
            }) {
            return Err(EditError::InvalidSelection);
        }
        let mut stable_ids = BTreeSet::new();
        let mut candidate_cards = BTreeMap::new();
        let mut candidate_paragraphs = BTreeMap::new();
        for old_card in &old.cards {
            let new_card = cards.get(old_card.foreign_id.as_str()).copied()
                .ok_or(EditError::InvalidSelection)?;
            if !stable_ids.insert(old_card.id.clone()) {
                return Err(EditError::InvalidSelection);
            }
            candidate_cards.insert(old_card.id.clone(), new_card);
            for old_paragraph in &old_card.paragraphs {
                if !stable_ids.insert(old_paragraph.id.clone()) {
                    return Err(EditError::InvalidSelection);
                }
                if let Some(id) = old_paragraph.foreign_id.as_deref() {
                    let (_, new_paragraph) = paragraphs.get(id).copied().ok_or(EditError::InvalidSelection)?;
                    candidate_paragraphs.insert(old_paragraph.id.clone(), (new_card, new_paragraph));
                }
            }
        }
        Ok(Self { cards, paragraphs, candidate_cards, candidate_paragraphs })
    }

    /// Resolve a candidate ListItem identity to its reread card.
    pub fn card(&self, durable_id: &str) -> Option<&'a BeatCard> {
        self.cards.get(durable_id).copied()
    }

    /// Resolve a candidate Paragraph identity only within its durable card.
    pub fn paragraph(&self, card_id: &str, paragraph_id: &str) -> Option<&'a BeatParagraph> {
        self.paragraphs.get(paragraph_id)
            .and_then(|(owner, paragraph)| (*owner == card_id).then_some(*paragraph))
    }

    /// Resolve a candidate-local card key via the candidate's durable identity.
    pub fn card_for_candidate_id(&self, id: &StableId) -> Option<&'a BeatCard> {
        self.candidate_cards.get(id).copied()
    }

    /// Resolve a candidate-local paragraph key through durable card and
    /// paragraph identities, returning its reread card and paragraph.
    pub fn paragraph_for_candidate_id(&self, id: &StableId) -> Option<(&'a BeatCard, &'a BeatParagraph)> {
        self.candidate_paragraphs.get(id).copied()
    }
}

fn paragraph_cost(paragraph: &BeatParagraph) -> Option<usize> {
    let mut cost = paragraph.text.len();
    for run in &paragraph.runs {
        cost = cost.checked_add(run.text.len())?;
        for attribute in &run.attributes {
            cost = cost.checked_add(attribute.name.len())?.checked_add(attribute.value.len())?;
        }
    }
    Some(cost)
}

fn same_content(left: &BeatParagraph, right: &BeatParagraph) -> bool {
    left.text == right.text && left.runs == right.runs
        && left.direct_settings == right.direct_settings
}

/// Fallback for a legitimately id-less candidate paragraph only. The stable
/// ID identifies the candidate owner, not the reread owner. Exact content must
/// occur once on each side; repeated or over-budget occurrences fail closed.
pub fn map_idless_beat_paragraph<'a>(
    candidate_board: &BeatBoard,
    reread_board: &'a BeatBoard,
    card_id: &str,
    candidate_paragraph: &StableId,
) -> Result<&'a BeatParagraph, EditError> {
    if card_id.is_empty() || candidate_board.foreign_id.as_deref().is_none_or(str::is_empty)
        || candidate_board.foreign_id != reread_board.foreign_id
        || candidate_board.cards.len() > MAX_ELEMENTS || reread_board.cards.len() > MAX_ELEMENTS {
        return Err(EditError::InvalidSelection);
    }
    let mut candidate_cards = candidate_board.cards.iter().filter(|card| card.foreign_id == card_id);
    let candidate = candidate_cards.next().ok_or(EditError::InvalidSelection)?;
    let mut reread_cards = reread_board.cards.iter().filter(|card| card.foreign_id == card_id);
    let reread = reread_cards.next().ok_or(EditError::InvalidSelection)?;
    if candidate_cards.next().is_some() || reread_cards.next().is_some()
        || candidate.paragraphs.len() > MAX_ELEMENTS || reread.paragraphs.len() > MAX_ELEMENTS {
        return Err(EditError::InvalidSelection);
    }
    let mut owners = candidate.paragraphs.iter().filter(|p| &p.id == candidate_paragraph);
    let old = owners.next().ok_or(EditError::InvalidSelection)?;
    if owners.next().is_some() || old.foreign_id.is_some() {
        return Err(EditError::InvalidSelection);
    }
    let old_cost = paragraph_cost(old).ok_or(EditError::InvalidSelection)?;
    let mut compared = 0usize;
    // The candidate-side match must be unique too, even if another occurrence
    // has an ID: otherwise content does not establish ownership.
    let mut old_matches = 0usize;
    for paragraph in &candidate.paragraphs {
        compared = compared.checked_add(old_cost.max(paragraph_cost(paragraph)
            .ok_or(EditError::InvalidSelection)?)).ok_or(EditError::InvalidSelection)?;
        if compared > MAX_SOURCE_BYTES { return Err(EditError::InvalidSelection); }
        if same_content(old, paragraph) { old_matches += 1; }
    }
    if old_matches != 1 { return Err(EditError::InvalidSelection); }
    let mut matched = None;
    for paragraph in &reread.paragraphs {
        compared = compared.checked_add(old_cost.max(paragraph_cost(paragraph)
            .ok_or(EditError::InvalidSelection)?)).ok_or(EditError::InvalidSelection)?;
        if compared > MAX_SOURCE_BYTES { return Err(EditError::InvalidSelection); }
        if same_content(old, paragraph) && matched.replace(paragraph).is_some() {
            return Err(EditError::InvalidSelection);
        }
    }
    matched.ok_or(EditError::InvalidSelection)
}

/// Clamp to the nearest preceding UTF-8 boundary at or before the requested byte offset.
pub fn clamp_beat_text_offset(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) { offset -= 1; }
    offset
}
