//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses explicit effective inputs and fixed integer metrics rather than
//! a platform font or viewport. The deterministic defaults therefore have stable structure on every host: one
//! character cell is seven units wide and one line is twelve units high.

use crate::{
    formatted_text::{parse_style_attribute, RunStyleFlags},
    formatting::{
        formatting_for_role, EffectiveTextStyle, FixedPoint, FontFallback, ParagraphAlignment,
    },
    validate_layout_inputs, LayoutInputs, ModelError, Project, Role, RunAttribute, SourceLayoutSettings,
    StableId, TextRun,
};
use std::time::Instant;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

pub const FONT_FAMILY: &str = "Courier-family fixed metrics (12 pt target)";
pub const CHAR_WIDTH: u32 = 7;
pub const LINE_HEIGHT: u32 = 12;
pub const POINTS_PER_INCH: u32 = 72;
const DUAL_DIALOGUE_COLUMN_GAP_POINTS: i64 = 18;
const MAX_LINES_PER_ELEMENT: usize = 1_000_000;
const MAX_CACHED_ELEMENT_WRAPS: usize = 4_096;
const MAX_CACHED_ELEMENT_WRAP_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHED_PREPARED_OWNERS: usize = 4_096;
const MAX_CACHED_PREPARED_OWNER_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Precise signed rectangle in hundredths of a point. `Rect` remains as the
/// integer compatibility projection; all layout and output consumers use this
/// representation and its single conversion rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRect {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub width: FixedPoint,
    pub height: FixedPoint,
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
    pub role: Role,
    pub element: usize,
    pub source: SourceRange,
    pub text: Arc<str>,
    /// Display-only text; canonical `text` and source byte ranges are untouched.
    pub display_text: Arc<str>,
    /// Source-character to display-character spans, relative to this line's
    /// display text and absolute within the canonical element text.
    pub display_source_map: Arc<[DisplaySourceSegment]>,
    /// Per-run display and fixed-cell geometry. `geometry` and `text_style` are
    /// retained as line-level compatibility projections; mixed-style consumers
    /// must use these positioned spans.
    pub spans: Arc<[PositionedStyledSpan]>,
    pub text_style: EffectiveTextStyle,
    pub char_width: FixedPoint,
    /// Effective paragraph alignment before fixed-width text alignment is applied.
    pub alignment: ParagraphAlignment,
    /// Left anchor before alignment shifts the line; includes a first-visual-line indent.
    pub unaligned_line_left: FixedPoint,
    /// Text extent after first-line indent and alignment.
    pub geometry: FixedRect,
    /// Full paragraph bounds, retained separately for hit testing and measure.
    pub paragraph_bounds: FixedRect,
    /// Rounded integer-point compatibility geometry for older callers.
    pub rect: Rect,
    pub keep_with_next: bool,
    pub oversized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionedStyledSpan {
    pub display_text: Arc<str>,
    pub source: SourceRange,
    pub display_source_map: Arc<[DisplaySourceSegment]>,
    pub text_style: EffectiveTextStyle,
    pub char_width: FixedPoint,
    pub geometry: FixedRect,
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

impl LocatedLayoutLine<'_> {
    /// Applies the existing deterministic fixed-cell x-to-source mapping to
    /// this already located line.
    pub fn fixed_caret_for_x(&self, x: FixedPoint) -> Caret {
        let display_char = display_char_for_x(self.line, x);
        Caret {
            element: self.line.element,
            offset: self.line.source_for_display_char(display_char),
        }
    }

    /// Applies the existing fixed-cell source-to-x mapping to a caret owned by
    /// this line. This does not perform another line-ownership query.
    pub fn fixed_caret_geometry(&self, caret: &Caret) -> Result<FixedRect, LayoutError> {
        if caret.element != self.line.element
            || caret.offset < self.line.source.start
            || caret.offset > self.line.source.end
            || !is_boundary_in_line(self.line, caret.offset)
        {
            return Err(LayoutError {
                kind: LayoutErrorKind::InvalidPosition,
            });
        }
        let display_char = self.line.display_char_for_source(caret.offset, false);
        Ok(FixedRect {
            x: FixedPoint::from_raw(x_for_display_char(self.line, display_char)),
            y: self.line.geometry.y,
            width: FixedPoint::from_points(1),
            height: self.line.geometry.height,
        })
    }
}

/// Selects which document surface is laid out. Print's compatibility builders
/// include the title page by default; callers can explicitly disable it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutComposition {
    BodyEditor,
    TitlePageEditor,
    Print { include_title_page: bool },
}

/// Transient presentation choice for one Body-editor layout request. This is
/// deliberately separate from `Project` and is not accepted by Print builders.
#[derive(Debug, Clone, Default)]
pub struct BodyEditorPresentation {
    pub active_group_id: Option<StableId>,
    project_identity: Option<Arc<()>>,
}

impl BodyEditorPresentation {
    /// Binds a transient active-group choice to the current document instance.
    /// Reusing it after document replacement safely resolves to inactive mode.
    pub fn for_project(project: &Project, active_group_id: Option<StableId>) -> Self {
        Self {
            active_group_id,
            project_identity: Some(project.layout_identity().clone()),
        }
    }

    pub fn active_group_id(&self) -> Option<&StableId> {
        self.active_group_id.as_ref()
    }
}

impl PartialEq for BodyEditorPresentation {
    fn eq(&self, other: &Self) -> bool {
        self.active_group_id == other.active_group_id
            && match (&self.project_identity, &other.project_identity) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for BodyEditorPresentation {}

impl Hash for BodyEditorPresentation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.active_group_id.hash(state);
        self.project_identity
            .as_ref()
            .map(|identity| Arc::as_ptr(identity) as usize)
            .hash(state);
    }
}

/// Editor-only page geometry for the active group's upper and lower boundary.
/// The zero-height rectangles are metadata for desktop painting, not lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorGroupBoundary {
    pub group_id: StableId,
    pub upper_page: usize,
    pub upper: FixedRect,
    pub lower_page: usize,
    pub lower: FixedRect,
}

impl LayoutComposition {
    const fn purpose(self) -> LayoutPurpose {
        match self {
            Self::Print { .. } => LayoutPurpose::Print,
            Self::BodyEditor | Self::TitlePageEditor => LayoutPurpose::Editor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutPurpose {
    Editor,
    Print,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySourceSegment {
    pub display_start: usize,
    pub display_end: usize,
    pub source_start: usize,
    pub source_end: usize,
}

impl LayoutLine {
    /// Maps a canonical UTF-8 boundary to a display-character boundary. The
    /// optional trailing choice is used only for an offset inside an expanded
    /// source scalar; normal editing offsets are always scalar boundaries.
    pub fn display_char_for_source(&self, source_offset: usize, trailing: bool) -> usize {
        for segment in self.display_source_map.iter() {
            if source_offset <= segment.source_start {
                return segment.display_start;
            }
            if source_offset < segment.source_end {
                return if trailing {
                    segment.display_end
                } else {
                    segment.display_start
                };
            }
            if source_offset == segment.source_end {
                return segment.display_end;
            }
        }
        self.display_text.chars().count()
    }

    /// Maps a display caret boundary to the nearest valid canonical boundary.
    /// Boundaries inside a Unicode uppercase expansion deterministically map
    /// before the source scalar; the boundary after its final cell maps after.
    pub fn source_for_display_char(&self, display_char: usize) -> usize {
        for segment in self.display_source_map.iter() {
            if display_char <= segment.display_start {
                return segment.source_start;
            }
            if display_char < segment.display_end {
                return segment.source_start;
            }
            if display_char == segment.display_end {
                return segment.source_end;
            }
        }
        self.source.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPage {
    pub number: usize,
    pub title_page: bool,
    pub bounds: Rect,
    pub content_bounds: Rect,
    pub lines: Vec<LayoutLine>,
    pub occupied_lines: u32,
    next_line_y: FixedPoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayLayout {
    pub inputs: LayoutInputs,
    pub source_settings: SourceLayoutSettings,
    pub font_family: String,
    pub char_width: u32,
    pub line_height: u32,
    pub purpose: LayoutPurpose,
    pub composition: LayoutComposition,
    /// The identity resolved against this project's current group records.
    /// Print and stale Body identities always expose `None`.
    pub active_group_id: Option<StableId>,
    /// Editor-only boundary metadata; always empty for Print and inactive Body.
    pub editor_group_boundaries: Vec<EditorGroupBoundary>,
    pub pages: Vec<LayoutPage>,
    element_line_index: Vec<Vec<LineLocation>>,
    body_line_index: Vec<BodyLineIndexEntry>,
    body_line_y_order: Vec<usize>,
    body_lines_by_element: Vec<Vec<usize>>,
    title_line_index: Vec<BodyLineIndexEntry>,
    title_lines_by_element: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineLocation {
    page: usize,
    line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BodyLineIndexEntry {
    location: LineLocation,
    absolute_y: i64,
    element: usize,
    source: SourceRange,
}

/// Value-free work count for one indexed interaction query. It counts index
/// entries actually inspected, including binary-search probes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InteractionQueryDiagnostics {
    pub index_entries_examined: u64,
    pub index_queries: u64,
    pub equal_y_candidates_examined: u64,
}

impl InteractionQueryDiagnostics {
    fn query(&mut self) {
        self.index_queries = self.index_queries.saturating_add(1);
    }

    fn examined(&mut self) {
        self.index_entries_examined = self.index_entries_examined.saturating_add(1);
    }

    fn equal_y_candidate(&mut self) {
        self.equal_y_candidates_examined = self.equal_y_candidates_examined.saturating_add(1);
        self.examined();
    }
}

/// Session-owned immutable layout cache shared by desktop adapters and
/// headless measurements. The revision, effective geometry and source-derived
/// layout settings are the cache key; viewport presentation state is excluded.
#[derive(Debug, Default)]
pub struct LayoutCache {
    revision: Option<u64>,
    inputs: Option<LayoutInputs>,
    source_settings: Option<SourceLayoutSettings>,
    purpose: Option<LayoutPurpose>,
    composition: Option<LayoutComposition>,
    body_presentation: Option<BodyEditorPresentation>,
    project_layout_signature: Option<u64>,
    layout: Option<Arc<ScreenplayLayout>>,
    project_identity: Option<Arc<()>>,
    element_wraps: ElementWrapCache,
    prepared_owners: PreparedOwnerCache,
}

/// Measurements from one layout build. These values intentionally contain no
/// document-owned content and are returned separately from the layout so that
/// diagnostics cannot affect layout equality or pagination output.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LayoutBuildDiagnostics {
    /// Complete Project validations performed by this layout request.
    pub complete_project_validations: u64,
    pub layout_builds: u64,
    /// Document-sized semantic signatures constructed by the generic cache path.
    /// Validated-session lookups must leave this at zero on both hits and misses.
    pub semantic_signature_constructions: u64,
    pub element_wraps_built: u64,
    pub element_wrap_misses: u64,
    pub element_wraps_reused: u64,
    pub element_wrap_nanos: u128,
    /// Complete owner-local preparations retained or rebuilt across revisions.
    pub prepared_owner_builds: u64,
    pub prepared_owner_reuses: u64,
    pub prepared_owner_evictions: u64,
    /// Display projection and run/style preparation before each wrap-cache lookup.
    pub display_projection_nanos: u128,
    pub projected_scalars: u64,
    pub run_range_searches: u64,
    pub style_attribute_parses: u64,
    /// Source-map remapping performed after either a wrap-cache hit or miss.
    pub source_map_remap_nanos: u128,
    pub source_map_segments_examined: u64,
    /// Shared owner-run materializations after wrap lookup; cache hits reuse the
    /// cached owner slice and do not construct another run vector.
    pub post_cache_run_constructions: u64,
    pub post_cache_run_bytes_cloned: u64,
    pub post_cache_run_nanos: u128,
    /// Positioned geometry built while materializing each layout line.
    pub positioned_span_nanos: u128,
    pub positioned_span_characters: u64,
    /// Complete interaction-index construction at the end of a revision build.
    pub interaction_index_build_nanos: u128,
    pub interaction_index_entries: u64,
    pub interaction_index_entries_sorted: u64,
    /// Whole-cache scans made while evicting wrap entries.
    pub wrap_cache_evictions: u64,
    pub wrap_cache_eviction_entries_examined: u64,
}

impl LayoutBuildDiagnostics {
    fn record_element_wrap(&mut self, started: Instant) {
        self.element_wraps_built = self.element_wraps_built.saturating_add(1);
        self.element_wrap_misses = self.element_wrap_misses.saturating_add(1);
        self.element_wrap_nanos = self
            .element_wrap_nanos
            .saturating_add(started.elapsed().as_nanos());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ElementWrapContext {
    element_id: crate::StableId,
    role: Role,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    atomic_signature: Vec<(usize, usize)>,
}

/// Collision-safe owner identity for the complete paragraph-relative layout
/// value. The effective paragraph/style metrics and wrap context are retained
/// rather than raw document-wide settings: placement-only inputs (page starts,
/// keep-with-next, page Y, and active-group presentation) remain outside this
/// owner-invariant boundary.
#[derive(Debug, Clone)]
struct PreparedOwnerIdentity {
    fingerprint: u64,
    context: ElementWrapContext,
    composition: LayoutComposition,
    canonical_text: Arc<str>,
    runs: Arc<[TextRun]>,
}

impl PreparedOwnerIdentity {
    fn new(
        context: ElementWrapContext,
        composition: LayoutComposition,
        canonical_text: &str,
        runs: Arc<[TextRun]>,
    ) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        context.hash(&mut hasher);
        composition.hash(&mut hasher);
        canonical_text.hash(&mut hasher);
        runs.len().hash(&mut hasher);
        for run in runs.iter() {
            run.text.hash(&mut hasher);
            run.attributes.len().hash(&mut hasher);
            for attribute in &run.attributes {
                attribute.name.hash(&mut hasher);
                attribute.value.hash(&mut hasher);
            }
        }
        Self {
            fingerprint: hasher.finish(),
            context,
            composition,
            canonical_text: Arc::from(canonical_text),
            runs,
        }
    }

    /// Fingerprints and shared immutable identity are only fast rejection/accept
    /// paths; independently allocated candidates still receive full-content
    /// equality checks before they can be considered the same owner.
    fn matches(&self, other: &Self) -> bool {
        if self.fingerprint != other.fingerprint {
            return false;
        }
        let shared_content = Arc::ptr_eq(&self.canonical_text, &other.canonical_text)
            && Arc::ptr_eq(&self.runs, &other.runs);
        self.context == other.context
            && self.composition == other.composition
            && (shared_content
                || (self.canonical_text.as_ref() == other.canonical_text.as_ref()
                    && self.runs.as_ref() == other.runs.as_ref()))
    }

    fn fingerprint_parts(
        context: &ElementWrapContext,
        composition: LayoutComposition,
        canonical_text: &str,
        runs: &[TextRun],
    ) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        context.hash(&mut hasher);
        composition.hash(&mut hasher);
        canonical_text.hash(&mut hasher);
        runs.len().hash(&mut hasher);
        for run in runs {
            run.text.hash(&mut hasher);
            run.attributes.len().hash(&mut hasher);
            for attribute in &run.attributes {
                attribute.name.hash(&mut hasher);
                attribute.value.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    fn matches_parts(
        &self,
        fingerprint: u64,
        context: &ElementWrapContext,
        composition: LayoutComposition,
        canonical_text: &str,
        runs: &[TextRun],
    ) -> bool {
        self.fingerprint == fingerprint
            && &self.context == context
            && self.composition == composition
            && self.canonical_text.as_ref() == canonical_text
            && self.runs.as_ref() == runs
    }
}

impl PartialEq for PreparedOwnerIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.matches(other)
    }
}

impl Eq for PreparedOwnerIdentity {}

/// Immutable, owner-relative line data ready for later page placement. All X
/// values are relative to the paragraph's left edge and all Y values are
/// relative to the owner's first line; source offsets remain canonical UTF-8
/// byte offsets within that owner. Page numbers, page Y, and placement policy
/// are intentionally not represented here.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedOwnerLine {
    role: Role,
    source_start: usize,
    source_end: usize,
    text: Arc<str>,
    display_text: Arc<str>,
    display_source_map: Arc<[DisplaySourceSegment]>,
    runs: Arc<[TextRun]>,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    paragraph: EffectiveParagraph,
    first_visual_line: bool,
    final_visual_line: bool,
    oversized: bool,
    line_left_offset: FixedPoint,
    geometry: FixedRect,
    paragraph_bounds: FixedRect,
    spans: Arc<[PositionedStyledSpan]>,
}

/// Complete expensive per-owner preparation retained independently of any
/// page-placement pass. The bounded prepared-owner cache retains this immutable
/// value by its collision-safe identity; placement and interaction indexing
/// rebase into fresh layout data and never mutate the cached representation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedOwnerLayout {
    identity: PreparedOwnerIdentity,
    lines: Arc<[PreparedOwnerLine]>,
    relative_bounds: FixedRect,
    estimated_bytes: usize,
}

impl PreparedOwnerLayout {
    fn from_wrapped_lines(
        identity: PreparedOwnerIdentity,
        wrapped_lines: &[WrappedLine],
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Self {
        let span_started = diagnostics.as_ref().map(|_| Instant::now());
        let span_characters = if diagnostics.is_some() {
            wrapped_lines
                .iter()
                .map(|line| line.display_text.chars().count() as u64)
                .sum()
        } else {
            0
        };
        let mut lines = Vec::with_capacity(wrapped_lines.len());
        let mut relative_y = FixedPoint::default();
        let mut relative_width = FixedPoint::default();
        let mut retained_bytes = std::mem::size_of::<PreparedOwnerLayout>()
            .saturating_add(std::mem::size_of::<PreparedOwnerIdentity>())
            .saturating_add(identity.canonical_text.len())
            .saturating_add(identity.runs.iter().fold(0usize, |total, run| {
                total
                    .saturating_add(std::mem::size_of::<TextRun>())
                    .saturating_add(run.text.len())
                    .saturating_add(
                    run.attributes.iter().fold(0usize, |attribute_bytes, attribute| {
                        attribute_bytes
                            .saturating_add(std::mem::size_of::<crate::RunAttribute>())
                            .saturating_add(attribute.name.len())
                            .saturating_add(attribute.value.len())
                    }),
                    )
            }));

        for wrapped in wrapped_lines {
            let paragraph = wrapped.paragraph;
            let indent = if wrapped.first_visual_line {
                paragraph.first_indent
            } else {
                FixedPoint::default()
            };
            let line_left = FixedPoint::from_raw(paragraph.left.raw().saturating_add(indent.raw()));
            let line_width = FixedPoint::from_raw(
                paragraph.right.raw().saturating_sub(line_left.raw()),
            );
            let mut spans = positioned_spans(
                wrapped,
                FixedRect {
                    x: line_left,
                    y: FixedPoint::default(),
                    width: FixedPoint::default(),
                    height: paragraph.line_advance,
                },
                diagnostics.as_deref_mut(),
            );
            let text_width = FixedPoint::from_raw(
                spans.iter().fold(0i64, |total, span| {
                    total.saturating_add(span.geometry.width.raw())
                }),
            );
            let spare = line_width.raw().saturating_sub(text_width.raw()).max(0);
            let display_chars = wrapped.display_text.chars().collect::<Vec<_>>();
            let justified_gaps = if paragraph.alignment == ParagraphAlignment::Justified
                && !wrapped.final_visual_line
            {
                (1..display_chars.len().saturating_sub(1))
                    .filter(|index| {
                        display_chars[*index].is_whitespace()
                            && !display_chars[*index - 1].is_whitespace()
                            && !display_chars[*index + 1].is_whitespace()
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let distribute = !justified_gaps.is_empty();
            let distributed_width = if distribute { spare } else { 0 };
            let gap_width = if distribute {
                spare.div_euclid(justified_gaps.len() as i64)
            } else {
                0
            };
            let gap_remainder = if distribute {
                spare.rem_euclid(justified_gaps.len() as i64) as usize
            } else {
                0
            };
            let text_x = match paragraph.alignment {
                ParagraphAlignment::Left | ParagraphAlignment::Justified => line_left,
                ParagraphAlignment::Right => FixedPoint::from_raw(
                    paragraph.right.raw().saturating_sub(text_width.raw()),
                ),
                ParagraphAlignment::Center => {
                    FixedPoint::from_raw(line_left.raw().saturating_add(spare / 2))
                }
            };
            let alignment_shift = text_x.raw().saturating_sub(line_left.raw());
            for span in &mut spans {
                span.geometry.x = FixedPoint::from_raw(
                    span.geometry.x.raw().saturating_add(alignment_shift),
                );
            }
            if distribute {
                let gap_adjustment = |index: usize| {
                    justified_gaps
                        .iter()
                        .position(|gap| *gap == index)
                        .map(|rank| {
                            gap_width.saturating_add(if rank < gap_remainder { 1 } else { 0 })
                        })
                        .unwrap_or(0)
                };
                let mut display_index = 0usize;
                let mut shift = 0i64;
                for span in &mut spans {
                    let count = span.display_text.chars().count();
                    let internal = (display_index..display_index.saturating_add(count))
                        .map(gap_adjustment)
                        .sum::<i64>();
                    span.geometry.x = FixedPoint::from_raw(
                        span.geometry.x.raw().saturating_add(shift),
                    );
                    span.geometry.width = FixedPoint::from_raw(
                        span.geometry.width.raw().saturating_add(internal),
                    );
                    shift = shift.saturating_add(internal);
                    display_index = display_index.saturating_add(count);
                }
            }

            let paragraph_left = paragraph.left.raw();
            for span in &mut spans {
                span.source.element = 0;
                span.geometry.x = FixedPoint::from_raw(
                    span.geometry.x.raw().saturating_sub(paragraph_left),
                );
                span.geometry.y = relative_y;
            }
            let geometry = FixedRect {
                x: FixedPoint::from_raw(text_x.raw().saturating_sub(paragraph_left)),
                y: relative_y,
                width: FixedPoint::from_raw(text_width.raw().saturating_add(distributed_width)),
                height: paragraph.line_advance,
            };
            let paragraph_bounds = FixedRect {
                x: FixedPoint::default(),
                y: relative_y,
                width: FixedPoint::from_raw(paragraph.right.raw().saturating_sub(paragraph_left)),
                height: paragraph.line_advance,
            };
            if paragraph_bounds.width.raw() > relative_width.raw() {
                relative_width = paragraph_bounds.width;
            }
            retained_bytes = retained_bytes
                .saturating_add(std::mem::size_of::<PreparedOwnerLine>())
                .saturating_add(wrapped.text.len())
                .saturating_add(wrapped.display_text.len())
                .saturating_add(
                    wrapped
                        .display_source_map
                        .len()
                        .saturating_mul(std::mem::size_of::<DisplaySourceSegment>()),
                )
                .saturating_add(spans.iter().fold(0usize, |total, span| {
                    total
                        .saturating_add(std::mem::size_of::<PositionedStyledSpan>())
                        .saturating_add(span.display_text.len())
                        .saturating_add(
                            span.display_source_map
                                .len()
                                .saturating_mul(std::mem::size_of::<DisplaySourceSegment>()),
                        )
                }));
            let relative_paragraph = EffectiveParagraph {
                left: FixedPoint::default(),
                right: paragraph_bounds.width,
                ..paragraph
            };
            lines.push(PreparedOwnerLine {
                role: wrapped.role,
                source_start: wrapped.start,
                source_end: wrapped.end,
                text: wrapped.text.clone(),
                display_text: wrapped.display_text.clone(),
                display_source_map: wrapped.display_source_map.clone(),
                runs: wrapped.runs.clone(),
                text_style: wrapped.text_style.clone(),
                char_width: wrapped.char_width,
                paragraph: relative_paragraph,
                first_visual_line: wrapped.first_visual_line,
                final_visual_line: wrapped.final_visual_line,
                oversized: wrapped.oversized,
                line_left_offset: FixedPoint::from_raw(line_left.raw().saturating_sub(paragraph_left)),
                geometry,
                paragraph_bounds,
                spans: spans.into(),
            });
            relative_y = FixedPoint::from_raw(
                relative_y.raw().saturating_add(paragraph.line_advance.raw()),
            );
        }
        if let (Some(diagnostics), Some(started)) = (diagnostics.as_deref_mut(), span_started) {
            diagnostics.positioned_span_nanos = diagnostics
                .positioned_span_nanos
                .saturating_add(started.elapsed().as_nanos());
            diagnostics.positioned_span_characters = diagnostics
                .positioned_span_characters
                .saturating_add(span_characters);
        }
        Self {
            identity,
            lines: lines.into(),
            relative_bounds: FixedRect {
                x: FixedPoint::default(),
                y: FixedPoint::default(),
                width: relative_width,
                height: relative_y,
            },
            estimated_bytes: retained_bytes,
        }
    }

    /// A single owner larger than the prepared-owner cache byte budget is used
    /// ephemerally rather than retained. The entry-count bound is enforced by
    /// the cache, not by this immutable value.
    fn within_owner_cache_byte_limit(&self) -> bool {
        self.estimated_bytes <= MAX_CACHED_PREPARED_OWNER_BYTES
    }
}

#[derive(Debug)]
struct PreparedOwnerEntry {
    layout: Arc<PreparedOwnerLayout>,
    last_used_generation: u64,
    insertion_order: u64,
    byte_cost: usize,
}

#[derive(Debug, Default)]
struct PreparedOwnerCache {
    buckets: HashMap<u64, Vec<PreparedOwnerEntry>>,
    generation: u64,
    next_order: u64,
    entry_count: usize,
    retained_bytes: usize,
}

impl PreparedOwnerCache {
    fn start_generation(&mut self) {
        if self.generation == u64::MAX {
            self.generation = 1;
            for entry in self.buckets.values_mut().flatten() {
                entry.last_used_generation = 0;
            }
        } else {
            self.generation = self.generation.saturating_add(1).max(1);
        }
    }

    fn clear(&mut self) {
        self.buckets.clear();
        self.generation = 0;
        self.next_order = 0;
        self.entry_count = 0;
        self.retained_bytes = 0;
    }

    fn get_or_build(
        &mut self,
        context: ElementWrapContext,
        composition: LayoutComposition,
        canonical_text: &str,
        runs: &[TextRun],
        build: impl FnOnce(PreparedOwnerIdentity) -> PreparedOwnerLayout,
    ) -> (Arc<PreparedOwnerLayout>, bool, u64) {
        let fingerprint = PreparedOwnerIdentity::fingerprint_parts(
            &context,
            composition,
            canonical_text,
            runs,
        );
        if let Some(entry) = self.buckets.get_mut(&fingerprint).and_then(|bucket| {
            bucket.iter_mut().find(|entry| {
                entry.layout.identity.matches_parts(
                    fingerprint,
                    &context,
                    composition,
                    canonical_text,
                    runs,
                )
            })
        }) {
            entry.last_used_generation = self.generation;
            return (entry.layout.clone(), true, 0);
        }

        let identity = PreparedOwnerIdentity::new(
            context,
            composition,
            canonical_text,
            Arc::from(runs.to_vec()),
        );
        let prepared = Arc::new(build(identity));
        let mut evictions = 0;
        if prepared.within_owner_cache_byte_limit()
        {
            let byte_cost = prepared.estimated_bytes;
            let bucket = self.buckets.entry(fingerprint).or_default();
            bucket.push(PreparedOwnerEntry {
                layout: prepared.clone(),
                last_used_generation: self.generation,
                insertion_order: self.next_order,
                byte_cost,
            });
            self.next_order = self.next_order.saturating_add(1);
            self.entry_count = self.entry_count.saturating_add(1);
            self.retained_bytes = self.retained_bytes.saturating_add(byte_cost);
            evictions = self.evict_if_needed();
        }
        (prepared, false, evictions)
    }

    fn evict_if_needed(&mut self) -> u64 {
        let mut evictions = 0u64;
        while self.entry_count > MAX_CACHED_PREPARED_OWNERS
            || self.retained_bytes > MAX_CACHED_PREPARED_OWNER_BYTES
        {
            let oldest = self
                .buckets
                .iter()
                .flat_map(|(fingerprint, bucket)| {
                    bucket.iter().enumerate().map(move |(index, entry)| {
                        (
                            *fingerprint,
                            index,
                            entry.last_used_generation,
                            entry.insertion_order,
                        )
                    })
                })
                .min_by_key(|(_, _, last_used, order)| (*last_used, *order));
            let Some((fingerprint, index, _, _)) = oldest else {
                break;
            };
            if let Some(bucket) = self.buckets.get_mut(&fingerprint) {
                let entry = bucket.remove(index);
                self.entry_count = self.entry_count.saturating_sub(1);
                self.retained_bytes = self.retained_bytes.saturating_sub(entry.byte_cost);
                evictions = evictions.saturating_add(1);
                if bucket.is_empty() {
                    self.buckets.remove(&fingerprint);
                }
            }
        }
        evictions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EffectiveGeometryMode {
    Sequential,
    DualColumn,
}

#[derive(Debug)]
struct ElementWrapEntry {
    context: ElementWrapContext,
    canonical_text: Arc<str>,
    runs: Arc<[TextRun]>,
    lines: Arc<[WrappedLine]>,
    last_used_generation: u64,
    insertion_order: u64,
    byte_cost: usize,
}

#[derive(Debug, Default)]
struct ElementWrapCache {
    buckets: HashMap<u64, Vec<ElementWrapEntry>>,
    generation: u64,
    next_order: u64,
    entry_count: usize,
    retained_bytes: usize,
}

impl ElementWrapCache {
    fn start_generation(&mut self) {
        if self.generation == u64::MAX {
            self.generation = 1;
            for entry in self.buckets.values_mut().flatten() {
                entry.last_used_generation = 0;
            }
        } else {
            self.generation = self.generation.saturating_add(1).max(1);
        }
    }

    fn clear(&mut self) {
        self.buckets.clear();
        self.generation = 0;
        self.next_order = 0;
        self.entry_count = 0;
        self.retained_bytes = 0;
    }

    fn fingerprint(&self, context: &ElementWrapContext, text: &str, runs: &[TextRun]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        context.hash(&mut hasher);
        text.hash(&mut hasher);
        runs.len().hash(&mut hasher);
        for run in runs {
            run.text.hash(&mut hasher);
            run.attributes.len().hash(&mut hasher);
            for attribute in &run.attributes {
                attribute.name.hash(&mut hasher);
                attribute.value.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    // This internal hot path receives validated layout inputs individually to
    // avoid building temporary state during per-paragraph wrapping.
    #[allow(clippy::too_many_arguments)]
    fn wrap(
        &mut self,
        element_id: &crate::StableId,
        text: &str,
        element: usize,
        role: Role,
        paragraph: EffectiveParagraph,
        text_style: EffectiveTextStyle,
        char_width: FixedPoint,
        runs: &[TextRun],
        source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
        purpose: LayoutPurpose,
        title_page: bool,
        geometry_mode: EffectiveGeometryMode,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Vec<WrappedLine> {
        let context = ElementWrapContext {
            element_id: element_id.clone(),
            role,
            purpose,
            title_page,
            geometry_mode,
            paragraph,
            text_style: text_style.clone(),
            char_width,
            atomic_signature: source_map
                .iter()
                .map(|(display, _)| (display.start, display.end))
                .collect(),
        };
        let fingerprint = self.fingerprint(&context, text, runs);
        if let Some(entry) = self.buckets.get_mut(&fingerprint).and_then(|bucket| {
            bucket
                .iter_mut()
                .find(|entry| {
                    entry.context == context
                        && entry.canonical_text.as_ref() == text
                        && entry.runs.as_ref() == runs
                })
        }) {
            entry.last_used_generation = self.generation;
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.element_wraps_reused =
                    diagnostics.element_wraps_reused.saturating_add(1);
            }
            return entry
                .lines
                .iter()
                .cloned()
                .map(|mut line| {
                    line.element = element;
                    line.runs = entry.runs.clone();
                    line
                })
                .collect();
        }

        let started = diagnostics.as_ref().map(|_| Instant::now());
        let mut lines = wrap_element(
            text, element, role, paragraph, text_style, char_width, source_map,
        );
        let shared_runs = share_owner_runs(&mut lines, runs, diagnostics.as_deref_mut());
        if let (Some(diagnostics), Some(started)) = (diagnostics.as_deref_mut(), started) {
            diagnostics.record_element_wrap(started);
        }
        let cached_lines = lines
            .iter()
            .cloned()
            .map(|mut line| {
                line.element = 0;
                line
            })
            .collect::<Vec<_>>();
        let run_bytes = runs.iter().fold(0usize, |total, run| {
            total
                .saturating_add(run.text.len())
                .saturating_add(run.attributes.iter().fold(0usize, |attribute_total, attribute| {
                    attribute_total
                        .saturating_add(attribute.name.len())
                        .saturating_add(attribute.value.len())
                }))
        });
        let byte_cost = text
            .len()
            .saturating_add(run_bytes)
            .saturating_add(cached_lines.iter().fold(0usize, |total, line| {
                total
                    .saturating_add(line.text.len())
                    .saturating_add(line.display_text.len())
                    .saturating_add(
                        line.display_source_map
                            .len()
                            .saturating_mul(std::mem::size_of::<DisplaySourceSegment>()),
                    )
            }));
        if byte_cost > MAX_CACHED_ELEMENT_WRAP_BYTES {
            return lines;
        }
        self.next_order = self.next_order.saturating_add(1);
        self.buckets
            .entry(fingerprint)
            .or_default()
            .push(ElementWrapEntry {
                context,
                canonical_text: Arc::from(text),
                runs: shared_runs,
                lines: cached_lines.into(),
                last_used_generation: self.generation,
                insertion_order: self.next_order,
                byte_cost,
            });
        self.entry_count = self.entry_count.saturating_add(1);
        self.retained_bytes = self.retained_bytes.saturating_add(byte_cost);
        lines
    }

    fn evict_if_needed(&mut self, diagnostics: Option<&mut LayoutBuildDiagnostics>) {
        let mut diagnostics = diagnostics;
        while self.entry_count > MAX_CACHED_ELEMENT_WRAPS
            || self.retained_bytes > MAX_CACHED_ELEMENT_WRAP_BYTES
        {
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.wrap_cache_eviction_entries_examined = diagnostics
                    .wrap_cache_eviction_entries_examined
                    .saturating_add(self.entry_count as u64);
            }
            let oldest = self
                .buckets
                .iter()
                .flat_map(|(fingerprint, bucket)| {
                    bucket.iter().enumerate().map(move |(index, entry)| {
                        (
                            entry.last_used_generation,
                            entry.insertion_order,
                            *fingerprint,
                            index,
                        )
                    })
                })
                .min();
            let Some((_, _, fingerprint, index)) = oldest else {
                self.entry_count = 0;
                break;
            };
            let (removed_bytes, remove_bucket) =
                if let Some(bucket) = self.buckets.get_mut(&fingerprint) {
                    let removed = bucket.remove(index);
                    let remove_bucket = bucket.is_empty();
                    (removed.byte_cost, remove_bucket)
                } else {
                    (0, false)
                };
            if remove_bucket {
                self.buckets.remove(&fingerprint);
            }
            self.entry_count = self.entry_count.saturating_sub(1);
            self.retained_bytes = self.retained_bytes.saturating_sub(removed_bytes);
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.wrap_cache_evictions =
                    diagnostics.wrap_cache_evictions.saturating_add(1);
            }
        }
    }
}

fn share_owner_runs(
    lines: &mut [WrappedLine],
    runs: &[TextRun],
    mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
) -> Arc<[TextRun]> {
    let started = diagnostics.as_ref().map(|_| Instant::now());
    let run_bytes = if diagnostics.is_some() {
        runs.iter().fold(0u64, |total, run| {
            total.saturating_add(run.text.len() as u64).saturating_add(
                run.attributes.iter().fold(0u64, |attribute_total, attribute| {
                    attribute_total
                        .saturating_add(attribute.name.len() as u64)
                        .saturating_add(attribute.value.len() as u64)
                }),
            )
        })
    } else {
        0
    };
    let shared_runs: Arc<[TextRun]> = Arc::from(runs.to_vec());
    for line in lines {
        line.runs = shared_runs.clone();
    }
    if let (Some(diagnostics), Some(started)) = (diagnostics.as_deref_mut(), started) {
        diagnostics.post_cache_run_nanos = diagnostics
            .post_cache_run_nanos
            .saturating_add(started.elapsed().as_nanos());
        diagnostics.post_cache_run_constructions = diagnostics
            .post_cache_run_constructions
            .saturating_add(1);
        diagnostics.post_cache_run_bytes_cloned = diagnostics
            .post_cache_run_bytes_cloned
            .saturating_add(run_bytes);
    }
    shared_runs
}

impl LayoutCache {
    pub fn get_or_build(
        &mut self,
        revision: u64,
        project: &Project,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        let inputs = project.session_layout_inputs();
        self.get_or_build_with_inputs(revision, project, &inputs)
    }

    pub fn get_or_build_with_inputs(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_inputs_for_purpose(revision, project, inputs, LayoutPurpose::Editor)
    }

    pub fn get_or_build_with_inputs_for_purpose(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            revision, project, inputs, purpose,
            match purpose { LayoutPurpose::Print => LayoutComposition::Print { include_title_page: true }, LayoutPurpose::Editor => LayoutComposition::BodyEditor },
            None, false, BodyEditorPresentation::default(),
        )
    }

    pub fn get_or_build_with_composition(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        composition: LayoutComposition,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            revision, project, inputs, composition.purpose(), composition, None, false,
            BodyEditorPresentation::default(),
        )
    }

    /// Builds or reuses a Body-editor layout for a transient active-group
    /// presentation. There is intentionally no corresponding Print overload.
    pub fn get_or_build_body_editor(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        presentation: &BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_body_editor_with_diagnostics(
            revision,
            project,
            inputs,
            presentation,
            &mut LayoutBuildDiagnostics::default(),
        )
    }

    pub fn get_or_build_body_editor_with_diagnostics(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        presentation: &BodyEditorPresentation,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            Some(diagnostics),
            false,
            presentation.clone(),
        )
    }

    /// Looks up a Body-editor layout using the validated revision published by
    /// the editing session. The transient presentation remains part of the key.
    pub fn get_or_build_body_editor_for_session(
        &mut self,
        session: &crate::editing::EditingSession,
        presentation: &BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_body_editor_for_session_with_diagnostics(
            session,
            presentation,
            &mut LayoutBuildDiagnostics::default(),
        )
    }

    /// Diagnostic counterpart to [`Self::get_or_build_body_editor_for_session`].
    pub fn get_or_build_body_editor_for_session_with_diagnostics(
        &mut self,
        session: &crate::editing::EditingSession,
        presentation: &BodyEditorPresentation,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            Some(diagnostics),
            true,
            presentation.clone(),
        )
    }

    /// Looks up the Title-page-editor layout using the session's validated
    /// revision and the title-editor composition.
    pub fn get_or_build_title_page_editor_for_session(
        &mut self,
        session: &crate::editing::EditingSession,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_title_page_editor_for_session_with_diagnostics(
            session,
            &mut LayoutBuildDiagnostics::default(),
        )
    }

    /// Diagnostic counterpart to [`Self::get_or_build_title_page_editor_for_session`].
    pub fn get_or_build_title_page_editor_for_session_with_diagnostics(
        &mut self,
        session: &crate::editing::EditingSession,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::TitlePageEditor,
            Some(diagnostics),
            true,
            BodyEditorPresentation::default(),
        )
    }

    /// Diagnostic counterpart to the ordinary cache lookup. A cache hit
    /// returns zero build measurements; a miss reports wrapping work separately
    /// from validation, pagination, and cache lookup.
    pub fn get_or_build_with_inputs_for_purpose_and_diagnostics(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            purpose,
            match purpose { LayoutPurpose::Print => LayoutComposition::Print { include_title_page: true }, LayoutPurpose::Editor => LayoutComposition::BodyEditor },
            Some(diagnostics),
            false,
            BodyEditorPresentation::default(),
        )
    }

    /// Builds from a live editing session whose complete baseline and every
    /// published edit candidate are validated by the session boundary.
    pub fn get_or_build_for_session_with_diagnostics(
        &mut self,
        revision: u64,
        session: &crate::editing::EditingSession,
        purpose: LayoutPurpose,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            revision,
            session.project(),
            session.layout_inputs(),
            purpose,
            match purpose { LayoutPurpose::Print => LayoutComposition::Print { include_title_page: true }, LayoutPurpose::Editor => LayoutComposition::BodyEditor },
            Some(diagnostics),
            true,
            BodyEditorPresentation::default(),
        )
    }

    fn get_or_build_with_optional_diagnostics(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
        composition: LayoutComposition,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
        validated_session: bool,
        body_presentation: BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.complete_project_validations = if validated_session { 0 } else { 1 };
        }
        if !validated_session {
            project.validate()?;
        }
        let mut source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings().clone())
            .unwrap_or_default();
        source_settings
            .direct_paragraph_overrides
            .extend(project.direct_paragraph_overrides.clone());
        let same_document = self
            .project_identity
            .as_ref()
            .is_some_and(|identity| Arc::ptr_eq(identity, project.layout_identity()));
        if self.project_identity.is_some() && !same_document {
            self.revision = None;
            self.inputs = None;
            self.source_settings = None;
            self.purpose = None;
            self.composition = None;
            self.body_presentation = None;
            self.project_layout_signature = None;
            self.layout = None;
            self.element_wraps.clear();
            self.prepared_owners.clear();
        }
        let source_settings_match = self
            .source_settings
            .as_ref()
            .is_some_and(|cached| cached == &source_settings);
        let cache_keys_match = self.revision == Some(revision)
            && self.inputs.as_ref() == Some(inputs)
            && source_settings_match
            && same_document
            && self.purpose == Some(purpose)
            && self.composition == Some(composition)
            && self.body_presentation.as_ref() == Some(&body_presentation);
        let mut project_layout_signature = None;
        if cache_keys_match {
            if validated_session {
                return self
                    .layout
                    .clone()
                    .ok_or(LayoutError {
                        kind: LayoutErrorKind::NoGeometry,
                    })
                    .map(|layout| (layout, true));
            }
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.semantic_signature_constructions = diagnostics
                    .semantic_signature_constructions
                    .saturating_add(1);
            }
            let signature = semantic_project_layout_signature(project);
            if self.project_layout_signature == Some(signature) {
                return self
                    .layout
                    .clone()
                    .ok_or(LayoutError {
                        kind: LayoutErrorKind::NoGeometry,
                    })
                    .map(|layout| (layout, true));
            }
            project_layout_signature = Some(signature);
        }
        if !validated_session && project_layout_signature.is_none() {
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.semantic_signature_constructions = diagnostics
                    .semantic_signature_constructions
                    .saturating_add(1);
            }
            project_layout_signature = Some(semantic_project_layout_signature(project));
        }
        self.element_wraps.start_generation();
        self.prepared_owners.start_generation();
        let layout = Arc::new(
            ScreenplayLayout::build_with_effective_inputs_for_purpose_internal(
                project,
                inputs,
                &source_settings,
                purpose,
                composition,
                diagnostics.as_deref_mut(),
                Some(&mut self.element_wraps),
                Some(&mut self.prepared_owners),
                true,
                &body_presentation,
            )?,
        );
        self.element_wraps
            .evict_if_needed(diagnostics.as_deref_mut());
        let prepared_owner_evictions = self.prepared_owners.evict_if_needed();
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.prepared_owner_evictions = diagnostics
                .prepared_owner_evictions
                .saturating_add(prepared_owner_evictions);
        }
        self.revision = Some(revision);
        self.inputs = Some(inputs.clone());
        self.source_settings = Some(source_settings);
        self.purpose = Some(purpose);
        self.composition = Some(composition);
        self.body_presentation = Some(body_presentation);
        self.project_layout_signature = project_layout_signature;
        self.layout = Some(layout.clone());
        self.project_identity = Some(project.layout_identity().clone());
        Ok((layout, false))
    }
}

fn semantic_project_layout_signature(project: &Project) -> u64 {
    let mut signature_hasher = std::collections::hash_map::DefaultHasher::new();
    format!(
        "{:?}{:?}{:?}{:?}{:?}",
        project.elements,
        project.title_page,
        project.direct_paragraph_overrides,
        project.dual_dialogue_groups,
        project.dual_dialogue_paragraph_overrides,
    )
    .hash(&mut signature_hasher);
    signature_hasher.finish()
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
}

impl From<ModelError> for LayoutError {
    fn from(error: ModelError) -> Self {
        Self {
            kind: LayoutErrorKind::Model(error),
        }
    }
}

impl ScreenplayLayout {
    pub fn build(project: &Project) -> Result<Self, LayoutError> {
        let inputs = project.session_layout_inputs();
        Self::build_with_inputs(project, &inputs)
    }

    pub fn build_with_inputs(
        project: &Project,
        inputs: &LayoutInputs,
    ) -> Result<Self, LayoutError> {
        Self::build_with_inputs_for_purpose(project, inputs, LayoutPurpose::Editor)
    }

    pub fn build_with_inputs_for_purpose(
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
    ) -> Result<Self, LayoutError> {
        let source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings.clone())
            .unwrap_or_default();
        Self::build_with_effective_inputs_for_purpose(project, inputs, &source_settings, purpose)
    }

    pub fn build_with_effective_inputs(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
    ) -> Result<Self, LayoutError> {
        Self::build_with_effective_inputs_for_purpose(
            project,
            inputs,
            source_settings,
            LayoutPurpose::Editor,
        )
    }

    pub fn build_with_effective_inputs_for_purpose(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        purpose: LayoutPurpose,
    ) -> Result<Self, LayoutError> {
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            source_settings,
            purpose,
            match purpose {
                LayoutPurpose::Print => LayoutComposition::Print { include_title_page: true },
                LayoutPurpose::Editor => LayoutComposition::BodyEditor,
            },
            None,
            None,
            None,
            false,
            &BodyEditorPresentation::default(),
        )
    }

    pub fn build_with_composition(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        composition: LayoutComposition,
    ) -> Result<Self, LayoutError> {
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            source_settings,
            composition.purpose(),
            composition,
            None,
            None,
            None,
            false,
            &BodyEditorPresentation::default(),
        )
    }

    /// Builds Body-editor geometry with one optional, transient active group.
    pub fn build_with_body_presentation(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        presentation: &BodyEditorPresentation,
    ) -> Result<Self, LayoutError> {
        let mut effective_settings = source_settings.clone();
        effective_settings
            .direct_paragraph_overrides
            .extend(project.direct_paragraph_overrides.clone());
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            &effective_settings,
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            None,
            None,
            None,
            false,
            presentation,
        )
    }

    /// Builds the same deterministic layout while returning value-free work
    /// measurements separately from the layout result.
    pub fn build_with_effective_inputs_for_purpose_and_diagnostics(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        purpose: LayoutPurpose,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<Self, LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        diagnostics.complete_project_validations = 1;
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            source_settings,
            purpose,
            match purpose {
                LayoutPurpose::Print => LayoutComposition::Print { include_title_page: true },
                LayoutPurpose::Editor => LayoutComposition::BodyEditor,
            },
            Some(diagnostics),
            None,
            None,
            false,
            &BodyEditorPresentation::default(),
        )
    }

    fn build_with_effective_inputs_for_purpose_internal(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        purpose: LayoutPurpose,
        composition: LayoutComposition,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
        mut wrap_cache: Option<&mut ElementWrapCache>,
        mut prepared_owner_cache: Option<&mut PreparedOwnerCache>,
        validated_session: bool,
        body_presentation: &BodyEditorPresentation,
    ) -> Result<Self, LayoutError> {
        if !validated_session {
            project.validate()?;
        }
        validate_layout_inputs(inputs).map_err(|_| LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        })?;
        validate_profile(inputs)?;
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.layout_builds = diagnostics.layout_builds.saturating_add(1);
        }
        let active_group_id = if composition == LayoutComposition::BodyEditor {
            body_presentation
                .active_group_id
                .as_ref()
                .filter(|_| body_presentation.project_identity.as_ref().is_none_or(|identity| Arc::ptr_eq(identity, project.layout_identity())))
                .filter(|requested| project.dual_dialogue_groups.iter().any(|group| &group.id == *requested))
                .cloned()
        } else {
            None
        };
        let char_width = inputs.char_width;
        let font_family = inputs.font_family.clone();
        let mut output = Self {
            inputs: inputs.clone(),
            source_settings: source_settings.clone(),
            font_family,
            char_width,
            line_height: inputs.line_advance,
            purpose,
            composition,
            active_group_id,
            editor_group_boundaries: Vec::new(),
            pages: Vec::new(),
            element_line_index: Vec::new(),
            body_line_index: Vec::new(),
            body_line_y_order: Vec::new(),
            body_lines_by_element: Vec::new(),
            title_line_index: Vec::new(),
            title_lines_by_element: Vec::new(),
        };
        let include_title_page = match composition {
            LayoutComposition::BodyEditor => false,
            LayoutComposition::TitlePageEditor => true,
            LayoutComposition::Print { include_title_page } => include_title_page,
        };
        let include_body = !matches!(composition, LayoutComposition::TitlePageEditor);
        if include_title_page && !project.title_page.is_empty() {
            let mut page = output.new_page(true);
            for (index, element) in project.title_page.iter().enumerate() {
                if element.formatting.starts_new_page == Some(true) && !page.lines.is_empty() {
                    output.pages.push(page);
                    page = output.new_page(true);
                }
                let default_left = FixedPoint::from_points(i64::from(output.inputs.margin_left));
                let default_right = FixedPoint::from_points(i64::from(
                    output.inputs.paper_width.saturating_sub(output.inputs.margin_right),
                ));
                let requested_left = element.formatting.left_indent.unwrap_or(default_left);
                let requested_right = element.formatting.right_indent.unwrap_or(default_right);
                let (title_left, title_right) = if requested_left.raw() >= 0
                    && requested_right > requested_left
                    && requested_right <= FixedPoint::from_points(i64::from(output.inputs.paper_width))
                {
                    (requested_left, requested_right)
                } else {
                    (default_left, default_right)
                };
                let requested_indent = element.formatting.first_indent.unwrap_or_default();
                let title_indent = if title_right.raw().saturating_sub(title_left.raw())
                    .saturating_sub(requested_indent.raw())
                    >= i64::from(output.inputs.char_width) * FixedPoint::SCALE
                {
                    requested_indent
                } else {
                    FixedPoint::default()
                };
                let paragraph = EffectiveParagraph {
                    left: title_left,
                    right: title_right,
                    first_indent: title_indent,
                    alignment: element
                        .formatting
                        .alignment
                        .map(Into::into)
                        .unwrap_or(ParagraphAlignment::Left),
                    space_before: element.formatting.space_before.unwrap_or_default(),
                    line_advance: FixedPoint::from_raw(
                        i64::from(output.inputs.line_advance)
                            .saturating_mul(i64::from(inputs.font_size.clamp(1, 72)))
                            .saturating_mul(FixedPoint::SCALE)
                            .saturating_add(6)
                            .div_euclid(12),
                    ),
                    char_width: FixedPoint::from_raw(
                        i64::from(output.inputs.char_width)
                            .saturating_mul(i64::from(inputs.font_size.clamp(1, 72)))
                            .saturating_mul(FixedPoint::SCALE)
                            .saturating_add(6)
                            .div_euclid(12),
                    ),
                };
                let style = plain_title_style(inputs);
                let lines = owner_layout_for_layout(
                    &mut wrap_cache,
                    &mut prepared_owner_cache,
                    &mut diagnostics,
                    composition,
                    &element.id,
                    &element.text,
                    index,
                    element.role.unwrap_or(Role::General),
                    paragraph,
                    style,
                    &element.runs,
                    paragraph.char_width,
                    purpose,
                    true,
                    EffectiveGeometryMode::Sequential,
                );
                if !page.lines.is_empty() {
                    page.next_line_y = FixedPoint::from_raw(
                        page.next_line_y.raw().saturating_add(paragraph.space_before.raw().max(0)),
                    );
                }
                for line_index in 0..lines.len() {
                    let line_advance = lines.paragraph(line_index).line_advance.raw();
                    let laid_out = output.make_owner_line(
                        &page,
                        &lines,
                        line_index,
                        true,
                        page.next_line_y,
                        false,
                        diagnostics.as_deref_mut(),
                    );
                    page.lines.push(laid_out);
                    page.occupied_lines = page.occupied_lines.saturating_add(1);
                    page.next_line_y =
                        FixedPoint::from_raw(page.next_line_y.raw().saturating_add(line_advance));
                }
            }
            output.pages.push(page);
        }
        if include_title_page
            && include_body
            && output.pages.last().is_some_and(|page| page.title_page)
        {
            output.pages.push(output.new_page(false));
        }
        let mut index = 0;
        while include_body && index < project.elements.len() {
            let element = &project.elements[index];
            if let Some(group) = crate::dual_dialogue::group_at_element_index(project, index)
                .filter(|group| crate::dual_dialogue::first_member(project, group) == Some(index))
            {
                if output.active_group_id.as_ref() != Some(&group.id) {
                let (outer_left, outer_right) = dual_outer_extent(
                    &output.inputs,
                    source_settings,
                    project,
                    group,
                )?;
                let gap = FixedPoint::from_points(DUAL_DIALOGUE_COLUMN_GAP_POINTS);
                let column_width = FixedPoint::from_raw(
                    outer_right
                        .raw()
                        .saturating_sub(outer_left.raw())
                        .saturating_sub(gap.raw())
                        .div_euclid(2),
                );
                if column_width.raw() <= 0 {
                    return Err(LayoutError {
                        kind: LayoutErrorKind::InvalidProfile,
                    });
                }
                let left_box = (outer_left, FixedPoint::from_raw(
                    outer_left.raw().saturating_add(column_width.raw()),
                ));
                let right_box = (
                    FixedPoint::from_raw(left_box.1.raw().saturating_add(gap.raw())),
                    FixedPoint::from_raw(
                        left_box
                            .1
                            .raw()
                            .saturating_add(gap.raw())
                            .saturating_add(column_width.raw()),
                    ),
                );
                let left = wrap_dual_side(
                    project,
                    &group.left,
                    left_box,
                    &output.inputs,
                    source_settings,
                    project.legacy_json_state.is_some(),
                    purpose,
                    &mut wrap_cache,
                    &mut prepared_owner_cache,
                    composition,
                    &mut diagnostics,
                );
                let right = wrap_dual_side(
                    project,
                    &group.right,
                    right_box,
                    &output.inputs,
                    source_settings,
                    project.legacy_json_state.is_some(),
                    purpose,
                    &mut wrap_cache,
                    &mut prepared_owner_cache,
                    composition,
                    &mut diagnostics,
                );

                // Print-hidden members remain semantic group members but do not
                // create a visible block or apply wrapper-owned vertical fields.
                if left.line_count > 0 || right.line_count > 0 {
                    let wrapper = wrapper_overrides(project, source_settings, &group.id);
                    let starts_new_page = wrapper
                        .starts_new_page
                        .unwrap_or_else(|| source_settings.starts_new_page(Role::General));
                    let native_general = formatting_for_role(Role::General).paragraph;
                    let space_before = wrapper
                        .paragraph
                        .space_before
                        .or(source_settings.paragraph_spec(Role::General).space_before)
                        .unwrap_or_else(|| FixedPoint::from_points(i64::from(native_general.space_before)));
                    let first_visible_space_before = |side: &WrappedDualSide| {
                        side.paragraphs
                            .iter()
                            .find_map(|paragraph| {
                                (paragraph.len() > 0)
                                    .then(|| paragraph.paragraph(0).space_before.raw())
                            })
                            .unwrap_or_default()
                    };
                    let space_before = FixedPoint::from_raw(
                        space_before
                            .raw()
                            .max(first_visible_space_before(&left))
                            .max(first_visible_space_before(&right)),
                    );
                    output.place_dual_block(
                        &left,
                        &right,
                        starts_new_page,
                        space_before,
                        diagnostics.as_deref_mut(),
                    );
                }
                index = crate::dual_dialogue::last_member(project, group)
                    .map_or(index.saturating_add(1), |last| last.saturating_add(1));
                continue;
                }
            }

            let text_style = output.source_settings.text_style(element.role);
            if purpose == LayoutPurpose::Print && text_style.hidden_text {
                index = index.saturating_add(1);
                continue;
            }
            if output
                .source_settings
                .starts_new_page_for_element(element.role, &element.id)
                && output
                    .pages
                    .last()
                    .is_some_and(|page| !page.title_page && !page.lines.is_empty())
            {
                output.pages.push(output.new_page(false));
            }
            let paragraph = effective_paragraph(
                &output.inputs,
                source_settings,
                element.role,
                &element.id,
                project.legacy_json_state.is_some(),
                &text_style,
            );
            let lines = owner_layout_for_layout(
                &mut wrap_cache,
                &mut prepared_owner_cache,
                &mut diagnostics,
                composition,
                &element.id,
                element.text.as_str(),
                index,
                element.role,
                paragraph,
                text_style.clone(),
                    &element.runs,
                    paragraph.char_width,
                    purpose,
                false,
                EffectiveGeometryMode::Sequential,
            );
            output.place_element(lines, element.role, paragraph, diagnostics.as_deref_mut());
            index = index.saturating_add(1);
        }
        if output.pages.is_empty() {
            output.pages.push(output.new_page(composition == LayoutComposition::TitlePageEditor));
        }
        if let Some(active_group_id) = output.active_group_id.clone() {
            if let Some(group) = project.dual_dialogue_groups.iter().find(|group| group.id == active_group_id) {
                output.editor_group_boundaries = active_group_boundaries(&output, project, group);
            }
        }
        output.rebuild_interaction_indexes(diagnostics.as_deref_mut());
        Ok(output)
    }

    pub fn hit_test(&self, page: usize, x: u32, y: u32) -> Option<Caret> {
        self.hit_test_fixed(
            page,
            FixedPoint::from_points(i64::from(x)),
            FixedPoint::from_points(i64::from(y)),
        )
    }

    pub fn hit_test_fixed(&self, page: usize, x: FixedPoint, y: FixedPoint) -> Option<Caret> {
        let page = self.pages.get(page)?;
        if page.title_page && self.composition != LayoutComposition::TitlePageEditor {
            return None;
        }
        let line = page.lines.iter().find(|line| {
            let paragraph_left = line.paragraph_bounds.x.raw();
            let paragraph_right = paragraph_left.saturating_add(line.paragraph_bounds.width.raw());
            let text_left = line.geometry.x.raw();
            let text_right = text_left.saturating_add(line.geometry.width.raw());
            x.raw() >= paragraph_left.min(text_left)
                && x.raw() <= paragraph_right.max(text_right)
                && y.raw() >= line.paragraph_bounds.y.raw()
                && y.raw()
                    < line
                        .paragraph_bounds
                        .y
                        .raw()
                        .saturating_add(line.paragraph_bounds.height.raw())
        })?;
        Some(self.caret_for_line(line, x))
    }

    /// Hit-test the body as one document-flow domain. `page` and `y` describe
    /// the pointer in page-local coordinates; the page height is used only to
    /// compare lines across pages. Horizontal placement is deliberately
    /// clamped to the selected line's source range.
    pub fn nearest_line_hit_test(&self, page: usize, x: i64, y: i64) -> Option<Caret> {
        if self.pages.get(page)?.title_page
            && self.composition != LayoutComposition::TitlePageEditor
        {
            return None;
        }
        self.nearest_line_hit_test_fixed(
            page,
            FixedPoint::from_points(x),
            FixedPoint::from_points(y),
        )
    }

    pub fn nearest_line_hit_test_fixed(
        &self,
        page: usize,
        x: FixedPoint,
        y: FixedPoint,
    ) -> Option<Caret> {
        self.nearest_line_hit_test_fixed_with_diagnostics(
            page,
            x,
            y,
            &mut InteractionQueryDiagnostics::default(),
        )
    }

    pub fn nearest_line_hit_test_fixed_with_diagnostics(
        &self,
        page: usize,
        x: FixedPoint,
        y: FixedPoint,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<Caret> {
        self.nearest_line_location_fixed_with_diagnostics(page, x, y, diagnostics)
            .map(|location| location.fixed_caret_for_x(x))
    }

    /// Selects the same nearest core line as fixed hit testing without mapping
    /// the pointer's x coordinate to a caret.
    pub fn nearest_line_location_fixed_with_diagnostics(
        &self,
        page: usize,
        x: FixedPoint,
        y: FixedPoint,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<LocatedLayoutLine<'_>> {
        diagnostics.query();
        if self.pages.get(page)?.title_page {
            if self.composition != LayoutComposition::TitlePageEditor {
                return None;
            }
            let page_height = i64::from(self.inputs.paper_height).saturating_mul(FixedPoint::SCALE);
            let pointer_y = i64::try_from(page).ok()?.saturating_mul(page_height).saturating_add(y.raw());
            let entry = self.title_line_index.iter().enumerate().min_by_key(|(_, entry)| {
                diagnostics.examined();
                pointer_y.saturating_sub(entry.absolute_y).saturating_abs()
            })?.1;
            let line = &self.pages[entry.location.page].lines[entry.location.line];
            return Some(LocatedLayoutLine {
                page_index: entry.location.page,
                line,
            });
        }
        let page_height = i64::from(self.inputs.paper_height).saturating_mul(FixedPoint::SCALE);
        let pointer_y = i64::try_from(page)
            .ok()?
            .saturating_mul(page_height)
            .saturating_add(y.raw());
        let insertion = self.body_line_y_order.partition_point(|index| {
            diagnostics.examined();
            self.body_line_index[*index].absolute_y < pointer_y
        });
        let neighbor_coordinates = [insertion.checked_sub(1), (insertion < self.body_line_y_order.len()).then_some(insertion)];
        let mut nearest_y_distance = i64::MAX;
        let mut candidate_ys = [None, None];
        let mut candidate_y_count = 0usize;
        for coordinate_index in neighbor_coordinates.into_iter().flatten() {
            diagnostics.examined();
            let y = self.body_line_index[self.body_line_y_order[coordinate_index]].absolute_y;
            let distance = pointer_y.saturating_sub(y).saturating_abs();
            if distance < nearest_y_distance {
                nearest_y_distance = distance;
                candidate_ys = [Some(y), None];
                candidate_y_count = 1;
            } else if distance == nearest_y_distance
                && !candidate_ys[..candidate_y_count].contains(&Some(y))
                && candidate_y_count < candidate_ys.len()
            {
                candidate_ys[candidate_y_count] = Some(y);
                candidate_y_count += 1;
            }
        }
        let horizontal_distance = |line: &LayoutLine| {
            let left = line.paragraph_bounds.x.raw();
            let right = left.saturating_add(line.paragraph_bounds.width.raw());
            if x.raw() < left { left.saturating_sub(x.raw()) }
            else if x.raw() > right { x.raw().saturating_sub(right) }
            else { 0 }
        };
        let mut best: Option<(i64, usize)> = None;
        for y in candidate_ys[..candidate_y_count].iter().flatten().copied() {
            let start = self.body_line_y_order.partition_point(|index| {
                diagnostics.examined();
                self.body_line_index[*index].absolute_y < y
            });
            let end = self.body_line_y_order.partition_point(|index| {
                diagnostics.examined();
                self.body_line_index[*index].absolute_y <= y
            });
            for coordinate_index in start..end {
                let index = self.body_line_y_order[coordinate_index];
                diagnostics.equal_y_candidate();
                let entry = self.body_line_index[index];
                let line = &self.pages[entry.location.page].lines[entry.location.line];
                let candidate = (horizontal_distance(line), index);
                if best.is_none_or(|current| candidate < current) {
                    best = Some(candidate);
                }
            }
        }
        let (_, best_index) = best?;
        let entry = self.body_line_index[best_index];
        Some(LocatedLayoutLine {
            page_index: entry.location.page,
            line: &self.pages[entry.location.page].lines[entry.location.line],
        })
    }

    fn caret_for_line(&self, line: &LayoutLine, x: FixedPoint) -> Caret {
        let cells = display_char_for_x(line, x);
        Caret {
            element: line.element,
            offset: line.source_for_display_char(cells),
        }
    }

    pub fn caret_rect(&self, caret: Caret) -> Result<Rect, LayoutError> {
        self.caret_geometry(caret).map(compat_rect)
    }

    pub fn caret_geometry(&self, caret: Caret) -> Result<FixedRect, LayoutError> {
        self.caret_geometry_with_diagnostics(caret, &mut InteractionQueryDiagnostics::default())
    }

    pub fn caret_geometry_with_diagnostics(
        &self,
        caret: Caret,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Result<FixedRect, LayoutError> {
        self.line_for_caret_with_diagnostics(&caret, diagnostics)?
            .fixed_caret_geometry(&caret)
    }

    /// Finds the core line that owns a canonical caret without computing its
    /// fixed-cell x coordinate.
    pub fn line_for_caret_with_diagnostics(
        &self,
        caret: &Caret,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Result<LocatedLayoutLine<'_>, LayoutError> {
        diagnostics.query();
        let Some(locations) = self.element_line_index.get(caret.element) else {
            return Err(LayoutError {
                kind: LayoutErrorKind::InvalidPosition,
            });
        };
        for location in locations {
            diagnostics.examined();
            let line = &self.pages[location.page].lines[location.line];
            if caret.offset >= line.source.start
                && caret.offset <= line.source.end
                && is_boundary_in_line(line, caret.offset)
            {
                return Ok(LocatedLayoutLine {
                    page_index: location.page,
                    line,
                });
            }
        }
        Err(LayoutError {
            kind: LayoutErrorKind::InvalidPosition,
        })
    }

    /// First page owning this element/source range in the same page-and-line
    /// order used by the former complete-layout owner scan.
    pub fn page_for_position_with_diagnostics(
        &self,
        element: usize,
        offset: usize,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<usize> {
        self.page_for_position_in_range_with_diagnostics(element, offset, None, diagnostics)
    }

    pub fn page_for_position_in_range_with_diagnostics(
        &self,
        element: usize,
        offset: usize,
        page_range: Option<(usize, usize)>,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<usize> {
        diagnostics.query();
        self.element_line_index
            .get(element)?
            .iter()
            .find_map(|location| {
                diagnostics.examined();
                if page_range
                    .is_some_and(|(first, last)| location.page < first || location.page > last)
                {
                    return None;
                }
                let line = &self.pages[location.page].lines[location.line];
                (offset >= line.source.start && offset <= line.source.end).then_some(location.page)
            })
    }

    /// Locates the current body line in document order for vertical movement.
    /// Title-page lines are deliberately outside this navigation domain.
    pub fn body_line_index_for_position_with_diagnostics(
        &self,
        element: usize,
        offset: usize,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<usize> {
        diagnostics.query();
        self.body_lines_by_element
            .get(element)?
            .iter()
            .copied()
            .find(|index| {
                diagnostics.examined();
                let entry = self.body_line_index[*index];
                offset >= entry.source.start && offset <= entry.source.end
            })
    }

    pub fn body_line(&self, index: usize) -> Option<&LayoutLine> {
        let entry = self.body_line_index.get(index)?;
        self.pages
            .get(entry.location.page)?
            .lines
            .get(entry.location.line)
    }

    pub fn body_line_count(&self) -> usize {
        self.body_line_index.len()
    }

    /// Surface-local title navigation. It is intentionally unavailable on a
    /// body editor or a print composition, even when print includes title pages.
    pub fn title_line_index_for_position(&self, element: usize, offset: usize) -> Option<usize> {
        if self.composition != LayoutComposition::TitlePageEditor {
            return None;
        }
        self.title_lines_by_element.get(element)?.iter().copied().find(|index| {
            let entry = self.title_line_index[*index];
            offset >= entry.source.start && offset <= entry.source.end
        })
    }

    pub fn title_line(&self, index: usize) -> Option<&LayoutLine> {
        if self.composition != LayoutComposition::TitlePageEditor { return None; }
        let entry = self.title_line_index.get(index)?;
        self.pages.get(entry.location.page)?.lines.get(entry.location.line)
    }

    pub fn title_page_for_position(&self, element: usize, offset: usize) -> Option<usize> {
        if self.composition != LayoutComposition::TitlePageEditor { return None; }
        self.page_for_position_with_diagnostics(
            element,
            offset,
            &mut InteractionQueryDiagnostics::default(),
        ).filter(|page| self.pages.get(*page).is_some_and(|page| page.title_page))
    }

    pub fn title_caret_geometry(&self, caret: Caret) -> Result<FixedRect, LayoutError> {
        if self.composition != LayoutComposition::TitlePageEditor {
            return Err(LayoutError { kind: LayoutErrorKind::InvalidPosition });
        }
        self.caret_geometry(caret)
    }

    fn rebuild_interaction_indexes(&mut self, diagnostics: Option<&mut LayoutBuildDiagnostics>) {
        let started = diagnostics.as_ref().map(|_| Instant::now());
        let max_element = self
            .pages
            .iter()
            .flat_map(|page| page.lines.iter().map(|line| line.element))
            .max();
        let count = max_element.map_or(0, |element| element.saturating_add(1));
        self.element_line_index = vec![Vec::new(); count];
        self.body_line_index.clear();
        self.body_line_y_order.clear();
        self.body_lines_by_element = vec![Vec::new(); count];
        self.title_line_index.clear();
        self.title_lines_by_element = vec![Vec::new(); count];
        let page_height = i64::from(self.inputs.paper_height).saturating_mul(FixedPoint::SCALE);
        for (page_index, page) in self.pages.iter().enumerate() {
            for (line_index, line) in page.lines.iter().enumerate() {
                let location = LineLocation {
                    page: page_index,
                    line: line_index,
                };
                if let Some(element_lines) = self.element_line_index.get_mut(line.element) {
                    element_lines.push(location);
                }
                if page.title_page {
                    let absolute_y = i64::try_from(page_index)
                        .unwrap_or(i64::MAX)
                        .saturating_mul(page_height)
                        .saturating_add(line.geometry.y.raw());
                    let index = self.title_line_index.len();
                    self.title_line_index.push(BodyLineIndexEntry {
                        location,
                        absolute_y,
                        element: line.element,
                        source: line.source,
                    });
                    if let Some(element_lines) = self.title_lines_by_element.get_mut(line.element) {
                        element_lines.push(index);
                    }
                    continue;
                }
                let absolute_y = i64::try_from(page_index)
                    .unwrap_or(i64::MAX)
                    .saturating_mul(page_height)
                    .saturating_add(line.geometry.y.raw());
                let index = self.body_line_index.len();
                self.body_line_index.push(BodyLineIndexEntry {
                    location,
                    absolute_y,
                    element: line.element,
                    source: line.source,
                });
                if let Some(element_lines) = self.body_lines_by_element.get_mut(line.element) {
                    element_lines.push(index);
                }
            }
        }
        self.body_line_y_order = (0..self.body_line_index.len()).collect();
        self.body_line_y_order.sort_by_key(|index| {
            let entry = self.body_line_index[*index];
            (entry.absolute_y, entry.location.page, entry.location.line)
        });
        if let (Some(diagnostics), Some(started)) = (diagnostics, started) {
            diagnostics.interaction_index_build_nanos = diagnostics
                .interaction_index_build_nanos
                .saturating_add(started.elapsed().as_nanos());
            diagnostics.interaction_index_entries = diagnostics
                .interaction_index_entries
                .saturating_add(self.body_line_index.len() as u64 + self.title_line_index.len() as u64);
            diagnostics.interaction_index_entries_sorted = diagnostics
                .interaction_index_entries_sorted
                .saturating_add(self.body_line_y_order.len() as u64);
        }
    }

    fn new_page(&self, title_page: bool) -> LayoutPage {
        LayoutPage {
            number: self.pages.len(),
            title_page,
            bounds: Rect {
                x: 0,
                y: 0,
                width: self.inputs.paper_width,
                height: self.inputs.paper_height,
            },
            content_bounds: Rect {
                x: self.inputs.margin_left,
                y: self.inputs.margin_top,
                width: self.inputs.paper_width - self.inputs.margin_left - self.inputs.margin_right,
                height: self.inputs.paper_height
                    - self.inputs.margin_top
                    - self.inputs.margin_bottom,
            },
            lines: Vec::new(),
            occupied_lines: 0,
            next_line_y: FixedPoint::from_points(i64::from(self.inputs.margin_top)),
        }
    }

    fn make_line(
        &self,
        page: &LayoutPage,
        line: WrappedLine,
        title_page: bool,
        y: FixedPoint,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> LayoutLine {
        let indent = if line.first_visual_line {
            line.paragraph.first_indent
        } else {
            FixedPoint::default()
        };
        let line_left =
            FixedPoint::from_raw(line.paragraph.left.raw().saturating_add(indent.raw()));
        let line_width =
            FixedPoint::from_raw(line.paragraph.right.raw().saturating_sub(line_left.raw()));
        let span_started = diagnostics.as_ref().map(|_| Instant::now());
        let span_characters = if diagnostics.is_some() {
            line.display_text.chars().count() as u64
        } else {
            0
        };
        let mut diagnostics = diagnostics;
        let mut spans = positioned_spans(
            &line,
            FixedRect {
                x: line_left,
                y,
                width: FixedPoint::default(),
                height: line.paragraph.line_advance,
            },
            diagnostics.as_deref_mut(),
        );
        if let (Some(diagnostics), Some(started)) = (diagnostics.as_deref_mut(), span_started) {
            diagnostics.positioned_span_nanos = diagnostics
                .positioned_span_nanos
                .saturating_add(started.elapsed().as_nanos());
            diagnostics.positioned_span_characters = diagnostics
                .positioned_span_characters
                .saturating_add(span_characters);
        }
        let text_width = FixedPoint::from_raw(
            spans.iter().map(|span| span.geometry.width.raw()).sum::<i64>(),
        );
        let spare = line_width.raw().saturating_sub(text_width.raw()).max(0);
        let display_chars = line.display_text.chars().collect::<Vec<_>>();
        let justified_gaps = if line.paragraph.alignment == ParagraphAlignment::Justified
            && !line.final_visual_line
        {
            (1..display_chars.len().saturating_sub(1))
                .filter(|index| {
                    display_chars[*index].is_whitespace()
                        && !display_chars[*index - 1].is_whitespace()
                        && !display_chars[*index + 1].is_whitespace()
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let distribute = !justified_gaps.is_empty();
        let distributed_width = if distribute { spare } else { 0 };
        let gap_width = if distribute { spare.div_euclid(justified_gaps.len() as i64) } else { 0 };
        let gap_remainder = if distribute { spare.rem_euclid(justified_gaps.len() as i64) as usize } else { 0 };
        let text_x = match line.paragraph.alignment {
            ParagraphAlignment::Left => line_left,
            ParagraphAlignment::Right => {
                FixedPoint::from_raw(line.paragraph.right.raw().saturating_sub(text_width.raw()))
            }
            ParagraphAlignment::Center => {
                FixedPoint::from_raw(line_left.raw().saturating_add(spare / 2))
            }
            ParagraphAlignment::Justified => line_left,
        };
        let alignment_shift = text_x.raw().saturating_sub(line_left.raw());
        for span in &mut spans {
            span.geometry.x = FixedPoint::from_raw(span.geometry.x.raw().saturating_add(alignment_shift));
        }
        let height = line.paragraph.line_advance;
        let paragraph_bounds = FixedRect {
            x: line.paragraph.left,
            y,
            width: FixedPoint::from_raw(
                line.paragraph
                    .right
                    .raw()
                    .saturating_sub(line.paragraph.left.raw()),
            ),
            height,
        };
        let geometry = FixedRect {
            x: text_x,
            y,
            width: FixedPoint::from_raw(text_width.raw().saturating_add(distributed_width)),
            height,
        };
        if distribute {
            let gap_adjustment = |index: usize| {
                justified_gaps.iter().position(|gap| *gap == index).map(|rank| {
                    gap_width.saturating_add(if rank < gap_remainder { 1 } else { 0 })
                }).unwrap_or(0)
            };
            let mut display_index = 0usize;
            let mut shift = 0i64;
            for span in &mut spans {
                let count = span.display_text.chars().count();
                let internal = (display_index..display_index.saturating_add(count))
                    .map(gap_adjustment).sum::<i64>();
                span.geometry.x = FixedPoint::from_raw(span.geometry.x.raw().saturating_add(shift));
                span.geometry.width = FixedPoint::from_raw(span.geometry.width.raw().saturating_add(internal));
                shift = shift.saturating_add(internal);
                display_index = display_index.saturating_add(count);
            }
        }
        LayoutLine {
            page: page.number,
            role: line.role,
            element: line.element,
            source: SourceRange {
                element: line.element,
                start: line.start,
                end: line.end,
            },
            text: line.text,
            display_text: line.display_text,
            display_source_map: line.display_source_map,
            spans: spans.into(),
            text_style: line.text_style,
            char_width: line.char_width,
            alignment: line.paragraph.alignment,
            unaligned_line_left: line_left,
            geometry,
            paragraph_bounds,
            rect: compat_rect(paragraph_bounds),
            keep_with_next: !title_page && keep_with_next(line.role),
            oversized: line.oversized,
        }
    }

    fn make_owner_line(
        &self,
        page: &LayoutPage,
        owner: &OwnerLayout,
        line_index: usize,
        title_page: bool,
        y: FixedPoint,
        oversized: bool,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> LayoutLine {
        match owner {
            OwnerLayout::Wrapped { lines, .. } => {
                let mut line = self.make_line(
                    page,
                    lines[line_index].clone(),
                    title_page,
                    y,
                    diagnostics,
                );
                line.oversized |= oversized;
                line
            }
            OwnerLayout::Prepared { element, layout } => {
                let prepared = &layout.lines[line_index];
                let owner_paragraph = layout.identity.context.paragraph;
                let paragraph_left = owner_paragraph.left;
                let paragraph_bounds = FixedRect {
                    x: paragraph_left,
                    y,
                    width: prepared.paragraph_bounds.width,
                    height: prepared.paragraph_bounds.height,
                };
                let geometry = FixedRect {
                    x: FixedPoint::from_raw(
                        paragraph_left.raw().saturating_add(prepared.geometry.x.raw()),
                    ),
                    y,
                    width: prepared.geometry.width,
                    height: prepared.geometry.height,
                };
                let spans = prepared
                    .spans
                    .iter()
                    .cloned()
                    .map(|mut span| {
                        span.source.element = *element;
                        span.geometry.x = FixedPoint::from_raw(
                            span.geometry.x.raw().saturating_add(paragraph_left.raw()),
                        );
                        span.geometry.y = y;
                        span
                    })
                    .collect::<Vec<_>>();
                let line = LayoutLine {
                    page: page.number,
                    role: prepared.role,
                    element: *element,
                    source: SourceRange {
                        element: *element,
                        start: prepared.source_start,
                        end: prepared.source_end,
                    },
                    text: prepared.text.clone(),
                    display_text: prepared.display_text.clone(),
                    display_source_map: prepared.display_source_map.clone(),
                    spans: spans.into(),
                    text_style: prepared.text_style.clone(),
                    char_width: prepared.char_width,
                    alignment: prepared.paragraph.alignment,
                    unaligned_line_left: FixedPoint::from_raw(
                        paragraph_left
                            .raw()
                            .saturating_add(prepared.line_left_offset.raw()),
                    ),
                    geometry,
                    paragraph_bounds,
                    rect: compat_rect(paragraph_bounds),
                    keep_with_next: !title_page && keep_with_next(prepared.role),
                    oversized: prepared.oversized || oversized,
                };
                line
            }
        }
    }

    fn place_dual_side(
        &self,
        page: &LayoutPage,
        side: &WrappedDualSide,
        block_top: FixedPoint,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Vec<LayoutLine> {
        let mut diagnostics = diagnostics;
        let mut y = block_top;
        let mut placed = Vec::with_capacity(side.line_count);
        for (paragraph_index, paragraph_lines) in side.paragraphs.iter().enumerate() {
            if paragraph_index > 0 && paragraph_lines.len() > 0 {
                y = FixedPoint::from_raw(y.raw().saturating_add(
                    paragraph_lines.paragraph(0).space_before.raw().max(0),
                ));
            }
            for line_index in 0..paragraph_lines.len() {
                let advance = paragraph_lines.paragraph(line_index).line_advance;
                let mut line = self.make_owner_line(
                    page,
                    paragraph_lines,
                    line_index,
                    false,
                    y,
                    false,
                    diagnostics.as_deref_mut(),
                );
                line.keep_with_next = false;
                placed.push(line);
                y = FixedPoint::from_raw(y.raw().saturating_add(advance.raw()));
            }
        }
        placed
    }

    fn place_dual_block(
        &mut self,
        left: &WrappedDualSide,
        right: &WrappedDualSide,
        starts_new_page: bool,
        space_before: FixedPoint,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) {
        let mut diagnostics = diagnostics;
        if self.pages.is_empty() || self.pages.last().is_some_and(|page| page.title_page) {
            self.pages.push(self.new_page(false));
        }
        if starts_new_page
            && self.pages.last().is_some_and(|page| !page.lines.is_empty())
        {
            self.pages.push(self.new_page(false));
        }

        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let page_top = FixedPoint::from_points(i64::from(self.inputs.margin_top));
        let content_height = content_bottom.raw().saturating_sub(page_top.raw());
        let capacity = self.inputs.lines_per_page as usize;
        let block_height = left.height.raw().max(right.height.raw()).max(0);
        let required_lines = left.line_count.max(right.line_count);
        // A saturated sum remains larger than any validated printable extent,
        // so overflow cannot be mistaken for a fitting block.
        let fits_empty_page = block_height <= content_height
            && required_lines <= capacity;

        let current_page = self.pages.len() - 1;
        let has_boundary = !self.pages[current_page].lines.is_empty();
        let applied_gap = if has_boundary {
            space_before.raw().max(0)
        } else {
            0
        };
        let block_top = FixedPoint::from_raw(
            self.pages[current_page]
                .next_line_y
                .raw()
                .saturating_add(applied_gap),
        );
        let vertical_end = block_top.raw().saturating_add(block_height);
        let line_end = (self.pages[current_page].occupied_lines as usize)
            .saturating_add(required_lines);
        let fits_remaining = line_end <= capacity && vertical_end <= content_bottom.raw();

        if !fits_empty_page {
            // The exception is limited to a block that cannot fit on a fresh
            // page. Start it there once; each fallback line is consumed exactly
            // once and a page is advanced only after a prior line made progress.
            if !self.pages[current_page].lines.is_empty() {
                self.pages.push(self.new_page(false));
            }
            self.spill_oversized_dual_block(left, right, diagnostics.as_deref_mut());
            return;
        }

        let page_index = if !fits_remaining && !self.pages[current_page].lines.is_empty() {
            self.pages.push(self.new_page(false));
            self.pages.len() - 1
        } else {
            current_page
        };
        let applied_gap = if self.pages[page_index].lines.is_empty() {
            0
        } else {
            applied_gap
        };
        let block_top = FixedPoint::from_raw(
            self.pages[page_index]
                .next_line_y
                .raw()
                .saturating_add(applied_gap),
        );
        let page = self.pages[page_index].clone();
        let left_lines = self.place_dual_side(&page, left, block_top, diagnostics.as_deref_mut());
        let right_lines = self.place_dual_side(&page, right, block_top, diagnostics.as_deref_mut());
        self.pages[page_index].lines.extend(left_lines);
        self.pages[page_index].lines.extend(right_lines);
        self.pages[page_index].occupied_lines = self.pages[page_index]
            .occupied_lines
            .saturating_add(required_lines.min(u32::MAX as usize) as u32);
        self.pages[page_index].next_line_y = FixedPoint::from_raw(
            block_top.raw().saturating_add(block_height),
        );
    }

    fn spill_oversized_dual_block(
        &mut self,
        left: &WrappedDualSide,
        right: &WrappedDualSide,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) {
        let mut diagnostics = diagnostics;
        let start_page = self.pages.len() - 1;
        let capacity = self.inputs.lines_per_page as usize;
        let page_top = FixedPoint::from_points(i64::from(self.inputs.margin_top));
        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let left_placements = spill_side_lines(left, page_top, content_bottom, capacity);
        let right_placements = spill_side_lines(right, page_top, content_bottom, capacity);
        let page_count = left_placements
            .page_count
            .max(right_placements.page_count)
            .max(1);
        while self.pages.len() < start_page.saturating_add(page_count) {
            self.pages.push(self.new_page(false));
        }

        let mut occupied = vec![0usize; page_count];
        let mut bottoms = vec![page_top.raw(); page_count];
        for placements in [&left_placements, &right_placements] {
            for (page_offset, count) in placements.lines_per_page.iter().enumerate() {
                occupied[page_offset] = occupied[page_offset].max(*count);
            }
            for (page_offset, bottom) in placements.bottoms.iter().enumerate() {
                bottoms[page_offset] = bottoms[page_offset].max(*bottom);
            }
        }

        // Append every left line before every right line, including pages where
        // an oversized side has spilled forward.
        for placements in [&left_placements, &right_placements] {
            for placement in &placements.lines {
                let page_index = start_page.saturating_add(placement.page_offset);
                let page = self.pages[page_index].clone();
                let mut line = self.make_owner_line(
                    &page,
                    &placement.owner,
                    placement.line_index,
                    false,
                    placement.y,
                    true,
                    diagnostics.as_deref_mut(),
                );
                line.keep_with_next = false;
                self.pages[page_index].lines.push(line);
            }
        }
        for page_offset in 0..page_count {
            let page = &mut self.pages[start_page + page_offset];
            page.occupied_lines = page
                .occupied_lines
                .saturating_add(occupied[page_offset].min(u32::MAX as usize) as u32);
            page.next_line_y = FixedPoint::from_raw(bottoms[page_offset]);
        }
    }

    fn place_element(
        &mut self,
        lines: OwnerLayout,
        role: Role,
        paragraph: EffectiveParagraph,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) {
        let mut diagnostics = diagnostics;
        let keep = keep_with_next(role);
        let minimum = if keep { 2 } else { 1 };
        if self.pages.is_empty() {
            self.pages.push(self.new_page(false));
        }
        let capacity = self.inputs.lines_per_page as usize;
        let has_boundary = self.pages.last().is_some_and(|page| !page.lines.is_empty());
        let extra_gap = if has_boundary {
            paragraph.space_before.raw().max(0)
        } else {
            0
        };
        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let minimum_height = paragraph.line_advance.raw().saturating_mul(minimum as i64);
        let vertical_fit = self.pages.last().is_some_and(|page| {
            page.next_line_y
                .raw()
                .saturating_add(extra_gap)
                .saturating_add(minimum_height)
                <= content_bottom.raw()
        });
        if lines.len() > 0
            && !self.pages.last().is_some_and(|page| page.title_page)
            && has_boundary
            && (self
                .pages
                .last()
                .map_or(0, |page| page.occupied_lines as usize)
                .saturating_add(minimum)
                > capacity
                || !vertical_fit)
        {
            self.pages.push(self.new_page(false));
        }
        let applied_gap =
            if has_boundary && self.pages.last().is_some_and(|page| !page.lines.is_empty()) {
                if let Some(page) = self.pages.last_mut() {
                    page.next_line_y =
                        FixedPoint::from_raw(page.next_line_y.raw().saturating_add(extra_gap));
                }
                extra_gap
            } else {
                0
            };
        let total_height = paragraph
            .line_advance
            .raw()
            .saturating_mul(lines.len().min(i64::MAX as usize) as i64)
            .saturating_add(applied_gap);
        let content_height =
            content_bottom.raw() - FixedPoint::from_points(i64::from(self.inputs.margin_top)).raw();
        let oversized_element = lines.len() > capacity || total_height > content_height;
        for line_index in 0..lines.len() {
            let current_fits = self.pages.last().is_some_and(|page| {
                (page.occupied_lines as usize) < capacity
                    && (page.lines.is_empty()
                        || page
                            .next_line_y
                            .raw()
                            .saturating_add(paragraph.line_advance.raw())
                            <= content_bottom.raw())
            });
            if !current_fits {
                self.pages.push(self.new_page(false));
            }
            let page_index = self.pages.len() - 1;
            if self.pages[page_index].lines.is_empty() {
                self.pages[page_index].occupied_lines = 0;
            }
            let page = self.pages[page_index].clone();
            let line = self.make_owner_line(
                &page,
                &lines,
                line_index,
                false,
                page.next_line_y,
                oversized_element,
                diagnostics.as_deref_mut(),
            );
            self.pages[page_index].lines.push(line);
            self.pages[page_index].occupied_lines =
                self.pages[page_index].occupied_lines.saturating_add(1);
            self.pages[page_index].next_line_y = FixedPoint::from_raw(
                self.pages[page_index]
                    .next_line_y
                    .raw()
                    .saturating_add(paragraph.line_advance.raw()),
            );
        }
    }
}

fn display_char_for_x(line: &LayoutLine, x: FixedPoint) -> usize {
    if line.spans.is_empty() {
        let cell = line.char_width.raw().max(1);
        let units = x.raw().saturating_sub(line.geometry.x.raw()).max(0);
        return (units.saturating_add(cell / 2) / cell) as usize;
    }
    let mut display_offset = 0usize;
    for span in line.spans.iter() {
        let count = span.display_text.chars().count();
        let cell = if count == 0 { span.char_width.raw().max(1) }
            else { span.geometry.width.raw().div_euclid(count as i64).max(1) };
        if x.raw() <= span.geometry.x.raw().saturating_add(span.geometry.width.raw()) {
            let units = x.raw().saturating_sub(span.geometry.x.raw()).max(0);
            return display_offset.saturating_add((units.saturating_add(cell / 2).div_euclid(cell)) as usize).min(display_offset.saturating_add(count));
        }
        display_offset = display_offset.saturating_add(count);
    }
    display_offset
}

fn x_for_display_char(line: &LayoutLine, display_char: usize) -> i64 {
    let mut display_offset = 0usize;
    for span in line.spans.iter() {
        let count = span.display_text.chars().count();
        if display_char <= display_offset.saturating_add(count) {
            let local = display_char.saturating_sub(display_offset).min(count) as i64;
            let cell = if count == 0 { span.char_width.raw() }
                else { span.geometry.width.raw().div_euclid(count as i64) };
            return span.geometry.x.raw().saturating_add(local.saturating_mul(cell));
        }
        display_offset = display_offset.saturating_add(count);
    }
    line.geometry.x.raw().saturating_add(line.geometry.width.raw())
}

struct PositionedSpanBuilder {
    display_text: String,
    display_source_map: Vec<DisplaySourceSegment>,
    source: SourceRange,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    geometry: FixedRect,
    underline: bool,
    strikethrough: bool,
    last_character: char,
}

impl PositionedSpanBuilder {
    fn new(
        line: &WrappedLine,
        character: char,
        source_start: usize,
        source_end: usize,
        style: EffectiveTextStyle,
        strikethrough: bool,
        cell: FixedPoint,
        x: i64,
        line_geometry: FixedRect,
    ) -> Self {
        let mut display_text = String::new();
        display_text.push(character);
        Self {
            display_text,
            display_source_map: vec![DisplaySourceSegment {
                display_start: 0,
                display_end: 1,
                source_start,
                source_end,
            }],
            source: SourceRange {
                element: line.element,
                start: source_start,
                end: source_end,
            },
            underline: style.underline,
            strikethrough,
            text_style: style,
            char_width: cell,
            geometry: FixedRect {
                x: FixedPoint::from_raw(x),
                y: line_geometry.y,
                width: cell,
                height: line_geometry.height,
            },
            last_character: character,
        }
    }

    fn append(&mut self, character: char, source_start: usize, source_end: usize, cell: FixedPoint) {
        let display_start = self.display_source_map.len();
        self.display_text.push(character);
        self.display_source_map.push(DisplaySourceSegment {
            display_start,
            display_end: display_start.saturating_add(1),
            source_start,
            source_end,
        });
        self.source.end = source_end;
        self.geometry.width = FixedPoint::from_raw(
            self.geometry.width.raw().saturating_add(cell.raw()),
        );
        self.char_width = cell;
        self.last_character = character;
    }

    fn finish(self) -> PositionedStyledSpan {
        PositionedStyledSpan {
            display_text: Arc::from(self.display_text),
            source: self.source,
            display_source_map: self.display_source_map.into(),
            text_style: self.text_style,
            char_width: self.char_width,
            geometry: self.geometry,
            underline: self.underline,
            strikethrough: self.strikethrough,
        }
    }
}

fn positioned_spans(
    line: &WrappedLine,
    line_geometry: FixedRect,
    mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
) -> Vec<PositionedStyledSpan> {
    let base_size = i64::from(line.text_style.size.max(1));
    let mut map_cursor = 0usize;
    let mut run_cursor = 0usize;
    let mut run_start = 0usize;
    let mut run_end = line.runs.first().map_or(0, |run| run.text.len());
    let mut active_run: Option<Option<usize>> = None;
    let mut active_style: Option<(EffectiveTextStyle, RunStyleFlags)> = None;
    let mut current: Option<PositionedSpanBuilder> = None;
    let mut output = Vec::new();
    let mut x = line_geometry.x.raw();

    for (display_index, character) in line.display_text.chars().enumerate() {
        while let Some(mapping) = line.display_source_map.get(map_cursor) {
            if diagnostics.is_some() {
                if let Some(diagnostics) = diagnostics.as_deref_mut() {
                    diagnostics.source_map_segments_examined = diagnostics
                        .source_map_segments_examined
                        .saturating_add(1);
                }
            }
            if display_index < mapping.display_end {
                break;
            }
            map_cursor = map_cursor.saturating_add(1);
        }
        let Some(mapping) = line.display_source_map.get(map_cursor).filter(|mapping| {
            display_index >= mapping.display_start && display_index < mapping.display_end
        }) else {
            continue;
        };

        while run_cursor < line.runs.len() && mapping.source_start >= run_end {
            run_start = run_end;
            run_cursor = run_cursor.saturating_add(1);
            run_end = run_end.saturating_add(
                line.runs.get(run_cursor).map_or(0, |run| run.text.len()),
            );
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.run_range_searches = diagnostics.run_range_searches.saturating_add(1);
            }
        }
        let run_index = (run_cursor < line.runs.len()
            && mapping.source_start >= run_start
            && mapping.source_start < run_end)
            .then_some(run_cursor);
        if active_run != Some(run_index) {
            let mut style = line.text_style.clone();
            let flags = run_index
                .and_then(|index| line.runs.get(index))
                .map(|run| {
                    if let Some(diagnostics) = diagnostics.as_deref_mut() {
                        diagnostics.style_attribute_parses = diagnostics
                            .style_attribute_parses
                            .saturating_add(1);
                    }
                    let flags = parse_style_attribute(&run.attributes)
                        .ok()
                        .map(|projection| projection.flags)
                        .unwrap_or_default();
                    style.bold |= flags.bold;
                    style.italic |= flags.italics;
                    style.underline |= flags.underline;
                    style.all_caps |= flags.all_caps;
                    for attribute in &run.attributes {
                        match attribute.name.as_str() {
                            "Font" if !attribute.value.is_empty() => {
                                style.declared_family = attribute.value.clone();
                                style.family = FontFallback::CourierPrime;
                            }
                            "Size" => {
                                if let Ok(size) = attribute.value.parse::<u8>() {
                                    if (1..=72).contains(&size) {
                                        style.size = size;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    flags
                })
                .unwrap_or_default();
            active_style = Some((style, flags));
            active_run = Some(run_index);
        }
        let (style, flags) = active_style
            .as_ref()
            .expect("a mapped character has an active run style");
        let cell = FixedPoint::from_raw(
            line.char_width
                .raw()
                .saturating_mul(i64::from(style.size.max(1)))
                .div_euclid(base_size)
                .max(1),
        );
        let same = current.as_ref().is_some_and(|span| {
            span.text_style == *style
                && span.underline == style.underline
                && span.strikethrough == flags.strikethrough
                && !character.is_whitespace()
                && !span.last_character.is_whitespace()
                && (span.source.end == mapping.source_start
                    || (span.source.start == mapping.source_start
                        && span.source.end == mapping.source_end))
        });
        if same {
            current.as_mut().expect("checked span exists").append(
                character,
                mapping.source_start,
                mapping.source_end,
                cell,
            );
        } else {
            if let Some(span) = current.take() {
                output.push(span.finish());
            }
            current = Some(PositionedSpanBuilder::new(
                line,
                character,
                mapping.source_start,
                mapping.source_end,
                style.clone(),
                flags.strikethrough,
                cell,
                x,
                line_geometry,
            ));
        }
        x = x.saturating_add(cell.raw());
    }
    if let Some(span) = current {
        output.push(span.finish());
    }
    output
}

#[derive(Debug, Clone)]
struct WrappedLine {
    element: usize,
    role: Role,
    start: usize,
    end: usize,
    text: Arc<str>,
    display_text: Arc<str>,
    display_source_map: Arc<[DisplaySourceSegment]>,
    text_style: EffectiveTextStyle,
    runs: Arc<[TextRun]>,
    char_width: FixedPoint,
    first_visual_line: bool,
    final_visual_line: bool,
    paragraph: EffectiveParagraph,
    oversized: bool,
}

#[derive(Debug, Clone)]
enum OwnerLayout {
    Wrapped { lines: Arc<[WrappedLine]> },
    Prepared {
        element: usize,
        layout: Arc<PreparedOwnerLayout>,
    },
}

impl OwnerLayout {
    fn len(&self) -> usize {
        match self {
            Self::Wrapped { lines, .. } => lines.len(),
            Self::Prepared { layout, .. } => layout.lines.len(),
        }
    }

    fn paragraph(&self, line_index: usize) -> EffectiveParagraph {
        match self {
            Self::Wrapped { lines, .. } => lines[line_index].paragraph,
            Self::Prepared { layout, .. } => layout.lines[line_index].paragraph,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EffectiveParagraph {
    left: FixedPoint,
    right: FixedPoint,
    first_indent: FixedPoint,
    alignment: ParagraphAlignment,
    space_before: FixedPoint,
    line_advance: FixedPoint,
    char_width: FixedPoint,
}

struct WrappedDualSide {
    paragraphs: Vec<OwnerLayout>,
    height: FixedPoint,
    line_count: usize,
}

struct SpilledDualLine {
    owner: OwnerLayout,
    line_index: usize,
    page_offset: usize,
    y: FixedPoint,
}

struct SpilledDualSide {
    lines: Vec<SpilledDualLine>,
    lines_per_page: Vec<usize>,
    bottoms: Vec<i64>,
    page_count: usize,
}

fn spill_side_lines(
    side: &WrappedDualSide,
    page_top: FixedPoint,
    content_bottom: FixedPoint,
    capacity: usize,
) -> SpilledDualSide {
    let mut result = SpilledDualSide {
        lines: Vec::with_capacity(side.line_count),
        lines_per_page: vec![0],
        bottoms: vec![page_top.raw()],
        page_count: 1,
    };
    let mut page_offset = 0usize;
    let mut y = page_top;

    for (paragraph_index, paragraph_lines) in side.paragraphs.iter().enumerate() {
        let mut gap_pending = paragraph_index > 0;
        for line_index in 0..paragraph_lines.len() {
            let paragraph = paragraph_lines.paragraph(line_index);
            let advance = paragraph.line_advance.raw().max(0);
            let current_count = result.lines_per_page[page_offset];
            let gap = if gap_pending && current_count > 0 {
                paragraph.space_before.raw().max(0)
            } else {
                0
            };
            let proposed_y = FixedPoint::from_raw(y.raw().saturating_add(gap));
            let proposed_end = proposed_y.raw().saturating_add(advance);
            let fits_page = current_count < capacity && proposed_end <= content_bottom.raw();

            if !fits_page && current_count > 0 {
                page_offset = page_offset.saturating_add(1);
                if result.lines_per_page.len() <= page_offset {
                    result.lines_per_page.push(0);
                    result.bottoms.push(page_top.raw());
                }
                result.page_count = page_offset.saturating_add(1);
                y = page_top;
                gap_pending = false;
            }

            let count = result.lines_per_page[page_offset];
            let line_gap = if gap_pending && count > 0 {
                paragraph.space_before.raw().max(0)
            } else {
                0
            };
            let line_y = FixedPoint::from_raw(y.raw().saturating_add(line_gap));
            let line_bottom = line_y.raw().saturating_add(advance);
            result.lines.push(SpilledDualLine {
                owner: paragraph_lines.clone(),
                line_index,
                page_offset,
                y: line_y,
            });
            result.lines_per_page[page_offset] = count.saturating_add(1);
            result.bottoms[page_offset] = result.bottoms[page_offset].max(line_bottom);
            y = FixedPoint::from_raw(line_bottom);
            gap_pending = false;
        }
    }

    result
}

fn wrapper_overrides(
    project: &Project,
    source: &SourceLayoutSettings,
    group_id: &StableId,
) -> crate::formatting::SourceParagraphOverrides {
    project
        .dual_dialogue_paragraph_overrides
        .get(group_id)
        .copied()
        .or_else(|| source.direct_paragraph_overrides.get(group_id).copied())
        .unwrap_or_default()
}

fn active_group_boundaries(
    layout: &ScreenplayLayout,
    project: &Project,
    group: &crate::dual_dialogue::DualDialogueGroup,
) -> Vec<EditorGroupBoundary> {
    let Some(first_element) = crate::dual_dialogue::first_member(project, group) else {
        return Vec::new();
    };
    let Some(last_element) = crate::dual_dialogue::last_member(project, group) else {
        return Vec::new();
    };
    let first = layout.pages.iter().enumerate().find_map(|(page_index, page)| {
        page.lines.iter().find(|line| line.element == first_element).map(|line| (page_index, line))
    });
    let last = layout.pages.iter().enumerate().rev().find_map(|(page_index, page)| {
        page.lines.iter().rev().find(|line| line.element == last_element).map(|line| (page_index, line))
    });
    let (Some((upper_page, first_line)), Some((lower_page, last_line))) = (first, last) else {
        return Vec::new();
    };
    let content = &layout.pages[upper_page].content_bounds;
    let x = FixedPoint::from_points(i64::from(content.x));
    let width = FixedPoint::from_points(i64::from(content.width));
    let upper = FixedRect {
        x,
        y: first_line.paragraph_bounds.y,
        width,
        height: FixedPoint::default(),
    };
    let lower = FixedRect {
        x,
        y: FixedPoint::from_raw(
            last_line.paragraph_bounds.y.raw().saturating_add(last_line.paragraph_bounds.height.raw()),
        ),
        width,
        height: FixedPoint::default(),
    };
    vec![EditorGroupBoundary {
        group_id: group.id.clone(),
        upper_page,
        upper,
        lower_page,
        lower,
    }]
}

fn dual_outer_extent(
    inputs: &LayoutInputs,
    source: &SourceLayoutSettings,
    project: &Project,
    group: &crate::dual_dialogue::DualDialogueGroup,
) -> Result<(FixedPoint, FixedPoint), LayoutError> {
    let native = formatting_for_role(Role::General).paragraph;
    let app_left = FixedPoint::from_points(i64::from(inputs.margin_left));
    let app_right = FixedPoint::from_points(i64::from(
        inputs.paper_width.saturating_sub(inputs.margin_right),
    ));
    let general = source.paragraph_spec(Role::General);
    let role_left = general
        .left_indent
        .unwrap_or_else(|| FixedPoint::parse_inches(native.left_indent).unwrap_or(app_left));
    let role_right = general
        .right_indent
        .unwrap_or_else(|| FixedPoint::parse_inches(native.right_indent).unwrap_or(app_right));
    let wrapper = wrapper_overrides(project, source, &group.id).paragraph;
    let left = wrapper.left_indent.unwrap_or(role_left);
    let right = wrapper.right_indent.unwrap_or(role_right);
    let page_right = FixedPoint::from_points(i64::from(inputs.paper_width));
    let gap = FixedPoint::from_points(DUAL_DIALOGUE_COLUMN_GAP_POINTS);
    if left.raw() < 0
        || right <= left
        || right > page_right
        || right.raw().saturating_sub(left.raw()) <= gap.raw()
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok((left, right))
}

#[allow(clippy::too_many_arguments)]
fn wrap_dual_side(
    project: &Project,
    side: &crate::dual_dialogue::DualDialogueSide,
    column: (FixedPoint, FixedPoint),
    inputs: &LayoutInputs,
    source: &SourceLayoutSettings,
    legacy_json_profile: bool,
    purpose: LayoutPurpose,
    cache: &mut Option<&mut ElementWrapCache>,
    prepared_cache: &mut Option<&mut PreparedOwnerCache>,
    composition: LayoutComposition,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) -> WrappedDualSide {
    let mut paragraphs = Vec::new();
    let mut height = 0i64;
    let mut line_count = 0usize;
    for id in side.member_ids() {
        let Some((index, element)) = project
            .elements
            .iter()
            .enumerate()
            .find(|(_, element)| &element.id == id)
        else {
            continue;
        };
        let text_style = source.text_style(element.role);
        if purpose == LayoutPurpose::Print && text_style.hidden_text {
            continue;
        }
        let mut paragraph = effective_paragraph(
            inputs,
            source,
            element.role,
            &element.id,
            legacy_json_profile,
            &text_style,
        );
        paragraph.left = column.0;
        paragraph.right = column.1;
        paragraph.first_indent = FixedPoint::default();
        if element.role == Role::Character {
            paragraph.alignment = ParagraphAlignment::Center;
        }
        let lines = owner_layout_for_layout(
            cache,
            prepared_cache,
            diagnostics,
            composition,
            &element.id,
            &element.text,
            index,
            element.role,
            paragraph,
            text_style,
            &element.runs,
            paragraph.char_width,
            purpose,
            false,
            EffectiveGeometryMode::DualColumn,
        );
        if !paragraphs.is_empty() {
            height = height.saturating_add(paragraph.space_before.raw().max(0));
        }
        height = height.saturating_add(
            paragraph
            .line_advance
            .raw()
            .saturating_mul(lines.len().min(i64::MAX as usize) as i64),
        );
        line_count = line_count.saturating_add(lines.len());
        paragraphs.push(lines);
    }
    WrappedDualSide {
        paragraphs,
        height: FixedPoint::from_raw(height),
        line_count,
    }
}

fn validate_profile(inputs: &LayoutInputs) -> Result<(), LayoutError> {
    if inputs.margin_left.saturating_add(inputs.margin_right) >= inputs.paper_width
        || inputs.margin_top.saturating_add(inputs.margin_bottom) >= inputs.paper_height
        || inputs.lines_per_page == 0
        || inputs.chars_per_line == 0
        || inputs.line_advance == 0
        || inputs
            .role_boxes
            .iter()
            .any(|box_| box_.width == 0 || box_.x.saturating_add(box_.width) > inputs.paper_width)
        || inputs
            .additional_role_boxes
            .iter()
            .flatten()
            .any(|box_| box_.width == 0 || box_.x.saturating_add(box_.width) > inputs.paper_width)
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok(())
}

fn effective_paragraph(
    inputs: &LayoutInputs,
    source: &SourceLayoutSettings,
    role: Role,
    element_id: &StableId,
    legacy_json_profile: bool,
    text_style: &EffectiveTextStyle,
) -> EffectiveParagraph {
    let native = formatting_for_role(role).paragraph;
    let fallback_left = FixedPoint::from_points(i64::from(inputs.margin_left));
    let fallback_right = FixedPoint::from_points(i64::from(
        inputs.paper_width.saturating_sub(inputs.margin_right),
    ));
    let default_left = FixedPoint::parse_inches(native.left_indent).unwrap_or(fallback_left);
    let default_right = FixedPoint::parse_inches(native.right_indent).unwrap_or(fallback_right);
    let projected = source.paragraph_spec(role);
    let direct = source.direct_paragraph_spec(element_id).paragraph;
    let mut left = projected.left_indent.unwrap_or(default_left);
    let mut right = projected.right_indent.unwrap_or(default_right);

    // Legacy JSON's explicit role boxes remain a transient compatibility input.
    // The reference-derived defaults, rather than those historical boxes, drive
    // all FDX sessions and ordinary new documents.
    let legacy_defaults = LayoutInputs::default();
    let role_box = inputs.role_box(role);
    let legacy_box = role_box
        .filter(|box_| legacy_json_profile || (legacy_defaults.role_box(role) != Some(*box_)));
    if let Some(box_) = legacy_box {
        left = FixedPoint::from_points(i64::from(box_.x));
        right = FixedPoint::from_points(i64::from(box_.x.saturating_add(box_.width)));
    }

    let page_right = FixedPoint::from_points(i64::from(inputs.paper_width));
    if left.raw() < 0 || right <= left || right > page_right {
        left = default_left;
        right = default_right;
    }
    if left.raw() < 0 || right <= left || right > page_right {
        left = fallback_left;
        right = fallback_right;
    }

    // Direct bounds are a pair at the geometry-validation boundary. If their
    // combination is unusable, discard that direct pair and retain the already
    // resolved role/default pair rather than losing a valid role override.
    let direct_left = direct.left_indent.unwrap_or(left);
    let direct_right = direct.right_indent.unwrap_or(right);
    if direct_left.raw() >= 0 && direct_right > direct_left && direct_right <= page_right {
        left = direct_left;
        right = direct_right;
    }

    let default_indent = FixedPoint::parse_inches(native.first_indent).unwrap_or_default();
    let char_width = i64::from(inputs.char_width.max(1)).saturating_mul(FixedPoint::SCALE);
    let usable = right.raw().saturating_sub(left.raw());
    let indent_fits = |indent: FixedPoint| {
        left.raw().saturating_add(indent.raw()) >= 0
            && usable.saturating_sub(indent.raw()) >= char_width
    };
    let role_indent = projected.first_indent.unwrap_or(default_indent);
    let role_indent = if indent_fits(role_indent) {
        role_indent
    } else if indent_fits(default_indent) {
        default_indent
    } else {
        FixedPoint::default()
    };
    let first_indent = direct
        .first_indent
        .filter(|indent| indent_fits(*indent))
        .unwrap_or(role_indent);

    let default_alignment = ParagraphAlignment::from_fdx(native.alignment)
        .unwrap_or(ParagraphAlignment::Left);
    let alignment = direct
        .alignment
        .or(projected.alignment)
        .unwrap_or(default_alignment);
    let default_space = FixedPoint::from_points(i64::from(native.space_before));
    let space_before = direct
        .space_before
        .or(projected.space_before)
        .unwrap_or(default_space);
    let scaled_metric = |base: u32| {
        let numerator = i64::from(base)
            .saturating_mul(i64::from(text_style.size))
            .saturating_mul(FixedPoint::SCALE);
        FixedPoint::from_raw(numerator.saturating_add(6).div_euclid(12).max(1))
    };
    EffectiveParagraph {
        left,
        right,
        first_indent,
        alignment,
        space_before,
        line_advance: scaled_metric(inputs.line_advance),
        char_width: scaled_metric(inputs.char_width),
    }
}

fn plain_title_style(inputs: &LayoutInputs) -> EffectiveTextStyle {
    EffectiveTextStyle {
        declared_family: inputs.font_family.clone(),
        family: FontFallback::CourierPrime,
        size: inputs.font_size.clamp(1, 72) as u8,
        bold: false,
        italic: false,
        underline: false,
        all_caps: false,
        hidden_text: false,
        source_color: None,
    }
}

fn compat_rect(rect: FixedRect) -> Rect {
    let rounded = |value: FixedPoint| value.rounded_points().clamp(0, i64::from(u32::MAX)) as u32;
    Rect {
        x: rounded(rect.x),
        y: rounded(rect.y),
        width: rounded(rect.width),
        height: rounded(rect.height),
    }
}

fn keep_with_next(role: Role) -> bool {
    matches!(
        role,
        Role::SceneHeading | Role::Character | Role::Parenthetical
    )
}

fn is_boundary_in_line(line: &LayoutLine, offset: usize) -> bool {
    if offset == line.source.start || offset == line.source.end {
        return true;
    }
    line.text.is_char_boundary(offset - line.source.start)
}

#[allow(clippy::too_many_arguments)]
fn wrap_for_layout(
    cache: Option<&mut ElementWrapCache>,
    mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
    element_id: &crate::StableId,
    text: &str,
    element: usize,
    role: Role,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    runs: &[TextRun],
    char_width: FixedPoint,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
) -> Vec<WrappedLine> {
    let collect_diagnostics = diagnostics.is_some();
    let projection_started = diagnostics.as_ref().map(|_| Instant::now());
    let mut projected_scalars = 0u64;
    let mut run_range_searches = 0u64;
    let mut style_attribute_parses = 0u64;
    let mut projected_text = String::new();
    let mut source_map = Vec::<(std::ops::Range<usize>, std::ops::Range<usize>)>::new();
    let mut prepared_runs = Vec::with_capacity(runs.len());
    let mut source_cursor = 0usize;
    let base_size = i64::from(text_style.size.max(1));
    let mut maximum_run_size = base_size;
    for run in runs {
        let end = source_cursor.saturating_add(run.text.len());
        let style = parse_style_attribute(&run.attributes)
            .ok()
            .map(|projection| projection.flags)
            .unwrap_or_default();
        if collect_diagnostics {
            style_attribute_parses = style_attribute_parses.saturating_add(1);
        }
        for attribute in &run.attributes {
            if attribute.name == "Size" {
                if let Ok(size) = attribute.value.parse::<u8>() {
                    if (1..=72).contains(&size) {
                        maximum_run_size = maximum_run_size.max(i64::from(size));
                    }
                }
            }
        }
        prepared_runs.push((source_cursor..end, style));
        source_cursor = end;
    }
    // Wrapping uses a conservative largest direct run metric; span placement
    // below restores each run's own fixed-cell width without host font advances.
    let wrapping_char_width = FixedPoint::from_raw(
        char_width.raw().saturating_mul(maximum_run_size).div_euclid(base_size).max(1),
    );
    let mut run_cursor = 0usize;
    for (source_start, scalar) in text.char_indices() {
        if collect_diagnostics {
            projected_scalars = projected_scalars.saturating_add(1);
        }
        let source_end = source_start + scalar.len_utf8();
        while run_cursor < prepared_runs.len()
            && source_start >= prepared_runs[run_cursor].0.end
        {
            if collect_diagnostics {
                run_range_searches = run_range_searches.saturating_add(1);
            }
            run_cursor += 1;
        }
        let flags = prepared_runs
            .get(run_cursor)
            .filter(|(range, _)| source_start >= range.start && source_start < range.end)
            .map(|(_, flags)| *flags)
            .unwrap_or_else(RunStyleFlags::default);
        let displayed = if text_style.all_caps || flags.all_caps {
            scalar.to_uppercase().collect::<String>()
        } else {
            scalar.to_string()
        };
        let start = projected_text.len();
        projected_text.push_str(&displayed);
        source_map.push((start..projected_text.len(), source_start..source_end));
    }
    if let (Some(diagnostics), Some(started)) = (diagnostics.as_deref_mut(), projection_started) {
        diagnostics.display_projection_nanos = diagnostics
            .display_projection_nanos
            .saturating_add(started.elapsed().as_nanos());
        diagnostics.projected_scalars =
            diagnostics.projected_scalars.saturating_add(projected_scalars);
        diagnostics.run_range_searches =
            diagnostics.run_range_searches.saturating_add(run_range_searches);
        diagnostics.style_attribute_parses = diagnostics
            .style_attribute_parses
            .saturating_add(style_attribute_parses);
    }
    let use_wrap_cache = cache.is_some();
    let mut lines = if let Some(cache) = cache {
        cache.wrap(
            element_id,
            &projected_text,
            element,
            role,
            paragraph,
            {
                let mut wrapping_style = text_style.clone();
                wrapping_style.all_caps = false;
                wrapping_style
            },
            wrapping_char_width,
            runs,
            &source_map,
            purpose,
            title_page,
            geometry_mode,
            diagnostics.as_deref_mut(),
        )
    } else {
        let started = diagnostics.as_ref().map(|_| Instant::now());
        let mut wrapping_style = text_style.clone();
        wrapping_style.all_caps = false;
        let lines = wrap_element(
            &projected_text, element, role, paragraph, wrapping_style, wrapping_char_width,
            &source_map,
        );
        if let (Some(diagnostics), Some(started)) = (diagnostics.as_deref_mut(), started) {
            diagnostics.record_element_wrap(started);
        }
        lines
    };
    if !use_wrap_cache {
        let _ = share_owner_runs(&mut lines, runs, diagnostics.as_deref_mut());
    }
    let remap_started = diagnostics.as_ref().map(|_| Instant::now());
    let mut source_map_segments_examined = 0u64;
    let mut source_map_cursor = DisplaySourceCursor::default();
    for line in &mut lines {
        let projected_start = line.start;
        let projected_end = line.end;
        let mapped_start = source_map_cursor
            .map_start(
                &source_map,
                projected_start,
                &mut source_map_segments_examined,
                collect_diagnostics,
            )
            .unwrap_or(text.len());
        let mut mapped_line_end = None;
        for segment in Arc::make_mut(&mut line.display_source_map) {
            let projected_segment_start = segment.source_start;
            let projected_segment_end = segment.source_end;
            let start = source_map_cursor
                .map_start(
                    &source_map,
                    projected_segment_start,
                    &mut source_map_segments_examined,
                    collect_diagnostics,
                )
                .unwrap_or(mapped_start);
            let end = source_map_cursor
                .map_end(
                    &source_map,
                    projected_segment_end,
                    &mut source_map_segments_examined,
                    collect_diagnostics,
                )
                .unwrap_or(start);
            segment.source_start = start;
            segment.source_end = end;
            mapped_line_end = Some(end);
        }
        let mapped_end = mapped_line_end.unwrap_or_else(|| {
            source_map_cursor
                .map_end(
                    &source_map,
                    projected_end,
                    &mut source_map_segments_examined,
                    collect_diagnostics,
                )
                .unwrap_or(mapped_start)
        });
        line.start = mapped_start;
        line.end = mapped_end;
        line.text = Arc::from(&text[mapped_start..mapped_end]);
        line.text_style = text_style.clone();
        line.char_width = char_width;
    }
    if let (Some(diagnostics), Some(started)) = (diagnostics.as_deref_mut(), remap_started) {
        diagnostics.source_map_remap_nanos = diagnostics
            .source_map_remap_nanos
            .saturating_add(started.elapsed().as_nanos());
        diagnostics.source_map_segments_examined = diagnostics
            .source_map_segments_examined
            .saturating_add(source_map_segments_examined);
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn owner_layout_for_layout(
    wrap_cache: &mut Option<&mut ElementWrapCache>,
    prepared_cache: &mut Option<&mut PreparedOwnerCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    composition: LayoutComposition,
    element_id: &StableId,
    text: &str,
    element: usize,
    role: Role,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    runs: &[TextRun],
    char_width: FixedPoint,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
) -> OwnerLayout {
    let Some(prepared_cache) = prepared_cache.as_deref_mut() else {
        return OwnerLayout::Wrapped {
            lines: wrap_for_layout(
                wrap_cache.as_deref_mut(),
                diagnostics.as_deref_mut(),
                element_id,
                text,
                element,
                role,
                paragraph,
                text_style,
                runs,
                char_width,
                purpose,
                title_page,
                geometry_mode,
            )
            .into(),
        };
    };

    let context = ElementWrapContext {
        element_id: element_id.clone(),
        role,
        purpose,
        title_page,
        geometry_mode,
        paragraph,
        text_style: text_style.clone(),
        char_width,
        // Canonical owner content, runs, and effective style are already in
        // the prepared identity. The wrap-only projected map is not required
        // to perform an allocation-free cache lookup.
        atomic_signature: Vec::new(),
    };
    let (layout, reused, evictions) = prepared_cache.get_or_build(
        context,
        composition,
        text,
        runs,
        |identity| {
            let wrapped = wrap_for_layout(
                wrap_cache.as_deref_mut(),
                diagnostics.as_deref_mut(),
                element_id,
                text,
                element,
                role,
                paragraph,
                text_style,
                runs,
                char_width,
                purpose,
                title_page,
                geometry_mode,
            );
            PreparedOwnerLayout::from_wrapped_lines(
                identity,
                &wrapped,
                diagnostics.as_deref_mut(),
            )
        },
    );
    if let Some(diagnostics) = diagnostics.as_deref_mut() {
        if reused {
            diagnostics.prepared_owner_reuses =
                diagnostics.prepared_owner_reuses.saturating_add(1);
        } else {
            diagnostics.prepared_owner_builds =
                diagnostics.prepared_owner_builds.saturating_add(1);
        }
        diagnostics.prepared_owner_evictions = diagnostics
            .prepared_owner_evictions
            .saturating_add(evictions);
    }
    OwnerLayout::Prepared { element, layout }
}

#[derive(Default)]
struct DisplaySourceCursor {
    // `next` advances monotonically; `previous` preserves the left-side owner
    // for end boundaries that land exactly where the next mapping begins.
    next: usize,
    previous: Option<usize>,
}

impl DisplaySourceCursor {
    fn examined(examined: &mut u64, collect_diagnostics: bool) {
        if collect_diagnostics {
            *examined = (*examined).saturating_add(1);
        }
    }

    fn map_start(
        &mut self,
        source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
        offset: usize,
        examined: &mut u64,
        collect_diagnostics: bool,
    ) -> Option<usize> {
        while let Some((display, source)) = source_map.get(self.next) {
            Self::examined(examined, collect_diagnostics);
            if offset < display.start {
                return None;
            }
            if offset < display.end {
                return Some(source.start);
            }
            self.previous = Some(self.next);
            self.next = self.next.saturating_add(1);
        }
        None
    }

    fn map_end(
        &mut self,
        source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
        offset: usize,
        examined: &mut u64,
        collect_diagnostics: bool,
    ) -> Option<usize> {
        while let Some((display, _)) = source_map.get(self.next) {
            Self::examined(examined, collect_diagnostics);
            if offset <= display.end {
                break;
            }
            self.previous = Some(self.next);
            self.next = self.next.saturating_add(1);
        }
        if let Some(previous) = self.previous {
            let (display, source) = &source_map[previous];
            Self::examined(examined, collect_diagnostics);
            if display.start < offset && offset <= display.end {
                return Some(source.end);
            }
        }
        source_map.get(self.next).and_then(|(display, source)| {
            Self::examined(examined, collect_diagnostics);
            (display.start < offset && offset <= display.end).then_some(source.end)
        })
    }
}

fn wrap_element(
    text: &str,
    element: usize,
    role: Role,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
) -> Vec<WrappedLine> {
    let cell = char_width.raw().max(1);
    let first_left = paragraph
        .left
        .raw()
        .saturating_add(paragraph.first_indent.raw());
    let first_limit = paragraph
        .right
        .raw()
        .saturating_sub(first_left)
        .div_euclid(cell)
        .max(1) as usize;
    let rest_limit = paragraph
        .right
        .raw()
        .saturating_sub(paragraph.left.raw())
        .div_euclid(cell)
        .max(1) as usize;
    let mut output = Vec::new();
    let mut first_visual_line = true;
    let mut paragraph_start = 0;
    let mut source_map_cursor = 0usize;
    let mut delimiters = text.char_indices().peekable();
    while let Some((newline, delimiter)) = delimiters.next() {
        if delimiter != '\n' && delimiter != '\r' {
            continue;
        }
        wrap_segment(
            &text[paragraph_start..newline],
            paragraph_start,
            element,
            role,
            first_limit,
            rest_limit,
            paragraph,
            &text_style,
            char_width,
            source_map,
            &mut source_map_cursor,
            &mut first_visual_line,
            &mut output,
        );
        paragraph_start = newline + delimiter.len_utf8();
        if delimiter == '\r' && delimiters.peek().is_some_and(|(_, next)| *next == '\n') {
            let _ = delimiters.next();
            paragraph_start += 1;
        }
    }
    wrap_segment(
        text[paragraph_start..].as_ref(),
        paragraph_start,
        element,
        role,
        first_limit,
        rest_limit,
        paragraph,
            &text_style,
            char_width,
            source_map,
            &mut source_map_cursor,
            &mut first_visual_line,
        &mut output,
    );
    if output.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: 0,
            end: 0,
        text: Arc::from(""),
        display_text: Arc::from(""),
        display_source_map: Arc::from([]),
        text_style,
        runs: Arc::from([]),
        char_width,
            first_visual_line: true,
            final_visual_line: true,
            paragraph,
            oversized: false,
        });
    }
    output
}

#[derive(Debug, Clone, Copy)]
struct DisplayCell {
    character: char,
    local_start: usize,
    local_end: usize,
    source_start: usize,
    source_end: usize,
}

// Segment wrapping keeps source ranges and layout constraints explicit at the
// call boundary; consolidating them into state would obscure their invariants.
#[allow(clippy::too_many_arguments)]
fn wrap_segment(
    text: &str,
    base: usize,
    element: usize,
    role: Role,
    first_limit: usize,
    rest_limit: usize,
    paragraph: EffectiveParagraph,
    text_style: &EffectiveTextStyle,
    char_width: FixedPoint,
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
    source_map_cursor: &mut usize,
    first_visual_line: &mut bool,
    output: &mut Vec<WrappedLine>,
) {
    if text.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: base,
            end: base,
            text: Arc::from(""),
            display_text: Arc::from(""),
            display_source_map: Arc::from([]),
            text_style: text_style.clone(),
            runs: Arc::from([]),
            char_width,
            first_visual_line: *first_visual_line,
            final_visual_line: true,
            paragraph,
            oversized: false,
        });
        *first_visual_line = false;
        return;
    }
    let mut cells = Vec::new();
    for (local_start, source_character) in text.char_indices() {
        let local_end = local_start + source_character.len_utf8();
        let source_start = base + local_start;
        let source_end = base + local_end;
        if text_style.all_caps {
            cells.extend(
                source_character
                    .to_uppercase()
                    .map(|character| DisplayCell {
                        character,
                        local_start,
                        local_end,
                        source_start,
                        source_end,
                    }),
            );
        } else {
            cells.push(DisplayCell {
                character: source_character,
                local_start,
                local_end,
                source_start,
                source_end,
            });
        }
    }
    let mut start = 0;
    while start < cells.len() && output.len() < MAX_LINES_PER_ELEMENT {
        let first_line = *first_visual_line;
        let limit = if first_line { first_limit } else { rest_limit };
        let mut hard_end = (start + limit).min(cells.len());
        while hard_end < cells.len()
            && hard_end > start
            && cells[hard_end - 1].source_start == cells[hard_end].source_start
        {
            hard_end += 1;
        }
        while hard_end < cells.len() && hard_end > start {
            let boundary = base.saturating_add(cells[hard_end].local_start);
            while source_map
                .get(*source_map_cursor)
                .is_some_and(|(display, _)| display.end <= boundary)
            {
                *source_map_cursor = (*source_map_cursor).saturating_add(1);
            }
            if source_map.get(*source_map_cursor).is_some_and(|(display, _)| {
                boundary > display.start && boundary < display.end
            }) {
                hard_end += 1;
            } else {
                break;
            }
        }
        let mut end = hard_end;
        if hard_end < cells.len() {
            if let Some(space) = (start..hard_end)
                .rev()
                .find(|index| cells[*index].character.is_whitespace())
            {
                end = space + 1;
            }
        }
        if end <= start {
            end = (start + 1).min(cells.len());
        }
        let selected = &cells[start..end];
        let first_cell = selected.first().copied().unwrap_or(DisplayCell {
            character: '\0',
            local_start: 0,
            local_end: 0,
            source_start: base,
            source_end: base,
        });
        let last_cell = selected.last().copied().unwrap_or(first_cell);
        let source_start = first_cell.source_start;
        let source_end = last_cell.source_end;
        let mut display_text = String::new();
        let mut display_source_map: Vec<DisplaySourceSegment> = Vec::new();
        for (display_index, cell) in selected.iter().enumerate() {
            display_text.push(cell.character);
            if let Some(previous) = display_source_map.last_mut() {
                if previous.source_start == cell.source_start
                    && previous.source_end == cell.source_end
                {
                    previous.display_end = display_index + 1;
                    continue;
                }
            }
            display_source_map.push(DisplaySourceSegment {
                display_start: display_index,
                display_end: display_index + 1,
                source_start: cell.source_start,
                source_end: cell.source_end,
            });
        }
        output.push(WrappedLine {
            element,
            role,
            start: source_start,
            end: source_end,
            text: Arc::from(&text[first_cell.local_start..last_cell.local_end]),
            display_text: Arc::from(display_text),
            display_source_map: display_source_map.into(),
            text_style: text_style.clone(),
            runs: Arc::from([]),
            char_width,
            first_visual_line: first_line,
            final_visual_line: end == cells.len(),
            paragraph,
            oversized: selected.len() > limit,
        });
        *first_visual_line = false;
        start = end;
    }
}

#[cfg(test)]
mod session_layout_cache_tests {
    use super::{
        BodyEditorPresentation, LayoutBuildDiagnostics, LayoutCache, LayoutComposition,
        LayoutPurpose,
    };
    use crate::{editing::EditingSession, Project, Role, StableId, TextRun};
    use std::sync::Arc;

    #[test]
    fn core_layout_phase_diagnostics_have_numeric_zero_defaults() {
        let diagnostics = LayoutBuildDiagnostics::default();
        assert!([
            diagnostics.complete_project_validations,
            diagnostics.layout_builds,
            diagnostics.semantic_signature_constructions,
            diagnostics.element_wraps_built,
            diagnostics.element_wrap_misses,
            diagnostics.element_wraps_reused,
            diagnostics.prepared_owner_builds,
            diagnostics.prepared_owner_reuses,
            diagnostics.prepared_owner_evictions,
            diagnostics.projected_scalars,
            diagnostics.run_range_searches,
            diagnostics.style_attribute_parses,
            diagnostics.source_map_segments_examined,
            diagnostics.post_cache_run_constructions,
            diagnostics.post_cache_run_bytes_cloned,
            diagnostics.positioned_span_characters,
            diagnostics.interaction_index_entries,
            diagnostics.interaction_index_entries_sorted,
            diagnostics.wrap_cache_evictions,
            diagnostics.wrap_cache_eviction_entries_examined,
        ].iter().all(|value| *value == 0));
        assert!([
            diagnostics.element_wrap_nanos,
            diagnostics.display_projection_nanos,
            diagnostics.source_map_remap_nanos,
            diagnostics.post_cache_run_nanos,
            diagnostics.positioned_span_nanos,
            diagnostics.interaction_index_build_nanos,
        ].iter().all(|value| *value == 0));
    }

    #[test]
    fn validated_session_wrappers_preserve_surface_keys_and_share_hits() {
        let project = Project::new().expect("default project is valid");
        let session = EditingSession::new(project).expect("valid project opens in a session");

        let presentation = BodyEditorPresentation::for_project(session.project(), None);
        let mut body_cache = LayoutCache::default();
        let mut diagnostics = LayoutBuildDiagnostics::default();
        let (body_layout, body_hit) = body_cache
            .get_or_build_body_editor_for_session_with_diagnostics(
                &session,
                &presentation,
                &mut diagnostics,
            )
            .expect("validated Body layout builds");
        assert!(!body_hit);
        assert_eq!(diagnostics.complete_project_validations, 0);
        assert_eq!(diagnostics.layout_builds, 1);
        assert_eq!(body_cache.composition, Some(LayoutComposition::BodyEditor));
        assert_eq!(body_cache.body_presentation.as_ref(), Some(&presentation));
        assert_eq!(body_cache.project_layout_signature, None);

        let (cached_body_layout, body_hit) = body_cache
            .get_or_build_body_editor_for_session_with_diagnostics(
                &session,
                &presentation,
                &mut diagnostics,
            )
            .expect("validated Body layout hits");
        assert!(body_hit);
        assert!(Arc::ptr_eq(&body_layout, &cached_body_layout));
        assert_eq!(diagnostics.complete_project_validations, 0);
        assert_eq!(diagnostics.layout_builds, 0);
        assert_eq!(body_cache.project_layout_signature, None);

        let mut title_cache = LayoutCache::default();
        let (title_layout, title_hit) = title_cache
            .get_or_build_title_page_editor_for_session_with_diagnostics(
                &session,
                &mut diagnostics,
            )
            .expect("validated Title layout builds");
        assert!(!title_hit);
        assert_eq!(diagnostics.complete_project_validations, 0);
        assert_eq!(diagnostics.layout_builds, 1);
        assert_eq!(
            title_cache.composition,
            Some(LayoutComposition::TitlePageEditor)
        );
        assert_eq!(title_cache.body_presentation.as_ref(), Some(&BodyEditorPresentation::default()));
        assert_eq!(title_cache.project_layout_signature, None);

        let (cached_title_layout, title_hit) = title_cache
            .get_or_build_title_page_editor_for_session_with_diagnostics(
                &session,
                &mut diagnostics,
            )
            .expect("validated Title layout hits");
        assert!(title_hit);
        assert!(Arc::ptr_eq(&title_layout, &cached_title_layout));
        assert_eq!(diagnostics.complete_project_validations, 0);
        assert_eq!(diagnostics.layout_builds, 0);
        assert_eq!(title_cache.project_layout_signature, None);
    }

    #[test]
    fn generic_cache_retains_same_revision_project_mutation_defense() {
        let mut project = Project::new().expect("default project is valid");
        let inputs = project.session_layout_inputs();
        let mut cache = LayoutCache::default();
        let (first_layout, first_hit) = cache
            .get_or_build_with_inputs_for_purpose(0, &project, &inputs, LayoutPurpose::Editor)
            .expect("generic layout builds");
        assert!(!first_hit);

        project.elements[0].text.push_str("changed without a revision");
        let (mutated_layout, mutated_hit) = cache
            .get_or_build_with_inputs_for_purpose(0, &project, &inputs, LayoutPurpose::Editor)
            .expect("generic cache rebuilds after same-revision mutation");
        assert!(!mutated_hit);
        assert!(!Arc::ptr_eq(&first_layout, &mutated_layout));
        assert!(cache.project_layout_signature.is_some());
    }

    #[test]
    fn core_phase_diagnostics_distinguish_cold_and_prepared_owner_reuse() {
        let mut project = Project::new().expect("default project is valid");
        let element = project.elements.first_mut().expect("default project has a Body owner");
        let text = "core phase diagnostics".to_owned();
        element.text = text.clone();
        element.runs = vec![crate::TextRun { text, attributes: Vec::new() }];
        let inputs = project.session_layout_inputs();
        let mut cache = LayoutCache::default();
        let mut diagnostics = LayoutBuildDiagnostics::default();

        let (_, cold_hit) = cache
            .get_or_build_with_inputs_for_purpose_and_diagnostics(
                1,
                &project,
                &inputs,
                LayoutPurpose::Editor,
                &mut diagnostics,
            )
            .expect("cold layout builds");
        assert!(!cold_hit);
        assert!(diagnostics.projected_scalars >= 1);
        assert!(diagnostics.source_map_segments_examined >= 1);
        assert!(diagnostics.post_cache_run_constructions >= 1);
        assert!(diagnostics.prepared_owner_builds >= 1);
        assert!(diagnostics.positioned_span_characters >= 1);
        assert!(diagnostics.interaction_index_entries >= 1);

        let (_, warm_hit) = cache
            .get_or_build_with_inputs_for_purpose_and_diagnostics(
                2,
                &project,
                &inputs,
                LayoutPurpose::Editor,
                &mut diagnostics,
            )
            .expect("warm wrap cache builds a fresh revision layout");
        assert!(!warm_hit);
        assert!(diagnostics.prepared_owner_reuses >= 1);
        assert_eq!(diagnostics.prepared_owner_builds, 0);
        assert_eq!(diagnostics.projected_scalars, 0);
        assert_eq!(diagnostics.source_map_segments_examined, 0);
        assert_eq!(diagnostics.post_cache_run_constructions, 0);
        assert_eq!(diagnostics.positioned_span_characters, 0);
        assert!(diagnostics.interaction_index_entries_sorted >= 1);
    }

    #[test]
    fn edited_owner_misses_unchanged_owners_reuse_and_indexes_rebuild() {
        let mut project = Project::new().expect("default project is valid");
        let template = project.elements[0].clone();
        for (index, id) in [
            "prepared-cache-owner-a",
            "prepared-cache-owner-b",
            "prepared-cache-owner-c",
        ]
        .into_iter()
        .enumerate()
        {
            let mut element = template.clone();
            element.id = StableId::new(id).expect("static owner ID is valid");
            element.role = Role::Action;
            let text = format!("unchanged prepared owner {index}");
            element.text = text.clone();
            element.runs = vec![TextRun {
                text,
                attributes: Vec::new(),
            }];
            project.elements.push(element);
        }

        let inputs = project.session_layout_inputs();
        let mut cache = LayoutCache::default();
        let (_, first_hit) = cache
            .get_or_build_with_inputs_for_purpose(1, &project, &inputs, LayoutPurpose::Editor)
            .expect("initial complete layout builds");
        assert!(!first_hit);

        let changed_text = "edited prepared owner".to_owned();
        project.elements[0].text = changed_text.clone();
        project.elements[0].runs = vec![TextRun {
            text: changed_text,
            attributes: Vec::new(),
        }];
        let mut diagnostics = LayoutBuildDiagnostics::default();
        let (layout, revision_hit) = cache
            .get_or_build_with_inputs_for_purpose_and_diagnostics(
                2,
                &project,
                &inputs,
                LayoutPurpose::Editor,
                &mut diagnostics,
            )
            .expect("changed revision performs complete layout composition");

        assert!(!revision_hit);
        assert_eq!(diagnostics.prepared_owner_builds, 1);
        assert!(diagnostics.prepared_owner_reuses >= 3);
        assert_eq!(diagnostics.layout_builds, 1);
        assert!(diagnostics.interaction_index_entries > 0);
        assert!(diagnostics.interaction_index_entries_sorted > 0);
        assert_eq!(
            layout.pages.iter().map(|page| page.lines.len()).sum::<usize>(),
            layout.body_line_count()
        );
    }
}

#[cfg(test)]
mod wrap_run_preparation_tests {
    use super::{
        positioned_spans, wrap_for_layout, EffectiveGeometryMode, EffectiveParagraph,
        ElementWrapCache, ElementWrapContext, FixedRect, LayoutBuildDiagnostics,
        LayoutComposition, LayoutPurpose, PreparedOwnerIdentity, PreparedOwnerLayout,
    };
    use crate::formatting::{EffectiveTextStyle, FixedPoint, FontFallback, ParagraphAlignment};
    use crate::{Role, RunAttribute, StableId, TextRun};

    fn layout_style() -> EffectiveTextStyle {
        EffectiveTextStyle {
            declared_family: "Courier Prime".to_owned(),
            family: FontFallback::CourierPrime,
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            source_color: None,
        }
    }

    fn layout_paragraph() -> EffectiveParagraph {
        EffectiveParagraph {
            left: FixedPoint::default(),
            right: FixedPoint::from_points(1_000),
            first_indent: FixedPoint::default(),
            alignment: ParagraphAlignment::Left,
            space_before: FixedPoint::default(),
            line_advance: FixedPoint::from_points(12),
            char_width: FixedPoint::from_points(7),
        }
    }

    fn run(text: &str, attributes: &[(&str, &str)]) -> TextRun {
        TextRun {
            text: text.to_owned(),
            attributes: attributes
                .iter()
                .map(|(name, value)| RunAttribute {
                    name: (*name).to_owned(),
                    value: (*value).to_owned(),
                })
                .collect(),
        }
    }

    fn prepare(
        cache: Option<&mut ElementWrapCache>,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
        text: &str,
        runs: &[TextRun],
    ) -> Vec<super::WrappedLine> {
        prepare_with_paragraph(cache, diagnostics, text, runs, layout_paragraph())
    }

    fn prepare_with_paragraph(
        cache: Option<&mut ElementWrapCache>,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
        text: &str,
        runs: &[TextRun],
        paragraph: super::EffectiveParagraph,
    ) -> Vec<super::WrappedLine> {
        let element_id = StableId::new("wrap-preparation-test").expect("static ID is valid");
        wrap_for_layout(
            cache,
            diagnostics,
            &element_id,
            text,
            0,
            Role::Action,
            paragraph,
            layout_style(),
            runs,
            FixedPoint::from_points(7),
            LayoutPurpose::Editor,
            false,
            EffectiveGeometryMode::Sequential,
        )
    }

    fn assert_lines_equal(left: &[super::WrappedLine], right: &[super::WrappedLine]) {
        assert_eq!(left.len(), right.len());
        for (left, right) in left.iter().zip(right) {
            assert_eq!(left.element, right.element);
            assert_eq!(left.role, right.role);
            assert_eq!(left.start, right.start);
            assert_eq!(left.end, right.end);
            assert_eq!(left.text, right.text);
            assert_eq!(left.display_text, right.display_text);
            assert_eq!(left.display_source_map, right.display_source_map);
            assert_eq!(left.text_style, right.text_style);
            assert_eq!(left.runs, right.runs);
            assert_eq!(left.char_width, right.char_width);
            assert_eq!(left.first_visual_line, right.first_visual_line);
            assert_eq!(left.final_visual_line, right.final_visual_line);
            assert_eq!(left.paragraph, right.paragraph);
            assert_eq!(left.oversized, right.oversized);
        }
    }

    #[test]
    fn ordered_projection_preserves_empty_runs_style_boundaries_unicode_and_cache_identity() {
        let text = "aß\nc";
        let runs = vec![
            run("", &[("Style", "Bold")]),
            run("a", &[("Style", "AllCaps")]),
            run("ß\n", &[("Style", "AllCaps")]),
            run("c", &[("Size", "18")]),
        ];
        let mut cache = ElementWrapCache::default();
        let mut diagnostics = LayoutBuildDiagnostics::default();

        let fresh = prepare(None, Some(&mut diagnostics), text, &runs);
        assert_eq!(fresh.len(), 2);
        assert_eq!(fresh[0].start, 0);
        assert_eq!(fresh[0].end, 3);
        assert_eq!(fresh[0].text.as_ref(), "aß");
        assert_eq!(fresh[0].display_text.as_ref(), "ASS");
        assert_eq!(fresh[0].display_source_map.len(), 2);
        assert_eq!(fresh[0].display_source_map[0].display_start, 0);
        assert_eq!(fresh[0].display_source_map[0].display_end, 1);
        assert_eq!(fresh[0].display_source_map[0].source_start, 0);
        assert_eq!(fresh[0].display_source_map[0].source_end, 1);
        assert_eq!(fresh[0].display_source_map[1].display_start, 1);
        assert_eq!(fresh[0].display_source_map[1].display_end, 3);
        assert_eq!(fresh[0].display_source_map[1].source_start, 1);
        assert_eq!(fresh[0].display_source_map[1].source_end, 3);
        assert!(fresh[0].first_visual_line);
        assert!(!fresh[0].final_visual_line);
        assert_eq!(fresh[1].start, 4);
        assert_eq!(fresh[1].end, 5);
        assert_eq!(fresh[1].text.as_ref(), "c");
        assert_eq!(fresh[1].display_text.as_ref(), "c");
        assert!(!fresh[1].first_visual_line);
        assert!(fresh[1].final_visual_line);
        assert_eq!(diagnostics.projected_scalars, 4);
        assert_eq!(diagnostics.run_range_searches, 3);
        assert_eq!(diagnostics.style_attribute_parses, runs.len() as u64);

        let cache_miss = prepare(Some(&mut cache), None, text, &runs);
        let cache_hit = prepare(Some(&mut cache), None, text, &runs);
        assert_lines_equal(&cache_miss, &fresh);
        assert_lines_equal(&cache_hit, &fresh);

        let changed_runs = vec![
            runs[0].clone(),
            runs[1].clone(),
            runs[2].clone(),
            run("c", &[("Style", "AllCaps"), ("Size", "18")]),
        ];
        let mut changed_diagnostics = LayoutBuildDiagnostics::default();
        let changed = prepare(Some(&mut cache), Some(&mut changed_diagnostics), text, &changed_runs);
        assert_eq!(changed[1].display_text.as_ref(), "C");
        assert_eq!(changed_diagnostics.element_wraps_reused, 0);
        assert_eq!(changed_diagnostics.element_wrap_misses, 1);
    }

    #[test]
    fn plain_single_run_and_empty_text_keep_canonical_boundaries() {
        let single = run("plain", &[]);
        let lines = prepare(None, None, "plain", &[single]);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text.as_ref(), "plain");
        assert_eq!(lines[0].display_text.as_ref(), "plain");
        assert_eq!(lines[0].start, 0);
        assert_eq!(lines[0].end, 5);
        assert!(lines[0].first_visual_line);
        assert!(lines[0].final_visual_line);

        let mut empty_diagnostics = LayoutBuildDiagnostics::default();
        let empty = prepare(
            None,
            Some(&mut empty_diagnostics),
            "",
            &[run("", &[("Style", "AllCaps")])],
        );
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].start, 0);
        assert_eq!(empty[0].end, 0);
        assert_eq!(empty[0].text.as_ref(), "");
        assert_eq!(empty[0].display_text.as_ref(), "");
        assert!(empty[0].display_source_map.is_empty());
        assert_eq!(empty_diagnostics.projected_scalars, 0);
        assert_eq!(empty_diagnostics.run_range_searches, 0);
        assert_eq!(empty_diagnostics.style_attribute_parses, 1);
    }

    #[test]
    fn cached_multiline_remapping_is_linear_and_lines_share_owner_runs() {
        let long_text = "abcdefgh".repeat(32);
        let long_runs = [run(&long_text, &[])];
        let narrow_paragraph = EffectiveParagraph {
            right: FixedPoint::from_points(28),
            ..layout_paragraph()
        };
        let mut cache = ElementWrapCache::default();
        let mut miss_diagnostics = LayoutBuildDiagnostics::default();
        let fresh = prepare_with_paragraph(
            None,
            None,
            &long_text,
            &long_runs,
            narrow_paragraph,
        );
        let cache_miss = prepare_with_paragraph(
            Some(&mut cache),
            Some(&mut miss_diagnostics),
            &long_text,
            &long_runs,
            narrow_paragraph,
        );
        let mut hit_diagnostics = LayoutBuildDiagnostics::default();
        let cache_hit = prepare_with_paragraph(
            Some(&mut cache),
            Some(&mut hit_diagnostics),
            &long_text,
            &long_runs,
            narrow_paragraph,
        );

        assert!(fresh.len() > 1);
        assert_lines_equal(&cache_miss, &fresh);
        assert_lines_equal(&cache_hit, &fresh);
        assert!(cache_hit
            .windows(2)
            .all(|lines| std::sync::Arc::ptr_eq(&lines[0].runs, &lines[1].runs)));
        assert_eq!(miss_diagnostics.post_cache_run_constructions, 1);
        assert!(miss_diagnostics.post_cache_run_bytes_cloned > 0);
        assert_eq!(hit_diagnostics.post_cache_run_constructions, 0);
        assert_eq!(hit_diagnostics.post_cache_run_bytes_cloned, 0);
        assert_eq!(hit_diagnostics.element_wraps_reused, 1);
        assert!(hit_diagnostics.source_map_segments_examined > 0);
        assert!(hit_diagnostics.source_map_segments_examined <= hit_diagnostics
            .projected_scalars
            .saturating_mul(16));
    }

    #[test]
    fn cached_remapping_preserves_expansion_run_and_empty_line_boundaries() {
        let text = "abßcd\n\nefghi";
        let runs = vec![
            run("", &[]),
            run("ab", &[]),
            run("ßcd", &[("Style", "AllCaps")]),
            run("\n\nefghi", &[]),
        ];
        let narrow_paragraph = EffectiveParagraph {
            right: FixedPoint::from_points(28),
            ..layout_paragraph()
        };
        let mut cache = ElementWrapCache::default();
        let fresh = prepare_with_paragraph(None, None, text, &runs, narrow_paragraph);
        let cached = prepare_with_paragraph(Some(&mut cache), None, text, &runs, narrow_paragraph);
        let cached_hit =
            prepare_with_paragraph(Some(&mut cache), None, text, &runs, narrow_paragraph);

        assert_lines_equal(&cached, &fresh);
        assert_lines_equal(&cached_hit, &fresh);
        assert_eq!(fresh[0].text.as_ref(), "abß");
        assert_eq!(fresh[0].display_text.as_ref(), "abSS");
        assert_eq!(fresh[0].start, 0);
        assert_eq!(fresh[0].end, 4);
        assert_eq!(fresh[1].text.as_ref(), "cd");
        assert_eq!(fresh[1].display_text.as_ref(), "CD");
        assert_eq!(fresh[1].start, 4);
        assert_eq!(fresh[1].end, 6);
        let empty_line = fresh.iter().find(|line| line.text.is_empty()).unwrap();
        assert_eq!(empty_line.start, 7);
        assert_eq!(empty_line.end, 7);
        assert!(fresh
            .windows(2)
            .all(|lines| std::sync::Arc::ptr_eq(&lines[0].runs, &lines[1].runs)));
        assert!(cached_hit
            .windows(2)
            .all(|lines| std::sync::Arc::ptr_eq(&lines[0].runs, &lines[1].runs)));
    }

    #[test]
    fn positioned_spans_walk_maps_and_runs_once_and_preserve_unicode_style_geometry() {
        let text = "aß b";
        let runs = vec![
            run("aß", &[("Style", "AllCaps"), ("Size", "18")]),
            run(" b", &[("Style", "Bold")]),
        ];
        let fresh = prepare(None, None, text, &runs);
        let mut cache = ElementWrapCache::default();
        let cached = prepare(Some(&mut cache), None, text, &runs);
        assert_lines_equal(&fresh, &cached);

        let mut diagnostics = LayoutBuildDiagnostics::default();
        let geometry = FixedRect {
            x: FixedPoint::default(),
            y: FixedPoint::from_points(24),
            width: FixedPoint::default(),
            height: FixedPoint::from_points(12),
        };
        let fresh_spans = positioned_spans(&fresh[0], geometry, Some(&mut diagnostics));
        let cached_spans = positioned_spans(&cached[0], geometry, None);

        assert_eq!(fresh_spans, cached_spans);
        assert_eq!(fresh_spans.len(), 4);
        assert_eq!(fresh_spans[0].display_text.as_ref(), "AS");
        assert_eq!(fresh_spans[0].source.start, 0);
        assert_eq!(fresh_spans[0].source.end, 3);
        assert_eq!(fresh_spans[0].display_source_map.len(), 2);
        assert_eq!(fresh_spans[0].display_source_map[1].source_start, 1);
        assert_eq!(fresh_spans[0].display_source_map[1].source_end, 3);
        assert_eq!(fresh_spans[1].display_text.as_ref(), "S");
        assert_eq!(fresh_spans[1].source.start, 1);
        assert_eq!(fresh_spans[1].source.end, 3);
        assert_eq!(fresh_spans[2].display_text.as_ref(), " ");
        assert_eq!(fresh_spans[3].display_text.as_ref(), "b");
        assert!(fresh_spans[0].text_style.all_caps);
        assert!(fresh_spans[3].text_style.bold);
        assert_eq!(fresh_spans[0].geometry.width, FixedPoint::from_raw(2 * 1_050));
        assert_eq!(fresh_spans[0].geometry.x, FixedPoint::default());
        assert_eq!(fresh_spans[3].geometry.y, geometry.y);
        assert!(diagnostics.source_map_segments_examined <= 9);
        assert_eq!(diagnostics.run_range_searches, 1);
        assert_eq!(diagnostics.style_attribute_parses, 2);
    }

    #[test]
    fn positioned_span_cursor_work_is_bounded_by_display_and_mapping_entries() {
        let text = "abcdefgh".repeat(32);
        let runs = [run(&text, &[])];
        let lines = prepare(None, None, &text, &runs);
        let display_characters = lines
            .iter()
            .map(|line| line.display_text.chars().count())
            .sum::<usize>();
        let mapping_entries = lines
            .iter()
            .map(|line| line.display_source_map.len())
            .sum::<usize>();
        let mut diagnostics = LayoutBuildDiagnostics::default();
        let geometry = FixedRect {
            x: FixedPoint::default(),
            y: FixedPoint::default(),
            width: FixedPoint::default(),
            height: FixedPoint::from_points(12),
        };

        for line in &lines {
            let _ = positioned_spans(line, geometry, Some(&mut diagnostics));
        }

        assert_eq!(display_characters, text.chars().count());
        assert_eq!(mapping_entries, display_characters);
        assert!(diagnostics.source_map_segments_examined
            <= (display_characters + mapping_entries) as u64);
        assert_eq!(diagnostics.run_range_searches, 0);
        assert_eq!(diagnostics.style_attribute_parses, lines.len() as u64);
    }

    fn prepared_identity(
        lines: &[super::WrappedLine],
        text: &str,
        runs: &[TextRun],
        mode: EffectiveGeometryMode,
        composition: LayoutComposition,
    ) -> PreparedOwnerIdentity {
        let first = lines.first().expect("prepared owner has a wrapped line");
        let shared_runs = first.runs.clone();
        assert_eq!(shared_runs.as_ref(), runs);
        PreparedOwnerIdentity::new(
            ElementWrapContext {
                element_id: StableId::new("wrap-preparation-test").expect("static ID is valid"),
                role: first.role,
                purpose: composition.purpose(),
                title_page: matches!(
                    composition,
                    LayoutComposition::TitlePageEditor
                        | LayoutComposition::Print {
                            include_title_page: true
                        }
                ),
                geometry_mode: mode,
                paragraph: first.paragraph,
                text_style: first.text_style.clone(),
                char_width: first.char_width,
                atomic_signature: lines
                    .iter()
                    .flat_map(|line| {
                        line.display_source_map
                            .iter()
                            .map(|segment| (segment.display_start, segment.display_end))
                    })
                    .collect(),
            },
            composition,
            text,
            shared_runs,
        )
    }

    #[test]
    fn prepared_owner_is_deterministic_immutable_and_paragraph_relative() {
        let text = "abß cd\nef";
        let runs = [run(text, &[("Style", "AllCaps"), ("Size", "18")])];
        let paragraph = EffectiveParagraph {
            left: FixedPoint::from_points(100),
            right: FixedPoint::from_points(128),
            alignment: ParagraphAlignment::Center,
            ..layout_paragraph()
        };
        let mut wrapped = prepare_with_paragraph(None, None, text, &runs, paragraph);
        assert!(wrapped.len() > 1);
        // A line-local overflow marker is retained as prepared owner data;
        // page-capacity overflow classification remains in placement.
        wrapped[0].oversized = true;

        let identity = prepared_identity(
            &wrapped,
            text,
            &runs,
            EffectiveGeometryMode::Sequential,
            LayoutComposition::BodyEditor,
        );
        let first = PreparedOwnerLayout::from_wrapped_lines(identity, &wrapped, None);
        let repeated_identity = prepared_identity(
            &wrapped,
            text,
            &runs,
            EffectiveGeometryMode::Sequential,
            LayoutComposition::BodyEditor,
        );
        let repeated = PreparedOwnerLayout::from_wrapped_lines(repeated_identity, &wrapped, None);

        assert_eq!(first, repeated);
        assert!(first.estimated_bytes > 0);
        assert!(first.within_owner_cache_byte_limit());
        assert_eq!(first.lines.len(), wrapped.len());
        assert_eq!(first.lines[0].source_start, 0);
        assert!(first.lines[0].oversized);
        assert!(first.lines[0].display_text.contains('S'));
        assert!(first.lines[0]
            .spans
            .iter()
            .any(|span| span.text_style.all_caps));
        let expanded_scalar = first
            .lines
            .iter()
            .find(|line| line.text.contains('ß'))
            .expect("the canonical expansion stays in a prepared line");
        assert!(expanded_scalar.display_text.contains("SS"));
        assert!(expanded_scalar.display_source_map.iter().any(|segment| {
            segment.source_end.saturating_sub(segment.source_start) == "ß".len()
                && segment.display_end.saturating_sub(segment.display_start) == 2
        }));
        assert_eq!(first.relative_bounds.x, FixedPoint::default());
        assert_eq!(first.relative_bounds.y, FixedPoint::default());
        assert_eq!(first.lines[0].paragraph.left, FixedPoint::default());
        assert_eq!(first.lines[0].paragraph_bounds.x, FixedPoint::default());
        assert_eq!(first.lines[0].geometry.y, FixedPoint::default());
        assert!(first
            .lines
            .iter()
            .all(|line| line.spans.iter().all(|span| span.geometry.y == line.geometry.y)));
        assert!(first.lines.windows(2).all(|lines| {
            lines[1].geometry.y.raw() >= lines[0].geometry.y.raw()
        }));
        assert!(first
            .lines
            .windows(2)
            .all(|lines| std::sync::Arc::ptr_eq(&lines[0].runs, &lines[1].runs)));
        assert!(std::sync::Arc::ptr_eq(
            &first.identity.runs,
            &first.lines[0].runs
        ));
        let shared = first.clone();
        assert!(std::sync::Arc::ptr_eq(&first.lines, &shared.lines));
    }

    #[test]
    fn prepared_owner_identity_checks_fingerprint_then_full_owner_and_geometry_inputs() {
        let text = "identity ß";
        let runs = [run(text, &[("Style", "AllCaps")])];
        let lines = prepare(None, None, text, &runs);
        let identity = prepared_identity(
            &lines,
            text,
            &runs,
            EffectiveGeometryMode::Sequential,
            LayoutComposition::BodyEditor,
        );
        let same_content = prepared_identity(
            &lines,
            text,
            &runs,
            EffectiveGeometryMode::Sequential,
            LayoutComposition::BodyEditor,
        );
        assert!(identity.matches(&same_content));
        let separately_owned_content = PreparedOwnerIdentity::new(
            identity.context.clone(),
            identity.composition,
            text,
            std::sync::Arc::from(runs.as_slice()),
        );
        assert!(identity.matches(&separately_owned_content));

        let changed_runs = [run(text, &[("Style", "Bold")])];
        let changed_lines = prepare(None, None, text, &changed_runs);
        let changed_content = prepared_identity(
            &changed_lines,
            text,
            &changed_runs,
            EffectiveGeometryMode::Sequential,
            LayoutComposition::BodyEditor,
        );
        assert!(!identity.matches(&changed_content));

        let dual_identity = prepared_identity(
            &lines,
            text,
            &runs,
            EffectiveGeometryMode::DualColumn,
            LayoutComposition::BodyEditor,
        );
        assert!(!identity.matches(&dual_identity));
        let print_identity = prepared_identity(
            &lines,
            text,
            &runs,
            EffectiveGeometryMode::Sequential,
            LayoutComposition::Print {
                include_title_page: false,
            },
        );
        assert!(!identity.matches(&print_identity));

        let mut narrower = layout_paragraph();
        narrower.right = FixedPoint::from_points(70);
        let narrower_lines = prepare_with_paragraph(None, None, text, &runs, narrower);
        let changed_geometry = prepared_identity(
            &narrower_lines,
            text,
            &runs,
            EffectiveGeometryMode::Sequential,
            LayoutComposition::BodyEditor,
        );
        assert!(!identity.matches(&changed_geometry));
    }
}
