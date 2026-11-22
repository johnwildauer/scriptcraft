use super::{add_elapsed_nanos, ElementWrapCache};
use crate::{
    geometry::{DisplaySourceSegment, LayoutLine, PositionedStyledSpan, ScreenplayLayout},
    inputs::{EffectiveGeometryMode, EffectiveParagraph, LayoutComposition, LayoutPurpose},
    wrap, LayoutBuildDiagnostics,
};
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::typography::EffectiveTextStyle as ResolvedTextStyle;
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, RunAttribute, TextRun,
};
use std::{
    collections::{BTreeSet, HashMap},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Instant,
};

const MAX_CACHED_PREPARED_BODY_PARAGRAPHS: usize = 4_096;
const MAX_CACHED_PREPARED_BODY_BYTES: usize = 64 * 1024 * 1024;

/// Every owner-local input consumed by wrapping, source/display preparation,
/// positioned spans, and line metrics. The full environment is retained and
/// compared, while its bounded content fingerprint is only a lookup candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedBodyEnvironment {
    pub(crate) purpose: LayoutPurpose,
    pub(crate) composition: LayoutComposition,
    pub(crate) geometry_mode: EffectiveGeometryMode,
    pub(crate) paragraph: EffectiveParagraph,
    pub(crate) resolved_text_style: ResolvedTextStyle,
    pub(crate) element_key: ElementTypeKey,
    pub(crate) layout_environment: Arc<crate::inputs::LayoutEnvironment>,
}

impl Hash for PreparedBodyEnvironment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.purpose.hash(state);
        self.composition.hash(state);
        self.geometry_mode.hash(state);
        self.paragraph.hash(state);
        self.resolved_text_style.hash(state);
        self.element_key.hash(state);
        self.layout_environment
            .cache_fingerprint()
            .hash(state);
    }
}

/// Complete immutable owner-local Body output. Lines have owner-local element
/// zero, page zero, and zero vertical placement; only placement rebases them.
#[derive(Debug)]
pub(crate) struct PreparedBodyParagraph {
    pub(crate) paragraph: EffectiveParagraph,
    pub(crate) lines: Arc<[LayoutLine]>,
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
pub(crate) struct PreparedBodyCache {
    buckets: HashMap<u64, Vec<PreparedBodyEntry>>,
    usage_order: BTreeSet<(u64, u64, u64)>,
    generation: u64,
    next_order: u64,
    entry_count: usize,
    retained_bytes: usize,
    layout_environment: Option<Arc<crate::inputs::LayoutEnvironment>>,
    environment_byte_cost: usize,
}

impl PreparedBodyCache {
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

    /// The entries share one immutable environment Arc. Charge its heap payload
    /// once per cache, not once for every owner-local prepared paragraph.
    pub(super) fn set_layout_environment(
        &mut self,
        environment: Option<Arc<crate::inputs::LayoutEnvironment>>,
    ) {
        if self.layout_environment.as_deref() != environment.as_deref() {
            self.clear();
            self.environment_byte_cost = environment
                .as_deref()
                .map_or(0, layout_environment_heap_bytes);
            self.retained_bytes = self.environment_byte_cost;
            self.layout_environment = environment;
        }
    }

    fn fingerprint(owner: &Arc<BodyOwner>, environment: &PreparedBodyEnvironment) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (Arc::as_ptr(owner) as usize).hash(&mut hasher);
        owner.revision().hash(&mut hasher);
        environment.hash(&mut hasher);
        hasher.finish()
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
        self.entry_count = self.entry_count.saturating_add(1);
        self.retained_bytes = self.retained_bytes.saturating_add(byte_cost);
    }

    pub(super) fn evict_if_needed(&mut self) {
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
            let (removed, remove_bucket) = if let Some(bucket) = self.buckets.get_mut(&fingerprint)
            {
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
            self.entry_count = self.entry_count.saturating_sub(1);
            self.retained_bytes = self.retained_bytes.saturating_sub(removed_bytes);
        }
    }
}

fn resolved_text_style_heap_bytes(style: &ResolvedTextStyle) -> usize {
    style
        .declared_family
        .len()
        .saturating_add(style.source_color.as_ref().map_or(0, String::len))
        .saturating_add(style.resolved_font_id.as_str().len())
}

fn layout_environment_heap_bytes(environment: &crate::inputs::LayoutEnvironment) -> usize {
    let fonts = &environment.fonts;
    let requests = fonts
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
        std::mem::size_of::<crate::inputs::LayoutEnvironment>()
            .saturating_add(requests)
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
        .saturating_add(resolved_text_style_heap_bytes(&environment.resolved_text_style))
        .saturating_add(environment.element_key.as_str().len());
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
            );
        for span in line.spans.iter() {
            total = total
                .saturating_add(span.display_text.len())
                .saturating_add(
                    span.display_source_map
                        .len()
                        .saturating_mul(std::mem::size_of::<DisplaySourceSegment>()),
                )
                .saturating_add(resolved_text_style_heap_bytes(&span.resolved_text_style))
                .saturating_add(
                    span.display_boundary_offsets
                        .len()
                        .saturating_mul(std::mem::size_of::<FixedPoint>()),
                );
        }
    }
    total
}

pub(crate) fn prepare_body_for_layout(
    mut prepared_cache: Option<&mut PreparedBodyCache>,
    mut wrap_cache: Option<&mut ElementWrapCache>,
    mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
    layout: &ScreenplayLayout,
    owner: &Arc<BodyOwner>,
    environment: PreparedBodyEnvironment,
    environment_started: Option<Instant>,
) -> Result<Arc<PreparedBodyParagraph>, crate::geometry::LayoutError> {
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
        return Ok(prepared);
    }
    let miss_started = (cache_attempted && diagnostics.is_some()).then(Instant::now);
    let element = owner.as_element();
    let paragraph = environment.paragraph;
    let element_type = Some(element.element_type());
    let lines = if let Some(cache) = wrap_cache.as_deref_mut() {
        cache.wrap_resolved(
            &element.id,
            Some(&environment.element_key),
            &element.text,
            0,
            element_type.clone(),
            paragraph,
            environment.resolved_text_style.clone(),
            &element.runs,
            environment.purpose,
            false,
            environment.geometry_mode,
            &environment.layout_environment,
            diagnostics.as_deref_mut(),
        )?
    } else {
        wrap::wrap_resolved_for_layout(
            &element.text,
            0,
            element_type,
            paragraph,
            &environment.resolved_text_style,
            &environment.layout_environment,
            &element.runs,
            true,
            0,
            0,
        )?
    };
    let prepared = Arc::new(PreparedBodyParagraph {
        paragraph,
        lines: lines
            .into_iter()
            .map(|line| layout.make_line(0, line, false, FixedPoint::default()))
            .collect::<Vec<_>>()
            .into(),
    });
    if let Some(cache) = prepared_cache {
        cache.insert(owner, environment, Arc::clone(&prepared));
    }
    if let Some(diagnostics) = diagnostics.as_deref_mut() {
        add_elapsed_nanos(&mut diagnostics.prepared_miss_nanos, miss_started);
    }
    Ok(prepared)
}
