use super::{
    geometry::{DisplaySourceSegment, FixedRect, SourceRange},
    wrap::ResolvedDisplayScalar,
};
use scriptcraft_model::{formatting::FixedPoint, typography::EffectiveTextStyle as ResolvedTextStyle};
use std::sync::Arc;

/// Backend-neutral span payload carrying resolved typography and metric geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Position already-resolved scalars using their measured advances. Each
/// displayed scalar gets its own boundary; repeated source ranges preserve
/// atomic uppercase expansions exactly.
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
