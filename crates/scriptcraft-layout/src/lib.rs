#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses explicit effective inputs and fixed integer metrics rather than
//! a platform font or viewport. The deterministic defaults therefore have stable structure on every host: one
//! character cell is seven units wide and one line is twelve units high.

mod cache;
mod dual_dialogue;
mod geometry;
mod inputs;
mod interaction;
mod paginate;
mod spans;
mod title_page;
mod wrap;

pub use cache::{LayoutBuildDiagnostics, LayoutCache};
pub use geometry::{
    Caret, DisplaySourceSegment, FixedRect, LayoutError, LayoutErrorKind, LayoutLine, LayoutPage,
    LocatedLayoutLine, PositionedStyledSpan, Rect, ScreenplayLayout, SourceRange, CHAR_WIDTH,
    FONT_FAMILY, LINE_HEIGHT, POINTS_PER_INCH,
};
pub use inputs::{BodyEditorPresentation, EditorGroupBoundary, LayoutComposition, LayoutPurpose};
pub use interaction::InteractionQueryDiagnostics;
