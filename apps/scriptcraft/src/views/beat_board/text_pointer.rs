//! Card-local text hit testing and transient drag selection.

use super::{geometry::{pixels, scale, ViewportGeometry}, text_state::BeatTextState};
use eframe::egui;
use scriptcraft_commands::{BeatCardIdValue, BeatSelectionValue, BeatTextFieldValue, BeatTextPositionValue, Command};
use scriptcraft_layout::{BeatCardLayout, Caret};
use scriptcraft_model::{BeatBoard, formatting::FixedPoint};
use scriptcraft_session::editing::{BeatSelection, BeatTextField, BeatTextPosition};

fn fixed(value: f64) -> FixedPoint {
    FixedPoint::from_points_f64(value).unwrap_or_default()
}

fn payload(card: &scriptcraft_model::StableId, field: BeatTextField, anchor: BeatTextPosition, focus: BeatTextPosition) -> Command {
    Command::BeatBoardSelectionSet { selection: BeatSelectionValue::Text {
        card: BeatCardIdValue::from_core(card.clone()),
        field: match field { BeatTextField::Title => BeatTextFieldValue::Title, BeatTextField::Body => BeatTextFieldValue::Body },
        anchor: BeatTextPositionValue { paragraph: anchor.paragraph as u32, offset: anchor.offset as u32 },
        focus: BeatTextPositionValue { paragraph: focus.paragraph as u32, offset: focus.offset as u32 },
    }}
}

fn caret_at(layout: &BeatCardLayout, field: BeatTextField, x: f64, y: f64, title_scroll: f32, body_scroll: f32) -> BeatTextPosition {
    match field {
        BeatTextField::Title => BeatTextPosition { paragraph: 0, offset: layout.title_caret_for_x(fixed(x + f64::from(title_scroll))).offset },
        BeatTextField::Body => {
            let lines = &layout.body_lines;
            let line = lines.iter().enumerate().filter(|(_, line)| line.source.element < usize::MAX)
                .min_by(|(_, a), (_, b)| {
                    let ay = f64::from(a.geometry.y.to_points_f32()) - f64::from(body_scroll);
                    let by = f64::from(b.geometry.y.to_points_f32()) - f64::from(body_scroll);
                    (ay - y).abs().total_cmp(&(by - y).abs())
                });
            if let Some((index, selected)) = line {
                let caret = layout.body_caret_for_x(index, fixed(x)).unwrap_or(Caret { element: selected.element, offset: selected.source.start });
                BeatTextPosition { paragraph: caret.element, offset: caret.offset }
            } else { BeatTextPosition { paragraph: 0, offset: 0 } }
        }
    }
}

fn word_range(text: &str, offset: usize) -> (usize, usize) {
    let mut boundaries = text.char_indices().collect::<Vec<_>>();
    boundaries.push((text.len(), '\0'));
    let index = boundaries.partition_point(|(byte, _)| *byte < offset).min(boundaries.len().saturating_sub(1));
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_';
    if index < boundaries.len().saturating_sub(1) && is_word(boundaries[index].1) {
        let mut left = index; let mut right = index + 1;
        while left > 0 && is_word(boundaries[left - 1].1) { left -= 1; }
        while right < boundaries.len().saturating_sub(1) && is_word(boundaries[right].1) { right += 1; }
        (boundaries[left].0, boundaries[right].0)
    } else { (offset, offset) }
}

/// Returns a validated SelectionSet command for click/drag; no semantic history is created.
pub(super) fn handle(
    ui: &egui::Ui,
    response: &egui::Response,
    geometry: ViewportGeometry,
    board: &BeatBoard,
    selection: &BeatSelection,
    layout: Option<&BeatCardLayout>,
    state: &mut BeatTextState,
) -> Option<Command> {
    let (pos, pressed, released, down, focused, time) = ui.ctx().input(|input| (
        input.pointer.interact_pos(), input.pointer.primary_pressed(), input.pointer.primary_released(),
        input.pointer.primary_down(), input.viewport().focused != Some(false), input.time,
    ));
    let double = response.double_clicked();
    let triple = response.triple_clicked();
    if !focused {
        state.blur();
        return None;
    }
    let Some(pos) = pos else {
        if released { state.drag_anchor = None; }
        return None;
    };
    let hit_card = board.cards.iter().rev().find(|card| {
        let x = pixels(card.rect.left); let y = pixels(card.rect.top);
        let min = geometry.board_to_screen(x, y);
        let max = geometry.board_to_screen(x + pixels(card.rect.width), y + pixels(card.rect.height));
        egui::Rect::from_min_max(min, max).contains(pos)
    });
    let active_id = match selection { BeatSelection::Card { card } | BeatSelection::Text { card, .. } => Some(card), _ => None };
    if pressed {
        let Some(card) = hit_card.filter(|card| active_id == Some(&card.id)) else { return None };
        let Some(layout) = layout else { return None };
        let zoom = scale(geometry.view);
        let card_origin = geometry.board_to_screen(pixels(card.rect.left), pixels(card.rect.top));
        let local = ((pos.x - card_origin.x) as f64 / zoom, (pos.y - card_origin.y) as f64 / zoom);
        let title_right = layout.title_rect.x.to_points_f32() + layout.title_rect.width.to_points_f32();
        let title_bottom = layout.title_rect.y.to_points_f32() + layout.title_rect.height.to_points_f32();
        let body_right = layout.body_rect.x.to_points_f32() + layout.body_rect.width.to_points_f32();
        let body_bottom = layout.body_rect.y.to_points_f32() + layout.body_rect.height.to_points_f32();
        let field = if local.0 >= f64::from(layout.title_rect.x.to_points_f32())
            && local.0 <= f64::from(title_right)
            && local.1 >= f64::from(layout.title_rect.y.to_points_f32())
            && local.1 <= f64::from(title_bottom) { Some(BeatTextField::Title) }
        else if local.0 >= f64::from(layout.body_rect.x.to_points_f32())
            && local.0 <= f64::from(body_right)
            && local.1 >= f64::from(layout.body_rect.y.to_points_f32())
            && local.1 <= f64::from(body_bottom) { Some(BeatTextField::Body) }
        else { None };
        if let Some(field) = field {
            let position = caret_at(layout, field, local.0, local.1, state.title_offset, state.body_offset);
            state.focus(card.id.clone(), field, time);
            state.drag_anchor = Some(position);
            state.preferred_x = None;
            if let Some(id) = state.viewport_id { ui.ctx().memory_mut(|memory| memory.request_focus(id)); }
            let (anchor, selected) = if triple {
                match field {
                    BeatTextField::Title => (BeatTextPosition { paragraph: 0, offset: 0 }, BeatTextPosition { paragraph: 0, offset: card.title.len() }),
                    BeatTextField::Body => (BeatTextPosition { paragraph: position.paragraph, offset: 0 }, BeatTextPosition { paragraph: position.paragraph, offset: card.paragraphs.get(position.paragraph).map_or(0, |p| p.text.len()) }),
                }
            } else if double {
                let text = match field { BeatTextField::Title => card.title.as_str(), BeatTextField::Body => card.paragraphs.get(position.paragraph).map_or("", |p| p.text.as_str()) };
                let (from, to) = word_range(text, position.offset);
                (BeatTextPosition { paragraph: position.paragraph, offset: from }, BeatTextPosition { paragraph: position.paragraph, offset: to })
            } else { (position, position) };
            state.drag_anchor = Some(anchor);
            return Some(payload(&card.id, field, anchor, selected));
        }
    }
    if down || released {
        let Some(anchor) = state.drag_anchor else { return None };
        let Some(card_id) = state.field.as_ref().map(|(id, _)| id) else { return None };
        let Some(card) = board.cards.iter().find(|card| &card.id == card_id) else { return None };
        let Some(layout) = layout else { return None };
        let field = state.field.as_ref().map(|(_, field)| *field)?;
        let zoom = scale(geometry.view);
        let origin = geometry.board_to_screen(pixels(card.rect.left), pixels(card.rect.top));
        let mut local = ((pos.x - origin.x) as f64 / zoom, (pos.y - origin.y) as f64 / zoom);
        match field {
            BeatTextField::Title => {
                let left = f64::from(layout.title_rect.x.to_points_f32());
                let right = left + f64::from(layout.title_rect.width.to_points_f32());
                if local.0 < left { state.title_offset = (state.title_offset - 8.0).max(0.0); }
                if local.0 > right { state.title_offset += 8.0; }
                state.title_offset = state.title_offset.min((layout.title.geometry.width.to_points_f32() - layout.title_rect.width.to_points_f32()).max(0.0));
                local.0 = local.0.clamp(left, right);
                local.1 = f64::from(layout.title_rect.y.to_points_f32());
            }
            BeatTextField::Body => {
                let top = f64::from(layout.body_rect.y.to_points_f32());
                let bottom = top + f64::from(layout.body_rect.height.to_points_f32());
                let max = (layout.body_content_height.to_points_f32() - layout.body_rect.height.to_points_f32()).max(0.0);
                if local.1 < top { state.body_offset = (state.body_offset - 10.0).max(0.0); }
                if local.1 > bottom { state.body_offset = (state.body_offset + 10.0).min(max); }
                local.0 = local.0.clamp(f64::from(layout.body_rect.x.to_points_f32()), f64::from(layout.body_rect.x.to_points_f32() + layout.body_rect.width.to_points_f32()));
                local.1 = local.1.clamp(top, bottom);
            }
        }
        let focus = caret_at(layout, field, local.0, local.1, state.title_offset, state.body_offset);
        if released { state.drag_anchor = None; }
        return Some(payload(&card.id, field, anchor, focus));
    }
    if released { state.drag_anchor = None; }
    let _ = response;
    None
}
