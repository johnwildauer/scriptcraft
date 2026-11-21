use crate::inputs::LayoutEnvironment;
use crate::{
    geometry::DisplaySourceSegment,
    inputs::{EffectiveGeometryMode, EffectiveParagraph, LayoutPurpose},
    wrap::{self as layout_wrap, WrappedLine},
    LayoutBuildDiagnostics,
};
use scriptcraft_model::typography::EffectiveTextStyle as ResolvedTextStyle;
use scriptcraft_model::{ElementTypeKey, StableId, TextRun};
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
    element_key: Option<ElementTypeKey>,
    element_type: Option<ElementTypeKey>,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    resolved_text_style: ResolvedTextStyle,
    runs: Arc<[TextRun]>,
    atomic_ranges: Arc<[std::ops::Range<usize>]>,
}

#[derive(Clone)]
struct ElementWrapIdentity<'a> {
    element_id: &'a StableId,
    element_key: Option<&'a ElementTypeKey>,
    element_type: Option<ElementTypeKey>,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    resolved_text_style: &'a ResolvedTextStyle,
    runs: &'a [TextRun],
    atomic_ranges: &'a [std::ops::Range<usize>],
}

impl ElementWrapContext {
    fn from_identity(identity: ElementWrapIdentity<'_>) -> Self {
        Self {
            element_id: identity.element_id.clone(),
            element_key: identity.element_key.cloned(),
            element_type: identity.element_type,
            purpose: identity.purpose,
            title_page: identity.title_page,
            geometry_mode: identity.geometry_mode,
            paragraph: identity.paragraph,
            resolved_text_style: identity.resolved_text_style.clone(),
            runs: Arc::from(identity.runs),
            atomic_ranges: Arc::from(identity.atomic_ranges),
        }
    }

    fn matches(&self, identity: &ElementWrapIdentity<'_>) -> bool {
        &self.element_id == identity.element_id
            && self.element_key.as_ref() == identity.element_key
            && self.element_type == identity.element_type
            && self.purpose == identity.purpose
            && self.title_page == identity.title_page
            && self.geometry_mode == identity.geometry_mode
            && self.paragraph == identity.paragraph
            && &self.resolved_text_style == identity.resolved_text_style
            && self.runs.as_ref() == identity.runs
            && self.atomic_ranges.as_ref() == identity.atomic_ranges
    }

    fn heap_bytes(&self) -> usize {
        let mut total = std::mem::size_of::<Self>()
            .saturating_add(self.element_id.as_str().len())
            .saturating_add(
                self.element_key
                    .as_ref()
                    .map_or(0, |key| key.as_str().len()),
            )
            .saturating_add(self.resolved_text_style.declared_family.len())
            .saturating_add(self.resolved_text_style.source_color.as_ref().map_or(0, String::len))
            .saturating_add(self.resolved_text_style.resolved_font_id.as_str().len())
            .saturating_add(
                self.atomic_ranges
                    .len()
                    .saturating_mul(std::mem::size_of::<std::ops::Range<usize>>()),
            );
        for run in self.runs.iter() {
            total = total
                .saturating_add(std::mem::size_of::<TextRun>())
                .saturating_add(run.text.len());
            for attribute in &run.attributes {
                total = total
                    .saturating_add(std::mem::size_of::<scriptcraft_model::RunAttribute>())
                    .saturating_add(attribute.name.len())
                    .saturating_add(attribute.value.len());
            }
        }
        total
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
    layout_environment: Option<Arc<LayoutEnvironment>>,
    environment_byte_cost: usize,
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
        self.layout_environment = None;
        self.environment_byte_cost = 0;
    }

    pub(super) fn set_layout_environment(&mut self, environment: Option<Arc<LayoutEnvironment>>) {
        if self.layout_environment.as_deref() != environment.as_deref() {
            self.clear();
            self.environment_byte_cost = environment
                .as_deref()
                .map_or(0, layout_environment_byte_cost);
            self.retained_bytes = self.environment_byte_cost;
            self.layout_environment = environment;
        }
    }

    fn fingerprint(&self, identity: &ElementWrapIdentity<'_>, text: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        identity.element_id.hash(&mut hasher);
        identity.element_key.hash(&mut hasher);
        identity.element_type.hash(&mut hasher);
        identity.purpose.hash(&mut hasher);
        identity.title_page.hash(&mut hasher);
        identity.geometry_mode.hash(&mut hasher);
        identity.paragraph.hash(&mut hasher);
        identity.resolved_text_style.hash(&mut hasher);
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn wrap_resolved(
        &mut self,
        element_id: &StableId,
        element_key: Option<&ElementTypeKey>,
        text: &str,
        element: usize,
        element_type: Option<ElementTypeKey>,
        paragraph: EffectiveParagraph,
        resolved_text_style: ResolvedTextStyle,
        runs: &[TextRun],
        purpose: LayoutPurpose,
        title_page: bool,
        geometry_mode: EffectiveGeometryMode,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Result<Vec<WrappedLine>, crate::geometry::LayoutError> {
        let identity = ElementWrapIdentity {
            element_id,
            element_key,
            element_type: element_type.clone(),
            purpose,
            title_page,
            geometry_mode,
            paragraph,
            resolved_text_style: &resolved_text_style,
            runs,
            atomic_ranges: &[],
        };
        let fingerprint = self.fingerprint(&identity, text);
        if let Some(entry) = self.buckets.get_mut(&fingerprint).and_then(|bucket| {
            bucket.iter_mut().find(|entry| {
                entry.context.matches(&identity) && entry.canonical_text.as_ref() == text
            })
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
            return Ok(entry
                .lines
                .iter()
                .cloned()
                .map(|mut line| {
                    line.element = element;
                    line
                })
                .collect());
        }
        let environment =
            self.layout_environment
                .as_deref()
                .ok_or(crate::geometry::LayoutError {
                    kind: crate::geometry::LayoutErrorKind::InvalidProfile,
                })?;
        let started = diagnostics.as_ref().map(|_| Instant::now());
        let context = ElementWrapContext::from_identity(identity);
        let lines = layout_wrap::wrap_resolved_for_layout(
            text,
            element,
            element_type,
            paragraph,
            &resolved_text_style,
            &environment.fonts,
            runs,
            true,
            0,
            0,
        )?;
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
        let byte_cost = context
            .heap_bytes()
            .saturating_add(text.len())
            .saturating_add(cached_lines.iter().fold(0usize, |total, line| {
                total
                    .saturating_add(line.text.len())
                    .saturating_add(line.display_text.len())
                    .saturating_add(
                        line.display_source_map
                            .len()
                            .saturating_mul(std::mem::size_of::<DisplaySourceSegment>()),
                    )
                    .saturating_add(line.resolved_scalars.len().saturating_mul(
                        std::mem::size_of::<layout_wrap::ResolvedDisplayScalar>(),
                    ))
            }));
        if byte_cost > MAX_CACHED_ELEMENT_WRAP_BYTES {
            return Ok(lines);
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
        Ok(lines)
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
                    (removed.map_or(0, |entry| entry.byte_cost), remove_bucket)
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

fn layout_environment_byte_cost(environment: &LayoutEnvironment) -> usize {
    let fonts = &environment.fonts;
    let request_bytes = fonts
        .requests()
        .iter()
        .fold(0usize, |total, (family, request)| {
            total
                .saturating_add(family.len())
                .saturating_add(request.regular.as_str().len())
                .saturating_add(request.bold.as_str().len())
                .saturating_add(request.italic.as_str().len())
                .saturating_add(request.bold_italic.as_str().len())
        });
    fonts.faces().values().fold(
        std::mem::size_of::<LayoutEnvironment>()
            .saturating_add(request_bytes)
            .saturating_add(
                environment
                    .neutral_title_style
                    .source_color
                    .as_ref()
                    .map_or(0, String::len),
            ),
        |total, face| {
            total
                .saturating_add(face.id.as_str().len())
                .saturating_add(face.family_label.len())
                .saturating_add(face.face_label.len())
                .saturating_add(face.metrics.scalar_advances.len().saturating_mul(
                    std::mem::size_of::<(char, scriptcraft_model::typography::ScalarAdvance)>(),
                ))
        },
    )
}
