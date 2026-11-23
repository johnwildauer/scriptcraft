//! Script View page, selection, decoration, caret, and IME painting.

use super::{
    caret::{caret_blink_id, caret_blink_visible, viewport_caret_rect, CaretPresentation},
    page_policy::{
        document_y_to_screen, normal_view_page_bottom_rule_geometry,
        normal_view_page_bottom_rule_visible, page_chrome_visible, page_local_screen_origin,
        NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH,
    },
    invisibles::{paint_overlay as paint_invisibles_overlay, OverlayPaintError},
    text::{neutral_resolved_font, resolve_span_paint},
    zoom_scroll::PresentationMetrics,
    ScriptCraftApp,
};
use crate::{
    app::add_performance_counter,
    chrome::{status_bar::PresentedPage, theme},
    presentation::geometry as presentation_geometry,
};
use eframe::egui::{self, Color32, Stroke, Vec2};
use scriptcraft_commands::{Command, EditorSurface, ScriptViewMode};
use scriptcraft_layout::{Caret, FixedRect, LayoutLine, PositionedStyledSpan, ScreenplayLayout};
use scriptcraft_session::editing::{Position, Selection};
use std::time::Instant;

const PAGE_BORDER_WIDTH: f32 = 2.5;

impl ScriptCraftApp {
    fn post_typography_diagnostic_once(&mut self, prefix: &str, message: &str) {
        let already_visible = self
            .diagnostic_message
            .as_ref()
            .is_some_and(|current| current.as_str().starts_with(prefix));
        let already_queued = self.pending_diagnostic_commands.iter().any(|command| {
            matches!(command, Command::UiStatusSet { message: Some(message) }
                if message.as_str().starts_with(prefix))
        });
        if !already_visible && !already_queued {
            self.post_diagnostic(message);
        }
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
                selected_start = Some(
                    selected_start.map_or(segment.display_start, |current: usize| {
                        current.min(segment.display_start)
                    }),
                );
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
        let show_invisibles = layout.purpose == scriptcraft_layout::LayoutPurpose::Editor
            && self.invisibles_visible_for(surface);
        let invisibles_color = show_invisibles.then(|| self.invisibles_color());
        if page_chrome_visible(presentation_mode) {
            painter.rect_filled(page_rect, 2.0, palette.paper);
            painter.rect_stroke(
                page_rect,
                2.0,
                Stroke::new(PAGE_BORDER_WIDTH, palette.border),
                egui::StrokeKind::Middle,
            );
        }
        if normal_view_page_bottom_rule_visible(presentation_mode, page_index, layout.pages.len()) {
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
            let selection =
                self.selection_range(line.source.start, line.source.end, line.element, surface);
            let Some(projection) = projections.get(line_index).and_then(Option::as_ref) else {
                match presentation_geometry::project_line(line, scale) {
                    Err(presentation_geometry::PresentationProjectionError::InvalidVerticalGeometry) => {
                        self.post_typography_diagnostic_once(
                            "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID:",
                            "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID: canonical vertical projection unavailable",
                        );
                    }
                    Err(presentation_geometry::PresentationProjectionError::InvalidDecorationGeometry) => {
                        self.post_typography_diagnostic_once(
                            "TYPOGRAPHY_DECORATION_GEOMETRY_INVALID:",
                            "TYPOGRAPHY_DECORATION_GEOMETRY_INVALID: canonical decoration projection unavailable",
                        );
                    }
                    _ => self.post_typography_diagnostic_once(
                        "TYPOGRAPHY_PROJECTION_UNAVAILABLE:",
                        "TYPOGRAPHY_PROJECTION_UNAVAILABLE: canonical line projection failed",
                    ),
                }
                continue 'paint_lines;
            };
            debug_assert!(std::ptr::eq(projection.core, line));
            if projection.natural_extent_exceeds_line_box {
                add_performance_counter(
                    &mut self.performance.natural_extent_exceeds_line_box,
                    1,
                );
            }
            let page_origin = page_local_screen_origin(presented_page, scale);
            let metric_cull_bounds = projection
                .metric_cull_bounds
                .translate(page_origin.to_vec2());
            if !metric_cull_bounds.intersects(painter.clip_rect()) {
                add_performance_counter(&mut self.performance.metric_culled_lines, 1);
                continue 'paint_lines;
            }
            let glyph_clip = egui::Rect::from_min_max(
                egui::pos2(
                    page_origin.x + projection.horizontal_clip_x[0],
                    page_rect.top(),
                ),
                egui::pos2(
                    page_origin.x + projection.horizontal_clip_x[1],
                    page_rect.bottom(),
                ),
            )
            .intersect(painter.clip_rect());
            let line_painter = painter.with_clip_rect(glyph_clip);
            let line_top_screen = page_origin.y + projection.line_box_top;
            let line_box_height = projection.line_box_height;
            let projected_decorations = projection
                .spans
                .iter()
                .map(|projected| {
                    let span = projected.core;
                    if span.underline != span.underline_geometry.is_some()
                        || span.strikethrough != span.strikethrough_geometry.is_some()
                    {
                        return Err(
                            presentation_geometry::PresentationProjectionError::InvalidDecorationGeometry,
                        );
                    }
                    let underline = span
                        .underline_geometry
                        .map(|geometry| {
                            presentation_geometry::project_decoration_rect(
                                projected,
                                geometry,
                                projection.baseline_y,
                                scale,
                            )
                        })
                        .transpose()?;
                    let strikethrough = span
                        .strikethrough_geometry
                        .map(|geometry| {
                            presentation_geometry::project_decoration_rect(
                                projected,
                                geometry,
                                projection.baseline_y,
                                scale,
                            )
                        })
                        .transpose()?;
                    Ok((underline, strikethrough))
                })
                .collect::<Result<Vec<_>, _>>();
            let Ok(projected_decorations) = projected_decorations else {
                self.post_typography_diagnostic_once(
                    "TYPOGRAPHY_DECORATION_GEOMETRY_INVALID:",
                    "TYPOGRAPHY_DECORATION_GEOMETRY_INVALID: canonical decoration projection unavailable",
                );
                continue 'paint_lines;
            };
            let Some(prepared_fonts) = self.prepared_document_fonts.clone() else {
                self.post_typography_diagnostic_once(
                    "TYPOGRAPHY_FACE_UNAVAILABLE:",
                    "TYPOGRAPHY_FACE_UNAVAILABLE: prepared font environment missing",
                );
                continue 'paint_lines;
            };
            let resolved_paints = line
                .spans
                .iter()
                .map(|span| {
                    resolve_span_paint(
                        &span.resolved_text_style,
                        span.resolved_text_style.size.to_points_f32() * scale,
                        palette,
                        &prepared_fonts,
                    )
                })
                .collect::<Option<Vec<_>>>();
            let Some(resolved_paints) = resolved_paints else {
                self.post_typography_diagnostic_once(
                    "TYPOGRAPHY_FACE_UNAVAILABLE:",
                    "TYPOGRAPHY_FACE_UNAVAILABLE: prepared resolved face missing",
                );
                continue 'paint_lines;
            };
            if resolved_paints
                .iter()
                .any(|resolved| resolved.unsupported_color)
            {
                self.post_typography_diagnostic_once(
                    "TYPOGRAPHY_COLOR_FALLBACK:",
                    "TYPOGRAPHY_COLOR_FALLBACK: unsupported explicit color uses the theme text palette",
                );
            }
            for ((projected, resolved), (underline_rect, strikethrough_rect)) in projection
                .spans
                .iter()
                .zip(&resolved_paints)
                .zip(&projected_decorations)
            {
                let span = projected.core;
                let span_origin =
                    egui::pos2(page_rect.left() + projected.origin_x, line_top_screen);
                let selected_display = Self::selected_display_range(span, selection);
                if let Some((from, to)) = selected_display {
                    add_performance_counter(&mut self.performance.selection_spans_submitted, 1);
                    let boundaries = &projected.display_boundary_offsets;
                    let start = from.min(boundaries.len().saturating_sub(1));
                    let end = to.min(boundaries.len().saturating_sub(1));
                    let selected_rect = egui::Rect::from_min_max(
                        egui::pos2(span_origin.x + boundaries[start], line_top_screen),
                        egui::pos2(span_origin.x + boundaries[end], line_top_screen + line_box_height),
                    );
                    if selected_rect.width() > 0.0 {
                        line_painter.rect_filled(selected_rect, 0.0, palette.selection);
                        line_painter.rect_stroke(
                            selected_rect,
                            0.0,
                            Stroke::new(1.0, palette.selection_boundary),
                            egui::StrokeKind::Inside,
                        );
                    }
                }

                let galley = match presentation_geometry::canonical_singleline_galley(
                    &line_painter,
                    &span.display_text,
                    &resolved.font,
                    &projected.display_boundary_offsets,
                    resolved.color,
                    line_box_height,
                    projection.baseline_y - projection.line_box_top,
                ) {
                    Ok(galley) => galley,
                    Err(presentation_geometry::CanonicalGalleyError::InvalidVerticalGeometry) => {
                        self.post_typography_diagnostic_once(
                            "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID:",
                            "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID: canonical galley vertical geometry was not painted",
                        );
                        continue;
                    }
                    Err(_) => {
                        self.post_typography_diagnostic_once(
                            "TYPOGRAPHY_GLYPH_TOPOLOGY_UNSUPPORTED:",
                            "TYPOGRAPHY_GLYPH_TOPOLOGY_UNSUPPORTED: canonical span topology was not painted",
                        );
                        continue;
                    }
                };
                self.performance.galleys_built = self.performance.galleys_built.saturating_add(1);
                line_painter.galley(span_origin, galley.clone(), resolved.color);
                if let Some((from, to)) = selected_display {
                    let boundaries = &projected.display_boundary_offsets;
                    let start = from.min(boundaries.len().saturating_sub(1));
                    let end = to.min(boundaries.len().saturating_sub(1));
                    let selected_rect = egui::Rect::from_min_max(
                        egui::pos2(span_origin.x + boundaries[start], line_top_screen),
                        egui::pos2(span_origin.x + boundaries[end], line_top_screen + line_box_height),
                    );
                    let selected_x_clip = egui::Rect::from_min_max(
                        egui::pos2(selected_rect.left(), glyph_clip.top()),
                        egui::pos2(selected_rect.right(), glyph_clip.bottom()),
                    );
                    let selected_clip = glyph_clip.intersect(selected_x_clip);
                    if selected_rect.width() > 0.0
                        && selected_clip.width() > 0.0
                        && selected_clip.height() > 0.0
                    {
                        let selected_galley =
                            presentation_geometry::recolor_galley(&galley, palette.selection_text);
                        painter.with_clip_rect(selected_clip).galley(
                            span_origin,
                            selected_galley,
                            palette.selection_text,
                        );
                    }
                }

                for rect in [*underline_rect, *strikethrough_rect].into_iter().flatten() {
                    if rect.width() > 0.0 && rect.height() > 0.0 {
                        line_painter.rect_filled(
                            rect.translate(page_origin.to_vec2()),
                            0.0,
                            resolved.color,
                        );
                    }
                }
            }

            if let Some(color) = invisibles_color {
                let paragraph_text = match surface {
                    EditorSurface::Body => self
                        .session
                        .project()
                        .elements
                        .get(line.element)
                        .map(|element| element.text.as_str()),
                    EditorSurface::TitlePage => self
                        .session
                        .project()
                        .title_page
                        .get(line.element)
                        .map(|paragraph| paragraph.text.as_str()),
                };
                match paint_invisibles_overlay(
                    &line_painter,
                    page_rect.left(),
                    line_top_screen,
                    projection,
                    paragraph_text,
                    scale,
                    metrics,
                    color,
                    &prepared_fonts,
                ) {
                    Ok(true) => {
                        self.performance.galleys_built =
                            self.performance.galleys_built.saturating_add(1);
                    }
                    Ok(false) => {}
                    Err(OverlayPaintError::MissingNeutralFont) => self
                        .post_typography_diagnostic_once(
                            "TYPOGRAPHY_FACE_UNAVAILABLE:",
                            "TYPOGRAPHY_FACE_UNAVAILABLE: prepared neutral face missing for invisibles",
                        ),
                    Err(OverlayPaintError::InvalidVerticalGeometry) => self
                        .post_typography_diagnostic_once(
                            "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID:",
                            "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID: invisibles baseline projection unavailable",
                        ),
                    Err(OverlayPaintError::UnsupportedGlyphTopology) => self
                        .post_typography_diagnostic_once(
                            "TYPOGRAPHY_GLYPH_TOPOLOGY_UNSUPPORTED:",
                            "TYPOGRAPHY_GLYPH_TOPOLOGY_UNSUPPORTED: invisibles glyph was not painted",
                        ),
                }
            }
        }

        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        if caret_page_index == Some(page_index) && selection.anchor == selection.focus {
            if let Some(caret_geometry) = caret_geometry {
                let viewport_geometry =
                    viewport_caret_rect(caret_geometry, presented_page, scale);
                let caret_rect = viewport_geometry.visual_rect;
                let canonical_line_rect = viewport_geometry.canonical_line_rect;
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
                    if let Some(clip) = viewport_geometry.clip {
                        painter
                            .with_clip_rect(clip)
                            .rect_filled(caret_rect, 0.0, palette.caret);
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
                    let ime_baseline_offset = caret_line
                        .map(|line| {
                            line.baseline_y().to_points_f32() * scale
                                - line.geometry.y.to_points_f32() * scale
                        })
                        .filter(|offset| offset.is_finite());
                    let caret_span =
                        caret_line.and_then(|line| Self::span_for_caret(line, caret.offset));
                    let prepared = self.prepared_document_fonts.as_ref();
                    let ime_font = caret_span
                        .and_then(|span| {
                            let style = &span.resolved_text_style;
                            prepared.and_then(|prepared| {
                                resolve_span_paint(
                                    style,
                                    style.size.to_points_f32() * scale,
                                    palette,
                                    prepared,
                                )
                            })
                        })
                        .map(|paint| paint.font)
                        .or_else(|| {
                            prepared.and_then(|prepared| {
                                neutral_resolved_font(metrics.font_size * scale, prepared)
                            })
                        });
                    if let Some(ime_font) = ime_font {
                        if let Some(ime_baseline_offset) = ime_baseline_offset {
                            let ime_painter = viewport_geometry.clip.map_or_else(
                                || painter.clone(),
                                |clip| painter.with_clip_rect(clip),
                            );
                            match presentation_geometry::ime_singleline_galley(
                                &ime_painter,
                                &self.ime_preedit,
                                &ime_font,
                                palette.ime_text,
                                canonical_line_rect.height(),
                                ime_baseline_offset,
                            ) {
                                Ok(galley) => {
                                    self.performance.galleys_built =
                                        self.performance.galleys_built.saturating_add(1);
                                    ime_painter.galley(
                                        canonical_line_rect.min,
                                        galley,
                                        palette.ime_text,
                                    );
                                }
                                Err(presentation_geometry::CanonicalGalleyError::InvalidVerticalGeometry) => {
                                    self.post_typography_diagnostic_once(
                                        "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID:",
                                        "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID: IME galley vertical geometry was not painted",
                                    );
                                }
                                Err(_) => self.post_typography_diagnostic_once(
                                    "TYPOGRAPHY_IME_TOPOLOGY_UNSUPPORTED:",
                                    "TYPOGRAPHY_IME_TOPOLOGY_UNSUPPORTED: preedit topology was not painted",
                                ),
                            }
                        } else {
                            self.post_typography_diagnostic_once(
                                "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID:",
                                "TYPOGRAPHY_VERTICAL_GEOMETRY_INVALID: IME baseline geometry was unavailable",
                            );
                        }
                    } else {
                        self.post_typography_diagnostic_once(
                            "TYPOGRAPHY_IME_FACE_UNAVAILABLE:",
                            "TYPOGRAPHY_IME_FACE_UNAVAILABLE: prepared resolved face missing",
                        );
                    }
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
        painter.line_segment([start, end], Stroke::new(physical_pixel, color));
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
