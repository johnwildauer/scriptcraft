//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses explicit effective inputs and fixed integer metrics rather than
//! a platform font or viewport. The deterministic defaults therefore have stable structure on every host: one
//! character cell is seven units wide and one line is twelve units high.

use crate::{
    formatted_text::parse_style_attribute,
    formatting::{
        formatting_for_role, EffectiveTextStyle, FixedPoint, FontFallback, ParagraphAlignment,
    },
    validate_layout_inputs, BodyOwner, LayoutInputs, ModelError, Project, Role, RunAttribute,
    SourceLayoutSettings, StableId, TextRun,
};
use scriptcraft_session::editing::EditingSession;
use std::time::Instant;
use std::{
    collections::{BTreeSet, HashMap},
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
const MAX_CACHED_PREPARED_BODY_PARAGRAPHS: usize = 4_096;
const MAX_CACHED_PREPARED_BODY_BYTES: usize = 64 * 1024 * 1024;

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
    prepared_body: PreparedBodyCache,
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
    /// Validated-session lookups leave this at zero on both hits and misses.
    pub semantic_signature_constructions: u64,
    pub element_wraps_built: u64,
    pub element_wrap_misses: u64,
    pub element_wraps_reused: u64,
    pub element_wrap_nanos: u128,
    /// Complete cache-request work through a successful actual build, including
    /// preflight, lookup, placement, indexes, eviction, and publication.
    pub whole_build_nanos: u128,
    /// Effective prepared-environment resolution and cache lookup, excluding
    /// all miss preparation. Hits and misses have separate paragraph counts.
    pub prepared_lookup_nanos: u128,
    pub prepared_hits: u64,
    pub prepared_misses: u64,
    /// Owner-local misses that reused a proven unchanged prepared-line prefix.
    pub prepared_incremental_hits: u64,
    /// Owner-local misses that used complete preparation instead.
    pub prepared_incremental_fallbacks: u64,
    /// Aggregate preparation after prepared-cache misses only.
    pub prepared_miss_nanos: u128,
    /// Ordinary, Title, dual-dialogue, and oversized placement/reflow work.
    /// Prepared line/span rebase is nested and is not additive with this total.
    pub placement_nanos: u128,
    pub rebase_nanos: u128,
    /// Complete interaction-index construction. Body Y-order/sort is nested.
    pub interaction_index_nanos: u128,
    pub body_y_sort_nanos: u128,
}

impl LayoutBuildDiagnostics {
    fn record_prepared_lookup(&mut self, started: Option<Instant>, cache_hit: bool) {
        if cache_hit {
            self.prepared_hits = self.prepared_hits.saturating_add(1);
        } else {
            self.prepared_misses = self.prepared_misses.saturating_add(1);
        }
        add_elapsed_nanos(&mut self.prepared_lookup_nanos, started);
    }

    fn record_element_wrap(&mut self, started: Instant) {
        self.element_wraps_built = self.element_wraps_built.saturating_add(1);
        self.element_wrap_misses = self.element_wrap_misses.saturating_add(1);
        self.element_wrap_nanos = self
            .element_wrap_nanos
            .saturating_add(started.elapsed().as_nanos());
    }
}

fn add_elapsed_nanos(total: &mut u128, started: Option<Instant>) {
    if let Some(started) = started {
        *total = total.saturating_add(started.elapsed().as_nanos());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ElementWrapContext {
    element_id: crate::StableId,
    role: Role,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    runs: Arc<[TextRun]>,
    atomic_ranges: Arc<[std::ops::Range<usize>]>,
}

#[derive(Clone, Copy)]
struct ElementWrapIdentity<'a> {
    element_id: &'a crate::StableId,
    role: Role,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    text_style: &'a EffectiveTextStyle,
    char_width: FixedPoint,
    runs: &'a [TextRun],
    atomic_ranges: &'a [std::ops::Range<usize>],
}

impl ElementWrapContext {
    fn from_identity(identity: ElementWrapIdentity<'_>) -> Self {
        Self {
            element_id: identity.element_id.clone(),
            role: identity.role,
            purpose: identity.purpose,
            title_page: identity.title_page,
            geometry_mode: identity.geometry_mode,
            paragraph: identity.paragraph,
            text_style: identity.text_style.clone(),
            char_width: identity.char_width,
            runs: Arc::from(identity.runs),
            atomic_ranges: Arc::from(identity.atomic_ranges),
        }
    }

    fn matches(&self, identity: ElementWrapIdentity<'_>) -> bool {
        &self.element_id == identity.element_id
            && self.role == identity.role
            && self.purpose == identity.purpose
            && self.title_page == identity.title_page
            && self.geometry_mode == identity.geometry_mode
            && self.paragraph == identity.paragraph
            && &self.text_style == identity.text_style
            && self.char_width == identity.char_width
            && self.runs.as_ref() == identity.runs
            && self.atomic_ranges.as_ref() == identity.atomic_ranges
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EffectiveGeometryMode {
    Sequential,
    DualColumn,
}

/// Every owner-local input consumed by wrapping, source/display preparation,
/// positioned spans, and line metrics. Page composition is deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PreparedBodyEnvironment {
    purpose: LayoutPurpose,
    composition: LayoutComposition,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
}

/// Complete immutable owner-local Body output. Lines have owner-local element
/// zero, page zero, and zero vertical placement; only placement rebases them.
#[derive(Debug)]
struct PreparedBodyParagraph {
    paragraph: EffectiveParagraph,
    lines: Arc<[LayoutLine]>,
}

#[derive(Debug)]
struct PreparedBodyEntry {
    owner: Arc<BodyOwner>,
    owner_revision: u64,
    environment: PreparedBodyEnvironment,
    prepared: Arc<PreparedBodyParagraph>,
    last_used_generation: u64,
    insertion_order: u64,
    byte_cost: usize,
}

#[derive(Debug, Default)]
struct PreparedBodyCache {
    buckets: HashMap<u64, Vec<PreparedBodyEntry>>,
    /// Latest retained entry for each stable-element/environment identity.
    /// Values are (collision-bucket fingerprint, unique insertion order).
    previous_by_identity: HashMap<u64, (u64, u64)>,
    usage_order: BTreeSet<(u64, u64, u64)>,
    generation: u64,
    next_order: u64,
    entry_count: usize,
    retained_bytes: usize,
}

#[derive(Debug)]
struct ElementWrapEntry {
    context: ElementWrapContext,
    canonical_text: Arc<str>,
    lines: Arc<[WrappedLine]>,
    last_used_generation: u64,
    insertion_order: u64,
    byte_cost: usize,
}

#[derive(Debug, Default)]
struct ElementWrapCache {
    buckets: HashMap<u64, Vec<ElementWrapEntry>>,
    /// (last-used generation, unique insertion order, collision-bucket key).
    usage_order: BTreeSet<(u64, u64, u64)>,
    generation: u64,
    next_order: u64,
    entry_count: usize,
    retained_bytes: usize,
}

impl PreparedBodyCache {
    fn start_generation(&mut self) {
        if self.generation == u64::MAX {
            self.generation = 1;
            self.usage_order.clear();
            for (fingerprint, bucket) in &mut self.buckets {
                for entry in bucket {
                    entry.last_used_generation = 0;
                    self.usage_order
                        .insert((0, entry.insertion_order, *fingerprint));
                }
            }
        } else {
            self.generation = self.generation.saturating_add(1).max(1);
        }
    }

    fn clear(&mut self) {
        self.buckets.clear();
        self.previous_by_identity.clear();
        self.usage_order.clear();
        self.generation = 0;
        self.next_order = 0;
        self.entry_count = 0;
        self.retained_bytes = 0;
    }

    fn fingerprint(owner: &Arc<BodyOwner>, environment: &PreparedBodyEnvironment) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (Arc::as_ptr(owner) as usize).hash(&mut hasher);
        owner.revision().hash(&mut hasher);
        environment.hash(&mut hasher);
        hasher.finish()
    }

    fn identity_fingerprint(
        owner: &Arc<BodyOwner>,
        environment: &PreparedBodyEnvironment,
    ) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        owner.id.hash(&mut hasher);
        environment.hash(&mut hasher);
        hasher.finish()
    }

    fn previous(
        &self,
        owner: &Arc<BodyOwner>,
        environment: &PreparedBodyEnvironment,
    ) -> Option<(Arc<BodyOwner>, Arc<PreparedBodyParagraph>)> {
        let identity = Self::identity_fingerprint(owner, environment);
        let (fingerprint, insertion_order) = *self.previous_by_identity.get(&identity)?;
        self.buckets
            .get(&fingerprint)?
            .iter()
            .find(|entry| {
                entry.insertion_order == insertion_order
                    && entry.owner.id == owner.id
                    && entry.environment == *environment
            })
            .map(|entry| (Arc::clone(&entry.owner), Arc::clone(&entry.prepared)))
    }

    fn lookup(
        &mut self,
        owner: &Arc<BodyOwner>,
        environment: &PreparedBodyEnvironment,
    ) -> Option<Arc<PreparedBodyParagraph>> {
        let fingerprint = Self::fingerprint(owner, environment);
        let generation = self.generation;
        let entry = self.buckets.get_mut(&fingerprint).and_then(|bucket| {
            bucket.iter_mut().find(|entry| {
                entry.owner_revision == owner.revision()
                    && entry.environment == *environment
                    && (Arc::ptr_eq(&entry.owner, owner)
                        || entry.owner.as_element() == owner.as_element())
            })
        })?;
        self.usage_order.remove(&(
            entry.last_used_generation,
            entry.insertion_order,
            fingerprint,
        ));
        entry.last_used_generation = generation;
        self.usage_order.insert((
            entry.last_used_generation,
            entry.insertion_order,
            fingerprint,
        ));
        Some(Arc::clone(&entry.prepared))
    }

    fn insert(
        &mut self,
        owner: &Arc<BodyOwner>,
        environment: PreparedBodyEnvironment,
        prepared: Arc<PreparedBodyParagraph>,
    ) {
        let fingerprint = Self::fingerprint(owner, &environment);
        let identity = Self::identity_fingerprint(owner, &environment);
        let byte_cost = prepared_body_entry_byte_cost(owner, &environment, &prepared);
        if byte_cost > MAX_CACHED_PREPARED_BODY_BYTES {
            return;
        }
        self.next_order = self.next_order.saturating_add(1);
        let insertion_order = self.next_order;
        self.buckets
            .entry(fingerprint)
            .or_default()
            .push(PreparedBodyEntry {
                owner: Arc::clone(owner),
                owner_revision: owner.revision(),
                environment,
                prepared,
                last_used_generation: self.generation,
                insertion_order,
                byte_cost,
            });
        self.usage_order
            .insert((self.generation, insertion_order, fingerprint));
        self.previous_by_identity
            .insert(identity, (fingerprint, insertion_order));
        self.entry_count = self.entry_count.saturating_add(1);
        self.retained_bytes = self.retained_bytes.saturating_add(byte_cost);
    }

    fn evict_if_needed(&mut self) {
        while self.entry_count > MAX_CACHED_PREPARED_BODY_PARAGRAPHS
            || self.retained_bytes > MAX_CACHED_PREPARED_BODY_BYTES
        {
            let Some(oldest) = self.usage_order.iter().next().copied() else {
                self.entry_count = 0;
                self.retained_bytes = 0;
                break;
            };
            self.usage_order.remove(&oldest);
            let (_, insertion_order, fingerprint) = oldest;
            let (removed, remove_bucket) = if let Some(bucket) = self.buckets.get_mut(&fingerprint) {
                let removed = bucket
                    .iter()
                    .position(|entry| entry.insertion_order == insertion_order)
                    .map(|index| bucket.remove(index));
                let remove_bucket = bucket.is_empty();
                (removed, remove_bucket)
            } else {
                (None, false)
            };
            if remove_bucket {
                self.buckets.remove(&fingerprint);
            }
            let removed_bytes = removed.as_ref().map_or(0, |entry| entry.byte_cost);
            if let Some(entry) = removed {
                let identity = Self::identity_fingerprint(&entry.owner, &entry.environment);
                if self
                    .previous_by_identity
                    .get(&identity)
                    .is_some_and(|(_, latest_order)| *latest_order == insertion_order)
                {
                    self.previous_by_identity.remove(&identity);
                }
            }
            self.entry_count = self.entry_count.saturating_sub(1);
            self.retained_bytes = self.retained_bytes.saturating_sub(removed_bytes);
        }
    }
}

fn text_style_heap_bytes(style: &EffectiveTextStyle) -> usize {
    style
        .declared_family
        .len()
        .saturating_add(style.source_color.as_ref().map_or(0, String::len))
}

fn prepared_body_entry_byte_cost(
    owner: &Arc<BodyOwner>,
    environment: &PreparedBodyEnvironment,
    prepared: &PreparedBodyParagraph,
) -> usize {
    let element = owner.as_element();
    let mut total = std::mem::size_of::<PreparedBodyEntry>()
        .saturating_add(std::mem::size_of::<BodyOwner>())
        .saturating_add(std::mem::size_of::<PreparedBodyParagraph>())
        .saturating_add(
            prepared
                .lines
                .len()
                .saturating_mul(std::mem::size_of::<LayoutLine>()),
        )
        .saturating_add(element.id.as_str().len())
        .saturating_add(element.text.len())
        .saturating_add(text_style_heap_bytes(&environment.text_style));
    for run in &element.runs {
        total = total
            .saturating_add(std::mem::size_of::<TextRun>())
            .saturating_add(run.text.len());
        for attribute in &run.attributes {
            total = total
                .saturating_add(std::mem::size_of::<RunAttribute>())
                .saturating_add(attribute.name.len())
                .saturating_add(attribute.value.len());
        }
    }
    if let Some(provenance) = &element.source_provenance {
        total = total
            .saturating_add(provenance.foreign_id.as_ref().map_or(0, String::len))
            .saturating_add(provenance.foreign_role.as_ref().map_or(0, String::len));
    }
    for line in prepared.lines.iter() {
        total = total
            .saturating_add(line.text.len())
            .saturating_add(line.display_text.len())
            .saturating_add(
                line.display_source_map
                    .len()
                    .saturating_mul(std::mem::size_of::<DisplaySourceSegment>()),
            )
            .saturating_add(
                line.spans
                    .len()
                    .saturating_mul(std::mem::size_of::<PositionedStyledSpan>()),
            )
            .saturating_add(text_style_heap_bytes(&line.text_style));
        for span in line.spans.iter() {
            total = total
                .saturating_add(span.display_text.len())
                .saturating_add(
                    span.display_source_map
                        .len()
                        .saturating_mul(std::mem::size_of::<DisplaySourceSegment>()),
                )
                .saturating_add(text_style_heap_bytes(&span.text_style));
        }
    }
    total
}

impl ElementWrapCache {
    fn start_generation(&mut self) {
        if self.generation == u64::MAX {
            self.generation = 1;
            self.usage_order.clear();
            for (fingerprint, bucket) in &mut self.buckets {
                for entry in bucket {
                    entry.last_used_generation = 0;
                    self.usage_order
                        .insert((0, entry.insertion_order, *fingerprint));
                }
            }
        } else {
            self.generation = self.generation.saturating_add(1).max(1);
        }
    }

    fn clear(&mut self) {
        self.buckets.clear();
        self.usage_order.clear();
        self.generation = 0;
        self.next_order = 0;
        self.entry_count = 0;
        self.retained_bytes = 0;
    }

    fn fingerprint(&self, identity: &ElementWrapIdentity<'_>, text: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        identity.element_id.hash(&mut hasher);
        identity.role.hash(&mut hasher);
        identity.purpose.hash(&mut hasher);
        identity.title_page.hash(&mut hasher);
        identity.geometry_mode.hash(&mut hasher);
        identity.paragraph.hash(&mut hasher);
        identity.text_style.hash(&mut hasher);
        identity.char_width.hash(&mut hasher);
        identity.runs.len().hash(&mut hasher);
        for run in identity.runs {
            run.text.hash(&mut hasher);
            run.attributes.len().hash(&mut hasher);
            for attribute in &run.attributes {
                attribute.name.hash(&mut hasher);
                attribute.value.hash(&mut hasher);
            }
        }
        identity.atomic_ranges.len().hash(&mut hasher);
        for range in identity.atomic_ranges {
            range.start.hash(&mut hasher);
            range.end.hash(&mut hasher);
        }
        text.hash(&mut hasher);
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
        atomic_ranges: &[std::ops::Range<usize>],
        atomic_boundaries: &[std::ops::Range<usize>],
        purpose: LayoutPurpose,
        title_page: bool,
        geometry_mode: EffectiveGeometryMode,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Vec<WrappedLine> {
        let identity = ElementWrapIdentity {
            element_id,
            role,
            purpose,
            title_page,
            geometry_mode,
            paragraph,
            text_style: &text_style,
            char_width,
            runs,
            atomic_ranges,
        };
        let fingerprint = self.fingerprint(&identity, text);
        if let Some(entry) = self.buckets.get_mut(&fingerprint).and_then(|bucket| {
            bucket
                .iter_mut()
                .find(|entry| entry.context.matches(identity) && entry.canonical_text.as_ref() == text)
        }) {
            self.usage_order.remove(&(
                entry.last_used_generation,
                entry.insertion_order,
                fingerprint,
            ));
            entry.last_used_generation = self.generation;
            self.usage_order.insert((
                entry.last_used_generation,
                entry.insertion_order,
                fingerprint,
            ));
            if let Some(diagnostics) = diagnostics {
                diagnostics.element_wraps_reused =
                    diagnostics.element_wraps_reused.saturating_add(1);
            }
            return entry
                .lines
                .iter()
                .cloned()
                .map(|mut line| {
                    line.element = element;
                    line
                })
                .collect();
        }

        let started = diagnostics.as_ref().map(|_| Instant::now());
        let context = ElementWrapContext::from_identity(identity);
        let lines = wrap_element(
            text,
            element,
            role,
            paragraph,
            text_style,
            char_width,
            atomic_boundaries,
            true,
            0,
        );
        if let (Some(diagnostics), Some(started)) = (diagnostics, started) {
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
        let byte_cost =
            text.len()
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
                lines: cached_lines.into(),
                last_used_generation: self.generation,
                insertion_order: self.next_order,
                byte_cost,
            });
        self.usage_order
            .insert((self.generation, self.next_order, fingerprint));
        self.entry_count = self.entry_count.saturating_add(1);
        self.retained_bytes = self.retained_bytes.saturating_add(byte_cost);
        lines
    }

    fn evict_if_needed(&mut self) {
        while self.entry_count > MAX_CACHED_ELEMENT_WRAPS
            || self.retained_bytes > MAX_CACHED_ELEMENT_WRAP_BYTES
        {
            let Some(oldest) = self.usage_order.iter().next().copied() else {
                self.entry_count = 0;
                break;
            };
            self.usage_order.remove(&oldest);
            let (_, insertion_order, fingerprint) = oldest;
            let (removed_bytes, remove_bucket) =
                if let Some(bucket) = self.buckets.get_mut(&fingerprint) {
                    let removed = bucket
                        .iter()
                        .position(|entry| entry.insertion_order == insertion_order)
                        .map(|index| bucket.remove(index));
                    let remove_bucket = bucket.is_empty();
                    (
                        removed.map_or(0, |entry| entry.byte_cost),
                        remove_bucket,
                    )
                } else {
                    (0, false)
                };
            if remove_bucket {
                self.buckets.remove(&fingerprint);
            }
            self.entry_count = self.entry_count.saturating_sub(1);
            self.retained_bytes = self.retained_bytes.saturating_sub(removed_bytes);
        }
    }
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
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            None,
            false,
            presentation.clone(),
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
        session: &EditingSession,
        presentation: &BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            None,
            true,
            presentation.clone(),
        )
    }

    /// Diagnostic counterpart to [`Self::get_or_build_body_editor_for_session`].
    pub fn get_or_build_body_editor_for_session_with_diagnostics(
        &mut self,
        session: &EditingSession,
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
        session: &EditingSession,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::TitlePageEditor,
            None,
            true,
            BodyEditorPresentation::default(),
        )
    }

    /// Diagnostic counterpart to [`Self::get_or_build_title_page_editor_for_session`].
    pub fn get_or_build_title_page_editor_for_session_with_diagnostics(
        &mut self,
        session: &EditingSession,
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
        session: &EditingSession,
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
        let whole_build_started = diagnostics.as_ref().map(|_| Instant::now());
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
            self.prepared_body.clear();
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
        self.prepared_body.start_generation();
        let layout = Arc::new(
            ScreenplayLayout::build_with_effective_inputs_for_purpose_internal(
                project,
                inputs,
                &source_settings,
                purpose,
                composition,
                diagnostics.as_deref_mut(),
                Some(&mut self.element_wraps),
                Some(&mut self.prepared_body),
                true,
                &body_presentation,
            )?,
        );
        self.element_wraps.evict_if_needed();
        self.prepared_body.evict_if_needed();
        self.revision = Some(revision);
        self.inputs = Some(inputs.clone());
        self.source_settings = Some(source_settings);
        self.purpose = Some(purpose);
        self.composition = Some(composition);
        self.body_presentation = Some(body_presentation);
        self.project_layout_signature = project_layout_signature;
        self.layout = Some(layout.clone());
        self.project_identity = Some(project.layout_identity().clone());
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            add_elapsed_nanos(&mut diagnostics.whole_build_nanos, whole_build_started);
        }
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
            .map(|preservation| preservation.source_layout_settings().clone())
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
        mut prepared_body_cache: Option<&mut PreparedBodyCache>,
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
                let lines = wrap_for_layout(
                    wrap_cache.as_deref_mut(),
                    diagnostics.as_deref_mut(),
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
                let placement_started = diagnostics.as_ref().map(|_| Instant::now());
                for line in lines {
                    let line_advance = line.paragraph.line_advance.raw();
                    let laid_out = output.make_line(page.number, line, true, page.next_line_y);
                    page.lines.push(laid_out);
                    page.occupied_lines = page.occupied_lines.saturating_add(1);
                    page.next_line_y =
                        FixedPoint::from_raw(page.next_line_y.raw().saturating_add(line_advance));
                }
                if let Some(diagnostics) = diagnostics.as_deref_mut() {
                    add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
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
                    project.uses_legacy_layout_defaults(),
                    purpose,
                    composition,
                    &output,
                    &mut prepared_body_cache,
                    &mut wrap_cache,
                    &mut diagnostics,
                );
                let right = wrap_dual_side(
                    project,
                    &group.right,
                    right_box,
                    &output.inputs,
                    source_settings,
                    project.uses_legacy_layout_defaults(),
                    purpose,
                    composition,
                    &output,
                    &mut prepared_body_cache,
                    &mut wrap_cache,
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
                                paragraph
                                    .prepared
                                    .lines
                                    .first()
                                    .map(|_| paragraph.prepared.paragraph.space_before.raw())
                            })
                            .unwrap_or_default()
                    };
                    let space_before = FixedPoint::from_raw(
                        space_before
                            .raw()
                            .max(first_visible_space_before(&left))
                            .max(first_visible_space_before(&right)),
                    );
                    let placement_started = diagnostics.as_ref().map(|_| Instant::now());
                    output.place_dual_block(
                        &left,
                        &right,
                        starts_new_page,
                        space_before,
                        &mut diagnostics,
                    );
                    if let Some(diagnostics) = diagnostics.as_deref_mut() {
                        add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
                    }
                }
                index = crate::dual_dialogue::last_member(project, group)
                    .map_or(index.saturating_add(1), |last| last.saturating_add(1));
                continue;
                }
            }

            let prepared_lookup_started = diagnostics.as_ref().map(|_| Instant::now());
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
                project.uses_legacy_layout_defaults(),
                &text_style,
            );
            let environment = PreparedBodyEnvironment {
                purpose,
                composition,
                geometry_mode: EffectiveGeometryMode::Sequential,
                paragraph,
                text_style,
                char_width: paragraph.char_width,
            };
            let prepared = prepare_body_for_layout(
                prepared_body_cache.as_deref_mut(),
                wrap_cache.as_deref_mut(),
                diagnostics.as_deref_mut(),
                &output,
                element,
                environment,
                prepared_lookup_started,
            );
            let placement_started = diagnostics.as_ref().map(|_| Instant::now());
            output.place_element(prepared, index, element.role, paragraph, &mut diagnostics);
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
            }
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

    fn rebuild_interaction_indexes(
        &mut self,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) {
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
        let sort_started = diagnostics.as_ref().map(|_| Instant::now());
        self.body_line_y_order = (0..self.body_line_index.len()).collect();
        self.body_line_y_order.sort_by_key(|index| {
            let entry = self.body_line_index[*index];
            (entry.absolute_y, entry.location.page, entry.location.line)
        });
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            add_elapsed_nanos(&mut diagnostics.body_y_sort_nanos, sort_started);
            add_elapsed_nanos(&mut diagnostics.interaction_index_nanos, started);
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
        page_number: usize,
        line: WrappedLine,
        title_page: bool,
        y: FixedPoint,
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
        let mut spans = positioned_spans(&line, FixedRect {
            x: line_left,
            y,
            width: FixedPoint::default(),
            height: line.paragraph.line_advance,
        });
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
            page: page_number,
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

    fn place_dual_side(
        &self,
        page_number: usize,
        side: &WrappedDualSide,
        block_top: FixedPoint,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) -> Vec<LayoutLine> {
        let mut y = block_top;
        let mut placed = Vec::with_capacity(side.line_count);
        for (paragraph_index, paragraph) in side.paragraphs.iter().enumerate() {
            if paragraph_index > 0 && !paragraph.prepared.lines.is_empty() {
                y = FixedPoint::from_raw(
                    y.raw()
                        .saturating_add(paragraph.prepared.paragraph.space_before.raw().max(0)),
                );
            }
            for wrapped in paragraph.prepared.lines.iter() {
                let advance = paragraph.prepared.paragraph.line_advance;
                let mut line = rebase_prepared_line(
                    wrapped,
                    paragraph.element,
                    page_number,
                    y,
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
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) {
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
            self.spill_oversized_dual_block(left, right, diagnostics);
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
        let page_number = self.pages[page_index].number;
        let left_lines = self.place_dual_side(page_number, left, block_top, diagnostics);
        let right_lines = self.place_dual_side(page_number, right, block_top, diagnostics);
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
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) {
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
                let page_number = self.pages[page_index].number;
                let mut line = rebase_prepared_line(
                    &placement.line,
                    placement.element,
                    page_number,
                    placement.y,
                    diagnostics.as_deref_mut(),
                );
                line.oversized = true;
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
        prepared: Arc<PreparedBodyParagraph>,
        element: usize,
        role: Role,
        paragraph: EffectiveParagraph,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) {
        let lines = &prepared.lines;
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
        if !lines.is_empty()
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
        for wrapped in lines.iter() {
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
            let page_number = self.pages[page_index].number;
            let next_line_y = self.pages[page_index].next_line_y;
            let mut line = rebase_prepared_line(
                wrapped,
                element,
                page_number,
                next_line_y,
                diagnostics.as_deref_mut(),
            );
            line.oversized |= oversized_element;
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

fn rebase_prepared_line(
    prepared: &LayoutLine,
    element: usize,
    page: usize,
    y: FixedPoint,
    diagnostics: Option<&mut LayoutBuildDiagnostics>,
) -> LayoutLine {
    // Per-line timing is intentionally limited to an active optional carrier.
    // Development readings include this measurement overhead.
    let started = diagnostics.as_ref().map(|_| Instant::now());
    let mut line = prepared.clone();
    let vertical_delta = y.raw().saturating_sub(prepared.paragraph_bounds.y.raw());
    line.page = page;
    line.element = element;
    line.source.element = element;
    line.geometry.y = FixedPoint::from_raw(prepared.geometry.y.raw().saturating_add(vertical_delta));
    line.paragraph_bounds.y = y;
    line.rect = compat_rect(line.paragraph_bounds);
    for span in Arc::make_mut(&mut line.spans) {
        span.source.element = element;
        span.geometry.y = FixedPoint::from_raw(span.geometry.y.raw().saturating_add(vertical_delta));
    }
    if let Some(diagnostics) = diagnostics {
        add_elapsed_nanos(&mut diagnostics.rebase_nanos, started);
    }
    line
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

fn positioned_spans(line: &WrappedLine, line_geometry: FixedRect) -> Vec<PositionedStyledSpan> {
    if line.display_text.is_empty() {
        return Vec::new();
    }
    let mut run_styles = Vec::with_capacity(line.runs.len());
    let mut source_cursor = 0usize;
    for run in line.runs.iter() {
        let end = source_cursor.saturating_add(run.text.len());
        let flags = parse_style_attribute(&run.attributes)
            .map(|projection| projection.flags)
            .unwrap_or_default();
        let mut style = line.text_style.clone();
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
        run_styles.push((source_cursor..end, style, flags.strikethrough));
        source_cursor = end;
    }

    let base_size = i64::from(line.text_style.size.max(1));
    let mut output: Vec<PositionedSpanBuilder> = Vec::new();
    let mut x = line_geometry.x.raw();
    let mut mapping_index = 0usize;
    let mut run_index = 0usize;
    for (display_index, character) in line.display_text.chars().enumerate() {
        while line
            .display_source_map
            .get(mapping_index)
            .is_some_and(|segment| display_index >= segment.display_end)
        {
            mapping_index = mapping_index.saturating_add(1);
        }
        let Some(mapping) = line
            .display_source_map
            .get(mapping_index)
            .filter(|segment| {
                display_index >= segment.display_start && display_index < segment.display_end
            })
        else {
            continue;
        };
        while run_styles
            .get(run_index)
            .is_some_and(|(range, _, _)| mapping.source_start >= range.end)
        {
            run_index = run_index.saturating_add(1);
        }
        let (style, strikethrough) = run_styles
            .get(run_index)
            .filter(|(range, _, _)| {
                mapping.source_start >= range.start && mapping.source_start < range.end
            })
            .map(|(_, style, strikethrough)| (style, *strikethrough))
            .unwrap_or((&line.text_style, false));
        let source_start = mapping.source_start;
        let source_end = mapping.source_end;
        let cell = FixedPoint::from_raw(
            line.char_width
                .raw()
                .saturating_mul(i64::from(style.size.max(1)))
                .div_euclid(base_size)
                .max(1),
        );
        let same = output.last().is_some_and(|span| {
            &span.text_style == style
                && span.underline == style.underline
                && span.strikethrough == strikethrough
                && !character.is_whitespace()
                && !span.display_text.chars().last().is_some_and(char::is_whitespace)
                && (span.source.end == source_start
                    || (span.source.start == source_start && span.source.end == source_end))
        });
        if same {
            output
                .last_mut()
                .expect("checked span exists")
                .push(character, source_start, source_end, cell);
        } else {
            output.push(PositionedSpanBuilder {
                display_text: character.to_string(),
                source: SourceRange {
                    element: line.element,
                    start: source_start,
                    end: source_end,
                },
                display_source_map: vec![DisplaySourceSegment {
                    display_start: 0,
                    display_end: 1,
                    source_start,
                    source_end,
                }],
                text_style: style.clone(),
                char_width: cell,
                geometry: FixedRect {
                    x: FixedPoint::from_raw(x),
                    y: line_geometry.y,
                    width: cell,
                    height: line_geometry.height,
                },
                underline: style.underline,
                strikethrough,
            });
        }
        x = x.saturating_add(cell.raw());
    }
    output.into_iter().map(PositionedSpanBuilder::finish).collect()
}

struct PositionedSpanBuilder {
    display_text: String,
    source: SourceRange,
    display_source_map: Vec<DisplaySourceSegment>,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    geometry: FixedRect,
    underline: bool,
    strikethrough: bool,
}

impl PositionedSpanBuilder {
    fn push(
        &mut self,
        character: char,
        source_start: usize,
        source_end: usize,
        cell: FixedPoint,
    ) {
        let display_start = self
            .display_source_map
            .last()
            .map_or(0, |segment| segment.display_end);
        self.display_text.push(character);
        self.source.end = source_end;
        self.geometry.width = FixedPoint::from_raw(
            self.geometry.width.raw().saturating_add(cell.raw()),
        );
        self.char_width = cell;
        self.display_source_map.push(DisplaySourceSegment {
            display_start,
            display_end: display_start.saturating_add(1),
            source_start,
            source_end,
        });
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

struct PreparedDualParagraph {
    element: usize,
    prepared: Arc<PreparedBodyParagraph>,
}

struct WrappedDualSide {
    paragraphs: Vec<PreparedDualParagraph>,
    height: FixedPoint,
    line_count: usize,
}

struct SpilledDualLine {
    element: usize,
    line: LayoutLine,
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

    for (paragraph_index, paragraph) in side.paragraphs.iter().enumerate() {
        let mut gap_pending = paragraph_index > 0;
        for wrapped in paragraph.prepared.lines.iter() {
            let advance = paragraph.prepared.paragraph.line_advance.raw().max(0);
            let current_count = result.lines_per_page[page_offset];
            let gap = if gap_pending && current_count > 0 {
                paragraph.prepared.paragraph.space_before.raw().max(0)
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
                paragraph.prepared.paragraph.space_before.raw().max(0)
            } else {
                0
            };
            let line_y = FixedPoint::from_raw(y.raw().saturating_add(line_gap));
            let line_bottom = line_y.raw().saturating_add(advance);
            result.lines.push(SpilledDualLine {
                element: paragraph.element,
                line: wrapped.clone(),
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
    composition: LayoutComposition,
    layout: &ScreenplayLayout,
    prepared_cache: &mut Option<&mut PreparedBodyCache>,
    cache: &mut Option<&mut ElementWrapCache>,
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
        let prepared_lookup_started = diagnostics.as_ref().map(|_| Instant::now());
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
        let environment = PreparedBodyEnvironment {
            purpose,
            composition,
            geometry_mode: EffectiveGeometryMode::DualColumn,
            paragraph,
            text_style,
            char_width: paragraph.char_width,
        };
        let prepared = prepare_body_for_layout(
            prepared_cache.as_deref_mut(),
            cache.as_deref_mut(),
            diagnostics.as_deref_mut(),
            layout,
            element,
            environment,
            prepared_lookup_started,
        );
        if !paragraphs.is_empty() {
            height = height.saturating_add(paragraph.space_before.raw().max(0));
        }
        height = height.saturating_add(
            paragraph
                .line_advance
                .raw()
                .saturating_mul(prepared.lines.len().min(i64::MAX as usize) as i64),
        );
        line_count = line_count.saturating_add(prepared.lines.len());
        paragraphs.push(PreparedDualParagraph {
            element: index,
            prepared,
        });
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

fn prepare_body_for_layout(
    mut prepared_cache: Option<&mut PreparedBodyCache>,
    wrap_cache: Option<&mut ElementWrapCache>,
    mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
    layout: &ScreenplayLayout,
    owner: &Arc<BodyOwner>,
    environment: PreparedBodyEnvironment,
    environment_started: Option<Instant>,
) -> Arc<PreparedBodyParagraph> {
    let cache_attempted = prepared_cache.is_some();
    let cached = prepared_cache
        .as_deref_mut()
        .and_then(|cache| cache.lookup(owner, &environment));
    if cache_attempted {
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.record_prepared_lookup(environment_started, cached.is_some());
        }
    }
    if let Some(prepared) = cached {
        return prepared;
    }
    let miss_started = (cache_attempted && diagnostics.is_some()).then(Instant::now);

    let previous = prepared_cache
        .as_deref_mut()
        .and_then(|cache| cache.previous(owner, &environment));
    let incremental = previous.as_ref().and_then(|(previous_owner, previous)| {
        prepare_incremental_body_for_layout(
            diagnostics.as_deref_mut(),
            layout,
            owner,
            previous_owner,
            previous,
            &environment,
        )
    });
    if cache_attempted {
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            if incremental.is_some() {
                diagnostics.prepared_incremental_hits =
                    diagnostics.prepared_incremental_hits.saturating_add(1);
            } else {
                diagnostics.prepared_incremental_fallbacks =
                    diagnostics.prepared_incremental_fallbacks.saturating_add(1);
            }
        }
    }
    let element = owner.as_element();
    let paragraph = environment.paragraph;
    let prepared = if let Some(prepared) = incremental {
        prepared
    } else {
        let lines = wrap_for_layout(
            wrap_cache,
            diagnostics.as_deref_mut(),
            &element.id,
            &element.text,
            0,
            element.role,
            paragraph,
            environment.text_style.clone(),
            &element.runs,
            environment.char_width,
            environment.purpose,
            false,
            environment.geometry_mode,
        );
        Arc::new(PreparedBodyParagraph {
            paragraph,
            lines: lines
                .into_iter()
                .map(|line| layout.make_line(0, line, false, FixedPoint::default()))
                .collect::<Vec<_>>()
                .into(),
        })
    };
    if let Some(cache) = prepared_cache {
        cache.insert(owner, environment, Arc::clone(&prepared));
    }
    if let Some(diagnostics) = diagnostics.as_deref_mut() {
        add_elapsed_nanos(&mut diagnostics.prepared_miss_nanos, miss_started);
    }
    prepared
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SingleTextEdit {
    start: usize,
    old_end: usize,
    new_end: usize,
}

fn single_insertion_or_deletion(old: &str, new: &str) -> Option<SingleTextEdit> {
    if old == new {
        return None;
    }
    let old_bytes = old.as_bytes();
    let new_bytes = new.as_bytes();
    let shared = old_bytes.len().min(new_bytes.len());
    let mut start = 0usize;
    while start < shared && old_bytes[start] == new_bytes[start] {
        start = start.saturating_add(1);
    }
    let mut suffix = 0usize;
    while suffix < shared.saturating_sub(start)
        && old_bytes[old_bytes.len() - 1 - suffix] == new_bytes[new_bytes.len() - 1 - suffix]
    {
        suffix = suffix.saturating_add(1);
    }
    let old_end = old_bytes.len().saturating_sub(suffix);
    let new_end = new_bytes.len().saturating_sub(suffix);
    if !old.is_char_boundary(start)
        || !new.is_char_boundary(start)
        || !old.is_char_boundary(old_end)
        || !new.is_char_boundary(new_end)
        || (start < old_end && start < new_end)
    {
        return None;
    }
    Some(SingleTextEdit {
        start,
        old_end,
        new_end,
    })
}

fn runs_match_single_edit(
    old: &crate::ScreenplayElement,
    new: &crate::ScreenplayElement,
    edit: SingleTextEdit,
) -> bool {
    if old.runs.is_empty() || new.runs.is_empty() {
        return old.runs.is_empty() && new.runs.is_empty();
    }
    if old.runs.len() != new.runs.len() {
        return false;
    }
    let shift = new.text.len() as i128 - old.text.len() as i128;
    let mut old_base = 0usize;
    let mut new_base = 0usize;
    for (old_run, new_run) in old.runs.iter().zip(&new.runs) {
        if old_run.attributes != new_run.attributes {
            return false;
        }
        let old_end = old_base.saturating_add(old_run.text.len());
        let new_end = new_base.saturating_add(new_run.text.len());
        let before_edit = old_end <= edit.start || new_end <= edit.start;
        if before_edit {
            let shared_end = old_end.min(new_end).min(edit.start);
            let shared_len = shared_end.saturating_sub(old_base);
            if old_base != new_base
                || old_run.text[..shared_len] != new_run.text[..shared_len]
                || (old_end != new_end && (old_end < edit.start || new_end < edit.start))
            {
                return false;
            }
        }
        let after_edit = old_base >= edit.old_end || new_base >= edit.new_end;
        if after_edit {
            let shifted_old_base = old_base as i128 + shift;
            if old_base < edit.old_end
                || new_base < edit.new_end
                || shifted_old_base != new_base as i128
                || old_run.text != new_run.text
            {
                return false;
            }
        }
        old_base = old_end;
        new_base = new_end;
    }
    old_base == old.text.len() && new_base == new.text.len()
}

fn prepare_incremental_body_for_layout(
    diagnostics: Option<&mut LayoutBuildDiagnostics>,
    layout: &ScreenplayLayout,
    owner: &Arc<BodyOwner>,
    previous_owner: &Arc<BodyOwner>,
    previous: &Arc<PreparedBodyParagraph>,
    environment: &PreparedBodyEnvironment,
) -> Option<Arc<PreparedBodyParagraph>> {
    let old = previous_owner.as_element();
    let new = owner.as_element();
    if old.id != new.id
        || old.role != new.role
        || old.source_provenance != new.source_provenance
        || previous.paragraph != environment.paragraph
    {
        return None;
    }
    let edit = single_insertion_or_deletion(&old.text, &new.text)?;
    if !runs_match_single_edit(old, new, edit) {
        return None;
    }
    let first_affected = previous
        .lines
        .iter()
        .position(|line| line.source.end >= edit.start)
        .unwrap_or(previous.lines.len().saturating_sub(1));
    // Rebuild the affected line and its predecessor so backward whitespace
    // selection, source-boundary affinity, and final-line justification cannot
    // inherit a stale break context. Empty prefixes use complete preparation.
    let reused_lines = first_affected.checked_sub(1)?;
    let suffix_start = previous.lines.get(reused_lines)?.source.start;
    if suffix_start > edit.start
        || !old.text.is_char_boundary(suffix_start)
        || !new.text.is_char_boundary(suffix_start)
        || old.text[..suffix_start] != new.text[..suffix_start]
        || previous.lines[..reused_lines]
            .iter()
            .any(|line| line.source.end > edit.start)
    {
        return None;
    }
    let suffix = wrap_for_layout_range(
        None,
        diagnostics,
        &new.id,
        &new.text,
        suffix_start..new.text.len(),
        false,
        reused_lines,
        0,
        new.role,
        environment.paragraph,
        environment.text_style.clone(),
        &new.runs,
        environment.char_width,
        environment.purpose,
        false,
        environment.geometry_mode,
    );
    let mut lines = previous.lines[..reused_lines].to_vec();
    lines.extend(
        suffix
            .into_iter()
            .map(|line| layout.make_line(0, line, false, FixedPoint::default())),
    );
    Some(Arc::new(PreparedBodyParagraph {
        paragraph: environment.paragraph,
        lines: lines.into(),
    }))
}

#[allow(clippy::too_many_arguments)]
fn wrap_for_layout(
    cache: Option<&mut ElementWrapCache>,
    diagnostics: Option<&mut LayoutBuildDiagnostics>,
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
    wrap_for_layout_range(
        cache,
        diagnostics,
        element_id,
        text,
        0..text.len(),
        true,
        0,
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
}

#[allow(clippy::too_many_arguments)]
fn wrap_for_layout_range(
    cache: Option<&mut ElementWrapCache>,
    diagnostics: Option<&mut LayoutBuildDiagnostics>,
    element_id: &crate::StableId,
    text: &str,
    source_range: std::ops::Range<usize>,
    first_visual_line: bool,
    line_offset: usize,
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
    let mut projected_text = String::new();
    let mut source_map = Vec::<(std::ops::Range<usize>, std::ops::Range<usize>)>::new();
    let mut run_styles = Vec::with_capacity(runs.len());
    let mut source_cursor = 0usize;
    for run in runs {
        let end = source_cursor.saturating_add(run.text.len());
        let flags = parse_style_attribute(&run.attributes)
            .map(|projection| projection.flags)
            .unwrap_or_default();
        run_styles.push((source_cursor..end, flags));
        source_cursor = end;
    }
    let base_size = i64::from(text_style.size.max(1));
    let maximum_run_size = runs.iter().flat_map(|run| run.attributes.iter())
        .filter(|attribute| attribute.name == "Size")
        .filter_map(|attribute| attribute.value.parse::<u8>().ok())
        .filter(|size| (1..=72).contains(size))
        .map(i64::from)
        .max()
        .unwrap_or(base_size)
        .max(base_size);
    // Wrapping uses a conservative largest direct run metric; span placement
    // below restores each run's own fixed-cell width without host font advances.
    let wrapping_char_width = FixedPoint::from_raw(
        char_width.raw().saturating_mul(maximum_run_size).div_euclid(base_size).max(1),
    );
    let mut run_index = 0usize;
    for (local_start, scalar) in text[source_range.clone()].char_indices() {
        let source_start = source_range.start.saturating_add(local_start);
        let source_end = source_start + scalar.len_utf8();
        while run_styles
            .get(run_index)
            .is_some_and(|(range, _)| source_start >= range.end)
        {
            run_index = run_index.saturating_add(1);
        }
        let flags = run_styles
            .get(run_index)
            .filter(|(range, _)| source_start >= range.start && source_start < range.end)
            .map(|(_, flags)| *flags)
            .unwrap_or_default();
        let displayed = if text_style.all_caps || flags.all_caps {
            scalar.to_uppercase().collect::<String>()
        } else {
            scalar.to_string()
        };
        let start = projected_text.len();
        projected_text.push_str(&displayed);
        source_map.push((start..projected_text.len(), source_start..source_end));
    }
    // The complete ranges remain cache identity; only genuine display expansions
    // need an interior-boundary lookup. The noncache path avoids the full temporary.
    let atomic_ranges = cache.is_some().then(|| {
        source_map
            .iter()
            .map(|(display, _)| display.clone())
            .collect::<Vec<_>>()
    });
    let atomic_boundaries = atomic_boundary_index(&projected_text, &source_map);
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
            atomic_ranges.as_deref().expect("cached wrapping retains complete ranges"),
            &atomic_boundaries,
            purpose,
            title_page,
            geometry_mode,
            diagnostics,
        )
    } else {
        let started = diagnostics.as_ref().map(|_| Instant::now());
        let mut wrapping_style = text_style.clone();
        wrapping_style.all_caps = false;
        let lines = wrap_element(
            &projected_text,
            element,
            role,
            paragraph,
            wrapping_style,
            wrapping_char_width,
            &atomic_boundaries,
            first_visual_line,
            line_offset,
        );
        if let (Some(diagnostics), Some(started)) = (diagnostics, started) {
            diagnostics.record_element_wrap(started);
        }
        lines
    };
    let owner_runs: Arc<[TextRun]> = Arc::from(runs.to_vec());
    let mut line_map_index = 0usize;
    let mut segment_map_index = 0usize;
    for line in &mut lines {
        let projected_start = line.start;
        let projected_end = line.end;
        let mapped_start = source_start_for_display_offset(
            &source_map,
            &mut line_map_index,
            projected_start,
        )
        .unwrap_or(text.len());
        let mapped_end = source_end_for_display_offset(
            &source_map,
            &mut line_map_index,
            projected_end,
        )
        .unwrap_or(mapped_start);
        line.start = mapped_start;
        line.end = mapped_end;
        line.text = Arc::from(&text[mapped_start..mapped_end]);
        for segment in Arc::make_mut(&mut line.display_source_map) {
            let start = source_start_for_display_offset(
                &source_map,
                &mut segment_map_index,
                segment.source_start,
            )
            .unwrap_or(mapped_start);
            let end = source_end_for_display_offset(
                &source_map,
                &mut segment_map_index,
                segment.source_end,
            )
            .unwrap_or(start);
            segment.source_start = start;
            segment.source_end = end;
        }
        line.text_style = text_style.clone();
        line.char_width = char_width;
        line.runs = Arc::clone(&owner_runs);
    }
    lines
}

fn source_start_for_display_offset(
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
    cursor: &mut usize,
    display_offset: usize,
) -> Option<usize> {
    while source_map
        .get(*cursor)
        .is_some_and(|(display, _)| display_offset >= display.end)
    {
        *cursor = (*cursor).saturating_add(1);
    }
    source_map
        .get(*cursor)
        .filter(|(display, _)| display_offset >= display.start && display_offset < display.end)
        .map(|(_, source)| source.start)
}

fn source_end_for_display_offset(
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
    cursor: &mut usize,
    display_offset: usize,
) -> Option<usize> {
    while source_map
        .get(*cursor)
        .is_some_and(|(display, _)| display_offset > display.end)
    {
        *cursor = (*cursor).saturating_add(1);
    }
    source_map
        .get(*cursor)
        .filter(|(display, _)| display_offset > display.start && display_offset <= display.end)
        .map(|(_, source)| source.end)
}

fn atomic_boundary_index(
    text: &str,
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
) -> Vec<std::ops::Range<usize>> {
    let mut index = Vec::new();
    for (display, _) in source_map {
        if text[display.clone()].char_indices().nth(1).is_some() {
            index.push(display.clone());
        }
    }
    index
}

fn atomic_boundary_is_forbidden(
    atomic_boundaries: &[std::ops::Range<usize>],
    boundary: usize,
) -> bool {
    let candidate = atomic_boundaries.partition_point(|range| range.end <= boundary);
    atomic_boundaries
        .get(candidate)
        .is_some_and(|range| range.start < boundary)
}

fn wrap_element(
    text: &str,
    element: usize,
    role: Role,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    atomic_boundaries: &[std::ops::Range<usize>],
    first_visual_line: bool,
    line_offset: usize,
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
    let mut first_visual_line = first_visual_line;
    let mut paragraph_start = 0;
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
            atomic_boundaries,
            &mut first_visual_line,
            line_offset,
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
            atomic_boundaries,
            &mut first_visual_line,
        line_offset,
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
            first_visual_line,
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
    atomic_boundaries: &[std::ops::Range<usize>],
    first_visual_line: &mut bool,
    line_offset: usize,
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
    while start < cells.len() && line_offset.saturating_add(output.len()) < MAX_LINES_PER_ELEMENT {
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
            if atomic_boundary_is_forbidden(atomic_boundaries, boundary) {
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
