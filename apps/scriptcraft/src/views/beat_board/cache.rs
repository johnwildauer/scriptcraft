//! Disposable card-local layouts, compared against their complete inputs.

use crate::presentation::document_font::PreparedDocumentFonts;
use scriptcraft_layout::{BeatCardLayout, BeatCardMetrics, LayoutEnvironment, LayoutError, BEAT_BODY_PLACEHOLDER, BEAT_TITLE_PLACEHOLDER};
use scriptcraft_model::{BeatBoard, BeatCard, BeatParagraph, BoardFixed, StableId};
use std::collections::{BTreeSet, HashMap};

pub(crate) struct CardLayouts {
    pub(crate) card: BeatCardLayout,
    pub(crate) placeholder: Option<BeatCardLayout>,
}

struct Entry {
    card: BeatCard,
    metrics: BeatCardMetrics,
    environment: LayoutEnvironment,
    fonts: PreparedDocumentFonts,
    layouts: CardLayouts,
}

#[derive(Default)]
pub(crate) struct BeatLayoutCache {
    entries: HashMap<StableId, Entry>,
}

fn same_font_bytes(left: &PreparedDocumentFonts, right: &PreparedDocumentFonts) -> bool {
    left.resolved_ids().count() == right.resolved_ids().count()
        && left.resolved_ids().all(|id| {
            left.face_bytes(id) == right.face_bytes(id)
                && left.face_index(id) == right.face_index(id)
        })
}

impl BeatLayoutCache {
    pub(super) fn retain_board(&mut self, board: &BeatBoard) {
        let ids: BTreeSet<_> = board.cards.iter().map(|card| &card.id).collect();
        self.entries.retain(|id, _| ids.contains(id));
    }

    pub(crate) fn get_or_build(
        &mut self,
        card: &BeatCard,
        environment: &LayoutEnvironment,
        fonts: &PreparedDocumentFonts,
        metrics: BeatCardMetrics,
    ) -> Result<&CardLayouts, LayoutError> {
        let mut key = card.clone();
        key.rect.left = BoardFixed::default();
        key.rect.top = BoardFixed::default();
        let matches = self.entries.get(&card.id).is_some_and(|entry| {
            entry.card == key && entry.metrics == metrics && entry.environment == *environment
                && same_font_bytes(&entry.fonts, fonts)
        });
        if !matches {
            let layout = BeatCardLayout::new(card, environment, metrics)?;
            let placeholder = if card.title.is_empty() || layout.body_is_empty {
                let mut sample = card.clone();
                sample.title = BEAT_TITLE_PLACEHOLDER.to_owned();
                sample.paragraphs = vec![BeatParagraph {
                    id: card.id.clone(),
                    foreign_id: None,
                    source_paragraph_ordinal: None,
                    text: BEAT_BODY_PLACEHOLDER.to_owned(),
                    runs: Vec::new(),
                    direct_settings: Default::default(),
                }];
                Some(BeatCardLayout::new(&sample, environment, metrics)?)
            } else {
                None
            };
            self.entries.insert(card.id.clone(), Entry {
                card: key,
                metrics,
                environment: environment.clone(),
                fonts: fonts.clone(),
                layouts: CardLayouts { card: layout, placeholder },
            });
        }
        // A successful build or a matching entry always leaves this key present.
        self.entries.get(&card.id).map(|entry| &entry.layouts).ok_or(LayoutError {
            kind: scriptcraft_layout::LayoutErrorKind::NoGeometry,
        })
    }
}
