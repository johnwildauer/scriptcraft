//! Read-only card paint using canonical line and decoration projections.

use super::{cache::CardLayouts, geometry::{pixels, scale, ViewportGeometry}, text_state::BeatTextState};
use crate::{
    chrome::theme::PagePalette,
    presentation::{document_font::PreparedDocumentFonts, geometry, span_paint::resolve_span_paint},
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use scriptcraft_layout::{BeatCardLayout, FixedRect, LayoutLine};
use scriptcraft_model::BeatCard;

const CARD_BORDER_WIDTH: f32 = 2.5;
const SELECTED_BORDER_WIDTH: f32 = 3.0;
const SELECTED_BORDER_COLOR: Color32 = Color32::from_rgb(112, 190, 255);

fn card_rect(geometry: ViewportGeometry, card: &BeatCard) -> Rect {
    let x = pixels(card.rect.left);
    let y = pixels(card.rect.top);
    Rect::from_min_max(
        geometry.board_to_screen(x, y),
        geometry.board_to_screen(x + pixels(card.rect.width), y + pixels(card.rect.height)),
    )
}

pub(super) fn paint_card_frame(painter: &egui::Painter, geometry: ViewportGeometry, card: &BeatCard, palette: PagePalette, selected: bool) {
    let rect = card_rect(geometry, card);
    painter.rect_filled(rect, 0.0, palette.paper);
    let stroke = if selected {
        Stroke::new(SELECTED_BORDER_WIDTH, SELECTED_BORDER_COLOR)
    } else {
        Stroke::new(CARD_BORDER_WIDTH, palette.border)
    };
    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
}

fn local_rect(origin: Pos2, rect: FixedRect, zoom: f32) -> Rect {
    Rect::from_min_size(
        origin + egui::vec2(rect.x.to_points_f32() * zoom, rect.y.to_points_f32() * zoom),
        egui::vec2(rect.width.to_points_f32() * zoom, rect.height.to_points_f32() * zoom),
    )
}

fn paint_line(
    painter: &egui::Painter,
    origin: Pos2,
    line: &LayoutLine,
    zoom: f32,
    palette: PagePalette,
    fonts: &PreparedDocumentFonts,
    subdued: bool,
) -> bool {
    let Ok(projection) = geometry::project_line(line, zoom) else { return false };
    if !projection.metric_cull_bounds.translate(origin.to_vec2()).intersects(painter.clip_rect()) {
        return true;
    }
    for span in &projection.spans {
        let Some(resolved) = resolve_span_paint(
            &span.core.resolved_text_style,
            span.core.resolved_text_style.size.to_points_f32() * zoom,
            palette,
            fonts,
        ) else { return false };
        let color = if subdued { palette.subdued_text } else { resolved.color };
        let Ok(galley) = geometry::canonical_singleline_galley(
            painter, &span.core.display_text, &resolved.font, &span.display_boundary_offsets,
            color, projection.line_box_height, projection.baseline_y - projection.line_box_top,
        ) else { return false };
        painter.galley(origin + egui::vec2(span.origin_x, projection.line_box_top), galley, color);
        for decoration in [span.core.underline_geometry, span.core.strikethrough_geometry].into_iter().flatten() {
            let Ok(rect) = geometry::project_decoration_rect(span, decoration, projection.baseline_y, zoom) else { return false };
            if rect.is_positive() {
                painter.rect_filled(rect.translate(origin.to_vec2()), 0.0, color);
            }
        }
    }
    true
}

fn paint_title(
    painter: &egui::Painter,
    origin: Pos2,
    layout: &BeatCardLayout,
    zoom: f32,
    palette: PagePalette,
    fonts: &PreparedDocumentFonts,
    subdued: bool,
) -> bool {
    let Some(truncated) = &layout.truncated_title else {
        return paint_line(painter, origin, &layout.title, zoom, palette, fonts, subdued);
    };
    if !paint_line(painter, origin, &truncated.prefix, zoom, palette, fonts, subdued) {
        return false;
    }
    let Ok(projection) = geometry::project_line(&truncated.prefix, zoom) else { return false };
    let ellipsis = &truncated.ellipsis;
    let Some(resolved) = resolve_span_paint(&ellipsis.resolved_text_style,
        ellipsis.resolved_text_style.size.to_points_f32() * zoom, palette, fonts) else { return false };
    let color = if subdued { palette.subdued_text } else { resolved.color };
    let boundaries = [0.0, ellipsis.geometry.width.to_points_f32() * zoom];
    let Ok(galley) = geometry::canonical_singleline_galley(painter, ellipsis.text, &resolved.font,
        &boundaries, color, projection.line_box_height, projection.baseline_y - projection.line_box_top)
        else { return false };
    painter.galley(origin + egui::vec2(ellipsis.geometry.x.to_points_f32() * zoom, projection.line_box_top), galley, color);
    true
}

pub(super) fn paint_card(
    painter: &egui::Painter,
    geometry: ViewportGeometry,
    card: &BeatCard,
    layouts: &CardLayouts,
    palette: PagePalette,
    fonts: &PreparedDocumentFonts,
    text_state: &BeatTextState,
) -> bool {
    let rect = card_rect(geometry, card);
    let origin = rect.min;
    let zoom = scale(geometry.view) as f32;
    let card_clip = painter.clip_rect().intersect(rect);
    let title_clip = card_clip.intersect(local_rect(origin, layouts.card.title_rect, zoom));
    let body_clip = card_clip.intersect(local_rect(origin, layouts.card.body_rect, zoom));
    let mut success = true;
    if title_clip.is_positive() {
        let is_placeholder = card.title.is_empty();
        let editing_title = text_state.focused && text_state.field.as_ref().is_some_and(|(id, field)| id == &card.id && *field == scriptcraft_session::editing::BeatTextField::Title);
        if editing_title && !is_placeholder {
            success &= paint_line(&painter.with_clip_rect(title_clip), origin - egui::vec2(text_state.title_offset * zoom, 0.0), &layouts.card.title, zoom, palette, fonts, false);
        } else {
            let title = if is_placeholder { layouts.placeholder.as_ref().unwrap_or(&layouts.card) } else { &layouts.card };
            success &= paint_title(&painter.with_clip_rect(title_clip), origin, title, zoom, palette, fonts, is_placeholder);
        }
    }
    if body_clip.is_positive() {
        let is_placeholder = layouts.card.body_is_empty;
        let body = if is_placeholder { layouts.placeholder.as_ref().unwrap_or(&layouts.card) } else { &layouts.card };
        let body_painter = painter.with_clip_rect(body_clip);
        let active_body = text_state.focused && text_state.field.as_ref().is_some_and(|(id, field)| id == &card.id && *field == scriptcraft_session::editing::BeatTextField::Body);
        for line in body.body_lines.iter() {
            let translated = line.geometry.y.to_points_f32() - if active_body { text_state.body_offset } else { 0.0 };
            if translated + line.geometry.height.to_points_f32() <= body.body_rect.y.to_points_f32()
                || translated >= body.body_rect.bottom().to_points_f32() { continue; }
            success &= paint_line(&body_painter, origin + egui::vec2(0.0, -if active_body { text_state.body_offset * zoom } else { 0.0 }), line, zoom, palette, fonts, is_placeholder);
            if is_placeholder { break; }
        }
    }
    success
}
