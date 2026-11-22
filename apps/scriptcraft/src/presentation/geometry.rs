//! Desktop projection of canonical layout geometry and display boundaries.

use eframe::egui::{self, Color32, FontId};
use egui::epaint::text::{Galley, LayoutJob};
use scriptcraft_layout::{Caret, LayoutLine, PositionedStyledSpan};
use scriptcraft_model::formatting::FixedPoint;
use std::sync::Arc;

/// One ephemeral span in the canonical desktop line projection. `origin_x`,
/// `advance`, and `display_boundary_offsets` are page-local screen points.
#[derive(Clone, Debug)]
pub(crate) struct PresentationSpan<'a> {
    pub core: &'a PositionedStyledSpan,
    pub origin_x: f32,
    pub advance: f32,
    pub display_boundary_offsets: Arc<[f32]>,
}

/// A presentation-only projection of the canonical line and its source maps.
#[derive(Clone, Debug)]
pub(crate) struct PresentationLine<'a> {
    pub core: &'a LayoutLine,
    pub line_origin_x: f32,
    pub total_advance: f32,
    /// Page-local screen-space line-box interval; the bottom is the baseline.
    pub line_box_top: f32,
    pub line_box_height: f32,
    pub baseline_y: f32,
    /// Paragraph/column x clip expanded to the canonical indented line left,
    /// deliberately independent of line-box y.
    pub horizontal_clip_x: [f32; 2],
    /// Paint-only broad phase; never used as a clip or interaction bound.
    pub metric_cull_bounds: egui::Rect,
    /// Natural font-table metrics exceed the interaction line box.
    pub natural_extent_exceeds_line_box: bool,
    pub spans: Vec<PresentationSpan<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PresentationProjectionError {
    InvalidScale,
    DisplayTopologyMismatch,
    InvalidCanonicalBoundaries,
    InvalidVerticalGeometry,
}

/// Projects one line without querying egui font advances. Every horizontal
/// coordinate is a direct scaling of canonical layout geometry.
pub(crate) fn project_line<'a>(
    line: &'a LayoutLine,
    scale: f32,
) -> Result<PresentationLine<'a>, PresentationProjectionError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(PresentationProjectionError::InvalidScale);
    }
    let line_chars = line.display_text.chars().collect::<Vec<_>>();
    let mut display_base = 0usize;
    for span in line.spans.iter() {
        let count = span.display_text.chars().count();
        if line_chars.get(display_base..display_base.saturating_add(count))
            != Some(span.display_text.chars().collect::<Vec<_>>().as_slice())
        {
            return Err(PresentationProjectionError::DisplayTopologyMismatch);
        }
        let offsets = span
            .display_boundary_offsets
            .iter()
            .map(|offset| offset.to_points_f32() * scale)
            .collect::<Vec<_>>();
        let width = span.geometry.width.to_points_f32() * scale;
        if offsets.len() != count + 1
            || offsets.first().copied() != Some(0.0)
            || offsets.last().copied() != Some(width)
            || !offsets
                .windows(2)
                .all(|pair| pair[0].is_finite() && pair[0] <= pair[1])
        {
            return Err(PresentationProjectionError::InvalidCanonicalBoundaries);
        }
        display_base = display_base.saturating_add(count);
    }
    if display_base != line_chars.len() {
        return Err(PresentationProjectionError::DisplayTopologyMismatch);
    }

    let total_advance = line.geometry.width.to_points_f32() * scale;
    let line_origin_x = line.geometry.x.to_points_f32() * scale;
    let paragraph_left = line.paragraph_bounds.x.to_points_f32() * scale;
    let indented_line_left = line.unaligned_line_left.to_points_f32() * scale;
    let paragraph_right = FixedPoint::from_raw(
        line.paragraph_bounds
            .x
            .raw()
            .saturating_add(line.paragraph_bounds.width.raw()),
    )
    .to_points_f32()
        * scale;
    let clip_left = paragraph_left.min(indented_line_left);
    let line_box_top = line.geometry.y.to_points_f32() * scale;
    let line_box_height = line.geometry.height.to_points_f32() * scale;
    let baseline_y = line.baseline_y().to_points_f32() * scale;
    let canonical_bottom_raw = line
        .geometry
        .y
        .raw()
        .checked_add(line.geometry.height.raw());
    let natural_ascent = line.vertical_metrics.natural_ascent.to_points_f32() * scale;
    let natural_descent = line.vertical_metrics.natural_descent.to_points_f32() * scale;
    let natural_top = baseline_y - natural_ascent;
    let natural_bottom = baseline_y - natural_descent;
    if !line_box_top.is_finite()
        || !line_box_height.is_finite()
        || line_box_height <= 0.0
        || !baseline_y.is_finite()
        || baseline_y <= line_box_top
        || canonical_bottom_raw != Some(line.baseline_y().raw())
        || !natural_ascent.is_finite()
        || natural_ascent < 0.0
        || !natural_descent.is_finite()
        || natural_descent > 0.0
        || !natural_top.is_finite()
        || !natural_bottom.is_finite()
        || natural_top > natural_bottom
        || !paragraph_left.is_finite()
        || !paragraph_right.is_finite()
        || paragraph_left > paragraph_right
        || !indented_line_left.is_finite()
        || !clip_left.is_finite()
        || clip_left > paragraph_right
    {
        return Err(PresentationProjectionError::InvalidVerticalGeometry);
    }
    let horizontal_clip_x = [clip_left, paragraph_right];
    let metric_cull_bounds = egui::Rect::from_min_max(
        egui::pos2(clip_left, line_box_top.min(natural_top)),
        egui::pos2(paragraph_right, baseline_y.max(natural_bottom)),
    );
    let natural_extent_exceeds_line_box =
        natural_top < line_box_top || natural_bottom > baseline_y;
    let spans = line
        .spans
        .iter()
        .map(|core| PresentationSpan {
            core,
            origin_x: core.geometry.x.to_points_f32() * scale,
            advance: core.geometry.width.to_points_f32() * scale,
            display_boundary_offsets: Arc::from(
                core.display_boundary_offsets
                    .iter()
                    .map(|offset| offset.to_points_f32() * scale)
                    .collect::<Vec<_>>(),
            ),
        })
        .collect();
    Ok(PresentationLine {
        core: line,
        line_origin_x,
        total_advance,
        line_box_top,
        line_box_height,
        baseline_y,
        horizontal_clip_x,
        metric_cull_bounds,
        natural_extent_exceeds_line_box,
        spans,
    })
}

/// Concatenates the span-local vectors into line-global display boundaries.
/// Each span contributes its leading boundary only once, so a shared span edge
/// is represented once and the final boundary belongs to the last span.
pub(super) fn line_display_boundaries(line: &PresentationLine<'_>) -> Vec<f32> {
    let mut boundaries = Vec::new();
    for span in &line.spans {
        let span_origin = span.origin_x - line.line_origin_x;
        if boundaries.is_empty() {
            boundaries.extend(
                span.display_boundary_offsets
                    .iter()
                    .map(|offset| span_origin + offset),
            );
        } else {
            boundaries.extend(
                span.display_boundary_offsets
                    .iter()
                    .skip(1)
                    .map(|offset| span_origin + offset),
            );
        }
    }
    if boundaries.is_empty() {
        boundaries.push(0.0);
    }
    boundaries
}

/// Returns the page-local screen-point x for a line-global display boundary.
pub(super) fn x_for_display_boundary(
    line: &PresentationLine<'_>,
    display_boundary: usize,
) -> Option<f32> {
    if line.spans.is_empty() {
        return (display_boundary == 0).then_some(line.line_origin_x);
    }
    let (span_index, local_boundary) = span_for_display_boundary(line, display_boundary)?;
    let span = line.spans.get(span_index)?;
    span.display_boundary_offsets
        .get(local_boundary)
        .map(|offset| span.origin_x + offset)
}

/// Maps screen-point x to the nearest projected display boundary. Midpoint
/// ties choose the leading boundary; the result is clamped to this line only.
pub(super) fn display_boundary_for_x(line: &PresentationLine<'_>, x: f32) -> usize {
    let boundaries = line_display_boundaries(line);
    let Some(first) = boundaries.first().copied() else {
        return 0;
    };
    let offsets = x - line.line_origin_x;
    if offsets <= first {
        return 0;
    }
    for (index, pair) in boundaries.windows(2).enumerate() {
        let midpoint = pair[0] + (pair[1] - pair[0]) * 0.5;
        if offsets <= midpoint {
            return index;
        }
    }
    boundaries.len().saturating_sub(1)
}

/// Resolves a projected page-local screen x through the line's canonical
/// display-to-source map. Core line/column ownership has already been chosen.
pub(crate) fn projected_caret_for_x(line: &PresentationLine<'_>, x: f32) -> Caret {
    let display_boundary = display_boundary_for_x(line, x);
    Caret {
        element: line.core.element,
        offset: line.core.source_for_display_char(display_boundary),
    }
}

/// Identifies the span-local owner of a line-global display boundary. At a
/// span transition the boundary is owned by the preceding span's final edge.
pub(super) fn span_for_display_boundary(
    line: &PresentationLine<'_>,
    display_boundary: usize,
) -> Option<(usize, usize)> {
    let mut base = 0usize;
    for (span_index, span) in line.spans.iter().enumerate() {
        let count = span.display_boundary_offsets.len().saturating_sub(1);
        if display_boundary <= base.saturating_add(count) {
            return Some((span_index, display_boundary.saturating_sub(base).min(count)));
        }
        base = base.saturating_add(count);
    }
    line.spans.len().checked_sub(1).map(|span_index| {
        (
            span_index,
            line.spans[span_index]
                .display_boundary_offsets
                .len()
                .saturating_sub(1),
        )
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProjectedCaretRect {
    /// Page-local visual caret rectangle in screen logical points.
    pub rect: egui::Rect,
    /// Page-local canonical line box, retained as the IME baseline anchor.
    pub canonical_line_rect: egui::Rect,
    /// Paragraph/column x clip only; viewport use supplies the page y interval.
    pub horizontal_clip_x: [f32; 2],
}

pub(crate) const CARET_VERTICAL_SHIFT_LEADING_RATIO: f32 = 0.15;

pub(crate) fn projected_caret_rect(
    line: &PresentationLine<'_>,
    source_offset: usize,
    scale: f32,
    caret_width: f32,
) -> Option<ProjectedCaretRect> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let display_boundary = line.core.display_char_for_source(source_offset, false);
    let x = x_for_display_boundary(line, display_boundary)?;
    let canonical_rect = egui::Rect::from_min_size(
        egui::pos2(x, line.line_box_top),
        egui::vec2(caret_width, line.line_box_height.max(0.0)),
    );
    let visual_rect = canonical_rect.translate(egui::vec2(
        0.0,
        line.line_box_height * CARET_VERTICAL_SHIFT_LEADING_RATIO,
    ));
    Some(ProjectedCaretRect {
        rect: visual_rect,
        canonical_line_rect: canonical_rect,
        horizontal_clip_x: line.horizontal_clip_x,
    })
}

/// Stores the preferred vertical x in document fixed points by converting the
/// shared canonical screen-space caret boundary exactly once.
pub(crate) fn preferred_vertical_document_x(
    line: &LayoutLine,
    source_offset: usize,
    projection: Option<&PresentationLine<'_>>,
    scale: f32,
) -> Option<i64> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let projection = projection?;
    let display = line.display_char_for_source(source_offset, false);
    x_for_display_boundary(projection, display)
        .and_then(|x| FixedPoint::from_points_f64(f64::from(x / scale)))
        .map(FixedPoint::raw)
}

/// Converts the stored document-space preferred x to the projected screen
/// point space once, then applies the exact pointer midpoint policy.
pub(crate) fn projected_target_boundary(
    projection: &PresentationLine<'_>,
    document_x_raw: i64,
    scale: f32,
) -> Option<usize> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let screen_x = FixedPoint::from_raw(document_x_raw).to_points_f32() * scale;
    screen_x
        .is_finite()
        .then(|| display_boundary_for_x(projection, screen_x))
}

/// Makes a color-only variant for the selected-foreground pass.
/// Glyph positions, advances, row sizes, and mesh positions remain identical.
pub(crate) fn recolor_galley(galley: &Arc<Galley>, color: Color32) -> Arc<Galley> {
    let mut galley = (**galley).clone();
    for placed_row in &mut galley.rows {
        let row = Arc::make_mut(&mut placed_row.row);
        for vertex in &mut row.visuals.mesh.vertices {
            vertex.color = color;
        }
    }
    Arc::new(galley)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanonicalGalleyError {
    Elided,
    RowCount,
    Newline,
    TextMismatch,
    GlyphCount,
    GlyphCharacter,
    InvalidBoundaries,
    InvalidMeshTopology,
    InvalidVerticalGeometry,
}

/// Builds a canonical single-line galley with canonical x boundaries and a
/// line-box-bottom baseline. Font shaping supplies glyph UVs only; it never
/// supplies line advances, vertical placement, or interaction coordinates.
pub(crate) fn canonical_singleline_galley(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    display_boundary_offsets: &[f32],
    ordinary_color: Color32,
    line_box_height: f32,
    baseline_offset: f32,
) -> Result<Arc<Galley>, CanonicalGalleyError> {
    singleline_galley(
        painter,
        text,
        font,
        ordinary_color,
        line_box_height,
        baseline_offset,
        Some(display_boundary_offsets),
    )
}

/// Builds transient IME preedit with natural horizontal shaping but the same
/// canonical line-box height and baseline as the caret's containing line.
pub(crate) fn ime_singleline_galley(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    color: Color32,
    line_box_height: f32,
    baseline_offset: f32,
) -> Result<Arc<Galley>, CanonicalGalleyError> {
    singleline_galley(
        painter,
        text,
        font,
        color,
        line_box_height,
        baseline_offset,
        None,
    )
}

fn singleline_galley(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    ordinary_color: Color32,
    line_box_height: f32,
    baseline_offset: f32,
    display_boundary_offsets: Option<&[f32]>,
) -> Result<Arc<Galley>, CanonicalGalleyError> {
    let count = text.chars().count();
    if let Some(offsets) = display_boundary_offsets {
        if offsets.len() != count.saturating_add(1)
            || offsets.first().copied() != Some(0.0)
            || !offsets
                .windows(2)
                .all(|pair| pair[0].is_finite() && pair[0] <= pair[1])
        {
            return Err(CanonicalGalleyError::InvalidBoundaries);
        }
    }
    if !line_box_height.is_finite()
        || line_box_height <= 0.0
        || !baseline_offset.is_finite()
        || baseline_offset <= 0.0
    {
        return Err(CanonicalGalleyError::InvalidVerticalGeometry);
    }

    let job = LayoutJob::simple_singleline(text.to_owned(), font.clone(), ordinary_color);
    let mut galley = painter.layout_job(job).as_ref().clone();
    let fail = |reason| Err(reason);
    if galley.elided {
        return fail(CanonicalGalleyError::Elided);
    }
    if galley.rows.len() != 1 {
        return fail(CanonicalGalleyError::RowCount);
    }
    if galley.text() != text {
        return fail(CanonicalGalleyError::TextMismatch);
    }

    let row = &galley.rows[0];
    if row.ends_with_newline {
        return fail(CanonicalGalleyError::Newline);
    }
    let chars = text.chars().collect::<Vec<_>>();
    if row.glyphs.len() != chars.len() {
        return fail(CanonicalGalleyError::GlyphCount);
    }
    if row
        .glyphs
        .iter()
        .zip(&chars)
        .any(|(glyph, expected)| glyph.chr != *expected)
    {
        return fail(CanonicalGalleyError::GlyphCharacter);
    }
    let placed_row = &mut galley.rows[0];
    let row = Arc::make_mut(&mut placed_row.row);
    if !row.size.x.is_finite()
        || !row.size.y.is_finite()
        || !placed_row.pos.x.is_finite()
        || !placed_row.pos.y.is_finite()
    {
        return fail(CanonicalGalleyError::InvalidVerticalGeometry);
    }
    let expected_vertex_count = row
        .glyphs
        .iter()
        .filter(|glyph| !glyph.uv_rect.is_nothing())
        .count()
        .saturating_mul(4);
    if row.visuals.glyph_vertex_range.len() != expected_vertex_count {
        return fail(CanonicalGalleyError::InvalidMeshTopology);
    }

    let pixels_per_point = galley.pixels_per_point;
    let mut vertex_index = row.visuals.glyph_vertex_range.start;
    let vertex_end = row.visuals.glyph_vertex_range.end;
    for (index, glyph) in row.glyphs.iter_mut().enumerate() {
        let (left_boundary, advance) = if let Some(offsets) = display_boundary_offsets {
            let left_boundary = offsets[index];
            let right_boundary = offsets[index + 1];
            (left_boundary, right_boundary - left_boundary)
        } else {
            (glyph.pos.x, glyph.advance_width)
        };
        if !left_boundary.is_finite() || !advance.is_finite() || advance < 0.0 {
            return fail(if display_boundary_offsets.is_some() {
                CanonicalGalleyError::InvalidBoundaries
            } else {
                CanonicalGalleyError::InvalidVerticalGeometry
            });
        }
        let original_baseline = glyph.pos.y;
        let baseline_delta = baseline_offset - original_baseline;
        if !original_baseline.is_finite() || !baseline_delta.is_finite() {
            return fail(CanonicalGalleyError::InvalidVerticalGeometry);
        }
        if !glyph.uv_rect.is_nothing() {
            let Some(vertices) = row
                .visuals
                .mesh
                .vertices
                .get_mut(vertex_index..vertex_index.saturating_add(4))
            else {
                return fail(CanonicalGalleyError::InvalidMeshTopology);
            };
            let round_to_pixel = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
            let left = left_boundary + round_to_pixel(glyph.uv_rect.offset.x);
            let right = left + glyph.uv_rect.size.x;
            if !left.is_finite() || !right.is_finite() {
                return fail(CanonicalGalleyError::InvalidVerticalGeometry);
            }
            vertices[0].pos.x = left;
            vertices[1].pos.x = right;
            vertices[2].pos.x = left;
            vertices[3].pos.x = right;
            for vertex in vertices.iter_mut() {
                let y = vertex.pos.y + baseline_delta;
                if !vertex.pos.y.is_finite() || !y.is_finite() {
                    return fail(CanonicalGalleyError::InvalidVerticalGeometry);
                }
                vertex.pos.y = y;
            }
            vertex_index = vertex_index.saturating_add(4);
        }
        if display_boundary_offsets.is_some() {
            glyph.pos.x = left_boundary;
        }
        glyph.pos.y = baseline_offset;
        glyph.advance_width = advance;
        glyph.line_height = line_box_height;
    }
    if vertex_index != vertex_end {
        return fail(CanonicalGalleyError::InvalidMeshTopology);
    }

    let width = display_boundary_offsets
        .and_then(|offsets| offsets.last().copied())
        .unwrap_or(row.size.x);
    if !width.is_finite() || width < 0.0 {
        return fail(CanonicalGalleyError::InvalidVerticalGeometry);
    }
    placed_row.pos = egui::pos2(0.0, 0.0);
    row.size = egui::vec2(width, line_box_height);
    row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    galley.rect.min.x = 0.0;
    galley.rect.max.x = width;
    galley.rect.min.y = 0.0;
    galley.rect.max.y = line_box_height;
    galley.mesh_bounds = galley.rows.iter().fold(egui::Rect::NOTHING, |bounds, row| {
        bounds.union(row.visuals.mesh_bounds)
    });
    Ok(Arc::new(galley))
}

