use super::{
    geometry::{DisplaySourceSegment, LayoutError, LayoutErrorKind},
    inputs::EffectiveParagraph,
};
use scriptcraft_model::{
    formatted_text::parse_style_attribute,
    formatting::FixedPoint,
    typography::{DocumentFontEnvironment, EffectiveTextStyle as ResolvedTextStyle},
    ElementTypeKey, TextRun,
};
use std::{ops::Range, sync::Arc};

const MAX_LINES_PER_ELEMENT: usize = 1_000_000;

/// One displayed Unicode scalar together with its exact canonical source scalar,
/// resolved typography, decoration, and measured advance. Uppercase expansions
/// intentionally produce multiple records with the same `source_range`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedDisplayScalar {
    pub(super) character: char,
    pub(super) source_range: Range<usize>,
    pub(super) text_style: ResolvedTextStyle,
    pub(super) strikethrough: bool,
    pub(super) advance: FixedPoint,
}

/// Projects canonical source runs through the supplied, already-resolved base
/// style and explicit document font environment. This primitive performs no
/// host discovery and never substitutes a hidden environment.
pub(super) fn project_resolved_display_scalars(
    text: &str,
    source_range: Range<usize>,
    runs: &[TextRun],
    base_style: &ResolvedTextStyle,
    fonts: &DocumentFontEnvironment,
) -> Result<Vec<ResolvedDisplayScalar>, LayoutError> {
    if source_range.start > source_range.end
        || source_range.end > text.len()
        || !text.is_char_boundary(source_range.start)
        || !text.is_char_boundary(source_range.end)
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidPosition,
        });
    }
    let mut run_ranges = Vec::with_capacity(runs.len());
    let mut run_source_cursor = 0usize;
    for run in runs {
        let end = run_source_cursor.saturating_add(run.text.len());
        run_ranges.push((run_source_cursor..end, run));
        run_source_cursor = end;
    }

    let mut output = Vec::new();
    let mut run_index = 0usize;
    for (local_start, source_character) in text[source_range.clone()].char_indices() {
        let absolute_start = source_range.start.saturating_add(local_start);
        let absolute_end = absolute_start.saturating_add(source_character.len_utf8());
        while run_ranges
            .get(run_index)
            .is_some_and(|(range, _)| absolute_start >= range.end)
        {
            run_index = run_index.saturating_add(1);
        }
        let run = run_ranges
            .get(run_index)
            .filter(|(range, _)| absolute_start >= range.start && absolute_start < range.end)
            .map(|(_, run)| *run);
        let (style, strikethrough) = if let Some(run) = run {
            resolve_display_run_style(base_style, run, fonts)?
        } else {
            (base_style.clone(), false)
        };
        let displayed = if style.all_caps {
            source_character.to_uppercase().collect::<String>()
        } else {
            source_character.to_string()
        };
        let face = if matches!(source_character, '\n' | '\r') {
            None
        } else {
            Some(fonts.require_face(&style.resolved_font_id)?)
        };
        for character in displayed.chars() {
            let advance = if matches!(source_character, '\n' | '\r') {
                FixedPoint::default()
            } else {
                face.expect("non-newline scalar has a resolved face")
                    .metrics
                    .scale_advance(character, style.size)?
            };
            output.push(ResolvedDisplayScalar {
                character,
                source_range: absolute_start..absolute_end,
                text_style: style.clone(),
                strikethrough,
                advance,
            });
        }
    }
    Ok(output)
}

fn resolve_display_run_style(
    base: &ResolvedTextStyle,
    run: &TextRun,
    fonts: &DocumentFontEnvironment,
) -> Result<(ResolvedTextStyle, bool), LayoutError> {
    let flags = parse_style_attribute(&run.attributes)
        .map_err(|_| scriptcraft_model::ModelError::InvalidRunProjection)?
        .flags;
    let element_style =
        scriptcraft_model::typography::FaceStyle::from_flags(base.bold, base.italic);
    let mut family = base.declared_family.clone();
    let mut size = base.size;
    let mut color = base.source_color.clone();
    for attribute in &run.attributes {
        match attribute.name.as_str() {
            "Font" if !attribute.value.is_empty() => family.clone_from(&attribute.value),
            "Size" => {
                if let Ok(parsed) = attribute.value.parse::<u16>() {
                    if (1..=scriptcraft_model::typography::MAX_SUPPORTED_FONT_SIZE)
                        .contains(&parsed)
                    {
                        size = parsed;
                    }
                }
            }
            "Color" => color = Some(attribute.value.clone()),
            _ => {}
        }
    }
    let bold = base.bold || flags.bold;
    let italic = base.italic || flags.italics;
    let requested_style = scriptcraft_model::typography::FaceStyle::from_flags(bold, italic);
    let preferred_style = (requested_style != element_style).then_some(element_style);
    let face_id = fonts
        .resolve_face_id(&family, requested_style, preferred_style)?
        .clone();
    let style = ResolvedTextStyle::new(
        family,
        color,
        face_id,
        size,
        bold,
        italic,
        base.underline || flags.underline,
        base.all_caps || flags.all_caps,
        base.hidden_text,
    )?;
    Ok((style, flags.strikethrough))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedWrappedLine {
    /// Scalar indices into the projected display-scalar array.
    pub(super) scalar_range: Range<usize>,
    pub(super) source_range: Range<usize>,
    pub(super) first_visual_line: bool,
    pub(super) final_visual_line: bool,
    pub(super) oversized: bool,
}

/// Wraps resolved display scalars by their measured advances. Expansions that
/// share one canonical source range are indivisible; every nonempty line makes
/// progress even when its first atomic unit exceeds the available width.
pub(super) fn wrap_resolved_display_scalars(
    scalars: &[ResolvedDisplayScalar],
    paragraph: EffectiveParagraph,
    first_visual_line: bool,
    line_offset: usize,
    empty_source_offset: usize,
) -> Vec<ResolvedWrappedLine> {
    let first_left = paragraph
        .left
        .raw()
        .saturating_add(paragraph.first_indent.raw());
    let first_limit = paragraph.right.raw().saturating_sub(first_left).max(1);
    let rest_limit = paragraph
        .right
        .raw()
        .saturating_sub(paragraph.left.raw())
        .max(1);
    let mut lines = Vec::new();
    let mut paragraph_start = 0usize;
    let mut cursor = 0usize;
    let mut first_visual_line = first_visual_line;
    let mut empty_source_offset = empty_source_offset;
    while cursor < scalars.len() {
        if matches!(scalars[cursor].character, '\n' | '\r') {
            wrap_resolved_segment(
                scalars,
                paragraph_start..cursor,
                first_limit,
                rest_limit,
                &mut first_visual_line,
                line_offset,
                empty_source_offset,
                &mut lines,
            );
            if scalars[cursor].character == '\r'
                && scalars
                    .get(cursor + 1)
                    .is_some_and(|scalar| scalar.character == '\n')
            {
                cursor += 1;
            }
            empty_source_offset = scalars[cursor].source_range.end;
            cursor += 1;
            paragraph_start = cursor;
        } else {
            cursor += 1;
        }
    }
    wrap_resolved_segment(
        scalars,
        paragraph_start..scalars.len(),
        first_limit,
        rest_limit,
        &mut first_visual_line,
        line_offset,
        empty_source_offset,
        &mut lines,
    );
    lines
}

/// Projects, measures, wraps, and retains the exact scalar payload for one
/// canonical Body paragraph. All source offsets remain byte offsets into the
/// original UTF-8 text; display expansions never create fictitious source bytes.
#[allow(clippy::too_many_arguments)]
pub(super) fn wrap_resolved_for_layout(
    text: &str,
    element: usize,
    element_type: Option<ElementTypeKey>,
    paragraph: EffectiveParagraph,
    base_style: &ResolvedTextStyle,
    fonts: &DocumentFontEnvironment,
    runs: &[TextRun],
    first_visual_line: bool,
    line_offset: usize,
    empty_source_offset: usize,
) -> Result<Vec<WrappedLine>, LayoutError> {
    let scalars = project_resolved_display_scalars(text, 0..text.len(), runs, base_style, fonts)?;
    let ranges = wrap_resolved_display_scalars(
        &scalars,
        paragraph,
        first_visual_line,
        line_offset,
        empty_source_offset,
    );
    Ok(ranges
        .into_iter()
        .map(|range| {
            let selected = &scalars[range.scalar_range.clone()];
            let mut display_text = String::new();
            let mut display_source_map: Vec<DisplaySourceSegment> = Vec::new();
            for (display_index, scalar) in selected.iter().enumerate() {
                display_text.push(scalar.character);
                if let Some(previous) = display_source_map.last_mut() {
                    if previous.source_start == scalar.source_range.start
                        && previous.source_end == scalar.source_range.end
                    {
                        previous.display_end = display_index.saturating_add(1);
                        continue;
                    }
                }
                display_source_map.push(DisplaySourceSegment {
                    display_start: display_index,
                    display_end: display_index.saturating_add(1),
                    source_start: scalar.source_range.start,
                    source_end: scalar.source_range.end,
                });
            }
            let source = range.source_range;
            WrappedLine {
                element,
                element_type: element_type.clone(),
                start: source.start,
                end: source.end,
                text: Arc::from(&text[source.clone()]),
                display_text: Arc::from(display_text),
                display_source_map: display_source_map.into(),
                resolved_scalars: Arc::from(selected),
                runs: Arc::from(runs.to_vec()),
                first_visual_line: range.first_visual_line,
                final_visual_line: range.final_visual_line,
                paragraph,
                oversized: range.oversized,
            }
        })
        .collect())
}

#[derive(Debug, Clone)]
pub(super) struct WrappedLine {
    pub(super) element: usize,
    pub(super) element_type: Option<ElementTypeKey>,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) text: Arc<str>,
    pub(super) display_text: Arc<str>,
    pub(super) display_source_map: Arc<[DisplaySourceSegment]>,
    pub(super) resolved_scalars: Arc<[ResolvedDisplayScalar]>,
    pub(super) runs: Arc<[TextRun]>,
    pub(super) first_visual_line: bool,
    pub(super) final_visual_line: bool,
    pub(super) paragraph: EffectiveParagraph,
    pub(super) oversized: bool,
}

#[allow(clippy::too_many_arguments)]
fn wrap_resolved_segment(
    scalars: &[ResolvedDisplayScalar],
    segment: Range<usize>,
    first_limit: i64,
    rest_limit: i64,
    first_visual_line: &mut bool,
    line_offset: usize,
    empty_source_offset: usize,
    output: &mut Vec<ResolvedWrappedLine>,
) {
    if segment.is_empty() {
        let source = scalars
            .get(segment.start)
            .map_or(empty_source_offset, |scalar| scalar.source_range.start);
        output.push(ResolvedWrappedLine {
            scalar_range: segment.clone(),
            source_range: source..source,
            first_visual_line: *first_visual_line,
            final_visual_line: true,
            oversized: false,
        });
        *first_visual_line = false;
        return;
    }
    let mut start = segment.start;
    while start < segment.end && line_offset.saturating_add(output.len()) < MAX_LINES_PER_ELEMENT {
        let first_line = *first_visual_line;
        let limit = if first_line { first_limit } else { rest_limit };
        let mut end = start;
        let mut width = 0i64;
        let mut last_whitespace_end = None;
        while end < segment.end {
            let source = scalars[end].source_range.clone();
            let mut unit_end = end + 1;
            while unit_end < segment.end && scalars[unit_end].source_range == source {
                unit_end += 1;
            }
            let unit_width = scalars[end..unit_end]
                .iter()
                .map(|scalar| scalar.advance.raw().max(0))
                .fold(0i64, i64::saturating_add);
            if end > start && width.saturating_add(unit_width) > limit {
                break;
            }
            width = width.saturating_add(unit_width);
            if scalars[end..unit_end]
                .iter()
                .any(|scalar| scalar.character.is_whitespace())
            {
                last_whitespace_end = Some(unit_end);
            }
            end = unit_end;
        }
        if end < segment.end {
            if let Some(whitespace_end) = last_whitespace_end.filter(|end| *end > start) {
                end = whitespace_end;
            }
        }
        if end <= start {
            end = (start + 1).min(segment.end);
            while end < segment.end && scalars[end].source_range == scalars[start].source_range {
                end += 1;
            }
        }
        let source_start = scalars[start].source_range.start;
        let source_end = scalars[end - 1].source_range.end;
        let actual_width = scalars[start..end]
            .iter()
            .map(|scalar| scalar.advance.raw().max(0))
            .fold(0i64, i64::saturating_add);
        output.push(ResolvedWrappedLine {
            scalar_range: start..end,
            source_range: source_start..source_end,
            first_visual_line: first_line,
            final_visual_line: end == segment.end,
            oversized: actual_width > limit,
        });
        *first_visual_line = false;
        start = end;
    }
}
