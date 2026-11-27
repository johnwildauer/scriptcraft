use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use scriptcraft_model::{BeatCard, Project, StoryMapLane};

use crate::chrome::theme::{ApplicationPalette, PagePalette, StoryMapPalette};
use crate::presentation::document_font::PreparedDocumentFonts;
use crate::presentation::span_paint::font_for_active_family;

use super::{
    constants::{
        LANE_LABEL_INSET, RULER_STROKE_DEVICE_PIXELS, STATIC_BEAT_CORNER_RADIUS,
        STATIC_BEAT_HORIZONTAL_INSET, STORY_MAP_BEAT_GHOST_BORDER_WIDTH,
        STORY_MAP_BEAT_GHOST_CORNER_RADIUS,
    },
    geometry::{interval_intersects_visible_range, placed_beat_rect, StripGeometry, TimelineGeometry},
    interaction::{BeatGroupRole, BeatInteractionOutput},
    style,
};

pub(crate) fn paint_shell(
    ui: &egui::Ui,
    geometry: &StripGeometry,
    project: &Project,
    prepared_fonts: Option<&PreparedDocumentFonts>,
    palette: ApplicationPalette,
) {
    let story_palette = palette.story_map;
    let painter = ui.painter().with_clip_rect(geometry.rect);
    painter.rect_filled(geometry.rect, 0.0, story_palette.strip_background);
    painter.rect_filled(geometry.label_column, 0.0, story_palette.label_background);

    for (row, lane) in geometry.lane_rows.iter().zip(&project.story_map().lanes) {
        let lane_timeline = geometry
            .timeline_viewport
            .intersect(*row)
            .intersect(geometry.rect);
        if lane_timeline.is_positive() {
            painter.rect_filled(lane_timeline, 0.0, story_palette.lane_background);
        }
        let label_cell = geometry.rect.intersect(Rect::from_min_max(
            Pos2::new(geometry.label_column.left(), row.top()),
            Pos2::new(geometry.label_column.right(), row.bottom()),
        ));
        let resolved = style::resolve_lane_style(
            project,
            lane,
            prepared_fonts,
            palette.script_workspace.page,
        );
        paint_label(ui, label_cell, &lane.label, resolved.font, resolved.color);
        paint_row_separator(ui, *row, story_palette.separator);
    }
    paint_row_separator(ui, geometry.ruler_row, story_palette.separator);
    let script_label_cell = geometry.rect.intersect(Rect::from_min_max(
        Pos2::new(geometry.label_column.left(), geometry.script_row.top()),
        Pos2::new(geometry.label_column.right(), geometry.script_row.bottom()),
    ));
    let script_style = style::resolve_scene_heading_style(
        project,
        prepared_fonts,
        palette.script_workspace.page,
    );
    paint_label(
        ui,
        script_label_cell,
        "Script",
        script_style.font,
        script_style.color,
    );
    paint_row_separator(ui, geometry.script_row, story_palette.separator);
    paint_row_separator(ui, geometry.scrollbar_row, story_palette.separator);

    if geometry.resize_band.is_positive() {
        painter.line_segment(
            [
                Pos2::new(geometry.rect.left(), geometry.rect.bottom() - 0.5),
                Pos2::new(geometry.rect.right(), geometry.rect.bottom() - 0.5),
            ],
            Stroke::new(1.0, story_palette.separator),
        );
    }
}

pub(crate) fn paint_dynamic_lane_grid(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    pixels_per_point: f64,
    palette: StoryMapPalette,
) {
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return;
    }
    let Some((visible_start, visible_end)) = timeline.visible_page_range() else {
        return;
    };
    let Some(first) = bounded_eighth_index((visible_start * 8.0).ceil()) else {
        return;
    };
    let Some(last) = bounded_eighth_index((visible_end.min(timeline.domain_pages) * 8.0).floor()) else {
        return;
    };
    if first > last {
        return;
    }
    let stroke_width = (RULER_STROKE_DEVICE_PIXELS / pixels_per_point) as f32;
    if !stroke_width.is_finite() || stroke_width <= 0.0 {
        return;
    }

    for row in &strip.lane_rows {
        let lane_clip = row
            .intersect(strip.timeline_viewport)
            .intersect(strip.rect)
            .intersect(ui.clip_rect());
        if !lane_clip.is_positive() {
            continue;
        }
        let painter = ui.painter().with_clip_rect(lane_clip);
        let mut eighth = first;
        loop {
            let page_position = eighth as f64 / 8.0;
            if let Some(screen_x) = timeline
                .page_to_screen(page_position)
                .and_then(|x| snap_to_device_pixel_center(x, pixels_per_point))
            {
                if screen_x >= lane_clip.left() && screen_x <= lane_clip.right() {
                    painter.line_segment(
                        [
                            Pos2::new(screen_x, lane_clip.top()),
                            Pos2::new(screen_x, lane_clip.bottom()),
                        ],
                        Stroke::new(stroke_width, palette.separator),
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
}

pub(crate) fn paint_static_beats(
    ui: &egui::Ui,
    strip: &StripGeometry,
    project: &Project,
    lanes: &[StoryMapLane],
    cards: &[BeatCard],
    target_pages: u32,
    timeline: &TimelineGeometry,
    palette: StoryMapPalette,
    page_palette: PagePalette,
    prepared_fonts: Option<&PreparedDocumentFonts>,
    suppressed_cards: &[scriptcraft_model::StableId],
) {
    let clip = strip
        .timeline_viewport
        .intersect(strip.rect)
        .intersect(ui.clip_rect());
    if !clip.is_positive() {
        return;
    }
    let visible = ui.painter().with_clip_rect(clip);
    for card in cards {
        if suppressed_cards.iter().any(|id| id == &card.id) {
            continue;
        }
        let Some(placement) = card.story_map_placement.as_ref() else {
            continue;
        };
        let mut matching_lanes = lanes
            .iter()
            .enumerate()
            .filter(|(_, lane)| lane.uuid == placement.lane_uuid);
        let Some((lane_index, lane)) = matching_lanes.next() else {
            continue;
        };
        if matching_lanes.next().is_some() {
            continue;
        }
        let Some(end_eighths) = placement.checked_end_eighths() else { continue };
        let start_page = placement.start_eighths as f64 / 8.0;
        let end_page = end_eighths as f64 / 8.0;
        if !interval_intersects_visible_range(timeline, start_page, end_page) {
            continue;
        }
        let Some(block) = placed_beat_rect(
            strip,
            lane_index,
            placement.start_eighths,
            placement.length_eighths,
            target_pages,
            timeline,
            clip,
        ) else { continue };
        if !block.is_positive() || !ui.is_rect_visible(block) {
            continue;
        }
        visible.rect_filled(block, STATIC_BEAT_CORNER_RADIUS, palette.beat_fill);
        let resolved = style::resolve_lane_style(project, lane, prepared_fonts, page_palette);
        paint_static_title(ui, block, &card.title, resolved.font, resolved.color);
    }
}

pub(crate) fn paint_group_preview(
    ui: &egui::Ui,
    strip: &StripGeometry,
    project: &Project,
    lanes: &[StoryMapLane],
    cards: &[BeatCard],
    output: &BeatInteractionOutput,
    target_pages: u32,
    timeline: &TimelineGeometry,
    palette: StoryMapPalette,
    page_palette: PagePalette,
    prepared_fonts: Option<&PreparedDocumentFonts>,
) {
    let clip = strip.timeline_viewport.intersect(strip.rect).intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);
    for ghost in &output.ghosts {
        let mut matching = lanes.iter().enumerate().filter(|(_, lane)| lane.uuid == ghost.lane_uuid);
        let Some((lane_index, lane)) = matching.next() else { continue };
        if matching.next().is_some() { continue; }
        let Some(block) = placed_beat_rect(
            strip, lane_index, ghost.start_eighths, ghost.length_eighths, target_pages, timeline, clip,
        ) else { continue };
        if !ui.is_rect_visible(block) { continue; }
        let Some(card) = cards.iter().find(|card| card.id == ghost.card_id) else { continue };
        let (fill, border, title) = match ghost.role {
            BeatGroupRole::Primary => (palette.beat_drag_fill, palette.beat_drag_border, palette.beat_drag_title),
            BeatGroupRole::CarriedFollower => (palette.beat_carried_fill, palette.beat_carried_border, palette.beat_carried_title),
            BeatGroupRole::Displaced => (palette.beat_displaced_fill, palette.beat_displaced_border, palette.beat_displaced_title),
            BeatGroupRole::PropagatedFollower => (palette.beat_propagated_fill, palette.beat_propagated_border, palette.beat_propagated_title),
            BeatGroupRole::Rejected => (palette.beat_rejected_fill, palette.beat_rejected_border, palette.beat_rejected_title),
        };
        painter.rect_filled(block, STORY_MAP_BEAT_GHOST_CORNER_RADIUS, fill);
        painter.rect_stroke(
            block,
            STORY_MAP_BEAT_GHOST_CORNER_RADIUS,
            Stroke::new(STORY_MAP_BEAT_GHOST_BORDER_WIDTH, border),
            StrokeKind::Inside,
        );
        let resolved = style::resolve_lane_style(project, lane, prepared_fonts, page_palette);
        paint_static_title(ui, block, &card.title, resolved.font, title);
    }
}

fn paint_static_title(ui: &egui::Ui, block: Rect, title: &str, font: FontId, color: Color32) {
    if title.is_empty() || !block.is_positive() || !ui.is_rect_visible(block) {
        return;
    }
    let title_rect = block
        .shrink2(Vec2::new(STATIC_BEAT_HORIZONTAL_INSET, 0.0))
        .intersect(block);
    if !title_rect.is_positive() {
        return;
    }
    let font = font_for_active_family(ui, &font);
    let painter = ui.painter().with_clip_rect(block);
    let complete = painter.layout_no_wrap(
        title.to_owned(),
        font.clone(),
        color,
    );
    let complete_size = complete.size();
    if !complete_size.x.is_finite() || !complete_size.y.is_finite() {
        return;
    }
    let galley = if complete_size.x <= title_rect.width() {
        complete
    } else {
        let mut words: Vec<&str> = title.split_whitespace().collect();
        let mut fitted = None;
        while !words.is_empty() {
            let candidate = words.join(" ");
            let candidate_galley = painter.layout_no_wrap(
                candidate,
                font.clone(),
                color,
            );
            let size = candidate_galley.size();
            if !size.x.is_finite() || !size.y.is_finite() {
                return;
            }
            if size.x <= title_rect.width() {
                fitted = Some(candidate_galley);
                break;
            }
            words.pop();
        }
        let Some(fitted) = fitted else {
            return;
        };
        fitted
    };
    let size = galley.size();
    if !size.x.is_finite()
        || !size.y.is_finite()
        || size.x <= 0.0
        || size.y <= 0.0
        || size.x > title_rect.width()
        || size.y > block.height()
    {
        return;
    }
    let position = Pos2::new(title_rect.left(), title_rect.center().y - size.y * 0.5);
    painter.galley(position, galley, color);
}

fn bounded_eighth_index(value: f64) -> Option<u64> {
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

pub(crate) fn paint_zoom_slider_knob(
    ui: &egui::Ui,
    clip_rect: Rect,
    response: &egui::Response,
    value: f64,
    minimum: f64,
    maximum: f64,
    palette: ApplicationPalette,
) {
    let clip_rect = clip_rect.intersect(ui.clip_rect());
    if !ui.is_rect_visible(clip_rect)
        || !value.is_finite()
        || !minimum.is_finite()
        || !maximum.is_finite()
        || maximum <= minimum
    {
        return;
    }
    let handle_radius = response.rect.height() / 2.5;
    let position_range = response.rect.x_range().shrink(handle_radius);
    let normalized = ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0) as f32;
    let center = Pos2::new(
        egui::emath::lerp(position_range, normalized),
        response.rect.center().y,
    );
    let visuals = ui.style().interact(response);
    let foreground = if response.hovered() {
        palette.status.slider_knob_hover
    } else {
        palette.status.slider_knob
    };
    ui.painter().with_clip_rect(clip_rect).circle(
        center,
        handle_radius + visuals.expansion,
        foreground,
        Stroke::new(visuals.fg_stroke.width, foreground),
    );
}

fn paint_label(ui: &egui::Ui, cell: Rect, label: &str, font: FontId, color: Color32) {
    if !cell.is_positive() {
        return;
    }
    let font = font_for_active_family(ui, &font);
    let painter = ui.painter().with_clip_rect(cell);
    painter.text(
        Pos2::new(cell.right() - LANE_LABEL_INSET, cell.center().y),
        Align2::RIGHT_CENTER,
        label,
        font,
        color,
    );
}

fn paint_row_separator(ui: &egui::Ui, row: Rect, color: Color32) {
    if row.is_positive() {
        ui.painter().with_clip_rect(row).line_segment(
            [Pos2::new(row.left(), row.bottom() - 0.5), Pos2::new(row.right(), row.bottom() - 0.5)],
            Stroke::new(1.0, color),
        );
    }
}
