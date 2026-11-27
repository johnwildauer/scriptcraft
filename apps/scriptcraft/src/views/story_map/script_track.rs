//! Read-only Script-track scene and Body-focus caret paint.

use eframe::egui::{self, FontId, Pos2, Rect, Stroke, Vec2};
use scriptcraft_layout::{SceneProjection, StoryMapProjection};
use scriptcraft_model::Project;

use crate::{
    chrome::theme::{PagePalette, StoryMapPalette},
    presentation::document_font::PreparedDocumentFonts,
    presentation::span_paint::font_for_active_family,
};

use super::{
    constants::{
        SCENE_BLOCK_CORNER_RADIUS, SCENE_BLOCK_HORIZONTAL_INSET, SCENE_BLOCK_LEFT_EDGE_WIDTH,
        SCENE_BLOCK_VERTICAL_INSET, SCENE_CARET_STROKE_DEVICE_PIXELS,
    },
    geometry::{visible_interval_screen_bounds, StripGeometry, TimelineGeometry},
    style,
};

pub(crate) fn paint(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    palette: StoryMapPalette,
    page_palette: PagePalette,
    project: &Project,
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
            paint_scene(
                ui,
                script_clip,
                strip,
                timeline,
                pixels_per_point,
                scene,
                palette,
                page_palette,
                project,
                prepared_fonts,
            );
        }
    }
}

fn paint_scene(
    ui: &egui::Ui,
    clip: Rect,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    scene: &SceneProjection,
    palette: StoryMapPalette,
    page_palette: PagePalette,
    project: &Project,
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
    if let Some(start_x) = timeline
        .page_to_screen(scene.start_page)
        .filter(|x| x.is_finite() && *x >= f64::from(clip.left()) && *x <= f64::from(clip.right()))
        .and_then(|x| snap_to_device_pixel_boundary(x, pixels_per_point))
    {
        painter.line_segment(
            [Pos2::new(start_x, block.top()), Pos2::new(start_x, block.bottom())],
            Stroke::new(SCENE_BLOCK_LEFT_EDGE_WIDTH, palette.scene_border),
        );
    }

    let Some(owner) = project.elements.get(scene.element_index) else {
        return;
    };
    let owner = owner.as_element();
    if owner.id != scene.owner_id {
        return;
    }
    let heading_palette = PagePalette { text: palette.scene_heading, ..page_palette };
    let definition = project.element_definitions().get(&owner.element_type);
    let all_caps = definition.is_some_and(|definition| definition.font.all_caps);
    let resolved = definition
        .and_then(|definition| {
            prepared_fonts.and_then(|prepared| {
                style::resolve_definition_style(definition, prepared, heading_palette)
            })
        })
        .filter(|resolved| font_for_active_family(ui, &resolved.font) == resolved.font)
        .or_else(|| {
            prepared_fonts
                .and_then(|prepared| style::neutral_courier_fallback(prepared, heading_palette))
                .filter(|resolved| font_for_active_family(ui, &resolved.font) == resolved.font)
        })
        .unwrap_or_else(|| style::generic_fallback(heading_palette));

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
    let display_heading = if all_caps {
        scene.heading.chars().flat_map(char::to_uppercase).collect::<String>()
    } else {
        scene.heading.clone()
    };
    let font = font_for_active_family(ui, &resolved.font);
    paint_heading(
        ui,
        block,
        &display_heading,
        heading_x as f32,
        available_width as f32,
        font,
        resolved.color,
    );
}

fn paint_heading(
    ui: &egui::Ui,
    block: Rect,
    heading: &str,
    heading_x: f32,
    available_width: f32,
    font: FontId,
    color: eframe::egui::Color32,
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
    let painter = ui.painter().with_clip_rect(block);
    let full = painter.layout_no_wrap(heading.to_owned(), font.clone(), color);
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
            let candidate = painter.layout_no_wrap(heading[..*end].to_owned(), font.clone(), color);
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
    painter.galley(position, galley, color);
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

pub(crate) fn paint_caret(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    snapshot: Option<&StoryMapProjection>,
    palette: StoryMapPalette,
) {
    let Some(caret_page) = snapshot
        .and_then(|snapshot| snapshot.caret_page)
        .filter(|page| page.is_finite() && *page >= 0.0)
    else {
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
        .and_then(|x| snap_to_device_pixel_boundary(x, pixels_per_point))
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

fn snap_to_device_pixel_boundary(x: f64, pixels_per_point: f64) -> Option<f32> {
    if !x.is_finite() || !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    let device_position = x * pixels_per_point;
    if !device_position.is_finite() {
        return None;
    }
    let snapped = device_position.round() / pixels_per_point;
    (snapped.is_finite() && snapped >= f64::from(f32::MIN) && snapped <= f64::from(f32::MAX))
        .then_some(snapped as f32)
}

fn finite_size(size: Vec2) -> bool {
    size.x.is_finite() && size.y.is_finite()
}
