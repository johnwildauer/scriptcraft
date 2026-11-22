//! Native/title window lifecycle and transient window-state projection.

use super::{ScriptCraftApp, TitlePageViewState};
use crate::{
    chrome::theme,
    commands::CommandInvocation,
    documents::replacement::replacement_resume_focus_allowed,
    views::{script::zoom_scroll, title_page},
};
use eframe::egui::{self, Vec2};
use scriptcraft_commands::{Command, CommandOrigin, EditorSurface, ZoomFactor};
use scriptcraft_fdx as fdx;

impl ScriptCraftApp {
    /// Immediate child viewports may enter their callback without the app-level
    /// raw-input hook. Reuse the same title-viewport adapter and pinch route as
    /// the hook; removing from both event collections keeps editor scrolling and
    /// manual-scroll anchor cancellation from seeing a zoom gesture.
    pub(crate) fn adapt_title_page_wheel_input(&mut self, ctx: &egui::Context) {
        let viewport_height_points = ctx.content_rect().height();
        let factors = ctx.input_mut(|input| {
            let previous_event_count = input.events.len();
            let factors = self
                .title_view
                .wheel_zoom
                .adapt_events(&mut input.events, viewport_height_points);
            let consumed_event = input.events.len() < previous_event_count;
            input.raw.events.clone_from(&input.events);
            if consumed_event && !zoom_scroll::has_nonzero_mouse_wheel(&input.raw.events) {
                // At this fallback point egui has already accumulated wheel
                // deltas. Clear those only when no ordinary wheel event remains.
                input.smooth_scroll_delta = Vec2::ZERO;
            }
            factors
        });
        self.dispatch_wheel_zoom_factors(EditorSurface::TitlePage, ctx, factors);
    }

    pub(crate) fn open_title_page_window(&mut self, ctx: &egui::Context) {
        if self.title_viewport_unsupported {
            return;
        }
        if self.title_view_reset_pending {
            self.title_view = TitlePageViewState::default();
            self.title_view_reset_pending = false;
        }
        self.title_page_focus_requested = true;
        self.set_title_page_window_open(true);
        ctx.request_repaint();
    }

    pub(crate) fn close_title_page_window(&mut self, ctx: &egui::Context) {
        if self.title_page_window_open {
            title_page::send_close(ctx);
        }
        self.timer_title_host_geometry = None;
        if self.timer_popup_surface == EditorSurface::TitlePage {
            self.finalize_sprint_timer_popup(ctx);
            self.timer_popup_surface = EditorSurface::Body;
        }
        self.set_title_page_window_open(false);
        self.title_page_focus_requested = false;
        self.title_view_reset_pending = true;
        self.set_active_editor_surface(EditorSurface::Body);
        let availability = self.app_command_availability();
        if !self.replacement_open_pending()
            && !self.element_windows.owns_input()
            && replacement_resume_focus_allowed(
                availability.modal_blocked,
                availability.native_output_active,
            )
        {
            self.restore_editor_focus(ctx);
        }
        ctx.request_repaint();
    }

    pub(crate) fn capture_sprint_timer_host_geometry(
        &mut self,
        surface: EditorSurface,
        ctx: &egui::Context,
    ) {
        let geometry =
            crate::widgets::sprint_timer::geometry::SprintTimerHostGeometry::from_context(ctx);
        match surface {
            EditorSurface::Body => {
                self.timer_body_host_geometry = Some(geometry);
                self.timer_title_host_geometry = None;
            }
            EditorSurface::TitlePage => {
                self.timer_title_host_geometry = Some(geometry);
                self.timer_body_host_geometry = None;
            }
        }
    }

    pub(crate) fn sprint_timer_host_geometry(
        &self,
        surface: EditorSurface,
    ) -> Option<crate::widgets::sprint_timer::geometry::SprintTimerHostGeometry> {
        match surface {
            EditorSurface::Body => self.timer_body_host_geometry,
            EditorSurface::TitlePage => self.timer_title_host_geometry,
        }
    }

    pub(crate) fn clear_sprint_timer_popup_geometry(&mut self) {
        self.timer_body_host_geometry = None;
        self.timer_title_host_geometry = None;
        self.sprint_timer.popup_anchor = egui::Rect::NOTHING;
        self.sprint_timer.popup_rect = egui::Rect::NOTHING;
    }

    pub(super) fn update_native_window_state(&mut self, ctx: &egui::Context) {
        let reported = ctx.input(|input| {
            let viewport = input.viewport();
            window_state_from_rects(viewport.inner_rect, viewport.outer_rect)
        });
        if let Some(reported) = reported {
            self.latest_window_state = Some(reported);
        }
        if let Some(commands) = take_window_restore_commands(&mut self.pending_window_restore) {
            self.restore_imported_window(commands, ctx);
        }
    }
}

impl ScriptCraftApp {
    pub(crate) fn set_title_page_window_open(&mut self, open: bool) {
        self.title_page_window_open = open;
        if !open {
            self.title_view.wheel_zoom.reset();
            self.appearance_applications
                .clear(theme::AppearanceContext::TitlePage);
        }
        if !open && self.active_editor_surface == EditorSurface::TitlePage {
            self.set_active_editor_surface(EditorSurface::Body);
        }
    }

    pub(crate) fn dispatch_wheel_zoom_factors(
        &mut self,
        surface: EditorSurface,
        ctx: &egui::Context,
        factors: Vec<f32>,
    ) {
        if !factors.is_empty() {
            self.set_active_editor_surface(surface);
        }
        for factor in factors {
            if let Ok(factor) = ZoomFactor::new(factor) {
                self.dispatch_command(
                    CommandInvocation::new(
                        Command::ViewZoomPinch { factor },
                        CommandOrigin::Pointer,
                    ),
                    ctx,
                );
            }
        }
    }
}

fn window_state_from_rects(
    inner: Option<egui::Rect>,
    outer: Option<egui::Rect>,
) -> Option<fdx::WindowState> {
    fn bounded_integer(value: f32) -> Option<i32> {
        let value = f64::from(value);
        (value.is_finite() && value >= f64::from(i32::MIN) && value <= f64::from(i32::MAX))
            .then(|| value.round() as i32)
    }

    let inner = inner?;
    let outer = outer?;
    fdx::WindowState::new(
        bounded_integer(outer.min.x)?,
        bounded_integer(outer.min.y)?,
        bounded_integer(inner.width())?,
        bounded_integer(inner.height())?,
    )
}

fn window_restore_commands(state: fdx::WindowState) -> [egui::ViewportCommand; 2] {
    [
        egui::ViewportCommand::OuterPosition(egui::pos2(state.left as f32, state.top as f32)),
        egui::ViewportCommand::InnerSize(egui::vec2(state.width as f32, state.height as f32)),
    ]
}

fn take_window_restore_commands(
    pending: &mut Option<fdx::WindowState>,
) -> Option<[egui::ViewportCommand; 2]> {
    pending.take().map(window_restore_commands)
}
