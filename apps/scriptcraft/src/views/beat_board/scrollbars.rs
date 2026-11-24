//! Transient overlay scrollbars for the finite Beat Board viewport.

use super::geometry::{clamp_origin, pixels, scale, ViewportGeometry};
use crate::chrome::theme::EditorScrollbarPalette;
use eframe::egui::{self, Pos2, Rect, Vec2};
use scriptcraft_fdx::BeatBoardViewState;
use scriptcraft_model::{BeatPoint, BoardFixed};
use std::time::{Duration, Instant};

const THICKNESS: f32 = 10.0;
const HIT_EXPANSION: f32 = 3.0;
const MIN_THUMB: f32 = 24.0;
const HIDE_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis { Horizontal, Vertical }

impl Axis {
    fn coordinate(self, point: Pos2) -> f32 {
        match self { Self::Horizontal => point.x, Self::Vertical => point.y }
    }
}

#[derive(Clone, Copy)]
struct AxisGeometry {
    axis: Axis,
    track: Rect,
    thumb: Rect,
    max_origin: f64,
    travel: f32,
}

#[derive(Clone, Copy, Default)]
struct ScrollbarGeometry {
    horizontal: Option<AxisGeometry>,
    vertical: Option<AxisGeometry>,
}

impl ScrollbarGeometry {
    fn axis(self, axis: Axis) -> Option<AxisGeometry> {
        match axis { Axis::Horizontal => self.horizontal, Axis::Vertical => self.vertical }
    }
}

#[derive(Clone, Copy)]
struct Capture {
    axis: Axis,
    grab_offset: f32,
    token: u64,
}

#[derive(Default)]
pub(crate) struct ScrollbarInteraction {
    capture: Option<Capture>,
    consumed_press: Option<u64>,
    hovered: Option<Axis>,
    visible: bool,
    hide_at: Option<Instant>,
    last_view: Option<(i64, i64, u32)>,
}

impl ScrollbarInteraction {
    fn reveal(&mut self, ctx: &egui::Context, now: Instant) {
        self.visible = true;
        self.hide_at = Some(now + HIDE_DELAY);
        ctx.request_repaint_after(HIDE_DELAY);
    }

    pub(crate) fn observe_view(&mut self, ctx: &egui::Context, view: BeatBoardViewState) {
        let origin = view.scroll_origin();
        let current = (origin.x.raw(), origin.y.raw(), view.zoom_milli_percent());
        if self.last_view.is_some_and(|previous| previous != current) {
            self.reveal(ctx, Instant::now());
        }
        self.last_view = Some(current);
    }
}

fn finite_extent(value: f32) -> f64 {
    if value.is_finite() { f64::from(value.max(0.0)) } else { 0.0 }
}

fn track_rect(geometry: ViewportGeometry, axis: Axis, other_visible: bool) -> Rect {
    let viewport = geometry.viewport;
    match axis {
        Axis::Horizontal => {
            let right_reserve = if other_visible { THICKNESS } else { 0.0 };
            Rect::from_min_max(
                Pos2::new(viewport.left(), viewport.bottom() - THICKNESS),
                Pos2::new(viewport.right() - right_reserve, viewport.bottom()),
            )
        }
        Axis::Vertical => {
            let bottom_reserve = if other_visible { THICKNESS } else { 0.0 };
            Rect::from_min_max(
                Pos2::new(viewport.right() - THICKNESS, viewport.top()),
                Pos2::new(viewport.right(), viewport.bottom() - bottom_reserve),
            )
        }
    }
}

fn axis_geometry(geometry: ViewportGeometry, axis: Axis, other_visible: bool, maximum: f64, fraction: f64) -> Option<AxisGeometry> {
    let track = track_rect(geometry, axis, other_visible);
    let length = match axis { Axis::Horizontal => track.width(), Axis::Vertical => track.height() };
    if !track.is_positive() || !length.is_finite() || length <= 0.0 || !maximum.is_finite() || maximum <= 0.0 {
        return None;
    }
    let thumb_length = ((f64::from(length) * fraction).max(f64::from(MIN_THUMB))).min(f64::from(length)) as f32;
    let travel = (length - thumb_length).max(0.0);
    let origin = geometry.view.scroll_origin();
    let origin = match axis { Axis::Horizontal => pixels(origin.x), Axis::Vertical => pixels(origin.y) };
    let ratio = if maximum > 0.0 { (origin / maximum).clamp(0.0, 1.0) } else { 0.0 };
    let offset = (f64::from(travel) * ratio) as f32;
    let thumb = match axis {
        Axis::Horizontal => Rect::from_min_size(track.min + Vec2::new(offset, 0.0), Vec2::new(thumb_length, track.height())),
        Axis::Vertical => Rect::from_min_size(track.min + Vec2::new(0.0, offset), Vec2::new(track.width(), thumb_length)),
    };
    Some(AxisGeometry { axis, track, thumb, max_origin: maximum, travel })
}

fn geometry_for(geometry: ViewportGeometry) -> ScrollbarGeometry {
    let z = scale(geometry.view);
    let width = pixels(geometry.board_width);
    let height = pixels(geometry.board_height);
    let viewport_width = finite_extent(geometry.viewport.width());
    let viewport_height = finite_extent(geometry.viewport.height());
    let max_x = (width - viewport_width / z).max(0.0);
    let max_y = (height - viewport_height / z).max(0.0);
    let scaled_width = width * z;
    let scaled_height = height * z;
    let fraction_x = if scaled_width.is_finite() && scaled_width > 0.0 { (viewport_width / scaled_width).clamp(0.0, 1.0) } else { 1.0 };
    let fraction_y = if scaled_height.is_finite() && scaled_height > 0.0 { (viewport_height / scaled_height).clamp(0.0, 1.0) } else { 1.0 };
    let show_x = max_x.is_finite() && max_x > 0.0 && fraction_x < 1.0;
    let show_y = max_y.is_finite() && max_y > 0.0 && fraction_y < 1.0;
    ScrollbarGeometry {
        horizontal: show_x.then(|| axis_geometry(geometry, Axis::Horizontal, show_y, max_x, fraction_x)).flatten(),
        vertical: show_y.then(|| axis_geometry(geometry, Axis::Vertical, show_x, max_y, fraction_y)).flatten(),
    }
}

fn hit_region(axis: AxisGeometry, viewport: Rect) -> Rect {
    axis.track.expand(HIT_EXPANSION).intersect(viewport)
}

fn pointer_in(axis: AxisGeometry, viewport: Rect, pointer: Pos2) -> bool {
    hit_region(axis, viewport).contains(pointer)
}

fn with_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn fixed_origin(value: f64) -> BoardFixed {
    let raw = (value.max(0.0) * BoardFixed::SCALE as f64).floor();
    BoardFixed::from_milli_pixels(if raw.is_finite() { raw.min(i64::MAX as f64) as i64 } else { 0 })
}

fn update_origin(geometry: ViewportGeometry, axis_geometry: AxisGeometry, pointer: Pos2, grab_offset: f32) -> BeatBoardViewState {
    let origin = geometry.view.scroll_origin();
    let (mut x, mut y) = (pixels(origin.x), pixels(origin.y));
    if axis_geometry.travel > 0.0 && axis_geometry.travel.is_finite() {
        let pointer_offset = (axis_geometry.axis.coordinate(pointer) - axis_geometry.axis.coordinate(axis_geometry.track.min) - grab_offset).clamp(0.0, axis_geometry.travel);
        let value = f64::from(pointer_offset / axis_geometry.travel) * axis_geometry.max_origin;
        match axis_geometry.axis { Axis::Horizontal => x = value, Axis::Vertical => y = value }
    }
    let origin = clamp_origin((x, y), geometry.board_width, geometry.board_height, geometry.viewport.size(), scale(geometry.view));
    BeatBoardViewState::new(origin, geometry.view.zoom_milli_percent()).unwrap_or(geometry.view)
}

pub(crate) fn handle(
    ui: &egui::Ui,
    geometry: ViewportGeometry,
    view: &mut BeatBoardViewState,
    state: &mut ScrollbarInteraction,
    token: u64,
    enabled: bool,
) -> bool {
    let ctx = ui.ctx();
    let (pointer, pressed, released, down, escape, focused, gone, moved_inside) = ctx.input(|input| (
        input.pointer.interact_pos(), input.pointer.primary_pressed(), input.pointer.primary_released(),
        input.pointer.primary_down(), input.key_pressed(egui::Key::Escape), input.viewport().focused != Some(false),
        input.events.iter().any(|event| matches!(event, egui::Event::PointerGone)),
        input.events.iter().any(|event| matches!(event, egui::Event::PointerMoved(pos) if geometry.clip.contains(*pos))),
    ));
    let now = Instant::now();
    if moved_inside {
        state.reveal(ctx, now);
    }
    let layout = geometry_for(geometry);
    let finite_pointer = pointer.filter(|point| point.x.is_finite() && point.y.is_finite());
    let pointer_over_bar = finite_pointer.is_some_and(|pointer| [layout.horizontal, layout.vertical]
        .into_iter().flatten().any(|bar| pointer_in(bar, geometry.viewport, pointer)));
    let activity_pending = state.hide_at.is_some_and(|deadline| now < deadline);
    state.visible = state.capture.is_some() || activity_pending || (state.visible && pointer_over_bar);
    if !state.visible {
        state.hovered = None;
    }
    let invalid_capture = state.capture.is_some_and(|capture| capture.token != token);
    let invalid_press = state.consumed_press.is_some_and(|press_token| press_token != token);
    if !enabled || escape || !focused || gone || invalid_capture || invalid_press {
        let had_scrollbar_gesture = state.capture.is_some() || state.consumed_press.is_some();
        state.capture = None;
        state.hovered = None;
        // Cancellation ends the thumb drag but still consumes the remainder of
        // its physical press, so release cannot become a card/text click.
        if had_scrollbar_gesture && down && !released {
            state.consumed_press = Some(token);
            return true;
        }
        state.consumed_press = None;
        return false;
    }

    if let Some(capture) = state.capture {
        let Some(bar) = layout.axis(capture.axis) else {
            state.capture = None;
            state.consumed_press = None;
            state.hovered = None;
            return false;
        };
        if let Some(pointer) = pointer.filter(|point| point.x.is_finite() && point.y.is_finite()) {
            *view = update_origin(geometry, bar, pointer, capture.grab_offset);
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            ctx.request_repaint();
        }
        if released || !down {
            state.capture = None;
            state.consumed_press = None;
            state.hovered = None;
        }
        return true;
    }

    if let Some(token) = state.consumed_press {
        if released || !down { state.consumed_press = None; }
        else { state.consumed_press = Some(token); }
        return true;
    }

    state.hovered = None;
    if !state.visible { return false; }
    let Some(pointer) = finite_pointer else { return false };
    let Some(bar) = [layout.horizontal, layout.vertical].into_iter().flatten().find(|bar| pointer_in(*bar, geometry.viewport, pointer)) else { return false };
    let on_thumb = bar.thumb.expand(HIT_EXPANSION).intersect(geometry.viewport).contains(pointer);
    if on_thumb {
        state.hovered = Some(bar.axis);
        ctx.set_cursor_icon(if pressed { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
    } else {
        ctx.set_cursor_icon(egui::CursorIcon::Default);
    }
    if pressed {
        state.consumed_press = Some(token);
        if on_thumb {
            // Keep the actual pointer-to-thumb offset, including a small signed
            // offset when the expanded hit target is touched just outside the thumb.
            let grab_offset = bar.axis.coordinate(pointer) - bar.axis.coordinate(bar.thumb.min);
            state.capture = Some(Capture { axis: bar.axis, grab_offset, token });
            *view = update_origin(geometry, bar, pointer, grab_offset);
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }
    }
    true
}

pub(crate) fn paint(ui: &egui::Ui, geometry: ViewportGeometry, state: &ScrollbarInteraction, palette: EditorScrollbarPalette) {
    if !state.visible { return; }
    let layout = geometry_for(geometry);
    let painter = ui.painter().with_clip_rect(geometry.viewport.intersect(geometry.clip));
    for bar in [layout.horizontal, layout.vertical].into_iter().flatten() {
        let color = if state.capture.is_some_and(|capture| capture.axis == bar.axis) {
            palette.dragged_handle
        } else if state.hovered == Some(bar.axis) {
            palette.hovered_handle
        } else {
            palette.resting_handle
        };
        painter.rect_filled(bar.thumb, 5.0, with_alpha(color, 64));
        painter.rect_stroke(
            bar.thumb,
            5.0,
            egui::Stroke::new(1.0, with_alpha(color, 128)),
            egui::StrokeKind::Inside,
        );
    }
}
