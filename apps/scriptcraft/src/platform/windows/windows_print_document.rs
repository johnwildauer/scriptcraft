//! Immutable vector-page package derived from the canonical Print layout.
//!
//! The page model deliberately contains no editor/egui state and keeps geometry
//! in the core's exact hundredths-of-a-point units. Each visible run copies the
//! placed line-box-bottom baseline, effective leading, natural table extents,
//! and canonical scalar boundaries; only the DirectWrite adapter measures its
//! own line baseline to translate drawing onto this captured baseline. Include
//! and omit title packages are fresh Print compositions from the same immutable
//! request inputs, never from editor caches or printer dimensions.

use scriptcraft_layout::{
    FixedRect, LayoutComposition, LayoutEnvironment, LayoutError, ScreenplayLayout,
};
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::typography::{
    EffectiveTextColor, FontResolutionKind, ResolvedFontSourceKind, MAX_SUPPORTED_FONT_SIZE,
};
use scriptcraft_model::{LayoutInputs, Project};
use std::sync::Arc;

const MAX_PACKAGE_PAGES: usize = 512;
const MAX_PACKAGE_RUNS: usize = 262_144;
const MAX_PACKAGE_TEXT_BYTES: usize = 16 * 1024 * 1024;

const MAX_REPORT_COUNT: u32 = 1_000_000;

/// Fixed-size, value-free summary of Windows' deliberate host-font limitations.
/// Counts are capped so the report remains bounded independently of document size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct WindowsPrintTypographyReport {
    courier_new_substitutions: u32,
    nearest_family_substitutions: u32,
    face_identity_unverified_runs: u32,
    metric_policy_unverified_runs: u32,
    glyph_coverage_unverified_scalars: u32,
    resolved_face_missing_glyph_scalars: u32,
    natural_extent_exceeds_line_box_lines: u32,
    unsupported_color_fallbacks: u32,
    suppressed_events: u32,
}

impl WindowsPrintTypographyReport {
    fn add(counter: &mut u32, amount: usize, suppressed: &mut u32) {
        let amount = u32::try_from(amount).unwrap_or(u32::MAX);
        let available = MAX_REPORT_COUNT.saturating_sub(*counter);
        let recorded = amount.min(available);
        *counter = counter.saturating_add(recorded);
        *suppressed = suppressed.saturating_add(amount.saturating_sub(recorded));
    }

    pub(super) fn summary(self) -> Option<String> {
        if self == Self::default() {
            return None;
        }
        Some(format!(
            "Windows typography report: Courier New substitutions {}; nearest-family substitutions {}; DirectWrite face identity unverified for {} runs; DirectWrite metric/shaping policy unverified for {} runs; glyph coverage unverified for {} scalars; resolved-face missing glyph markers {}; natural font-table extent exceeds line box for {} lines; unsupported colors rendered black {}; report events capped {}",
            self.courier_new_substitutions,
            self.nearest_family_substitutions,
            self.face_identity_unverified_runs,
            self.metric_policy_unverified_runs,
            self.glyph_coverage_unverified_scalars,
            self.resolved_face_missing_glyph_scalars,
            self.natural_extent_exceeds_line_box_lines,
            self.unsupported_color_fallbacks,
            self.suppressed_events,
        ))
    }

    fn record_run(
        &mut self,
        courier_new_substitution: bool,
        nearest_family_substitution: bool,
        scalar_count: usize,
        missing_glyph_count: usize,
        unsupported_color: bool,
    ) {
        let suppressed = &mut self.suppressed_events;
        if courier_new_substitution {
            Self::add(&mut self.courier_new_substitutions, 1, suppressed);
        }
        if nearest_family_substitution {
            Self::add(&mut self.nearest_family_substitutions, 1, suppressed);
        }
        Self::add(&mut self.face_identity_unverified_runs, 1, suppressed);
        Self::add(&mut self.metric_policy_unverified_runs, 1, suppressed);
        Self::add(
            &mut self.glyph_coverage_unverified_scalars,
            scalar_count,
            suppressed,
        );
        Self::add(
            &mut self.resolved_face_missing_glyph_scalars,
            missing_glyph_count,
            suppressed,
        );
        if unsupported_color {
            Self::add(&mut self.unsupported_color_fallbacks, 1, suppressed);
        }
    }

    fn record_natural_extent_exceeds_line_box(&mut self) {
        Self::add(
            &mut self.natural_extent_exceeds_line_box_lines,
            1,
            &mut self.suppressed_events,
        );
    }
}

fn directwrite_family(source_kind: ResolvedFontSourceKind, family_label: &str) -> (String, bool) {
    if source_kind == ResolvedFontSourceKind::Bundled {
        ("Courier New".to_owned(), true)
    } else {
        (family_label.to_owned(), false)
    }
}

fn valid_boundaries(offsets: &[FixedPoint], scalar_count: usize, width: FixedPoint) -> bool {
    scalar_count.checked_add(1) == Some(offsets.len())
        && offsets.first().is_some_and(|offset| offset.raw() == 0)
        && offsets.last() == Some(&width)
        && offsets.windows(2).all(|pair| pair[1] >= pair[0])
}

fn directwrite_style_flags(bold: bool, italic: bool) -> (bool, bool) {
    (bold, italic)
}

fn bounded_font_size(size: u16) -> Option<u16> {
    (size > 0 && size <= MAX_SUPPORTED_FONT_SIZE).then_some(size)
}

fn print_color(color: EffectiveTextColor) -> ([u8; 3], bool) {
    match color {
        EffectiveTextColor::ThemeDefault => ([0, 0, 0], false),
        EffectiveTextColor::ExplicitRgb8 { red, green, blue } => ([red, green, blue], false),
        EffectiveTextColor::UnsupportedExplicit => ([0, 0, 0], true),
    }
}

/// A page whose coordinates and size remain exact hundredths of a point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VectorPage {
    number: usize,
    title_page: bool,
    width: FixedPoint,
    height: FixedPoint,
    runs: Vec<VectorTextRun>,
}

impl VectorPage {
    pub(super) fn number(&self) -> usize {
        self.number
    }

    pub(super) fn is_title_page(&self) -> bool {
        self.title_page
    }

    pub(super) fn size(&self) -> (FixedPoint, FixedPoint) {
        (self.width, self.height)
    }

    pub(super) fn runs(&self) -> &[VectorTextRun] {
        &self.runs
    }
}

/// One positioned DirectWrite text span and its independently represented
/// baseline-relative decorations. The source/display string is retained without
/// a PDF-like ASCII conversion; it is valid UTF-8 and encoded scalar-by-scalar
/// as one or two UTF-16 code units. Canonical x boundaries and captured line
/// metrics remain immutable; resolved family names do not establish face-byte,
/// shaping, or coverage identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VectorTextRun {
    text: Arc<str>,
    geometry: FixedRect,
    baseline_y: FixedPoint,
    line_leading: FixedPoint,
    natural_ascent: FixedPoint,
    natural_descent: FixedPoint,
    display_boundary_offsets: Arc<[FixedPoint]>,
    family_name: Arc<str>,
    bold: bool,
    italic: bool,
    color_rgb: [u8; 3],
    font_size_pt: u16,
    underline: bool,
    strikethrough: bool,
}

impl VectorTextRun {
    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn geometry(&self) -> FixedRect {
        self.geometry
    }

    pub(super) fn baseline_y(&self) -> FixedPoint {
        self.baseline_y
    }

    pub(super) fn line_leading(&self) -> FixedPoint {
        self.line_leading
    }

    pub(super) fn natural_ascent(&self) -> FixedPoint {
        self.natural_ascent
    }

    pub(super) fn natural_descent(&self) -> FixedPoint {
        self.natural_descent
    }

    pub(super) fn display_boundary_offsets(&self) -> &[FixedPoint] {
        &self.display_boundary_offsets
    }

    pub(super) fn family_name(&self) -> &str {
        &self.family_name
    }

    pub(super) fn bold(&self) -> bool {
        self.bold
    }

    pub(super) fn italic(&self) -> bool {
        self.italic
    }

    pub(super) fn color_rgb(&self) -> [u8; 3] {
        self.color_rgb
    }

    pub(super) fn font_size_pt(&self) -> u16 {
        self.font_size_pt
    }

    pub(super) fn underline(&self) -> bool {
        self.underline
    }

    pub(super) fn strikethrough(&self) -> bool {
        self.strikethrough
    }
}

/// Opaque immutable output. No method exposes mutable page/run storage, and all
/// text is shared with the freshly built immutable layout via `Arc<str>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WindowsPrintPackage {
    include_title_page: bool,
    pages: Vec<VectorPage>,
    run_count: usize,
    text_bytes: usize,
    typography_report: WindowsPrintTypographyReport,
}

impl WindowsPrintPackage {
    pub(super) fn includes_title_page(&self) -> bool {
        self.include_title_page
    }

    pub(super) fn pages(&self) -> &[VectorPage] {
        &self.pages
    }

    pub(super) fn run_count(&self) -> usize {
        self.run_count
    }

    pub(super) fn text_bytes(&self) -> usize {
        self.text_bytes
    }

    pub(super) fn typography_report(&self) -> WindowsPrintTypographyReport {
        self.typography_report
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PackageBuildError {
    Layout,
    EmptyLayout,
    ResourceLimit,
    Allocation,
    InvalidTypographyEnvironment,
    InvalidLayoutGeometry,
}

impl From<LayoutError> for PackageBuildError {
    fn from(_: LayoutError) -> Self {
        Self::Layout
    }
}

fn validate_line_geometry(
    line: &scriptcraft_layout::LayoutLine,
) -> Result<FixedPoint, PackageBuildError> {
    let metrics = line.vertical_metrics;
    if metrics.leading.raw() <= 0
        || line.geometry.height != metrics.leading
        || line.paragraph_bounds.y != line.geometry.y
        || line.paragraph_bounds.height != metrics.leading
    {
        return Err(PackageBuildError::InvalidLayoutGeometry);
    }
    let baseline_raw = line
        .geometry
        .y
        .raw()
        .checked_add(line.geometry.height.raw())
        .ok_or(PackageBuildError::InvalidLayoutGeometry)?;
    let paragraph_bottom = line
        .paragraph_bounds
        .y
        .raw()
        .checked_add(line.paragraph_bounds.height.raw())
        .ok_or(PackageBuildError::InvalidLayoutGeometry)?;
    if baseline_raw != paragraph_bottom {
        return Err(PackageBuildError::InvalidLayoutGeometry);
    }
    Ok(FixedPoint::from_raw(baseline_raw))
}

#[derive(Debug, Clone, Copy)]
struct PackageLimits {
    pages: usize,
    runs: usize,
    text_bytes: usize,
}

const PACKAGE_LIMITS: PackageLimits = PackageLimits {
    pages: MAX_PACKAGE_PAGES,
    runs: MAX_PACKAGE_RUNS,
    text_bytes: MAX_PACKAGE_TEXT_BYTES,
};

/// Build one fresh Print composition; page-only inputs must equal the captured
/// environment. This does not consult the editor layout cache or accept
/// viewport, printer, DirectWrite, or presentation layout authority.
pub(super) fn build_package(
    project: &Project,
    inputs: &LayoutInputs,
    environment: &LayoutEnvironment,
    include_title_page: bool,
) -> Result<WindowsPrintPackage, PackageBuildError> {
    if inputs.page_only() != environment.page {
        return Err(PackageBuildError::InvalidTypographyEnvironment);
    }
    let source_settings = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_default();
    let layout = ScreenplayLayout::build_with_composition_and_environment(
        project,
        inputs,
        &source_settings,
        LayoutComposition::Print { include_title_page },
        environment,
    )?;
    package_from_layout(
        &layout,
        &environment.fonts,
        include_title_page,
        PACKAGE_LIMITS,
    )
}

fn package_from_layout(
    layout: &ScreenplayLayout,
    fonts: &scriptcraft_model::typography::DocumentFontEnvironment,
    include_title_page: bool,
    limits: PackageLimits,
) -> Result<WindowsPrintPackage, PackageBuildError> {
    if layout.pages.is_empty() {
        return Err(PackageBuildError::EmptyLayout);
    }
    if layout.pages.len() > limits.pages {
        return Err(PackageBuildError::ResourceLimit);
    }

    let mut pages = Vec::new();
    pages
        .try_reserve_exact(layout.pages.len())
        .map_err(|_| PackageBuildError::Allocation)?;
    let mut run_count = 0usize;
    let mut text_bytes = 0usize;
    let mut typography_report = WindowsPrintTypographyReport::default();

    for page in &layout.pages {
        let mut required_runs = 0usize;
        for span in page.lines.iter().flat_map(|line| line.spans.iter()) {
            let style = &span.resolved_text_style;
            if !style.hidden_text && !span.display_text.is_empty() {
                required_runs = required_runs
                    .checked_add(1)
                    .ok_or(PackageBuildError::ResourceLimit)?;
            }
        }
        run_count = run_count
            .checked_add(required_runs)
            .ok_or(PackageBuildError::ResourceLimit)?;
        if run_count > limits.runs {
            return Err(PackageBuildError::ResourceLimit);
        }

        let mut runs = Vec::new();
        runs.try_reserve_exact(required_runs)
            .map_err(|_| PackageBuildError::Allocation)?;
        for line in &page.lines {
            let baseline_y = validate_line_geometry(line)?;
            let natural_top_raw = baseline_y
                .raw()
                .checked_sub(line.vertical_metrics.natural_ascent.raw())
                .ok_or(PackageBuildError::InvalidLayoutGeometry)?;
            let natural_bottom_raw = baseline_y
                .raw()
                .checked_sub(line.vertical_metrics.natural_descent.raw())
                .ok_or(PackageBuildError::InvalidLayoutGeometry)?;
            if natural_top_raw > natural_bottom_raw {
                return Err(PackageBuildError::InvalidLayoutGeometry);
            }
            let mut line_has_visible_text = false;
            for span in line.spans.iter() {
                let style = &span.resolved_text_style;
                if style.hidden_text {
                    continue;
                }
                let scalar_count = span.display_text.chars().count();
                if span.geometry.width.raw() < 0
                    || span.geometry.y != line.geometry.y
                    || span.geometry.height != line.vertical_metrics.leading
                    || span
                        .geometry
                        .y
                        .raw()
                        .checked_add(span.geometry.height.raw())
                        != Some(baseline_y.raw())
                    || !valid_boundaries(
                        &span.display_boundary_offsets,
                        scalar_count,
                        span.geometry.width,
                    )
                {
                    return Err(PackageBuildError::InvalidLayoutGeometry);
                }
                let request = fonts
                    .require_resolution_for(&style.declared_family)
                    .map_err(|_| PackageBuildError::InvalidTypographyEnvironment)?;
                let resolved_face = fonts
                    .require_face(&style.resolved_font_id)
                    .map_err(|_| PackageBuildError::InvalidTypographyEnvironment)?;
                if span.display_text.is_empty() {
                    continue;
                }
                line_has_visible_text = true;
                text_bytes = text_bytes
                    .checked_add(span.display_text.len())
                    .ok_or(PackageBuildError::ResourceLimit)?;
                if text_bytes > limits.text_bytes {
                    return Err(PackageBuildError::ResourceLimit);
                }
                let (family_name, courier_new_substitution) =
                    directwrite_family(resolved_face.source_kind, &resolved_face.family_label);
                let nearest_family_substitution = matches!(
                    request.kind,
                    FontResolutionKind::NearestSans
                        | FontResolutionKind::NearestSerif
                        | FontResolutionKind::ExactHost
                ) && resolved_face.source_kind != ResolvedFontSourceKind::Bundled
                    && !resolved_face
                    .family_label
                    .eq_ignore_ascii_case(&style.declared_family);
                let size = bounded_font_size(style.size)
                    .ok_or(PackageBuildError::InvalidTypographyEnvironment)?;
                let (color_rgb, unsupported_color) = print_color(style.color);
                let (bold, italic) = directwrite_style_flags(style.bold, style.italic);
                let missing_glyph_count = span
                    .display_text
                    .chars()
                    .try_fold(0usize, |count, scalar| {
                        let metric = resolved_face
                            .metrics
                            .advance_for(scalar)
                            .ok_or(PackageBuildError::InvalidTypographyEnvironment)?;
                        count
                            .checked_add(usize::from(metric.missing_glyph))
                            .ok_or(PackageBuildError::ResourceLimit)
                    })?;
                typography_report.record_run(
                    courier_new_substitution,
                    nearest_family_substitution,
                    scalar_count,
                    missing_glyph_count,
                    unsupported_color,
                );
                runs.push(VectorTextRun {
                    text: Arc::clone(&span.display_text),
                    geometry: span.geometry,
                    baseline_y,
                    line_leading: line.vertical_metrics.leading,
                    natural_ascent: line.vertical_metrics.natural_ascent,
                    natural_descent: line.vertical_metrics.natural_descent,
                    display_boundary_offsets: Arc::clone(&span.display_boundary_offsets),
                    family_name: Arc::from(family_name),
                    bold,
                    italic,
                    color_rgb,
                    font_size_pt: size,
                    underline: span.underline,
                    strikethrough: span.strikethrough,
                });
            }
            if line_has_visible_text
                && (natural_top_raw < line.geometry.y.raw()
                    || natural_bottom_raw > baseline_y.raw())
            {
                typography_report.record_natural_extent_exceeds_line_box();
            }
        }
        pages.push(VectorPage {
            number: page.number,
            title_page: page.title_page,
            width: FixedPoint::from_points(i64::from(layout.inputs.paper_width)),
            height: FixedPoint::from_points(i64::from(layout.inputs.paper_height)),
            runs,
        });
    }

    Ok(WindowsPrintPackage {
        include_title_page,
        pages,
        run_count,
        text_bytes,
        typography_report,
    })
}
