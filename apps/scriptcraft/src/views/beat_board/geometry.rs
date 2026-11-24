//! The single authority for Beat board projection and bounded navigation.

use eframe::egui::{self, Pos2, Rect, Vec2};
use scriptcraft_fdx::BeatBoardViewState;
use scriptcraft_model::{BeatBoard, BeatPoint, BeatRect, BoardFixed, BOARD_SNAP_INCREMENT};

#[derive(Debug, Clone, Copy)]
pub(crate) struct BoardArea {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewportGeometry {
    pub(crate) viewport: Rect,
    pub(crate) clip: Rect,
    pub(crate) board_width: BoardFixed,
    pub(crate) board_height: BoardFixed,
    pub(crate) view: BeatBoardViewState,
    pub(crate) visible: Option<BoardArea>,
    pub(crate) pointer_screen: Option<Pos2>,
    pub(crate) pointer_board: Option<(f64, f64)>,
}

pub(crate) fn pixels(value: BoardFixed) -> f64 {
    value.raw() as f64 / BoardFixed::SCALE as f64
}

fn fixed_floor(value: f64) -> BoardFixed {
    let raw = (value.max(0.0) * BoardFixed::SCALE as f64).floor();
    BoardFixed::from_milli_pixels(if raw.is_finite() { raw.min(i64::MAX as f64) as i64 } else { 0 })
}

pub(crate) fn scale(view: BeatBoardViewState) -> f64 {
    let z = f64::from(view.zoom_milli_percent()) / 100_000.0;
    if z.is_finite() && z > 0.0 { z } else { 1.0 }
}

pub(crate) fn clamp_origin(origin: (f64, f64), width: BoardFixed, height: BoardFixed, viewport: Vec2, z: f64) -> BeatPoint {
    let z = if z.is_finite() && z > 0.0 { z } else { 1.0 };
    let size = |v: f32| if v.is_finite() { f64::from(v.max(0.0)) } else { 0.0 };
    let axis = |value: f64, dimension: BoardFixed, extent: f32| {
        let upper = (pixels(dimension) - size(extent) / z).max(0.0);
        fixed_floor(if value.is_finite() { value.clamp(0.0, upper) } else { 0.0 })
    };
    BeatPoint { x: axis(origin.0, width, viewport.x), y: axis(origin.1, height, viewport.y) }
}

impl ViewportGeometry {
    pub(crate) fn new(viewport: Rect, clip: Rect, board: &BeatBoard, view: BeatBoardViewState, pointer: Option<Pos2>) -> Self {
        let origin = view.scroll_origin();
        let origin = clamp_origin((pixels(origin.x), pixels(origin.y)), board.width, board.height, viewport.size(), scale(view));
        let view = BeatBoardViewState::new(origin, view.zoom_milli_percent()).unwrap_or_default();
        Self::from_parts(viewport, clip, board.width, board.height, view, pointer)
    }

    fn from_parts(viewport: Rect, clip: Rect, board_width: BoardFixed, board_height: BoardFixed, view: BeatBoardViewState, pointer_screen: Option<Pos2>) -> Self {
        let mut result = Self { viewport, clip: viewport.intersect(clip), board_width, board_height, view, visible: None, pointer_screen, pointer_board: None };
        let board_clip = result.clip.intersect(result.board_screen_rect());
        if board_clip.is_positive() {
            let (left, top) = result.screen_to_board(board_clip.min);
            let (right, bottom) = result.screen_to_board(board_clip.max);
            result.visible = Some(BoardArea { left: left.max(0.0), top: top.max(0.0), right: right.min(pixels(board_width)), bottom: bottom.min(pixels(board_height)) });
        }
        result.pointer_board = pointer_screen.filter(|p| board_clip.contains(*p)).map(|p| result.screen_to_board(p));
        result
    }

    pub(crate) fn with_view(self, view: BeatBoardViewState) -> Self {
        Self::from_parts(self.viewport, self.clip, self.board_width, self.board_height, view, self.pointer_screen)
    }

    pub(crate) fn with_pointer(self, pointer: Option<Pos2>) -> Self {
        Self::from_parts(self.viewport, self.clip, self.board_width, self.board_height, self.view, pointer)
    }

    pub(crate) fn board_to_screen(self, x: f64, y: f64) -> Pos2 {
        let origin = self.view.scroll_origin();
        let z = scale(self.view);
        egui::pos2((f64::from(self.viewport.min.x) + (x - pixels(origin.x)) * z) as f32, (f64::from(self.viewport.min.y) + (y - pixels(origin.y)) * z) as f32)
    }

    pub(crate) fn screen_to_board(self, point: Pos2) -> (f64, f64) {
        let origin = self.view.scroll_origin();
        let z = scale(self.view);
        (pixels(origin.x) + f64::from(point.x - self.viewport.min.x) / z, pixels(origin.y) + f64::from(point.y - self.viewport.min.y) / z)
    }

    pub(crate) fn board_screen_rect(self) -> Rect {
        Rect::from_min_max(self.board_to_screen(0.0, 0.0), self.board_to_screen(pixels(self.board_width), pixels(self.board_height)))
    }

    pub(crate) fn origin_for_anchor(self, anchor_board: (f64, f64), anchor_screen: Pos2, new_zoom: u32) -> BeatPoint {
        let z = f64::from(new_zoom) / 100_000.0;
        let z = if z.is_finite() && z > 0.0 { z } else { 1.0 };
        clamp_origin((anchor_board.0 - f64::from(anchor_screen.x - self.viewport.min.x) / z, anchor_board.1 - f64::from(anchor_screen.y - self.viewport.min.y) / z), self.board_width, self.board_height, self.viewport.size(), z)
    }

    pub(crate) fn pan(self, delta: Vec2) -> BeatPoint {
        let origin = self.view.scroll_origin();
        let z = scale(self.view);
        clamp_origin((pixels(origin.x) - f64::from(delta.x) / z, pixels(origin.y) - f64::from(delta.y) / z), self.board_width, self.board_height, self.viewport.size(), z)
    }
}

/// Later card paint can cull without converting canonical geometry to screen coordinates.
#[allow(dead_code)]
pub(crate) fn rect_visible(rect: BeatRect, visible: BoardArea) -> bool {
    match (rect.right(), rect.bottom()) {
        (Some(right), Some(bottom)) => pixels(rect.left) < visible.right && pixels(right) > visible.left && pixels(rect.top) < visible.bottom && pixels(bottom) > visible.top,
        _ => false,
    }
}

#[allow(dead_code)]
pub(crate) fn snap_point(point: BeatPoint) -> Option<BeatPoint> {
    fn axis(value: BoardFixed) -> Option<BoardFixed> {
        let step = BOARD_SNAP_INCREMENT.raw();
        let rounded = value.raw().checked_add(step / 2)?.div_euclid(step).checked_mul(step)?;
        Some(BoardFixed::from_milli_pixels(rounded))
    }
    Some(BeatPoint { x: axis(point.x)?, y: axis(point.y)? })
}

#[allow(dead_code)]
pub(crate) fn snap_rect(rect: BeatRect) -> Option<BeatRect> {
    let origin = snap_point(BeatPoint { x: rect.left, y: rect.top })?;
    let size = snap_point(BeatPoint { x: rect.width, y: rect.height })?;
    Some(BeatRect { left: origin.x, top: origin.y, width: size.x, height: size.y })
}
