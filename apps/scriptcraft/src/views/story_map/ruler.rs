//! Read-only Page Ruler paint over the shared WO-202 timeline transform.

use eframe::egui::{self, FontId, Pos2, Rect, Stroke};

use super::{
    constants::{
        RULER_EIGHTH_TICK_HEIGHT, RULER_LABEL_DENSITY_EVERY_PAGE,
        RULER_LABEL_DENSITY_EVEN_PAGES, RULER_LABEL_DENSITY_FIFTH_PAGES,
        RULER_LABEL_FONT_SIZE, RULER_LABEL_INSET, RULER_LABEL_TOP_INSET,
        RULER_STROKE_DEVICE_PIXELS, RULER_WHOLE_PAGE_TICK_HEIGHT,
    },
    geometry::{StripGeometry, TimelineGeometry},
};
use crate::chrome::theme::StoryMapPalette;

#[derive(Clone, Copy)]
enum LabelDensity {
    EveryPage,
    EvenPages,
    FifthPages,
    TenthPages,
}

fn label_density(effective_page_pixels: f64) -> Option<LabelDensity> {
    if !effective_page_pixels.is_finite() || effective_page_pixels <= 0.0 {
        return None;
    }
    Some(if effective_page_pixels >= RULER_LABEL_DENSITY_EVERY_PAGE {
        LabelDensity::EveryPage
    } else if effective_page_pixels >= RULER_LABEL_DENSITY_EVEN_PAGES {
        LabelDensity::EvenPages
    } else if effective_page_pixels >= RULER_LABEL_DENSITY_FIFTH_PAGES {
        LabelDensity::FifthPages
    } else {
        LabelDensity::TenthPages
    })
}

fn show_page(density: LabelDensity, page: u64) -> bool {
    match density {
        LabelDensity::EveryPage => true,
        LabelDensity::EvenPages => page % 2 == 0,
        LabelDensity::FifthPages => page == 1 || page % 5 == 0,
        LabelDensity::TenthPages => page == 1 || page % 10 == 0,
    }
}

fn bounded_u64(value: f64) -> Option<u64> {
    (value.is_finite() && value >= 0.0 && value < u64::MAX as f64).then_some(value as u64)
}

fn snap_to_device_pixel_center(x: f64, pixels_per_point: f64) -> Option<f32> {
    if !x.is_finite() || !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    let device_center = (x * pixels_per_point - 0.5).round() + 0.5;
    let snapped = device_center / pixels_per_point;
    (snapped.is_finite() && snapped >= f64::from(f32::MIN) && snapped <= f64::from(f32::MAX))
        .then_some(snapped as f32)
}

fn paint_ticks(
    ui: &egui::Ui,
    clip: Rect,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    palette: StoryMapPalette,
) {
    let Some((visible_start, visible_end)) = timeline.visible_page_range() else {
        return;
    };
    let Some(first) = bounded_u64((visible_start * 8.0).ceil()) else {
        return;
    };
    let Some(last) = bounded_u64((visible_end.min(timeline.domain_pages) * 8.0).floor()) else {
        return;
    };
    if first > last {
        return;
    }

    let stroke_width = (RULER_STROKE_DEVICE_PIXELS / pixels_per_point) as f32;
    if !stroke_width.is_finite() || stroke_width <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(clip);
    let bottom = clip.bottom();
    let mut eighth = first;
    loop {
        let page_position = eighth as f64 / 8.0;
        if let Some(screen_x) = timeline
            .page_to_screen(page_position)
            .and_then(|x| snap_to_device_pixel_center(x, pixels_per_point))
        {
            let x = f64::from(screen_x);
            if x >= f64::from(clip.left()) && x <= f64::from(clip.right()) {
                let whole_page = eighth % 8 == 0;
                let height = if whole_page {
                    RULER_WHOLE_PAGE_TICK_HEIGHT
                } else {
                    RULER_EIGHTH_TICK_HEIGHT
                };
                let color = if page_position >= timeline.target_pages {
                    palette.overflow_ruler_mark
                } else {
                    palette.ruler_mark
                };
                painter.line_segment(
                    [Pos2::new(screen_x, bottom - height), Pos2::new(screen_x, bottom)],
                    Stroke::new(stroke_width, color),
                );
            }
        }
        if eighth == last {
            break;
        }
        let Some(next) = eighth.checked_add(1) else {
            break;
        };
        eighth = next;
    }
}

fn paint_labels(
    ui: &egui::Ui,
    clip: Rect,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    palette: StoryMapPalette,
) {
    let Some(density) = label_density(timeline.page_pitch * pixels_per_point) else {
        return;
    };
    let Some((visible_start, visible_end)) = timeline.visible_page_range() else {
        return;
    };
    let Some(first_page_start) = bounded_u64(visible_start.floor()) else {
        return;
    };
    let last_page_start_value = visible_end.min(timeline.domain_pages).floor();
    let Some(last_page_start) = bounded_u64(last_page_start_value) else {
        return;
    };
    let Some(max_page_number) = bounded_u64(timeline.domain_pages.ceil()) else {
        return;
    };
    if first_page_start > last_page_start {
        return;
    }

    let painter = ui.painter().with_clip_rect(clip);
    let font = FontId::proportional(RULER_LABEL_FONT_SIZE);
    let mut page_start = first_page_start;
    loop {
        let Some(page) = page_start.checked_add(1) else {
            break;
        };
        let page_position = page_start as f64;
        if page <= max_page_number
            && page_position < timeline.domain_pages
            && show_page(density, page)
        {
            if let Some(x) = timeline.page_to_screen(page_position) {
                let label = page.to_string();
                let color = if page_position >= timeline.target_pages {
                    palette.overflow_ruler_mark
                } else {
                    palette.ruler_mark
                };
                let galley = painter.layout_no_wrap(label, font.clone(), color);
                let size = galley.size();
                let left = x + f64::from(RULER_LABEL_INSET);
                let right = left + f64::from(size.x);
                let top = clip.top() + RULER_LABEL_TOP_INSET;
                let bottom = top + size.y;
                if size.x.is_finite()
                    && size.y.is_finite()
                    && left.is_finite()
                    && right.is_finite()
                    && left >= f64::from(clip.left())
                    && right <= f64::from(clip.right())
                    && bottom <= clip.bottom() - RULER_WHOLE_PAGE_TICK_HEIGHT
                {
                    painter.galley(Pos2::new(left as f32, top), galley, color);
                }
            }
        }
        if page_start == last_page_start {
            break;
        }
        let Some(next) = page_start.checked_add(1) else {
            break;
        };
        page_start = next;
    }
}

/// Paint visible eighth ticks and density-selected page labels inside the ruler
/// timeline cell only. Invalid device scale suppresses ruler paint without
/// changing navigation state.
pub(crate) fn paint(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    palette: StoryMapPalette,
) {
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return;
    }
    let clip = strip
        .ruler_row
        .intersect(strip.timeline_viewport)
        .intersect(strip.rect)
        .intersect(ui.clip_rect());
    if !clip.is_positive() || !ui.is_rect_visible(clip) {
        return;
    }
    paint_labels(ui, clip, timeline, pixels_per_point, palette);
    paint_ticks(ui, clip, timeline, pixels_per_point, palette);
}
