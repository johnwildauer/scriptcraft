use super::{
    cache::ElementWrapCache,
    geometry::{DisplaySourceSegment, LayoutError, LayoutErrorKind},
    inputs::{EffectiveGeometryMode, EffectiveParagraph, LayoutPurpose},
    LayoutBuildDiagnostics,
};
use scriptcraft_model::{
    formatted_text::parse_style_attribute,
    formatting::{EffectiveTextStyle, FixedPoint},
    typography::{DocumentFontEnvironment, EffectiveTextStyle as ResolvedTextStyle},
    ElementTypeKey, Role, StableId, TextRun,
};
use std::{ops::Range, sync::Arc, time::Instant};

const MAX_LINES_PER_ELEMENT: usize = 1_000_000;

/// One displayed Unicode scalar together with its exact canonical source scalar,
/// resolved typography, decoration, and measured advance. Uppercase expansions
/// intentionally produce multiple records with the same `source_range`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Consumed by the sequential-builder migration in WO-005A.2.
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
#[allow(dead_code)] // Exposed internally for the next bounded builder work order.
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
#[allow(dead_code)] // Consumed by the sequential-builder migration in WO-005A.2.
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
#[allow(dead_code)] // Introduced independently before builder routing by contract.
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
    role: Role,
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
    let compatibility_style = legacy_style_metadata(base_style);
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
                role,
                start: source.start,
                end: source.end,
                text: Arc::from(&text[source.clone()]),
                display_text: Arc::from(display_text),
                display_source_map: display_source_map.into(),
                text_style: compatibility_style.clone(),
                resolved_text_style: Some(base_style.clone()),
                resolved_scalars: Arc::from(selected),
                runs: Arc::from(runs.to_vec()),
                char_width: FixedPoint::default(),
                first_visual_line: range.first_visual_line,
                final_visual_line: range.final_visual_line,
                paragraph,
                oversized: range.oversized,
            }
        })
        .collect())
}

fn legacy_style_metadata(style: &ResolvedTextStyle) -> EffectiveTextStyle {
    EffectiveTextStyle {
        declared_family: style.declared_family.clone(),
        // Compatibility-only metadata: the resolved font ID on this line and
        // its spans is the sole font authority for metric-driven Body geometry.
        family: scriptcraft_model::formatting::FontFallback::CourierPrime,
        size: style.size.min(u16::from(u8::MAX)) as u8,
        bold: style.bold,
        italic: style.italic,
        underline: style.underline,
        all_caps: style.all_caps,
        hidden_text: style.hidden_text,
        source_color: style.source_color.clone(),
    }
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

#[derive(Debug, Clone)]
pub(super) struct WrappedLine {
    pub(super) element: usize,
    pub(super) role: Role,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) text: Arc<str>,
    pub(super) display_text: Arc<str>,
    pub(super) display_source_map: Arc<[DisplaySourceSegment]>,
    pub(super) text_style: EffectiveTextStyle,
    pub(super) resolved_text_style: Option<ResolvedTextStyle>,
    pub(super) resolved_scalars: Arc<[ResolvedDisplayScalar]>,
    pub(super) runs: Arc<[TextRun]>,
    pub(super) char_width: FixedPoint,
    pub(super) first_visual_line: bool,
    pub(super) final_visual_line: bool,
    pub(super) paragraph: EffectiveParagraph,
    pub(super) oversized: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn wrap_for_layout(
    cache: Option<&mut ElementWrapCache>,
    diagnostics: Option<&mut LayoutBuildDiagnostics>,
    element_id: &StableId,
    text: &str,
    element: usize,
    role: Role,
    element_key: Option<&ElementTypeKey>,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    runs: &[TextRun],
    char_width: FixedPoint,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
) -> Vec<WrappedLine> {
    wrap_for_layout_range(
        cache,
        diagnostics,
        element_id,
        text,
        0..text.len(),
        true,
        0,
        element,
        role,
        element_key,
        paragraph,
        text_style,
        runs,
        char_width,
        purpose,
        title_page,
        geometry_mode,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn wrap_for_layout_range(
    cache: Option<&mut ElementWrapCache>,
    diagnostics: Option<&mut LayoutBuildDiagnostics>,
    element_id: &StableId,
    text: &str,
    source_range: std::ops::Range<usize>,
    first_visual_line: bool,
    line_offset: usize,
    element: usize,
    role: Role,
    element_key: Option<&ElementTypeKey>,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    runs: &[TextRun],
    char_width: FixedPoint,
    purpose: LayoutPurpose,
    title_page: bool,
    geometry_mode: EffectiveGeometryMode,
) -> Vec<WrappedLine> {
    let mut projected_text = String::new();
    let mut source_map = Vec::<(std::ops::Range<usize>, std::ops::Range<usize>)>::new();
    let mut run_styles = Vec::with_capacity(runs.len());
    let mut source_cursor = 0usize;
    for run in runs {
        let end = source_cursor.saturating_add(run.text.len());
        let flags = parse_style_attribute(&run.attributes)
            .map(|projection| projection.flags)
            .unwrap_or_default();
        run_styles.push((source_cursor..end, flags));
        source_cursor = end;
    }
    let base_size = i64::from(text_style.size.max(1));
    let maximum_run_size = runs
        .iter()
        .flat_map(|run| run.attributes.iter())
        .filter(|attribute| attribute.name == "Size")
        .filter_map(|attribute| attribute.value.parse::<u8>().ok())
        .filter(|size| (1..=72).contains(size))
        .map(i64::from)
        .max()
        .unwrap_or(base_size)
        .max(base_size);
    // Wrapping uses a conservative largest direct run metric; span placement
    // below restores each run's own fixed-cell width without host font advances.
    let wrapping_char_width = FixedPoint::from_raw(
        char_width
            .raw()
            .saturating_mul(maximum_run_size)
            .div_euclid(base_size)
            .max(1),
    );
    let mut run_index = 0usize;
    for (local_start, scalar) in text[source_range.clone()].char_indices() {
        let source_start = source_range.start.saturating_add(local_start);
        let source_end = source_start + scalar.len_utf8();
        while run_styles
            .get(run_index)
            .is_some_and(|(range, _)| source_start >= range.end)
        {
            run_index = run_index.saturating_add(1);
        }
        let flags = run_styles
            .get(run_index)
            .filter(|(range, _)| source_start >= range.start && source_start < range.end)
            .map(|(_, flags)| *flags)
            .unwrap_or_default();
        let displayed = if text_style.all_caps || flags.all_caps {
            scalar.to_uppercase().collect::<String>()
        } else {
            scalar.to_string()
        };
        let start = projected_text.len();
        projected_text.push_str(&displayed);
        source_map.push((start..projected_text.len(), source_start..source_end));
    }
    // The complete ranges remain cache identity; only genuine display expansions
    // need an interior-boundary lookup. The noncache path avoids the full temporary.
    let atomic_ranges = cache.is_some().then(|| {
        source_map
            .iter()
            .map(|(display, _)| display.clone())
            .collect::<Vec<_>>()
    });
    let atomic_boundaries = atomic_boundary_index(&projected_text, &source_map);
    let mut lines = if let Some(cache) = cache {
        cache.wrap(
            element_id,
            element_key,
            &projected_text,
            element,
            role,
            paragraph,
            {
                let mut wrapping_style = text_style.clone();
                wrapping_style.all_caps = false;
                wrapping_style
            },
            wrapping_char_width,
            runs,
            atomic_ranges
                .as_deref()
                .expect("cached wrapping retains complete ranges"),
            &atomic_boundaries,
            purpose,
            title_page,
            geometry_mode,
            diagnostics,
        )
    } else {
        let started = diagnostics.as_ref().map(|_| Instant::now());
        let mut wrapping_style = text_style.clone();
        wrapping_style.all_caps = false;
        let lines = wrap_element(
            &projected_text,
            element,
            role,
            paragraph,
            wrapping_style,
            wrapping_char_width,
            &atomic_boundaries,
            first_visual_line,
            line_offset,
        );
        if let (Some(diagnostics), Some(started)) = (diagnostics, started) {
            diagnostics.record_element_wrap(started);
        }
        lines
    };
    let owner_runs: Arc<[TextRun]> = Arc::from(runs.to_vec());
    let mut line_map_index = 0usize;
    let mut segment_map_index = 0usize;
    for line in &mut lines {
        let projected_start = line.start;
        let projected_end = line.end;
        let mapped_start =
            source_start_for_display_offset(&source_map, &mut line_map_index, projected_start)
                .unwrap_or(text.len());
        let mapped_end =
            source_end_for_display_offset(&source_map, &mut line_map_index, projected_end)
                .unwrap_or(mapped_start);
        line.start = mapped_start;
        line.end = mapped_end;
        line.text = Arc::from(&text[mapped_start..mapped_end]);
        for segment in Arc::make_mut(&mut line.display_source_map) {
            let start = source_start_for_display_offset(
                &source_map,
                &mut segment_map_index,
                segment.source_start,
            )
            .unwrap_or(mapped_start);
            let end = source_end_for_display_offset(
                &source_map,
                &mut segment_map_index,
                segment.source_end,
            )
            .unwrap_or(start);
            segment.source_start = start;
            segment.source_end = end;
        }
        line.text_style = text_style.clone();
        line.char_width = char_width;
        line.runs = Arc::clone(&owner_runs);
    }
    lines
}

fn source_start_for_display_offset(
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
    cursor: &mut usize,
    display_offset: usize,
) -> Option<usize> {
    while source_map
        .get(*cursor)
        .is_some_and(|(display, _)| display_offset >= display.end)
    {
        *cursor = (*cursor).saturating_add(1);
    }
    source_map
        .get(*cursor)
        .filter(|(display, _)| display_offset >= display.start && display_offset < display.end)
        .map(|(_, source)| source.start)
}

fn source_end_for_display_offset(
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
    cursor: &mut usize,
    display_offset: usize,
) -> Option<usize> {
    while source_map
        .get(*cursor)
        .is_some_and(|(display, _)| display_offset > display.end)
    {
        *cursor = (*cursor).saturating_add(1);
    }
    source_map
        .get(*cursor)
        .filter(|(display, _)| display_offset > display.start && display_offset <= display.end)
        .map(|(_, source)| source.end)
}

fn atomic_boundary_index(
    text: &str,
    source_map: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
) -> Vec<std::ops::Range<usize>> {
    let mut index = Vec::new();
    for (display, _) in source_map {
        if text[display.clone()].char_indices().nth(1).is_some() {
            index.push(display.clone());
        }
    }
    index
}

fn atomic_boundary_is_forbidden(
    atomic_boundaries: &[std::ops::Range<usize>],
    boundary: usize,
) -> bool {
    let candidate = atomic_boundaries.partition_point(|range| range.end <= boundary);
    atomic_boundaries
        .get(candidate)
        .is_some_and(|range| range.start < boundary)
}

pub(super) fn wrap_element(
    text: &str,
    element: usize,
    role: Role,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    atomic_boundaries: &[std::ops::Range<usize>],
    first_visual_line: bool,
    line_offset: usize,
) -> Vec<WrappedLine> {
    let advances = vec![char_width; text.chars().count()];
    wrap_element_with_advances(
        text,
        element,
        role,
        paragraph,
        text_style,
        char_width,
        &advances,
        atomic_boundaries,
        first_visual_line,
        line_offset,
    )
}

/// Metric-driven wrapping primitive. `scalar_advances` contains one canonical
/// advance per input scalar; all width decisions use their sum rather than a
/// character count. The resolved caller supplies the exact effective advance
/// for each displayed scalar, including all-caps expansions.
#[allow(clippy::too_many_arguments)]
pub(super) fn wrap_element_with_advances(
    text: &str,
    element: usize,
    role: Role,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    scalar_advances: &[FixedPoint],
    atomic_boundaries: &[std::ops::Range<usize>],
    first_visual_line: bool,
    line_offset: usize,
) -> Vec<WrappedLine> {
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
    let mut output = Vec::new();
    let mut first_visual_line = first_visual_line;
    let mut paragraph_start = 0;
    let mut delimiters = text.char_indices().peekable();
    let mut scalar_offset = 0usize;
    while let Some((newline, delimiter)) = delimiters.next() {
        if delimiter != '\n' && delimiter != '\r' {
            continue;
        }
        let segment_scalars = text[paragraph_start..newline].chars().count();
        wrap_segment(
            &text[paragraph_start..newline],
            paragraph_start,
            element,
            role,
            first_limit,
            rest_limit,
            paragraph,
            &text_style,
            char_width,
            scalar_advances
                .get(scalar_offset..scalar_offset.saturating_add(segment_scalars))
                .unwrap_or(&[]),
            atomic_boundaries,
            &mut first_visual_line,
            line_offset,
            &mut output,
        );
        scalar_offset = scalar_offset.saturating_add(segment_scalars);
        paragraph_start = newline + delimiter.len_utf8();
        scalar_offset = scalar_offset.saturating_add(1);
        if delimiter == '\r' && delimiters.peek().is_some_and(|(_, next)| *next == '\n') {
            let _ = delimiters.next();
            paragraph_start += 1;
            scalar_offset = scalar_offset.saturating_add(1);
        }
    }
    wrap_segment(
        text[paragraph_start..].as_ref(),
        paragraph_start,
        element,
        role,
        first_limit,
        rest_limit,
        paragraph,
        &text_style,
        char_width,
        scalar_advances.get(scalar_offset..).unwrap_or(&[]),
        atomic_boundaries,
        &mut first_visual_line,
        line_offset,
        &mut output,
    );
    if output.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: 0,
            end: 0,
            text: Arc::from(""),
            display_text: Arc::from(""),
            display_source_map: Arc::from([]),
            text_style,
            resolved_text_style: None,
            resolved_scalars: Arc::from([]),
            runs: Arc::from([]),
            char_width,
            first_visual_line,
            final_visual_line: true,
            paragraph,
            oversized: false,
        });
    }
    output
}

#[derive(Debug, Clone, Copy)]
struct DisplayCell {
    character: char,
    local_start: usize,
    local_end: usize,
    source_start: usize,
    source_end: usize,
    advance: FixedPoint,
}

// Segment wrapping keeps source ranges and layout constraints explicit at the
// call boundary; consolidating them into state would obscure their invariants.
#[allow(clippy::too_many_arguments)]
fn wrap_segment(
    text: &str,
    base: usize,
    element: usize,
    role: Role,
    first_limit: i64,
    rest_limit: i64,
    paragraph: EffectiveParagraph,
    text_style: &EffectiveTextStyle,
    char_width: FixedPoint,
    scalar_advances: &[FixedPoint],
    atomic_boundaries: &[std::ops::Range<usize>],
    first_visual_line: &mut bool,
    line_offset: usize,
    output: &mut Vec<WrappedLine>,
) {
    if text.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: base,
            end: base,
            text: Arc::from(""),
            display_text: Arc::from(""),
            display_source_map: Arc::from([]),
            text_style: text_style.clone(),
            resolved_text_style: None,
            resolved_scalars: Arc::from([]),
            runs: Arc::from([]),
            char_width,
            first_visual_line: *first_visual_line,
            final_visual_line: true,
            paragraph,
            oversized: false,
        });
        *first_visual_line = false;
        return;
    }
    let mut cells = Vec::new();
    for (scalar_index, (local_start, source_character)) in text.char_indices().enumerate() {
        let local_end = local_start + source_character.len_utf8();
        let source_start = base + local_start;
        let source_end = base + local_end;
        let advance = scalar_advances
            .get(scalar_index)
            .copied()
            .unwrap_or(char_width);
        if text_style.all_caps {
            cells.extend(
                source_character
                    .to_uppercase()
                    .map(|character| DisplayCell {
                        character,
                        local_start,
                        local_end,
                        source_start,
                        source_end,
                        advance,
                    }),
            );
        } else {
            cells.push(DisplayCell {
                character: source_character,
                local_start,
                local_end,
                source_start,
                source_end,
                advance,
            });
        }
    }
    let mut start = 0;
    while start < cells.len() && line_offset.saturating_add(output.len()) < MAX_LINES_PER_ELEMENT {
        let first_line = *first_visual_line;
        let limit = if first_line { first_limit } else { rest_limit };
        let mut hard_end = start;
        let mut measured_width = 0i64;
        while hard_end < cells.len() {
            let source_start = cells[hard_end].source_start;
            let mut candidate_end = hard_end.saturating_add(1);
            while candidate_end < cells.len() && cells[candidate_end].source_start == source_start {
                candidate_end = candidate_end.saturating_add(1);
            }
            let candidate_width = cells[hard_end..candidate_end]
                .iter()
                .map(|cell| cell.advance.raw().max(0))
                .fold(0i64, i64::saturating_add);
            if hard_end > start && measured_width.saturating_add(candidate_width) > limit {
                break;
            }
            measured_width = measured_width.saturating_add(candidate_width);
            hard_end = candidate_end;
            while hard_end < cells.len() && hard_end > start {
                let boundary = base.saturating_add(cells[hard_end].local_start);
                if atomic_boundary_is_forbidden(atomic_boundaries, boundary) {
                    measured_width =
                        measured_width.saturating_add(cells[hard_end].advance.raw().max(0));
                    hard_end = hard_end.saturating_add(1);
                } else {
                    break;
                }
            }
        }
        let mut end = hard_end;
        if hard_end < cells.len() {
            if let Some(space) = (start..hard_end)
                .rev()
                .find(|index| cells[*index].character.is_whitespace())
            {
                end = space + 1;
            }
        }
        if end <= start {
            end = (start + 1).min(cells.len());
        }
        let selected = &cells[start..end];
        let first_cell = selected.first().copied().unwrap_or(DisplayCell {
            character: '\0',
            local_start: 0,
            local_end: 0,
            source_start: base,
            source_end: base,
            advance: char_width,
        });
        let last_cell = selected.last().copied().unwrap_or(first_cell);
        let source_start = first_cell.source_start;
        let source_end = last_cell.source_end;
        let mut display_text = String::new();
        let mut display_source_map: Vec<DisplaySourceSegment> = Vec::new();
        for (display_index, cell) in selected.iter().enumerate() {
            display_text.push(cell.character);
            if let Some(previous) = display_source_map.last_mut() {
                if previous.source_start == cell.source_start
                    && previous.source_end == cell.source_end
                {
                    previous.display_end = display_index + 1;
                    continue;
                }
            }
            display_source_map.push(DisplaySourceSegment {
                display_start: display_index,
                display_end: display_index + 1,
                source_start: cell.source_start,
                source_end: cell.source_end,
            });
        }
        output.push(WrappedLine {
            element,
            role,
            start: source_start,
            end: source_end,
            text: Arc::from(&text[first_cell.local_start..last_cell.local_end]),
            display_text: Arc::from(display_text),
            display_source_map: display_source_map.into(),
            text_style: text_style.clone(),
            resolved_text_style: None,
            resolved_scalars: Arc::from([]),
            runs: Arc::from([]),
            char_width,
            first_visual_line: first_line,
            final_visual_line: end == cells.len(),
            paragraph,
            oversized: selected
                .iter()
                .map(|cell| cell.advance.raw().max(0))
                .fold(0i64, i64::saturating_add)
                > limit,
        });
        *first_visual_line = false;
        start = end;
    }
}

#[cfg(test)]
mod metric_wrap_tests {
    use super::*;
    use crate::inputs::PaginationPolicy;
    use scriptcraft_model::{
        formatting::{FontFallback, ParagraphAlignment},
        typography::{
            DocumentFontEnvironment, EffectiveTextColor, FaceStyle, FontResolutionKind,
            RequestedFontResolution, ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId,
            ResolvedFontSourceKind, ScalarAdvance,
        },
        RunAttribute,
    };
    use std::collections::BTreeMap;

    fn paragraph(width: i64) -> EffectiveParagraph {
        EffectiveParagraph {
            left: FixedPoint::default(),
            right: FixedPoint::from_raw(width),
            first_indent: FixedPoint::default(),
            alignment: ParagraphAlignment::Left,
            space_before: FixedPoint::default(),
            line_advance: FixedPoint::from_points(12),
            char_width: FixedPoint::default(),
            starts_new_page: false,
            pagination: PaginationPolicy::Ordinary,
        }
    }

    fn style() -> EffectiveTextStyle {
        EffectiveTextStyle {
            declared_family: "Synthetic".to_owned(),
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

    #[test]
    fn wraps_by_the_sum_of_explicit_scalar_advances() {
        let text = "abc";
        let advances = [
            FixedPoint::from_raw(600),
            FixedPoint::from_raw(100),
            FixedPoint::from_raw(600),
        ];
        let lines = wrap_element_with_advances(
            text,
            0,
            Role::Action,
            paragraph(1_000),
            style(),
            FixedPoint::from_points(7),
            &advances,
            &[],
            true,
            0,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].display_text.as_ref(), "ab");
        assert_eq!(lines[1].display_text.as_ref(), "c");
        assert_eq!((lines[0].start, lines[0].end), (0, 2));
        assert_eq!((lines[1].start, lines[1].end), (2, 3));
    }

    #[test]
    fn oversized_scalar_is_emitted_once_and_marked() {
        let lines = wrap_element_with_advances(
            "界",
            0,
            Role::Action,
            paragraph(100),
            style(),
            FixedPoint::from_points(7),
            &[FixedPoint::from_raw(500)],
            &[],
            true,
            0,
        );
        assert_eq!(lines.len(), 1);
        assert!(lines[0].oversized);
        assert_eq!(lines[0].display_text.as_ref(), "界");
    }

    fn synthetic_environment() -> DocumentFontEnvironment {
        let mut requests = BTreeMap::new();
        let mut faces = BTreeMap::new();
        for family in ["Alpha", "Beta", "Courier Prime", "Futura"] {
            let ids: Vec<_> = FaceStyle::ALL
                .into_iter()
                .map(|style| ResolvedFontId::new(format!("{family}-{style:?}")).unwrap())
                .collect();
            requests.insert(
                family.to_owned(),
                RequestedFontResolution {
                    kind: if family == "Courier Prime" {
                        FontResolutionKind::CourierAlias
                    } else {
                        FontResolutionKind::NearestSans
                    },
                    regular: ids[0].clone(),
                    bold: ids[1].clone(),
                    italic: ids[2].clone(),
                    bold_italic: if family == "Futura" {
                        ids[1].clone()
                    } else {
                        ids[3].clone()
                    },
                },
            );
            for (style, id) in FaceStyle::ALL.into_iter().zip(ids) {
                if family == "Futura" && style == FaceStyle::BoldItalic {
                    continue;
                }
                let scalar_advances = [
                    ('A', 500),
                    ('S', 700),
                    ('a', 200),
                    ('b', 300),
                    ('c', 600),
                    (' ', 250),
                ]
                .into_iter()
                .map(|(scalar, advance_units)| {
                    (
                        scalar,
                        ScalarAdvance {
                            advance_units,
                            missing_glyph: false,
                        },
                    )
                })
                .collect();
                let metrics =
                    ResolvedFaceMetrics::new(1_000, 800, -200, 0, scalar_advances).unwrap();
                faces.insert(
                    id.clone(),
                    ResolvedFontFace::new(
                        id,
                        family,
                        format!("{family} {style:?}"),
                        style,
                        ResolvedFontSourceKind::Synthetic,
                        128,
                        1,
                        metrics,
                    )
                    .unwrap(),
                );
            }
        }
        DocumentFontEnvironment::new(requests, faces).unwrap()
    }

    fn resolved_style(family: &str, bold: bool) -> ResolvedTextStyle {
        let id = ResolvedFontId::new(format!(
            "{family}-{}",
            if bold { "Bold" } else { "Regular" }
        ))
        .unwrap();
        ResolvedTextStyle::new(family, None, id, 12, bold, false, false, false, false).unwrap()
    }

    fn run(text: &str, attributes: &[(&str, &str)]) -> TextRun {
        TextRun {
            text: text.to_owned(),
            attributes: attributes
                .iter()
                .map(|(name, value)| RunAttribute {
                    name: (*name).to_owned(),
                    value: (*value).to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn projection_preserves_canonical_ranges_and_resolves_direct_run_overlays() {
        let environment = synthetic_environment();
        let text = "aßb";
        let overlay = [
            ("Font", "Beta"),
            ("Size", "18"),
            ("Color", "#FFFF00000000"),
            ("Style", "Bold+Underline+AllCaps+Strikethrough"),
        ];
        let runs = [run("a", &overlay), run("ß", &overlay), run("b", &[])];
        let scalars = project_resolved_display_scalars(
            text,
            0..text.len(),
            &runs,
            &resolved_style("Alpha", false),
            &environment,
        )
        .unwrap();

        assert_eq!(
            scalars
                .iter()
                .map(|scalar| scalar.character)
                .collect::<String>(),
            "ASSb"
        );
        assert_eq!(scalars[0].source_range, 0..1);
        assert_eq!(scalars[1].source_range, 1..3);
        assert_eq!(scalars[2].source_range, 1..3);
        assert_eq!(scalars[3].source_range, 3..4);
        for scalar in &scalars[..3] {
            assert_eq!(scalar.text_style.declared_family, "Beta");
            assert_eq!(scalar.text_style.resolved_font_id.as_str(), "Beta-Bold");
            assert_eq!(scalar.text_style.size, 18);
            assert!(scalar.text_style.bold && scalar.text_style.underline);
            assert!(scalar.text_style.all_caps);
            assert_eq!(
                scalar.text_style.color,
                EffectiveTextColor::ExplicitRgb8 {
                    red: 255,
                    green: 0,
                    blue: 0
                }
            );
            assert!(scalar.strikethrough);
            assert_eq!(
                scalar.advance,
                FixedPoint::from_raw(if scalar.character == 'S' { 1_260 } else { 900 })
            );
        }
        assert_eq!(scalars[3].text_style.declared_family, "Alpha");
        assert!(!scalars[3].text_style.bold);
        assert_eq!(scalars[3].advance, FixedPoint::from_raw(360));
    }

    #[test]
    fn display_style_override_falls_back_to_the_elements_face_before_family_priority() {
        let environment = synthetic_environment();
        let base = ResolvedTextStyle::new(
            "Futura",
            None,
            ResolvedFontId::new("Futura-Italic").unwrap(),
            18,
            false,
            true,
            false,
            false,
            false,
        )
        .unwrap();
        let run = run("sample", &[("Style", "Bold+Italic")]);
        let resolved = resolve_display_run_style(&base, &run, &environment).unwrap();
        assert!(resolved.0.bold && resolved.0.italic);
        assert_eq!(resolved.0.resolved_font_id.as_str(), "Futura-Italic");

        let family_fallback = environment
            .resolve_face_id("Futura", FaceStyle::BoldItalic, None)
            .unwrap();
        assert_eq!(family_fallback.as_str(), "Futura-Bold");
    }

    #[test]
    fn projection_rejects_noncanonical_source_range_boundaries() {
        let environment = synthetic_environment();
        let error = project_resolved_display_scalars(
            "é",
            1..2,
            &[],
            &resolved_style("Alpha", false),
            &environment,
        )
        .unwrap_err();
        assert_eq!(error.kind, LayoutErrorKind::InvalidPosition);
    }

    #[test]
    fn resolved_wrap_honors_hanging_indent_whitespace_and_atomic_expansion() {
        let environment = synthetic_environment();
        let text = "a b";
        let scalars = project_resolved_display_scalars(
            text,
            0..text.len(),
            &[run("a b", &[])],
            &resolved_style("Alpha", false),
            &environment,
        )
        .unwrap();
        // With a 100-raw-unit first indent, the first 800-unit measure breaks
        // after the whitespace; later lines retain the full 900-unit measure.
        let mut paragraph_value = paragraph(900);
        paragraph_value.first_indent = FixedPoint::from_raw(100);
        let lines = wrap_resolved_display_scalars(&scalars, paragraph_value, true, 0, 0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].source_range, 0..2);
        assert_eq!(lines[1].source_range, 2..3);
        assert!(lines[0].first_visual_line);
        assert!(!lines[1].first_visual_line);

        let expanded = project_resolved_display_scalars(
            "ß",
            0..2,
            &[run("ß", &[("Style", "AllCaps")])],
            &resolved_style("Alpha", false),
            &environment,
        )
        .unwrap();
        let mut narrow = paragraph(100);
        narrow.first_indent = FixedPoint::default();
        let lines = wrap_resolved_display_scalars(&expanded, narrow, true, 0, 0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].scalar_range, 0..2);
        assert_eq!(lines[0].source_range, 0..2);
        assert!(lines[0].oversized);
    }

    #[test]
    fn resolved_wrap_preserves_empty_paragraph_source_boundaries() {
        let environment = synthetic_environment();
        let text = "a\n\n";
        let scalars = project_resolved_display_scalars(
            text,
            0..text.len(),
            &[run(text, &[])],
            &resolved_style("Alpha", false),
            &environment,
        )
        .unwrap();
        let lines = wrap_resolved_display_scalars(&scalars, paragraph(800), true, 0, 0);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].source_range, 0..1);
        assert_eq!(lines[1].source_range, 2..2);
        assert_eq!(lines[2].source_range, 3..3);
    }
}
