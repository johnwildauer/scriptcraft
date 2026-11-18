//! Script View span styling, galley construction, and interaction projection.

use super::ScriptCraftApp;
use crate::{
    app::PerformanceDiagnostics,
    chrome::theme,
    presentation::{document_font, geometry as presentation_geometry, style},
};
use crate::presentation::style::AdvanceMode;
use eframe::egui::{self, Color32, FontFamily, FontId};
use egui::epaint::text::{Galley, LayoutJob};
use scriptcraft_layout::LayoutLine;
use scriptcraft_model::formatting::EffectiveTextStyle;

impl ScriptCraftApp {
    pub(crate) fn project_line_for_interaction<'a>(
        &mut self,
        ctx: &egui::Context,
        line: &'a LayoutLine,
        scale: f32,
    ) -> Result<Option<presentation_geometry::PresentationLine<'a>>, ()> {
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
                let (geometry, reused) = presentation_geometry::natural_singleline_geometry(
                    &painter,
                    &span.display_text,
                    &paint.font,
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
        presentation_geometry::project_line(line, &modes, &natural, scale)
            .map(Some)
            .map_err(|_| ())
    }
}

pub(super) struct FixedMetricTextStyle {
    pub(super) color: Color32,
    pub(super) selected_color: Color32,
    pub(super) selection: Option<(usize, usize)>,
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

// Text, selection, and paint instrumentation are independent view inputs.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_fixed_metric_text_with_selection(
    painter: &egui::Painter,
    origin: egui::Pos2,
    text: &str,
    font: FontId,
    char_width: f32,
    style: FixedMetricTextStyle,
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
            span_text,
            &font,
            char_width,
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
    text: &str,
    font: &FontId,
    char_width: f32,
    color: Color32,
) -> (std::sync::Arc<Galley>, bool, bool) {
    // Keep ScriptCraft's fixed-cell transformation local to this egui pass.
    // Painter::layout_job can use egui's own atlas-aware layout cache, while no
    // UV-bearing transformed galley is retained by the application.
    let job = LayoutJob::simple_singleline(text.to_owned(), font.clone(), color);
    let mut galley = painter.layout_job(job).as_ref().clone();
    if !position_galley_in_fixed_cells(&mut galley, char_width) {
        // The epaint 0.29 single-row galley contract is one glyph record per
        // Unicode scalar and four mesh vertices per textured glyph. Preserve
        // correctness if a future backend violates that contract by leaving its
        // host layout untouched; the caller records the requested build.
        return (std::sync::Arc::new(galley), false, false);
    }
    let galley = std::sync::Arc::new(galley);
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

pub(super) fn renderer_font(font_family: &str, font_size: f32) -> FontId {
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
pub(super) struct ResolvedSpanPaint {
    pub(super) font: FontId,
    pub(super) color: Color32,
    pub(super) advance_mode: AdvanceMode,
}

pub(super) fn resolve_span_paint(
    style: &EffectiveTextStyle,
    font_size: f32,
    palette: theme::PagePalette,
    availability: &document_font::FontAvailability,
) -> ResolvedSpanPaint {
    let resolved = style::resolve_effective_text_style(style);
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
