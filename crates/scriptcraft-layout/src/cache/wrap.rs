use crate::{
    geometry::DisplaySourceSegment,
    inputs::{EffectiveGeometryMode, EffectiveParagraph, LayoutPurpose},
    wrap::{self as layout_wrap, WrappedLine},
    LayoutBuildDiagnostics,
};
use scriptcraft_model::formatting::{EffectiveTextStyle, FixedPoint};
use scriptcraft_model::{Role, StableId, TextRun};
use std::{
    collections::{BTreeSet, HashMap},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Instant,
};

const MAX_CACHED_ELEMENT_WRAPS: usize = 4_096;
const MAX_CACHED_ELEMENT_WRAP_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ElementWrapContext {
    element_id: StableId,
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
    element_id: &'a StableId,
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
pub(super) struct ElementWrapCache {
    buckets: HashMap<u64, Vec<ElementWrapEntry>>,
    /// (last-used generation, unique insertion order, collision-bucket key).
    usage_order: BTreeSet<(u64, u64, u64)>,
    generation: u64,
    next_order: u64,
    entry_count: usize,
    retained_bytes: usize,
}

impl ElementWrapCache {
    pub(super) fn start_generation(&mut self) {
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

    pub(super) fn clear(&mut self) {
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
    pub(super) fn wrap(
        &mut self,
        element_id: &StableId,
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
        let lines = layout_wrap::wrap_element(
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

    pub(super) fn evict_if_needed(&mut self) {
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
