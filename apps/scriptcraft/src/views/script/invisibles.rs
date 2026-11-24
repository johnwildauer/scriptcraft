//! Noninteractive editor-only marks painted over canonical line projections.

use super::zoom_scroll::PresentationMetrics;
use crate::presentation::span_paint::neutral_resolved_font;
use crate::{
    presentation::{document_font::PreparedDocumentFonts, geometry},
};
use eframe::egui::{self, Color32};

const SPACE_DOT_RADIUS_SCALE: f32 = 0.8;
const MIN_SPACE_DOT_RADIUS: f32 = 0.55;
const MAX_SPACE_DOT_RADIUS: f32 = 1.4;
const PILCROW_X_OFFSET_SCALE: f32 = 0.75;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OverlayPaintError {
    MissingNeutralFont,
    InvalidVerticalGeometry,
    UnsupportedGlyphTopology,
}

/// Paints view-only marks from canonical paragraph text and projected line
/// boundaries. The return value indicates whether a pilcrow galley was painted.
pub(super) fn paint_overlay(
    painter: &egui::Painter,
    page_left: f32,
    line_top_screen: f32,
    line: &geometry::PresentationLine<'_>,
    canonical_text: Option<&str>,
    scale: f32,
    metrics: PresentationMetrics,
    color: Color32,
    prepared_fonts: &PreparedDocumentFonts,
) -> Result<bool, OverlayPaintError> {
    let radius = (SPACE_DOT_RADIUS_SCALE * scale)
        .clamp(MIN_SPACE_DOT_RADIUS, MAX_SPACE_DOT_RADIUS);
    for projected in &line.spans {
        let span = projected.core;
        for (display_index, scalar) in span.display_text.chars().enumerate() {
            if scalar != ' ' {
                continue;
            }
            let Some((&leading, &trailing)) = projected
                .display_boundary_offsets
                .get(display_index)
                .zip(projected.display_boundary_offsets.get(display_index + 1))
            else {
                continue;
            };
            let x = page_left + projected.origin_x + leading + (trailing - leading) * 0.5;
            let y = line_top_screen + line.line_box_height * 0.5;
            if x.is_finite() && y.is_finite() {
                painter.circle_filled(egui::pos2(x, y), radius, color);
            }
        }
    }

    let Some(canonical_text) = canonical_text else {
        return Ok(false);
    };
    let source_end = line.core.source.end;
    if source_end > canonical_text.len() || !canonical_text.is_char_boundary(source_end) {
        return Ok(false);
    }
    let paragraph_end = source_end == canonical_text.len();
    let next_scalar = canonical_text
        .get(source_end..)
        .and_then(|remaining| remaining.chars().next());
    let previous_scalar = canonical_text
        .get(..source_end)
        .and_then(|preceding| preceding.chars().next_back());
    let imported_break = match next_scalar {
        Some('\r') => true,
        Some('\n') => previous_scalar != Some('\r'),
        _ => false,
    };
    if !paragraph_end && !imported_break {
        return Ok(false);
    }

    let font = neutral_resolved_font(metrics.font_size * scale, prepared_fonts)
        .ok_or(OverlayPaintError::MissingNeutralFont)?;
    let baseline_offset = line.baseline_y - line.line_box_top;
    let pilcrow = geometry::ime_singleline_galley(
        painter,
        "¶",
        &font,
        color,
        line.line_box_height,
        baseline_offset,
    )
    .map_err(|error| match error {
        geometry::CanonicalGalleyError::InvalidVerticalGeometry => {
            OverlayPaintError::InvalidVerticalGeometry
        }
        _ => OverlayPaintError::UnsupportedGlyphTopology,
    })?;
    let x_offset = (PILCROW_X_OFFSET_SCALE * scale).clamp(0.5, 1.25);
    painter.galley(
        egui::pos2(
            page_left + line.line_origin_x + line.total_advance + x_offset,
            line_top_screen,
        ),
        pilcrow,
        color,
    );
    Ok(true)
}
