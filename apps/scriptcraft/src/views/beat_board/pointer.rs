//! Spatial gestures are transient; losing focus or the pointer cancels capture.
use super::geometry::{pixels, ViewportGeometry};
use eframe::egui::{self, Pos2};
use scriptcraft_commands::{BeatCardIdValue, BeatPointValue, BeatRectValue, BeatSelectionValue, Command};
use scriptcraft_model::{BeatBoard, BeatRect, BoardFixed, StableId};
use scriptcraft_session::editing::BeatSelection;

#[derive(Clone, Copy)]
struct Edges { x: i8, y: i8 }
#[derive(Clone, Copy)]
enum Mode { Move, Resize(Edges), Text }
#[derive(Clone)]
struct Press { card: StableId, start_screen: Pos2, start: (f64, f64), last: (f64, f64), rect: [f64; 4], mode: Mode, token: u64, dragging: bool }
#[derive(Default)]
pub(crate) struct BeatInteraction { press: Option<Press> }

fn card_rect(rect: BeatRect) -> [f64; 4] { [pixels(rect.left), pixels(rect.top), pixels(rect.width), pixels(rect.height)] }
fn fixed(v: f64) -> BoardFixed { BoardFixed::from_milli_pixels((v.max(0.0) * 1000.0).round() as i64) }
fn model_rect(r: [f64; 4]) -> BeatRect { BeatRect { left: fixed(r[0]), top: fixed(r[1]), width: fixed(r[2]), height: fixed(r[3]) } }
fn snap(v: f64) -> f64 { ((v.max(0.0) + 10.0) / 20.0).floor() * 20.0 }
fn grid_limit(v: f64) -> f64 { (v.max(0.0) / 20.0).floor() * 20.0 }

fn preview(press: &Press, point: (f64, f64), board: &BeatBoard) -> [f64; 4] {
    let [x, y, w, h] = press.rect;
    let dx = point.0 - press.start.0;
    let dy = point.1 - press.start.1;
    let bw = pixels(board.width); let bh = pixels(board.height);
    match press.mode {
        Mode::Move | Mode::Text => [(x + dx).clamp(0.0, (bw - w).max(0.0)), (y + dy).clamp(0.0, (bh - h).max(0.0)), w, h],
        Mode::Resize(edge) => {
            let (mut left, mut top, mut right, mut bottom) = (x, y, x + w, y + h);
            if edge.x < 0 { left = (x + dx).clamp((right - 1000.0).max(0.0), (right - 120.0).max(0.0)); }
            if edge.x > 0 { right = (right + dx).clamp(left + 120.0, (left + 1000.0).min(bw)); }
            if edge.y < 0 { top = (y + dy).clamp((bottom - 1000.0).max(0.0), (bottom - 60.0).max(0.0)); }
            if edge.y > 0 { bottom = (bottom + dy).clamp(top + 60.0, (top + 1000.0).min(bh)); }
            [left, top, right - left, bottom - top]
        }
    }
}

fn final_rect(press: &Press, rect: [f64; 4], board: &BeatBoard) -> BeatRect {
    let bw = grid_limit(pixels(board.width)); let bh = grid_limit(pixels(board.height));
    let w = snap(rect[2]).clamp(120.0, 1000.0).min(bw);
    let h = snap(rect[3]).clamp(60.0, 1000.0).min(bh);
    let (mut x, mut y) = (snap(rect[0]), snap(rect[1]));
    if let Mode::Resize(edge) = press.mode {
        if edge.x < 0 { x = snap(press.rect[0] + press.rect[2]) - w; }
        if edge.y < 0 { y = snap(press.rect[1] + press.rect[3]) - h; }
    }
    model_rect([x.clamp(0.0, (bw - w).max(0.0)), y.clamp(0.0, (bh - h).max(0.0)), w, h])
}

fn hit(board: &BeatBoard, geometry: ViewportGeometry, selection: &BeatSelection, pos: Pos2) -> Option<(StableId, Mode)> {
    for card in board.cards.iter().rev() {
        let [x, y, w, h] = card_rect(card.rect);
        let rect = egui::Rect::from_min_max(geometry.board_to_screen(x, y), geometry.board_to_screen(x + w, y + h));
        if !rect.expand(5.0).contains(pos) { continue; }
        let selected = matches!(selection, BeatSelection::Card { card: id } | BeatSelection::Text { card: id, .. } if id == &card.id);
        if selected {
            let edge = Edges { x: if (pos.x - rect.left()).abs() <= 5.0 { -1 } else if (pos.x - rect.right()).abs() <= 5.0 { 1 } else { 0 },
                y: if (pos.y - rect.top()).abs() <= 5.0 { -1 } else if (pos.y - rect.bottom()).abs() <= 5.0 { 1 } else { 0 } };
            if edge.x != 0 || edge.y != 0 { return Some((card.id.clone(), Mode::Resize(edge))); }
        }
        if !rect.contains(pos) { continue; }
        let (px, py) = geometry.screen_to_board(pos);
        let lx = px - x; let ly = py - y;
        // Same default padded Title and Body regions as the canonical card layout.
        let text = lx >= 12.0 && lx <= w - 12.0 && ((ly >= 12.0 && ly <= 32.0) || (ly >= 40.0 && ly <= h - 12.0));
        return Some((card.id.clone(), if text { Mode::Text } else { Mode::Move }));
    }
    None
}

fn cursor(mode: Mode) -> egui::CursorIcon {
    match mode {
        Mode::Resize(Edges { x: 0, .. }) => egui::CursorIcon::ResizeVertical,
        Mode::Resize(Edges { y: 0, .. }) => egui::CursorIcon::ResizeHorizontal,
        Mode::Resize(Edges { x, y }) if x == y => egui::CursorIcon::ResizeNwSe,
        Mode::Resize(_) => egui::CursorIcon::ResizeNeSw,
        Mode::Move => egui::CursorIcon::Grab,
        Mode::Text => egui::CursorIcon::Default,
    }
}

impl BeatInteraction {
    pub(crate) fn cancel(&mut self) { self.press = None; }
    pub(super) fn preview_card(&self, board: &BeatBoard, geometry: ViewportGeometry, pos: Option<Pos2>) -> Option<(StableId, BeatRect)> {
        let press = self.press.as_ref().filter(|p| p.dragging)?;
        let point = pos.map(|p| geometry.screen_to_board(p)).unwrap_or(press.last);
        Some((press.card.clone(), model_rect(preview(press, point, board))))
    }
    pub(super) fn input(&mut self, ui: &egui::Ui, response: &egui::Response, geometry: ViewportGeometry,
        board: &BeatBoard, selection: &BeatSelection, token: u64, enabled: bool) -> Option<Command> {
        let ctx = ui.ctx();
        let (pos, pressed, released, down, escape, focused, gone) = ctx.input(|i| (i.pointer.interact_pos(), i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.primary_down(), i.key_pressed(egui::Key::Escape), i.viewport().focused != Some(false), i.events.iter().any(|e| matches!(e, egui::Event::PointerGone))));
        if !enabled || escape || !focused || gone || self.press.as_ref().is_some_and(|p| p.token != token) { self.cancel(); return None; }
        if let Some(press) = self.press.as_mut() {
            if let Some(pos) = pos {
                press.last = geometry.screen_to_board(pos);
                if (down || released) && !matches!(press.mode, Mode::Text) && pos.distance(press.start_screen) >= 3.0 { press.dragging = true; }
                if press.dragging { ctx.set_cursor_icon(if matches!(press.mode, Mode::Move) { egui::CursorIcon::Grabbing } else { cursor(press.mode) }); ctx.request_repaint(); }
            }
            if released {
                let press = self.press.take()?;
                let card = BeatCardIdValue::from_core(press.card.clone());
                if press.dragging {
                    let point = press.last;
                    return Some(Command::BeatBoardCardGeometrySet { card, rect: BeatRectValue::from_core(final_rect(&press, preview(&press, point, board), board)), activate: true });
                }
                return Some(Command::BeatBoardCardActivate { card });
            }
            return None;
        }
        if let Some(pos) = pos.filter(|p| response.hovered() && geometry.clip.contains(*p) && geometry.board_screen_rect().contains(*p)) {
            let target = hit(board, geometry, selection, pos);
            if let Some((id, mode)) = &target {
                let selected = matches!(selection, BeatSelection::Card { card } if card == id);
                ctx.set_cursor_icon(if !selected && matches!(mode, Mode::Text) { egui::CursorIcon::Grab } else { cursor(*mode) });
            }
            if response.double_clicked() && target.is_none() {
                let (x, y) = geometry.screen_to_board(pos);
                let x = snap(x).min(grid_limit(pixels(board.width) - 240.0));
                let y = snap(y).min(grid_limit(pixels(board.height) - 100.0));
                return Some(Command::BeatBoardCardCreate { position: BeatPointValue { x: fixed(x).raw(), y: fixed(y).raw() } });
            }
            if pressed {
                if let Some((card, mode)) = target {
                    let item = board.cards.iter().find(|item| item.id == card)?;
                    // A selected card's text region belongs to the text editor;
                    // do not create a spatial press that can consume its drag.
                    if matches!(selection, BeatSelection::Card { card: active } | BeatSelection::Text { card: active, .. } if active == &card)
                        && matches!(mode, Mode::Text)
                    { return None; }
                    let start = geometry.screen_to_board(pos);
                    self.press = Some(Press { card, start_screen: pos, start, last: start, rect: card_rect(item.rect), mode, token, dragging: false });
                } else {
                    return Some(Command::BeatBoardSelectionSet { selection: BeatSelectionValue::None });
                }
            }
        }
        None
    }
}
