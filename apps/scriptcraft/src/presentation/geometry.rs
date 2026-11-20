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
    /// Page-local screen-space paragraph/column clip bounds.
    pub clip_bounds: egui::Rect,
    pub spans: Vec<PresentationSpan<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PresentationProjectionError {
    InvalidScale,
    DisplayTopologyMismatch,
    InvalidCanonicalBoundaries,
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
    let paragraph_right = FixedPoint::from_raw(
        line.paragraph_bounds
            .x
            .raw()
            .saturating_add(line.paragraph_bounds.width.raw()),
    )
    .to_points_f32()
        * scale;
    let paragraph_y = line.paragraph_bounds.y.to_points_f32() * scale;
    let paragraph_height = line.paragraph_bounds.height.to_points_f32() * scale;
    let clip_bounds = egui::Rect::from_min_size(
        egui::pos2(paragraph_left, paragraph_y),
        egui::vec2(
            (paragraph_right - paragraph_left).max(0.0),
            paragraph_height.max(0.0),
        ),
    );
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
        clip_bounds,
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
    /// Page-local screen logical points, matching the projected paint geometry.
    pub rect: egui::Rect,
    /// Page-local screen logical points; intersect with the page at viewport use.
    pub clip_bounds: egui::Rect,
}

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
    let y = line.core.geometry.y.to_points_f32() * scale;
    let height = line.core.geometry.height.to_points_f32() * scale;
    Some(ProjectedCaretRect {
        rect: egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(caret_width, height.max(0.0))),
        clip_bounds: line.clip_bounds,
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

/// Makes a color-only variant for the clipped selected-foreground pass.
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
}

/// Builds a single-line galley and places each glyph origin at the matching
/// canonical display boundary. Font shaping supplies glyph UVs only; it never
/// supplies line advances or interaction coordinates.
pub(crate) fn canonical_singleline_galley(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    display_boundary_offsets: &[f32],
    ordinary_color: Color32,
) -> Result<Arc<Galley>, CanonicalGalleyError> {
    let count = text.chars().count();
    if display_boundary_offsets.len() != count.saturating_add(1)
        || display_boundary_offsets.first().copied() != Some(0.0)
        || !display_boundary_offsets
            .windows(2)
            .all(|pair| pair[0].is_finite() && pair[0] <= pair[1])
    {
        return Err(CanonicalGalleyError::InvalidBoundaries);
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
        let left_boundary = display_boundary_offsets[index];
        let right_boundary = display_boundary_offsets[index + 1];
        let advance = right_boundary - left_boundary;
        if !left_boundary.is_finite() || !advance.is_finite() || advance < 0.0 {
            return fail(CanonicalGalleyError::InvalidBoundaries);
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
            vertices[0].pos.x = left;
            vertices[1].pos.x = right;
            vertices[2].pos.x = left;
            vertices[3].pos.x = right;
            vertex_index = vertex_index.saturating_add(4);
        }
        glyph.pos.x = left_boundary;
        glyph.advance_width = advance;
    }
    if vertex_index != vertex_end {
        return fail(CanonicalGalleyError::InvalidMeshTopology);
    }

    let width = *display_boundary_offsets.last().unwrap_or(&0.0);
    placed_row.pos.x = 0.0;
    row.size.x = width;
    row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    galley.rect.min.x = 0.0;
    galley.rect.max.x = width;
    galley.mesh_bounds = galley.rows.iter().fold(egui::Rect::NOTHING, |bounds, row| {
        bounds.union(row.visuals.mesh_bounds)
    });
    Ok(Arc::new(galley))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_galley_uses_supplied_offsets_for_every_glyph() {
        let context = egui::Context::default();
        context.run(egui::RawInput::default(), |context| {
            let painter = context.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("canonical-galley-test"),
            ));
            let offsets = [0.0, 4.25, 17.0];
            let galley = canonical_singleline_galley(
                &painter,
                "Wi",
                &FontId::monospace(12.0),
                &offsets,
                Color32::WHITE,
            )
            .expect("supported one-glyph-per-scalar topology");
            let glyphs = &galley.rows[0].row.glyphs;
            assert_eq!(glyphs.len(), 2);
            assert_eq!(glyphs[0].pos.x, 0.0);
            assert_eq!(glyphs[0].advance_width, 4.25);
            assert_eq!(glyphs[1].pos.x, 4.25);
            assert_eq!(glyphs[1].advance_width, 12.75);
            assert_eq!(galley.rect.width(), 17.0);
        });
    }

    #[test]
    fn canonical_galley_rejects_boundary_topology_mismatch() {
        let context = egui::Context::default();
        let painter = context.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("canonical-galley-invalid-test"),
        ));
        let error = canonical_singleline_galley(
            &painter,
            "two",
            &FontId::monospace(12.0),
            &[0.0, 5.0],
            Color32::WHITE,
        )
        .expect_err("one display boundary is required per scalar plus final edge");
        assert_eq!(error, CanonicalGalleyError::InvalidBoundaries);
    }

    #[test]
    fn proportional_canonical_boundaries_drive_pointer_and_caret_projection() {
        let mut project = scriptcraft_fdx::new_project()
            .unwrap_or_else(|_| unreachable!("default project is valid"));
        project.elements = vec![scriptcraft_model::BodyOwner::shared(
            scriptcraft_model::ScreenplayElement::new(
                "desktop-proportional-boundary",
                scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
                "Wi",
            )
            .unwrap_or_else(|_| unreachable!("bounded short element is valid")),
        )];
        let layout = scriptcraft_layout::ScreenplayLayout::build(&project)
            .unwrap_or_else(|_| unreachable!("default layout inputs are valid"));
        let source_line = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .find(|line| line.element == 0)
            .unwrap_or_else(|| unreachable!("project contains the test line"));
        let mut line = source_line.clone();
        let mut spans = line.spans.to_vec();
        assert_eq!(spans.len(), 1);
        spans[0].geometry.width = FixedPoint::from_raw(1_700);
        spans[0].display_boundary_offsets = Arc::from(vec![
            FixedPoint::default(),
            FixedPoint::from_raw(425),
            FixedPoint::from_raw(1_700),
        ]);
        line.spans = Arc::from(spans);
        line.geometry.width = FixedPoint::from_raw(1_700);

        let projected = project_line(&line, 2.0)
            .unwrap_or_else(|_| unreachable!("synthetic proportional boundaries are valid"));
        let second_boundary = projected.line_origin_x + 8.5;
        assert_eq!(x_for_display_boundary(&projected, 1), Some(second_boundary));
        assert_eq!(
            projected_caret_for_x(&projected, second_boundary + 0.5).offset,
            1
        );
        let caret = projected_caret_rect(&projected, 1, 2.0, 1.0)
            .unwrap_or_else(|| unreachable!("source boundary resolves to a projected caret"));
        assert_eq!(caret.rect.min.x, second_boundary);
        assert_eq!(display_boundary_for_x(&projected, second_boundary + 0.5), 1);
    }
}
