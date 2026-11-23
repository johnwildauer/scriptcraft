use crate::{
    app::{add_performance_counter, ScriptCraftApp},
    chrome::{status_bar::PresentedPage, theme},
    presentation::geometry as presentation_geometry,
};
use eframe::egui::{self, Sense, Vec2};
use scriptcraft_commands::{EditorSurface, ScriptViewMode};
use std::time::Instant;

use super::page_policy::{
    presented_page_policy, resolve_presentation_mode, viewport_paint_page_range,
};
use super::reveal;
use super::ruler::{paint_ruler, RULER_ALLOCATED_HEIGHT};
use super::zoom_scroll::{
    page_scroll_geometry, prepaint_zoom_scroll_offset, presentation_metrics_at_zoom,
    SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING, SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING,
};

const EDITOR_SURFACE_ID: &str = "page-editor-surface";
const EDITOR_SCROLLBAR_TRACK_WIDTH: f32 = 14.0;
const EDITOR_SCROLLBAR_HANDLE_THICKNESS: f32 = 8.0;
const EDITOR_SCROLLBAR_HANDLE_MAX_CORNER_RADIUS: f32 = 4.0;

pub(crate) fn chrome_clear_color(
    mode: ScriptViewMode,
    palette: theme::ScriptWorkspacePalette,
) -> [f32; 4] {
    super::page_policy::script_view_backdrop(mode, palette).to_normalized_gamma_f32()
}

pub(crate) fn central_panel_frame(
    _ctx: &egui::Context,
    mode: ScriptViewMode,
    palette: theme::ScriptWorkspacePalette,
) -> egui::Frame {
    // Keep the resolved Script View surface color without a fixed viewport inset.
    egui::Frame::NONE.fill(super::page_policy::script_view_backdrop(mode, palette))
}

pub(crate) fn with_editor_surface_scrollbar_presentation<R>(
    ui: &mut egui::Ui,
    palette: theme::EditorScrollbarPalette,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let previous_scroll = ui.style().spacing.scroll;
    let previous_visuals = ui.visuals().clone();
    let mut editor_scroll = previous_scroll;
    editor_scroll.floating = true;
    editor_scroll.floating_width = EDITOR_SCROLLBAR_TRACK_WIDTH;
    editor_scroll.bar_width = EDITOR_SCROLLBAR_TRACK_WIDTH;
    editor_scroll.floating_allocated_width = 0.0;
    editor_scroll.foreground_color = true;
    editor_scroll.dormant_background_opacity = 1.0;
    editor_scroll.active_background_opacity = 1.0;
    editor_scroll.interact_background_opacity = 1.0;
    editor_scroll.dormant_handle_opacity = 0.0;
    editor_scroll.active_handle_opacity = 0.0;
    editor_scroll.interact_handle_opacity = 0.0;
    ui.style_mut().spacing.scroll = editor_scroll;

    let visuals = ui.visuals_mut();
    visuals.extreme_bg_color = palette.track;

    let result = add_contents(ui);
    ui.style_mut().spacing.scroll = previous_scroll;
    *ui.visuals_mut() = previous_visuals;
    result
}

fn paint_editor_scrollbar_handle<R>(
    ui: &egui::Ui,
    scroll_output: &egui::scroll_area::ScrollAreaOutput<R>,
    palette: theme::EditorScrollbarPalette,
) {
    let inner_rect = scroll_output.inner_rect;
    let content_size = scroll_output.content_size;
    let max_offset = content_size - inner_rect.size();
    let scroll_offset = scroll_output.state.offset;
    let scroll_style = ui.style().spacing.scroll;

    for axis in 0..2 {
        let needed = content_size[axis] > inner_rect.size()[axis];
        let show_factor = ui.ctx().animate_bool_responsive(
            scroll_output.id.with(if axis == 0 { "h" } else { "v" }),
            needed,
        );
        if !needed || show_factor == 0.0 {
            continue;
        }

        // With zero floating allocation, the stock outer rectangle and the
        // published inner rectangle share these bounds.
        let outer_margin = show_factor * scroll_style.bar_outer_margin;
        let mut max_cross = inner_rect.max[1 - axis] - outer_margin;
        if ui.clip_rect().max[1 - axis] - outer_margin < max_cross {
            max_cross = ui.clip_rect().max[1 - axis] - outer_margin;
        }

        let track_thickness = EDITOR_SCROLLBAR_TRACK_WIDTH * show_factor;
        let track_cross_min = max_cross - track_thickness;
        let track_rect = if axis == 0 {
            egui::Rect::from_min_max(
                egui::pos2(inner_rect.min.x, track_cross_min),
                egui::pos2(inner_rect.max.x, max_cross),
            )
        } else {
            egui::Rect::from_min_max(
                egui::pos2(track_cross_min, inner_rect.min.y),
                egui::pos2(max_cross, inner_rect.max.y),
            )
        };

        let handle_cross_thickness = EDITOR_SCROLLBAR_HANDLE_THICKNESS * show_factor;
        let handle_cross_min = track_cross_min + (track_thickness - handle_cross_thickness) * 0.5;
        let from_content = |content| {
            egui::emath::remap_clamp(
                content,
                0.0..=content_size[axis],
                inner_rect.min[axis]..=inner_rect.max[axis],
            )
        };
        let handle_length = (from_content(scroll_offset[axis] + inner_rect.size()[axis])
            - from_content(scroll_offset[axis]))
        .max(scroll_style.handle_min_length);
        let handle_start = egui::emath::remap_clamp(
            scroll_offset[axis],
            0.0..=max_offset[axis],
            inner_rect.min[axis]..=(inner_rect.max[axis] - handle_length),
        );
        let handle_rect = if axis == 0 {
            egui::Rect::from_min_max(
                egui::pos2(handle_start, handle_cross_min),
                egui::pos2(
                    handle_start + handle_length,
                    handle_cross_min + handle_cross_thickness,
                ),
            )
        } else {
            egui::Rect::from_min_max(
                egui::pos2(handle_cross_min, handle_start),
                egui::pos2(
                    handle_cross_min + handle_cross_thickness,
                    handle_start + handle_length,
                ),
            )
        };

        let response = ui.ctx().read_response(scroll_output.id.with(axis));
        let pointer_on_handle = response.as_ref().is_some_and(|response| response.hovered())
            && ui.input(|input| {
                input
                    .pointer
                    .latest_pos()
                    .is_some_and(|pointer_pos| handle_rect.contains(pointer_pos))
            });
        let handle_color = match response.as_ref() {
            Some(response) if response.is_pointer_button_down_on() || response.dragged() => {
                palette.dragged_handle
            }
            Some(response) if response.hovered() && pointer_on_handle => palette.hovered_handle,
            _ => palette.resting_handle,
        };

        if ui.is_rect_visible(track_rect) {
            let corner_radius = EDITOR_SCROLLBAR_HANDLE_MAX_CORNER_RADIUS
                .min(handle_rect.width().min(handle_rect.height()) * 0.5);
            ui.painter().add(egui::epaint::Shape::rect_filled(
                handle_rect,
                corner_radius,
                handle_color,
            ));
        }
    }
}

impl ScriptCraftApp {
    pub(crate) fn live_document_surface(&mut self, ui: &mut egui::Ui, surface: EditorSurface) {
        let phase_started = self
            .development_mode
            .then(|| (Instant::now(), self.performance.paint_nanos));
        if self.development_mode {
            self.responsiveness.begin_document_surface(surface);
        }
        self.live_document_surface_inner(ui, surface);
        if let Some((started, paint_nanos_before)) = phase_started {
            self.responsiveness.record_document_surface(
                started.elapsed(),
                paint_nanos_before,
                self.performance.paint_nanos,
            );
        }
    }

    fn live_document_surface_inner(&mut self, ui: &mut egui::Ui, surface: EditorSurface) {
        let element_windows_own_input = self.element_windows.owns_input();
        let presentation_mode = resolve_presentation_mode(surface, || self.script_view_mode);
        let layout = match self.layout_for_render(surface) {
            Ok(layout) => layout,
            Err(error) => {
                self.page_geometry.invalidate();
                self.page_scale = None;
                self.show_error(format!(
                    "LAYOUT_BUILD_FAILED: {}; document unchanged",
                    crate::app::diagnostics::safe_layout_error_category(&error)
                ));
                return;
            }
        };
        let Some(prepared_fonts) = self.prepared_document_fonts.clone() else {
            self.show_error(
                "FONT_PREPARATION_FAILED: no prepared environment; document unchanged".to_owned(),
            );
            return;
        };
        let fonts_ready = self
            .document_font_store
            .register_prepared_fonts(ui.ctx(), &prepared_fonts)
            .map_err(|_| ());
        match fonts_ready {
            Err(()) => {
                self.show_error(
                    "FONT_PREPARATION_FAILED: resolved face registration failed; document unchanged"
                        .to_owned(),
                );
                return;
            }
            Ok(false) => {
                // add_font publishes at the next egui pass. Do not allow either
                // editor surface to build a galley with a merely queued family.
                self.page_geometry.invalidate();
                self.page_scale = None;
                ui.ctx().request_repaint();
                return;
            }
            Ok(true) => {}
        }
        self.rendered_revision = Some(self.session.revision());
        let editor_id = ui.id().with(EDITOR_SURFACE_ID);
        let ruler_rect = if surface == EditorSurface::Body && self.settings_store.ruler_visible() {
            let (_, rect) =
                ui.allocate_space(Vec2::new(ui.available_width(), RULER_ALLOCATED_HEIGHT));
            ui.painter()
                .rect_filled(rect, 0.0, self.theme_palette.script_workspace.background);
            Some(rect)
        } else {
            None
        };
        let scroll_outer_rect = ui.available_rect_before_wrap();
        let mut presented_pages = Vec::with_capacity(layout.pages.len());
        let mut frame_projections = std::iter::repeat_with(|| None)
            .take(layout.pages.len())
            .collect::<Vec<Option<Vec<Option<presentation_geometry::PresentationLine<'_>>>>>>();
        let mut rendered_scale = 1.0;
        let mut visible_pages = 0;
        let reveal_intent = self.reveal_intent;
        let pending_zoom_anchor = self.pending_zoom_anchor;
        let zoom_compensation_offset = pending_zoom_anchor.and_then(|anchor| {
            prepaint_zoom_scroll_offset(
                anchor,
                layout.pages.len(),
                layout.inputs.paper_width as f32,
                layout.inputs.paper_height as f32,
                presentation_mode,
                layout.pages.iter().map(|page| {
                    (
                        page.content_bounds.y as f32,
                        page.content_bounds.height as f32,
                    )
                }),
                layout.inputs.font_size as f32,
                self.zoom_percent,
                scroll_outer_rect,
                ui.spacing().item_spacing.y,
            )
            .map(|plan| plan.scroll_offset)
        });
        let zoom_compensation_queued = zoom_compensation_offset.is_some();
        if zoom_compensation_queued {
            add_performance_counter(&mut self.performance.zoom_compensations_queued, 1);
        }
        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        // Temporarily override the current surface Ui style; keep the shared
        // context style and this ScrollArea's persistent identity unchanged.
        let scroll_fade = ui.style().spacing.scroll.fade;
        ui.style_mut().spacing.scroll.fade.strength = 0.0;
        let mut scroll_area = egui::ScrollArea::both()
            .id_salt("page-editor-scroll")
            .content_margin(egui::Margin {
                left: 0,
                right: 0,
                top: SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING as i8,
                bottom: SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING as i8,
            });
        if element_windows_own_input {
            scroll_area = scroll_area.scroll_source(egui::scroll_area::ScrollSource::NONE);
        }
        if let Some(offset) = zoom_compensation_offset {
            scroll_area = scroll_area.scroll_offset(offset);
        }
        let scroll_output = scroll_area.show(ui, |ui| {
            // ScrollArea's child max rect is the actual inner viewport after
            // reserved scrollbar width and parent clipping have been applied.
            let viewport_width = ui.available_width().max(0.0);
            let metrics = presentation_metrics_at_zoom(
                viewport_width,
                layout.inputs.paper_width as f32,
                layout.inputs.font_size as f32,
                self.zoom_percent,
            );
            let geometry = page_scroll_geometry(viewport_width, metrics.page_width);
            debug_assert_eq!(
                geometry.horizontal_overflow,
                geometry.content_width > geometry.viewport_width
            );
            let scale = metrics.scale;
            rendered_scale = scale;
            self.page_scale = Some(scale);
            ui.set_min_width(geometry.content_width);
            // Allocate every page row to preserve full scroll extent and source geometry;
            // page painting is a separate viewport-bounded pass below.
            let editor_clip = ui.clip_rect();
            let vertical_item_spacing = ui.spacing().item_spacing.y;
            if presentation_mode == ScriptViewMode::Speed {
                // Only page-row allocation uses this local Ui style; restore it
                // before paint/reveal widgets and leave the shared context intact.
                ui.spacing_mut().item_spacing.y = 0.0;
            }
            for (page_index, page) in layout.pages.iter().enumerate() {
                add_performance_counter(&mut self.performance.pages_considered, 1);
                let policy = presented_page_policy(
                    presentation_mode,
                    layout.inputs.paper_height as f32,
                    page.content_bounds.y as f32,
                    page.content_bounds.height as f32,
                    scale,
                    vertical_item_spacing,
                );
                ui.horizontal(|ui| {
                    // Explicit geometry owns all horizontal spacing; default
                    // row gaps would inflate measured scroll content.
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(geometry.leading_padding);
                    let page_rect = ui
                        .allocate_space(Vec2::new(metrics.page_width, policy.page_height))
                        .1;
                    let trailing_padding =
                        (geometry.content_width - geometry.leading_padding - metrics.page_width)
                            .max(0.0);
                    ui.add_space(trailing_padding);
                    presented_pages.push(PresentedPage {
                        rect: page_rect,
                        document_y_offset: policy.document_y_offset,
                    });
                });
                if policy.explicit_gap > 0.0 {
                    ui.add_space(policy.explicit_gap);
                }
            }
            if presentation_mode == ScriptViewMode::Speed {
                ui.spacing_mut().item_spacing.y = vertical_item_spacing;
            }
            let paint_range = viewport_paint_page_range(&presented_pages, editor_clip);
            if self.development_mode {
                visible_pages = presented_pages
                    .iter()
                    .filter(|page| page.rect.intersects(editor_clip))
                    .count();
            }
            let painted_in_range = paint_range.map_or(0, |(first, last)| {
                last.saturating_sub(first).saturating_add(1)
            });
            add_performance_counter(
                &mut self.performance.pages_culled,
                presented_pages.len().saturating_sub(painted_in_range) as u64,
            );
            let caret_page_index =
                self.caret_page_index(&layout, surface, selection, reveal_intent, paint_range);
            let mut pages_to_project = paint_range
                .map(|(first, last)| (first..=last).collect::<Vec<_>>())
                .unwrap_or_default();
            if let Some(page_index) = caret_page_index {
                if !pages_to_project.contains(&page_index) {
                    pages_to_project.push(page_index);
                }
            }
            for page_index in pages_to_project {
                if frame_projections[page_index].is_none() {
                    frame_projections[page_index] = Some(
                        layout.pages[page_index]
                            .lines
                            .iter()
                            .map(|line| {
                                self.project_line_for_interaction(ui.ctx(), line, scale)
                                    .ok()
                                    .flatten()
                            })
                            .collect(),
                    );
                }
            }
            let caret_geometry = self.resolve_caret_geometry(
                &layout,
                surface,
                selection,
                reveal_intent,
                caret_page_index,
                &frame_projections,
                scale,
            );
            let caret_paint_page_index = (selection.anchor == selection.focus)
                .then_some(caret_page_index)
                .flatten();
            reveal::reveal_caret_if_needed(
                ui,
                reveal_intent,
                zoom_compensation_queued,
                caret_page_index,
                caret_geometry,
                &presented_pages,
                scale,
                editor_clip,
            );
            if let Some((first, last)) = paint_range {
                for (page_index, presented_page) in presented_pages
                    .iter()
                    .enumerate()
                    .take(last.saturating_add(1))
                    .skip(first)
                {
                    self.paint_page(
                        ui,
                        &layout,
                        page_index,
                        *presented_page,
                        metrics,
                        frame_projections[page_index].as_deref().unwrap_or(&[]),
                        editor_id,
                        caret_paint_page_index,
                        caret_geometry,
                        surface,
                        presentation_mode,
                    );
                }
            }
        });
        if let (Some(strip), Some(first_page)) = (ruler_rect, presented_pages.first()) {
            paint_ruler(
                ui,
                strip,
                first_page.rect.left(),
                first_page.rect.width(),
                self.theme_palette.script_workspace.page.paper,
                rendered_scale,
                layout.inputs.paper_width,
                selection.focus.element,
                &layout,
                &mut self.ruler,
                ui.visuals().dark_mode,
            );
        }
        let editor_scrollbar_palette = self.theme_palette.script_workspace.editor_scrollbar;
        paint_editor_scrollbar_handle(ui, &scroll_output, editor_scrollbar_palette);
        if self.development_mode {
            self.responsiveness
                .note_revision_painted(self.session.revision(), Instant::now());
            if surface == EditorSurface::Body {
                self.responsiveness
                    .note_pages(visible_pages, layout.pages.len());
            }
        }
        ui.style_mut().spacing.scroll.fade = scroll_fade;
        let scale = rendered_scale;
        let published_scroll_offset = scroll_output.state.offset;
        self.reveal_intent_after_frame(zoom_compensation_queued);

        let actual_clip = scroll_output.inner_rect.intersect(ui.clip_rect());
        let geometry_count_matches = presented_pages.len() == layout.pages.len();
        let zoom_anchor_was_pending = pending_zoom_anchor.is_some();
        let needs_repaint = if geometry_count_matches {
            self.page_geometry.publish(
                self.performance.frames,
                self.session.revision(),
                ui.ctx().content_rect(),
                scale,
                &presented_pages,
                actual_clip,
                published_scroll_offset,
            )
        } else {
            self.page_geometry.invalidate();
            true
        };
        if geometry_count_matches {
            // The request is consumed only after the scaled page geometry has
            // been published. Invalid/missing anchors simply preserve old behavior.
            if zoom_anchor_was_pending {
                add_performance_counter(&mut self.performance.zoom_geometry_published, 1);
                add_performance_counter(&mut self.performance.zoom_anchor_consumed, 1);
            }
            self.pending_zoom_anchor = None;
        }
        if needs_repaint {
            self.request_repaint(ui.ctx());
        }

        // This is the only interactive response in the painted editor. Pages remain
        // source-mapped geometry, not independent focusable widgets.
        let mut editor_interaction_rect = scroll_output.inner_rect;
        if !element_windows_own_input {
            let scrollbar_width = ui.style().spacing.scroll.bar_width;
            if scroll_output.content_size.y > scroll_output.inner_rect.height() {
                editor_interaction_rect.max.x = (editor_interaction_rect.max.x - scrollbar_width)
                    .max(editor_interaction_rect.min.x);
            }
            if scroll_output.content_size.x > scroll_output.inner_rect.width() {
                editor_interaction_rect.max.y = (editor_interaction_rect.max.y - scrollbar_width)
                    .max(editor_interaction_rect.min.y);
            }
        }
        let response = ui
            .interact(
                editor_interaction_rect,
                editor_id,
                if element_windows_own_input {
                    Sense::hover()
                } else {
                    Sense::click_and_drag()
                },
            )
            .on_hover_cursor(document_cursor_icon());
        if !element_windows_own_input {
            self.handle_pointer_response(
                ui,
                editor_id,
                &response,
                &layout,
                &presented_pages,
                scale,
                &frame_projections,
                surface,
            );
        }
        self.native_input_diagnostics.focused = self.surface_focused && !element_windows_own_input;
        if self.surface_focused && !element_windows_own_input {
            ui.ctx().memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    editor_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
            self.native_input_diagnostics.focus_lock_installed = true;
        } else {
            self.native_input_diagnostics.focus_lock_installed = false;
        }
    }
}

fn document_cursor_icon() -> egui::CursorIcon {
    egui::CursorIcon::Text
}
