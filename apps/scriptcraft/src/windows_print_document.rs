//! Immutable vector-page package derived from the canonical Print layout.
//!
//! The page model deliberately contains no editor/egui state and keeps geometry
//! in the core's exact hundredths-of-a-point units. WO-006.3 can turn these
//! records into Direct2D command lists and pass them to the
//! `ID2D1PrintControl::AddPage` route without rebuilding the screenplay layout.

use scriptcraft_core::formatting::FixedPoint;
use scriptcraft_core::layout::{
    FixedRect, LayoutComposition, LayoutError, ScreenplayLayout,
};
use scriptcraft_core::{LayoutInputs, Project};
use std::sync::Arc;

const MAX_PACKAGE_PAGES: usize = 512;
const MAX_PACKAGE_RUNS: usize = 262_144;
const MAX_PACKAGE_TEXT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PrintFontFace {
    CourierNew,
    CourierNewBold,
    CourierNewItalic,
    CourierNewBoldItalic,
}

impl PrintFontFace {
    pub(super) const fn family_name(self) -> &'static str {
        "Courier New"
    }

    fn for_style(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (true, true) => Self::CourierNewBoldItalic,
            (true, false) => Self::CourierNewBold,
            (false, true) => Self::CourierNewItalic,
            (false, false) => Self::CourierNew,
        }
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
    cell_advance: FixedPoint,
    face: PrintFontFace,
    font_size_pt: u8,
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

    pub(super) fn cell_advance(&self) -> FixedPoint {
        self.cell_advance
    }

    pub(super) fn face(&self) -> PrintFontFace {
        self.face
    }

    pub(super) fn font_size_pt(&self) -> u8 {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PackageBuildError {
    Layout,
    EmptyLayout,
    ResourceLimit,
    Allocation,
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
    include_title_page: bool,
) -> Result<WindowsPrintPackage, PackageBuildError> {
    let source_settings = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_default();
    let layout = ScreenplayLayout::build_with_composition(
        project,
        inputs,
        &source_settings,
        LayoutComposition::Print { include_title_page },
    )?;
    package_from_layout(&layout, include_title_page, PACKAGE_LIMITS)
}

fn package_from_layout(
    layout: &ScreenplayLayout,
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

    for page in &layout.pages {
        let required_runs = page.lines.iter().try_fold(0usize, |count, line| {
            count.checked_add(line.spans.len())
        });
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
                let face = PrintFontFace::for_style(
                    span.text_style.bold,
                    span.text_style.italic,
                );
                runs.push(VectorTextRun {
                    text: Arc::clone(&span.display_text),
                    geometry: span.geometry,
                    cell_advance: span.char_width,
                    face,
                    font_size_pt: span.text_style.size,
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
    })
}

