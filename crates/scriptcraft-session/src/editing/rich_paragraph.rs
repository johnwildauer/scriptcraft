//! Small run/text primitives shared by Beat and ScriptNote paragraph editors.

use super::EditError;
use scriptcraft_model::formatted_text::normalize_runs;
use scriptcraft_model::{TextRun, MAX_TEXT_BYTES};

pub(super) fn effective_runs(text: &str, runs: &[TextRun]) -> Vec<TextRun> {
    if runs.is_empty() {
        if text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun { text: text.to_owned(), attributes: Vec::new() }]
        }
    } else {
        runs.to_vec()
    }
}

pub(super) fn slice_runs(runs: &[TextRun], start: usize, end: usize) -> Vec<TextRun> {
    let mut result = Vec::new();
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        let from = start.saturating_sub(base).min(run.text.len());
        let to = end.saturating_sub(base).min(run.text.len());
        if from < to {
            result.push(TextRun { text: run.text[from..to].to_owned(), attributes: run.attributes.clone() });
        }
        base = limit;
        if base >= end { break; }
    }
    result
}

pub(super) fn attributes_at(runs: &[TextRun], offset: usize, total: usize) -> Vec<scriptcraft_model::RunAttribute> {
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        if offset < limit || (offset == total && limit == total) { return run.attributes.clone(); }
        base = limit;
    }
    Vec::new()
}

pub(super) fn previous_boundary(text: &str, offset: usize) -> usize {
    text[..offset].char_indices().next_back().map_or(0, |(index, _)| index)
}

pub(super) fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..].char_indices().nth(1).map_or(text.len(), |(index, _)| offset + index)
}

pub(super) fn push_run(runs: &mut Vec<TextRun>, text: &str, attributes: &[scriptcraft_model::RunAttribute]) {
    if !text.is_empty() {
        runs.push(TextRun { text: text.to_owned(), attributes: attributes.to_vec() });
    }
}

/// Compose paragraph text/run payloads for a multi-paragraph replacement.
/// Identity and settings ownership remain with the caller: editors can retain
/// the left owner while applying their own policy to subsequent chunks.
pub(super) fn compose_replacement_chunks(
    left_text: &str,
    left_runs: &[TextRun],
    start: usize,
    right_text: &str,
    right_runs: &[TextRun],
    end: usize,
    chunks: &[String],
    insertion_attributes: &[scriptcraft_model::RunAttribute],
) -> Vec<(String, Vec<TextRun>)> {
    if chunks.is_empty() { return Vec::new(); }
    let prefix = &left_text[..start];
    let mut result = Vec::with_capacity(chunks.len());
    for (index, chunk) in chunks.iter().enumerate() {
        let mut text = String::new();
        let mut runs = Vec::new();
        if index == 0 {
            text.push_str(prefix);
            runs.extend(slice_runs(left_runs, 0, start));
        }
        text.push_str(chunk);
        push_run(&mut runs, chunk, insertion_attributes);
        if index + 1 == chunks.len() {
            text.push_str(&right_text[end..]);
            runs.extend(slice_runs(right_runs, end, right_text.len()));
        }
        result.push((text, runs));
    }
    result
}

pub(super) fn set_normalized_text(
    target_text: &mut String,
    target_runs: &mut Vec<TextRun>,
    text: String,
    runs: Vec<TextRun>,
) -> Result<(), EditError> {
    if text.len() > MAX_TEXT_BYTES { return Err(EditError::TextTooLong); }
    *target_runs = normalize_runs(&text, &runs).map_err(EditError::FormattedText)?;
    *target_text = text;
    Ok(())
}
