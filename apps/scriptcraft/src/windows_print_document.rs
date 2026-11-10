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

#[cfg(test)]
mod tests {
    use super::{
        build_package, package_from_layout, PackageBuildError, PackageLimits, PrintFontFace,
        WindowsPrintPackage, MAX_PACKAGE_PAGES, MAX_PACKAGE_RUNS, MAX_PACKAGE_TEXT_BYTES,
    };
    use scriptcraft_core::dual_dialogue::{
        DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance,
    };
    use scriptcraft_core::editing::EditingSession;
    use scriptcraft_core::formatting::FixedPoint;
    use scriptcraft_core::layout::{LayoutComposition, ScreenplayLayout};
    use scriptcraft_core::{
        LayoutInputs, Project, Role, RunAttribute, ScreenplayElement, StableId, TextRun,
    };

    fn valid<T, E>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|_| unreachable!("package fixtures use valid bounded input"))
    }

    fn base_project() -> Project {
        let mut project = valid(Project::new());
        project.elements = vec![
            valid(ScreenplayElement::new("hidden-summary", Role::Summary, "HIDDEN_ROLE")),
            valid(ScreenplayElement::new("action-1", Role::Action, "VISIBLE ACTION")),
        ];
        project
    }

    fn print_layout(project: &Project, inputs: &LayoutInputs, include_title_page: bool) -> ScreenplayLayout {
        let source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings().clone())
            .unwrap_or_default();
        valid(ScreenplayLayout::build_with_composition(
            project,
            inputs,
            &source_settings,
            LayoutComposition::Print { include_title_page },
        ))
    }

    fn flatten_runs(package: &WindowsPrintPackage) -> Vec<&super::VectorTextRun> {
        package
            .pages()
            .iter()
            .flat_map(|page| page.runs())
            .collect()
    }

    fn dual_dialogue_project() -> Project {
        let mut project = valid(Project::new());
        project.elements = [
            (Role::Action, "Before"),
            (Role::Character, "LEFT CUE"),
            (Role::Dialogue, "Left dialogue"),
            (Role::Character, "RIGHT CUE"),
            (Role::Dialogue, "Right dialogue"),
            (Role::Action, "After"),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (role, text))| {
            valid(ScreenplayElement::new(format!("dual-{index}"), role, text))
        })
        .collect();
        project.dual_dialogue_groups.push(DualDialogueGroup {
            id: valid(StableId::new("dual-print-group")),
            left: DualDialogueSide {
                character: project.elements[1].id.clone(),
                paragraphs: vec![project.elements[2].id.clone()],
            },
            right: DualDialogueSide {
                character: project.elements[3].id.clone(),
                paragraphs: vec![project.elements[4].id.clone()],
            },
            wrapper: DualDialogueWrapperProvenance::default(),
        });
        project
    }

    #[test]
    fn package_pages_keep_print_page_count_size_and_exact_span_coordinates() {
        let project = base_project();
        let inputs = LayoutInputs::default();
        let layout = print_layout(&project, &inputs, false);
        let package = valid(build_package(&project, &inputs, false));

        assert_eq!(package.pages().len(), layout.pages.len());
        assert_eq!(package.pages()[0].size(), (
            FixedPoint::from_points(i64::from(inputs.paper_width)),
            FixedPoint::from_points(i64::from(inputs.paper_height)),
        ));
        for (page, layout_page) in package.pages().iter().zip(&layout.pages) {
            assert_eq!(page.number(), layout_page.number);
            let packaged_geometry = page
                .runs()
                .iter()
                .map(|run| run.geometry())
                .collect::<Vec<_>>();
            let layout_geometry = layout_page
                .lines
                .iter()
                .flat_map(|line| line.spans.iter().map(|span| span.geometry))
                .collect::<Vec<_>>();
            assert_eq!(packaged_geometry, layout_geometry);
        }
    }

    #[test]
    fn include_and_exclude_title_page_preserve_title_first_body_only_order() {
        let project = base_project();
        let inputs = LayoutInputs::default();
        let included = valid(build_package(&project, &inputs, true));
        let excluded = valid(build_package(&project, &inputs, false));

        assert!(included.includes_title_page());
        assert!(included.pages().first().is_some_and(|page| page.is_title_page()));
        assert!(included.pages().iter().skip(1).all(|page| !page.is_title_page()));
        assert!(!excluded.includes_title_page());
        assert!(excluded.pages().iter().all(|page| !page.is_title_page()));
        assert_eq!(included.pages().len(), excluded.pages().len() + 1);
        assert!(flatten_runs(&included).iter().any(|run| run.text().contains("VISIBLE ACTION")));
        assert!(flatten_runs(&excluded).iter().any(|run| run.text().contains("VISIBLE ACTION")));
    }

    #[test]
    fn print_layout_omits_hidden_roles_and_package_retains_visible_text() {
        let project = base_project();
        let package = valid(build_package(&project, &LayoutInputs::default(), false));
        let mut text = String::new();
        for run in flatten_runs(&package) {
            text.push_str(run.text());
        }

        assert!(text.contains("VISIBLE ACTION"));
        assert!(!text.contains("HIDDEN_ROLE"));
    }

    #[test]
    fn font_face_and_decorations_are_explicit_and_unicode_is_not_substituted() {
        let mut project = base_project();
        project.elements[1] = valid(ScreenplayElement::with_runs(
            "styled-unicode",
            Role::Action,
            vec![TextRun {
                text: "café 漢字 😀".to_owned(),
                attributes: vec![RunAttribute {
                    name: "Style".to_owned(),
                    value: "Bold+Italic+Underline+Strikethrough".to_owned(),
                }],
            }],
            None,
        ));
        let package = valid(build_package(&project, &LayoutInputs::default(), false));
        let run = flatten_runs(&package)
            .into_iter()
            .find(|run| run.text().contains("漢字 😀"))
            .unwrap_or_else(|| unreachable!("Print layout retains the Unicode sentinel"));

        assert_eq!(run.face(), PrintFontFace::CourierNewBoldItalic);
        assert_eq!(run.face().family_name(), "Courier New");
        assert_eq!(run.font_size_pt(), 12);
        assert!(run.underline());
        assert!(run.strikethrough());
        assert!(run.text().contains("café") || run.text().contains("CAFÉ"));
        assert!(!run.text().contains('?'));
        let utf16 = run.text().encode_utf16().collect::<Vec<_>>();
        assert!(String::from_utf16(&utf16).is_ok_and(|decoded| decoded == run.text()));
    }

    #[test]
    fn font_styles_map_deterministically_to_four_windows_faces() {
        assert_eq!(PrintFontFace::for_style(false, false), PrintFontFace::CourierNew);
        assert_eq!(PrintFontFace::for_style(true, false), PrintFontFace::CourierNewBold);
        assert_eq!(PrintFontFace::for_style(false, true), PrintFontFace::CourierNewItalic);
        assert_eq!(
            PrintFontFace::for_style(true, true),
            PrintFontFace::CourierNewBoldItalic
        );
    }

    #[test]
    fn print_dual_dialogue_keeps_two_column_geometry_without_editor_presentation() {
        let project = dual_dialogue_project();
        let layout = print_layout(&project, &LayoutInputs::default(), false);
        let package = valid(build_package(&project, &LayoutInputs::default(), false));
        let packaged = flatten_runs(&package);
        let left = packaged.iter().find(|run| run.text().contains("Left dialogue"));
        let right = packaged.iter().find(|run| run.text().contains("Right dialogue"));
        let expected = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter().flat_map(|line| line.spans.iter()))
            .filter(|span| span.display_text.contains("dialogue"))
            .map(|span| span.geometry)
            .collect::<Vec<_>>();

        assert_eq!(expected.len(), 2);
        assert!(left.is_some());
        assert!(right.is_some());
        assert_ne!(
            left.unwrap_or_else(|| unreachable!("left column is in Print layout")).geometry().x,
            right.unwrap_or_else(|| unreachable!("right column is in Print layout")).geometry().x,
        );
        assert!(layout.active_group_id.is_none());
    }

    #[test]
    fn package_output_is_deterministic_detached_and_does_not_mutate_project_state() {
        let mut project = base_project();
        let before = project.clone();
        let inputs = LayoutInputs::default();
        let inputs_before = inputs.clone();
        let first = valid(build_package(&project, &inputs, false));
        let second = valid(build_package(&project, &inputs, false));

        assert_eq!(first, second);
        assert!(first.pages().len() <= MAX_PACKAGE_PAGES);
        assert!(first.run_count() <= MAX_PACKAGE_RUNS);
        assert!(first.text_bytes() <= MAX_PACKAGE_TEXT_BYTES);
        assert_eq!(project, before);
        assert_eq!(inputs, inputs_before);
        project.elements[1].text = "edited after package creation".to_owned();
        assert!(flatten_runs(&first).iter().any(|run| run.text().contains("VISIBLE ACTION")));
        assert!(!flatten_runs(&first)
            .iter()
            .any(|run| run.text().contains("edited after package creation")));
    }

    #[test]
    fn package_build_preserves_editing_session_document_and_transient_state() {
        let session = valid(EditingSession::new(base_project()));
        let project_before = session.project().clone();
        let inputs_before = session.layout_inputs().clone();
        let selection_before = session.selection();
        let title_selection_before = session.title_selection();
        let dirty_before = session.dirty();
        let revision_before = session.revision();
        let package = valid(build_package(
            session.project(),
            session.layout_inputs(),
            true,
        ));

        assert!(!package.pages().is_empty());
        assert_eq!(session.project(), &project_before);
        assert_eq!(session.layout_inputs(), &inputs_before);
        assert_eq!(session.selection(), selection_before);
        assert_eq!(session.title_selection(), title_selection_before);
        assert_eq!(session.dirty(), dirty_before);
        assert_eq!(session.revision(), revision_before);
    }

    #[test]
    fn package_builder_enforces_page_run_and_text_bounds_and_discards_partial_work() {
        let project = base_project();
        let layout = print_layout(&project, &LayoutInputs::default(), false);
        let page_limited = PackageLimits {
            pages: 0,
            runs: MAX_PACKAGE_RUNS,
            text_bytes: MAX_PACKAGE_TEXT_BYTES,
        };
        assert_eq!(
            package_from_layout(&layout, false, page_limited),
            Err(PackageBuildError::ResourceLimit)
        );

        let run_limited = PackageLimits {
            pages: MAX_PACKAGE_PAGES,
            runs: 0,
            text_bytes: MAX_PACKAGE_TEXT_BYTES,
        };
        assert_eq!(
            package_from_layout(&layout, false, run_limited),
            Err(PackageBuildError::ResourceLimit)
        );

        let text_limited = PackageLimits {
            pages: MAX_PACKAGE_PAGES,
            runs: MAX_PACKAGE_RUNS,
            text_bytes: 1,
        };
        assert_eq!(
            package_from_layout(&layout, false, text_limited),
            Err(PackageBuildError::ResourceLimit)
        );
        assert!(MAX_PACKAGE_PAGES <= 512);
        assert!(MAX_PACKAGE_RUNS <= 262_144);
        assert!(MAX_PACKAGE_TEXT_BYTES <= 16 * 1024 * 1024);
    }

    #[test]
    fn package_preserves_span_geometry_for_each_style_and_decoration() {
        let mut project = base_project();
        project.elements[1] = valid(ScreenplayElement::with_runs(
            "mixed-style",
            Role::Action,
            vec![
                TextRun {
                    text: "plain ".to_owned(),
                    attributes: Vec::new(),
                },
                TextRun {
                    text: "bold".to_owned(),
                    attributes: vec![RunAttribute {
                        name: "Style".to_owned(),
                        value: "Bold+Underline".to_owned(),
                    }],
                },
                TextRun {
                    text: " italic".to_owned(),
                    attributes: vec![RunAttribute {
                        name: "Style".to_owned(),
                        value: "Italic+Strikethrough".to_owned(),
                    }],
                },
            ],
            None,
        ));
        let layout = print_layout(&project, &LayoutInputs::default(), false);
        let package = valid(build_package(&project, &LayoutInputs::default(), false));
        let expected = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter().flat_map(|line| line.spans.iter()))
            .collect::<Vec<_>>();
        let actual = flatten_runs(&package);

        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_eq!(actual.geometry(), expected.geometry);
            assert_eq!(actual.cell_advance(), expected.char_width);
            assert_eq!(actual.underline(), expected.underline);
            assert_eq!(actual.strikethrough(), expected.strikethrough);
            assert_eq!(actual.font_size_pt(), expected.text_style.size);
            assert_eq!(actual.text(), expected.display_text.as_ref());
        }
    }
}
