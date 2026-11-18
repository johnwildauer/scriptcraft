//! Script View presentation metrics, zoom planning, and scroll geometry.

use crate::chrome::status_bar::{self, PresentedPage, ViewportCenterAnchor};
use eframe::egui::{self, Vec2};
use scriptcraft_commands::ScriptViewMode;

use super::page_policy::presented_page_policy;

const PAGE_HORIZONTAL_PADDING: f32 = 16.0;
pub(super) const SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING: f32 = 8.0;
pub(super) const SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING: f32 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PresentationMetrics {
    pub(super) page_width: f32,
    pub(super) scale: f32,
    pub(super) font_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresentationPlatform {
    MacOS,
    #[cfg(target_os = "windows")]
    Windows,
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Other,
}

fn logical_points_per_document_point(platform: PresentationPlatform) -> f32 {
    match platform {
        PresentationPlatform::MacOS => 1.0,
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        PresentationPlatform::Other => 1.0,
        #[cfg(target_os = "windows")]
        PresentationPlatform::Windows => 96.0 / 72.0,
    }
}

fn current_presentation_platform() -> PresentationPlatform {
    #[cfg(target_os = "macos")]
    {
        PresentationPlatform::MacOS
    }
    #[cfg(target_os = "windows")]
    {
        PresentationPlatform::Windows
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PresentationPlatform::Other
    }
}

pub(super) fn presentation_metrics_at_zoom(
    available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
) -> PresentationMetrics {
    presentation_metrics_at_zoom_for_platform(
        available_width,
        paper_width,
        base_font_size,
        zoom_percent,
        current_presentation_platform(),
    )
}

fn presentation_metrics_at_zoom_for_platform(
    _available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
    platform: PresentationPlatform,
) -> PresentationMetrics {
    let normalized = status_bar::normalized_zoom_percent(zoom_percent as f64);
    let scale = logical_points_per_document_point(platform) * normalized as f32 / 100.0;
    let page_width = paper_width * scale;
    PresentationMetrics {
        page_width,
        scale,
        font_size: (base_font_size * scale).max(5.0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PageScrollGeometry {
    pub(super) viewport_width: f32,
    pub(super) content_width: f32,
    pub(super) leading_padding: f32,
    pub(super) horizontal_overflow: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PrepaintZoomPlan {
    page_origins: Vec<f32>,
    page_heights: Vec<f32>,
    page_strides: Vec<f32>,
    content_size: Vec2,
    pub(super) scroll_offset: Vec2,
}

pub(crate) fn has_nonzero_mouse_wheel(events: &[egui::Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            egui::Event::MouseWheel { delta, .. } if *delta != Vec2::ZERO
        )
    })
}

pub(crate) fn page_geometry_input_invalidated(
    ctx: &egui::Context,
    previous_clip: Option<egui::Rect>,
) -> bool {
    ctx.input(|input| {
        let scroll_changed = has_nonzero_mouse_wheel(&input.raw.events)
            || input.smooth_scroll_delta != Vec2::ZERO;
        let pointer_scrolling = input.pointer.any_down()
            && input
                .pointer
                .interact_pos()
                .zip(previous_clip)
                .is_some_and(|(position, clip)| clip.contains(position));
        scroll_changed || pointer_scrolling
    })
}

pub(super) fn page_scroll_geometry(viewport_width: f32, page_width: f32) -> PageScrollGeometry {
    let viewport_width = if viewport_width.is_finite() {
        viewport_width.max(0.0)
    } else {
        0.0
    };
    let page_width = if page_width.is_finite() {
        page_width.max(0.0)
    } else {
        0.0
    };
    let buffered_page_width = page_width + PAGE_HORIZONTAL_PADDING * 2.0;
    let content_width = viewport_width.max(buffered_page_width);
    PageScrollGeometry {
        viewport_width,
        content_width,
        leading_padding: ((content_width - page_width) / 2.0).max(0.0),
        horizontal_overflow: buffered_page_width > viewport_width,
    }
}

// Pure geometry inputs stay explicit so this helper remains directly testable.
#[allow(clippy::too_many_arguments)]
pub(super) fn prepaint_zoom_scroll_offset(
    anchor: ViewportCenterAnchor,
    page_count: usize,
    paper_width: f32,
    paper_height: f32,
    mode: ScriptViewMode,
    page_content_bounds: impl ExactSizeIterator<Item = (f32, f32)> + Clone,
    base_font_size: f32,
    zoom_percent: u16,
    outer_rect: egui::Rect,
    vertical_item_spacing: f32,
) -> Option<PrepaintZoomPlan> {
    if anchor.page_index >= page_count
        || page_count == 0
        || page_content_bounds.len() != page_count
        || !paper_width.is_finite()
        || !paper_height.is_finite()
        || !base_font_size.is_finite()
        || !vertical_item_spacing.is_finite()
        || page_content_bounds
            .clone()
            .any(|(top, height)| !top.is_finite() || !height.is_finite())
    {
        return None;
    }
    // egui's overlay scrollbars do not consume the ScrollArea inner viewport.
    // Keep planning dimensions identical to the rect that will paint this frame.
    let inner_size = outer_rect.size().max(Vec2::ZERO);
    let metrics =
        presentation_metrics_at_zoom(inner_size.x, paper_width, base_font_size, zoom_percent);
    let geometry = page_scroll_geometry(inner_size.x, metrics.page_width);
    if !metrics.scale.is_finite() || metrics.scale <= 0.0 {
        return None;
    }
    let mut page_origins = Vec::with_capacity(page_count);
    let mut page_heights = Vec::with_capacity(page_count);
    let mut page_strides = Vec::with_capacity(page_count);
    let mut next_page_top = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING;
    let mut content_height = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING
        + SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING;
    let mut target_policy = None;
    for (page_index, (content_top, content_page_height)) in
        page_content_bounds.enumerate()
    {
        let policy = presented_page_policy(
            mode,
            paper_height,
            content_top,
            content_page_height,
            metrics.scale,
            vertical_item_spacing,
        );
        if !policy.page_height.is_finite()
            || policy.page_height <= 0.0
            || !policy.document_y_offset.is_finite()
            || !policy.explicit_gap.is_finite()
            || !policy.vertical_item_spacing.is_finite()
        {
            return None;
        }
        if page_index == anchor.page_index {
            target_policy = Some((policy, content_top, content_page_height));
        }
        let page_stride =
            policy.page_height + policy.explicit_gap + policy.vertical_item_spacing;
        if !page_stride.is_finite() || page_stride <= 0.0 {
            return None;
        }
        page_origins.push(next_page_top);
        page_heights.push(policy.page_height);
        page_strides.push(page_stride);
        next_page_top += page_stride;
        content_height += page_stride;
    }
    let (target_policy, target_content_top, target_content_height) = target_policy?;
    let content_size = Vec2::new(
        geometry.content_width,
        content_height,
    );
    if !content_size.x.is_finite() || !content_size.y.is_finite() {
        return None;
    }
    let target_document_y = if mode == ScriptViewMode::Speed {
        anchor
            .page_local_document_point
            .y
            .clamp(target_content_top, target_content_top + target_content_height)
    } else {
        anchor.page_local_document_point.y.clamp(0.0, paper_height)
    };
    let target_presented_y = target_document_y - target_policy.document_y_offset;
    let content_target = outer_rect.min
        + Vec2::new(geometry.leading_padding, page_origins[anchor.page_index])
        + Vec2::new(
            anchor.page_local_document_point.x,
            target_presented_y,
        ) * metrics.scale
        + anchor.outside_page_offset_points;
    let viewport_center = outer_rect.min + inner_size * 0.5;
    let requested_offset = content_target - viewport_center;
    let max_offset = (content_size - inner_size).max(Vec2::ZERO);
    (requested_offset.x.is_finite() && requested_offset.y.is_finite()).then(|| {
        let scroll_offset = Vec2::new(
            requested_offset.x.clamp(0.0, max_offset.x),
            requested_offset.y.clamp(0.0, max_offset.y),
        );
        PrepaintZoomPlan {
            page_origins,
            page_heights,
            page_strides,
            content_size,
            scroll_offset,
        }
    })
}
