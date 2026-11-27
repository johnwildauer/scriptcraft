//! Immediate, disposable navigation for the shared Story Map timeline.

use eframe::egui::{self, Pos2, Rect, Response};
use scriptcraft_fdx::StoryMapViewState;
use scriptcraft_model::{BoardFixed, MAX_BOARD_DIMENSION};

use super::{
    constants::*,
    geometry::{StripGeometry, TimelineGeometry},
    paint,
    scrollbar::{self, StoryMapScrollbarState},
};
use crate::presentation::auto_scroll::AutoScrollGesture;
use crate::chrome::theme::{ApplicationPalette, EditorScrollbarPalette};

fn fixed_milli(value: f64) -> Option<BoardFixed> {
    let max_pixels = MAX_BOARD_DIMENSION.raw() as f64 / BoardFixed::SCALE as f64;
    if !value.is_finite() || value < 0.0 || value > max_pixels {
        return None;
    }
    let raw = (value * BoardFixed::SCALE as f64).round();
    (raw.is_finite() && raw >= 0.0 && raw <= MAX_BOARD_DIMENSION.raw() as f64)
        .then_some(BoardFixed::from_milli_pixels(raw as i64))
}

fn fixed_origin(value: f64, maximum: f64) -> Option<BoardFixed> {
    if !value.is_finite() || !maximum.is_finite() || value < 0.0 || maximum < 0.0 {
        return None;
    }
    let raw = (value.min(maximum) * BoardFixed::SCALE as f64).floor();
    (raw.is_finite() && raw >= 0.0 && raw <= MAX_BOARD_DIMENSION.raw() as f64)
        .then_some(BoardFixed::from_milli_pixels(raw as i64))
}

fn milli_percent(value: f64, minimum: f64, maximum: f64) -> Option<u32> {
    if !value.is_finite() || value <= 0.0 || value > 120_000.0 {
        return None;
    }
    let lower = (minimum * 1_000.0).ceil();
    let upper = (maximum * 1_000.0).floor();
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return None;
    }
    let milli = (value * 1_000.0).round().clamp(lower, upper);
    (milli.is_finite() && milli >= 1.0 && milli <= 120_000_000.0)
        .then_some(milli as u32)
}

fn state_with_geometry(
    view: StoryMapViewState,
    geometry: &TimelineGeometry,
    width: f64,
) -> Option<StoryMapViewState> {
    let width = fixed_milli(width)?;
    let origin = fixed_origin(geometry.origin, geometry.max_origin)?;
    let zoom = milli_percent(geometry.zoom_percent, geometry.zoom_min, geometry.zoom_max)?;
    StoryMapViewState::new(view.height(), Some(width), origin, zoom)
}

fn derive_and_clamp(
    strip: &StripGeometry,
    target_pages: f64,
    domain_pages: f64,
    view: &mut StoryMapViewState,
    pinch_zoom_percent: &mut f64,
) -> Option<TimelineGeometry> {
    let viewport = strip.timeline_viewport;
    let width = f64::from(viewport.width());
    if !viewport.left().is_finite() || !width.is_finite() || width <= 0.0 {
        return None;
    }
    let prior_zoom = f64::from(view.zoom_milli_percent()) / 1_000.0;
    let prior_origin = view.scroll_origin().raw() as f64 / BoardFixed::SCALE as f64;
    let geometry = TimelineGeometry::new(
        target_pages,
        domain_pages,
        f64::from(viewport.left()),
        width,
        prior_zoom,
        prior_origin,
    )?;
    let next = state_with_geometry(*view, &geometry, width)?;
    let committed = TimelineGeometry::new(
        target_pages,
        domain_pages,
        f64::from(viewport.left()),
        width,
        f64::from(next.zoom_milli_percent()) / 1_000.0,
        next.scroll_origin().raw() as f64 / BoardFixed::SCALE as f64,
    )?;
    if next.zoom_milli_percent() != view.zoom_milli_percent() {
        *pinch_zoom_percent = geometry.zoom_percent;
    }
    *view = next;
    Some(committed)
}

fn navigation_track(strip: &StripGeometry, slider: Rect) -> Option<Rect> {
    let row = strip.scrollbar_row;
    let left = strip.timeline_viewport.left();
    let right = slider.left().min(strip.timeline_viewport.right());
    if !left.is_finite() || !right.is_finite() || !row.is_positive() {
        return None;
    }
    let center_y = row.center().y;
    let height = STORY_MAP_SCROLLBAR_THICKNESS.min(row.height());
    let top = center_y - height / 2.0;
    let track = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, top + height));
    Some(track)
}

fn slider_rect(strip: &StripGeometry) -> Option<Rect> {
    let row = strip.scrollbar_row;
    let right = strip.timeline_viewport.right() - STORY_MAP_SLIDER_TRAILING_PADDING;
    let left = (right - STORY_MAP_SLIDER_WIDTH).max(strip.timeline_viewport.left());
    if !right.is_finite() || !left.is_finite() || right <= left || !row.is_positive() {
        return None;
    }
    Some(Rect::from_min_max(
        Pos2::new(left, row.top()),
        Pos2::new(right, row.bottom()),
    ))
}

fn wheel_points(unit: egui::MouseWheelUnit, delta: egui::Vec2, page_width: f64) -> Option<f64> {
    let raw = if delta.x != 0.0 { delta.x } else { delta.y };
    if !raw.is_finite() || raw == 0.0 {
        return None;
    }
    let multiplier = match unit {
        egui::MouseWheelUnit::Point => 1.0,
        egui::MouseWheelUnit::Line => STORY_MAP_WHEEL_POINTS_PER_LINE,
        egui::MouseWheelUnit::Page => page_width,
    };
    let points = f64::from(raw) * multiplier;
    points.is_finite().then_some(points)
}

fn handle_wheel(
    ui: &egui::Ui,
    timeline_rect: Rect,
    geometry: TimelineGeometry,
    view: &mut StoryMapViewState,
    scrollbars: &mut StoryMapScrollbarState,
    auto_scroll: &mut AutoScrollGesture,
) -> (TimelineGeometry, bool) {
    let ctx = ui.ctx();
    let pointer = ctx.input(|input| input.pointer.interact_pos());
    let over_timeline = pointer.is_some_and(|position| {
        position.x.is_finite() && position.y.is_finite() && timeline_rect.contains(position)
    });
    if !over_timeline {
        return (geometry, false);
    }
    let events = ctx.input(|input| input.events.clone());
    let mut next = geometry;
    let mut consumed = false;
    for event in events {
        if let egui::Event::MouseWheel { unit, delta, .. } = event {
            let Some(points) = wheel_points(unit, delta, geometry.viewport_width) else {
                continue;
            };
            let Some(candidate) = next.with_origin(next.origin - points) else {
                continue;
            };
            if let Some(new_state) = state_with_geometry(
                *view,
                &candidate,
                geometry.viewport_width,
            ) {
                if let Some(committed) = TimelineGeometry::new(
                        geometry.target_pages,
                        geometry.domain_pages,
                        geometry.timeline_left,
                        geometry.viewport_width,
                        f64::from(new_state.zoom_milli_percent()) / 1_000.0,
                        new_state.scroll_origin().raw() as f64 / BoardFixed::SCALE as f64,
                ) {
                    auto_scroll.cancel();
                    if new_state != *view {
                        *view = new_state;
                        next = committed;
                        scrollbars.observe_view(ctx, *view);
                    }
                    consumed = true;
                }
            }
        }
    }
    if consumed {
        ctx.input_mut(|input| {
            input.events.retain(|event| {
                !matches!(event, egui::Event::MouseWheel { .. })
            });
            input.raw.events.clone_from(&input.events);
            if !input
                .raw
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::MouseWheel { .. }))
            {
                input.smooth_scroll_delta = egui::Vec2::ZERO;
            }
        });
        ctx.request_repaint();
    }
    (next, consumed)
}

fn handle_slider(
    ui: &mut egui::Ui,
    rect: Rect,
    geometry: TimelineGeometry,
    view: &mut StoryMapViewState,
    pinch_zoom_percent: &mut f64,
    scrollbars: &mut StoryMapScrollbarState,
    palette: ApplicationPalette,
    auto_scroll: &mut AutoScrollGesture,
) -> (TimelineGeometry, Option<Response>, bool) {
    if !rect.is_positive() {
        return (geometry, None, false);
    }
    let mut zoom = geometry.zoom_percent;
    let range_is_active = geometry.zoom_max > geometry.zoom_min;
    let safe_slider_max = if range_is_active {
        geometry.zoom_max
    } else {
        geometry.zoom_min + 1.0
    };
    let response = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(*ui.layout()),
            |ui| {
            ui.spacing_mut().slider_rail_height = STORY_MAP_SLIDER_RAIL_HEIGHT;
            ui.spacing_mut().slider_width = rect.width();
            ui.visuals_mut().widgets.inactive.bg_fill = palette.status.slider_rail;
            ui.visuals_mut().selection.bg_fill = palette.status.slider_trailing_fill;
            ui.add_enabled_ui(range_is_active, |ui| {
                ui.add_sized(
                    rect.size(),
                    egui::Slider::new(&mut zoom, geometry.zoom_min..=safe_slider_max)
                        .step_by(0.001)
                        .trailing_fill(true)
                        .show_value(false)
                        .handle_shape(egui::style::HandleShape::Circle),
                )
            })
            .inner
            },
        )
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::slider(response.enabled(), zoom, "Story Map zoom")
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_label("Story Map zoom");
        node.set_numeric_value(zoom);
        node.set_min_numeric_value(geometry.zoom_min);
        node.set_max_numeric_value(geometry.zoom_max);
        node.set_numeric_value_step(0.001);
    });
    paint::paint_zoom_slider_knob(ui, rect, &response, zoom, geometry.zoom_min, geometry.zoom_max, palette);

    if response.changed() && zoom.is_finite() {
        let stored_zoom = milli_percent(zoom, geometry.zoom_min, geometry.zoom_max)
            .map(|value| f64::from(value) / 1_000.0);
        if let Some(anchored) = stored_zoom.and_then(|value| geometry.zoom_around_center(value)) {
            if let Some(next) = state_with_geometry(*view, &anchored, geometry.viewport_width) {
                if let Some(committed) = TimelineGeometry::new(
                    geometry.target_pages,
                    geometry.domain_pages,
                    geometry.timeline_left,
                    geometry.viewport_width,
                    f64::from(next.zoom_milli_percent()) / 1_000.0,
                    next.scroll_origin().raw() as f64 / BoardFixed::SCALE as f64,
                ) {
                    auto_scroll.cancel();
                    *pinch_zoom_percent = zoom;
                    if next != *view {
                        *view = next;
                        scrollbars.observe_view(ui.ctx(), *view);
                        ui.ctx().request_repaint();
                    }
                    return (committed, Some(response), true);
                }
            }
        }
    }
    (geometry, Some(response), false)
}

/// Applies current-width clamps, slider/wheel navigation, and thumb input. The
/// returned transform is the one paint and later ruler/Beat/scene consumers share,
/// alongside whether the scrollbar consumed the current pointer gesture.
pub(crate) fn compose_navigation(
    ui: &mut egui::Ui,
    strip: &StripGeometry,
    target_pages: f64,
    domain_pages: f64,
    view: &mut StoryMapViewState,
    pinch_zoom_percent: &mut f64,
    scrollbars: &mut StoryMapScrollbarState,
    auto_scroll: &mut AutoScrollGesture,
    palette: ApplicationPalette,
) -> Option<(TimelineGeometry, Rect, bool, bool)> {
    let slider = slider_rect(strip)?;
    let mut timeline = derive_and_clamp(
        strip,
        target_pages,
        domain_pages,
        view,
        pinch_zoom_percent,
    )?;
    let track = navigation_track(strip, slider)?;
    scrollbars.observe_view(ui.ctx(), *view);

    let (after_wheel, wheel_consumed) = handle_wheel(
        ui,
        strip.timeline_viewport,
        timeline,
        view,
        scrollbars,
        auto_scroll,
    );
    timeline = after_wheel;
    let (after_slider, _, slider_changed) = handle_slider(
        ui,
        slider,
        timeline,
        view,
        pinch_zoom_percent,
        scrollbars,
        palette,
        auto_scroll,
    );
    timeline = after_slider;

    let palette_scrollbar: EditorScrollbarPalette = palette.script_workspace.editor_scrollbar;
    let primary_pressed_on_navigation_row = ui.ctx().input(|input| {
        input.pointer.primary_pressed()
            && input.pointer.interact_pos().is_some_and(|position| strip.scrollbar_row.contains(position))
    });
    if primary_pressed_on_navigation_row || scrollbars.consumes_pointer() {
        auto_scroll.cancel();
    }
    let consumed = scrollbar::handle(
        ui,
        strip.rect,
        track,
        &timeline,
        view,
        scrollbars,
        palette_scrollbar,
    );
    if consumed {
        auto_scroll.cancel();
    }
    // Thumb capture may have updated the origin; derive once more from the same
    // immutable policy so the geometry returned to all consumers is current.
    let timeline = TimelineGeometry::new(
        target_pages,
        domain_pages,
        timeline.timeline_left,
        timeline.viewport_width,
        f64::from(view.zoom_milli_percent()) / 1_000.0,
        view.scroll_origin().raw() as f64 / BoardFixed::SCALE as f64,
    )?;
    scrollbars.observe_view(ui.ctx(), *view);
    scrollbar::paint(ui, &timeline, track, scrollbars, palette_scrollbar);
    Some((timeline, track, consumed, wheel_consumed || slider_changed || consumed))
}

/// Apply only the horizontal component through the same fixed-origin state path
/// used by wheel and thumb navigation, then reconcile from the stored state.
pub(crate) fn commit_auto_scroll_delta(
    ctx: &egui::Context,
    timeline: TimelineGeometry,
    delta: egui::Vec2,
    view: &mut StoryMapViewState,
    scrollbars: &mut StoryMapScrollbarState,
    gesture: &mut AutoScrollGesture,
) -> Option<TimelineGeometry> {
    if !delta.x.is_finite() || !delta.y.is_finite() {
        gesture.reset();
        return None;
    }
    if delta.x == 0.0 {
        return Some(timeline);
    }
    let requested = timeline.origin + f64::from(delta.x);
    if !requested.is_finite() {
        gesture.reset();
        return None;
    }
    let Some(candidate) = timeline.with_origin(requested) else {
        gesture.reset();
        return None;
    };
    let next = state_with_geometry(*view, &candidate, timeline.viewport_width);
    let Some(next) = next else {
        gesture.reset();
        return None;
    };
    let committed = TimelineGeometry::new(
        timeline.target_pages,
        timeline.domain_pages,
        timeline.timeline_left,
        timeline.viewport_width,
        f64::from(next.zoom_milli_percent()) / 1_000.0,
        next.scroll_origin().raw() as f64 / BoardFixed::SCALE as f64,
    );
    let Some(committed) = committed else {
        gesture.reset();
        return None;
    };
    if next != *view {
        *view = next;
        scrollbars.observe_view(ctx, *view);
        ctx.request_repaint();
    }
    Some(committed)
}

/// Commit an Outline-owned continuous pinch through the existing fixed-state path.
/// The returned zoom is the clamped continuous value, not its milli-percent rounding.
pub(crate) fn commit_pinch_zoom(
    view: &mut StoryMapViewState,
    timeline: TimelineGeometry,
    requested_zoom: f64,
    pointer_x: Option<f64>,
) -> Option<(TimelineGeometry, bool, f64)> {
    if !requested_zoom.is_finite() || requested_zoom <= 0.0 {
        return None;
    }
    let requested_zoom = requested_zoom.clamp(timeline.zoom_min, timeline.zoom_max);
    let candidate = pointer_x
        .filter(|x| x.is_finite())
        .and_then(|x| timeline.zoom_around_pointer(requested_zoom, x))
        .or_else(|| timeline.zoom_around_center(requested_zoom))?;
    let continuous_zoom = candidate.zoom_percent;
    if !continuous_zoom.is_finite() || continuous_zoom <= 0.0 {
        return None;
    }
    let next = state_with_geometry(*view, &candidate, timeline.viewport_width)?;
    let committed = TimelineGeometry::new(
        timeline.target_pages,
        timeline.domain_pages,
        timeline.timeline_left,
        timeline.viewport_width,
        f64::from(next.zoom_milli_percent()) / 1_000.0,
        next.scroll_origin().raw() as f64 / BoardFixed::SCALE as f64,
    )?;
    let changed = next != *view;
    *view = next;
    Some((committed, changed, continuous_zoom))
}
