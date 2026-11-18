//! Script View page, selection, decoration, caret, and IME painting.

use super::{
    caret::{caret_blink_id, caret_blink_visible, viewport_caret_rect, CaretPresentation},
    page_policy::{
        document_y_to_screen, normal_view_page_bottom_rule_geometry,
        normal_view_page_bottom_rule_visible, page_chrome_visible, page_local_screen_origin,
        NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH,
    },
    text::{paint_fixed_metric_text_with_selection, renderer_font, resolve_span_paint, FixedMetricTextStyle},
    zoom_scroll::PresentationMetrics, ScriptCraftApp,
};
use crate::{
    app::add_performance_counter,
    chrome::{status_bar::PresentedPage, theme},
    presentation::{geometry as presentation_geometry, style::AdvanceMode},
};
use eframe::egui::{self, Color32, Stroke, Vec2};
use scriptcraft_commands::{EditorSurface, ScriptViewMode};
use scriptcraft_layout::{
    Caret, FixedRect, LayoutLine, PositionedStyledSpan, ScreenplayLayout,
};
use scriptcraft_session::editing::{Position, Selection};
use std::time::Instant;

const PAGE_BORDER_WIDTH: f32 = 2.5;

impl ScriptCraftApp {
    fn selection_range(
        &self,
        line_start: usize,
        line_end: usize,
        element: usize,
        surface: EditorSurface,
    ) -> Option<(usize, usize)> {
        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        let (start, end) = ordered_selection(selection);
        if start.element > element || end.element < element {
            return None;
        }
        let from = if start.element == element {
            start.offset
        } else {
            line_start
        };
        let to = if end.element == element {
            end.offset
        } else {
            line_end
        };
        (from < line_end && to > line_start).then_some((from, to))
    }

    fn selected_display_range(
        span: &PositionedStyledSpan,
        source_selection: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
        let (selection_start, selection_end) = source_selection?;
        let mut selected_start = None;
        let mut selected_end = None;
        for segment in span.display_source_map.iter() {
            if selection_start < segment.source_end && selection_end > segment.source_start {
                selected_start = Some(selected_start.map_or(segment.display_start, |current: usize| {
                    current.min(segment.display_start)
                }));
                selected_end = Some(selected_end.map_or(segment.display_end, |current: usize| {
                    current.max(segment.display_end)
                }));
            }
        }
        selected_start.zip(selected_end)
    }

    fn span_for_caret<'a>(
        line: &'a scriptcraft_layout::LayoutLine,
        offset: usize,
    ) -> Option<&'a PositionedStyledSpan> {
        line.spans
            .iter()
            .find(|span| offset >= span.source.start && offset < span.source.end)
            .or_else(|| {
                line.spans
                    .iter()
                    .rev()
                    .find(|span| offset == span.source.end)
            })
    }

    // Rendering needs these geometry and interaction inputs together; wrapping
    // them in a temporary parameter object would add noise to the paint path.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_page(
        &mut self,
        ui: &mut egui::Ui,
        layout: &ScreenplayLayout,
        page_index: usize,
        presented_page: PresentedPage,
        metrics: PresentationMetrics,
        projections: &[Option<presentation_geometry::PresentationLine<'_>>],
        editor_id: egui::Id,
        caret_page_index: Option<usize>,
        caret_geometry: Option<CaretPresentation>,
        surface: EditorSurface,
        presentation_mode: ScriptViewMode,
    ) {
        let scale = metrics.scale;
        let page_rect = presented_page.rect;
        let paint_started = Instant::now();
        self.performance.painted_pages = self.performance.painted_pages.saturating_add(1);
        add_performance_counter(
            &mut self.performance.lines_submitted,
            layout.pages[page_index].lines.len() as u64,
        );
        let painter = ui.painter_at(page_rect);
        let palette = self.theme_palette.script_workspace.page;
        if page_chrome_visible(presentation_mode) {
            painter.rect_filled(page_rect, 2.0, palette.paper);
            painter.rect_stroke(
                page_rect,
                2.0,
                Stroke::new(PAGE_BORDER_WIDTH, palette.border),
                egui::StrokeKind::Middle,
            );
        }
        if normal_view_page_bottom_rule_visible(
            presentation_mode,
            page_index,
            layout.pages.len(),
        ) {
            let rule_painter = ui.painter().with_clip_rect(ui.clip_rect());
            rule_painter.line_segment(
                normal_view_page_bottom_rule_geometry(page_rect),
                Stroke::new(NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH, palette.text),
            );
        }

        if surface == EditorSurface::Body
            && layout.purpose == scriptcraft_layout::LayoutPurpose::Editor
        {
            if let Some(active_group_id) = layout.active_group_id.as_ref() {
                for boundary in layout
                    .editor_group_boundaries
                    .iter()
                    .filter(|boundary| &boundary.group_id == active_group_id)
                {
                    for (boundary_page, geometry) in [
                        (boundary.upper_page, boundary.upper),
                        (boundary.lower_page, boundary.lower),
                    ] {
                        if boundary_page != page_index {
                            continue;
                        }
                        let (start_x, end_x, y) =
                            boundary_screen_geometry(presented_page, geometry, scale);
                        paint_dotted_boundary(
                            &painter,
                            start_x,
                            end_x,
                            y,
                            ui.ctx().pixels_per_point(),
                            palette.dual_dialogue_boundary,
                        );
                    }
                }
            }
        }

        'paint_lines: for (line_index, line) in layout.pages[page_index].lines.iter().enumerate() {
            let selection = self.selection_range(
                line.source.start,
                line.source.end,
                line.element,
                surface,
            );
            let resolved_paints = line
                .spans
                .iter()
                .map(|span| {
                    resolve_span_paint(
                        &span.text_style,
                        span.text_style.size as f32 * scale,
                        palette,
                        &self.document_font_availability,
                    )
                })
                .collect::<Vec<_>>();
            let has_proportional = resolved_paints
                .iter()
                .any(|resolved| resolved.advance_mode == AdvanceMode::Proportional);

            if has_proportional {
                // Unsupported natural topology fails closed. The same projection
                // object is also consumed by caret/reveal/IME and pointer dispatch.
                let Some(projection) = projections.get(line_index).and_then(Option::as_ref) else {
                    continue 'paint_lines;
                };
                debug_assert!(std::ptr::eq(projection.core, line));
                debug_assert!(projection.total_advance.is_finite());
                if let Some(first_span) = projection.spans.first() {
                    debug_assert_eq!(first_span.origin_x, projection.line_origin_x);
                }
                let page_origin = page_local_screen_origin(presented_page, scale);
                let line_clip = projection
                    .clip_bounds
                    .translate(page_origin.to_vec2())
                    .intersect(page_rect);
                let line_painter = painter.with_clip_rect(line_clip);
                for (projected, resolved) in projection.spans.iter().zip(&resolved_paints) {
                    let span = projected.core;
                    let span_y = document_y_to_screen(
                        presented_page,
                        span.geometry.y.to_points_f32(),
                        scale,
                    );
                    let span_height = span.geometry.height.to_points_f32() * scale;
                    let span_origin = egui::pos2(
                        page_rect.left() + projected.origin_x,
                        span_y,
                    );
                    let selected_display = Self::selected_display_range(span, selection);
                    if let Some((from, to)) = selected_display {
                        add_performance_counter(&mut self.performance.selection_spans_submitted, 1);
                        let boundaries = &projected.display_boundary_offsets;
                        let start = from.min(boundaries.len().saturating_sub(1));
                        let end = to.min(boundaries.len().saturating_sub(1));
                        let selected_x = span_origin.x + boundaries[start];
                        let selected_width = (boundaries[end] - boundaries[start]).max(0.0);
                        let selected_rect = egui::Rect::from_min_size(
                            egui::pos2(selected_x, span_y),
                            Vec2::new(selected_width.max(1.0), span_height),
                        );
                        line_painter.rect_filled(selected_rect, 0.0, palette.selection);
                        line_painter.rect_stroke(
                            selected_rect,
                            0.0,
                            Stroke::new(1.0, palette.selection_boundary),
                            egui::StrokeKind::Inside,
                        );
                    }

                    if projected.advance_mode == AdvanceMode::Proportional {
                        if let Some(geometry) = projected.natural.as_ref() {
                            line_painter.galley(
                                span_origin,
                                geometry.galley.clone(),
                                resolved.color,
                            );
                            if let Some((from, to)) = selected_display {
                                let boundaries = &projected.display_boundary_offsets;
                                let start = from.min(boundaries.len().saturating_sub(1));
                                let end = to.min(boundaries.len().saturating_sub(1));
                                let selected_rect = egui::Rect::from_min_max(
                                    egui::pos2(span_origin.x + boundaries[start], span_y),
                                    egui::pos2(
                                        span_origin.x + boundaries[end],
                                        span_y + span_height,
                                    ),
                                );
                                let selected_clip = line_clip.intersect(selected_rect);
                                if selected_rect.width() > 0.0
                                    && selected_clip.width() > 0.0
                                    && selected_clip.height() > 0.0
                                {
                                    let selected_galley =
                                        presentation_geometry::recolor_natural_galley(
                                            geometry,
                                            palette.selection_text,
                                        );
                                    painter
                                        .with_clip_rect(selected_clip)
                                        .galley(span_origin, selected_galley, palette.selection_text);
                                }
                            }
                        }
                    } else {
                        let display_count = span.display_text.chars().count();
                        let cell_width = if display_count == 0 {
                            span.char_width.to_points_f32() * scale
                        } else {
                            projected.advance / display_count as f32
                        };
                        paint_fixed_metric_text_with_selection(
                            &line_painter,
                            span_origin,
                            &span.display_text,
                            resolved.font.clone(),
                            cell_width,
                            FixedMetricTextStyle {
                                color: resolved.color,
                                selected_color: palette.selection_text,
                                selection: selected_display,
                            },
                            &mut self.performance,
                        );
                    }

                    if span.underline || span.strikethrough {
                        if span.underline {
                            let underline_y = span_y + span_height * 0.88;
                            line_painter.line_segment(
                                [
                                    egui::pos2(span_origin.x, underline_y),
                                    egui::pos2(span_origin.x + projected.advance, underline_y),
                                ],
                                Stroke::new(1.0_f32, resolved.color),
                            );
                        }
                        if span.strikethrough {
                            let strike_y = span_y + span_height * 0.52;
                            line_painter.line_segment(
                                [
                                    egui::pos2(span_origin.x, strike_y),
                                    egui::pos2(span_origin.x + projected.advance, strike_y),
                                ],
                                Stroke::new(1.0_f32, resolved.color),
                            );
                        }
                    }
                }
                continue;
            }

            for (span, resolved_paint) in line.spans.iter().zip(&resolved_paints) {
                let span_x = page_rect.left() + span.geometry.x.to_points_f32() * scale;
                let span_y = document_y_to_screen(
                    presented_page,
                    span.geometry.y.to_points_f32(),
                    scale,
                );
                let span_height = span.geometry.height.to_points_f32() * scale;
                let span_width = span.geometry.width.to_points_f32() * scale;
                let display_count = span.display_text.chars().count();
                if display_count == 0 {
                    continue;
                }
                let nominal_cell = span.char_width.to_points_f32() * scale;
                let cell_width = if span_width > nominal_cell * display_count as f32 {
                    span_width / display_count as f32
                } else {
                    nominal_cell
                };
                let selected_display = Self::selected_display_range(span, selection);
                if let Some((from, to)) = selected_display {
                    add_performance_counter(&mut self.performance.selection_spans_submitted, 1);
                    let selected_x = span_x + from as f32 * cell_width;
                    let selected_width = if display_count == 1 {
                        span_width
                    } else {
                        to.saturating_sub(from) as f32 * cell_width
                    };
                    let selected_rect = egui::Rect::from_min_size(
                        egui::pos2(selected_x, span_y),
                        Vec2::new(selected_width.max(1.0), span_height),
                    );
                    painter.rect_filled(selected_rect, 0.0, palette.selection);
                    painter.rect_stroke(
                        selected_rect,
                        0.0,
                        Stroke::new(1.0, palette.selection_boundary),
                        egui::StrokeKind::Inside,
                    );
                }
                let span_font = resolved_paint.font.clone();
                let color = resolved_paint.color;
                paint_fixed_metric_text_with_selection(
                    &painter,
                    egui::pos2(span_x, span_y),
                    &span.display_text,
                    span_font,
                    cell_width,
                    FixedMetricTextStyle {
                        color,
                        selected_color: palette.selection_text,
                        selection: selected_display,
                    },
                    &mut self.performance,
                );
                if span.underline || span.strikethrough {
                    if span.underline {
                        let underline_y = span_y + span_height * 0.88;
                        painter.line_segment(
                            [
                                egui::pos2(span_x, underline_y),
                                egui::pos2(span_x + span_width, underline_y),
                            ],
                            Stroke::new(1.0_f32, color),
                        );
                    }
                    if span.strikethrough {
                        let strike_y = span_y + span_height * 0.52;
                        painter.line_segment(
                            [
                                egui::pos2(span_x, strike_y),
                                egui::pos2(span_x + span_width, strike_y),
                            ],
                            Stroke::new(1.0_f32, color),
                        );
                    }
                }
            }
        }

        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        if caret_page_index == Some(page_index)
            && selection.anchor == selection.focus
        {
            if let Some(caret_geometry) = caret_geometry {
                let (caret_rect, caret_clip) =
                    viewport_caret_rect(caret_geometry, presented_page, scale);
                let caret = Caret {
                    element: selection.focus.element,
                    offset: selection.focus.offset,
                };
                let blink_id = caret_blink_id(
                    editor_id,
                    self.session.revision(),
                    caret.clone(),
                    self.caret_blink_focus_generation,
                );
                let blink_active = self.surface_focused;
                if caret_blink_visible(
                    ui.ctx(),
                    blink_id,
                    blink_active,
                    true,
                    self.caret_blink_reset_time,
                ) {
                    if let Some(clip) = caret_clip {
                        painter.with_clip_rect(clip).rect_filled(caret_rect, 0.0, palette.caret);
                    } else {
                        painter.rect_filled(caret_rect, 0.0, palette.caret);
                    }
                }
                if self.surface_focused {
                    self.performance.repaint_requests =
                        self.performance.repaint_requests.saturating_add(1);
                }
                if !self.ime_preedit.is_empty() {
                    let caret_line = layout.pages[page_index].lines.iter().find(|line| {
                        line.element == caret.element
                            && caret.offset >= line.source.start
                            && caret.offset <= line.source.end
                    });
                    let caret_span = caret_line.and_then(|line| Self::span_for_caret(line, caret.offset));
                    let ime_font = caret_span.map_or_else(
                        || renderer_font(&layout.inputs.font_family, metrics.font_size),
                        |span| resolve_span_paint(
                            &span.text_style,
                            span.text_style.size as f32 * scale,
                            palette,
                            &self.document_font_availability,
                        ).font,
                    );
                    let ime_painter = caret_clip.map_or_else(
                        || painter.clone(),
                        |clip| painter.with_clip_rect(clip),
                    );
                    ime_painter.text(
                        caret_rect.min,
                        egui::Align2::LEFT_TOP,
                        &self.ime_preedit,
                        ime_font,
                        palette.ime_text,
                    );
                }
            }
        }

        self.performance.paint_nanos = self
            .performance
            .paint_nanos
            .saturating_add(paint_started.elapsed().as_nanos());
    }
}

fn paint_dotted_boundary(
    painter: &egui::Painter,
    start_x: f32,
    end_x: f32,
    y: f32,
    pixels_per_point: f32,
    color: Color32,
) {
    let physical_pixel = boundary_stroke_width(pixels_per_point);
    for [start, end] in dotted_boundary_segments(start_x, end_x, y, pixels_per_point) {
        painter.line_segment(
            [start, end],
            Stroke::new(physical_pixel, color),
        );
    }
}

fn boundary_stroke_width(pixels_per_point: f32) -> f32 {
    1.0 / pixels_per_point.max(f32::MIN_POSITIVE)
}

fn boundary_screen_geometry(
    page: PresentedPage,
    geometry: FixedRect,
    scale: f32,
) -> (f32, f32, f32) {
    let start_x = page.rect.left() + geometry.x.to_points_f32() * scale;
    let end_x = start_x + geometry.width.to_points_f32() * scale;
    let y = document_y_to_screen(page, geometry.y.to_points_f32(), scale);
    (start_x, end_x, y)
}

fn dotted_boundary_segments(
    start_x: f32,
    end_x: f32,
    y: f32,
    pixels_per_point: f32,
) -> Vec<[egui::Pos2; 2]> {
    let physical_pixel = 1.0 / pixels_per_point.max(f32::MIN_POSITIVE);
    let dash = physical_pixel * 2.0;
    let gap = physical_pixel * 2.0;
    let mut segments = Vec::new();
    let mut x = start_x;
    while x < end_x {
        let next = (x + dash).min(end_x);
        segments.push([egui::pos2(x, y), egui::pos2(next, y)]);
        x = next + gap;
    }
    segments
}

pub(super) fn ordered_selection(selection: Selection) -> (Position, Position) {
    if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}
