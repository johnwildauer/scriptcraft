use super::{
    add_performance_counter, add_performance_nanos, document_font, status_bar, theme,
    PerformanceDiagnostics, RevealIntent, ScriptCraftApp,
};
use crate::presentation_style::AdvanceMode;
use super::app_commands::CommandInvocation;
use super::shortcuts::{editor_command_for_key_event, ShortcutPlatform};
use eframe::egui::{self, Color32, FontFamily, FontId, Key, Sense, Stroke, Vec2};
use egui::epaint::text::{Galley, LayoutJob};
use scriptcraft_commands::{
    Command, CommandId, CommandOrigin, EditorSurface, HorizontalDirection, LineEdge,
    PointerPhase, VerticalDirection,
};
use scriptcraft_core::editing::{EditError, Position, Selection};
use scriptcraft_core::formatting::EffectiveTextStyle;
use scriptcraft_core::layout::{
    Caret, FixedRect, InteractionQueryDiagnostics, LayoutLine, PositionedStyledSpan,
    ScreenplayLayout,
};
use scriptcraft_core::FixedPoint;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const EDITOR_SURFACE_ID: &str = "page-editor-surface";
const CARET_WIDTH: f32 = 1.5;
const PAGE_HORIZONTAL_PADDING: f32 = 16.0;
const PAGE_VERTICAL_GAP: f32 = 12.0;
const SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING: f32 = 8.0;
const SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING: f32 = 0.0;
const PAGE_BORDER_WIDTH: f32 = 2.5;
const PAGE_PAINT_OVERSCAN: usize = 1;
const FIXED_CELL_GALLEY_CACHE_ID: &str = "scriptcraft-fixed-cell-galley-cache-v1";
const FIXED_CELL_GALLEY_CACHE_LIMIT: usize = 16_384;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
enum CaretPresentation {
    Fixed(FixedRect),
    Projected(super::presentation_geometry::ProjectedCaretRect),
}

fn viewport_caret_rect(
    presentation: CaretPresentation,
    page_rect: egui::Rect,
    scale: f32,
) -> (egui::Rect, Option<egui::Rect>) {
    match presentation {
        CaretPresentation::Fixed(rect) => (
            egui::Rect::from_min_size(
                egui::pos2(
                    page_rect.left() + rect.x.to_points_f32() * scale,
                    page_rect.top() + rect.y.to_points_f32() * scale,
                ),
                Vec2::new(CARET_WIDTH, rect.height.to_points_f32() * scale),
            ),
            None,
        ),
        CaretPresentation::Projected(projected) => {
            let viewport_rect = projected.rect.translate(page_rect.min.to_vec2());
            let viewport_clip = egui::Rect::from_min_max(
                page_rect.min + projected.clip_bounds.min.to_vec2(),
                page_rect.min + projected.clip_bounds.max.to_vec2(),
            )
            .intersect(page_rect);
            (viewport_rect, Some(viewport_clip))
        }
    }
}

fn dual_group_navigation_edge(
    project: &scriptcraft_core::Project,
    group: &scriptcraft_core::dual_dialogue::DualDialogueGroup,
    forward: bool,
) -> Option<Position> {
    let element = if forward {
        scriptcraft_core::dual_dialogue::first_member(project, group)?
    } else {
        scriptcraft_core::dual_dialogue::last_member(project, group)?
    };
    let offset = if forward {
        0
    } else {
        project.elements.get(element)?.text.len()
    };
    Some(Position { element, offset })
}

fn viewport_paint_page_range(
    page_rects: &[egui::Rect],
    editor_clip: egui::Rect,
) -> Option<(usize, usize)> {
    let mut first_visible = None;
    let mut last_visible = None;
    for (page_index, page_rect) in page_rects.iter().enumerate() {
        if page_rect.intersects(editor_clip) {
            first_visible.get_or_insert(page_index);
            last_visible = Some(page_index);
        }
    }
    let first_visible = first_visible?;
    let last_visible = last_visible?;
    Some((
        first_visible.saturating_sub(PAGE_PAINT_OVERSCAN),
        last_visible
            .saturating_add(PAGE_PAINT_OVERSCAN)
            .min(page_rects.len().saturating_sub(1)),
    ))
}

pub(super) fn has_nonzero_mouse_wheel(events: &[egui::Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            egui::Event::MouseWheel { delta, .. } if *delta != Vec2::ZERO
        )
    })
}


impl ScriptCraftApp {
    fn caret_from_pointer(
        &mut self,
        layout: &ScreenplayLayout,
        page: usize,
        page_rect: egui::Rect,
        scale: f32,
        pointer: egui::Pos2,
        projections: Option<&[Option<Vec<Option<super::presentation_geometry::PresentationLine<'_>>>>]>,
    ) -> Option<Position> {
        let local_x =
            FixedPoint::from_points_f64(f64::from((pointer.x - page_rect.left()) / scale))?;
        let local_y =
            FixedPoint::from_points_f64(f64::from((pointer.y - page_rect.top()) / scale))?;
        let started = Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let result = layout
            .nearest_line_location_fixed_with_diagnostics(
                page,
                local_x,
                local_y,
                &mut query_diagnostics,
            )
            .and_then(|located| {
                let line_index = layout.pages.get(located.page_index)?.lines.iter()
                    .position(|line| std::ptr::eq(line, located.line));
                let projected = line_index
                    .and_then(|index| {
                        projections
                            .and_then(|pages| pages.get(located.page_index))
                            .and_then(Option::as_ref)
                            .and_then(|lines| lines.get(index))
                    })
                    .and_then(Option::as_ref);
                if let Some(projected) = projected {
                    let screen_x = pointer.x - page_rect.left();
                    Some(super::presentation_geometry::projected_caret_for_x(
                        projected,
                        screen_x,
                    ))
                } else {
                    let has_proportional = located.line.spans.iter().any(|span| {
                        resolve_span_paint(
                            &span.text_style,
                            span.text_style.size as f32 * scale,
                            self.theme_palette.script_workspace.page,
                            &self.document_font_availability,
                        ).advance_mode == AdvanceMode::Proportional
                    });
                    (!has_proportional).then(|| located.fixed_caret_for_x(local_x))
                }
            })
            .map(|caret| Position {
                element: caret.element,
                offset: caret.offset,
            });
        self.performance.indexed_interaction_queries = self
            .performance
            .indexed_interaction_queries
            .saturating_add(1);
        self.performance.interaction_index_entries_examined = self
            .performance
            .interaction_index_entries_examined
            .saturating_add(query_diagnostics.index_entries_examined);
        self.performance.caret_lookup_nanos = self
            .performance
            .caret_lookup_nanos
            .saturating_add(started.elapsed().as_nanos());
        result
    }

    pub(super) fn project_line_for_interaction<'a>(
        &mut self,
        ctx: &egui::Context,
        line: &'a LayoutLine,
        scale: f32,
    ) -> Result<Option<super::presentation_geometry::PresentationLine<'a>>, ()> {
        let palette = self.theme_palette.script_workspace.page;
        let resolved = line.spans.iter().map(|span| {
            resolve_span_paint(
                &span.text_style,
                span.text_style.size as f32 * scale,
                palette,
                &self.document_font_availability,
            )
        }).collect::<Vec<_>>();
        if !resolved.iter().any(|paint| paint.advance_mode == AdvanceMode::Proportional) {
            return Ok(None);
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("scriptcraft-interaction-projection"),
        ));
        let mut natural = Vec::with_capacity(line.spans.len());
        for (span, paint) in line.spans.iter().zip(&resolved) {
            if paint.advance_mode == AdvanceMode::Proportional {
                let (geometry, reused) = super::presentation_geometry::natural_singleline_geometry(
                    &painter,
                    ctx,
                    &span.display_text,
                    &paint.font,
                    &span.text_style,
                    paint.advance_mode,
                    paint.color,
                ).map_err(|_| ())?;
                if reused {
                    self.performance.galleys_reused = self.performance.galleys_reused.saturating_add(1);
                } else {
                    self.performance.galleys_built = self.performance.galleys_built.saturating_add(1);
                }
                natural.push(Some(geometry));
            } else {
                natural.push(None);
            }
        }
        let modes = resolved.iter().map(|paint| paint.advance_mode).collect::<Vec<_>>();
        super::presentation_geometry::project_line(line, &modes, &natural, scale)
            .map(Some)
            .map_err(|_| ())
    }

    fn pointer_surface_page(
        layout: &ScreenplayLayout,
        page_rects: &[egui::Rect],
        pointer: egui::Pos2,
        surface: EditorSurface,
    ) -> Option<usize> {
        page_rects
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                layout.pages[*index].title_page == (surface == EditorSurface::TitlePage)
            })
            .min_by_key(|(index, rect)| {
                let distance = if pointer.y < rect.top() {
                    rect.top() - pointer.y
                } else if pointer.y > rect.bottom() {
                    pointer.y - rect.bottom()
                } else {
                    0.0
                };
                (distance.to_bits(), *index)
            })
            .map(|(index, _)| index)
    }

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
        line: &'a scriptcraft_core::layout::LayoutLine,
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
    fn paint_page(
        &mut self,
        ui: &mut egui::Ui,
        layout: &ScreenplayLayout,
        page_index: usize,
        page_rect: egui::Rect,
        metrics: PresentationMetrics,
        projections: &[Option<super::presentation_geometry::PresentationLine<'_>>],
        editor_id: egui::Id,
        caret_page_index: Option<usize>,
        caret_geometry: Option<CaretPresentation>,
        surface: EditorSurface,
    ) {
        let scale = metrics.scale;
        let paint_started = Instant::now();
        self.performance.painted_pages = self.performance.painted_pages.saturating_add(1);
        add_performance_counter(
            &mut self.performance.lines_submitted,
            layout.pages[page_index].lines.len() as u64,
        );
        let painter = ui.painter_at(page_rect);
        let palette = self.theme_palette.script_workspace.page;
        painter.rect_filled(page_rect, 2.0, palette.paper);
        painter.rect_stroke(
            page_rect,
            2.0,
            Stroke::new(PAGE_BORDER_WIDTH, palette.border),
            egui::StrokeKind::Middle,
        );

        if surface == EditorSurface::Body
            && layout.purpose == scriptcraft_core::layout::LayoutPurpose::Editor
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
                            boundary_screen_geometry(page_rect, geometry, scale);
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
                let line_clip = page_rect.intersect(egui::Rect::from_min_max(
                    page_rect.min + projection.clip_bounds.min.to_vec2(),
                    page_rect.min + projection.clip_bounds.max.to_vec2(),
                ));
                let line_painter = painter.with_clip_rect(line_clip);
                for (projected, resolved) in projection.spans.iter().zip(&resolved_paints) {
                    let span = projected.core;
                    let span_y = page_rect.top() + span.geometry.y.to_points_f32() * scale;
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
                                        super::presentation_geometry::recolor_natural_galley(
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
                            ui.ctx(),
                            span_origin,
                            &span.display_text,
                            resolved.font.clone(),
                            cell_width,
                            span_height,
                            FixedMetricTextStyle {
                                color: resolved.color,
                                selected_color: palette.selection_text,
                                selection: selected_display,
                            },
                            &span.text_style,
                            projected.advance,
                            span.underline,
                            span.strikethrough,
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
                let span_y = page_rect.top() + span.geometry.y.to_points_f32() * scale;
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
                    ui.ctx(),
                    egui::pos2(span_x, span_y),
                    &span.display_text,
                    span_font,
                    cell_width,
                    span_height,
                    FixedMetricTextStyle {
                        color,
                        selected_color: palette.selection_text,
                        selection: selected_display,
                    },
                    &span.text_style,
                    span_width,
                    span.underline,
                    span.strikethrough,
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
                let (caret_rect, caret_clip) = viewport_caret_rect(caret_geometry, page_rect, scale);
                let caret = Caret {
                    element: selection.focus.element,
                    offset: selection.focus.offset,
                };
                let blink_id = editor_id.with((
                    "caret-blink",
                    self.session.revision(),
                    caret.element,
                    caret.offset,
                    self.caret_blink_focus_generation,
                ));
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

    pub(super) fn live_document_surface(&mut self, ui: &mut egui::Ui, surface: EditorSurface) {
        let layout = match self.layout_for_render(surface) {
            Ok(layout) => layout,
            Err(error) => {
                self.page_geometry.invalidate();
                self.page_scale = None;
                self.show_error(format!(
                    "LAYOUT_BUILD_FAILED: {}; document unchanged",
                    crate::safe_layout_error_category(&error)
                ));
                return;
            }
        };
        self.rendered_revision = Some(self.session.revision());
        let editor_id = ui.id().with(EDITOR_SURFACE_ID);
        let scroll_outer_rect = ui.available_rect_before_wrap();
        let mut page_rects = Vec::with_capacity(layout.pages.len());
        let mut frame_projections = std::iter::repeat_with(|| None)
            .take(layout.pages.len())
            .collect::<Vec<
                Option<Vec<Option<super::presentation_geometry::PresentationLine<'_>>>>,
            >>();
        let mut rendered_scale = 1.0;
        let reveal_intent = self.reveal_intent;
        let pending_zoom_anchor = self.pending_zoom_anchor;
        let zoom_compensation_offset = pending_zoom_anchor.and_then(|anchor| {
            prepaint_zoom_scroll_offset(
                anchor,
                layout.pages.len(),
                layout.inputs.paper_width as f32,
                layout.inputs.paper_height as f32,
                layout.inputs.font_size as f32,
                self.zoom_percent,
                scroll_outer_rect,
                ui.spacing().item_spacing.y,
            )
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
            for _ in 0..layout.pages.len() {
                add_performance_counter(&mut self.performance.pages_considered, 1);
                ui.horizontal(|ui| {
                    // Explicit geometry owns all horizontal spacing; default
                    // row gaps would inflate measured scroll content.
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(geometry.leading_padding);
                    let page_height = layout.inputs.paper_height as f32 * scale;
                    let page_rect = ui
                        .allocate_space(Vec2::new(metrics.page_width, page_height))
                        .1;
                    let trailing_padding =
                        (geometry.content_width - geometry.leading_padding - metrics.page_width)
                            .max(0.0);
                    ui.add_space(trailing_padding);
                    page_rects.push(page_rect);
                });
                ui.add_space(PAGE_VERTICAL_GAP);
            }
            let paint_range = viewport_paint_page_range(&page_rects, editor_clip);
            let painted_in_range = paint_range.map_or(0, |(first, last)| {
                last.saturating_sub(first).saturating_add(1)
            });
            add_performance_counter(
                &mut self.performance.pages_culled,
                page_rects.len().saturating_sub(painted_in_range) as u64,
            );
            let caret_page_index = if reveal_intent == RevealIntent::Conditional {
                let mut query_diagnostics = InteractionQueryDiagnostics::default();
                let page = match surface {
                    EditorSurface::Body => layout.page_for_position_in_range_with_diagnostics(
                        selection.focus.element,
                        selection.focus.offset,
                        None,
                        &mut query_diagnostics,
                    ),
                    EditorSurface::TitlePage => layout.title_page_for_position(
                        selection.focus.element,
                        selection.focus.offset,
                    ),
                };
                self.performance.indexed_interaction_queries = self
                    .performance
                    .indexed_interaction_queries
                    .saturating_add(1);
                self.performance.interaction_index_entries_examined = self
                    .performance
                    .interaction_index_entries_examined
                    .saturating_add(query_diagnostics.index_entries_examined);
                page
            } else if selection.anchor == selection.focus {
                paint_range.and_then(|(first, last)| {
                    let mut query_diagnostics = InteractionQueryDiagnostics::default();
                    let page = match surface {
                        EditorSurface::Body => layout.page_for_position_in_range_with_diagnostics(
                            selection.focus.element,
                            selection.focus.offset,
                            Some((first, last)),
                            &mut query_diagnostics,
                        ),
                        EditorSurface::TitlePage => layout.title_page_for_position(
                            selection.focus.element,
                            selection.focus.offset,
                        ).filter(|page| *page >= first && *page <= last),
                    };
                    self.performance.indexed_interaction_queries = self
                        .performance
                        .indexed_interaction_queries
                        .saturating_add(1);
                    self.performance.interaction_index_entries_examined = self
                        .performance
                        .interaction_index_entries_examined
                        .saturating_add(query_diagnostics.index_entries_examined);
                    page
                })
            } else {
                None
            };
            let resolve_caret = caret_page_index.is_some_and(|page_index| {
                reveal_intent == RevealIntent::Conditional
                    || (selection.anchor == selection.focus && !layout.pages[page_index].title_page)
            });
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
            let caret_geometry = if resolve_caret {
                let caret_started = Instant::now();
                self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
                let mut query_diagnostics = InteractionQueryDiagnostics::default();
                let caret = Caret {
                    element: selection.focus.element,
                    offset: selection.focus.offset,
                };
                let located = if surface == EditorSurface::Body {
                    layout.line_for_caret_with_diagnostics(&caret, &mut query_diagnostics)
                } else {
                    // Title's prior wrapper did not publish its internal
                    // diagnostics, so preserve the established desktop count.
                    layout.line_for_caret_with_diagnostics(
                        &caret,
                        &mut InteractionQueryDiagnostics::default(),
                    )
                };
                let result = located
                    .ok()
                    .and_then(|located| {
                        let projection = frame_projections
                            .get(located.page_index)
                            .and_then(Option::as_ref)
                            .and_then(|projections| {
                                layout.pages[located.page_index]
                                    .lines
                                    .iter()
                                    .position(|line| std::ptr::eq(line, located.line))
                                    .and_then(|index| projections.get(index))
                                    .and_then(Option::as_ref)
                            });
                        if let Some(projection) = projection {
                            super::presentation_geometry::projected_caret_rect(
                                projection,
                                caret.offset,
                                scale,
                                CARET_WIDTH,
                            )
                            .map(CaretPresentation::Projected)
                        } else if located.line.spans.iter().any(|span| {
                            resolve_span_paint(
                                &span.text_style,
                                span.text_style.size as f32 * scale,
                                self.theme_palette.script_workspace.page,
                                &self.document_font_availability,
                            )
                            .advance_mode
                                == AdvanceMode::Proportional
                        }) {
                            None
                        } else {
                            located
                                .fixed_caret_geometry(&caret)
                                .ok()
                                .map(CaretPresentation::Fixed)
                        }
                    });
                self.performance.indexed_interaction_queries = self
                    .performance
                    .indexed_interaction_queries
                    .saturating_add(1);
                self.performance.interaction_index_entries_examined = self
                    .performance
                    .interaction_index_entries_examined
                    .saturating_add(query_diagnostics.index_entries_examined);
                self.performance.caret_lookup_nanos = self
                    .performance
                    .caret_lookup_nanos
                    .saturating_add(caret_started.elapsed().as_nanos());
                result
            } else {
                None
            };
            let caret_paint_page_index = (selection.anchor == selection.focus)
                .then_some(caret_page_index)
                .flatten();
            if reveal_intent == RevealIntent::Conditional && !zoom_compensation_queued {
                if let (Some(page_index), Some(caret)) = (caret_page_index, caret_geometry) {
                    let page_rect = page_rects[page_index];
                    let (caret_rect, caret_clip) = viewport_caret_rect(caret, page_rect, scale);
                    let caret_is_clipped_out =
                        caret_clip.is_some_and(|clip| !clip.intersects(caret_rect));
                    if !caret_is_clipped_out && caret_needs_reveal(caret_rect, editor_clip) {
                        // `None` asks egui for nearest/minimal scrolling;
                        // centering would override the writer's viewport.
                        ui.scroll_to_rect(caret_rect, None);
                    }
                }
            }
            if let Some((first, last)) = paint_range {
                for (page_index, page_rect) in page_rects
                    .iter()
                    .enumerate()
                    .take(last.saturating_add(1))
                    .skip(first)
                {
                    self.paint_page(
                        ui,
                        &layout,
                        page_index,
                        *page_rect,
                        metrics,
                        frame_projections[page_index].as_deref().unwrap_or(&[]),
                        editor_id,
                        caret_paint_page_index,
                        caret_geometry,
                        surface,
                    );
                }
            }
        });
        ui.style_mut().spacing.scroll.fade = scroll_fade;
        let scale = rendered_scale;
        let published_scroll_offset = scroll_output.state.offset;
        let published_page_rects = page_rects.clone();
        if !zoom_compensation_queued {
            self.reveal_intent = RevealIntent::None;
        }

        let actual_clip = scroll_output.inner_rect.intersect(ui.clip_rect());
        let geometry_count_matches = page_rects.len() == layout.pages.len();
        let zoom_anchor_was_pending = pending_zoom_anchor.is_some();
        let needs_repaint = if geometry_count_matches {
            self.page_geometry.publish(
                self.performance.frames,
                self.session.revision(),
                ui.ctx().content_rect(),
                scale,
                published_page_rects,
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
        match surface {
            EditorSurface::Body => self.editor_surface_id = Some(editor_id),
            EditorSurface::TitlePage => self.title_editor_surface_id = Some(editor_id),
        }
        let response = ui
            .interact(scroll_output.inner_rect, editor_id, Sense::click_and_drag())
            .on_hover_cursor(document_cursor_icon());
        self.editor_surface_rect = Some(response.rect);
        let surface_focused = response.has_focus();
        if surface_focused != self.surface_focused {
            self.caret_blink_focus_generation = self.caret_blink_focus_generation.wrapping_add(1);
            if surface_focused {
                self.caret_blink_reset_time = ui.ctx().input(|input| input.time.max(0.0));
            }
            self.request_repaint(ui.ctx());
        }
        self.surface_focused = surface_focused;
        if surface_focused {
            self.set_active_editor_surface(surface);
        } else {
            self.reconcile_body_active_group();
        }
        if let Some(pointer) = response.interact_pointer_pos() {
            let pointer_active = response.clicked()
                || response.double_clicked()
                || response.triple_clicked()
                || response.drag_started()
                || response.dragged()
                || (response.hovered() && ui.input(|input| input.pointer.primary_down()));
            if pointer_active {
                self.native_input_diagnostics.pointer_activations = self
                    .native_input_diagnostics
                    .pointer_activations
                    .saturating_add(1);
                self.set_active_editor_surface(surface);
                self.dispatch_command(
                    CommandInvocation::new(Command::EditorFocus, CommandOrigin::Pointer),
                    ui.ctx(),
                );
                if let Some(page_index) =
                    Self::pointer_surface_page(&layout, &page_rects, pointer, surface)
                {
                    if let Some(position) = self.caret_from_pointer(
                        &layout,
                        page_index,
                        page_rects[page_index],
                        scale,
                        pointer,
                        Some(&frame_projections),
                    ) {
                        let phase = if response.drag_started() {
                            PointerPhase::Press
                        } else if response.dragged() {
                            PointerPhase::Drag
                        } else if response.triple_clicked() {
                            PointerPhase::TripleClick
                        } else if response.double_clicked() {
                            PointerPhase::DoubleClick
                        } else {
                            PointerPhase::Release
                        };
                        if let Ok(position) = scriptcraft_commands::Position::from_core(position) {
                            self.dispatch_command(
                                CommandInvocation::new(
                                    Command::EditorSelectionPointer { position, phase, surface },
                                    CommandOrigin::Pointer,
                                ),
                                ui.ctx(),
                            );
                        }
                    }
                }
            }
        }
        self.native_input_diagnostics.focused = self.surface_focused;
        if self.surface_focused {
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

    fn dispatch_surface_events(
        &mut self,
        events: &[egui::Event],
        modifiers: egui::Modifiers,
        ctx: &egui::Context,
        surface: EditorSurface,
    ) {
        let started = Instant::now();
        let ime_event_in_frame = events.iter().any(|event| matches!(event, egui::Event::Ime(_)));
        self.performance.events = self.performance.events.saturating_add(events.len() as u64);
        for (index, event) in events.iter().enumerate() {
            self.record_native_event(match event {
                egui::Event::Text(_) => "text",
                egui::Event::Paste(_) => "paste",
                egui::Event::Ime(egui::ImeEvent::Preedit(_)) => "ime-preedit",
                egui::Event::Ime(egui::ImeEvent::Commit(_)) => "ime-commit",
                egui::Event::Ime(egui::ImeEvent::Enabled) => "ime-enabled",
                egui::Event::Ime(egui::ImeEvent::Disabled) => "ime-disabled",
                egui::Event::Zoom(_) => "zoom",
                egui::Event::Key { pressed: true, .. } => "key-pressed",
                egui::Event::Key { pressed: false, .. } => "key-released",
                _ => "other",
            });
            let mut command = None;
            let result = match event {
                egui::Event::Zoom(factor) => {
                    if let Ok(factor) = scriptcraft_commands::ZoomFactor::new(*factor) {
                        command = Some(CommandInvocation::new(
                            Command::ViewZoomPinch { factor },
                            CommandOrigin::InternalFollowUp,
                        ));
                    }
                    None
                }
                egui::Event::Text(text)
                    if !text.is_empty()
                        && !self.ime_active
                        && !events[index + 1..].iter().any(|next| {
                            matches!(next, egui::Event::Ime(egui::ImeEvent::Commit(commit)) if commit == text)
                        })
                        && !modifiers.command
                        && !(modifiers.ctrl && !cfg!(target_os = "macos"))
                        && text.chars().all(|character| !character.is_control()) =>
                {
                    command = Some(CommandInvocation::new(
                        Command::EditorTextInsert { text: text.clone(), surface },
                        CommandOrigin::Keyboard,
                    ));
                    None
                }
                egui::Event::Paste(text) if !text.is_empty() => {
                    command = Some(CommandInvocation::new(
                        Command::EditorTextPaste { text: text.clone(), surface },
                        CommandOrigin::Keyboard,
                    ));
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                    self.ime_active = true;
                    self.ime_preedit.clone_from(text);
                    self.rendered_revision = None;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    if !text.is_empty() {
                        command = Some(CommandInvocation::new(
                            Command::EditorTextInsert { text: text.clone(), surface },
                            CommandOrigin::ImeCommit,
                        ));
                    }
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Enabled) => {
                    self.ime_active = true;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Disabled) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    None
                }
                egui::Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if super::shortcuts::shortcut_input_allowed(
                        self.ime_active,
                        ime_event_in_frame,
                    ) {
                        if surface == EditorSurface::TitlePage
                            && self.menu_policy.in_window_shortcuts_owned()
                        {
                            command = super::menu::command_for_key_event_command(*key, *modifiers)
                                .map(|command| {
                                    CommandInvocation::new(command, CommandOrigin::Shortcut)
                                });
                        }
                        if command.is_none() {
                            command = super::shortcuts::dual_dialogue_toggle_for_focused_key_event(
                                *key,
                                *physical_key,
                                *modifiers,
                                ShortcutPlatform::current(),
                                self.menu_policy.in_window_shortcuts_owned(),
                            )
                            .filter(|_| surface == EditorSurface::Body)
                            .map(|command| {
                                CommandInvocation::new(command, CommandOrigin::Shortcut)
                            });
                        }
                        if command.is_none() {
                            command = editor_command_for_key_event(
                                *key,
                                *physical_key,
                                *modifiers,
                                ShortcutPlatform::current(),
                            )
                            .filter(|command| {
                                surface == EditorSurface::Body
                                    || !matches!(
                                        command,
                                        Command::EditorElementChange { .. }
                                            | Command::EditorParagraphTab
                                    )
                            })
                            .map(|command| {
                                let origin = match command.id() {
                                    CommandId::EditorParagraphEnter
                                    | CommandId::EditorParagraphTab
                                    | CommandId::EditorElementChange => CommandOrigin::Shortcut,
                                    _ => CommandOrigin::Keyboard,
                                };
                                CommandInvocation::new(command, origin)
                            });
                        }
                    }
                    if command.is_none() {
                        command = self
                            .navigation_key_event(*key, *modifiers, surface)
                            .map(|command| {
                            CommandInvocation::new(command, CommandOrigin::Keyboard)
                        });
                    }
                    None
                }
                _ => continue,
            };
            if let Some(command) = command {
                self.dispatch_command(command, ctx);
                continue;
            }
            if let Some(result) = result {
                self.mutate(
                    result,
                    RevealIntent::Conditional,
                );
            }
        }
        self.performance.event_nanos = self
            .performance
            .event_nanos
            .saturating_add(started.elapsed().as_nanos());
    }

    fn navigation_key_event(
        &self,
        key: Key,
        modifiers: egui::Modifiers,
        surface: EditorSurface,
    ) -> Option<Command> {
        match key {
            Key::ArrowLeft => Some(Command::EditorNavigationHorizontal {
                direction: HorizontalDirection::Left,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowRight => Some(Command::EditorNavigationHorizontal {
                direction: HorizontalDirection::Right,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowUp => Some(Command::EditorNavigationVertical {
                direction: VerticalDirection::Up,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowDown => Some(Command::EditorNavigationVertical {
                direction: VerticalDirection::Down,
                extend: modifiers.shift,
                surface,
            }),
            Key::Home => Some(Command::EditorNavigationLineEdge {
                edge: LineEdge::Start,
                extend: modifiers.shift,
                surface,
            }),
            Key::End => Some(Command::EditorNavigationLineEdge {
                edge: LineEdge::End,
                extend: modifiers.shift,
                surface: EditorSurface::Body,
            }),
            _ => None,
        }
    }

    pub(super) fn handle_surface_input(
        &mut self,
        ctx: &egui::Context,
        surface: EditorSurface,
    ) {
        if self.replacement_open_pending() {
            // Do not consume Body or Title Page semantic input while a selected
            // replacement is pending; the frame's events are intentionally not
            // queued for replay after a later safe resume.
            self.native_input_diagnostics.dispatch_enabled = false;
            return;
        }
        self.native_input_diagnostics.dispatch_enabled = self.surface_focused;
        if ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            // Explicit user scrolling wins over a not-yet-applied zoom request.
            // It must not be pulled back to the old viewport center afterward.
            if self.pending_zoom_anchor.is_some() {
                add_performance_counter(&mut self.performance.zoom_manual_scroll_cancellations, 1);
            }
            self.pending_zoom_anchor = None;
            self.reveal_intent = RevealIntent::None;
        }
        if !surface_input_is_dispatchable(
            self.surface_focused,
            self.active_editor_surface(),
            surface,
        ) {
            return;
        }
        self.native_input_diagnostics.dispatched_frames = self
            .native_input_diagnostics
            .dispatched_frames
            .saturating_add(1);
        let events = ctx.input(|input| input.events.clone());
        let modifiers = ctx.input(|input| input.modifiers);
        self.dispatch_surface_events(&events, modifiers, ctx, surface);
        if !events.is_empty() || ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            self.request_repaint(ctx);
        }
    }

    pub(super) fn navigation_selection_horizontal(
        &mut self,
        direction: HorizontalDirection,
        extend: bool,
    ) -> Selection {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        if selection.anchor != selection.focus && !extend {
            let position = if direction == HorizontalDirection::Right {
                ordered_selection(selection).1
            } else {
                ordered_selection(selection).0
            };
            return Selection::caret(position);
        }
        let position = selection.focus;
        let project = self.session.project();
        let origin_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            position.element,
        );
        let item = &project.elements[position.element];
        let next = if direction == HorizontalDirection::Right {
            if position.offset < item.text.len() {
                Position {
                    offset: item.text[position.offset..]
                        .chars()
                        .next()
                        .map_or(position.offset, |c| position.offset + c.len_utf8()),
                    ..position
                }
            } else if position.element + 1 < project.elements.len() {
                Position {
                    element: position.element + 1,
                    offset: 0,
                }
            } else {
                position
            }
        } else if position.offset > 0 {
            Position {
                offset: item.text[..position.offset]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(at, _)| at),
                ..position
            }
        } else if position.element > 0 {
            let previous = &project.elements[position.element - 1];
            Position {
                element: position.element - 1,
                offset: previous.text.len(),
            }
        } else {
            position
        };
        let target_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            next.element,
        );
        let next = if let Some(group) = target_group.filter(|group| {
            origin_group.map_or(true, |origin| origin.id != group.id)
        }) {
            dual_group_navigation_edge(
                project,
                group,
                direction == HorizontalDirection::Right,
            )
            .unwrap_or(next)
        } else {
            next
        };
        if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        }
    }

    pub(super) fn navigation_selection_vertical(
        &mut self,
        direction: VerticalDirection,
        extend: bool,
        ctx: &egui::Context,
    ) -> Result<Selection, EditError> {
        let selection = self.session.selection();
        let current = selection.focus;
        let layout = match self.layout_for_render(self.active_editor_surface()) {
            Ok(layout) => layout,
            Err(_) => {
                self.preferred_vertical_x = None;
                return Err(EditError::InvalidElement);
            }
        };
        let interaction_started = Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let current_index = layout.body_line_index_for_position_with_diagnostics(
            current.element,
            current.offset,
            &mut query_diagnostics,
        );
        self.performance.indexed_interaction_queries = self
            .performance
            .indexed_interaction_queries
            .saturating_add(1);
        self.performance.interaction_index_entries_examined = self
            .performance
            .interaction_index_entries_examined
            .saturating_add(query_diagnostics.index_entries_examined);
        add_performance_nanos(
            &mut self.performance.caret_lookup_nanos,
            interaction_started.elapsed().as_nanos(),
        );
        let Some(current_index) = current_index else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let down = direction == VerticalDirection::Down;
        let target_index = if down {
            current_index
                .checked_add(1)
                .filter(|index| *index < layout.body_line_count())
        } else {
            current_index.checked_sub(1)
        };
        let Some(current_line) = layout.body_line(current_index) else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let Some(target_index) = target_index else {
            self.preferred_vertical_x = None;
            let next = Position {
                element: current_line.element,
                offset: if down {
                    current_line.source.end
                } else {
                    current_line.source.start
                },
            };
            return Ok(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        };
        let Some(target_line) = layout.body_line(target_index) else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let project = self.session.project();
        let origin_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            current.element,
        );
        let target_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            target_line.element,
        );
        let mode_transition = match (origin_group, target_group) {
            (Some(origin), Some(target)) => origin.id != target.id,
            (None, Some(_)) | (Some(_), None) => true,
            (None, None) => false,
        };
        if let Some(group) = target_group.filter(|_| mode_transition) {
            self.preferred_vertical_x = None;
            let next = dual_group_navigation_edge(
                project,
                group,
                direction == VerticalDirection::Down,
            )
            .unwrap_or(Position {
                element: target_line.element,
                offset: target_line.source.start,
            });
            return Ok(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        }
        let scale = self.page_scale.unwrap_or(1.0);
        let current_projection = self
            .project_line_for_interaction(ctx, current_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let target_projection = self
            .project_line_for_interaction(ctx, target_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let existing_goal = self.preferred_vertical_x;
        let goal = existing_goal.unwrap_or_else(|| {
            super::presentation_geometry::preferred_vertical_document_x(
                current_line,
                current.offset,
                current_projection.as_ref(),
                scale,
            )
            .unwrap_or(current_line.geometry.x.raw())
        });
        self.preferred_vertical_x = Some(goal);
        let target_display = if let Some(projection) = target_projection.as_ref() {
            super::presentation_geometry::projected_target_boundary(projection, goal, scale)
                .ok_or(EditError::InvalidElement)?
        } else {
            let cell = target_line.char_width.raw().max(1);
            (goal
                .saturating_sub(target_line.geometry.x.raw())
                .saturating_add(cell / 2)
                / cell) as u32 as usize
        };
        let next = Position {
            element: target_line.element,
            offset: target_line.source_for_display_char(target_display),
        };
        Ok(if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    pub(super) fn navigation_selection_line_edge(
        &mut self,
        edge: LineEdge,
        extend: bool,
    ) -> Selection {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        let position = if edge == LineEdge::End {
            Position {
                element: selection.focus.element,
                offset: self.session.project().elements[selection.focus.element]
                    .text
                    .len(),
            }
        } else {
            Position {
                element: selection.focus.element,
                offset: 0,
            }
        };
        if extend {
            Selection {
                anchor: selection.anchor,
                focus: position,
            }
        } else {
            Selection::caret(position)
        }
    }
}

fn surface_input_is_dispatchable(
    surface_focused: bool,
    active_surface: EditorSurface,
    requested_surface: EditorSurface,
) -> bool {
    surface_focused && active_surface == requested_surface
}

#[cfg(test)]
mod focused_input_gate_tests {
    use super::{surface_input_is_dispatchable, EditorSurface};

    #[test]
    fn lost_focus_and_inactive_surface_do_not_dispatch_editor_input() {
        assert!(!surface_input_is_dispatchable(
            false,
            EditorSurface::Body,
            EditorSurface::Body,
        ));
        assert!(!surface_input_is_dispatchable(
            true,
            EditorSurface::TitlePage,
            EditorSurface::Body,
        ));
        assert!(surface_input_is_dispatchable(
            true,
            EditorSurface::Body,
            EditorSurface::Body,
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PresentationMetrics {
    page_width: f32,
    scale: f32,
    font_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresentationPlatform {
    MacOS,
    #[cfg(target_os = "windows")]
    Windows,
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Other,
}

fn logical_points_per_document_point(platform: PresentationPlatform) -> f32 {
    match platform {
        PresentationPlatform::MacOS => 1.0,
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        PresentationPlatform::Other => 1.0,
        #[cfg(target_os = "windows")]
        PresentationPlatform::Windows => 96.0 / 72.0,
    }
}

fn current_presentation_platform() -> PresentationPlatform {
    #[cfg(target_os = "macos")]
    {
        PresentationPlatform::MacOS
    }
    #[cfg(target_os = "windows")]
    {
        PresentationPlatform::Windows
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PresentationPlatform::Other
    }
}

fn presentation_metrics_at_zoom(
    available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
) -> PresentationMetrics {
    presentation_metrics_at_zoom_for_platform(
        available_width,
        paper_width,
        base_font_size,
        zoom_percent,
        current_presentation_platform(),
    )
}

fn presentation_metrics_at_zoom_for_platform(
    _available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
    platform: PresentationPlatform,
) -> PresentationMetrics {
    let normalized = status_bar::normalized_zoom_percent(zoom_percent as f64);
    let scale = logical_points_per_document_point(platform) * normalized as f32 / 100.0;
    let page_width = paper_width * scale;
    PresentationMetrics {
        page_width,
        scale,
        font_size: (base_font_size * scale).max(5.0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PageScrollGeometry {
    viewport_width: f32,
    content_width: f32,
    leading_padding: f32,
    horizontal_overflow: bool,
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
    page_rect: egui::Rect,
    geometry: FixedRect,
    scale: f32,
) -> (f32, f32, f32) {
    let start_x = page_rect.left() + geometry.x.to_points_f32() * scale;
    let end_x = start_x + geometry.width.to_points_f32() * scale;
    let y = page_rect.top() + geometry.y.to_points_f32() * scale;
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

fn page_scroll_geometry(viewport_width: f32, page_width: f32) -> PageScrollGeometry {
    let viewport_width = if viewport_width.is_finite() {
        viewport_width.max(0.0)
    } else {
        0.0
    };
    let page_width = if page_width.is_finite() {
        page_width.max(0.0)
    } else {
        0.0
    };
    let buffered_page_width = page_width + PAGE_HORIZONTAL_PADDING * 2.0;
    let content_width = viewport_width.max(buffered_page_width);
    PageScrollGeometry {
        viewport_width,
        content_width,
        leading_padding: ((content_width - page_width) / 2.0).max(0.0),
        horizontal_overflow: buffered_page_width > viewport_width,
    }
}

// Pure geometry inputs stay explicit so this helper remains directly testable.
#[allow(clippy::too_many_arguments)]
fn prepaint_zoom_scroll_offset(
    anchor: status_bar::ViewportCenterAnchor,
    page_count: usize,
    paper_width: f32,
    paper_height: f32,
    base_font_size: f32,
    zoom_percent: u16,
    outer_rect: egui::Rect,
    vertical_item_spacing: f32,
) -> Option<Vec2> {
    if anchor.page_index >= page_count || page_count == 0 {
        return None;
    }
    // egui's overlay scrollbars do not consume the ScrollArea inner viewport.
    // Keep planning dimensions identical to the rect that will paint this frame.
    let inner_size = outer_rect.size().max(Vec2::ZERO);
    let metrics =
        presentation_metrics_at_zoom(inner_size.x, paper_width, base_font_size, zoom_percent);
    let geometry = page_scroll_geometry(inner_size.x, metrics.page_width);
    let page_height = paper_height * metrics.scale;
    // `live_document_surface` allocates each page row and then a 12-point
    // spacer in egui's top-down layout. The layout also applies its vertical
    // item spacing between rows, which is part of both the page stride and
    // committed scroll extent; the first page starts after the top content margin.
    let page_stride = page_height + PAGE_VERTICAL_GAP + vertical_item_spacing;
    let content_size = Vec2::new(
        geometry.content_width,
        page_count as f32 * page_stride
            + SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING
            + SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING,
    );
    let page_top = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING + anchor.page_index as f32 * page_stride;
    let content_target = outer_rect.min
        + Vec2::new(geometry.leading_padding, page_top)
        + anchor.page_local_document_point * metrics.scale
        + anchor.outside_page_offset_points;
    let viewport_center = outer_rect.min + inner_size * 0.5;
    let requested_offset = content_target - viewport_center;
    let max_offset = (content_size - inner_size).max(Vec2::ZERO);
    (requested_offset.x.is_finite() && requested_offset.y.is_finite()).then(|| {
        Vec2::new(
            requested_offset.x.clamp(0.0, max_offset.x),
            requested_offset.y.clamp(0.0, max_offset.y),
        )
    })
}

struct FixedMetricTextStyle {
    color: Color32,
    selected_color: Color32,
    selection: Option<(usize, usize)>,
}

impl FixedMetricTextStyle {
    fn color_for(&self, selected: bool) -> Color32 {
        if selected {
            self.selected_color
        } else {
            self.color
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FixedCellGalleyKey {
    text: String,
    font_family: FontFamily,
    font_size_bits: u32,
    effective_style: EffectiveTextStyle,
    char_width_bits: u32,
    line_height_bits: u32,
    span_width_bits: u32,
    underline: bool,
    strikethrough: bool,
    color: Color32,
    pixels_per_point_bits: u32,
}

impl FixedCellGalleyKey {
    fn new(
        text: &str,
        font: &FontId,
        effective_style: &EffectiveTextStyle,
        char_width: f32,
        line_height: f32,
        span_width: f32,
        underline: bool,
        strikethrough: bool,
        color: Color32,
        pixels_per_point: f32,
    ) -> Self {
        Self {
            text: text.to_owned(),
            font_family: font.family.clone(),
            font_size_bits: font.size.to_bits(),
            effective_style: effective_style.clone(),
            char_width_bits: char_width.to_bits(),
            line_height_bits: line_height.to_bits(),
            span_width_bits: span_width.to_bits(),
            underline,
            strikethrough,
            color,
            pixels_per_point_bits: pixels_per_point.to_bits(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct FixedCellGalleyCache {
    galleys: HashMap<FixedCellGalleyKey, std::sync::Arc<Galley>>,
}

impl FixedCellGalleyCache {
    fn insert(&mut self, key: FixedCellGalleyKey, galley: std::sync::Arc<Galley>) {
        if self.galleys.len() >= FIXED_CELL_GALLEY_CACHE_LIMIT {
            // Keep retention bounded under repeated text edits and document replacement.
            // Clearing is diagnostic-neutral: the next use is simply a fresh build.
            self.galleys.clear();
        }
        self.galleys.insert(key, galley);
    }
}

// Text, selection, and paint instrumentation are independent view inputs.
#[allow(clippy::too_many_arguments)]
fn paint_fixed_metric_text_with_selection(
    painter: &egui::Painter,
    ctx: &egui::Context,
    origin: egui::Pos2,
    text: &str,
    font: FontId,
    char_width: f32,
    line_height: f32,
    style: FixedMetricTextStyle,
    effective_style: &EffectiveTextStyle,
    span_width: f32,
    underline: bool,
    strikethrough: bool,
    performance: &mut PerformanceDiagnostics,
) {
    if text.is_empty() {
        return;
    }

    let char_count = text.chars().count();
    let selection = style
        .selection
        .map(|(start, end)| (start.min(char_count), end.min(char_count)));
    let mut spans = Vec::with_capacity(3);
    let mut span_start_byte = 0;
    let mut span_start_column = 0;
    let mut span_selected = selection.is_some_and(|(start, end)| start == 0 && end > 0);
    for (column, (byte, _)) in text.char_indices().enumerate().skip(1) {
        let selected = selection.is_some_and(|(start, end)| column >= start && column < end);
        if selected != span_selected {
            spans.push((span_start_byte..byte, span_start_column, span_selected));
            span_start_byte = byte;
            span_start_column = column;
            span_selected = selected;
        }
    }
    spans.push((
        span_start_byte..text.len(),
        span_start_column,
        span_selected,
    ));

    for (byte_range, start_column, selected) in spans {
        let span_text = &text[byte_range];
        if span_text.is_empty() {
            continue;
        }
        let color = style.color_for(selected);
        let span_origin = egui::pos2(fixed_metric_x(origin.x, start_column, char_width), origin.y);
        let (galley, reused, fixed_geometry) = fixed_cell_span_galley(
            painter,
            ctx,
            span_text,
            &font,
            effective_style,
            char_width,
            line_height,
            span_width,
            underline,
            strikethrough,
            color,
        );
        if reused {
            performance.galleys_reused = performance.galleys_reused.saturating_add(1);
        } else {
            performance.galleys_built = performance.galleys_built.saturating_add(1);
        }
        if fixed_geometry {
            painter.galley(span_origin, galley, color);
        } else {
            // Keep output geometry exact if a later epaint version changes its
            // single-row glyph/mesh contract; this compatibility path is not used
            // by the pinned 0.29 desktop implementation.
            for (column, character) in span_text.chars().enumerate() {
                painter.text(
                    egui::pos2(
                        fixed_metric_x(span_origin.x, column, char_width),
                        span_origin.y,
                    ),
                    egui::Align2::LEFT_TOP,
                    character.to_string(),
                    font.clone(),
                    color,
                );
            }
        }
    }
}

fn fixed_cell_span_galley(
    painter: &egui::Painter,
    ctx: &egui::Context,
    text: &str,
    font: &FontId,
    effective_style: &EffectiveTextStyle,
    char_width: f32,
    line_height: f32,
    span_width: f32,
    underline: bool,
    strikethrough: bool,
    color: Color32,
) -> (std::sync::Arc<Galley>, bool, bool) {
    let pixels_per_point = ctx.pixels_per_point();
    let key = FixedCellGalleyKey::new(
        text,
        font,
        effective_style,
        char_width,
        line_height,
        span_width,
        underline,
        strikethrough,
        color,
        pixels_per_point,
    );
    let cached = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<FixedCellGalleyCache>(egui::Id::new(
            FIXED_CELL_GALLEY_CACHE_ID,
        ))
        .galleys
        .get(&key)
        .cloned()
    });
    if let Some(galley) = cached {
        return (galley, true, true);
    }

    let job = LayoutJob::simple_singleline(key.text.clone(), font.clone(), color);
    let mut galley = painter.layout_job(job).as_ref().clone();
    if !position_galley_in_fixed_cells(&mut galley, char_width) {
        // The epaint 0.29 single-row galley contract is one glyph record per
        // Unicode scalar and four mesh vertices per textured glyph. Preserve
        // correctness if a future backend violates that contract by leaving its
        // host layout untouched; the caller records the requested build.
        return (std::sync::Arc::new(galley), false, false);
    }
    let galley = std::sync::Arc::new(galley);
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<FixedCellGalleyCache>(egui::Id::new(
            FIXED_CELL_GALLEY_CACHE_ID,
        ))
        .insert(key, galley.clone());
    });
    (galley, false, true)
}

fn position_galley_in_fixed_cells(galley: &mut Galley, char_width: f32) -> bool {
    if galley.rows.len() != 1 {
        return false;
    }
    let char_count = galley.job.text.chars().count();
    let placed_row = &mut galley.rows[0];
    let row = std::sync::Arc::make_mut(&mut placed_row.row);
    if row.glyphs.len() != char_count {
        return false;
    }
    let expected_vertex_count = row
        .glyphs
        .iter()
        .filter(|glyph| !glyph.uv_rect.is_nothing())
        .count()
        .saturating_mul(4);
    if row.visuals.glyph_vertex_range.len() != expected_vertex_count {
        return false;
    }

    let pixels_per_point = galley.pixels_per_point;
    let mut vertex_index = row.visuals.glyph_vertex_range.start;
    let vertex_end = row.visuals.glyph_vertex_range.end;
    for (column, glyph) in row.glyphs.iter_mut().enumerate() {
        let fixed_x = column as f32 * char_width;
        if !glyph.uv_rect.is_nothing() {
            let Some(vertices) = row
                .visuals
                .mesh
                .vertices
                .get_mut(vertex_index..vertex_index.saturating_add(4))
            else {
                return false;
            };
            let round_to_pixel = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
            let left = fixed_x + round_to_pixel(glyph.uv_rect.offset.x);
            let right = left + glyph.uv_rect.size.x;
            vertices[0].pos.x = left;
            vertices[1].pos.x = right;
            vertices[2].pos.x = left;
            vertices[3].pos.x = right;
            vertex_index = vertex_index.saturating_add(4);
        }
        glyph.pos.x = fixed_x;
        glyph.advance_width = char_width;
    }
    if vertex_index != vertex_end {
        return false;
    }

    let fixed_width = char_count as f32 * char_width;
    placed_row.pos.x = 0.0;
    row.size.x = fixed_width;
    row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    galley.rect.min.x = 0.0;
    galley.rect.max.x = fixed_width;
    galley.mesh_bounds = galley.rows.iter().fold(egui::Rect::NOTHING, |bounds, row| {
        bounds.union(row.visuals.mesh_bounds)
    });
    true
}

fn renderer_font(font_family: &str, font_size: f32) -> FontId {
    FontId::new(
        font_size,
        if font_family == document_font::COURIER_PRIME_FAMILY {
            FontFamily::Name(document_font::COURIER_PRIME_FAMILY.into())
        } else {
            // Preserve the renderer's established appearance for explicit legacy
            // Courier/Courier New profiles; only the new Prime default selects
            // the embedded face. Viewport scale remains presentation-only.
            FontFamily::Monospace
        },
    )
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedSpanPaint {
    font: FontId,
    color: Color32,
    advance_mode: AdvanceMode,
}

fn resolve_span_paint(
    style: &EffectiveTextStyle,
    font_size: f32,
    palette: theme::PagePalette,
    availability: &document_font::FontAvailability,
) -> ResolvedSpanPaint {
    let resolved = super::presentation_style::resolve_effective_text_style(style);
    let face = document_font::resolve_document_face(
        resolved.family,
        style.bold,
        style.italic,
        availability,
    );
    let color = resolved.color_rgb8.map_or_else(
        || {
            if style.hidden_text {
                palette.subdued_text
            } else {
                palette.text
            }
        },
        |[red, green, blue]| Color32::from_rgb(red, green, blue),
    );
    ResolvedSpanPaint {
        font: FontId::new(font_size.max(1.0), face.family),
        color,
        advance_mode: face.advance_mode,
    }
}

fn fixed_metric_x(origin: f32, column: usize, char_width: f32) -> f32 {
    origin + column as f32 * char_width
}

fn caret_blink_visible(
    ctx: &egui::Context,
    blink_id: egui::Id,
    focused: bool,
    collapsed: bool,
    reset_time: f64,
) -> bool {
    if !focused || !collapsed {
        return false;
    }

    // egui owns both the clock and animation entry. A zero-duration animation
    // makes each 500 ms phase switch discrete rather than introducing a fade.
    // The caller's identity includes revision and caret position, so changes
    // begin a distinct transient entry without model or thread state.
    let time = ctx.input(|input| input.time.max(0.0));
    let elapsed = (time - reset_time).max(0.0);
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (elapsed / interval).floor() as u64;
    let target_visible = phase.is_multiple_of(2);
    let phase_value = ctx.animate_bool_with_time(blink_id, target_visible, 0.0);
    ctx.request_repaint_after(caret_blink_repaint_delay(elapsed));
    phase_value > 0.5
}

fn caret_blink_repaint_delay(time: f64) -> Duration {
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (time.max(0.0) / interval).floor() as u64;
    Duration::from_secs_f64(((phase + 1) as f64 * interval - time.max(0.0)).max(0.001))
}

fn caret_needs_reveal(caret_rect: egui::Rect, visible_clip: egui::Rect) -> bool {
    !visible_clip.contains_rect(caret_rect)
}

fn ordered_selection(selection: Selection) -> (Position, Position) {
    if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}

pub(super) fn chrome_clear_color(palette: theme::ScriptWorkspacePalette) -> [f32; 4] {
    palette.background.to_normalized_gamma_f32()
}

pub(super) fn central_panel_frame(
    _ctx: &egui::Context,
    palette: theme::ScriptWorkspacePalette,
) -> egui::Frame {
    // Keep the Script Workspace surface color without a fixed viewport inset.
    egui::Frame::NONE.fill(palette.background)
}

fn document_cursor_icon() -> egui::CursorIcon {
    egui::CursorIcon::Text
}

#[cfg(test)]
mod title_surface_input_tests {
    use super::{EditorSurface, ScriptCraftApp};
    use eframe::egui;
    use scriptcraft_core::editing::{Position, Selection};

    #[test]
    fn ime_commit_in_child_targets_title_selection_and_child_ime_state() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = egui::Context::default();
        let title_index = 17;
        let original_title = app.session.project().title_page[title_index].text.clone();
        let title_caret = Position {
            element: title_index,
            offset: original_title.len(),
        };
        assert!(app
            .session
            .set_title_selection(Selection::caret(title_caret))
            .is_ok());
        let body_selection = app.session.selection();
        app.set_active_editor_surface(EditorSurface::TitlePage);
        app.swap_title_view_state();

        let events = [
            egui::Event::Ime(egui::ImeEvent::Preedit("draft".to_owned())),
            egui::Event::Ime(egui::ImeEvent::Commit("X".to_owned())),
        ];
        app.dispatch_surface_events(
            &events,
            egui::Modifiers::NONE,
            &context,
            EditorSurface::TitlePage,
        );

        assert_eq!(
            app.session.project().title_page[title_index].text,
            format!("{original_title}X"),
        );
        assert_eq!(app.session.selection(), body_selection);
        assert_eq!(app.session.title_selection().focus.offset, title_caret.offset + 1);
        assert!(!app.ime_active);
        assert!(app.ime_preedit.is_empty());
        app.swap_title_view_state();
    }

    #[test]
    fn body_paragraph_tab_shortcut_is_ignored_in_title_surface() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = egui::Context::default();
        let project_before = app.session.project().clone();
        let title_selection_before = app.session.title_selection();
        app.set_active_editor_surface(EditorSurface::TitlePage);
        app.swap_title_view_state();

        let events = [egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: Some(egui::Key::Tab),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }];
        app.dispatch_surface_events(
            &events,
            egui::Modifiers::NONE,
            &context,
            EditorSurface::TitlePage,
        );

        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.title_selection(), title_selection_before);
        app.swap_title_view_state();
    }
}
