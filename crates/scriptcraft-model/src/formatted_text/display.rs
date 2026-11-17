//! Styled display text and source-byte projection.

use std::ops::Range;

use super::runs::{effective_runs, parse_style_attribute, FormattedTextError, RunStyleFlags};
use crate::element::TextRun;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySourceSegment {
    pub display_range: Range<usize>,
    pub source_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledDisplaySpan {
    pub source_range: Range<usize>,
    pub display_text: String,
    pub styles: RunStyleFlags,
    /// One mapping per canonical Unicode scalar; uppercase expansion may map
    /// multiple display bytes back to the same source scalar.
    pub display_to_source: Vec<DisplaySourceSegment>,
}
/// Build display spans without modifying canonical text. AllCaps uses Unicode
/// uppercase expansion and records every display scalar's canonical source
/// byte range, including one-to-many expansions such as `ß` → `SS`.
pub fn styled_display_spans(
    canonical_text: &str,
    runs: &[TextRun],
) -> Result<Vec<StyledDisplaySpan>, FormattedTextError> {
    let source_runs = effective_runs(canonical_text, runs)?;
    let mut spans = Vec::with_capacity(source_runs.len());
    let mut source_cursor = 0usize;
    for run in source_runs {
        let flags = parse_style_attribute(&run.attributes)?.flags;
        let source_start = source_cursor;
        let mut display_text = String::new();
        let mut display_to_source = Vec::new();
        for (local_start, scalar) in run.text.char_indices() {
            let local_end = local_start + scalar.len_utf8();
            if flags.all_caps {
                for displayed_scalar in scalar.to_uppercase() {
                    let display_start = display_text.len();
                    display_text.push(displayed_scalar);
                    display_to_source.push(DisplaySourceSegment {
                        display_range: display_start..display_text.len(),
                        source_range: source_start + local_start..source_start + local_end,
                    });
                }
            } else {
                let display_start = display_text.len();
                display_text.push(scalar);
                display_to_source.push(DisplaySourceSegment {
                    display_range: display_start..display_text.len(),
                    source_range: source_start + local_start..source_start + local_end,
                });
            }
        }
        source_cursor += run.text.len();
        if !run.text.is_empty() {
            spans.push(StyledDisplaySpan {
                source_range: source_start..source_cursor,
                display_text,
                styles: flags,
                display_to_source,
            });
        }
    }
    Ok(spans)
}
