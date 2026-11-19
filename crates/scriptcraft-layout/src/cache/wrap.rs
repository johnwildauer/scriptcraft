use crate::inputs::LayoutEnvironment;
use crate::{
    geometry::DisplaySourceSegment,
    inputs::{EffectiveGeometryMode, EffectiveParagraph, LayoutPurpose},
    wrap::{self as layout_wrap, WrappedLine},
    LayoutBuildDiagnostics,
};
use scriptcraft_model::formatting::{EffectiveTextStyle, FixedPoint};
use scriptcraft_model::typography::{
    parse_effective_text_color, EffectiveTextColor, EffectiveTextStyle as ResolvedTextStyle,
};
use scriptcraft_model::{ElementTypeKey, Role, RunAttribute, StableId, TextRun};
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
    role: Role,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    resolved_text_style: Option<ResolvedTextStyle>,
    effective_color: EffectiveTextColor,
    char_width: FixedPoint,
    runs: Arc<[TextRun]>,
    atomic_ranges: Arc<[std::ops::Range<usize>]>,
}

#[derive(Clone, Copy)]
struct ElementWrapIdentity<'a> {
    element_id: &'a StableId,
    element_key: Option<&'a ElementTypeKey>,
    role: Role,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
    paragraph: EffectiveParagraph,
    text_style: &'a EffectiveTextStyle,
    resolved_text_style: Option<&'a ResolvedTextStyle>,
    char_width: FixedPoint,
    runs: &'a [TextRun],
    atomic_ranges: &'a [std::ops::Range<usize>],
}

impl ElementWrapContext {
    fn from_identity(identity: ElementWrapIdentity<'_>) -> Self {
        Self {
            element_id: identity.element_id.clone(),
            element_key: identity.element_key.cloned(),
            role: identity.role,
            purpose: identity.purpose,
            title_page: identity.title_page,
            geometry_mode: identity.geometry_mode,
            paragraph: identity.paragraph,
            text_style: identity.text_style.clone(),
            resolved_text_style: identity.resolved_text_style.cloned(),
            effective_color: parse_effective_text_color(
                identity.text_style.source_color.as_deref(),
            ),
            char_width: identity.char_width,
            runs: Arc::from(identity.runs),
            atomic_ranges: Arc::from(identity.atomic_ranges),
        }
    }

    fn matches(&self, identity: ElementWrapIdentity<'_>) -> bool {
        &self.element_id == identity.element_id
            && self.element_key.as_ref() == identity.element_key
            && self.role == identity.role
            && self.purpose == identity.purpose
            && self.title_page == identity.title_page
            && self.geometry_mode == identity.geometry_mode
            && self.paragraph == identity.paragraph
            && &self.text_style == identity.text_style
            && self.resolved_text_style.as_ref() == identity.resolved_text_style
            && self.effective_color
                == parse_effective_text_color(identity.text_style.source_color.as_deref())
            && self.char_width == identity.char_width
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
            .saturating_add(self.text_style.declared_family.len())
            .saturating_add(self.text_style.source_color.as_ref().map_or(0, String::len))
            .saturating_add(self.resolved_text_style.as_ref().map_or(0, |style| {
                style
                    .declared_family
                    .len()
                    .saturating_add(style.source_color.as_ref().map_or(0, String::len))
                    .saturating_add(style.resolved_font_id.as_str().len())
            }))
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
        identity.role.hash(&mut hasher);
        identity.purpose.hash(&mut hasher);
        identity.title_page.hash(&mut hasher);
        identity.geometry_mode.hash(&mut hasher);
        identity.paragraph.hash(&mut hasher);
        identity.text_style.hash(&mut hasher);
        identity.resolved_text_style.hash(&mut hasher);
        parse_effective_text_color(identity.text_style.source_color.as_deref()).hash(&mut hasher);
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
        element_key: Option<&ElementTypeKey>,
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
            element_key,
            role,
            purpose,
            title_page,
            geometry_mode,
            paragraph,
            text_style: &text_style,
            resolved_text_style: None,
            char_width,
            runs,
            atomic_ranges,
        };
        let fingerprint = self.fingerprint(&identity, text);
        if let Some(entry) = self.buckets.get_mut(&fingerprint).and_then(|bucket| {
            bucket.iter_mut().find(|entry| {
                entry.context.matches(identity) && entry.canonical_text.as_ref() == text
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
        let byte_cost = context
            .heap_bytes()
            .saturating_add(text.len())
            .saturating_add(cached_lines.iter().fold(0usize, |total, line| {
                total
                    .saturating_add(line.text.len())
                    .saturating_add(line.display_text.len())
                    .saturating_add(line.text_style.declared_family.len())
                    .saturating_add(line.text_style.source_color.as_ref().map_or(0, String::len))
                    .saturating_add(line.resolved_text_style.as_ref().map_or(0, |style| {
                        style
                            .declared_family
                            .len()
                            .saturating_add(style.source_color.as_ref().map_or(0, String::len))
                            .saturating_add(style.resolved_font_id.as_str().len())
                    }))
                    .saturating_add(line.resolved_scalars.iter().fold(0usize, |bytes, scalar| {
                        bytes
                            .saturating_add(
                                std::mem::size_of::<layout_wrap::ResolvedDisplayScalar>(),
                            )
                            .saturating_add(scalar.text_style.declared_family.len())
                            .saturating_add(
                                scalar
                                    .text_style
                                    .source_color
                                    .as_ref()
                                    .map_or(0, String::len),
                            )
                            .saturating_add(scalar.text_style.resolved_font_id.as_str().len())
                    }))
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn wrap_resolved(
        &mut self,
        element_id: &StableId,
        element_key: Option<&ElementTypeKey>,
        text: &str,
        element: usize,
        role: Role,
        paragraph: EffectiveParagraph,
        text_style: EffectiveTextStyle,
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
            role,
            purpose,
            title_page,
            geometry_mode,
            paragraph,
            text_style: &text_style,
            resolved_text_style: Some(&resolved_text_style),
            char_width: FixedPoint::default(),
            runs,
            atomic_ranges: &[],
        };
        let fingerprint = self.fingerprint(&identity, text);
        if let Some(entry) = self.buckets.get_mut(&fingerprint).and_then(|bucket| {
            bucket.iter_mut().find(|entry| {
                entry.context.matches(identity) && entry.canonical_text.as_ref() == text
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
            role,
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
        let byte_cost =
            context
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

#[cfg(test)]
mod identity_tests {
    use super::*;
    use scriptcraft_model::formatting::{FontFallback, ParagraphAlignment};

    fn paragraph() -> EffectiveParagraph {
        EffectiveParagraph {
            left: FixedPoint::default(),
            right: FixedPoint::from_points(500),
            first_indent: FixedPoint::default(),
            alignment: ParagraphAlignment::Left,
            space_before: FixedPoint::default(),
            line_advance: FixedPoint::from_points(12),
            char_width: FixedPoint::from_points(7),
            starts_new_page: false,
            pagination: Default::default(),
        }
    }

    fn style(color: Option<&str>) -> EffectiveTextStyle {
        EffectiveTextStyle {
            declared_family: "Courier Prime".to_owned(),
            family: FontFallback::CourierPrime,
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            source_color: color.map(str::to_owned),
        }
    }

    #[test]
    fn open_key_and_effective_color_remain_full_equality_after_fingerprint_lookup(
    ) -> Result<(), scriptcraft_model::ModelError> {
        let element_id = StableId::new("owner")?;
        let action = ElementTypeKey::new("Action")?;
        let custom = ElementTypeKey::new("Custom Action")?;
        let base_style = style(None);
        let runs: [TextRun; 0] = [];
        let atomic: [std::ops::Range<usize>; 0] = [];
        let original_identity = ElementWrapIdentity {
            element_id: &element_id,
            element_key: Some(&action),
            role: Role::Action,
            purpose: LayoutPurpose::Editor,
            title_page: false,
            geometry_mode: EffectiveGeometryMode::Sequential,
            paragraph: paragraph(),
            text_style: &base_style,
            resolved_text_style: None,
            char_width: FixedPoint::from_points(7),
            runs: &runs,
            atomic_ranges: &atomic,
        };
        let retained = ElementWrapContext::from_identity(original_identity);
        let different_key = ElementWrapIdentity {
            element_key: Some(&custom),
            ..original_identity
        };
        assert!(!retained.matches(different_key));
        let explicit_color = style(Some("unsupported-explicit-color"));
        let different_color = ElementWrapIdentity {
            text_style: &explicit_color,
            ..original_identity
        };
        assert!(!retained.matches(different_color));
        let custom_color = style(Some("#87878F8FE0E0"));
        let different_explicit_rgb = ElementWrapIdentity {
            text_style: &custom_color,
            ..original_identity
        };
        assert!(!retained.matches(different_explicit_rgb));
        let expanded_runs = [TextRun {
            text: "x".to_owned(),
            attributes: Vec::new(),
        }];
        let different_run_repertoire = ElementWrapIdentity {
            runs: &expanded_runs,
            ..original_identity
        };
        assert!(!retained.matches(different_run_repertoire));
        let attributed_runs = [TextRun {
            text: "x".to_owned(),
            attributes: vec![RunAttribute {
                name: "Bold".to_owned(),
                value: "Yes".to_owned(),
            }],
        }];
        let different_run_attributes = ElementWrapIdentity {
            runs: &attributed_runs,
            ..original_identity
        };
        assert!(!retained.matches(different_run_attributes));

        // Force two unequal identities into the same candidate bucket so the
        // lookup path proves that a fingerprint is not accepted as equality.
        let mut cache = ElementWrapCache::default();
        let mut diagnostics = LayoutBuildDiagnostics::default();
        let original_fingerprint = cache.fingerprint(&original_identity, "x");
        let _ = cache.wrap(
            &element_id,
            Some(&action),
            "x",
            0,
            Role::Action,
            original_identity.paragraph,
            base_style.clone(),
            original_identity.char_width,
            &runs,
            &atomic,
            &atomic,
            LayoutPurpose::Editor,
            false,
            EffectiveGeometryMode::Sequential,
            Some(&mut diagnostics),
        );
        let different_fingerprint = cache.fingerprint(&different_key, "x");
        let mut retained_bucket = cache
            .buckets
            .remove(&original_fingerprint)
            .expect("the original wrap should be retained");
        let retained_entry = retained_bucket
            .pop()
            .expect("the original wrap bucket should contain an entry");
        cache
            .buckets
            .entry(different_fingerprint)
            .or_default()
            .push(retained_entry);
        let _ = cache.wrap(
            &element_id,
            Some(&custom),
            "x",
            0,
            Role::Action,
            different_key.paragraph,
            base_style.clone(),
            different_key.char_width,
            &runs,
            &atomic,
            &atomic,
            LayoutPurpose::Editor,
            false,
            EffectiveGeometryMode::Sequential,
            Some(&mut diagnostics),
        );
        assert_eq!(diagnostics.element_wraps_reused, 0);
        Ok(())
    }
}
