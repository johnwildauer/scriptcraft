//! Non-paginated, card-local Beat text layout. Board pixels are logical points.

use crate::{
    geometry::{Caret, FixedRect, LayoutError, LayoutLine, LocatedLayoutLine, ScreenplayLayout},
    inputs::{EffectiveParagraph, LayoutEnvironment, PaginationPolicy},
    wrap::{self, ResolvedWrappedLine},
};
use scriptcraft_model::{
    beat_board::{BeatCard, BoardFixed},
    formatting::{FixedPoint, ParagraphAlignment, ParagraphLeading, ParagraphSpacing},
    layout_inputs::DocumentLeading,
    typography::{EffectiveTextStyle, FaceStyle},
};

pub const BEAT_TITLE_PLACEHOLDER: &str = "Beat Title";
pub const BEAT_BODY_PLACEHOLDER: &str = "Body";

/// Card-local dimensions, in unscaled logical points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BeatCardMetrics {
    pub padding: FixedPoint,
    pub title_height: FixedPoint,
    pub title_body_gap: FixedPoint,
}

impl Default for BeatCardMetrics {
    fn default() -> Self {
        Self {
            padding: FixedPoint::from_points(12),
            title_height: FixedPoint::from_points(20),
            title_body_gap: FixedPoint::from_points(8),
        }
    }
}

/// A display-only glyph: unlike a positioned text span it has no source range
/// or caret boundaries. The consumer clips it to the title content rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatDisplayOnlySpan {
    pub text: &'static str,
    pub resolved_text_style: EffectiveTextStyle,
    pub geometry: FixedRect,
}

/// Ordinary truncated presentation; interaction always uses the full title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatTitleProjection {
    pub prefix: LayoutLine,
    pub ellipsis: BeatDisplayOnlySpan,
}

/// Complete, unscaled card-local layout. Body line `element` and `source.element`
/// are the paragraph index; all source offsets are paragraph-local UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatCardLayout {
    pub title_rect: FixedRect,
    pub body_rect: FixedRect,
    pub title: LayoutLine,
    pub truncated_title: Option<BeatTitleProjection>,
    pub body_lines: Vec<LayoutLine>,
    pub body_content_height: FixedPoint,
    pub body_is_empty: bool,
}

impl BeatCardLayout {
    /// Build without host font discovery, screenplay settings, or pagination.
    /// The supplied environment must contain the needed Arial and run faces.
    pub fn new(
        card: &BeatCard,
        environment: &LayoutEnvironment,
        metrics: BeatCardMetrics,
    ) -> Result<Self, LayoutError> {
        // Layout's hundredths cannot represent every board milli-pixel. Whole
        // pixels (and hundredths) are exact; other values round to nearest, ties
        // away from zero. The durable board geometry is never changed or snapped.
        let width = board_dimension(card.rect.width);
        let height = board_dimension(card.rect.height);
        let padding = metrics.padding.raw().max(0);
        let left = padding.min(width);
        let right = width.saturating_sub(padding).max(left);
        let top = padding.min(height);
        let bottom = height.saturating_sub(padding).max(top);
        let title_bottom = top.saturating_add(metrics.title_height.raw().max(0)).min(bottom);
        let body_top = title_bottom.saturating_add(metrics.title_body_gap.raw().max(0)).min(bottom);
        let rect = |y: i64, h: i64| FixedRect {
            x: FixedPoint::from_raw(left),
            y: FixedPoint::from_raw(y),
            width: FixedPoint::from_raw(right - left),
            height: FixedPoint::from_raw(h),
        };
        let title_rect = rect(top, title_bottom - top);
        let body_rect = rect(body_top, bottom - body_top);

        // Card leading is independent of the screenplay's DocumentLeading.
        let mut card_environment = environment.clone();
        card_environment.page.document_leading = DocumentLeading::Normal;
        let title_style = base_style(&card_environment, true)?;
        let body_style = base_style(&card_environment, false)?;
        let paragraph = EffectiveParagraph {
            left: title_rect.x,
            right: FixedPoint::from_raw(right),
            first_indent: FixedPoint::default(),
            alignment: ParagraphAlignment::Left,
            space_before: FixedPoint::default(),
            space_before_units: 0,
            base_leading: FixedPoint::from_points(12),
            leading: ParagraphLeading::Regular,
            spacing: ParagraphSpacing::Single,
            starts_new_page: false,
            pagination: PaginationPolicy::Ordinary,
        };

        let mut title_scalars = wrap::project_resolved_display_scalars(
            &card.title, 0..card.title.len(), &[], &title_style, &card_environment.fonts,
        )?;
        // Imported line breaks remain canonical source scalars, but a fixed
        // single-line title displays each as a measured space.
        let face = card_environment.fonts.require_face(&title_style.resolved_font_id)?;
        for scalar in &mut title_scalars {
            if matches!(scalar.character, '\r' | '\n') {
                scalar.character = ' ';
                scalar.advance = face.metrics.scale_advance(' ', title_style.size)?;
            }
        }
        let title_line = |count: usize, end: usize| -> Result<LayoutLine, LayoutError> {
            let wrapped = wrap::make_resolved_wrapped_line(
                &card.title, 0, None, paragraph, &title_style, &card_environment, &[],
                &title_scalars, ResolvedWrappedLine {
                    scalar_range: 0..count,
                    source_range: 0..end,
                    first_visual_line: true,
                    final_visual_line: true,
                    oversized: false,
                },
            )?;
            Ok(ScreenplayLayout::make_line(0, wrapped, true, title_rect.y))
        };
        let title = title_line(title_scalars.len(), card.title.len())?;
        let truncated_title = if title.geometry.width > title_rect.width {
            let ellipsis_width = face.metrics.scale_advance('…', title_style.size)?;
            let available = title_rect.width.raw().saturating_sub(ellipsis_width.raw().max(0)).max(0);
            let mut count = 0;
            let mut width = 0i64;
            while count < title_scalars.len() {
                let source = &title_scalars[count].source_range;
                let mut end = count + 1;
                while end < title_scalars.len() && title_scalars[end].source_range == *source {
                    end += 1;
                }
                let unit = title_scalars[count..end].iter().fold(0i64, |sum, scalar| {
                    sum.saturating_add(scalar.advance.raw().max(0))
                });
                if width.saturating_add(unit) > available {
                    break;
                }
                width = width.saturating_add(unit);
                count = end;
            }
            let end = title_scalars.get(count.wrapping_sub(1)).map_or(0, |scalar| scalar.source_range.end);
            let prefix = title_line(count, end)?;
            let ellipsis = BeatDisplayOnlySpan {
                text: "…",
                resolved_text_style: title_style.clone(),
                geometry: FixedRect {
                    x: FixedPoint::from_raw(left.saturating_add(width)),
                    y: title_rect.y,
                    width: ellipsis_width,
                    height: prefix.geometry.height,
                },
            };
            Some(BeatTitleProjection { prefix, ellipsis })
        } else {
            None
        };

        let mut body_lines = Vec::new();
        let mut y = body_top;
        for (index, source) in card.paragraphs.iter().enumerate() {
            let wrapped = wrap::wrap_resolved_for_card(
                &source.text, index, paragraph, &body_style, &card_environment, &source.runs,
            )?;
            for line in wrapped {
                let line = ScreenplayLayout::make_line(0, line, false, FixedPoint::from_raw(y));
                y = y.saturating_add(line.vertical_metrics.leading.raw());
                body_lines.push(line);
            }
        }
        Ok(Self {
            title_rect,
            body_rect,
            title,
            truncated_title,
            body_lines,
            body_content_height: FixedPoint::from_raw(y.saturating_sub(body_top)),
            body_is_empty: card.paragraphs.iter().all(|paragraph| paragraph.text.is_empty()),
        })
    }

    /// Whole leading-height line boxes that fit, stopping at the first overflow.
    pub fn visible_body_lines(&self) -> impl Iterator<Item = &LayoutLine> {
        let bottom = self.body_rect.bottom().raw();
        self.body_lines.iter().take_while(move |line| {
            line.geometry.y.raw().saturating_add(line.geometry.height.raw()) <= bottom
        })
    }

    /// Map x on the full title, never on its display-only ellipsis.
    pub fn title_caret_for_x(&self, x: FixedPoint) -> Caret {
        located(&self.title).fixed_caret_for_x(x)
    }

    pub fn title_caret_geometry(&self, offset: usize) -> Result<FixedRect, LayoutError> {
        located(&self.title).fixed_caret_geometry(&Caret { element: 0, offset })
    }

    /// Map on a previously chosen body line; no selection or scrolling policy is implied.
    pub fn body_caret_for_x(&self, line_index: usize, x: FixedPoint) -> Option<Caret> {
        self.body_lines.get(line_index).map(|line| located(line).fixed_caret_for_x(x))
    }

    pub fn body_caret_geometry(&self, line_index: usize, caret: &Caret) -> Result<FixedRect, LayoutError> {
        let line = self.body_lines.get(line_index).ok_or(LayoutError {
            kind: crate::geometry::LayoutErrorKind::InvalidPosition,
        })?;
        located(line).fixed_caret_geometry(caret)
    }
}

fn located(line: &LayoutLine) -> LocatedLayoutLine<'_> {
    LocatedLayoutLine { page_index: 0, line }
}

fn board_dimension(value: BoardFixed) -> i64 {
    let raw = value.raw().max(0);
    // Divide before rounding to avoid overflowing at the edge of i64.
    raw / 10 + if raw % 10 >= 5 { 1 } else { 0 }
}

fn base_style(environment: &LayoutEnvironment, bold: bool) -> Result<EffectiveTextStyle, LayoutError> {
    let face = environment.fonts.resolve_face_id("Arial", FaceStyle::from_flags(bold, false), None)?.clone();
    Ok(EffectiveTextStyle::new(
        "Arial", None, face, FixedPoint::from_points(12), bold, false, false, false, false,
    )?)
}
