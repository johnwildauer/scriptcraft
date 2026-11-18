//! Surface-local app state operations and zoom coordination.

use super::{diagnostics::add_performance_counter, ScriptCraftApp};
use crate::views::script::RevealIntent;
use eframe::egui;
use scriptcraft_commands::EditorSurface;
use scriptcraft_model::StableId;

impl ScriptCraftApp {
    pub(crate) fn set_active_body_group(&mut self, next: Option<StableId>) -> bool {
        if next == self.active_body_dual_dialogue_group {
            return false;
        }
        self.active_body_dual_dialogue_group = next;
        self.preferred_vertical_x = None;
        self.rendered_revision = None;
        self.page_geometry.invalidate();
        self.page_scale = None;
        self.last_layout_identity = None;
        true
    }

    pub(crate) fn reconcile_body_active_group(&mut self) -> bool {
        let next = if self.active_editor_surface == EditorSurface::Body
            && self.surface_focused
            && self.pending_fdx_open.is_none()
            && !self.replacement_open_pending()
        {
            scriptcraft_model::dual_dialogue::group_at_element_index(
                self.session.project(),
                self.session.selection().focus.element,
            )
            .map(|group| group.id.clone())
        } else {
            None
        };
        self.set_active_body_group(next)
    }

    pub(crate) fn set_active_editor_surface(&mut self, surface: EditorSurface) {
        if self.active_editor_surface != surface {
            if self.active_editor_surface == EditorSurface::TitlePage
                && !self.title_view_state_swapped
            {
                self.title_view.drag_anchor = None;
                self.title_view.preferred_vertical_x = None;
                self.title_view.reveal_intent = RevealIntent::None;
            }
            self.active_editor_surface = surface;
            self.drag_anchor = None;
            self.preferred_vertical_x = None;
            self.reveal_intent = RevealIntent::None;
            if surface == EditorSurface::Body {
                // Surface routing alone is not evidence that Body owns focus.
                self.set_active_body_group(None);
            } else {
                self.reconcile_body_active_group();
            }
        }
    }

    pub(crate) fn active_editor_surface(&self) -> EditorSurface {
        self.active_editor_surface
    }

    pub(crate) fn swap_title_view_state(&mut self) {
        std::mem::swap(&mut self.surface_focused, &mut self.title_view.surface_focused);
        std::mem::swap(&mut self.drag_anchor, &mut self.title_view.drag_anchor);
        std::mem::swap(&mut self.ime_preedit, &mut self.title_view.ime_preedit);
        std::mem::swap(&mut self.ime_active, &mut self.title_view.ime_active);
        std::mem::swap(&mut self.rendered_revision, &mut self.title_view.rendered_revision);
        std::mem::swap(&mut self.reveal_intent, &mut self.title_view.reveal_intent);
        std::mem::swap(&mut self.preferred_vertical_x, &mut self.title_view.preferred_vertical_x);
        std::mem::swap(&mut self.page_geometry, &mut self.title_view.page_geometry);
        std::mem::swap(&mut self.pending_zoom_anchor, &mut self.title_view.pending_zoom_anchor);
        std::mem::swap(&mut self.page_scale, &mut self.title_view.page_scale);
        std::mem::swap(
            &mut self.effective_pixels_per_point,
            &mut self.title_view.effective_pixels_per_point,
        );
        std::mem::swap(&mut self.zoom_percent, &mut self.title_view.zoom_percent);
        std::mem::swap(&mut self.pinch_zoom_percent, &mut self.title_view.pinch_zoom_percent);
        std::mem::swap(
            &mut self.caret_blink_focus_generation,
            &mut self.title_view.caret_blink_focus_generation,
        );
        std::mem::swap(
            &mut self.caret_blink_reset_time,
            &mut self.title_view.caret_blink_reset_time,
        );
        self.title_view_state_swapped = !self.title_view_state_swapped;
    }

    pub(crate) fn invalidate_surface(&mut self, surface: EditorSurface) {
        match surface {
            EditorSurface::Body => {
                self.rendered_revision = None;
                self.page_geometry.invalidate();
                self.page_scale = None;
            }
            EditorSurface::TitlePage => {
                if self.title_view_state_swapped {
                    self.rendered_revision = None;
                    self.page_geometry.invalidate();
                    self.page_scale = None;
                } else {
                    self.title_view.rendered_revision = None;
                    self.title_view.page_geometry.invalidate();
                    self.title_view.page_scale = None;
                }
            }
        }
    }

    fn zoom_for_surface(&self, surface: EditorSurface) -> u16 {
        match surface {
            EditorSurface::Body => self.zoom_percent,
            EditorSurface::TitlePage if self.title_view_state_swapped => self.zoom_percent,
            EditorSurface::TitlePage => self.title_view.zoom_percent,
        }
    }

    pub(crate) fn pinch_zoom_for_surface(&self, surface: EditorSurface) -> f64 {
        match surface {
            EditorSurface::Body => self.pinch_zoom_percent,
            EditorSurface::TitlePage if self.title_view_state_swapped => self.pinch_zoom_percent,
            EditorSurface::TitlePage => self.title_view.pinch_zoom_percent,
        }
    }

    pub(crate) fn set_zoom_for_surface(&mut self, surface: EditorSurface, zoom: u16, ctx: &egui::Context) {
        if self.zoom_for_surface(surface) == zoom {
            return;
        }
        self.capture_pending_zoom_anchor_for(surface, ctx);
        match surface {
            EditorSurface::Body => {
                self.zoom_percent = zoom;
                self.pinch_zoom_percent = f64::from(zoom);
            }
            EditorSurface::TitlePage if self.title_view_state_swapped => {
                self.zoom_percent = zoom;
                self.pinch_zoom_percent = f64::from(zoom);
            }
            EditorSurface::TitlePage => {
                self.title_view.zoom_percent = zoom;
                self.title_view.pinch_zoom_percent = f64::from(zoom);
            }
        }
        self.invalidate_surface(surface);
        ctx.request_repaint();
    }

    fn capture_pending_zoom_anchor_for(&mut self, surface: EditorSurface, ctx: &egui::Context) {
        match surface {
            EditorSurface::Body => self.capture_pending_zoom_anchor(ctx),
            EditorSurface::TitlePage if self.title_view_state_swapped => {
                self.capture_pending_zoom_anchor(ctx)
            }
            EditorSurface::TitlePage => {
                if self.title_view.pending_zoom_anchor.is_none() {
                    self.title_view.pending_zoom_anchor = self.title_view.page_geometry.capture_center_anchor(
                        self.performance.frames,
                        self.session.revision(),
                        ctx.content_rect(),
                        self.title_view.page_scale.unwrap_or(f32::NAN),
                    );
                }
            }
        }
    }

    pub(crate) fn set_pinch_zoom_for_surface(&mut self, surface: EditorSurface, value: f64) {
        match surface {
            EditorSurface::Body => self.pinch_zoom_percent = value,
            EditorSurface::TitlePage if self.title_view_state_swapped => {
                self.pinch_zoom_percent = value
            }
            EditorSurface::TitlePage => self.title_view.pinch_zoom_percent = value,
        }
    }

    pub(crate) fn preferred_vertical_x_for(&self, surface: EditorSurface) -> Option<i64> {
        match surface {
            EditorSurface::Body => self.preferred_vertical_x,
            EditorSurface::TitlePage if self.title_view_state_swapped => {
                self.preferred_vertical_x
            }
            EditorSurface::TitlePage => self.title_view.preferred_vertical_x,
        }
    }

    pub(crate) fn set_preferred_vertical_x_for(&mut self, surface: EditorSurface, value: Option<i64>) {
        match surface {
            EditorSurface::Body => self.preferred_vertical_x = value,
            EditorSurface::TitlePage if self.title_view_state_swapped => {
                self.preferred_vertical_x = value
            }
            EditorSurface::TitlePage => self.title_view.preferred_vertical_x = value,
        }
    }

    pub(crate) fn sync_pinch_zoom_from_controls(&mut self) {
        self.pinch_zoom_percent = f64::from(self.zoom_percent);
    }

    pub(crate) fn capture_pending_zoom_anchor(&mut self, ctx: &egui::Context) {
        if self.pending_zoom_anchor.is_none() {
            let anchor = self.page_geometry.capture_center_anchor(
                self.performance.frames,
                self.session.revision(),
                ctx.content_rect(),
                self.page_scale.unwrap_or(f32::NAN),
            );
            if anchor.is_some() {
                add_performance_counter(&mut self.performance.zoom_anchor_capture_successes, 1);
            } else {
                add_performance_counter(&mut self.performance.zoom_anchor_capture_misses, 1);
            }
            self.pending_zoom_anchor = anchor;
        }
    }
}
