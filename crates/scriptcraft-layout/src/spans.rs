use super::{
    geometry::{DisplaySourceSegment, FixedRect, PositionedStyledSpan, SourceRange},
    wrap::{ResolvedDisplayScalar, WrappedLine},
};
use scriptcraft_model::{
    formatted_text::parse_style_attribute,
    formatting::{EffectiveTextStyle, FixedPoint, FontFallback},
    typography::EffectiveTextStyle as ResolvedTextStyle,
};
use std::sync::Arc;

/// Backend-neutral span payload for the resolved metric path. The legacy
/// `PositionedStyledSpan` remains untouched until active builders migrate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Consumed by the resolved builder migration in WO-005A.2+.
pub(super) struct ResolvedPositionedSpan {
    pub(super) display_text: Arc<str>,
    pub(super) source: SourceRange,
    pub(super) display_source_map: Arc<[DisplaySourceSegment]>,
    pub(super) text_style: ResolvedTextStyle,
    pub(super) geometry: FixedRect,
    pub(super) display_boundary_offsets: Arc<[FixedPoint]>,
    pub(super) underline: bool,
    pub(super) strikethrough: bool,
}

/// Position already-resolved scalars without consulting legacy role styles or
/// manufacturing cell widths. Each displayed scalar gets its own boundary;
/// repeated source ranges preserve atomic uppercase expansions exactly.
#[allow(dead_code)] // Introduced before active builder routing by the recovery contract.
pub(super) fn resolved_positioned_spans(
    scalars: &[ResolvedDisplayScalar],
    element: usize,
    line_geometry: FixedRect,
) -> Vec<ResolvedPositionedSpan> {
    let mut output: Vec<ResolvedPositionedSpanBuilder> = Vec::new();
    let mut x = line_geometry.x.raw();
    for scalar in scalars {
        let can_coalesce = output.last().is_some_and(|span| {
            span.text_style == scalar.text_style
                && span.strikethrough == scalar.strikethrough
                && span.display_source_map.last().is_some_and(|previous| {
                    previous.source_end == scalar.source_range.start
                        || (previous.source_start == scalar.source_range.start
                            && previous.source_end == scalar.source_range.end)
                })
        });
        if can_coalesce {
            output.last_mut().expect("checked span exists").push(scalar);
        } else {
            output.push(ResolvedPositionedSpanBuilder::new(
                scalar,
                element,
                line_geometry,
                x,
            ));
        }
        x = x.saturating_add(scalar.advance.raw().max(0));
    }
    output
        .into_iter()
        .map(ResolvedPositionedSpanBuilder::finish)
        .collect()
}

pub(super) fn compatibility_style(style: &ResolvedTextStyle) -> EffectiveTextStyle {
    EffectiveTextStyle {
        declared_family: style.declared_family.clone(),
        // This closed enum is legacy metadata only. Resolved spans carry the
        // authoritative face ID and metric-derived size/style/color values.
        family: FontFallback::CourierPrime,
        size: style.size.min(u16::from(u8::MAX)) as u8,
        bold: style.bold,
        italic: style.italic,
        underline: style.underline,
        all_caps: style.all_caps,
        hidden_text: style.hidden_text,
        source_color: style.source_color.clone(),
    }
}

#[allow(dead_code)] // Used by resolved_positioned_spans once the builders migrate.
struct ResolvedPositionedSpanBuilder {
    display_text: String,
    source: SourceRange,
    display_source_map: Vec<DisplaySourceSegment>,
    text_style: ResolvedTextStyle,
    geometry: FixedRect,
    display_boundary_offsets: Vec<FixedPoint>,
    underline: bool,
    strikethrough: bool,
}

impl ResolvedPositionedSpanBuilder {
    fn new(
        scalar: &ResolvedDisplayScalar,
        element: usize,
        line_geometry: FixedRect,
        x: i64,
    ) -> Self {
        Self {
            display_text: scalar.character.to_string(),
            source: SourceRange {
                element,
                start: scalar.source_range.start,
                end: scalar.source_range.end,
            },
            display_source_map: vec![DisplaySourceSegment {
                display_start: 0,
                display_end: 1,
                source_start: scalar.source_range.start,
                source_end: scalar.source_range.end,
            }],
            text_style: scalar.text_style.clone(),
            geometry: FixedRect {
                x: FixedPoint::from_raw(x),
                y: line_geometry.y,
                width: FixedPoint::from_raw(scalar.advance.raw().max(0)),
                height: line_geometry.height,
            },
            display_boundary_offsets: vec![
                FixedPoint::default(),
                FixedPoint::from_raw(scalar.advance.raw().max(0)),
            ],
            underline: scalar.text_style.underline,
            strikethrough: scalar.strikethrough,
        }
    }

    fn push(&mut self, scalar: &ResolvedDisplayScalar) {
        let display_start = self.display_text.chars().count();
        let advance = scalar.advance.raw().max(0);
        self.display_text.push(scalar.character);
        self.source.end = self.source.end.max(scalar.source_range.end);
        self.geometry.width =
            FixedPoint::from_raw(self.geometry.width.raw().saturating_add(advance));
        self.display_boundary_offsets.push(self.geometry.width);
        self.display_source_map.push(DisplaySourceSegment {
            display_start,
            display_end: display_start.saturating_add(1),
            source_start: scalar.source_range.start,
            source_end: scalar.source_range.end,
        });
    }

    fn finish(self) -> ResolvedPositionedSpan {
        ResolvedPositionedSpan {
            display_text: Arc::from(self.display_text),
            source: self.source,
            display_source_map: self.display_source_map.into(),
            text_style: self.text_style,
            geometry: self.geometry,
            display_boundary_offsets: self.display_boundary_offsets.into(),
            underline: self.underline,
            strikethrough: self.strikethrough,
        }
    }
}

pub(super) fn positioned_spans(
    line: &WrappedLine,
    line_geometry: FixedRect,
) -> Vec<PositionedStyledSpan> {
    if line.display_text.is_empty() {
        return Vec::new();
    }
    let mut run_styles = Vec::with_capacity(line.runs.len());
    let mut source_cursor = 0usize;
    for run in line.runs.iter() {
        let end = source_cursor.saturating_add(run.text.len());
        let flags = parse_style_attribute(&run.attributes)
            .map(|projection| projection.flags)
            .unwrap_or_default();
        let mut style = line.text_style.clone();
        style.bold |= flags.bold;
        style.italic |= flags.italics;
        style.underline |= flags.underline;
        style.all_caps |= flags.all_caps;
        for attribute in &run.attributes {
            match attribute.name.as_str() {
                "Font" if !attribute.value.is_empty() => {
                    style.declared_family = attribute.value.clone();
                    style.family = FontFallback::CourierPrime;
                }
                "Size" => {
                    if let Ok(size) = attribute.value.parse::<u8>() {
                        if (1..=72).contains(&size) {
                            style.size = size;
                        }
                    }
                }
                _ => {}
            }
        }
        run_styles.push((source_cursor..end, style, flags.strikethrough));
        source_cursor = end;
    }

    let base_size = i64::from(line.text_style.size.max(1));
    let mut output: Vec<PositionedSpanBuilder> = Vec::new();
    let mut x = line_geometry.x.raw();
    let mut mapping_index = 0usize;
    let mut run_index = 0usize;
    for (display_index, character) in line.display_text.chars().enumerate() {
        while line
            .display_source_map
            .get(mapping_index)
            .is_some_and(|segment| display_index >= segment.display_end)
        {
            mapping_index = mapping_index.saturating_add(1);
        }
        let Some(mapping) = line
            .display_source_map
            .get(mapping_index)
            .filter(|segment| {
                display_index >= segment.display_start && display_index < segment.display_end
            })
        else {
            continue;
        };
        while run_styles
            .get(run_index)
            .is_some_and(|(range, _, _)| mapping.source_start >= range.end)
        {
            run_index = run_index.saturating_add(1);
        }
        let (style, strikethrough) = run_styles
            .get(run_index)
            .filter(|(range, _, _)| {
                mapping.source_start >= range.start && mapping.source_start < range.end
            })
            .map(|(_, style, strikethrough)| (style, *strikethrough))
            .unwrap_or((&line.text_style, false));
        let source_start = mapping.source_start;
        let source_end = mapping.source_end;
        let cell = FixedPoint::from_raw(
            line.char_width
                .raw()
                .saturating_mul(i64::from(style.size.max(1)))
                .div_euclid(base_size)
                .max(1),
        );
        let same = output.last().is_some_and(|span| {
            &span.text_style == style
                && span.underline == style.underline
                && span.strikethrough == strikethrough
                && !character.is_whitespace()
                && !span
                    .display_text
                    .chars()
                    .last()
                    .is_some_and(char::is_whitespace)
                && (span.source.end == source_start
                    || (span.source.start == source_start && span.source.end == source_end))
        });
        if same {
            output.last_mut().expect("checked span exists").push(
                character,
                source_start,
                source_end,
                cell,
            );
        } else {
            output.push(PositionedSpanBuilder {
                display_text: character.to_string(),
                source: SourceRange {
                    element: line.element,
                    start: source_start,
                    end: source_end,
                },
                display_source_map: vec![DisplaySourceSegment {
                    display_start: 0,
                    display_end: 1,
                    source_start,
                    source_end,
                }],
                text_style: style.clone(),
                char_width: cell,
                geometry: FixedRect {
                    x: FixedPoint::from_raw(x),
                    y: line_geometry.y,
                    width: cell,
                    height: line_geometry.height,
                },
                display_boundary_offsets: vec![FixedPoint::default(), cell],
                underline: style.underline,
                strikethrough,
            });
        }
        x = x.saturating_add(cell.raw());
    }
    output
        .into_iter()
        .map(PositionedSpanBuilder::finish)
        .collect()
}

struct PositionedSpanBuilder {
    display_text: String,
    source: SourceRange,
    display_source_map: Vec<DisplaySourceSegment>,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    geometry: FixedRect,
    display_boundary_offsets: Vec<FixedPoint>,
    underline: bool,
    strikethrough: bool,
}

impl PositionedSpanBuilder {
    fn push(&mut self, character: char, source_start: usize, source_end: usize, cell: FixedPoint) {
        let display_start = self
            .display_source_map
            .last()
            .map_or(0, |segment| segment.display_end);
        self.display_text.push(character);
        self.source.end = source_end;
        self.geometry.width =
            FixedPoint::from_raw(self.geometry.width.raw().saturating_add(cell.raw()));
        self.char_width = cell;
        self.display_boundary_offsets.push(self.geometry.width);
        self.display_source_map.push(DisplaySourceSegment {
            display_start,
            display_end: display_start.saturating_add(1),
            source_start,
            source_end,
        });
    }

    fn finish(self) -> PositionedStyledSpan {
        PositionedStyledSpan {
            display_text: Arc::from(self.display_text),
            source: self.source,
            display_source_map: self.display_source_map.into(),
            text_style: self.text_style,
            resolved_text_style: None,
            char_width: self.char_width,
            geometry: self.geometry,
            display_boundary_offsets: self.display_boundary_offsets.into(),
            underline: self.underline,
            strikethrough: self.strikethrough,
        }
    }
}

#[cfg(test)]
mod resolved_span_tests {
    use super::*;
    use scriptcraft_model::typography::ResolvedFontId;

    fn style(family: &str, size: u16, underline: bool) -> ResolvedTextStyle {
        ResolvedTextStyle::new(
            family,
            None,
            ResolvedFontId::new(format!("{family}-regular")).unwrap(),
            size,
            false,
            false,
            underline,
            false,
            false,
        )
        .unwrap()
    }

    fn scalar(
        character: char,
        source_range: std::ops::Range<usize>,
        text_style: ResolvedTextStyle,
        advance: i64,
    ) -> ResolvedDisplayScalar {
        ResolvedDisplayScalar {
            character,
            source_range,
            text_style,
            strikethrough: false,
            advance: FixedPoint::from_raw(advance),
        }
    }

    #[test]
    fn resolved_spans_keep_metric_boundaries_and_exact_expansion_mappings() {
        let base = style("Proportional", 19, false);
        let alternate = style("Second Face", 24, true);
        let scalars = [
            scalar('a', 0..1, base.clone(), 105),
            scalar('S', 1..3, base.clone(), 250),
            scalar('S', 1..3, base, 375),
            scalar('b', 3..4, alternate, 215),
        ];
        let spans = resolved_positioned_spans(
            &scalars,
            7,
            FixedRect {
                x: FixedPoint::from_raw(1_000),
                y: FixedPoint::from_raw(2_000),
                width: FixedPoint::from_raw(0),
                height: FixedPoint::from_points(18),
            },
        );

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].display_text.as_ref(), "aSS");
        assert_eq!(
            spans[0].source,
            SourceRange {
                element: 7,
                start: 0,
                end: 3
            }
        );
        assert_eq!(
            spans[0]
                .display_boundary_offsets
                .iter()
                .map(|offset| offset.raw())
                .collect::<Vec<_>>(),
            [0, 105, 355, 730]
        );
        assert_eq!(spans[0].display_source_map.len(), 3);
        assert_eq!(spans[0].display_source_map[1].source_start, 1);
        assert_eq!(spans[0].display_source_map[1].source_end, 3);
        assert_eq!(spans[0].display_source_map[2].source_start, 1);
        assert_eq!(spans[0].display_source_map[2].source_end, 3);
        assert_eq!(spans[0].geometry.x.raw(), 1_000);
        assert_eq!(spans[0].geometry.width.raw(), 730);
        assert_eq!(spans[1].display_text.as_ref(), "b");
        assert_eq!(spans[1].text_style.size, 24);
        assert!(spans[1].underline);
        assert_eq!(spans[1].geometry.x.raw(), 1_730);
        assert_eq!(spans[1].display_boundary_offsets[1].raw(), 215);
    }
}
