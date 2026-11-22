use super::{
    inputs::{EditorGroupBoundary, LayoutComposition, LayoutPurpose},
    interaction::{BodyLineIndexEntry, LineLocation},
};
use crate::inputs::LayoutEnvironment;
use scriptcraft_model::{
    formatting::{FixedPoint, ParagraphAlignment, SourceLayoutSettings},
    ElementTypeKey, LayoutInputs, ModelError, StableId,
};
use std::sync::Arc;

pub const POINTS_PER_INCH: u32 = 72;
pub(super) const DUAL_DIALOGUE_COLUMN_GAP_POINTS: i64 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Precise signed rectangle in hundredths of a point. Layout and output
/// consumers use this representation and its single conversion rule; `Rect`
/// is the rounded integer-point projection exposed for page-level geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRect {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub width: FixedPoint,
    pub height: FixedPoint,
}

impl FixedRect {
    pub fn bottom(self) -> FixedPoint {
        FixedPoint::from_raw(self.y.raw().saturating_add(self.height.raw()))
    }
}

/// The selected baseline advance/line-box height and natural font-table
/// extents. Natural ascent/descent do not control pagination or clipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineVerticalMetrics {
    pub leading: FixedPoint,
    pub natural_ascent: FixedPoint,
    pub natural_descent: FixedPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub element: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutLine {
    pub page: usize,
    pub element_type: Option<ElementTypeKey>,
    pub element: usize,
    pub source: SourceRange,
    pub text: Arc<str>,
    /// Display-only text; canonical `text` and source byte ranges are untouched.
    pub display_text: Arc<str>,
    /// Source-character to display-character spans, relative to this line's
    /// display text and absolute within the canonical element text.
    pub display_source_map: Arc<[DisplaySourceSegment]>,
    /// Per-run display and canonical geometry derived from resolved typography.
    pub spans: Arc<[PositionedStyledSpan]>,
    /// Effective paragraph alignment before alignment shifts the line geometry.
    pub alignment: ParagraphAlignment,
    /// Left anchor before alignment shifts the line; includes a first-visual-line indent.
    pub unaligned_line_left: FixedPoint,
    /// Text extent after first-line indent and alignment. The rectangle shares
    /// the paragraph/span y and leading-sized height; its bottom is baseline.
    pub geometry: FixedRect,
    /// Full paragraph bounds, retained separately for hit testing and measure.
    /// It shares the line's y and height, so its bottom is the baseline too.
    pub paragraph_bounds: FixedRect,
    /// Per-visual-line selected leading and natural metric extent.
    pub vertical_metrics: LineVerticalMetrics,
    /// Rounded integer-point projection of the canonical fixed-point geometry.
    pub rect: Rect,
    pub keep_with_next: bool,
    pub oversized: bool,
}

impl LayoutLine {
    /// The canonical baseline is the line-box bottom, independent of font ink.
    pub fn baseline_y(&self) -> FixedPoint {
        self.geometry.bottom()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionedStyledSpan {
    pub display_text: Arc<str>,
    pub source: SourceRange,
    pub display_source_map: Arc<[DisplaySourceSegment]>,
    /// The resolved style used for face selection and color policy.
    pub resolved_text_style: scriptcraft_model::typography::EffectiveTextStyle,
    pub geometry: FixedRect,
    /// Canonical x offsets for every display-character boundary in this span.
    /// The vector starts at zero and ends at the span's laid-out width.
    pub display_boundary_offsets: Arc<[FixedPoint]>,
    pub underline: bool,
    pub strikethrough: bool,
}

/// A borrowed core line selected by an immutable location query.
///
/// The page index is the line's owning page in `ScreenplayLayout::pages`; no
/// internal line-index representation is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatedLayoutLine<'a> {
    pub page_index: usize,
    pub line: &'a LayoutLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySourceSegment {
    pub display_start: usize,
    pub display_end: usize,
    pub source_start: usize,
    pub source_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPage {
    pub number: usize,
    pub title_page: bool,
    pub bounds: Rect,
    pub content_bounds: Rect,
    pub lines: Vec<LayoutLine>,
    pub occupied_lines: u32,
    /// The top of the next line box. On a fresh page this is TopMargin; after
    /// placement it advances by the preceding line's effective leading.
    pub(super) next_line_top: FixedPoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayLayout {
    pub inputs: LayoutInputs,
    pub(super) layout_environment: Arc<LayoutEnvironment>,
    pub source_settings: SourceLayoutSettings,
    pub purpose: LayoutPurpose,
    pub composition: LayoutComposition,
    /// The identity resolved against this project's current group records.
    /// Print and stale Body identities always expose `None`.
    pub active_group_id: Option<StableId>,
    /// Editor-only boundary metadata; always empty for Print and inactive Body.
    pub editor_group_boundaries: Vec<EditorGroupBoundary>,
    pub pages: Vec<LayoutPage>,
    pub(super) element_line_index: Vec<Vec<LineLocation>>,
    pub(super) body_line_index: Vec<BodyLineIndexEntry>,
    pub(super) body_line_y_order: Vec<usize>,
    pub(super) body_lines_by_element: Vec<Vec<usize>>,
    pub(super) title_line_index: Vec<BodyLineIndexEntry>,
    pub(super) title_lines_by_element: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caret {
    pub element: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutError {
    pub kind: LayoutErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutErrorKind {
    Model(ModelError),
    InvalidProfile,
    InvalidPosition,
    NoGeometry,
    UnsupportedDocumentLeading,
    InvalidEffectiveLeading,
    InvalidSpaceBefore,
}

impl From<ModelError> for LayoutError {
    fn from(error: ModelError) -> Self {
        Self {
            kind: LayoutErrorKind::Model(error),
        }
    }
}
