use super::{
    geometry::{DisplaySourceSegment, FixedRect, PositionedStyledSpan, SourceRange},
    wrap::WrappedLine,
};
use scriptcraft_model::{
    formatted_text::parse_style_attribute,
    formatting::{EffectiveTextStyle, FixedPoint, FontFallback},
};
use std::sync::Arc;

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
                && !span.display_text.chars().last().is_some_and(char::is_whitespace)
                && (span.source.end == source_start
                    || (span.source.start == source_start && span.source.end == source_end))
        });
        if same {
            output
                .last_mut()
                .expect("checked span exists")
                .push(character, source_start, source_end, cell);
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
                underline: style.underline,
                strikethrough,
            });
        }
        x = x.saturating_add(cell.raw());
    }
    output.into_iter().map(PositionedSpanBuilder::finish).collect()
}

struct PositionedSpanBuilder {
    display_text: String,
    source: SourceRange,
    display_source_map: Vec<DisplaySourceSegment>,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    geometry: FixedRect,
    underline: bool,
    strikethrough: bool,
}

impl PositionedSpanBuilder {
    fn push(
        &mut self,
        character: char,
        source_start: usize,
        source_end: usize,
        cell: FixedPoint,
    ) {
        let display_start = self
            .display_source_map
            .last()
            .map_or(0, |segment| segment.display_end);
        self.display_text.push(character);
        self.source.end = source_end;
        self.geometry.width = FixedPoint::from_raw(
            self.geometry.width.raw().saturating_add(cell.raw()),
        );
        self.char_width = cell;
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
            char_width: self.char_width,
            geometry: self.geometry,
            underline: self.underline,
            strikethrough: self.strikethrough,
        }
    }
}

