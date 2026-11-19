//! Source-safe paragraph alignment and formatted-project run mutations.

use super::runs::{
    clear_supported_styles_over_ranges, toggle_style_for_range, toggle_style_over_ranges,
    FormattedTextError, RunStyle, RunStyleEdit, RunStyleFlags, RunStyleRange,
};
use crate::{
    element::BodyOwner, formatting::ParagraphAlignment, preservation::PreservationError,
    project::Project,
};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParagraphTarget {
    Body(usize),
    TitlePage(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParagraphAlignmentProjection {
    pub alignment: Option<ParagraphAlignment>,
}
/// Change paragraph alignment for an explicit set of body/title paragraph
/// targets. Source attributes are updated in-place by expanded name, preserving
/// all other attribute values and their order. The candidate publishes only
/// after complete project validation succeeds.
pub fn set_paragraph_alignments(
    project: &mut Project,
    targets: &[ParagraphTarget],
    alignment: ParagraphAlignment,
) -> Result<(), FormattedTextError> {
    if targets.is_empty() {
        return Err(FormattedTextError::EmptyParagraphSelection);
    }
    let mut candidate = project.clone();
    for target in targets {
        match *target {
            ParagraphTarget::Body(index) => {
                let element = candidate
                    .elements
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                let id = element.id.clone();
                let previous_alignment = candidate
                    .direct_paragraph_overrides
                    .get(&id)
                    .and_then(|overrides| overrides.paragraph.alignment);
                let overrides = candidate
                    .direct_paragraph_overrides
                    .entry(id.clone())
                    .or_default();
                overrides.paragraph.alignment = Some(alignment);
                if let Some(preservation) = candidate.preservation.as_mut() {
                    preservation.set_managed_paragraph_attribute(
                        &id,
                        "Alignment",
                        Some(alignment.fdx_value()),
                    )?;
                    preservation.set_direct_paragraph_alignment(&id, alignment);
                }
                if previous_alignment != Some(alignment) {
                    BodyOwner::revise(&mut candidate.elements[index]);
                }
            }
            ParagraphTarget::TitlePage(index) => {
                let paragraph = candidate
                    .title_page
                    .get_mut(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                if let Some(preservation) = candidate.preservation.as_mut() {
                    preservation.set_managed_paragraph_attribute(
                        &paragraph.id,
                        "Alignment",
                        Some(alignment.fdx_value()),
                    )?;
                }
                paragraph.formatting.alignment = Some(alignment.into());
            }
        }
    }
    candidate.validate()?;
    *project = candidate;
    Ok(())
}
/// A paragraph alignment is exclusive only when the nonempty affected set is
/// uniform. Mixed selections and empty selections produce no lit alignment.
pub fn project_paragraph_alignment(
    alignments: &[Option<ParagraphAlignment>],
) -> ParagraphAlignmentProjection {
    let first = alignments.first().copied().flatten();
    let uniform = first.filter(|first| {
        alignments.len() > 0
            && alignments
                .iter()
                .all(|alignment| *alignment == Some(*first))
    });
    ParagraphAlignmentProjection { alignment: uniform }
}
/// Mutate one semantic paragraph's run projection after checking preservation
/// source topology. A non-Text source child between Text runs is an explicit
/// safe-stop: this model has no run boundary with which to preserve its position.
pub fn toggle_paragraph_run_style(
    project: &mut Project,
    target: ParagraphTarget,
    range: Range<usize>,
    active_style: RunStyleFlags,
    style: RunStyle,
) -> Result<RunStyleEdit, FormattedTextError> {
    let mut candidate = project.clone();
    let (id, text, runs) = match target {
        ParagraphTarget::Body(index) => {
            let element = candidate
                .elements
                .get(index)
                .ok_or(FormattedTextError::ParagraphOutOfRange)?;
            (
                element.id.clone(),
                element.text.clone(),
                element.runs.clone(),
            )
        }
        ParagraphTarget::TitlePage(index) => {
            let paragraph = candidate
                .title_page
                .get(index)
                .ok_or(FormattedTextError::ParagraphOutOfRange)?;
            (
                paragraph.id.clone(),
                paragraph.text.clone(),
                paragraph.runs.clone(),
            )
        }
    };
    if !range.is_empty() {
        if let Some(preservation) = candidate.preservation.as_ref() {
            preservation.ensure_run_range_is_contiguous(&id, range.clone())?;
        }
    }
    let edit = toggle_style_for_range(&text, &runs, range, active_style, style)?;
    if edit.changed {
        match target {
            ParagraphTarget::Body(index) => {
                BodyOwner::update(&mut candidate.elements[index], |element| {
                    element.runs = edit.runs.clone();
                    element.text = text.clone();
                });
            }
            ParagraphTarget::TitlePage(index) => {
                candidate.title_page[index].runs = edit.runs.clone();
                candidate.title_page[index].text = text.clone();
            }
        }
        if let Some(preservation) = candidate.preservation.as_mut() {
            let (id, has_provenance) = match target {
                ParagraphTarget::Body(index) => {
                    let element = &candidate.elements[index];
                    (element.id.clone(), element.source_provenance.is_some())
                }
                ParagraphTarget::TitlePage(index) => {
                    let paragraph = &candidate.title_page[index];
                    (paragraph.id.clone(), paragraph.source_provenance.is_some())
                }
            };
            if preservation.has_managed_element(&id) {
                preservation.sync_managed_text_runs(&id, &text, &edit.runs)?;
            } else if has_provenance {
                return Err(FormattedTextError::Preservation(
                    PreservationError::ManagedAnchorMissing,
                ));
            }
        }
        candidate.validate()?;
        *project = candidate;
    }
    Ok(edit)
}

/// Project-wide style toggle for one range per affected body/title paragraph.
/// Opaque source barriers are checked for every target before any candidate is
/// changed; duplicate targets reject rather than apply ambiguous ranges twice.
pub fn toggle_paragraph_run_styles(
    project: &mut Project,
    selections: &[(ParagraphTarget, Range<usize>)],
    active_style: RunStyleFlags,
    style: RunStyle,
) -> Result<Vec<RunStyleEdit>, FormattedTextError> {
    if selections.is_empty() {
        return Err(FormattedTextError::EmptyParagraphSelection);
    }
    let mut seen = std::collections::BTreeSet::new();
    if selections.iter().any(|(target, _)| !seen.insert(*target)) {
        return Err(FormattedTextError::DuplicateParagraphTarget);
    }
    let mut candidate = project.clone();
    let mut snapshots = Vec::with_capacity(selections.len());
    for (target, range) in selections {
        let (id, text, runs) = match *target {
            ParagraphTarget::Body(index) => {
                let element = candidate
                    .elements
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (
                    element.id.clone(),
                    element.text.clone(),
                    element.runs.clone(),
                )
            }
            ParagraphTarget::TitlePage(index) => {
                let paragraph = candidate
                    .title_page
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (
                    paragraph.id.clone(),
                    paragraph.text.clone(),
                    paragraph.runs.clone(),
                )
            }
        };
        if !range.is_empty() {
            if let Some(preservation) = candidate.preservation.as_ref() {
                preservation.ensure_run_range_is_contiguous(&id, range.clone())?;
            }
        }
        snapshots.push((*target, text, runs, range.clone()));
    }
    let ranges = snapshots
        .iter()
        .map(|(_, text, runs, range)| RunStyleRange {
            canonical_text: text,
            runs,
            range: range.clone(),
        })
        .collect::<Vec<_>>();
    let edits = toggle_style_over_ranges(&ranges, active_style, style)?;
    let mut changed = false;
    for ((target, text, _, _), edit) in snapshots.iter().zip(&edits) {
        if !edit.changed {
            continue;
        }
        changed = true;
        match *target {
            ParagraphTarget::Body(index) => {
                BodyOwner::update(&mut candidate.elements[index], |element| {
                    element.text = text.clone();
                    element.runs = edit.runs.clone();
                });
            }
            ParagraphTarget::TitlePage(index) => {
                candidate.title_page[index].text = text.clone();
                candidate.title_page[index].runs = edit.runs.clone();
            }
        }
        if let Some(preservation) = candidate.preservation.as_mut() {
            let (id, has_provenance) = match *target {
                ParagraphTarget::Body(index) => {
                    let element = &candidate.elements[index];
                    (element.id.clone(), element.source_provenance.is_some())
                }
                ParagraphTarget::TitlePage(index) => {
                    let paragraph = &candidate.title_page[index];
                    (paragraph.id.clone(), paragraph.source_provenance.is_some())
                }
            };
            if preservation.has_managed_element(&id) {
                preservation.sync_managed_text_runs(&id, text, &edit.runs)?;
            } else if has_provenance {
                return Err(FormattedTextError::Preservation(
                    PreservationError::ManagedAnchorMissing,
                ));
            }
        }
    }
    if changed {
        candidate.validate()?;
        *project = candidate;
    }
    Ok(edits)
}

/// Transactionally clears supported run styles across Body/Title Page targets.
/// The caller supplies source-mapped ranges and owns transient typing state.
pub fn clear_paragraph_run_styles(
    project: &mut Project,
    selections: &[(ParagraphTarget, Range<usize>)],
) -> Result<Vec<RunStyleEdit>, FormattedTextError> {
    if selections.is_empty() {
        return Err(FormattedTextError::EmptyParagraphSelection);
    }
    let mut seen = std::collections::BTreeSet::new();
    if selections.iter().any(|(target, _)| !seen.insert(*target)) {
        return Err(FormattedTextError::DuplicateParagraphTarget);
    }
    let mut candidate = project.clone();
    let mut snapshots = Vec::with_capacity(selections.len());
    for (target, range) in selections {
        let (id, text, runs) = match *target {
            ParagraphTarget::Body(index) => {
                let element = candidate
                    .elements
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (
                    element.id.clone(),
                    element.text.clone(),
                    element.runs.clone(),
                )
            }
            ParagraphTarget::TitlePage(index) => {
                let paragraph = candidate
                    .title_page
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (
                    paragraph.id.clone(),
                    paragraph.text.clone(),
                    paragraph.runs.clone(),
                )
            }
        };
        if !range.is_empty() {
            if let Some(preservation) = candidate.preservation.as_ref() {
                preservation.ensure_run_range_is_contiguous(&id, range.clone())?;
            }
        }
        snapshots.push((*target, text, runs, range.clone()));
    }
    let ranges = snapshots
        .iter()
        .map(|(_, text, runs, range)| RunStyleRange {
            canonical_text: text,
            runs,
            range: range.clone(),
        })
        .collect::<Vec<_>>();
    let edits = clear_supported_styles_over_ranges(&ranges)?;
    let mut changed = false;
    for ((target, text, _, _), edit) in snapshots.iter().zip(&edits) {
        if !edit.changed {
            continue;
        }
        changed = true;
        match *target {
            ParagraphTarget::Body(index) => {
                BodyOwner::update(&mut candidate.elements[index], |element| {
                    element.text = text.clone();
                    element.runs = edit.runs.clone();
                });
            }
            ParagraphTarget::TitlePage(index) => {
                candidate.title_page[index].text = text.clone();
                candidate.title_page[index].runs = edit.runs.clone();
            }
        }
        if let Some(preservation) = candidate.preservation.as_mut() {
            let (id, has_provenance) = match *target {
                ParagraphTarget::Body(index) => {
                    let element = &candidate.elements[index];
                    (element.id.clone(), element.source_provenance.is_some())
                }
                ParagraphTarget::TitlePage(index) => {
                    let paragraph = &candidate.title_page[index];
                    (paragraph.id.clone(), paragraph.source_provenance.is_some())
                }
            };
            if preservation.has_managed_element(&id) {
                preservation.sync_managed_text_runs(&id, text, &edit.runs)?;
            } else if has_provenance {
                return Err(FormattedTextError::Preservation(
                    PreservationError::ManagedAnchorMissing,
                ));
            }
        }
    }
    if changed {
        candidate.validate()?;
        *project = candidate;
    }
    Ok(edits)
}
