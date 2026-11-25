//! Fixed, non-paginated measured text layout for a native ScriptNote card.

use crate::{
    geometry::{Caret, FixedRect, LayoutError, LayoutLine, LocatedLayoutLine, ScreenplayLayout},
    inputs::{EffectiveParagraph, LayoutEnvironment, PaginationPolicy},
    wrap::{self, ResolvedWrappedLine},
};
use scriptcraft_model::{
    formatting::{FixedPoint, ParagraphAlignment, ParagraphLeading, ParagraphSpacing},
    layout_inputs::DocumentLeading,
    typography::{EffectiveTextStyle, FaceStyle},
    ScriptNote,
};

pub const SCRIPT_NOTE_TITLE_PLACEHOLDER: &str = "Title";
pub const SCRIPT_NOTE_BODY_PLACEHOLDER: &str = "Body";

/// All fixed card geometry is centralized here; values are unscaled logical points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScriptNoteMetrics {
    pub width: FixedPoint,
    pub height: FixedPoint,
    pub top_bar_height: FixedPoint,
    pub title_top_gap: FixedPoint,
    pub title_height: FixedPoint,
    pub body_height: FixedPoint,
    pub footer_height: FixedPoint,
    pub horizontal_inset: FixedPoint,
    pub body_vertical_inset: FixedPoint,
    pub title_trailing_reserve: FixedPoint,
}

impl Default for ScriptNoteMetrics {
    fn default() -> Self {
        Self {
            width: FixedPoint::from_points(296),
            height: FixedPoint::from_points(260),
            top_bar_height: FixedPoint::from_points(20),
            title_top_gap: FixedPoint::from_points(4),
            title_height: FixedPoint::from_points(32),
            body_height: FixedPoint::from_points(176),
            footer_height: FixedPoint::from_points(28),
            horizontal_inset: FixedPoint::from_points(10),
            body_vertical_inset: FixedPoint::default(),
            title_trailing_reserve: FixedPoint::from_points(27),
        }
    }
}

/// Complete card-local measured layout. Line sources are paragraph-local UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNoteLayout {
    pub outer_rect: FixedRect,
    pub top_bar_rect: FixedRect,
    pub title_rect: FixedRect,
    pub body_rect: FixedRect,
    pub footer_rect: FixedRect,
    pub title: LayoutLine,
    pub truncated_title: Option<ScriptNoteTitleProjection>,
    pub body_lines: Vec<LayoutLine>,
    pub body_content_height: FixedPoint,
    pub body_is_empty: bool,
}

/// A display-only glyph with no source range or caret boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNoteDisplayOnlySpan {
    pub text: &'static str,
    pub resolved_text_style: EffectiveTextStyle,
    pub geometry: FixedRect,
}

/// Ordinary truncated title presentation; editing always uses the full title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNoteTitleProjection {
    pub prefix: LayoutLine,
    pub ellipsis: ScriptNoteDisplayOnlySpan,
}

impl ScriptNoteLayout {
    pub fn new(
        note: &ScriptNote,
        environment: &LayoutEnvironment,
        metrics: ScriptNoteMetrics,
    ) -> Result<Self, LayoutError> {
        let width = metrics.width.raw().max(0);
        let height = metrics.height.raw().max(0);
        let top = metrics.top_bar_height.raw().max(0).min(height);
        let title_top = top.saturating_add(metrics.title_top_gap.raw().max(0)).min(height);
        let title_end = title_top.saturating_add(metrics.title_height.raw().max(0)).min(height);
        let body_start = title_end;
        let footer_start = height.saturating_sub(metrics.footer_height.raw().max(0)).max(body_start);
        let inset = metrics.horizontal_inset.raw().max(0).min(width / 2);
        let title_reserve = metrics.title_trailing_reserve.raw().max(0)
            .min(width.saturating_sub(inset.saturating_mul(2)));
        let rect = |x: i64, y: i64, w: i64, h: i64| FixedRect {
            x: FixedPoint::from_raw(x), y: FixedPoint::from_raw(y),
            width: FixedPoint::from_raw(w.max(0)), height: FixedPoint::from_raw(h.max(0)),
        };
        let title_rect = rect(inset, title_top, width.saturating_sub(inset.saturating_mul(2)).saturating_sub(title_reserve), title_end.saturating_sub(title_top));
        let body_top = body_start.saturating_add(metrics.body_vertical_inset.raw().max(0));
        let body_rect = rect(inset, body_top, width.saturating_sub(inset.saturating_mul(2)), footer_start.saturating_sub(body_top).saturating_sub(metrics.body_vertical_inset.raw().max(0)));
        let top_bar_rect = rect(0, 0, width, top);
        let footer_rect = rect(0, footer_start, width, height.saturating_sub(footer_start));
        let outer_rect = rect(0, 0, width, height);

        let mut card_environment = environment.clone();
        card_environment.page.document_leading = DocumentLeading::Normal;
        let title_style = base_style(&card_environment, true)?;
        let body_style = base_style(&card_environment, false)?;
        let title_paragraph = card_paragraph(title_rect, None);
        let mut title_scalars = wrap::project_resolved_display_scalars(
            &note.title, 0..note.title.len(), &[], &title_style, &card_environment.fonts,
        )?;
        let title_face = card_environment.fonts.require_face(&title_style.resolved_font_id)?;
        for scalar in &mut title_scalars {
            if matches!(scalar.character, '\r' | '\n') {
                scalar.character = ' ';
                scalar.advance = title_face.metrics.scale_advance(' ', title_style.size)?;
            }
        }
        let title_line = |count: usize, end: usize| -> Result<LayoutLine, LayoutError> {
            let wrapped = wrap::make_resolved_wrapped_line(
                &note.title, 0, None, title_paragraph, &title_style, &card_environment, &[],
                &title_scalars, ResolvedWrappedLine {
                    scalar_range: 0..count, source_range: 0..end,
                    first_visual_line: true, final_visual_line: true, oversized: false,
                },
            )?;
            Ok(ScreenplayLayout::make_line(0, wrapped, true, title_rect.y))
        };
        let title = title_line(title_scalars.len(), note.title.len())?;
        let truncated_title = if title.geometry.width > title_rect.width {
            let ellipsis_width = title_face.metrics.scale_advance('…', title_style.size)?;
            let available = title_rect.width.raw().saturating_sub(ellipsis_width.raw().max(0)).max(0);
            let mut count = 0;
            let mut retained_width = 0i64;
            while count < title_scalars.len() {
                let source = &title_scalars[count].source_range;
                let mut end = count + 1;
                while end < title_scalars.len() && title_scalars[end].source_range == *source {
                    end += 1;
                }
                let unit_width = title_scalars[count..end].iter().fold(0i64, |sum, scalar| {
                    sum.saturating_add(scalar.advance.raw().max(0))
                });
                if retained_width.saturating_add(unit_width) > available { break; }
                retained_width = retained_width.saturating_add(unit_width);
                count = end;
            }
            let source_end = title_scalars.get(count.wrapping_sub(1))
                .map_or(0, |scalar| scalar.source_range.end);
            let prefix = title_line(count, source_end)?;
            let ellipsis = ScriptNoteDisplayOnlySpan {
                text: "…",
                resolved_text_style: title_style.clone(),
                geometry: FixedRect {
                    x: FixedPoint::from_raw(title_rect.x.raw().saturating_add(retained_width)),
                    y: title_rect.y,
                    width: ellipsis_width,
                    height: prefix.geometry.height,
                },
            };
            Some(ScriptNoteTitleProjection { prefix, ellipsis })
        } else { None };

        let mut body_lines = Vec::new();
        let mut y = body_rect.y;
        for (paragraph_index, paragraph) in note.paragraphs.iter().enumerate() {
            let effective = card_paragraph(body_rect, Some(paragraph.direct_settings.clone()));
            let wrapped = wrap::wrap_resolved_for_card(
                &paragraph.text, paragraph_index, effective, &body_style,
                &card_environment, &paragraph.runs,
            )?;
            for line in wrapped {
                let gap = if line.first_visual_line { effective.space_before } else { FixedPoint::default() };
                y = FixedPoint::from_raw(y.raw().saturating_add(gap.raw()));
                let line = ScreenplayLayout::make_line(0, line, false, y);
                y = FixedPoint::from_raw(y.raw().saturating_add(line.vertical_metrics.leading.raw()));
                body_lines.push(line);
            }
        }
        Ok(Self {
            outer_rect, top_bar_rect, title_rect, body_rect, footer_rect,
            title, truncated_title, body_lines,
            body_content_height: FixedPoint::from_raw(y.raw().saturating_sub(body_rect.y.raw())),
            body_is_empty: note.paragraphs.iter().all(|paragraph| paragraph.text.is_empty()),
        })
    }

    pub fn title_caret_for_x(&self, x: FixedPoint) -> Caret {
        LocatedLayoutLine { page_index: 0, line: &self.title }.fixed_caret_for_x(x)
    }

    pub fn title_caret_geometry(&self, offset: usize) -> Result<FixedRect, LayoutError> {
        LocatedLayoutLine { page_index: 0, line: &self.title }.fixed_caret_geometry(&Caret { element: 0, offset })
    }

    pub fn body_caret_for_x(&self, line_index: usize, x: FixedPoint) -> Option<Caret> {
        self.body_lines.get(line_index).map(|line| LocatedLayoutLine { page_index: 0, line }.fixed_caret_for_x(x))
    }

    pub fn body_caret_geometry(&self, line_index: usize, caret: &Caret) -> Result<FixedRect, LayoutError> {
        let line = self.body_lines.get(line_index).ok_or(LayoutError { kind: crate::geometry::LayoutErrorKind::InvalidPosition })?;
        LocatedLayoutLine { page_index: 0, line }.fixed_caret_geometry(caret)
    }
}

fn card_paragraph(rect: FixedRect, direct: Option<scriptcraft_model::formatting::SourceParagraphOverrides>) -> EffectiveParagraph {
    let direct = direct.unwrap_or_default().paragraph;
    let left = rect.x;
    let right = FixedPoint::from_raw(rect.x.raw().saturating_add(rect.width.raw()));
    EffectiveParagraph {
        left,
        right: if right > left { right } else { FixedPoint::from_raw(left.raw().saturating_add(1)) },
        first_indent: FixedPoint::default(),
        alignment: direct.alignment.unwrap_or(ParagraphAlignment::Left),
        space_before: direct.space_before.unwrap_or_default(),
        space_before_units: 0,
        base_leading: FixedPoint::from_points(12),
        leading: direct.leading.unwrap_or(ParagraphLeading::Regular),
        spacing: direct.spacing.unwrap_or(ParagraphSpacing::Single),
        starts_new_page: false,
        pagination: PaginationPolicy::Ordinary,
    }
}

fn base_style(environment: &LayoutEnvironment, bold: bool) -> Result<EffectiveTextStyle, LayoutError> {
    let face = environment.fonts.resolve_face_id("Arial", FaceStyle::from_flags(bold, false), None)?.clone();
    Ok(EffectiveTextStyle::new("Arial", None, face, FixedPoint::from_points(12), bold, false, false, false, false)?)
}
