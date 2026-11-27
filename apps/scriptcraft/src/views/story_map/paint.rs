use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use scriptcraft_model::{BeatCard, StoryMapLane};

use crate::chrome::theme::{ApplicationPalette, StoryMapPalette};

use super::{
    constants::{
        LANE_LABEL_FONT_SIZE, LANE_LABEL_INSET, STATIC_BEAT_BORDER_WIDTH,
        STATIC_BEAT_CORNER_RADIUS, STATIC_BEAT_HORIZONTAL_INSET, STATIC_BEAT_TITLE_FONT_SIZE,
        STORY_MAP_BEAT_GHOST_BORDER_WIDTH, STORY_MAP_BEAT_GHOST_CORNER_RADIUS,
    },
    geometry::{interval_intersects_visible_range, placed_beat_rect, StripGeometry, TimelineGeometry},
    interaction::{BeatGroupRole, BeatInteractionOutput},
};

pub(crate) fn paint_shell(
    ui: &egui::Ui,
    geometry: &StripGeometry,
    lane_labels: &[String],
    palette: ApplicationPalette,
    resize_hovered: bool,
) {
    let story_palette = palette.story_map;
    let painter = ui.painter().with_clip_rect(geometry.rect);
    painter.rect_filled(geometry.rect, 0.0, story_palette.strip_background);
    painter.rect_filled(geometry.label_column, 0.0, story_palette.label_background);

    for (row, label) in geometry.lane_rows.iter().zip(lane_labels) {
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
        paint_label(ui, label_cell, label, story_palette.label_foreground);
        paint_row_separator(ui, *row, story_palette.separator);
    }
    paint_row_separator(ui, geometry.ruler_row, story_palette.separator);
    let script_label_cell = geometry.rect.intersect(Rect::from_min_max(
        Pos2::new(geometry.label_column.left(), geometry.script_row.top()),
        Pos2::new(geometry.label_column.right(), geometry.script_row.bottom()),
    ));
    paint_label(
        ui,
        script_label_cell,
        "Script",
        story_palette.text_foreground,
    );
    paint_row_separator(ui, geometry.script_row, story_palette.separator);
    paint_row_separator(ui, geometry.scrollbar_row, story_palette.separator);

    let resize_color = if resize_hovered {
        palette.status.hover_foreground
    } else {
        story_palette.separator
    };
    if geometry.resize_band.is_positive() {
        painter.line_segment(
            [
                Pos2::new(geometry.rect.left(), geometry.rect.bottom() - 0.5),
                Pos2::new(geometry.rect.right(), geometry.rect.bottom() - 0.5),
            ],
            Stroke::new(1.0, resize_color),
        );
    }
}

pub(crate) fn paint_static_beats(
    ui: &egui::Ui,
    strip: &StripGeometry,
    lanes: &[StoryMapLane],
    cards: &[BeatCard],
    target_pages: u32,
    timeline: &TimelineGeometry,
    palette: StoryMapPalette,
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
        let Some((lane_index, _)) = matching_lanes.next() else {
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
        visible.rect_stroke(
            block,
            STATIC_BEAT_CORNER_RADIUS,
            Stroke::new(STATIC_BEAT_BORDER_WIDTH, palette.beat_border),
            StrokeKind::Inside,
        );
        paint_static_title(ui, block, &card.title, palette.beat_title);
    }
}

pub(crate) fn paint_group_preview(
    ui: &egui::Ui,
    strip: &StripGeometry,
    lanes: &[StoryMapLane],
    cards: &[BeatCard],
    output: &BeatInteractionOutput,
    target_pages: u32,
    timeline: &TimelineGeometry,
    palette: StoryMapPalette,
) {
    let clip = strip.timeline_viewport.intersect(strip.rect).intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);
    for ghost in &output.ghosts {
        let mut matching = lanes.iter().enumerate().filter(|(_, lane)| lane.uuid == ghost.lane_uuid);
        let Some((lane_index, _)) = matching.next() else { continue };
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
        paint_static_title(ui, block, &card.title, title);
    }
}

fn paint_static_title(ui: &egui::Ui, block: Rect, title: &str, color: Color32) {
    if title.is_empty() || !block.is_positive() || !ui.is_rect_visible(block) {
        return;
    }
    let title_rect = block
        .shrink2(Vec2::new(STATIC_BEAT_HORIZONTAL_INSET, 0.0))
        .intersect(block);
    if !title_rect.is_positive() {
        return;
    }
    let font = FontId::proportional(STATIC_BEAT_TITLE_FONT_SIZE);
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

fn paint_label(ui: &egui::Ui, cell: Rect, label: &str, color: Color32) {
    if !cell.is_positive() {
        return;
    }
    let painter = ui.painter().with_clip_rect(cell);
    painter.text(
        Pos2::new(cell.left() + LANE_LABEL_INSET, cell.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(LANE_LABEL_FONT_SIZE),
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
