//! Desktop-local natural text geometry. Nothing here is written back to core
//! layout or used to alter the fixed-cell page painter in this work order.

use crate::presentation_style::AdvanceMode;
use eframe::egui::{self, Color32, FontId};
use egui::epaint::text::{Galley, LayoutJob};
use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
use scriptcraft_layout::{Caret, LayoutLine, PositionedStyledSpan};
use std::sync::Arc;

/// An unmodified full-span galley and boundary x offsets derived from its own
/// cursor map. Offsets are in the galley's logical-point coordinate space.
#[derive(Clone, Debug)]
pub(super) struct NaturalGalleyGeometry {
    pub galley: Arc<Galley>,
    pub display_boundary_offsets: Arc<[f32]>,
    pub natural_advance: f32,
}

/// One ephemeral span in the mixed-mode desktop line projection. `origin_x`,
/// `advance`, and `display_boundary_offsets` are page-local screen points.
#[derive(Clone, Debug)]
pub(super) struct PresentationSpan<'a> {
    pub core: &'a PositionedStyledSpan,
    pub advance_mode: AdvanceMode,
    pub origin_x: f32,
    pub advance: f32,
    pub display_boundary_offsets: Arc<[f32]>,
    pub natural: Option<Arc<NaturalGalleyGeometry>>,
}

/// A line projection exists only if the line contains an actually available
/// proportional face. It owns no core layout state and is rebuilt per frame.
#[derive(Clone, Debug)]
pub(super) struct PresentationLine<'a> {
    pub core: &'a LayoutLine,
    pub line_origin_x: f32,
    pub total_advance: f32,
    /// Page-local screen-space paragraph/column clip bounds.
    pub clip_bounds: egui::Rect,
    pub spans: Vec<PresentationSpan<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PresentationProjectionError {
    InvalidScale,
    SpanCountMismatch,
    DisplayTopologyMismatch,
    MissingNaturalGeometry,
    UnexpectedNaturalGeometry,
    InvalidNaturalBoundaries,
    AmbiguousJustificationExtra,
}

/// Projects a complete line from actual-face modes and the natural geometry
/// already built for its proportional spans. Core fixed allocations are
/// converted once; natural offsets are already in the paint font's point space.
pub(super) fn project_line<'a>(
    line: &'a LayoutLine,
    modes: &[AdvanceMode],
    natural: &[Option<Arc<NaturalGalleyGeometry>>],
    scale: f32,
) -> Result<PresentationLine<'a>, PresentationProjectionError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(PresentationProjectionError::InvalidScale);
    }
    if modes.len() != line.spans.len() || natural.len() != line.spans.len() {
        return Err(PresentationProjectionError::SpanCountMismatch);
    }

    let line_chars = line.display_text.chars().collect::<Vec<_>>();
    let mut display_base = 0usize;
    let mut advances = Vec::with_capacity(line.spans.len());
    let mut boundaries = Vec::with_capacity(line.spans.len());
    let mut extras = Vec::with_capacity(line.spans.len());

    for (index, span) in line.spans.iter().enumerate() {
        let count = span.display_text.chars().count();
        if line_chars.get(display_base..display_base.saturating_add(count))
            != Some(span.display_text.chars().collect::<Vec<_>>().as_slice())
        {
            return Err(PresentationProjectionError::DisplayTopologyMismatch);
        }
        let fixed_width = span.geometry.width.to_points_f32() * scale;
        let nominal_width = FixedPoint::from_raw(
            span.char_width
                .raw()
                .saturating_mul(i64::try_from(count).unwrap_or(i64::MAX)),
        );
        let extra_raw = span
            .geometry
            .width
            .raw()
            .saturating_sub(nominal_width.raw())
            .max(0);
        let extra = FixedPoint::from_raw(extra_raw).to_points_f32() * scale;
        if extra_raw > 0 {
            let is_gap = count > 0
                && line.alignment == ParagraphAlignment::Justified
                && span.display_text.chars().all(char::is_whitespace)
                && display_base > 0
                && display_base + count < line_chars.len()
                && !line_chars[display_base - 1].is_whitespace()
                && !line_chars[display_base + count].is_whitespace();
            if !is_gap {
                return Err(PresentationProjectionError::AmbiguousJustificationExtra);
            }
        }
        extras.push(extra);

        match (modes[index], natural[index].as_ref()) {
            (AdvanceMode::Proportional, Some(geometry)) => {
                if geometry.display_boundary_offsets.len() != count + 1
                    || !geometry.natural_advance.is_finite()
                    || geometry.display_boundary_offsets.first().copied() != Some(0.0)
                    || geometry.display_boundary_offsets.last().copied()
                        != Some(geometry.natural_advance)
                    || !geometry
                        .display_boundary_offsets
                        .windows(2)
                        .all(|pair| pair[0].is_finite() && pair[0] <= pair[1])
                {
                    return Err(PresentationProjectionError::InvalidNaturalBoundaries);
                }
                let mut offsets = geometry.display_boundary_offsets.to_vec();
                if let Some(final_boundary) = offsets.last_mut() {
                    *final_boundary += extra;
                }
                advances.push(geometry.natural_advance + extra);
                boundaries.push(Arc::<[f32]>::from(offsets));
            }
            (AdvanceMode::Proportional, None) => {
                return Err(PresentationProjectionError::MissingNaturalGeometry);
            }
            (AdvanceMode::FixedCell, Some(_)) => {
                return Err(PresentationProjectionError::UnexpectedNaturalGeometry);
            }
            (AdvanceMode::FixedCell, None) => {
                let cell = if count > 0 && fixed_width > span.char_width.to_points_f32() * count as f32 * scale {
                    fixed_width / count as f32
                } else {
                    span.char_width.to_points_f32() * scale
                };
                let mut offsets = (0..=count)
                    .map(|boundary| boundary as f32 * cell)
                    .collect::<Vec<_>>();
                if let Some(final_boundary) = offsets.last_mut() {
                    *final_boundary = fixed_width;
                }
                advances.push(fixed_width);
                boundaries.push(Arc::<[f32]>::from(offsets));
            }
        }
        display_base = display_base.saturating_add(count);
    }
    if display_base != line_chars.len() {
        return Err(PresentationProjectionError::DisplayTopologyMismatch);
    }

    let total_advance = advances.iter().sum::<f32>();
    let line_left = line.unaligned_line_left.to_points_f32() * scale;
    let paragraph_left = line.paragraph_bounds.x.to_points_f32() * scale;
    let paragraph_right = FixedPoint::from_raw(
        line.paragraph_bounds
            .x
            .raw()
            .saturating_add(line.paragraph_bounds.width.raw()),
    )
    .to_points_f32()
        * scale;
    let line_origin_x = match line.alignment {
        ParagraphAlignment::Left | ParagraphAlignment::Justified => line_left,
        ParagraphAlignment::Right => paragraph_right - total_advance,
        ParagraphAlignment::Center => {
            let spare = (paragraph_right - line_left - total_advance).max(0.0);
            line_left + spare / 2.0
        }
    };
    let paragraph_y = line.paragraph_bounds.y.to_points_f32() * scale;
    let paragraph_height = line.paragraph_bounds.height.to_points_f32() * scale;
    let clip_bounds = egui::Rect::from_min_size(
        egui::pos2(paragraph_left, paragraph_y),
        egui::vec2(
            (paragraph_right - paragraph_left).max(0.0),
            paragraph_height.max(0.0),
        ),
    );
    let mut cursor_x = line_origin_x;
    let spans = line
        .spans
        .iter()
        .enumerate()
        .map(|(index, core)| {
            let projected = PresentationSpan {
                core,
                advance_mode: modes[index],
                origin_x: cursor_x,
                advance: advances[index],
                display_boundary_offsets: boundaries[index].clone(),
                natural: natural[index].clone(),
            };
            cursor_x += advances[index];
            projected
        })
        .collect();
    debug_assert_eq!(extras.len(), line.spans.len());
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
        if boundaries.is_empty() {
            boundaries.extend(span.display_boundary_offsets.iter().copied());
        } else {
            boundaries.extend(span.display_boundary_offsets.iter().skip(1).copied());
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
pub(super) fn projected_caret_for_x(line: &PresentationLine<'_>, x: f32) -> Caret {
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
    line.spans
        .len()
        .checked_sub(1)
        .map(|span_index| (span_index, line.spans[span_index].display_boundary_offsets.len().saturating_sub(1)))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ProjectedCaretRect {
    /// Page-local screen logical points, matching the projected paint geometry.
    pub rect: egui::Rect,
    /// Page-local screen logical points; intersect with the page at viewport use.
    pub clip_bounds: egui::Rect,
}

pub(super) fn projected_caret_rect(
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
        rect: egui::Rect::from_min_size(
            egui::pos2(x, y),
            egui::vec2(caret_width, height.max(0.0)),
        ),
        clip_bounds: line.clip_bounds,
    })
}

/// Stores the preferred vertical x in document fixed points. Fixed-only
/// callers keep the historical fixed-column expression; projected callers
/// convert the shared screen-space caret boundary exactly once.
pub(super) fn preferred_vertical_document_x(
    line: &LayoutLine,
    source_offset: usize,
    projection: Option<&PresentationLine<'_>>,
    scale: f32,
) -> Option<i64> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    if let Some(projection) = projection {
        let display = line.display_char_for_source(source_offset, false);
        return x_for_display_boundary(projection, display)
            .and_then(|x| FixedPoint::from_points_f64(f64::from(x / scale)))
            .map(FixedPoint::raw);
    }
    let columns = line.display_char_for_source(source_offset, false) as i64;
    Some(line.geometry.x.raw() + columns.saturating_mul(line.char_width.raw()))
}

/// Converts the stored document-space preferred x to the projected screen
/// point space once, then applies the exact pointer midpoint policy.
pub(super) fn projected_target_boundary(
    projection: &PresentationLine<'_>,
    document_x_raw: i64,
    scale: f32,
) -> Option<usize> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let screen_x = FixedPoint::from_raw(document_x_raw).to_points_f32() * scale;
    screen_x.is_finite().then(|| display_boundary_for_x(projection, screen_x))
}

/// Makes a color-only galley variant for the clipped selected-foreground pass.
/// Glyph positions, advances, row sizes, and mesh positions remain identical.
pub(super) fn recolor_natural_galley(
    geometry: &NaturalGalleyGeometry,
    color: Color32,
) -> Arc<Galley> {
    let mut galley = (*geometry.galley).clone();
    for placed_row in &mut galley.rows {
        let row = Arc::make_mut(&mut placed_row.row);
        for vertex in &mut row.visuals.mesh.vertices {
            vertex.color = color;
        }
    }
    Arc::new(galley)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NaturalGalleyTopology {
    Elided,
    RowCount,
    Newline,
    TextMismatch,
    GlyphCount,
    GlyphCharacter,
    InvalidOriginOrAdvance,
    InvalidGlyphGeometry,
    CursorMismatch,
    NonFiniteCursor,
    NonMonotonicCursor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NaturalGalleyError {
    NotProportional,
    UnsupportedTopology(NaturalGalleyTopology),
}

/// Builds one natural single-line galley and its cursor-derived display
/// boundaries. Selection is deliberately absent from both the signature and
/// geometry. The UV-bearing result is scoped to the current egui pass; egui's
/// own layout cache remains responsible for atlas-aware text layout reuse.
pub(super) fn natural_singleline_geometry(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    advance_mode: AdvanceMode,
    ordinary_color: Color32,
) -> Result<(Arc<NaturalGalleyGeometry>, bool), NaturalGalleyError> {
    if advance_mode != AdvanceMode::Proportional {
        return Err(NaturalGalleyError::NotProportional);
    }

    let job = LayoutJob::simple_singleline(text.to_owned(), font.clone(), ordinary_color);
    let galley = painter.layout_job(job);
    let geometry = Arc::new(geometry_from_galley(text, galley)?);
    Ok((geometry, false))
}

fn geometry_from_galley(
    display_text: &str,
    galley: Arc<Galley>,
) -> Result<NaturalGalleyGeometry, NaturalGalleyError> {
    use NaturalGalleyTopology as Topology;
    let fail = |reason| Err(NaturalGalleyError::UnsupportedTopology(reason));
    if galley.elided {
        return fail(Topology::Elided);
    }
    if galley.rows.len() != 1 {
        return fail(Topology::RowCount);
    }
    if galley.text() != display_text {
        return fail(Topology::TextMismatch);
    }

    let row = &galley.rows[0];
    if row.ends_with_newline {
        return fail(Topology::Newline);
    }
    let chars = display_text.chars().collect::<Vec<_>>();
    if row.glyphs.len() != chars.len() {
        return fail(Topology::GlyphCount);
    }
    if row
        .glyphs
        .iter()
        .zip(&chars)
        .any(|(glyph, expected)| glyph.chr != *expected)
    {
        return fail(Topology::GlyphCharacter);
    }

    let natural_advance = galley.rect.width();
    if !natural_advance.is_finite()
        || natural_advance < 0.0
        || galley.rect.left() != 0.0
        || galley.rect.right() != row.size.x
        || row.pos.x != 0.0
    {
        return fail(Topology::InvalidOriginOrAdvance);
    }
    if !row.size.x.is_finite()
        || row.glyphs.iter().any(|glyph| {
            !glyph.pos.x.is_finite()
                || !glyph.advance_width.is_finite()
                || glyph.advance_width < 0.0
        })
    {
        return fail(Topology::InvalidGlyphGeometry);
    }

    let mut offsets = Vec::with_capacity(chars.len() + 1);
    for boundary in 0..=chars.len() {
        // CCursor indexes Unicode scalar boundaries, not UTF-8 bytes. epaint's
        // cursor map and the glyph records are both read from this same galley.
        let cursor_x = galley
            .pos_from_cursor(egui::text::CCursor::new(boundary))
            .left();
        if !cursor_x.is_finite() {
            return fail(Topology::NonFiniteCursor);
        }
        let expected_x = row
            .glyphs
            .get(boundary)
            .map_or(natural_advance, |glyph| glyph.pos.x);
        if cursor_x != expected_x {
            return fail(Topology::CursorMismatch);
        }
        if offsets.last().is_some_and(|previous| cursor_x < *previous) {
            return fail(Topology::NonMonotonicCursor);
        }
        offsets.push(cursor_x);
    }
    if offsets.first().copied() != Some(0.0)
        || offsets.last().copied() != Some(natural_advance)
    {
        return fail(Topology::InvalidOriginOrAdvance);
    }

    Ok(NaturalGalleyGeometry {
        galley,
        display_boundary_offsets: offsets.into(),
        natural_advance,
    })
}
