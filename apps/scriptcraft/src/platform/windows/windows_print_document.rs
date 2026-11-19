//! Immutable vector-page package derived from the canonical Print layout.
//!
//! The page model deliberately contains no editor/egui state and keeps geometry
//! in the core's exact hundredths-of-a-point units. The completed Windows Print
//! adapter turns these records into Direct2D command lists and submits them
//! through `ID2D1PrintControl::AddPage` without rebuilding the screenplay layout.

use scriptcraft_layout::{
    FixedRect, LayoutComposition, LayoutEnvironment, LayoutError, ScreenplayLayout,
};
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::typography::{
    parse_effective_text_color, EffectiveTextColor, FontResolutionKind, ResolvedFontSourceKind,
    MAX_SUPPORTED_FONT_SIZE,
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
    canonical_geometry_unverified_runs: u32,
    glyph_coverage_unverified_scalars: u32,
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
            "Windows typography fallback report: Courier New substitutions {}; nearest-family substitutions {}; DirectWrite face identity unverified for {} runs; DirectWrite metric/shaping policy unverified for {} runs; canonical layout metric geometry unverified for {} runs; glyph coverage unverified for {} scalars; unsupported colors rendered black {}; report events capped {}",
            self.courier_new_substitutions,
            self.nearest_family_substitutions,
            self.face_identity_unverified_runs,
            self.metric_policy_unverified_runs,
            self.canonical_geometry_unverified_runs,
            self.glyph_coverage_unverified_scalars,
            self.unsupported_color_fallbacks,
            self.suppressed_events,
        ))
    }

    fn record_run(
        &mut self,
        courier_new_substitution: bool,
        nearest_family_substitution: bool,
        scalar_count: usize,
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
        Self::add(&mut self.canonical_geometry_unverified_runs, 1, suppressed);
        Self::add(
            &mut self.glyph_coverage_unverified_scalars,
            scalar_count,
            suppressed,
        );
        if unsupported_color {
            Self::add(&mut self.unsupported_color_fallbacks, 1, suppressed);
        }
    }
}

fn directwrite_family(
    kind: FontResolutionKind,
    source_kind: ResolvedFontSourceKind,
    family_label: &str,
) -> (String, bool) {
    if kind == FontResolutionKind::UnknownFallback || source_kind == ResolvedFontSourceKind::Bundled
    {
        ("Courier New".to_owned(), true)
    } else {
        (family_label.to_owned(), false)
    }
}

fn valid_boundaries(offsets: &[FixedPoint], scalar_count: usize, width: FixedPoint) -> bool {
    offsets.len() == scalar_count.saturating_add(1)
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

fn print_color(source: Option<&str>) -> ([u8; 3], bool) {
    match parse_effective_text_color(source) {
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
/// vector decorations. The source/display string is retained without a PDF-like
/// ASCII conversion; it is valid UTF-8 and can be encoded losslessly as UTF-16.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VectorTextRun {
    text: Arc<str>,
    geometry: FixedRect,
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

/// Build a fresh Print composition; this does not consult the editor layout
/// cache or accept viewport/presentation state.
pub(super) fn build_package(
    project: &Project,
    inputs: &LayoutInputs,
    environment: &LayoutEnvironment,
    include_title_page: bool,
) -> Result<WindowsPrintPackage, PackageBuildError> {
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
        let required_runs = page
            .lines
            .iter()
            .try_fold(0usize, |count, line| count.checked_add(line.spans.len()));
        let Some(required_runs) = required_runs else {
            return Err(PackageBuildError::ResourceLimit);
        };
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
            for span in line.spans.iter() {
                text_bytes = text_bytes
                    .checked_add(span.display_text.len())
                    .ok_or(PackageBuildError::ResourceLimit)?;
                if text_bytes > limits.text_bytes {
                    return Err(PackageBuildError::ResourceLimit);
                }
                let scalar_count = span.display_text.chars().count();
                if !valid_boundaries(
                    &span.display_boundary_offsets,
                    scalar_count,
                    span.geometry.width,
                ) {
                    return Err(PackageBuildError::InvalidLayoutGeometry);
                }
                let request = fonts
                    .require_resolution_for(&span.text_style.declared_family)
                    .map_err(|_| PackageBuildError::InvalidTypographyEnvironment)?;
                let font_id = request.face_for_flags(span.text_style.bold, span.text_style.italic);
                let resolved_face = fonts
                    .require_face(font_id)
                    .map_err(|_| PackageBuildError::InvalidTypographyEnvironment)?;
                let (family_name, courier_new_substitution) = directwrite_family(
                    request.kind,
                    resolved_face.source_kind,
                    &resolved_face.family_label,
                );
                let nearest_family_substitution = matches!(
                    request.kind,
                    FontResolutionKind::NearestSans | FontResolutionKind::NearestSerif
                ) && !resolved_face
                    .family_label
                    .eq_ignore_ascii_case(&span.text_style.declared_family);
                let size = bounded_font_size(u16::from(span.text_style.size))
                    .ok_or(PackageBuildError::InvalidTypographyEnvironment)?;
                let (color_rgb, unsupported_color) =
                    print_color(span.text_style.source_color.as_deref());
                let (bold, italic) =
                    directwrite_style_flags(span.text_style.bold, span.text_style.italic);
                typography_report.record_run(
                    courier_new_substitution,
                    nearest_family_substitution,
                    scalar_count,
                    unsupported_color,
                );
                runs.push(VectorTextRun {
                    text: Arc::clone(&span.display_text),
                    geometry: span.geometry,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courier_prime_and_unknown_fallback_are_reported_as_windows_substitutions() {
        assert_eq!(
            directwrite_family(
                FontResolutionKind::CourierAlias,
                ResolvedFontSourceKind::Bundled,
                "Courier Prime",
            ),
            ("Courier New".to_owned(), true),
        );
        assert_eq!(
            directwrite_family(
                FontResolutionKind::UnknownFallback,
                ResolvedFontSourceKind::Bundled,
                "Courier Prime",
            ),
            ("Courier New".to_owned(), true),
        );
    }

    #[test]
    fn nearest_host_family_uses_the_resolved_family_name() {
        assert_eq!(
            directwrite_family(
                FontResolutionKind::NearestSans,
                ResolvedFontSourceKind::Host,
                "Liberation Sans",
            ),
            ("Liberation Sans".to_owned(), false),
        );
        assert_eq!(
            directwrite_family(
                FontResolutionKind::NearestSerif,
                ResolvedFontSourceKind::Host,
                "Georgia",
            ),
            ("Georgia".to_owned(), false),
        );
    }

    #[test]
    fn reports_are_count_only_and_cap_each_category() {
        let mut report = WindowsPrintTypographyReport::default();
        report.record_run(true, true, MAX_REPORT_COUNT as usize + 4, true);
        let summary = report.summary().expect("fallbacks are present");
        assert!(summary.contains("Courier New substitutions 1"));
        assert!(summary.contains("nearest-family substitutions 1"));
        assert!(summary.contains("glyph coverage unverified for 1000000 scalars"));
        assert!(summary.contains("canonical layout metric geometry unverified for 1 runs"));
        assert!(summary.contains("unsupported colors rendered black 1"));
        assert!(summary.contains("report events capped 4"));
        assert!(!summary.contains("Courier Prime"));
        assert!(!summary.contains("Liberation Sans"));
    }

    #[test]
    fn scalar_boundaries_must_cover_the_run_and_end_at_its_canonical_width() {
        let width = FixedPoint::from_raw(250);
        let valid = [FixedPoint::default(), FixedPoint::from_raw(100), width];
        assert!(valid_boundaries(&valid, 2, width));
        assert!(!valid_boundaries(&valid[..2], 2, width));
        assert!(!valid_boundaries(&valid, 1, width));
        assert!(!valid_boundaries(&valid, 2, FixedPoint::from_raw(251)));
    }

    #[test]
    fn directwrite_style_and_size_policy_keeps_each_face_independent() {
        assert_eq!(directwrite_style_flags(false, false), (false, false));
        assert_eq!(directwrite_style_flags(true, false), (true, false));
        assert_eq!(directwrite_style_flags(false, true), (false, true));
        assert_eq!(directwrite_style_flags(true, true), (true, true));
        assert_eq!(bounded_font_size(1), Some(1));
        assert_eq!(bounded_font_size(72), Some(72));
        assert_eq!(
            bounded_font_size(MAX_SUPPORTED_FONT_SIZE),
            Some(MAX_SUPPORTED_FONT_SIZE)
        );
        assert_eq!(bounded_font_size(0), None);
        assert_eq!(bounded_font_size(MAX_SUPPORTED_FONT_SIZE + 1), None);
    }

    #[test]
    fn print_colors_preserve_explicit_rgb_and_bound_fallback_to_black() {
        assert_eq!(print_color(None), ([0, 0, 0], false));
        assert_eq!(
            print_color(Some("#87878F8FE0E0")),
            ([0x87, 0x8f, 0xe0], false)
        );
        assert_eq!(print_color(Some("not-a-color")), ([0, 0, 0], true));
    }

    #[test]
    fn package_and_callback_resource_limits_remain_explicit_and_bounded() {
        assert_eq!(PACKAGE_LIMITS.pages, MAX_PACKAGE_PAGES);
        assert_eq!(PACKAGE_LIMITS.runs, MAX_PACKAGE_RUNS);
        assert_eq!(PACKAGE_LIMITS.text_bytes, MAX_PACKAGE_TEXT_BYTES);
        assert!(MAX_PACKAGE_PAGES <= 512);
        assert!(MAX_PACKAGE_RUNS <= 262_144);
        assert!(MAX_PACKAGE_TEXT_BYTES <= 16 * 1024 * 1024);
    }

    #[test]
    fn cloned_print_package_remains_an_immutable_request_snapshot() {
        let text: Arc<str> = Arc::from("x");
        let run = VectorTextRun {
            text: Arc::clone(&text),
            geometry: FixedRect {
                x: FixedPoint::default(),
                y: FixedPoint::default(),
                width: FixedPoint::from_raw(100),
                height: FixedPoint::from_raw(1200),
            },
            display_boundary_offsets: Arc::from(vec![
                FixedPoint::default(),
                FixedPoint::from_raw(100),
            ]),
            family_name: Arc::from("Arial"),
            bold: false,
            italic: false,
            color_rgb: [0, 0, 0],
            font_size_pt: 12,
            underline: false,
            strikethrough: false,
        };
        let package = WindowsPrintPackage {
            include_title_page: true,
            pages: vec![VectorPage {
                number: 1,
                title_page: false,
                width: FixedPoint::from_points(612),
                height: FixedPoint::from_points(792),
                runs: vec![run],
            }],
            run_count: 1,
            text_bytes: 1,
            typography_report: WindowsPrintTypographyReport::default(),
        };
        let captured = package.clone();
        assert_eq!(captured, package);
        assert!(Arc::ptr_eq(
            &captured.pages[0].runs[0].text,
            &package.pages[0].runs[0].text,
        ));
        assert_eq!(
            captured.pages()[0].runs()[0]
                .display_boundary_offsets()
                .len(),
            2
        );
    }
}
