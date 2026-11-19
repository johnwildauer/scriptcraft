use super::{
    cache::ElementWrapCache,
    geometry::DisplaySourceSegment,
    inputs::{EffectiveGeometryMode, EffectiveParagraph, LayoutPurpose},
    LayoutBuildDiagnostics,
};
use scriptcraft_model::{
    formatted_text::parse_style_attribute,
    formatting::{EffectiveTextStyle, FixedPoint},
    ElementTypeKey, Role, StableId, TextRun,
};
use std::{sync::Arc, time::Instant};

const MAX_LINES_PER_ELEMENT: usize = 1_000_000;

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
    use scriptcraft_model::formatting::{FontFallback, ParagraphAlignment};

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
}
