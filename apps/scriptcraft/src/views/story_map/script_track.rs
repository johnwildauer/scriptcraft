//! Read-only Script-track scene and Body-focus caret paint.

use eframe::egui::{self, Pos2, Rect, Stroke, StrokeKind, Vec2};
use scriptcraft_layout::{SceneProjection, StoryMapProjection};

use crate::{
    chrome::theme::StoryMapPalette,
    presentation::{document_font::PreparedDocumentFonts, span_paint::neutral_resolved_font},
};

use super::{
    constants::{
        SCENE_BLOCK_BORDER_WIDTH, SCENE_BLOCK_CORNER_RADIUS, SCENE_BLOCK_HORIZONTAL_INSET,
        SCENE_BLOCK_TITLE_FONT_SIZE, SCENE_BLOCK_VERTICAL_INSET,
        SCENE_CARET_STROKE_DEVICE_PIXELS,
    },
    geometry::{visible_interval_screen_bounds, StripGeometry, TimelineGeometry},
};

pub(crate) fn paint(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    palette: StoryMapPalette,
    snapshot: Option<&StoryMapProjection>,
    prepared_fonts: Option<&PreparedDocumentFonts>,
) {
    let Some(snapshot) = snapshot else {
        return;
    };

    let script_clip = strip
        .script_row
        .intersect(strip.timeline_viewport)
        .intersect(strip.rect)
        .intersect(ui.clip_rect());
    if script_clip.is_positive() && ui.is_rect_visible(script_clip) {
        for scene in &snapshot.scenes {
            paint_scene(ui, script_clip, strip, timeline, scene, palette, prepared_fonts);
        }
    }

    paint_caret(
        ui,
        strip,
        timeline,
        pixels_per_point,
        snapshot.caret_page,
        palette,
    );
}

fn paint_scene(
    ui: &egui::Ui,
    clip: Rect,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    scene: &SceneProjection,
    palette: StoryMapPalette,
    prepared_fonts: Option<&PreparedDocumentFonts>,
) {
    let Some(block) = visible_scene_anchor(strip, timeline, scene, clip) else {
        return;
    };
    if !ui.is_rect_visible(block) {
        return;
    }

    let painter = ui.painter().with_clip_rect(clip);
    painter.rect_filled(block, SCENE_BLOCK_CORNER_RADIUS, palette.scene_fill);
    painter.rect_stroke(
        block,
        SCENE_BLOCK_CORNER_RADIUS,
        Stroke::new(SCENE_BLOCK_BORDER_WIDTH, palette.scene_border),
        StrokeKind::Inside,
    );
    if let Some(prepared_fonts) = prepared_fonts {
        let (Some(start_x), Some(end_x)) = (
            timeline.page_to_screen(scene.start_page),
            timeline.page_to_screen(scene.end_page),
        ) else {
            return;
        };
        let heading_x = start_x + f64::from(SCENE_BLOCK_HORIZONTAL_INSET);
        let available_width = end_x - start_x - 2.0 * f64::from(SCENE_BLOCK_HORIZONTAL_INSET);
        if !heading_x.is_finite()
            || !available_width.is_finite()
            || heading_x < f64::from(f32::MIN)
            || heading_x > f64::from(f32::MAX)
            || available_width <= 0.0
            || available_width > f64::from(f32::MAX)
        {
            return;
        }
        paint_heading(
            ui,
            block,
            &scene.heading,
            heading_x as f32,
            available_width as f32,
            palette,
            prepared_fonts,
        );
    }
}

/// The positively clipped visible scene rectangle shared by scene paint and
/// primary-click hit testing. It consumes the retained WO-302 transform inputs.
pub(crate) fn visible_scene_anchor(
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    scene: &SceneProjection,
    clip: Rect,
) -> Option<Rect> {
    let (left, right) = visible_interval_screen_bounds(
        timeline,
        scene.start_page,
        scene.end_page,
        clip,
    )?;
    let row = strip.script_row.intersect(strip.rect);
    if !row.is_positive() {
        return None;
    }
    let vertical_inset = SCENE_BLOCK_VERTICAL_INSET.min(row.height() * 0.5);
    let top = row.top() + vertical_inset;
    let bottom = row.bottom() - vertical_inset;
    if !top.is_finite() || !bottom.is_finite() || bottom <= top {
        return None;
    }
    let block = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom))
        .intersect(clip);
    block.is_positive().then_some(block)
}

fn paint_heading(
    ui: &egui::Ui,
    block: Rect,
    heading: &str,
    heading_x: f32,
    available_width: f32,
    palette: StoryMapPalette,
    prepared_fonts: &PreparedDocumentFonts,
) {
    if heading.is_empty()
        || heading.split_whitespace().next().is_none()
        || !block.is_positive()
        || !heading_x.is_finite()
        || !available_width.is_finite()
        || available_width <= 0.0
    {
        return;
    }
    let Some(font) = neutral_resolved_font(SCENE_BLOCK_TITLE_FONT_SIZE, prepared_fonts) else {
        return;
    };
    let resolved_family = font.family.clone();
    let courier_is_registered = ui.fonts(|fonts| {
        let definitions = fonts.definitions();
        definitions
            .families
            .get(&resolved_family)
            .is_some_and(|keys| keys.iter().any(|key| definitions.font_data.contains_key(key)))
    });
    if !courier_is_registered {
        return;
    }
    let painter = ui.painter().with_clip_rect(block);
    let full = painter.layout_no_wrap(heading.to_owned(), font.clone(), palette.scene_heading);
    let full_size = full.size();
    if !finite_size(full_size) {
        return;
    }
    let galley = if full_size.x <= available_width {
        full
    } else {
        let ends = word_ends(heading);
        let mut fitted = None;
        for end in ends.iter().rev().filter(|end| **end < heading.len()) {
            let candidate = painter.layout_no_wrap(
                heading[..*end].to_owned(),
                font.clone(),
                palette.scene_heading,
            );
            let size = candidate.size();
            if !finite_size(size) {
                return;
            }
            if size.x <= available_width {
                fitted = Some(candidate);
                break;
            }
        }
        let Some(fitted) = fitted else {
            return;
        };
        fitted
    };
    let size = galley.size();
    if !finite_size(size)
        || size.x <= 0.0
        || size.y <= 0.0
        || size.x > available_width
        || size.y > block.height()
    {
        return;
    }
    let position = Pos2::new(heading_x, block.center().y - size.y * 0.5);
    if !position.x.is_finite() || !position.y.is_finite() {
        return;
    }
    painter.galley(position, galley, palette.scene_heading);
}

fn word_ends(text: &str) -> Vec<usize> {
    let mut ends = Vec::new();
    let mut word_end = None;
    for (index, character) in text.char_indices() {
        if character.is_whitespace() {
            if let Some(end) = word_end.take() {
                ends.push(end);
            }
        } else {
            word_end = Some(index + character.len_utf8());
        }
    }
    if let Some(end) = word_end {
        ends.push(end);
    }
    ends
}

fn paint_caret(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    caret_page: Option<f64>,
    palette: StoryMapPalette,
) {
    let Some(caret_page) = caret_page.filter(|page| page.is_finite() && *page >= 0.0) else {
        return;
    };
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return;
    }
    let Some((visible_start, visible_end)) = timeline.visible_page_range() else {
        return;
    };
    if caret_page < visible_start || caret_page > visible_end || caret_page > timeline.domain_pages {
        return;
    }
    let content_top = strip
        .lane_rows
        .first()
        .map_or(strip.ruler_row.top(), |row| row.top());
    let content = Rect::from_min_max(
        Pos2::new(strip.timeline_viewport.left(), content_top),
        Pos2::new(strip.timeline_viewport.right(), strip.script_row.bottom()),
    )
    .intersect(strip.timeline_viewport)
    .intersect(strip.rect)
    .intersect(ui.clip_rect());
    if !content.is_positive() || !ui.is_rect_visible(content) {
        return;
    }
    let Some(x) = timeline
        .page_to_screen(caret_page)
        .and_then(|x| snap_to_device_pixel_center(x, pixels_per_point))
    else {
        return;
    };
    if x < content.left() || x > content.right() {
        return;
    }
    let stroke_width = SCENE_CARET_STROKE_DEVICE_PIXELS / pixels_per_point;
    if !stroke_width.is_finite() || stroke_width <= 0.0 || stroke_width > f64::from(f32::MAX) {
        return;
    }
    ui.painter().with_clip_rect(content).line_segment(
        [Pos2::new(x, content.top()), Pos2::new(x, content.bottom())],
        Stroke::new(stroke_width as f32, palette.scene_caret),
    );
}

fn snap_to_device_pixel_center(x: f64, pixels_per_point: f64) -> Option<f32> {
    if !x.is_finite() || !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    let center = (x * pixels_per_point - 0.5).round() + 0.5;
    let snapped = center / pixels_per_point;
    (snapped.is_finite() && snapped >= f64::from(f32::MIN) && snapped <= f64::from(f32::MAX))
        .then_some(snapped as f32)
}

fn finite_size(size: Vec2) -> bool {
    size.x.is_finite() && size.y.is_finite()
}
