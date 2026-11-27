#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses explicit effective inputs and fixed-point geometry rather than a
//! platform font or viewport. Courier leading is 1.0× effective style size and
//! Proportional leading is 1.2× size, selected by exact resolved-face category;
//! DocumentLeading adjusts candidates before each wrapped line's style maximum.
//! Each canonical baseline is its line-box top plus that line's natural ascent
//! and document-leading adjustment; line tops advance by full effective leading.
//! Candidate fit uses the final line's baseline minus signed natural descent;
//! full effective leading remains the line-top advance. Natural extents do not
//! clip glyphs to line boxes, and oversized forward progress remains bounded.

mod beat_board;
mod cache;
mod dual_dialogue;
mod geometry;
mod inputs;
mod interaction;
mod paginate;
mod script_note;
mod spans;
mod story_map;
mod title_page;
mod wrap;

pub use beat_board::{
    BeatCardLayout, BeatCardMetrics, BeatDisplayOnlySpan, BeatTitleProjection,
    BEAT_BODY_PLACEHOLDER, BEAT_TITLE_PLACEHOLDER,
};
pub use cache::{LayoutBuildDiagnostics, LayoutCache};
pub use script_note::{
    ScriptNoteDisplayOnlySpan, ScriptNoteLayout, ScriptNoteMetrics,
    ScriptNoteTitleProjection, SCRIPT_NOTE_BODY_PLACEHOLDER, SCRIPT_NOTE_TITLE_PLACEHOLDER,
};
pub use geometry::{
    Caret, DisplaySourceSegment, FixedRect, LayoutError, LayoutErrorKind, LayoutLine, LayoutPage,
    LineVerticalMetrics, LocatedLayoutLine, PositionedStyledSpan, Rect, ScreenplayLayout,
    SourceRange, POINTS_PER_INCH,
};
pub use inputs::{
    resolve_effective_dual_wrapper_paragraph, resolve_effective_paragraph,
    resolve_effective_run_text_style, resolve_effective_text_style, resolve_effective_title,
    BodyEditorPresentation, EditorGroupBoundary, EffectiveParagraph, LayoutComposition,
    LayoutEnvironment, LayoutPurpose, NeutralTitleStyle, PaginationPolicy,
};
pub use scriptcraft_model::layout_inputs::DocumentLeading;
pub use interaction::InteractionQueryDiagnostics;
pub use story_map::{
    measure_scene_preview, project_story_map, SceneParagraphProjection, SceneProjection,
    StoryMapPreviewLayout, StoryMapPreviewMetrics, StoryMapProjection, StoryMapProjectionError,
};
