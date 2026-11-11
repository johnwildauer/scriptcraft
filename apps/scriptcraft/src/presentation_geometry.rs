//! Desktop-local natural text geometry. Nothing here is written back to core
//! layout or used to alter the fixed-cell page painter in this work order.

use crate::presentation_style::AdvanceMode;
use eframe::egui::{self, Color32, FontFamily, FontId};
use egui::epaint::text::{Galley, LayoutJob};
use scriptcraft_core::formatting::{EffectiveTextStyle, ParagraphAlignment};
use scriptcraft_core::layout::{Caret, LayoutLine, PositionedStyledSpan};
use scriptcraft_core::FixedPoint;
use std::collections::HashMap;
use std::sync::Arc;

const NATURAL_GALLEY_CACHE_ID: &str = "scriptcraft-natural-galley-cache-v1";
const NATURAL_GALLEY_CACHE_LIMIT: usize = 16_384;

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NaturalGalleyKey {
    text: String,
    font_family: FontFamily,
    scaled_font_size_bits: u32,
    effective_style: EffectiveTextStyle,
    advance_mode: AdvanceMode,
    ordinary_color: Color32,
    pixels_per_point_bits: u32,
}

impl NaturalGalleyKey {
    fn new(
        text: &str,
        font: &FontId,
        effective_style: &EffectiveTextStyle,
        advance_mode: AdvanceMode,
        ordinary_color: Color32,
        pixels_per_point: f32,
    ) -> Self {
        Self {
            text: text.to_owned(),
            font_family: font.family.clone(),
            scaled_font_size_bits: font.size.to_bits(),
            effective_style: effective_style.clone(),
            advance_mode,
            ordinary_color,
            pixels_per_point_bits: pixels_per_point.to_bits(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct NaturalGalleyCache {
    geometries: HashMap<NaturalGalleyKey, Arc<NaturalGalleyGeometry>>,
}

impl NaturalGalleyCache {
    fn insert(&mut self, key: NaturalGalleyKey, geometry: Arc<NaturalGalleyGeometry>) {
        if self.geometries.len() >= NATURAL_GALLEY_CACHE_LIMIT {
            // Bounded and deterministic: clear all entries before retaining the
            // next successful value; a miss merely causes a later fresh layout.
            self.geometries.clear();
        }
        self.geometries.insert(key, geometry);
    }

    fn insert_supported(
        &mut self,
        key: NaturalGalleyKey,
        geometry: Result<NaturalGalleyGeometry, NaturalGalleyError>,
    ) -> Result<Arc<NaturalGalleyGeometry>, NaturalGalleyError> {
        let geometry = Arc::new(geometry?);
        self.insert(key, geometry.clone());
        Ok(geometry)
    }
}

/// Builds one natural single-line galley and its cursor-derived display
/// boundaries. Selection is deliberately absent from both the signature and
/// cache identity. A failed topology check is never cached.
pub(super) fn natural_singleline_geometry(
    painter: &egui::Painter,
    ctx: &egui::Context,
    text: &str,
    font: &FontId,
    effective_style: &EffectiveTextStyle,
    advance_mode: AdvanceMode,
    ordinary_color: Color32,
) -> Result<(Arc<NaturalGalleyGeometry>, bool), NaturalGalleyError> {
    if advance_mode != AdvanceMode::Proportional {
        return Err(NaturalGalleyError::NotProportional);
    }

    let key = NaturalGalleyKey::new(
        text,
        font,
        effective_style,
        advance_mode,
        ordinary_color,
        ctx.pixels_per_point(),
    );
    let cached = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<NaturalGalleyCache>(egui::Id::new(
            NATURAL_GALLEY_CACHE_ID,
        ))
        .geometries
        .get(&key)
        .cloned()
    });
    if let Some(geometry) = cached {
        return Ok((geometry, true));
    }

    let job = LayoutJob::simple_singleline(text.to_owned(), font.clone(), ordinary_color);
    let galley = painter.layout_job(job);
    let geometry = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<NaturalGalleyCache>(egui::Id::new(
            NATURAL_GALLEY_CACHE_ID,
        ))
        .insert_supported(key, geometry_from_galley(text, galley))
    })?;
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

#[cfg(test)]
mod tests {
    use crate::presentation_style::AdvanceMode;
    use eframe::egui::{self, Color32, FontFamily, FontId};
    use egui::epaint::text::{Galley, LayoutJob};
    use scriptcraft_core::formatting::{
        EffectiveTextStyle, FontFallback, ParagraphAlignment,
    };
    use scriptcraft_core::layout::{
        FixedRect, LayoutLine, PositionedStyledSpan, Rect, SourceRange,
    };
    use scriptcraft_core::{FixedPoint, Role};
    use std::sync::Arc;

    fn style() -> EffectiveTextStyle {
        EffectiveTextStyle {
            declared_family: "Arial".to_owned(),
            family: FontFallback::CourierPrime,
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            source_color: None,
        }
    }

    fn layout(text: &str) -> Arc<Galley> {
        let context = egui::Context::default();
        let mut galley = None;
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                galley = Some(ui.painter().layout_job(LayoutJob::simple_singleline(
                    text.to_owned(),
                    FontId::new(22.0, FontFamily::Proportional),
                    Color32::BLACK,
                )));
            });
        });
        match galley {
            Some(galley) => galley,
            None => context.fonts_mut(|fonts| {
                fonts.layout_job(LayoutJob::simple_singleline(
                text.to_owned(),
                FontId::new(22.0, FontFamily::Proportional),
                Color32::BLACK,
                ))
            }),
        }
    }

    fn positioned_span(text: &str, x: i64, width: i64) -> PositionedStyledSpan {
        PositionedStyledSpan {
            display_text: Arc::from(text),
            source: SourceRange {
                element: 0,
                start: 0,
                end: text.len(),
            },
            display_source_map: Arc::from([]),
            text_style: style(),
            char_width: FixedPoint::from_points(7),
            geometry: FixedRect {
                x: FixedPoint::from_points(x),
                y: FixedPoint::from_points(0),
                width: FixedPoint::from_points(width),
                height: FixedPoint::from_points(12),
            },
            underline: false,
            strikethrough: false,
        }
    }

    fn line(text: &str, spans: Vec<PositionedStyledSpan>, alignment: ParagraphAlignment) -> LayoutLine {
        let bounds = FixedRect {
            x: FixedPoint::from_points(10),
            y: FixedPoint::from_points(0),
            width: FixedPoint::from_points(100),
            height: FixedPoint::from_points(12),
        };
        LayoutLine {
            page: 0,
            role: Role::Action,
            element: 0,
            source: SourceRange {
                element: 0,
                start: 0,
                end: text.len(),
            },
            text: Arc::from(text),
            display_text: Arc::from(text),
            display_source_map: Arc::from([]),
            spans: spans.into(),
            text_style: style(),
            char_width: FixedPoint::from_points(7),
            alignment,
            unaligned_line_left: FixedPoint::from_points(15),
            geometry: FixedRect {
                x: FixedPoint::from_points(15),
                y: FixedPoint::from_points(0),
                width: FixedPoint::from_points(21),
                height: FixedPoint::from_points(12),
            },
            paragraph_bounds: bounds,
            rect: Rect {
                x: 10,
                y: 0,
                width: 100,
                height: 12,
            },
            keep_with_next: false,
            oversized: false,
        }
    }

    fn synthetic_natural(text: &str, boundaries: &[f32]) -> Arc<super::NaturalGalleyGeometry> {
        let galley = layout(text);
        Arc::new(super::NaturalGalleyGeometry {
            galley,
            display_boundary_offsets: Arc::from(boundaries),
            natural_advance: boundaries.last().copied().unwrap_or(0.0),
        })
    }

    #[test]
    fn mixed_line_projection_accumulates_natural_and_fixed_advances_with_alignment() {
        let mixed = line(
            "AVX",
            vec![positioned_span("AV", 15, 14), positioned_span("X", 29, 7)],
            ParagraphAlignment::Left,
        );
        let natural = synthetic_natural("AV", &[0.0, 5.0, 18.0]);
        let projected = super::project_line(
            &mixed,
            &[AdvanceMode::Proportional, AdvanceMode::FixedCell],
            &[Some(natural.clone()), None],
            1.0,
        );
        assert!(projected.is_ok());
        if let Ok(projected) = projected {
            assert_eq!(projected.line_origin_x, 15.0);
            assert_eq!(projected.total_advance, 25.0);
            assert_eq!(projected.spans[0].display_boundary_offsets.as_ref(), &[0.0, 5.0, 18.0]);
            assert_eq!(projected.spans[1].origin_x, 33.0);
            assert_eq!(projected.spans[1].advance, 7.0);
            assert_eq!(projected.clip_bounds.left(), 10.0);
            assert_eq!(projected.clip_bounds.right(), 110.0);
        }

        for (alignment, expected_origin) in [
            (ParagraphAlignment::Left, 15.0),
            (ParagraphAlignment::Justified, 15.0),
            (ParagraphAlignment::Right, 85.0),
            (ParagraphAlignment::Center, 50.0),
        ] {
            let aligned = line(
                "AVX",
                vec![positioned_span("AV", 15, 14), positioned_span("X", 29, 7)],
                alignment,
            );
            let result = super::project_line(
                &aligned,
                &[AdvanceMode::Proportional, AdvanceMode::FixedCell],
                &[Some(natural.clone()), None],
                1.0,
            );
            assert!(result.is_ok());
            assert_eq!(result.map(|line| line.line_origin_x), Ok(expected_origin));
        }
    }

    #[test]
    fn projection_scales_fixed_anchors_once_but_never_rescales_natural_boundaries() {
        let mixed = line(
            "AVX",
            vec![positioned_span("AV", 15, 14), positioned_span("X", 29, 7)],
            ParagraphAlignment::Right,
        );
        let result = super::project_line(
            &mixed,
            &[AdvanceMode::Proportional, AdvanceMode::FixedCell],
            &[Some(synthetic_natural("AV", &[0.0, 5.0, 18.0])), None],
            2.0,
        );
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.spans[0].display_boundary_offsets.as_ref(), &[0.0, 5.0, 18.0]);
            assert_eq!(result.spans[0].origin_x, 188.0);
            assert_eq!(result.spans[1].origin_x, 206.0);
            assert_eq!(result.spans[1].advance, 14.0);
            assert_eq!(result.total_advance, 32.0);
        }
    }

    #[test]
    fn justification_extra_is_appended_at_gap_boundary_and_never_pads_a_word() {
        let justified = line(
            "AV X",
            vec![
                positioned_span("AV", 15, 14),
                positioned_span(" ", 29, 10),
                positioned_span("X", 39, 7),
            ],
            ParagraphAlignment::Justified,
        );
        let result = super::project_line(
            &justified,
            &[
                AdvanceMode::Proportional,
                AdvanceMode::Proportional,
                AdvanceMode::FixedCell,
            ],
            &[
                Some(synthetic_natural("AV", &[0.0, 5.0, 18.0])),
                Some(synthetic_natural(" ", &[0.0, 4.0])),
                None,
            ],
            1.0,
        );
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.spans[1].display_boundary_offsets.as_ref(), &[0.0, 7.0]);
            assert_eq!(result.spans[1].advance, 7.0);
            assert_eq!(result.spans[2].origin_x, 40.0);
        }

        let ambiguous = line(
            "AVX",
            vec![positioned_span("AV", 15, 17), positioned_span("X", 32, 7)],
            ParagraphAlignment::Justified,
        );
        assert_eq!(
            super::project_line(
                &ambiguous,
                &[AdvanceMode::Proportional, AdvanceMode::FixedCell],
                &[Some(synthetic_natural("AV", &[0.0, 5.0, 18.0])), None],
                1.0,
            )
            .err(),
            Some(super::PresentationProjectionError::AmbiguousJustificationExtra),
        );
    }

    #[test]
    fn projected_selection_intervals_and_decorations_use_natural_advance_boundaries() {
        let mut decorated = positioned_span("AVi", 15, 21);
        decorated.underline = true;
        decorated.strikethrough = true;
        let core = line("AVi", vec![decorated], ParagraphAlignment::Left);
        let natural = synthetic_natural("AVi", &[0.0, 4.0, 10.0, 13.0]);
        let result = super::project_line(
            &core,
            &[AdvanceMode::Proportional],
            &[Some(natural)],
            1.0,
        );
        assert!(result.is_ok());
        if let Ok(projection) = result {
            let selection_edges = |range: std::ops::Range<usize>| {
                Some((
                    super::x_for_display_boundary(&projection, range.start)?,
                    super::x_for_display_boundary(&projection, range.end)?,
                ))
            };
            assert_eq!(selection_edges(0..0), Some((projection.line_origin_x, projection.line_origin_x)));
            assert_eq!(selection_edges(0..1), Some((projection.line_origin_x, projection.line_origin_x + 4.0)));
            assert_eq!(selection_edges(1..2), Some((projection.line_origin_x + 4.0, projection.line_origin_x + 10.0)));
            assert_eq!(selection_edges(2..3), Some((projection.line_origin_x + 10.0, projection.line_origin_x + 13.0)));
            assert_eq!(selection_edges(0..3), Some((projection.line_origin_x, projection.line_origin_x + 13.0)));

            let span = &projection.spans[0];
            assert!(span.core.underline && span.core.strikethrough);
            assert_eq!(span.advance, 13.0);
            assert_eq!(span.origin_x + span.advance, projection.line_origin_x + 13.0);
            assert_eq!(projection.total_advance, 13.0);
            assert_eq!(span.display_boundary_offsets.as_ref(), &[0.0, 4.0, 10.0, 13.0]);
        }
    }

    #[test]
    fn natural_overflow_keeps_advance_and_clips_to_the_owning_paragraph_column() {
        let mut wide = line(
            "AV",
            vec![positioned_span("AV", 15, 14)],
            ParagraphAlignment::Left,
        );
        wide.paragraph_bounds.x = FixedPoint::from_points(60);
        wide.paragraph_bounds.width = FixedPoint::from_points(40);
        wide.unaligned_line_left = FixedPoint::from_points(65);
        let original = wide.clone();
        let result = super::project_line(
            &wide,
            &[AdvanceMode::Proportional],
            &[Some(synthetic_natural("AV", &[0.0, 50.0, 140.0]))],
            1.0,
        );
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.total_advance, 140.0);
            assert_eq!(result.spans[0].advance, 140.0);
            assert_eq!(result.spans[0].origin_x, 65.0);
            assert_eq!(result.clip_bounds.left(), 60.0);
            assert_eq!(result.clip_bounds.right(), 100.0);
            assert_eq!(result.core, &original);
        }
        assert_eq!(wide, original);
    }

    #[test]
    fn no_gap_justification_and_empty_lines_do_not_invent_width() {
        let justified_no_gap = line(
            "AV",
            vec![positioned_span("AV", 15, 14)],
            ParagraphAlignment::Justified,
        );
        let result = super::project_line(
            &justified_no_gap,
            &[AdvanceMode::Proportional],
            &[Some(synthetic_natural("AV", &[0.0, 5.0, 18.0]))],
            1.0,
        );
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.line_origin_x, 15.0);
            assert_eq!(result.total_advance, 18.0);
        }

        let empty = line("", Vec::new(), ParagraphAlignment::Left);
        let result = super::project_line(&empty, &[], &[], 1.0);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.total_advance, 0.0);
            assert!(result.spans.is_empty());
        }
    }

    #[test]
    fn line_global_hit_testing_uses_midpoint_ties_leading_boundary_and_clamps_to_line() {
        let mut core = line(
            "AVX",
            vec![
                positioned_span("A", 15, 7),
                positioned_span("V", 22, 7),
                positioned_span("X", 29, 7),
            ],
            ParagraphAlignment::Left,
        );
        core.display_source_map = Arc::from([
            scriptcraft_core::layout::DisplaySourceSegment { display_start: 0, display_end: 1, source_start: 0, source_end: 1 },
            scriptcraft_core::layout::DisplaySourceSegment { display_start: 1, display_end: 2, source_start: 1, source_end: 2 },
            scriptcraft_core::layout::DisplaySourceSegment { display_start: 2, display_end: 3, source_start: 2, source_end: 3 },
        ]);
        let projected = super::project_line(
            &core,
            &[
                AdvanceMode::Proportional,
                AdvanceMode::Proportional,
                AdvanceMode::Proportional,
            ],
            &[
                Some(synthetic_natural("A", &[0.0, 4.0])),
                Some(synthetic_natural("V", &[0.0, 10.0])),
                Some(synthetic_natural("X", &[0.0, 3.0])),
            ],
            1.0,
        );
        assert!(projected.is_ok());
        if let Ok(projected) = projected {
            assert_eq!(super::line_display_boundaries(&projected).as_slice(), &[0.0, 4.0, 14.0, 17.0]);
            assert_eq!(super::x_for_display_boundary(&projected, 0), Some(15.0));
            assert_eq!(super::x_for_display_boundary(&projected, 3), Some(32.0));
            assert_eq!(super::span_for_display_boundary(&projected, 1), Some((0, 1)));
            assert_eq!(super::span_for_display_boundary(&projected, 2), Some((1, 1)));
            assert_eq!(super::display_boundary_for_x(&projected, -500.0), 0);
            assert_eq!(super::display_boundary_for_x(&projected, 17.0), 0);
            assert_eq!(super::display_boundary_for_x(&projected, 17.01), 1);
            assert_eq!(super::display_boundary_for_x(&projected, 24.0), 1);
            assert_eq!(super::display_boundary_for_x(&projected, 24.01), 2);
            assert_eq!(super::display_boundary_for_x(&projected, 32.0), 3);
            assert_eq!(super::display_boundary_for_x(&projected, 10_000.0), 3);
            assert_eq!(
                super::projected_caret_rect(&projected, 2, 1.0, 1.5)
                    .map(|caret| caret.rect.min),
                Some(egui::pos2(29.0, 0.0)),
            );
            assert_eq!(
                super::projected_caret_rect(&projected, 2, 1.0, 1.5)
                    .map(|caret| caret.clip_bounds),
                Some(projected.clip_bounds),
            );
        }
    }

    #[test]
    fn line_global_boundaries_preserve_monotonic_drag_through_uppercase_expansion() {
        let mut core = line("SSX", vec![positioned_span("SS", 15, 14), positioned_span("X", 29, 7)], ParagraphAlignment::Left);
        core.text = Arc::from("ßX");
        core.source = SourceRange { element: 0, start: 0, end: 3 };
        core.display_source_map = Arc::from([
            scriptcraft_core::layout::DisplaySourceSegment {
                display_start: 0,
                display_end: 2,
                source_start: 0,
                source_end: 2,
            },
            scriptcraft_core::layout::DisplaySourceSegment {
                display_start: 2,
                display_end: 3,
                source_start: 2,
                source_end: 3,
            },
        ]);
        let projected = super::project_line(
            &core,
            &[AdvanceMode::Proportional, AdvanceMode::Proportional],
            &[
                Some(synthetic_natural("SS", &[0.0, 3.0, 8.0])),
                Some(synthetic_natural("X", &[0.0, 6.0])),
            ],
            1.0,
        );
        assert!(projected.is_ok());
        if let Ok(projected) = projected {
            let positions = [-4.0, 15.0, 16.0, 17.0, 18.0, 20.0, 24.0, 100.0]
                .into_iter()
                .map(|x| {
                    super::projected_caret_for_x(&projected, x).offset
                })
                .collect::<Vec<_>>();
            assert!(positions.windows(2).all(|pair| pair[0] <= pair[1]));
            assert!(positions.iter().all(|offset| {
                *offset <= core.source.end && core.text.is_char_boundary(*offset)
            }));
            assert_eq!(super::projected_caret_for_x(&projected, 20.5).offset, 0);
            assert_eq!(super::projected_caret_for_x(&projected, 20.51).offset, 2);
        }
    }

    #[test]
    fn vertical_goal_conversion_covers_fixed_and_projected_source_and_target_lines() {
        let mut current = line(
            "AV",
            vec![positioned_span("AV", 15, 14)],
            ParagraphAlignment::Left,
        );
        current.display_source_map = Arc::from([
            scriptcraft_core::layout::DisplaySourceSegment {
                display_start: 0,
                display_end: 1,
                source_start: 0,
                source_end: 1,
            },
            scriptcraft_core::layout::DisplaySourceSegment {
                display_start: 1,
                display_end: 2,
                source_start: 1,
                source_end: 2,
            },
        ]);
        let target = line(
            "WIDE",
            vec![positioned_span("WIDE", 15, 28)],
            ParagraphAlignment::Left,
        );
        let current_projected = super::project_line(
            &current,
            &[AdvanceMode::Proportional],
            &[Some(synthetic_natural("AV", &[0.0, 5.0, 18.0]))],
            2.0,
        );
        let target_projected = super::project_line(
            &target,
            &[AdvanceMode::Proportional],
            &[Some(synthetic_natural("WIDE", &[0.0, 6.0, 13.0, 19.0, 27.0]))],
            2.0,
        );
        assert!(current_projected.is_ok());
        assert!(target_projected.is_ok());
        if let (Ok(current_projected), Ok(target_projected)) =
            (current_projected, target_projected)
        {
            let source = current.display_char_for_source(1, false);
            let fixed_goal = super::preferred_vertical_document_x(&current, 1, None, 2.0);
            let proportional_goal = super::preferred_vertical_document_x(
                &current,
                1,
                Some(&current_projected),
                2.0,
            );
            assert_eq!(fixed_goal, Some(current.geometry.x.raw() + source as i64 * current.char_width.raw()));
            assert_eq!(
                proportional_goal,
                super::x_for_display_boundary(&current_projected, source)
                    .and_then(|x| FixedPoint::from_points_f64(f64::from(x / 2.0)))
                    .map(FixedPoint::raw),
            );

            // Fixed -> fixed retains the document-cell target formula.
            let fixed_target_boundary = |goal: i64| {
                let cell = target.char_width.raw().max(1);
                (goal
                    .saturating_sub(target.geometry.x.raw())
                    .saturating_add(cell / 2)
                    / cell) as usize
            };
            assert_eq!(fixed_target_boundary(fixed_goal.unwrap_or_default()), 1);
            // Proportional -> fixed retains the projected current x as the
            // stored document goal, then uses the same fixed target formula.
            let projected_goal = proportional_goal.unwrap_or_default();
            assert_eq!(fixed_target_boundary(projected_goal), 0);

            // Fixed -> proportional and proportional -> proportional convert
            // that same stored document x to page-local screen points once.
            assert_eq!(
                super::projected_target_boundary(
                    &target_projected,
                    fixed_goal.unwrap_or_default(),
                    2.0,
                ),
                Some(2),
            );
            assert_eq!(
                super::projected_target_boundary(&target_projected, projected_goal, 2.0),
                Some(1),
            );
        }
    }
}
