use super::selection::{validate_position, validate_title_position};
use super::structure::remove_body_owner;
use super::{EditError, Position};
use scriptcraft_model::formatted_text::{
    project_style_at_caret, toggle_paragraph_run_styles, ParagraphTarget, RunBoundaryAffinity,
    RunStyle, RunStyleFlags,
};
use scriptcraft_model::preservation::PreservationError;
use scriptcraft_model::{
    BodyOwner, Project, RunAttribute, ScreenplayElement, TextRun, TitlePageParagraph,
    MAX_TEXT_BYTES,
};

pub(super) fn effective_title_runs(paragraph: &TitlePageParagraph) -> Vec<TextRun> {
    if paragraph.runs.is_empty() {
        if paragraph.text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun {
                text: paragraph.text.clone(),
                attributes: Vec::new(),
            }]
        }
    } else {
        paragraph.runs.clone()
    }
}

pub(super) fn title_run_attributes_at(
    runs: &[TextRun],
    offset: usize,
    total: usize,
) -> Vec<RunAttribute> {
    let mut cursor = 0usize;
    for run in runs {
        let limit = cursor.saturating_add(run.text.len());
        if offset < limit || (offset == total && limit == total) {
            return run.attributes.clone();
        }
        cursor = limit;
    }
    Vec::new()
}

pub(super) fn apply_typing_styles(
    mut attributes: Vec<RunAttribute>,
    typing: RunStyleFlags,
) -> Vec<RunAttribute> {
    let mut tokens = attributes
        .iter()
        .find(|attribute| attribute.name == "Style")
        .map(|attribute| {
            attribute
                .value
                .split('+')
                .filter(|token| {
                    !matches!(
                        *token,
                        "Bold"
                            | "Italic"
                            | "Underline"
                            | "AllCaps"
                            | "Strikethrough"
                            | "StrikeThrough"
                    )
                })
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (enabled, token) in [
        (typing.bold, "Bold"),
        (typing.italics, "Italic"),
        (typing.underline, "Underline"),
        (typing.all_caps, "AllCaps"),
        (typing.strikethrough, "Strikethrough"),
    ] {
        if enabled {
            tokens.push(token.to_owned());
        }
    }
    if let Some(attribute) = attributes
        .iter_mut()
        .find(|attribute| attribute.name == "Style")
    {
        attribute.value = tokens.join("+");
    } else if !tokens.is_empty() {
        attributes.push(RunAttribute {
            name: "Style".into(),
            value: tokens.join("+"),
        });
    }
    attributes
}

pub(super) fn synchronize_body_typing_style(
    project: &mut Project,
    start: Position,
    inserted: &str,
    typing: RunStyleFlags,
) -> Result<(), EditError> {
    let element = project
        .elements
        .get(start.element)
        .ok_or(EditError::InvalidElement)?;
    let current = project_style_at_caret(
        &element.text,
        &element.runs,
        start.offset,
        RunBoundaryAffinity::Right,
        RunStyleFlags::default(),
    )
    .map_err(EditError::FormattedText)?;
    let target = ParagraphTarget::Body(start.element);
    let range = start.offset..start.offset.saturating_add(inserted.len());
    for style in [
        RunStyle::Bold,
        RunStyle::Italics,
        RunStyle::Underline,
        RunStyle::AllCaps,
        RunStyle::Strikethrough,
    ] {
        if current.contains(style) != typing.contains(style) {
            toggle_paragraph_run_styles(
                project,
                &[(target, range.clone())],
                RunStyleFlags::default(),
                style,
            )
            .map_err(EditError::FormattedText)?;
        }
    }
    Ok(())
}

pub(super) fn replace_range(
    project: &mut Project,
    start: Position,
    end: Position,
    inserted: &str,
) -> Result<(), EditError> {
    validate_position(project, start)?;
    validate_position(project, end)?;
    if (start.element, start.offset) > (end.element, end.offset) {
        return Err(EditError::InvalidSelection);
    }
    if inserted.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    if start.element == end.element {
        replace_element_text(project, start.element, start.offset, end.offset, inserted)?;
        return Ok(());
    }
    // The transaction-level preservation reconciliation retains the first ID,
    // removes the consumed source paragraphs and carries source runs forward.
    let prefix = project.elements[start.element].text[..start.offset].to_owned();
    let suffix = project.elements[end.element].text[end.offset..].to_owned();
    let combined = format!("{prefix}{inserted}{suffix}");
    if combined.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let runs = replace_cross_element_runs(project, start, end, inserted, &combined)?;
    BodyOwner::update(&mut project.elements[start.element], |element| {
        element.text = combined;
        element.runs = runs;
    });
    for _ in start.element + 1..=end.element {
        remove_body_owner(project, start.element + 1);
    }
    Ok(())
}

pub(super) fn replace_element_text(
    project: &mut Project,
    element_index: usize,
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<(), EditError> {
    let item = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?;
    let old_text = item.text.clone();
    let old_runs = item.runs.clone();
    let segments = if old_runs.is_empty() {
        vec![old_text.clone()]
    } else {
        old_runs.iter().map(|run| run.text.clone()).collect()
    };
    let updated_segments = replace_text_segments(&segments, start, end, inserted)?;
    let new_text = updated_segments.concat();
    if new_text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let id = item.id.clone();
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        let expected_run_text = old_runs
            .iter()
            .map(|run| run.text.clone())
            .collect::<Vec<_>>();
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ))?
            .replace_managed_text(
                &id,
                &old_text,
                start,
                end,
                inserted,
                (!old_runs.is_empty()).then_some(expected_run_text.as_slice()),
            )
            .map_err(EditError::Preservation)?;
    } else if item.source_provenance.is_some() {
        return Err(EditError::Preservation(
            PreservationError::ManagedAnchorMissing,
        ));
    }
    BodyOwner::update(&mut project.elements[element_index], |item| {
        item.text = new_text;
        if item.text.is_empty() {
            item.runs.clear();
        } else if !old_runs.is_empty() {
            for (run, text) in item.runs.iter_mut().zip(updated_segments) {
                run.text = text;
            }
        }
    });
    if has_source_mapping {
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(PreservationError::ManagedAnchorMissing))?
            .sync_managed_text_runs(
                &id,
                &project.elements[element_index].text,
                &project.elements[element_index].runs,
            )
            .map_err(EditError::Preservation)?;
    }
    Ok(())
}

fn replace_text_segments(
    segments: &[String],
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<Vec<String>, EditError> {
    let total = segments.iter().map(String::len).sum::<usize>();
    if start > end || end > total {
        return Err(EditError::InvalidSelection);
    }
    let mut insertion_segment = None;
    let mut base = 0usize;
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        if start < limit || (start == total && limit == total) {
            insertion_segment = Some(index);
            break;
        }
        base = limit;
    }
    let insertion_segment = insertion_segment
        .or_else(|| segments.len().checked_sub(1))
        .ok_or(EditError::InvalidSelection)?;
    let mut base = 0usize;
    let mut updated = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        let before_len = start.saturating_sub(base).min(segment.len());
        let after_start = end.saturating_sub(base).min(segment.len());
        let mut value = segment[..before_len].to_owned();
        if index == insertion_segment {
            value.push_str(inserted);
        }
        if end < limit || end <= base {
            value.push_str(&segment[after_start..]);
        }
        updated.push(value);
        base = limit;
    }
    Ok(updated)
}

fn replace_cross_element_runs(
    project: &Project,
    start: Position,
    end: Position,
    inserted: &str,
    combined: &str,
) -> Result<Vec<TextRun>, EditError> {
    let left = &project.elements[start.element];
    let right = &project.elements[end.element];
    if left.runs.is_empty() && right.runs.is_empty() {
        return Ok(Vec::new());
    }
    let left_runs = effective_runs(left);
    let right_runs = effective_runs(right);
    let mut result = slice_runs(&left_runs, 0, start.offset);
    let insertion_attributes = run_attributes_at(&left_runs, start.offset);
    push_run(&mut result, inserted, insertion_attributes);
    let suffix_runs = if start.element == end.element {
        slice_runs(&left_runs, end.offset, left.text.len())
    } else {
        slice_runs(&right_runs, end.offset, right.text.len())
    };
    result.extend(suffix_runs);
    coalesce_runs(&mut result);
    let actual = result
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    if actual != combined {
        return Err(EditError::Preservation(
            PreservationError::UnsupportedTextMapping,
        ));
    }
    Ok(result)
}

pub(super) fn effective_runs(element: &ScreenplayElement) -> Vec<TextRun> {
    if element.runs.is_empty() {
        vec![TextRun {
            text: element.text.clone(),
            attributes: Vec::new(),
        }]
    } else {
        element.runs.clone()
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
            result.push(TextRun {
                text: run.text[from..to].to_owned(),
                attributes: run.attributes.clone(),
            });
        }
        base = limit;
        if base >= end {
            break;
        }
    }
    result
}

pub(super) fn run_attributes_at(runs: &[TextRun], offset: usize) -> Vec<RunAttribute> {
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        if offset < limit || (offset == limit && limit == runs.iter().map(|r| r.text.len()).sum()) {
            return run.attributes.clone();
        }
        base = limit;
    }
    Vec::new()
}

pub(super) fn push_run(runs: &mut Vec<TextRun>, text: &str, attributes: Vec<RunAttribute>) {
    if text.is_empty() {
        return;
    }
    runs.push(TextRun {
        text: text.to_owned(),
        attributes,
    });
}

pub(super) fn coalesce_runs(runs: &mut Vec<TextRun>) {
    let mut coalesced: Vec<TextRun> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let Some(previous) = coalesced.last_mut() {
            if previous.attributes == run.attributes {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        coalesced.push(run);
    }
    *runs = coalesced;
}

pub(super) fn previous_boundary(text: &str, offset: usize) -> usize {
    text[..offset]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

pub(super) fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .chars()
        .next()
        .map_or(offset, |character| offset + character.len_utf8())
}
