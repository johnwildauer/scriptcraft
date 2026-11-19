//! Script View page-mode policy and document/screen coordinate conversion.

use crate::chrome::{status_bar::PresentedPage, theme};
use eframe::egui::{self, Color32};
use scriptcraft_commands::{EditorSurface, ScriptViewMode};

const PAGE_VERTICAL_GAP: f32 = 12.0;
pub(super) const NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH: f32 = 1.0;
const PAGE_PAINT_OVERSCAN: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PresentedPagePolicy {
    pub(super) page_height: f32,
    pub(super) document_y_offset: f32,
    pub(super) explicit_gap: f32,
    pub(super) vertical_item_spacing: f32,
}

pub(super) fn resolve_presentation_mode(
    surface: EditorSurface,
    body_mode: impl FnOnce() -> ScriptViewMode,
) -> ScriptViewMode {
    match surface {
        EditorSurface::Body => body_mode(),
        EditorSurface::TitlePage => ScriptViewMode::Page,
    }
}

pub(super) fn presented_page_policy(
    mode: ScriptViewMode,
    paper_height: f32,
    content_top: f32,
    content_height: f32,
    scale: f32,
    vertical_item_spacing: f32,
) -> PresentedPagePolicy {
    let document_y_offset = match mode {
        ScriptViewMode::Speed => content_top,
        ScriptViewMode::Normal | ScriptViewMode::Page => 0.0,
    };
    let speed_view = mode == ScriptViewMode::Speed;
    PresentedPagePolicy {
        page_height: if speed_view {
            content_height * scale
        } else {
            paper_height * scale
        },
        document_y_offset,
        explicit_gap: if speed_view { 0.0 } else { PAGE_VERTICAL_GAP },
        vertical_item_spacing: if speed_view {
            0.0
        } else {
            vertical_item_spacing
        },
    }
}

pub(super) fn page_chrome_visible(mode: ScriptViewMode) -> bool {
    mode == ScriptViewMode::Page
}

pub(super) fn normal_view_page_bottom_rule_visible(
    mode: ScriptViewMode,
    page_index: usize,
    page_count: usize,
) -> bool {
    mode == ScriptViewMode::Normal && page_index < page_count.saturating_sub(1)
}

pub(super) fn normal_view_page_bottom_rule_geometry(page_rect: egui::Rect) -> [egui::Pos2; 2] {
    [
        egui::pos2(page_rect.left(), page_rect.bottom()),
        egui::pos2(page_rect.right(), page_rect.bottom()),
    ]
}

pub(super) fn script_view_backdrop(
    mode: ScriptViewMode,
    palette: theme::ScriptWorkspacePalette,
) -> Color32 {
    match mode {
        ScriptViewMode::Page => palette.background,
        ScriptViewMode::Normal | ScriptViewMode::Speed => palette.page.paper,
    }
}

pub(super) fn document_y_to_screen(page: PresentedPage, document_y: f32, scale: f32) -> f32 {
    page.rect.top() + (document_y - page.document_y_offset) * scale
}

pub(super) fn screen_y_to_document(page: PresentedPage, screen_y: f32, scale: f32) -> f32 {
    page.document_y_offset + (screen_y - page.rect.top()) / scale
}

pub(super) fn page_local_screen_origin(page: PresentedPage, scale: f32) -> egui::Pos2 {
    egui::pos2(page.rect.left(), document_y_to_screen(page, 0.0, scale))
}

pub(super) fn viewport_paint_page_range(
    pages: &[PresentedPage],
    editor_clip: egui::Rect,
) -> Option<(usize, usize)> {
    let mut first_visible = None;
    let mut last_visible = None;
    for (page_index, page) in pages.iter().enumerate() {
        if page.rect.intersects(editor_clip) {
            first_visible.get_or_insert(page_index);
            last_visible = Some(page_index);
        }
    }
    let first_visible = first_visible?;
    let last_visible = last_visible?;
    Some((
        first_visible.saturating_sub(PAGE_PAINT_OVERSCAN),
        last_visible
            .saturating_add(PAGE_PAINT_OVERSCAN)
            .min(pages.len().saturating_sub(1)),
    ))
}
