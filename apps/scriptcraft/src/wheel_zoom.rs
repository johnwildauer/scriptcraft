//! Raw-input adapter for wheel-driven editor zoom.
//!
//! The adapter owns only the transient middle-button gesture state. Zoom is
//! routed by the application through the same `ViewZoomPinch` command used by
//! native pinch input.

use eframe::egui;

pub(super) const POINTS_PER_LINE: f32 = 40.0;
pub(super) const POINTS_PER_ZOOM_E_FOLD: f32 = 200.0;

#[derive(Debug, Default)]
pub(super) struct WheelZoomAdapter {
    middle_button_down: bool,
}

impl WheelZoomAdapter {
    pub(super) fn reset(&mut self) {
        self.middle_button_down = false;
    }

    pub(super) fn middle_button_down(&self) -> bool {
        self.middle_button_down
    }

    /// Removes matched vertical zoom-wheel events and returns only valid
    /// multiplicative factors. State transitions are processed in event order.
    pub(super) fn adapt_events(
        &mut self,
        events: &mut Vec<egui::Event>,
        viewport_height_points: f32,
    ) -> Vec<f32> {
        let mut retained = Vec::with_capacity(events.len());
        let mut factors = Vec::new();

        for event in std::mem::take(events) {
            match &event {
                egui::Event::PointerButton {
                    button: egui::PointerButton::Middle,
                    pressed,
                    ..
                } => self.middle_button_down = *pressed,
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
                    let Some(points) = normalized_vertical_points(
                        *unit,
                        delta.y,
                        viewport_height_points,
                    ) else {
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

#[cfg(test)]
mod tests {
    use super::{normalized_vertical_points, WheelZoomAdapter, POINTS_PER_LINE, POINTS_PER_ZOOM_E_FOLD};
    use eframe::egui::{self, Event, Modifiers, MouseWheelUnit, PointerButton, Vec2};

    fn wheel(unit: MouseWheelUnit, delta: Vec2, modifiers: Modifiers) -> Event {
        Event::MouseWheel {
            unit,
            delta,
            modifiers,
            phase: egui::TouchPhase::Move,
        }
    }

    fn command_modifiers() -> Modifiers {
        Modifiers {
            command: true,
            ..Modifiers::NONE
        }
    }

    fn no_modifiers() -> Modifiers {
        Modifiers::NONE
    }

    fn wheel_y(points: f32) -> Event {
        wheel(MouseWheelUnit::Point, Vec2::new(0.0, points), command_modifiers())
    }

    #[test]
    fn windows_linux_control_and_macos_command_use_only_logical_command() {
        let mut adapter = WheelZoomAdapter::default();
        let windows_linux_control = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        let mut events = vec![wheel(
            MouseWheelUnit::Point,
            Vec2::new(0.0, 10.0),
            windows_linux_control,
        )];
        assert_eq!(adapter.adapt_events(&mut events, 700.0).len(), 1);
        assert!(events.is_empty());

        let macos_command = Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        };
        let mut events = vec![wheel(
            MouseWheelUnit::Point,
            Vec2::new(0.0, 10.0),
            macos_command,
        )];
        assert_eq!(adapter.adapt_events(&mut events, 700.0).len(), 1);
        assert!(events.is_empty());

        // Simulates literal Control on macOS: ctrl is set, logical command is
        // not. It remains an ordinary wheel event instead of becoming a zoom.
        let literal_control = Modifiers { ctrl: true, ..Modifiers::NONE };
        let mut events = vec![wheel(MouseWheelUnit::Point, Vec2::new(0.0, 10.0), literal_control)];
        assert!(adapter.adapt_events(&mut events, 700.0).is_empty());
        assert_eq!(events.len(), 1);

        // Logical command represents Control on Windows/Linux and Command on
        // macOS; physical modifier fields do not override that semantic bit.
        let logical_command = Modifiers { command: true, ..Modifiers::NONE };
        let mut events = vec![wheel(MouseWheelUnit::Point, Vec2::new(0.0, 10.0), logical_command)];
        assert_eq!(adapter.adapt_events(&mut events, 700.0).len(), 1);
        assert!(events.is_empty());
    }

    #[test]
    fn middle_button_wheel_works_without_modifier_and_release_clears_state() {
        let mut adapter = WheelZoomAdapter::default();
        let mut events = vec![
            Event::PointerButton {
                pos: egui::Pos2::ZERO,
                button: PointerButton::Middle,
                pressed: true,
                modifiers: no_modifiers(),
            },
            wheel(MouseWheelUnit::Point, Vec2::new(0.0, 1.0), no_modifiers()),
            Event::PointerButton {
                pos: egui::Pos2::ZERO,
                button: PointerButton::Middle,
                pressed: false,
                modifiers: no_modifiers(),
            },
            wheel(MouseWheelUnit::Point, Vec2::new(0.0, 1.0), no_modifiers()),
        ];
        let factors = adapter.adapt_events(&mut events, 700.0);
        assert_eq!(factors.len(), 1);
        assert!(!adapter.middle_button_down());
        assert_eq!(events.len(), 3); // Both button events and ordinary wheel remain.
    }

    #[test]
    fn middle_button_state_resets_on_pointer_loss_focus_loss_and_viewport_close() {
        let mut adapter = WheelZoomAdapter::default();
        for reset_event in [Event::PointerGone, Event::WindowFocused(false)] {
            let mut events = vec![
                Event::PointerButton {
                    pos: egui::Pos2::ZERO,
                    button: PointerButton::Middle,
                    pressed: true,
                    modifiers: no_modifiers(),
                },
                reset_event,
                wheel(MouseWheelUnit::Point, Vec2::new(0.0, 1.0), no_modifiers()),
            ];
            assert!(adapter.adapt_events(&mut events, 700.0).is_empty());
            assert!(!adapter.middle_button_down());
            assert_eq!(events.len(), 3);
        }

        let mut events = vec![Event::PointerButton {
            pos: egui::Pos2::ZERO,
            button: PointerButton::Middle,
            pressed: true,
            modifiers: no_modifiers(),
        }];
        adapter.adapt_events(&mut events, 700.0);
        adapter.reset(); // Called by the viewport-close path.
        assert!(!adapter.middle_button_down());
    }

    #[test]
    fn direction_units_and_exp_sensitivity_are_explicit() {
        assert_eq!(normalized_vertical_points(MouseWheelUnit::Point, 0.25, 700.0), Some(0.25));
        assert_eq!(normalized_vertical_points(MouseWheelUnit::Line, 2.0, 700.0), Some(2.0 * POINTS_PER_LINE));
        assert_eq!(normalized_vertical_points(MouseWheelUnit::Page, 0.5, 700.0), Some(350.0));

        let mut adapter = WheelZoomAdapter::default();
        let mut events = vec![wheel_y(100.0), wheel_y(-100.0)];
        let factors = adapter.adapt_events(&mut events, 700.0);
        assert!((factors[0] - (100.0_f32 / POINTS_PER_ZOOM_E_FOLD).exp()).abs() < 0.0001);
        assert!((factors[1] - (-100.0_f32 / POINTS_PER_ZOOM_E_FOLD).exp()).abs() < 0.0001);
        assert!(factors[0] > 1.0);
        assert!(factors[1] < 1.0);
    }

    #[test]
    fn small_large_and_multiple_deltas_produce_one_factor_per_valid_event() {
        let mut adapter = WheelZoomAdapter::default();
        let mut events = vec![wheel_y(0.01), wheel_y(120.0), wheel_y(-40.0)];
        let factors = adapter.adapt_events(&mut events, 700.0);
        assert_eq!(factors.len(), 3);
        assert!(factors[0] > 1.0);
        assert!(factors[1] > factors[0]);
        assert!(factors[2] < 1.0);
    }

    #[test]
    fn invalid_matched_and_horizontal_only_input_obey_consumption_contract() {
        let mut adapter = WheelZoomAdapter::default();
        let mut events = vec![
            wheel(MouseWheelUnit::Point, Vec2::new(1.0, 0.0), command_modifiers()),
            wheel(MouseWheelUnit::Point, Vec2::new(0.0, 0.0), command_modifiers()),
            wheel(MouseWheelUnit::Point, Vec2::new(0.0, f32::NAN), command_modifiers()),
            wheel(MouseWheelUnit::Point, Vec2::new(0.0, f32::INFINITY), command_modifiers()),
            wheel(MouseWheelUnit::Point, Vec2::new(0.0, f32::MAX), command_modifiers()),
            wheel(MouseWheelUnit::Page, Vec2::new(0.0, 1.0), command_modifiers()),
        ];
        let factors = adapter.adapt_events(&mut events, f32::NAN);
        assert!(factors.is_empty());
        // Horizontal, zero-vertical, and invalid vertical events are all
        // retained or consumed according to their specific input contract.
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::MouseWheel { delta, .. } if delta.x == 1.0 && delta.y == 0.0));
        assert!(matches!(events[1], Event::MouseWheel { delta, .. } if delta == Vec2::ZERO));
    }

    #[test]
    fn unmatched_wheels_are_retained_in_original_order_and_invalid_data_is_not_consumed() {
        let mut adapter = WheelZoomAdapter::default();
        let before = Event::PointerMoved(egui::pos2(2.0, 3.0));
        let ordinary = wheel(MouseWheelUnit::Line, Vec2::new(0.0, f32::NAN), no_modifiers());
        let horizontal = wheel(MouseWheelUnit::Point, Vec2::new(4.0, 0.0), no_modifiers());
        let after = Event::PointerGone;
        let mut events = vec![before.clone(), ordinary.clone(), horizontal.clone(), after.clone()];
        assert!(adapter.adapt_events(&mut events, 700.0).is_empty());
        assert_eq!(events, vec![before, ordinary, horizontal, after]);
    }

    #[test]
    fn existing_pinch_zoom_bounds_clamp_wheel_factors_in_both_directions() {
        assert_eq!(super::super::status_bar::zoom_after_native_factor(299.0, 2.0), Some(300.0));
        assert_eq!(super::super::status_bar::zoom_after_native_factor(76.0, 0.5), Some(75.0));
    }
}
