//! Shared transient middle-button anchor-scroll gesture and presentation.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

const RADIAL_DEAD_ZONE: f32 = 6.0;
const LINEAR_GAIN_POINTS_PER_SECOND: f32 = 180.0;
const MAX_AXIS_SPEED: f32 = 1_200.0;
const MAX_FRAME_DELTA: f64 = 0.05;
const ACTIVE_REPAINT: std::time::Duration = std::time::Duration::from_millis(16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Inactive,
    MiddlePressed,
    Locked,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Inactive
    }
}

/// A surface-independent gesture value. Surface ownership and eligibility stay
/// with its adapter; this value owns only the transient input lifecycle.
#[derive(Debug, Default)]
pub(crate) struct AutoScrollGesture {
    mode: Mode,
    anchor: Option<Pos2>,
    latest_pointer: Option<Pos2>,
    current_pointer: Option<Pos2>,
    held_movement: bool,
    velocity: Vec2,
    last_frame_time: Option<f64>,
    consume_middle_release: bool,
    cancel_after_events: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GestureFrame {
    pub(crate) owned_input: bool,
    pub(crate) active: bool,
    pub(crate) delta: Vec2,
}

/// The root raw-input router publishes one owner for the current pass so the
/// two independent surface adapters cannot both activate from the same press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoScrollOwner {
    Outline,
    Body,
    Beat,
}

const ROUTE_OWNER_ID: &str = "middle-auto-scroll-route-owner";

#[derive(Debug, Clone, Copy, Default)]
struct RouteOwners {
    outline: bool,
    body: bool,
    beat: bool,
}

pub(crate) fn set_route_owner(ctx: &egui::Context, owner: Option<AutoScrollOwner>) {
    ctx.data_mut(|data| {
        let id = egui::Id::new(ROUTE_OWNER_ID);
        let mut owners = if owner.is_some() {
            data.get_temp::<RouteOwners>(id).unwrap_or_default()
        } else {
            RouteOwners::default()
        };
        match owner {
            Some(AutoScrollOwner::Outline) => owners.outline = true,
            Some(AutoScrollOwner::Body) => owners.body = true,
            Some(AutoScrollOwner::Beat) => owners.beat = true,
            None => {}
        }
        data.insert_temp(id, owners);
    });
}

fn other_route_owns(ctx: &egui::Context, owner: AutoScrollOwner) -> bool {
    ctx.data(|data| {
        let owners = data
            .get_temp::<RouteOwners>(egui::Id::new(ROUTE_OWNER_ID))
            .unwrap_or_default();
        match owner {
            AutoScrollOwner::Outline => (owners.body || owners.beat) && !owners.outline,
            AutoScrollOwner::Body => (owners.outline || owners.beat) && !owners.body,
            AutoScrollOwner::Beat => (owners.outline || owners.body) && !owners.beat,
        }
    })
}

impl AutoScrollGesture {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn cancel(&mut self) {
        self.cancel_after_events = true;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.mode != Mode::Inactive
    }

    pub(crate) fn is_locked(&self) -> bool {
        self.mode == Mode::Locked
    }

    pub(crate) fn anchor(&self) -> Option<Pos2> {
        self.is_active().then_some(self.anchor).flatten()
    }

    pub(crate) fn velocity(&self) -> Vec2 {
        self.velocity
    }

    /// Process raw egui events in order, removing only middle-button events
    /// owned by this surface while keeping the processed and raw lists aligned.
    pub(crate) fn process_frame(
        &mut self,
        ctx: &egui::Context,
        eligible: impl Fn(Pos2) -> bool,
    ) -> GestureFrame {
        self.process_frame_for_owner(ctx, AutoScrollOwner::Outline, eligible)
    }

    pub(crate) fn process_body_frame(
        &mut self,
        ctx: &egui::Context,
        eligible: impl Fn(Pos2) -> bool,
    ) -> GestureFrame {
        self.process_frame_for_owner(ctx, AutoScrollOwner::Body, eligible)
    }

    pub(crate) fn process_beat_frame(
        &mut self,
        ctx: &egui::Context,
        eligible: impl Fn(Pos2) -> bool,
    ) -> GestureFrame {
        self.process_frame_for_owner(ctx, AutoScrollOwner::Beat, eligible)
    }

    fn process_frame_for_owner(
        &mut self,
        ctx: &egui::Context,
        owner: AutoScrollOwner,
        eligible: impl Fn(Pos2) -> bool,
    ) -> GestureFrame {
        if other_route_owns(ctx, owner) {
            return GestureFrame::default();
        }
        let (mut events, focused, time) = ctx.input(|input| {
            (
                input.events.clone(),
                input.viewport().focused != Some(false),
                input.time,
            )
        });
        let frame = self.process_events(
            &mut events,
            time.is_finite().then_some(time),
            focused,
            eligible,
        );
        if frame.owned_input {
            ctx.input_mut(|input| {
                input.events = events;
                input.raw.events.clone_from(&input.events);
            });
        }
        if frame.active {
            ctx.request_repaint_after(ACTIVE_REPAINT);
        }
        frame
    }

    /// Process an owned event sequence in original order. Raw-input adapters can
    /// use this before downstream wheel routing; the egui-frame adapter above
    /// additionally mirrors consumed events into egui's processed list.
    pub(crate) fn process_events(
        &mut self,
        events: &mut Vec<egui::Event>,
        time: Option<f64>,
        focused: bool,
        eligible: impl Fn(Pos2) -> bool,
    ) -> GestureFrame {
        if !focused {
            self.reset();
            return GestureFrame::default();
        }

        let mut consumed = vec![false; events.len()];
        let mut owned_input = false;
        let allow_activation = !self.cancel_after_events;
        let mut focus_lost = false;
        for (index, event) in events.iter().enumerate() {
            match event {
                egui::Event::WindowFocused(false) => {
                    focus_lost = true;
                    self.reset();
                }
                egui::Event::PointerGone => {
                    self.current_pointer = None;
                    self.velocity = Vec2::ZERO;
                }
                egui::Event::PointerMoved(position) if finite_pos(*position) => {
                    self.current_pointer = Some(*position);
                    self.latest_pointer = Some(*position);
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Middle,
                    pressed,
                    ..
                } => {
                    let position = finite_option(*pos);
                    if let Some(position) = position {
                        self.current_pointer = Some(position);
                        self.latest_pointer = Some(position);
                    }
                    match self.mode {
                        Mode::MiddlePressed if !*pressed => {
                            consumed[index] = true;
                            owned_input = true;
                            if self.anchor.zip(self.latest_pointer).is_some_and(|(anchor, pointer)| {
                                radial_distance(anchor, pointer)
                                    .is_some_and(|distance| distance > RADIAL_DEAD_ZONE)
                            }) {
                                self.held_movement = true;
                            }
                            if self.held_movement {
                                self.reset();
                            } else {
                                self.mode = Mode::Locked;
                                self.held_movement = false;
                                self.velocity = Vec2::ZERO;
                            }
                        }
                        Mode::Locked if *pressed => {
                            consumed[index] = true;
                            owned_input = true;
                            self.reset();
                            self.consume_middle_release = true;
                        }
                        Mode::Inactive if *pressed => {
                            if self.consume_middle_release {
                                consumed[index] = true;
                                owned_input = true;
                            } else if allow_activation && !focus_lost {
                                if let Some(position) = position.filter(|point| eligible(*point)) {
                                    self.activate(position, time);
                                    consumed[index] = true;
                                    owned_input = true;
                                }
                            }
                        }
                        Mode::Inactive if !*pressed && self.consume_middle_release => {
                            consumed[index] = true;
                            owned_input = true;
                            self.consume_middle_release = false;
                        }
                        _ => {}
                    }
                }
                egui::Event::PointerButton { pressed: true, .. }
                    if self.mode == Mode::Locked =>
                {
                    self.reset();
                }
                egui::Event::Key { pressed: true, .. } if self.is_active() => {
                    self.reset();
                }
                _ => {}
            }

            if self.mode == Mode::MiddlePressed {
                if let (Some(anchor), Some(pointer)) = (self.anchor, self.latest_pointer) {
                    if radial_distance(anchor, pointer).is_some_and(|distance| distance > RADIAL_DEAD_ZONE) {
                        self.held_movement = true;
                    }
                }
            }
        }

        if focus_lost {
            self.reset();
        }

        let consumed_any = consumed.iter().any(|owned| *owned);
        if consumed_any {
            let mut index = 0;
            events.retain(|_| {
                let keep = !consumed.get(index).copied().unwrap_or(false);
                index += 1;
                keep
            });
        }

        if self.cancel_after_events {
            self.reset();
            return GestureFrame { owned_input, ..GestureFrame::default() };
        }
        let delta = self.advance_frame(time);
        let active = self.is_active();
        GestureFrame {
            owned_input,
            active,
            delta,
        }
    }

    fn activate(&mut self, position: Pos2, time: Option<f64>) {
        self.mode = Mode::MiddlePressed;
        self.anchor = Some(position);
        self.latest_pointer = Some(position);
        self.current_pointer = Some(position);
        self.held_movement = false;
        self.velocity = Vec2::ZERO;
        self.last_frame_time = time.filter(|time| time.is_finite());
    }

    fn advance_frame(&mut self, time: Option<f64>) -> Vec2 {
        let time = time.filter(|time| time.is_finite());
        let delta_seconds = match (self.last_frame_time, time) {
            (Some(previous), Some(current)) => {
                let delta = current - previous;
                if delta.is_finite() && delta > 0.0 && delta <= MAX_FRAME_DELTA {
                    delta as f32
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        self.last_frame_time = time;
        if !self.is_active() || self.current_pointer.is_none() {
            self.velocity = Vec2::ZERO;
            return Vec2::ZERO;
        }
        self.velocity = self
            .anchor
            .zip(self.latest_pointer)
            .and_then(|(anchor, pointer)| distance_velocity(anchor, pointer))
            .unwrap_or(Vec2::ZERO);
        let delta = self.velocity * delta_seconds;
        if finite_vec(delta) {
            delta
        } else {
            self.velocity = Vec2::ZERO;
            Vec2::ZERO
        }
    }
}

/// Paint the shared circle-and-four-arrows anchor mark inside a caller clip.
pub(crate) fn paint_anchor(
    painter: &egui::Painter,
    clip: egui::Rect,
    anchor: Option<Pos2>,
    color: Color32,
) {
    let Some(anchor) = anchor.filter(|point| finite_pos(*point) && clip.contains(*point)) else {
        return;
    };
    let painter = painter.with_clip_rect(clip);
    let stroke = Stroke::new(1.25, color);
    painter.circle_stroke(anchor, 5.0, stroke);
    let directions = [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y];
    for direction in directions {
        let start = anchor + direction * 7.0;
        let tip = anchor + direction * 12.0;
        let side = Vec2::new(-direction.y, direction.x) * 2.5;
        painter.line_segment([start, tip], stroke);
        painter.line_segment([tip, tip - direction * 3.5 + side], stroke);
        painter.line_segment([tip, tip - direction * 3.5 - side], stroke);
    }
}

pub(crate) fn set_active_cursor(ctx: &egui::Context, active: bool) {
    if active {
        ctx.set_cursor_icon(egui::CursorIcon::AllScroll);
    }
}

fn distance_velocity(anchor: Pos2, pointer: Pos2) -> Option<Vec2> {
    let dx = pointer.x - anchor.x;
    let dy = pointer.y - anchor.y;
    let distance = dx.hypot(dy);
    if !dx.is_finite() || !dy.is_finite() || !distance.is_finite() {
        return None;
    }
    if distance <= RADIAL_DEAD_ZONE {
        return Some(Vec2::ZERO);
    }
    let beyond = (distance - RADIAL_DEAD_ZONE) / distance;
    let scale = beyond * LINEAR_GAIN_POINTS_PER_SECOND;
    let velocity = Vec2::new(dx * scale, dy * scale);
    if !finite_vec(velocity) {
        return None;
    }
    let velocity = velocity.clamp(Vec2::splat(-MAX_AXIS_SPEED), Vec2::splat(MAX_AXIS_SPEED));
    finite_vec(velocity).then_some(velocity)
}

fn radial_distance(anchor: Pos2, pointer: Pos2) -> Option<f32> {
    let distance = (pointer.x - anchor.x).hypot(pointer.y - anchor.y);
    distance.is_finite().then_some(distance)
}

fn finite_option(position: Pos2) -> Option<Pos2> {
    finite_pos(position).then_some(position)
}

fn finite_pos(position: Pos2) -> bool {
    position.x.is_finite() && position.y.is_finite()
}

fn finite_vec(vector: Vec2) -> bool {
    vector.x.is_finite() && vector.y.is_finite()
}
