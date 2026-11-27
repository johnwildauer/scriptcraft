//! Project-owned Beat content and exact board geometry, independent of view state.

use crate::{formatting::SourceParagraphOverrides, StableId, StoryMapPlacement, TextRun};

/// Signed logical pixels in thousandths, with no floating-point semantic state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoardFixed(i64);

impl BoardFixed {
    pub const SCALE: i64 = 1_000;

    pub const fn from_milli_pixels(value: i64) -> Self {
        Self(value)
    }

    pub const fn from_pixels(value: i64) -> Option<Self> {
        match value.checked_mul(Self::SCALE) {
            Some(raw) => Some(Self(raw)),
            None => None,
        }
    }

    pub const fn raw(self) -> i64 {
        self.0
    }

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(raw) => Some(Self(raw)),
            None => None,
        }
    }

    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(raw) => Some(Self(raw)),
            None => None,
        }
    }
}

pub const DEFAULT_BOARD_WIDTH: BoardFixed = BoardFixed::from_milli_pixels(24_000_000);
pub const DEFAULT_BOARD_HEIGHT: BoardFixed = BoardFixed::from_milli_pixels(10_000_000);
pub const DEFAULT_CARD_WIDTH: BoardFixed = BoardFixed::from_milli_pixels(240_000);
pub const DEFAULT_CARD_HEIGHT: BoardFixed = BoardFixed::from_milli_pixels(100_000);
pub const MIN_CARD_WIDTH: BoardFixed = BoardFixed::from_milli_pixels(120_000);
pub const MIN_CARD_HEIGHT: BoardFixed = BoardFixed::from_milli_pixels(60_000);
pub const MAX_CARD_WIDTH: BoardFixed = BoardFixed::from_milli_pixels(1_000_000);
pub const MAX_CARD_HEIGHT: BoardFixed = BoardFixed::from_milli_pixels(1_000_000);
pub const BOARD_SNAP_INCREMENT: BoardFixed = BoardFixed::from_milli_pixels(20_000);
/// Per-axis limit of one million logical pixels bounds semantic geometry.
pub const MAX_BOARD_DIMENSION: BoardFixed = BoardFixed::from_milli_pixels(1_000_000_000);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BeatPoint {
    pub x: BoardFixed,
    pub y: BoardFixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BeatRect {
    pub left: BoardFixed,
    pub top: BoardFixed,
    pub width: BoardFixed,
    pub height: BoardFixed,
}

impl BeatRect {
    pub const fn right(self) -> Option<BoardFixed> {
        self.left.checked_add(self.width)
    }

    pub const fn bottom(self) -> Option<BoardFixed> {
        self.top.checked_add(self.height)
    }

    /// True only for a positive rectangle completely inside the board.
    pub fn contained_in(self, width: BoardFixed, height: BoardFixed) -> bool {
        self.left.raw() >= 0
            && self.top.raw() >= 0
            && self.width.raw() > 0
            && self.height.raw() > 0
            && self.right().is_some_and(|right| right <= width)
            && self.bottom().is_some_and(|bottom| bottom <= height)
    }
}

impl Default for BeatRect {
    fn default() -> Self {
        Self {
            left: BoardFixed::default(),
            top: BoardFixed::default(),
            width: DEFAULT_CARD_WIDTH,
            height: DEFAULT_CARD_HEIGHT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatParagraph {
    pub id: StableId,
    pub foreign_id: Option<String>,
    /// Import-time coordinate used only to assign an ID to an originally id-less
    /// paragraph. It is not a text-run address or a durable selection identity.
    /// New paragraphs have no coordinate and receive an ID when created.
    pub source_paragraph_ordinal: Option<usize>,
    pub text: String,
    pub runs: Vec<TextRun>,
    /// Recognized direct source settings; opaque source remains retained separately.
    pub direct_settings: SourceParagraphOverrides,
}

impl BeatParagraph {
    /// Compare only the shared canonical text and ordered run values, without
    /// session identities or original occurrence coordinates.
    pub fn same_text_and_runs(&self, other: &Self) -> bool {
        self.text == other.text && self.runs == other.runs
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatCard {
    pub id: StableId,
    pub foreign_id: String,
    pub title: String,
    pub paragraphs: Vec<BeatParagraph>,
    pub rect: BeatRect,
    /// Optional Story Map PageGoal placement for this same ListItem/card.
    pub story_map_placement: Option<StoryMapPlacement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatBoard {
    pub foreign_id: Option<String>,
    pub width: BoardFixed,
    pub height: BoardFixed,
    /// Winning board Item order, back to front; not ListItems source order.
    pub cards: Vec<BeatCard>,
    /// Card identities in ListItems source/creation order, independent of stacking.
    /// Creation appends its new identity; deletion removes it; activation leaves it
    /// unchanged. This ledger must contain every current card exactly once.
    pub card_creation_order: Vec<StableId>,
}

impl Default for BeatBoard {
    fn default() -> Self {
        Self {
            foreign_id: None,
            width: DEFAULT_BOARD_WIDTH,
            height: DEFAULT_BOARD_HEIGHT,
            cards: Vec::new(),
            card_creation_order: Vec::new(),
        }
    }
}
