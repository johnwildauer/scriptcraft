//! Canonical Beat selection, caret, and transient IME preedit painting.

use super::{geometry::{pixels, scale, ViewportGeometry}, text_state::BeatTextState};
use crate::chrome::theme::PagePalette;
use eframe::egui::{self, Pos2, Rect};
use scriptcraft_layout::{BeatCardLayout, Caret};
use scriptcraft_model::BeatCard;
use scriptcraft_session::editing::{BeatSelection, BeatTextField, BeatTextPosition};
use std::time::Duration;

fn ordered(selection: &BeatSelection) -> Option<(BeatTextPosition, BeatTextPosition)> {
    let BeatSelection::Text { anchor, focus, .. } = selection else { return None };
    Some(if anchor <= focus { (*anchor, *focus) } else { (*focus, *anchor) })
}

fn local_screen(origin: Pos2, x: f32, y: f32, zoom: f32, x_offset: f32, y_offset: f32) -> Pos2 {
    origin + egui::vec2(x * zoom - x_offset * zoom, y * zoom - y_offset * zoom)
}

fn caret_rect(layout: &BeatCardLayout, field: BeatTextField, position: BeatTextPosition) -> Option<egui::Rect> {
    match field {
        BeatTextField::Title => layout.title_caret_geometry(position.offset).ok().map(|rect| Rect::from_min_size(
            Pos2::new(rect.x.to_points_f32(), rect.y.to_points_f32()), egui::vec2(1.0, rect.height.to_points_f32()))),
        BeatTextField::Body => {
            let index = layout.body_lines.iter().position(|line| line.element == position.paragraph && line.source.start <= position.offset && position.offset <= line.source.end)?;
            let fixed = layout.body_caret_geometry(index, &Caret { element: position.paragraph, offset: position.offset }).ok()?;
            Some(Rect::from_min_size(Pos2::new(fixed.x.to_points_f32(), fixed.y.to_points_f32()), egui::vec2(1.0, fixed.height.to_points_f32())))
        }
    }
}

pub(super) fn paint(
    painter: &egui::Painter,
    geometry: ViewportGeometry,
    card: &BeatCard,
    layout: &BeatCardLayout,
    selection: &BeatSelection,
    state: &BeatTextState,
    palette: PagePalette,
    selection_only: bool,
) {
    let (Some((card_id, field, _, _)), Some((start, end))) = (match selection {
        BeatSelection::Text { card, field, anchor, focus } => Some((card, field, anchor, focus)), _ => None
    }, ordered(selection)) else { return };
    if card_id != &card.id { return; }
    let zoom = scale(geometry.view) as f32;
    let origin = geometry.board_to_screen(pixels(card.rect.left), pixels(card.rect.top));
    let title_right = layout.title_rect.x.to_points_f32() + layout.title_rect.width.to_points_f32();
    let title_bottom = layout.title_rect.y.to_points_f32() + layout.title_rect.height.to_points_f32();
    let body_right = layout.body_rect.x.to_points_f32() + layout.body_rect.width.to_points_f32();
    let body_bottom = layout.body_rect.y.to_points_f32() + layout.body_rect.height.to_points_f32();
    let title_rect = egui::Rect::from_min_max(
        local_screen(origin, layout.title_rect.x.to_points_f32(), layout.title_rect.y.to_points_f32(), zoom, state.title_offset, 0.0),
        local_screen(origin, title_right, title_bottom, zoom, state.title_offset, 0.0),
    ).intersect(painter.clip_rect());
    let body_rect = egui::Rect::from_min_max(
        local_screen(origin, layout.body_rect.x.to_points_f32(), layout.body_rect.y.to_points_f32(), zoom, 0.0, state.body_offset),
        local_screen(origin, body_right, body_bottom, zoom, 0.0, state.body_offset),
    ).intersect(painter.clip_rect());
    let focused = state.focused && state.field.as_ref().is_some_and(|(id, active)| id == card_id && active == field);

    if selection_only { match field {
        BeatTextField::Title if start != end => {
            if let (Some(a), Some(b)) = (caret_rect(layout, *field, start), caret_rect(layout, *field, end)) {
                let a = local_screen(origin, a.left(), a.top(), zoom, state.title_offset, 0.0);
                let b = local_screen(origin, b.left(), b.top(), zoom, state.title_offset, 0.0);
                let rect = Rect::from_min_max(Pos2::new(a.x.min(b.x), a.y), Pos2::new(a.x.max(b.x).max(a.x + 1.0), a.y + layout.title.geometry.height.to_points_f32() * zoom)).intersect(title_rect);
                if rect.is_positive() { painter.rect_filled(rect, 0.0, palette.selection); }
            }
        }
        BeatTextField::Body if start != end => {
            for (index, line) in layout.body_lines.iter().enumerate() {
                if line.element < start.paragraph || line.element > end.paragraph { continue; }
                let from = if line.element == start.paragraph { start.offset.max(line.source.start) } else { line.source.start };
                let to = if line.element == end.paragraph { end.offset.min(line.source.end) } else { line.source.end };
                if from >= to { continue; }
                let (Some(a), Some(b)) = (
                    layout.body_caret_geometry(index, &Caret { element: line.element, offset: from }).ok(),
                    layout.body_caret_geometry(index, &Caret { element: line.element, offset: to }).ok(),
                ) else { continue };
                let a = local_screen(origin, a.x.to_points_f32(), a.y.to_points_f32(), zoom, 0.0, state.body_offset);
                let b = local_screen(origin, b.x.to_points_f32(), b.y.to_points_f32(), zoom, 0.0, state.body_offset);
                let rect = Rect::from_min_max(Pos2::new(a.x.min(b.x), a.y), Pos2::new(a.x.max(b.x).max(a.x + 1.0), a.y + line.geometry.height.to_points_f32() * zoom)).intersect(body_rect);
                if rect.is_positive() { painter.rect_filled(rect, 0.0, palette.selection); }
            }
        }
        _ => {}
    } }

    if selection_only { return; }

    if !focused || start != end { return; }
    let Some(rect) = caret_rect(layout, *field, end) else { return };
    let caret = Rect::from_min_size(
        local_screen(origin, rect.left(), rect.top(), zoom,
            if *field == BeatTextField::Title { state.title_offset } else { 0.0 },
            if *field == BeatTextField::Body { state.body_offset } else { 0.0 }),
        egui::vec2(1.2, rect.height().max(1.0) * zoom),
    );
    let now = painter.ctx().input(|input| input.time.max(0.0));
    let phase = ((now - state.caret_reset_time).max(0.0) / 0.5).floor() as u64;
    painter.ctx().request_repaint_after(Duration::from_millis(500));
    let clip = if *field == BeatTextField::Title { title_rect } else { body_rect };
    if phase.is_multiple_of(2) && clip.contains(caret.min) { painter.rect_filled(caret.intersect(clip), 0.0, palette.caret); }
    if !state.ime_preedit.is_empty() {
        let text_pos = Pos2::new(caret.left(), caret.top());
        let galley = painter.layout_no_wrap(state.ime_preedit.clone(), egui::FontId::proportional(12.0 * zoom), palette.ime_text);
        painter.galley(text_pos, galley, palette.ime_text);
        // Keep composition paint attached to the transformed caret. The current
        // egui viewport command API has no custom IME rectangle command; native
        // event delivery and commit handling remain the platform's responsibility.
    }
}
