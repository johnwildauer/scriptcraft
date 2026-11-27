//! Durable Story Map values. Disposable presentation state belongs to the
//! document/view boundary and is intentionally absent from these types.

/// Safe target used when no usable native target length can be projected.
pub const DEFAULT_STORY_MAP_TARGET_PAGES: u32 = 120;
/// Largest supported target. This bounds all eighth-page conversions and the
/// downstream timeline domain while leaving room for documents beyond 120 pages.
pub const MAX_STORY_MAP_TARGET_PAGES: u32 = 10_000;
/// Maximum number of ordered dynamic lanes in one Story Map.
pub const MAX_STORY_MAP_LANES: usize = 64;
/// Maximum UTF-8 byte length for Story Map UUID, label, and type strings.
pub const MAX_STORY_MAP_STRING_BYTES: usize = crate::element::MAX_PROVENANCE_STRING_BYTES;
/// Number of semantic placement units in one page.
pub const STORY_MAP_EIGHTHS_PER_PAGE: u64 = 8;

/// One ordered dynamic lane. `level` is retained metadata; `lane_type` is the
/// exact native Type value and is not inferred from level or vector position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMapLane {
    pub uuid: String,
    pub label: String,
    pub level: i64,
    pub lane_type: String,
}

/// Durable Story Map semantics: optional native board identity, target, and
/// source-ordered lanes. Height, width, zoom, scroll, and interaction state do
/// not belong here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMap {
    pub foreign_id: Option<String>,
    pub target_pages: u32,
    pub lanes: Vec<StoryMapLane>,
}

impl Default for StoryMap {
    fn default() -> Self {
        Self {
            foreign_id: None,
            target_pages: DEFAULT_STORY_MAP_TARGET_PAGES,
            lanes: Vec::new(),
        }
    }
}

impl StoryMap {
    /// Converts the bounded page target to exact placement units.
    pub const fn target_eighths(&self) -> Option<u64> {
        (self.target_pages as u64).checked_mul(STORY_MAP_EIGHTHS_PER_PAGE)
    }
}

/// Exact one-card PageGoal placement, represented without floating-point state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMapPlacement {
    pub lane_uuid: String,
    pub start_eighths: u64,
    pub length_eighths: u64,
}

impl StoryMapPlacement {
    /// Returns the half-open end in eighths, or `None` on integer overflow.
    pub const fn checked_end_eighths(&self) -> Option<u64> {
        self.start_eighths.checked_add(self.length_eighths)
    }
}
