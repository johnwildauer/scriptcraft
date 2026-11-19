use super::{add_elapsed_nanos, ElementWrapCache};
use crate::{
    geometry::{DisplaySourceSegment, LayoutLine, PositionedStyledSpan, ScreenplayLayout},
    inputs::{EffectiveGeometryMode, EffectiveParagraph, LayoutComposition, LayoutPurpose},
    wrap, LayoutBuildDiagnostics,
};
use scriptcraft_model::formatting::{EffectiveTextStyle, FixedPoint};
use scriptcraft_model::typography::EffectiveTextColor;
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, RunAttribute, ScreenplayElement, StableId, TextRun,
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
    pub(crate) text_style: EffectiveTextStyle,
    pub(crate) char_width: FixedPoint,
    pub(crate) element_key: Option<ElementTypeKey>,
    pub(crate) layout_environment: Option<Arc<crate::inputs::LayoutEnvironment>>,
    pub(crate) effective_color: EffectiveTextColor,
}

impl Hash for PreparedBodyEnvironment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.purpose.hash(state);
        self.composition.hash(state);
        self.geometry_mode.hash(state);
        self.paragraph.hash(state);
        self.text_style.hash(state);
        self.char_width.hash(state);
        self.element_key.hash(state);
        self.layout_environment
            .as_deref()
            .map(crate::inputs::LayoutEnvironment::cache_fingerprint)
            .hash(state);
        self.effective_color.hash(state);
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
    /// Latest retained entry for each stable-element/environment identity.
    /// Values are (collision-bucket fingerprint, unique insertion order).
    previous_by_identity: HashMap<u64, (u64, u64)>,
    usage_order: BTreeSet<(u64, u64, u64)>,
    generation: u64,
    next_order: u64,
    entry_count: usize,
    retained_bytes: usize,
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

    fn identity_fingerprint(owner: &Arc<BodyOwner>, environment: &PreparedBodyEnvironment) -> u64 {
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
        .saturating_add(text_style_heap_bytes(&environment.text_style))
        .saturating_add(
            environment
                .element_key
                .as_ref()
                .map_or(0, |key| key.as_str().len()),
        )
        .saturating_add(
            environment
                .layout_environment
                .as_deref()
                .map_or(0, layout_environment_heap_bytes),
        );
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

pub(crate) fn prepare_body_for_layout(
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
    // Phase 5 replaces this static-layout adapter with definition-driven layout.
    let role = element.transitional_role();
    let prepared = if let Some(prepared) = incremental {
        prepared
    } else {
        let lines = wrap::wrap_for_layout(
            wrap_cache,
            diagnostics.as_deref_mut(),
            &element.id,
            &element.text,
            0,
            role,
            environment.element_key.as_ref(),
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
    old: &ScreenplayElement,
    new: &ScreenplayElement,
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
    // Phase 5 replaces this static-role projection with definition-driven layout.
    let new_role = new.transitional_role();
    if old.id != new.id
        || old.element_type() != new.element_type()
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
    let suffix = wrap::wrap_for_layout_range(
        None,
        diagnostics,
        &new.id,
        &new.text,
        suffix_start..new.text.len(),
        false,
        reused_lines,
        0,
        new_role,
        Some(&new.element_type()),
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

#[cfg(test)]
mod typography_cache_identity_tests {
    use super::*;
    use crate::inputs::EffectiveGeometryMode;
    use scriptcraft_model::formatting::{FontFallback, ParagraphAlignment};
    use scriptcraft_model::typography::parse_effective_text_color;
    use scriptcraft_model::{BodyOwner, Project};

    fn paragraph() -> EffectiveParagraph {
        EffectiveParagraph {
            left: FixedPoint::default(),
            right: FixedPoint::from_points(100),
            first_indent: FixedPoint::default(),
            alignment: ParagraphAlignment::Left,
            space_before: FixedPoint::default(),
            line_advance: FixedPoint::from_points(12),
            char_width: FixedPoint::from_points(7),
            starts_new_page: false,
            pagination: Default::default(),
        }
    }

    fn environment(color: Option<&str>) -> PreparedBodyEnvironment {
        PreparedBodyEnvironment {
            purpose: LayoutPurpose::Editor,
            composition: LayoutComposition::BodyEditor,
            geometry_mode: EffectiveGeometryMode::Sequential,
            paragraph: paragraph(),
            text_style: EffectiveTextStyle {
                declared_family: "Courier Prime".to_owned(),
                family: FontFallback::CourierPrime,
                size: 12,
                bold: false,
                italic: false,
                underline: false,
                all_caps: false,
                hidden_text: false,
                source_color: color.map(str::to_owned),
            },
            char_width: FixedPoint::from_points(7),
            element_key: None,
            layout_environment: None,
            effective_color: parse_effective_text_color(color),
        }
    }

    fn test_owner() -> Result<Arc<BodyOwner>, scriptcraft_model::ModelError> {
        let project = Project::new()?;
        project
            .elements
            .first()
            .cloned()
            .ok_or(scriptcraft_model::ModelError::InvalidNumber)
    }

    #[test]
    fn prepared_lookup_rejects_unequal_identity_in_a_shared_candidate_bucket(
    ) -> Result<(), scriptcraft_model::ModelError> {
        let owner = test_owner()?;
        let requested = environment(None);
        let retained = environment(Some("unsupported-explicit-color"));
        let fingerprint = PreparedBodyCache::fingerprint(&owner, &requested);
        let prepared = Arc::new(PreparedBodyParagraph {
            paragraph: requested.paragraph,
            lines: Arc::from([]),
        });
        let mut cache = PreparedBodyCache::default();
        cache.buckets.insert(
            fingerprint,
            vec![PreparedBodyEntry {
                owner: Arc::clone(&owner),
                owner_revision: owner.revision(),
                environment: retained,
                prepared,
                last_used_generation: 0,
                insertion_order: 1,
                byte_cost: 1,
            }],
        );
        cache.entry_count = 1;
        assert!(cache.lookup(&owner, &requested).is_none());
        Ok(())
    }

    #[test]
    fn incremental_preparation_matches_a_fresh_owner_preparation(
    ) -> Result<(), scriptcraft_model::ModelError> {
        let project = Project::new()?;
        let layout = ScreenplayLayout::build(&project)
            .map_err(|_| scriptcraft_model::ModelError::InvalidNumber)?;
        let mut old_owner = project
            .elements
            .first()
            .cloned()
            .ok_or(scriptcraft_model::ModelError::InvalidNumber)?;
        BodyOwner::update(&mut old_owner, |element| {
            element.text =
                "A sufficiently long screenplay action line that wraps over several lines. "
                    .repeat(4);
        });
        let mut new_owner = Arc::clone(&old_owner);
        BodyOwner::update(&mut new_owner, |element| {
            element
                .text
                .push_str("An appended sentence exercises suffix rewrapping.");
        });
        let mut prepared_environment = environment(None);
        prepared_environment.element_key = Some(new_owner.element_type());
        let previous = prepare_body_for_layout(
            None,
            None,
            None,
            &layout,
            &old_owner,
            prepared_environment.clone(),
            None,
        );
        let incremental = prepare_incremental_body_for_layout(
            None,
            &layout,
            &new_owner,
            &old_owner,
            &previous,
            &prepared_environment,
        )
        .ok_or(scriptcraft_model::ModelError::InvalidNumber)?;
        let fresh = prepare_body_for_layout(
            None,
            None,
            None,
            &layout,
            &new_owner,
            prepared_environment,
            None,
        );
        assert_eq!(incremental.paragraph, fresh.paragraph);
        assert_eq!(incremental.lines.as_ref(), fresh.lines.as_ref());
        Ok(())
    }

    #[test]
    fn prepared_cache_eviction_preserves_entry_and_byte_bounds(
    ) -> Result<(), scriptcraft_model::ModelError> {
        let owner = test_owner()?;
        let environment = environment(None);
        let prepared = Arc::new(PreparedBodyParagraph {
            paragraph: environment.paragraph,
            lines: Arc::from([]),
        });
        let mut cache = PreparedBodyCache::default();
        for _ in 0..=MAX_CACHED_PREPARED_BODY_PARAGRAPHS {
            cache.insert(&owner, environment.clone(), Arc::clone(&prepared));
        }
        cache.evict_if_needed();
        assert!(cache.entry_count <= MAX_CACHED_PREPARED_BODY_PARAGRAPHS);
        assert!(cache.retained_bytes <= MAX_CACHED_PREPARED_BODY_BYTES);
        Ok(())
    }
}
