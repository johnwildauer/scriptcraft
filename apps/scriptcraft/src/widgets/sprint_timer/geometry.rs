use eframe::egui;

// Popup-local logical-point presentation tokens. Keep these independent of
// global egui spacing and use the planner below for every popup presentation.
pub(super) const POPUP_BODY_WIDTH: f32 = 300.0;
pub(super) const POPUP_BODY_HEIGHT: f32 = 165.0;
pub(super) const POPUP_BODY_INSET: f32 = 16.0;
pub(super) const POPUP_CORNER_RADIUS: f32 = 8.0;
pub(super) const POPUP_TOGGLE_TOP: f32 = 18.0;
pub(super) const POPUP_TOGGLE_HEIGHT: f32 = 22.0;
pub(super) const POPUP_DISPLAY_TOP: f32 = 54.0;
pub(super) const POPUP_DISPLAY_BOTTOM: f32 = 116.0;
pub(super) const POPUP_FEEDBACK_TOP: f32 = 116.0;
pub(super) const POPUP_FEEDBACK_HEIGHT: f32 = 12.0;
pub(super) const POPUP_ACTION_TOP: f32 = 132.0;
pub(super) const POPUP_ACTION_HEIGHT: f32 = 20.0;
pub(super) const POPUP_ACTION_WIDTH: f32 = 98.0;
pub(super) const POPUP_ACTION_GAP: f32 = 12.0;
pub(super) const POPUP_NOTCH_HEIGHT: f32 = 20.0;
pub(super) const POPUP_NOTCH_WIDTH: f32 = 28.0;
pub(super) const POPUP_ICON_CLEARANCE: f32 = 2.0;
pub(super) const POPUP_MODE_LABEL_WIDTH: f32 = 80.0;
pub(super) const POPUP_MODE_LABEL_HEIGHT: f32 = 22.0;
pub(super) const POPUP_MODE_FOCUS_INSET: f32 = 1.0;
pub(super) const POPUP_MODE_SWITCH_TRACK_WIDTH: f32 = 38.0;
pub(super) const POPUP_MODE_SWITCH_TRACK_HEIGHT: f32 = 22.0;
pub(super) const POPUP_MODE_SWITCH_KNOB_DIAMETER: f32 = 18.0;
pub(super) const POPUP_MODE_SWITCH_KNOB_END_INSET: f32 = 11.0;
pub(super) const POPUP_COUNTDOWN_EDITOR_WIDTH: f32 = 128.0;
pub(super) const POPUP_COUNTDOWN_EDITOR_HEIGHT: f32 = 38.0;
pub(super) const POPUP_ACTIVE_DISPLAY_WIDTH: f32 = 175.0;
pub(super) const POPUP_ACTIVE_DISPLAY_HEIGHT: f32 = 38.0;
pub(super) const MODE_SELECTOR_GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SprintTimerPopupLayout {
    pub(super) body: egui::Rect,
    pub(super) toggle: egui::Rect,
    pub(super) display: egui::Rect,
    pub(super) feedback: egui::Rect,
    pub(super) primary_action: egui::Rect,
    pub(super) end_action: egui::Rect,
}

/// Returns the fixed, popup-local layout in logical points. Placement relative
/// to a viewport or icon belongs to the outer presentation boundary.
pub(super) fn sprint_timer_popup_layout() -> SprintTimerPopupLayout {
    let body = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(POPUP_BODY_WIDTH, POPUP_BODY_HEIGHT),
    );
    let content_width = POPUP_BODY_WIDTH - 2.0 * POPUP_BODY_INSET;
    let content_left = POPUP_BODY_INSET;
    let centered_rect = |top: f32, width: f32, height: f32| {
        egui::Rect::from_min_size(
            egui::pos2((POPUP_BODY_WIDTH - width) / 2.0, top),
            egui::vec2(width, height),
        )
    };
    let pair_width = 2.0 * POPUP_ACTION_WIDTH + POPUP_ACTION_GAP;
    let pair_left = (POPUP_BODY_WIDTH - pair_width) / 2.0;

    SprintTimerPopupLayout {
        body,
        toggle: egui::Rect::from_min_size(
            egui::pos2(content_left, POPUP_TOGGLE_TOP),
            egui::vec2(content_width, POPUP_TOGGLE_HEIGHT),
        ),
        display: centered_rect(
            POPUP_DISPLAY_TOP,
            content_width,
            POPUP_DISPLAY_BOTTOM - POPUP_DISPLAY_TOP,
        ),
        feedback: egui::Rect::from_min_size(
            egui::pos2(content_left, POPUP_FEEDBACK_TOP),
            egui::vec2(content_width, POPUP_FEEDBACK_HEIGHT),
        ),
        primary_action: egui::Rect::from_min_size(
            egui::pos2(pair_left, POPUP_ACTION_TOP),
            egui::vec2(POPUP_ACTION_WIDTH, POPUP_ACTION_HEIGHT),
        ),
        end_action: egui::Rect::from_min_size(
            egui::pos2(pair_left + POPUP_ACTION_WIDTH + POPUP_ACTION_GAP, POPUP_ACTION_TOP),
            egui::vec2(POPUP_ACTION_WIDTH, POPUP_ACTION_HEIGHT),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SprintTimerPopupPlacement {
    pub(super) body: egui::Rect,
    pub(super) notch: egui::Rect,
    pub(super) popup: egui::Rect,
    pub(super) notch_tip: egui::Pos2,
    pub(super) notch_visible: bool,
}

/// Plans the fixed-size popup in the coordinate space of its own viewport.
/// When there is room, the notch stops just short of the icon response and the
/// body remains directly above it. Constrained and missing-anchor cases stay
/// origin-bounded and use no misleading pointer.
pub(super) fn sprint_timer_popup_placement(
    anchor: Option<egui::Rect>,
    viewport: egui::Rect,
) -> SprintTimerPopupPlacement {
    let viewport = if viewport.is_positive()
        && viewport.min.x.is_finite()
        && viewport.min.y.is_finite()
        && viewport.max.x.is_finite()
        && viewport.max.y.is_finite()
    {
        viewport
    } else {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0))
    };
    let width = POPUP_BODY_WIDTH;
    let height = POPUP_BODY_HEIGHT;
    let has_anchor = anchor.is_some_and(|rect| {
        rect.is_positive()
            && rect.min.x.is_finite()
            && rect.min.y.is_finite()
            && rect.max.x.is_finite()
            && rect.max.y.is_finite()
    });
    let anchor = anchor.filter(|_| has_anchor);

    let desired_center_x = anchor.map_or(viewport.center().x, |rect| rect.center().x);
    let max_left = (viewport.right() - width).max(viewport.left());
    let left = (desired_center_x - width / 2.0).clamp(viewport.left(), max_left);
    let body_top_limit = (viewport.bottom() - height).max(viewport.top());
    let anchor_tip_y = anchor.map(|rect| rect.top() - POPUP_ICON_CLEARANCE);
    let above_body_top = anchor_tip_y.map(|tip_y| tip_y - height);
    let above_notch_top = anchor_tip_y.map(|tip_y| tip_y - POPUP_NOTCH_HEIGHT - height);
    let above_body_fits = above_body_top.is_some_and(|top| {
        top >= viewport.top()
            && anchor_tip_y.is_some_and(|tip_y| tip_y <= viewport.bottom())
            && viewport.height() >= height
    });
    let notch_visible = above_body_fits
        && above_notch_top.is_some_and(|top| top >= viewport.top())
        && viewport.width() >= width
        && viewport.height() >= height + POPUP_NOTCH_HEIGHT + POPUP_ICON_CLEARANCE;
    let top = if notch_visible {
        above_notch_top.unwrap_or(viewport.top())
    } else if above_body_fits {
        above_body_top.unwrap_or(viewport.top())
    } else if !has_anchor && viewport.height() >= height {
        (viewport.center().y - height / 2.0).clamp(viewport.top(), body_top_limit)
    } else if let Some(anchor) = anchor.filter(|_| viewport.height() >= height) {
        (anchor.bottom() + POPUP_ICON_CLEARANCE).clamp(viewport.top(), body_top_limit)
    } else {
        // In short viewports keep the fixed body anchored at the visible content
        // origin rather than placing its origin offscreen or chasing the icon.
        viewport.top().clamp(viewport.top(), body_top_limit)
    };
    let body = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height));

    if !notch_visible {
        return SprintTimerPopupPlacement {
            body,
            notch: egui::Rect::NOTHING,
            popup: body,
            notch_tip: egui::Pos2::ZERO,
            notch_visible: false,
        };
    }

    let icon_top = anchor.map_or(viewport.top(), |rect| rect.top());
    let tip_y = icon_top - POPUP_ICON_CLEARANCE;
    let safe_half_width = POPUP_NOTCH_WIDTH / 2.0;
    let safe_left = body.left() + POPUP_CORNER_RADIUS + safe_half_width;
    let safe_right = body.right() - POPUP_CORNER_RADIUS - safe_half_width;
    let target_x = anchor.map_or(body.center().x, |rect| rect.center().x);
    let tip_x = target_x.clamp(safe_left, safe_right);
    let notch = egui::Rect::from_min_max(
        egui::pos2(tip_x - safe_half_width, body.bottom()),
        egui::pos2(tip_x + safe_half_width, tip_y),
    );
    let popup = body.union(notch);

    SprintTimerPopupPlacement {
        body,
        notch,
        popup,
        notch_tip: egui::pos2(tip_x, tip_y),
        notch_visible: true,
    }
}

/// Transient host-window geometry observed through the owning egui viewport.
/// Each component stays optional so unavailable observations never imply a
/// fabricated move or resize.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct SprintTimerHostGeometry {
    outer_left: Option<f32>,
    outer_top: Option<f32>,
    inner_width: Option<f32>,
    inner_height: Option<f32>,
}

impl SprintTimerHostGeometry {
    pub(crate) fn from_context(ctx: &egui::Context) -> Self {
        ctx.input(|input| {
            let viewport = input.viewport();
            Self::from_rects(viewport.inner_rect, viewport.outer_rect)
        })
    }

    fn from_rects(inner: Option<egui::Rect>, outer: Option<egui::Rect>) -> Self {
        let finite = |value: f32| value.is_finite().then_some(value);
        Self {
            outer_left: outer.and_then(|rect| finite(rect.left())),
            outer_top: outer.and_then(|rect| finite(rect.top())),
            inner_width: inner.and_then(|rect| finite(rect.width())),
            inner_height: inner.and_then(|rect| finite(rect.height())),
        }
    }

    pub(super) fn changed_from(self, captured: Self) -> bool {
        fn changed(captured: Option<f32>, current: Option<f32>) -> bool {
            captured
                .zip(current)
                .is_some_and(|(captured, current)| captured != current)
        }

        changed(captured.outer_left, self.outer_left)
            || changed(captured.outer_top, self.outer_top)
            || changed(captured.inner_width, self.inner_width)
            || changed(captured.inner_height, self.inner_height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ModeSelectorGeometry {
    pub(super) toggle: egui::Rect,
    pub(super) countdown: egui::Rect,
    pub(super) switch: egui::Rect,
    pub(super) stopwatch: egui::Rect,
}

impl Default for ModeSelectorGeometry {
    fn default() -> Self {
        Self {
            toggle: egui::Rect::NOTHING,
            countdown: egui::Rect::NOTHING,
            switch: egui::Rect::NOTHING,
            stopwatch: egui::Rect::NOTHING,
        }
    }
}

pub(super) fn mode_selector_geometry(toggle: egui::Rect) -> ModeSelectorGeometry {
    let group_width = POPUP_MODE_LABEL_WIDTH * 2.0
        + POPUP_MODE_SWITCH_TRACK_WIDTH
        + MODE_SELECTOR_GAP * 2.0;
    let group_left = toggle.center().x - group_width / 2.0;
    let label_top = toggle.center().y - POPUP_MODE_LABEL_HEIGHT / 2.0;
    let switch_left = group_left + POPUP_MODE_LABEL_WIDTH + MODE_SELECTOR_GAP;

    ModeSelectorGeometry {
        toggle,
        countdown: egui::Rect::from_min_size(
            egui::pos2(group_left, label_top),
            egui::vec2(POPUP_MODE_LABEL_WIDTH, POPUP_MODE_LABEL_HEIGHT),
        ),
        switch: egui::Rect::from_min_size(
            egui::pos2(switch_left, label_top),
            egui::vec2(POPUP_MODE_SWITCH_TRACK_WIDTH, POPUP_MODE_SWITCH_TRACK_HEIGHT),
        ),
        stopwatch: egui::Rect::from_min_size(
            egui::pos2(
                switch_left + POPUP_MODE_SWITCH_TRACK_WIDTH + MODE_SELECTOR_GAP,
                label_top,
            ),
            egui::vec2(POPUP_MODE_LABEL_WIDTH, POPUP_MODE_LABEL_HEIGHT),
        ),
    }
}

pub(super) fn mode_switch_track_rect(response_rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(
        response_rect.center(),
        egui::vec2(
            POPUP_MODE_SWITCH_TRACK_WIDTH,
            POPUP_MODE_SWITCH_TRACK_HEIGHT,
        ),
    )
}

pub(super) fn mode_switch_knob_center(track: egui::Rect, checked: bool) -> egui::Pos2 {
    let x = if checked {
        track.right() - POPUP_MODE_SWITCH_KNOB_END_INSET
    } else {
        track.left() + POPUP_MODE_SWITCH_KNOB_END_INSET
    };
    egui::pos2(x, track.center().y)
}

pub(super) fn mode_focus_rect(rect: egui::Rect) -> egui::Rect {
    rect.shrink(POPUP_MODE_FOCUS_INSET)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PopupGeometryCheckpoint {
    pub(super) content_min: egui::Rect,
    pub(super) parent_cursor: egui::Rect,
    pub(super) parent_available: egui::Rect,
}

impl Default for PopupGeometryCheckpoint {
    fn default() -> Self {
        Self {
            content_min: egui::Rect::NOTHING,
            parent_cursor: egui::Rect::NOTHING,
            parent_available: egui::Rect::NOTHING,
        }
    }
}

impl PopupGeometryCheckpoint {
    pub(super) fn capture(ui: &egui::Ui) -> Self {
        Self {
            content_min: ui.min_rect(),
            parent_cursor: ui.cursor(),
            parent_available: ui.available_rect_before_wrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PopupGeometryLedger {
    pub(super) planned_body: egui::Rect,
    pub(super) after_display: PopupGeometryCheckpoint,
    pub(super) editor_parent_before: PopupGeometryCheckpoint,
    pub(super) editor_parent_after: PopupGeometryCheckpoint,
    pub(super) editor_assigned: Option<egui::Rect>,
    pub(super) editor_child_min: Option<egui::Rect>,
    pub(super) editor_response: Option<egui::Rect>,
    pub(super) primary_parent_before: PopupGeometryCheckpoint,
    pub(super) primary_parent_after: PopupGeometryCheckpoint,
    pub(super) primary_assigned: egui::Rect,
    pub(super) primary_child_min: egui::Rect,
    pub(super) primary_response: egui::Rect,
    pub(super) end_parent_before: PopupGeometryCheckpoint,
    pub(super) end_parent_after: PopupGeometryCheckpoint,
    pub(super) end_assigned: egui::Rect,
    pub(super) end_child_min: egui::Rect,
    pub(super) end_response: egui::Rect,
    pub(super) after_actions: PopupGeometryCheckpoint,
    pub(super) frame_outer: egui::Rect,
    pub(super) area_response: egui::Rect,
    pub(super) next_pass_retained_size: Option<egui::Vec2>,
}

impl Default for PopupGeometryLedger {
    fn default() -> Self {
        Self {
            planned_body: egui::Rect::NOTHING,
            after_display: PopupGeometryCheckpoint::default(),
            editor_parent_before: PopupGeometryCheckpoint::default(),
            editor_parent_after: PopupGeometryCheckpoint::default(),
            editor_assigned: None,
            editor_child_min: None,
            editor_response: None,
            primary_parent_before: PopupGeometryCheckpoint::default(),
            primary_parent_after: PopupGeometryCheckpoint::default(),
            primary_assigned: egui::Rect::NOTHING,
            primary_child_min: egui::Rect::NOTHING,
            primary_response: egui::Rect::NOTHING,
            end_parent_before: PopupGeometryCheckpoint::default(),
            end_parent_after: PopupGeometryCheckpoint::default(),
            end_assigned: egui::Rect::NOTHING,
            end_child_min: egui::Rect::NOTHING,
            end_response: egui::Rect::NOTHING,
            after_actions: PopupGeometryCheckpoint::default(),
            frame_outer: egui::Rect::NOTHING,
            area_response: egui::Rect::NOTHING,
            next_pass_retained_size: None,
        }
    }
}

pub(super) fn retained_popup_area_size(ctx: &egui::Context) -> Option<egui::Vec2> {
    ctx.memory(|memory| {
        memory
            .area_rect(egui::Id::new("sprint-timer-popup"))
            .map(|rect| rect.size())
    })
}

pub(super) fn centered_active_display_panel_rect(stage: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(
        stage.center(),
        egui::vec2(POPUP_ACTIVE_DISPLAY_WIDTH, POPUP_ACTIVE_DISPLAY_HEIGHT),
    )
}

