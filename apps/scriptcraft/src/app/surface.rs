//! Surface-local app state operations and zoom coordination.

use super::{diagnostics::add_performance_counter, RootDocumentView, ScriptCraftApp};
use crate::chrome::status_bar::ZoomPolicy;
use crate::views::script::RevealIntent;
use eframe::egui;
use scriptcraft_commands::EditorSurface;
use scriptcraft_model::StableId;

impl ScriptCraftApp {
    pub(crate) fn beat_zoom_target(&self) -> bool {
        !self.title_view_state_swapped
            && self.active_editor_surface != EditorSurface::TitlePage
            && self.root_document_view == RootDocumentView::BeatBoard
    }

    pub(crate) fn active_zoom_policy(&self) -> ZoomPolicy {
        if self.beat_zoom_target() { ZoomPolicy::BEAT } else { ZoomPolicy::PAGE }
    }

    pub(crate) fn active_zoom(&self) -> u16 {
        if self.beat_zoom_target() {
            self.active_zoom_policy().normalize(f64::from(self.beat_board_view_state.zoom_milli_percent()) / 1000.0)
        } else {
            self.zoom_for_surface(if self.title_view_state_swapped { EditorSurface::TitlePage } else { self.active_editor_surface() })
        }
    }

    pub(crate) fn set_beat_zoom(&mut self, zoom: f64, pointer_anchor: bool, ctx: &egui::Context) {
        if !zoom.is_finite() || zoom <= 0.0 { return; }
        self.beat_auto_scroll.cancel();
        let milli = (zoom * 1000.0).round().clamp(5000.0, 300000.0) as u32;
        self.beat_pinch_zoom_percent = zoom;
        if milli != self.beat_board_view_state.zoom_milli_percent() {
            // Apply the anchor immediately, so a later pan or save cannot see
            // an unapplied zoom anchor or an obsolete scroll origin.
            let origin = if let Some(geometry) = self.beat_viewport {
                let screen = if pointer_anchor {
                    geometry.pointer_screen.filter(|_| geometry.pointer_board.is_some()).unwrap_or_else(|| geometry.viewport.center())
                } else {
                    geometry.viewport.center()
                };
                geometry.origin_for_anchor(geometry.screen_to_board(screen), screen, milli)
            } else {
                self.beat_board_view_state.scroll_origin()
            };
            self.beat_board_view_state = scriptcraft_fdx::BeatBoardViewState::new(
                origin, milli,
            ).expect("bounded Beat zoom and existing origin");
            self.beat_viewport = self.beat_viewport.map(|geometry| geometry.with_view(self.beat_board_view_state));
            ctx.request_repaint();
        }
    }

    pub(crate) fn set_root_document_view(&mut self, view: RootDocumentView, ctx: &egui::Context) {
        if self.root_document_view == view { return; }
        self.reset_body_auto_scroll();
        self.reset_title_auto_scroll();
        crate::presentation::auto_scroll::set_route_owner(ctx, None);
        self.root_document_view = view;
        self.beat_viewport = None;
        self.beat_auto_scroll.reset();
        self.beat_interaction.cancel();
        self.beat_text.blur();
        if let Some(id) = self.editor_surface_id {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
        if let Some(id) = self.beat_text.viewport_id {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
        if self.title_view_state_swapped {
            self.title_view.surface_focused = false;
            self.title_view.drag_anchor = None;
            self.title_view.ime_preedit.clear();
            self.title_view.ime_active = false;
            self.title_view.reveal_intent = RevealIntent::None;
            self.title_view.preferred_vertical_x = None;
            self.title_view.pending_zoom_anchor = None;
        } else {
            self.surface_focused = false;
            self.drag_anchor = None;
            self.ime_preedit.clear();
            self.ime_active = false;
            self.reveal_intent = RevealIntent::None;
            self.preferred_vertical_x = None;
            self.pending_zoom_anchor = None;
            self.editor_surface_rect = None;
            self.set_active_body_group(None);
        }
        self.wheel_zoom.reset();
        ctx.request_repaint();
    }

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
            self.reset_body_auto_scroll();
            self.reset_title_auto_scroll();
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

    pub(crate) fn invisibles_visible_for(&self, surface: EditorSurface) -> bool {
        match surface {
            EditorSurface::Body => self.invisibles.show_body,
            EditorSurface::TitlePage => self.invisibles.show_title_page,
        }
    }

    pub(crate) fn active_surface_invisibles_visible(&self) -> bool {
        self.invisibles_visible_for(self.active_editor_surface)
    }

    pub(crate) fn invisibles_action_label(&self) -> &'static str {
        if self.active_surface_invisibles_visible() {
            "Hide Invisibles"
        } else {
            "Show Invisibles"
        }
    }

    pub(crate) fn ruler_visible(&self) -> bool {
        self.settings_store.ruler_visible()
    }

    pub(crate) fn ruler_action_label(&self) -> &'static str {
        if self.ruler_visible() {
            "Hide Ruler"
        } else {
            "Show Ruler"
        }
    }

    pub(crate) fn outline_editor_action_label(show_story_map: bool) -> &'static str {
        if show_story_map {
            "Hide Outline Editor"
        } else {
            "Show Outline Editor"
        }
    }


    pub(crate) fn invisibles_color(&self) -> egui::Color32 {
        let color = self.invisibles.color;
        egui::Color32::from_rgb(color.red, color.green, color.blue)
    }

    pub(crate) fn toggle_active_surface_invisibles(&mut self, ctx: &egui::Context) {
        match self.active_editor_surface {
            EditorSurface::Body => self.invisibles.show_body = !self.invisibles.show_body,
            EditorSurface::TitlePage => {
                self.invisibles.show_title_page = !self.invisibles.show_title_page
            }
        }
        ctx.request_repaint();
        if self.title_page_window_open {
            ctx.request_repaint_of(crate::views::title_page::viewport_id());
        }
    }

    pub(crate) fn swap_title_view_state(&mut self) {
        self.reset_body_auto_scroll();
        std::mem::swap(
            &mut self.surface_focused,
            &mut self.title_view.surface_focused,
        );
        std::mem::swap(&mut self.drag_anchor, &mut self.title_view.drag_anchor);
        std::mem::swap(&mut self.ime_preedit, &mut self.title_view.ime_preedit);
        std::mem::swap(&mut self.ime_active, &mut self.title_view.ime_active);
        std::mem::swap(
            &mut self.rendered_revision,
            &mut self.title_view.rendered_revision,
        );
        std::mem::swap(&mut self.reveal_intent, &mut self.title_view.reveal_intent);
        std::mem::swap(
            &mut self.preferred_vertical_x,
            &mut self.title_view.preferred_vertical_x,
        );
        std::mem::swap(&mut self.page_geometry, &mut self.title_view.page_geometry);
        std::mem::swap(
            &mut self.pending_zoom_anchor,
            &mut self.title_view.pending_zoom_anchor,
        );
        std::mem::swap(&mut self.page_scale, &mut self.title_view.page_scale);
        std::mem::swap(
            &mut self.effective_pixels_per_point,
            &mut self.title_view.effective_pixels_per_point,
        );
        std::mem::swap(&mut self.zoom_percent, &mut self.title_view.zoom_percent);
        std::mem::swap(
            &mut self.pinch_zoom_percent,
            &mut self.title_view.pinch_zoom_percent,
        );
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

    pub(crate) fn zoom_for_surface(&self, surface: EditorSurface) -> u16 {
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

    pub(crate) fn set_zoom_for_surface(
        &mut self,
        surface: EditorSurface,
        zoom: u16,
        ctx: &egui::Context,
    ) {
        if self.zoom_for_surface(surface) == zoom {
            return;
        }
        if surface == EditorSurface::Body && !self.title_view_state_swapped {
            self.cancel_body_auto_scroll();
        } else if surface == EditorSurface::TitlePage {
            self.cancel_title_auto_scroll();
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
                self.cancel_title_auto_scroll();
                if self.title_view.pending_zoom_anchor.is_none() {
                    self.title_view.pending_zoom_anchor =
                        self.title_view.page_geometry.capture_center_anchor(
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
            EditorSurface::TitlePage if self.title_view_state_swapped => self.preferred_vertical_x,
            EditorSurface::TitlePage => self.title_view.preferred_vertical_x,
        }
    }

    pub(crate) fn set_preferred_vertical_x_for(
        &mut self,
        surface: EditorSurface,
        value: Option<i64>,
    ) {
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
        if self.title_view_state_swapped {
            self.cancel_title_auto_scroll();
        } else {
            self.cancel_body_auto_scroll();
        }
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
