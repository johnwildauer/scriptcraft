//! Raw-input adapter for wheel-driven editor zoom.
//!
//! The adapter owns only the transient middle-button gesture state. Zoom is
//! routed by the application through the same `ViewZoomPinch` command used by
//! native pinch input.

use eframe::egui;

pub(super) const POINTS_PER_LINE: f32 = 40.0;
pub(super) const POINTS_PER_ZOOM_E_FOLD: f32 = 200.0;

#[derive(Debug, Default)]
pub(crate) struct WheelZoomAdapter {
    middle_button_down: bool,
    owned_middle_button_down: bool,
}

impl WheelZoomAdapter {
    pub(crate) fn reset(&mut self) {
        self.middle_button_down = false;
        self.owned_middle_button_down = false;
    }

    pub(super) fn middle_button_down(&self) -> bool {
        self.middle_button_down
    }

    /// Removes matched vertical zoom-wheel events and returns only valid
    /// multiplicative factors. State transitions are processed in event order.
    pub(crate) fn adapt_events(
        &mut self,
        events: &mut Vec<egui::Event>,
        viewport_height_points: f32,
    ) -> Vec<f32> {
        self.adapt_events_inner(events, viewport_height_points, None)
    }

    /// Root Body variant: retain owned middle events for the shared surface
    /// gesture while preventing this adapter from latching their press/release.
    pub(crate) fn adapt_events_filtered(
        &mut self,
        events: &mut Vec<egui::Event>,
        viewport_height_points: f32,
        mut owner_for_press: impl FnMut(Option<egui::Pos2>) -> Option<crate::presentation::auto_scroll::AutoScrollOwner>,
    ) -> Vec<f32> {
        self.adapt_events_inner(events, viewport_height_points, Some(&mut owner_for_press))
    }

    fn adapt_events_inner(
        &mut self,
        events: &mut Vec<egui::Event>,
        viewport_height_points: f32,
        mut owner_for_press: Option<&mut dyn FnMut(Option<egui::Pos2>) -> Option<crate::presentation::auto_scroll::AutoScrollOwner>>,
    ) -> Vec<f32> {
        let mut retained = Vec::with_capacity(events.len());
        let mut factors = Vec::new();

        for event in std::mem::take(events) {
            match &event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Middle,
                    pressed,
                    ..
                } => {
                    if *pressed {
                        let owner = owner_for_press
                            .as_deref_mut()
                            .and_then(|route| {
                                route((pos.x.is_finite() && pos.y.is_finite()).then_some(*pos))
                            });
                        if owner.is_some() {
                            self.middle_button_down = false;
                            self.owned_middle_button_down = true;
                        } else {
                            self.owned_middle_button_down = false;
                            self.middle_button_down = true;
                        }
                    } else if self.owned_middle_button_down {
                        self.owned_middle_button_down = false;
                        self.middle_button_down = false;
                    } else {
                        self.middle_button_down = false;
                    }
                }
                egui::Event::PointerGone | egui::Event::WindowFocused(false) => {
                    self.reset();
                }
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                    ..
                } => {
                    let matched = modifiers.command || self.middle_button_down;
                    if !matched {
                        retained.push(event);
                        continue;
                    }

                    // A matched non-finite event is consumed defensively, even
                    // if its vertical component would otherwise be zero.
                    if !delta.x.is_finite() || !delta.y.is_finite() {
                        continue;
                    }
                    // Horizontal-only wheel gestures remain ordinary scrolling.
                    if delta.y == 0.0 {
                        retained.push(event);
                        continue;
                    }

                    // Any matched nonzero vertical component is consumed. A
                    // bad page height, factor overflow, or underflow therefore
                    // cannot leak invalid data into egui's scroll processing.
                    let Some(points) =
                        normalized_vertical_points(*unit, delta.y, viewport_height_points)
                    else {
                        continue;
                    };
                    let factor = (points / POINTS_PER_ZOOM_E_FOLD).exp();
                    if factor.is_finite() && factor > 0.0 {
                        factors.push(factor);
                    }
                    continue;
                }
                _ => {}
            }
            retained.push(event);
        }

        *events = retained;
        factors
    }
}

fn normalized_vertical_points(
    unit: egui::MouseWheelUnit,
    vertical_delta: f32,
    viewport_height_points: f32,
) -> Option<f32> {
    let points = match unit {
        egui::MouseWheelUnit::Point => vertical_delta,
        egui::MouseWheelUnit::Line => vertical_delta * POINTS_PER_LINE,
        egui::MouseWheelUnit::Page => {
            if !viewport_height_points.is_finite() || viewport_height_points <= 0.0 {
                return None;
            }
            vertical_delta * viewport_height_points
        }
    };
    points.is_finite().then_some(points)
}
