//! Horizontal proportional scrollbar for the Story Map timeline.

use std::time::{Duration, Instant};

use eframe::egui::{self, Pos2, Rect};
use scriptcraft_fdx::StoryMapViewState;
use scriptcraft_model::{BoardFixed, MAX_BOARD_DIMENSION};

use super::{constants::*, geometry::TimelineGeometry};
use crate::chrome::theme::EditorScrollbarPalette;

#[derive(Debug, Clone, Copy)]
struct ThumbGeometry {
    track: Rect,
    thumb: Rect,
    /// Canonical pre-translation length, independent of translated endpoint subtraction.
    thumb_length: f32,
    travel: f32,
    max_origin: f64,
}

#[derive(Debug, Clone, Copy)]
struct Capture {
    grab_fraction: f32,
    track_left: f32,
    track_top: f32,
    track_length: f32,
    thumb_length: f32,
    max_origin: f64,
    target_pages: f64,
    domain_pages: f64,
    timeline_left: f64,
    viewport_width: f64,
    page_pitch: f64,
    zoom_percent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureCompatibility {
    InvalidThumb,
    IncompatibleLayout,
    DomainRebase,
    DerivedMismatch,
    Unchanged,
}

#[derive(Debug, Default)]
pub(crate) struct StoryMapScrollbarState {
    capture: Option<Capture>,
    consumed_press: bool,
    hovered: bool,
    visible: bool,
    hide_at: Option<Instant>,
    last_view: Option<(i64, u32)>,
    last_pointer: Option<Pos2>,
}

impl StoryMapScrollbarState {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn consumes_pointer(&self) -> bool {
        self.capture.is_some() || self.consumed_press
    }

    pub(crate) fn observe_view(&mut self, ctx: &egui::Context, view: StoryMapViewState) {
        let current = (view.scroll_origin().raw(), view.zoom_milli_percent());
        if self.last_view.is_some_and(|previous| previous != current) {
            self.reveal(ctx, Instant::now());
        }
        self.last_view = Some(current);
    }

    fn reveal(&mut self, ctx: &egui::Context, now: Instant) {
        self.visible = true;
        let delay = Duration::from_secs(STORY_MAP_SCROLLBAR_HIDE_SECONDS);
        self.hide_at = Some(now + delay);
        ctx.request_repaint_after(delay);
    }
}

fn thumb_geometry(geometry: &TimelineGeometry, track: Rect) -> Option<ThumbGeometry> {
    let track_length = track.width();
    if !track.is_positive()
        || !track_length.is_finite()
        || track_length <= 0.0
        || !geometry.scaled_domain_width.is_finite()
        || geometry.scaled_domain_width <= geometry.viewport_width
        || !geometry.max_origin.is_finite()
        || geometry.max_origin <= 0.0
    {
        return None;
    }
    let fraction = (geometry.viewport_width / geometry.scaled_domain_width).clamp(0.0, 1.0);
    let thumb_length = (f64::from(track_length) * fraction)
        .max(f64::from(STORY_MAP_SCROLLBAR_MIN_THUMB))
        .min(f64::from(track_length)) as f32;
    if !thumb_length.is_finite() || thumb_length <= 0.0 {
        return None;
    }
    let travel = (track_length - thumb_length).max(0.0);
    let ratio = (geometry.origin / geometry.max_origin).clamp(0.0, 1.0);
    let offset = travel * ratio as f32;
    let thumb = Rect::from_min_max(
        Pos2::new(track.left() + offset, track.top()),
        Pos2::new(track.left() + offset + thumb_length, track.bottom()),
    );
    (thumb.is_positive() && travel.is_finite()).then_some(ThumbGeometry {
        track,
        thumb,
        thumb_length,
        travel,
        max_origin: geometry.max_origin,
    })
}

fn capture_compatibility(
    capture: Capture,
    thumb: Option<ThumbGeometry>,
    geometry: &TimelineGeometry,
) -> CaptureCompatibility {
    let Some(bar) = thumb else {
        return CaptureCompatibility::InvalidThumb;
    };

    if !capture.domain_pages.is_finite() || !geometry.domain_pages.is_finite() {
        return CaptureCompatibility::DerivedMismatch;
    }

    if capture.track_length != bar.track.width()
        || capture.track_left != bar.track.left()
        || capture.track_top != bar.track.top()
        || capture.target_pages != geometry.target_pages
        || capture.timeline_left != geometry.timeline_left
        || capture.viewport_width != geometry.viewport_width
        || capture.page_pitch != geometry.page_pitch
        || capture.zoom_percent != geometry.zoom_percent
    {
        return CaptureCompatibility::IncompatibleLayout;
    }

    if capture.domain_pages != geometry.domain_pages {
        return CaptureCompatibility::DomainRebase;
    }

    if capture.thumb_length != bar.thumb_length || capture.max_origin != bar.max_origin {
        return CaptureCompatibility::DerivedMismatch;
    }

    CaptureCompatibility::Unchanged
}

fn grab_fraction(position: Pos2, bar: ThumbGeometry) -> Option<f32> {
    let width = bar.thumb_length;
    if !position.x.is_finite()
        || !bar.thumb.left().is_finite()
        || !width.is_finite()
        || width <= 0.0
    {
        return None;
    }
    let fraction = (position.x - bar.thumb.left()) / width;
    fraction
        .is_finite()
        .then_some(fraction.clamp(0.0, 1.0))
}

fn mapped_origin(bar: ThumbGeometry, pointer_x: f32, grab_fraction: f32) -> Option<f64> {
    let thumb_length = bar.thumb_length;
    if !pointer_x.is_finite()
        || !grab_fraction.is_finite()
        || !(0.0..=1.0).contains(&grab_fraction)
        || !thumb_length.is_finite()
        || thumb_length <= 0.0
        || !bar.track.left().is_finite()
        || !bar.travel.is_finite()
        || bar.travel < 0.0
        || !bar.max_origin.is_finite()
        || bar.max_origin < 0.0
    {
        return None;
    }

    let grab_offset = grab_fraction * thumb_length;
    let pointer_offset = pointer_x - bar.track.left() - grab_offset;
    if !grab_offset.is_finite() || !pointer_offset.is_finite() {
        return None;
    }
    let offset = pointer_offset.clamp(0.0, bar.travel);
    if !offset.is_finite() {
        return None;
    }
    let origin = if bar.travel > 0.0 {
        f64::from(offset / bar.travel) * bar.max_origin
    } else {
        0.0
    };
    origin.is_finite().then_some(origin)
}

fn hit_rect(base: Rect, navigation_track: Rect, strip: Rect) -> Option<Rect> {
    let finite_rect = |rect: Rect| {
        rect.min.x.is_finite()
            && rect.min.y.is_finite()
            && rect.max.x.is_finite()
            && rect.max.y.is_finite()
    };
    if !finite_rect(base)
        || !finite_rect(navigation_track)
        || !navigation_track.is_positive()
        || !finite_rect(strip)
        || !strip.is_positive()
    {
        return None;
    }

    let expanded = base.expand(STORY_MAP_SCROLLBAR_HIT_EXPANSION);
    if !finite_rect(expanded) {
        return None;
    }
    let clipped = expanded.intersect(strip);
    let resize_top = strip.bottom() - RESIZE_HIT_BAND_HEIGHT;
    if !resize_top.is_finite() {
        return None;
    }
    let clipped = Rect::from_min_max(
        Pos2::new(
            clipped.left().max(navigation_track.left()),
            clipped.top(),
        ),
        Pos2::new(
            clipped.right().min(navigation_track.right()),
            clipped.bottom().min(resize_top),
        ),
    );
    (finite_rect(clipped) && clipped.is_positive()).then_some(clipped)
}

fn set_origin(view: &mut StoryMapViewState, geometry: &TimelineGeometry, origin: f64) -> bool {
    let Some(clamped) = geometry.with_origin(origin) else {
        return false;
    };
    let Some(fixed) = fixed_origin(clamped.origin, clamped.max_origin) else {
        return false;
    };
    if fixed == view.scroll_origin() {
        return false;
    }
    if let Some(next) = StoryMapViewState::new(
        view.height(),
        view.width(),
        fixed,
        view.zoom_milli_percent(),
    ) {
        *view = next;
        true
    } else {
        false
    }
}

fn fixed_origin(value: f64, maximum: f64) -> Option<BoardFixed> {
    if !value.is_finite() || !maximum.is_finite() || value < 0.0 || maximum < 0.0 {
        return None;
    }
    let raw = (value.min(maximum) * BoardFixed::SCALE as f64).floor();
    (raw.is_finite() && raw >= 0.0 && raw <= MAX_BOARD_DIMENSION.raw() as f64)
        .then_some(BoardFixed::from_milli_pixels(raw as i64))
}

pub(crate) fn handle(
    ui: &mut egui::Ui,
    strip: Rect,
    track: Rect,
    geometry: &TimelineGeometry,
    view: &mut StoryMapViewState,
    state: &mut StoryMapScrollbarState,
    palette: EditorScrollbarPalette,
) -> bool {
    let ctx = ui.ctx();
    let (hover_pointer, interact_pointer, latest_pointer, pressed, released, down, escape, focused, gone, moved_inside) = ctx.input(|input| {
        (
            input.pointer.hover_pos(),
            input.pointer.interact_pos(),
            input.pointer.latest_pos(),
            input.pointer.primary_pressed(),
            input.pointer.primary_released(),
            input.pointer.primary_down(),
            input.key_pressed(egui::Key::Escape),
            input.viewport().focused != Some(false),
            input.events.iter().any(|event| matches!(event, egui::Event::PointerGone)),
            input.events.iter().any(|event| {
                matches!(event, egui::Event::PointerMoved(position) if strip.contains(*position))
            }),
        )
    });
    let now = Instant::now();
    let finite = |point: Option<Pos2>| {
        point.filter(|point| point.x.is_finite() && point.y.is_finite())
    };
    let hover_pointer = finite(hover_pointer);
    let interact_pointer = finite(interact_pointer);
    let moved = !gone
        && moved_inside
        && hover_pointer.is_some_and(|position| state.last_pointer != Some(position));
    let thumb = thumb_geometry(geometry, track);
    if moved && thumb.is_some() {
        state.reveal(ctx, now);
    }
    let track_hit_rect = hit_rect(track, track, strip);
    let thumb_hit_rect = thumb.and_then(|bar| hit_rect(bar.thumb, bar.track, strip));
    let pointer_on_bar = !gone
        && hover_pointer.is_some_and(|position| {
            track_hit_rect.is_some_and(|rect| rect.contains(position))
        });
    let deadline_active = state.hide_at.is_some_and(|deadline| now < deadline);
    state.visible = thumb.is_some()
        && (state.capture.is_some() || deadline_active || pointer_on_bar);
    if !state.visible {
        state.hovered = false;
    }

    let interaction_rect = if state.visible && thumb.is_some() {
        track_hit_rect.filter(|rect| rect.is_positive())
    } else {
        None
    };
    let response = interaction_rect.map(|rect| {
        ui.interact(
            rect,
            ui.id().with("story-map-horizontal-scrollbar"),
            egui::Sense::click_and_drag(),
        )
    });
    let response_pointer = response
        .as_ref()
        .and_then(egui::Response::interact_pointer_pos)
        .and_then(|position| finite(Some(position)));
    let response_primary_down = response
        .as_ref()
        .is_some_and(egui::Response::is_pointer_button_down_on);
    let capture_pointer = response_pointer
        .or(interact_pointer)
        .or_else(|| finite(latest_pointer));
    if let Some(position) = capture_pointer.or(hover_pointer) {
        state.last_pointer = Some(position);
    }

    let compatibility = state
        .capture
        .map(|capture| capture_compatibility(capture, thumb, geometry));
    if compatibility == Some(CaptureCompatibility::DomainRebase) {
        if let (Some(mut capture), Some(bar)) = (state.capture, thumb) {
            capture.domain_pages = geometry.domain_pages;
            capture.thumb_length = bar.thumb_length;
            capture.max_origin = bar.max_origin;
            state.capture = Some(capture);
        }
    }
    let incompatible_capture = matches!(
        compatibility,
        Some(
            CaptureCompatibility::InvalidThumb
                | CaptureCompatibility::IncompatibleLayout
                | CaptureCompatibility::DerivedMismatch
        )
    );
    if escape || !focused || incompatible_capture || thumb.is_none() {
        let was_gesture = state.capture.is_some() || state.consumed_press;
        state.capture = None;
        state.hovered = false;
        if was_gesture && down && !released {
            state.consumed_press = true;
            return true;
        }
        state.consumed_press = false;
        if thumb.is_none() {
            state.visible = false;
        }
        return was_gesture;
    }

    if gone && state.capture.is_none() {
        state.hovered = false;
        if state.consumed_press {
            if released || !down {
                state.consumed_press = false;
            }
            return true;
        }
        return false;
    }

    if let Some(capture) = state.capture {
        if released || !down {
            if let (Some(bar), Some(position)) = (thumb, capture_pointer) {
                if let Some(origin) = mapped_origin(bar, position.x, capture.grab_fraction) {
                    if set_origin(view, geometry, origin) {
                        state.observe_view(ctx, *view);
                        ctx.request_repaint();
                    }
                }
            }
            state.capture = None;
            state.consumed_press = false;
            state.hovered = hover_pointer
                .is_some_and(|position| thumb_hit_rect.is_some_and(|rect| rect.contains(position)));
            return true;
        }
        let drag_position = capture_pointer.or(state.last_pointer);
        if let (Some(bar), Some(position)) = (thumb, drag_position) {
            if let Some(origin) = mapped_origin(bar, position.x, capture.grab_fraction) {
                if set_origin(view, geometry, origin) {
                    state.observe_view(ctx, *view);
                    ctx.request_repaint();
                }
            }
        }
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        return true;
    }
    if state.consumed_press {
        if released || !down {
            state.consumed_press = false;
        }
        return true;
    }
    if !state.visible {
        return false;
    }
    let Some(bar) = thumb else {
        return false;
    };
    let hit = hover_pointer
        .is_some_and(|position| thumb_hit_rect.is_some_and(|rect| rect.contains(position)));
    state.hovered = hit;
    if hit {
        ctx.set_cursor_icon(if pressed {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    } else if hover_pointer.is_some_and(|position| {
        track_hit_rect.is_some_and(|rect| rect.contains(position))
    }) {
        ctx.set_cursor_icon(egui::CursorIcon::Default);
    }
    if pressed && down {
        let activation_position = response_pointer
            .or(interact_pointer)
            .or_else(|| finite(latest_pointer))
            .or(hover_pointer);
        if let Some(position) = activation_position {
            let hit = thumb_hit_rect.is_some_and(|rect| rect.contains(position));
            let track_hit = response_primary_down
                && interaction_rect.is_some_and(|rect| rect.contains(position));
            if hit || (!hit && track_hit) {
                let fraction = if hit {
                    grab_fraction(position, bar)
                } else {
                    Some(0.5)
                };
                if let Some(grab_fraction) = fraction {
                    state.capture = Some(Capture {
                        grab_fraction,
                        track_left: bar.track.left(),
                        track_top: bar.track.top(),
                        track_length: bar.track.width(),
                        thumb_length: bar.thumb_length,
                        max_origin: bar.max_origin,
                        target_pages: geometry.target_pages,
                        domain_pages: geometry.domain_pages,
                        timeline_left: geometry.timeline_left,
                        viewport_width: geometry.viewport_width,
                        page_pitch: geometry.page_pitch,
                        zoom_percent: geometry.zoom_percent,
                    });
                    state.consumed_press = true;
                    if !hit {
                        if let Some(origin) = mapped_origin(bar, position.x, grab_fraction) {
                            if set_origin(view, geometry, origin) {
                                state.observe_view(ctx, *view);
                                ctx.request_repaint();
                            }
                        }
                    }
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn paint(
    ui: &egui::Ui,
    geometry: &TimelineGeometry,
    track: Rect,
    state: &StoryMapScrollbarState,
    palette: EditorScrollbarPalette,
) {
    if !state.visible {
        return;
    }
    let Some(bar) = thumb_geometry(geometry, track) else {
        return;
    };
    let color = if state.capture.is_some() {
        palette.dragged_handle
    } else if state.hovered {
        palette.hovered_handle
    } else {
        palette.resting_handle
    };
    let thumb = bar.thumb.shrink2(egui::vec2(0.0, ((bar.thumb.height() - STORY_MAP_SCROLLBAR_THICKNESS) / 2.0).max(0.0)));
    let painter = ui.painter().with_clip_rect(track);
    let radius = STORY_MAP_SCROLLBAR_THUMB_CORNER_RADIUS
        .min((thumb.width().min(thumb.height()) / 2.0).max(0.0));
    painter.rect_filled(thumb, radius, color);
}
