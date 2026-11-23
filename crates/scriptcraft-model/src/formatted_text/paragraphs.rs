//! Source-safe paragraph alignment and formatted-project run mutations.

use super::runs::{
    clear_supported_styles_over_ranges, toggle_style_for_range, toggle_style_over_ranges,
    FormattedTextError, RunStyle, RunStyleEdit, RunStyleFlags, RunStyleRange,
};
use crate::{
    element::BodyOwner,
    formatting::{
        FixedPoint, ParagraphAlignment, ParagraphFormattingPatch, ParagraphLeading,
        ParagraphSpacing, SourceParagraphOverrides, SourceParagraphSpec,
    },
    preservation::PreservationError,
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
    set_paragraph_formatting(
        project,
        targets,
        ParagraphFormattingPatch {
            alignment: Some(alignment),
            ..ParagraphFormattingPatch::default()
        },
    )
}

/// Applies one typed field to each selected paragraph using a complete
/// definition-derived baseline. Source projection and semantic overrides are
/// staged together and published only after full candidate validation.
pub fn set_paragraph_formatting(
    project: &mut Project,
    targets: &[ParagraphTarget],
    patch: ParagraphFormattingPatch,
) -> Result<(), FormattedTextError> {
    if targets.is_empty() {
        return Err(FormattedTextError::EmptyParagraphSelection);
    }
    if targets.iter().copied().collect::<std::collections::BTreeSet<_>>().len() != targets.len() {
        return Err(FormattedTextError::DuplicateParagraphTarget);
    }
    if patch.space_before_lines.is_some_and(|lines| lines > 3) {
        return Err(FormattedTextError::Model(crate::ModelError::InvalidNumber));
    }
    if patch == ParagraphFormattingPatch::default() {
        return Ok(());
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
                let element_type = element.element_type();
                let definition = candidate
                    .element_definitions()
                    .get(&element_type)
                    .ok_or_else(|| FormattedTextError::Model(
                        crate::ModelError::MissingElementDefinition(element_type.clone()),
                    ))?;
                let old = candidate
                    .direct_paragraph_overrides
                    .get(&id)
                    .copied()
                    .unwrap_or_default();
                let mut complete = definition_paragraph_baseline(definition);
                overlay_paragraph_overrides(&mut complete, old);
                if !paragraph_patch_changes(complete, patch, definition.font.size)? {
                    continue;
                }
                apply_paragraph_patch(&mut complete, patch, definition.font.size)?;
                let source_attributes = paragraph_source_attributes(
                    complete,
                    &definition.paragraph.leading,
                    definition.paragraph.spacing,
                );
                let has_source_provenance = candidate.elements[index].source_provenance.is_some();
                if let Some(preservation) = candidate.preservation.as_mut() {
                    if preservation.has_managed_element(&id) {
                        preservation.set_managed_paragraph_attributes(&id, &source_attributes)?;
                    } else if has_source_provenance {
                        return Err(FormattedTextError::Preservation(
                            PreservationError::ManagedAnchorMissing,
                        ));
                    }
                    preservation.set_direct_paragraph_overrides(&id, complete);
                }
                candidate.direct_paragraph_overrides.insert(id, complete);
                BodyOwner::revise(&mut candidate.elements[index]);
            }
            ParagraphTarget::TitlePage(index) => {
                let paragraph = candidate
                    .title_page
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                let id = paragraph.id.clone();
                let Some(element_type) = paragraph.element_type.clone() else {
                    if patch.alignment.is_none() {
                        return Err(FormattedTextError::Model(crate::ModelError::InvalidNumber));
                    }
                    let alignment = patch.alignment.ok_or(FormattedTextError::Model(
                        crate::ModelError::InvalidNumber,
                    ))?;
                    if paragraph.formatting.alignment == Some(alignment.into()) {
                        continue;
                    }
                    if let Some(preservation) = candidate.preservation.as_mut() {
                        preservation.set_managed_paragraph_attribute(
                            &id,
                            "Alignment",
                            Some(alignment.fdx_value()),
                        )?;
                    }
                    candidate.title_page[index].formatting.alignment = Some(alignment.into());
                    continue;
                };
                let definition = candidate
                    .element_definitions()
                    .get(&element_type)
                    .ok_or_else(|| FormattedTextError::Model(
                        crate::ModelError::MissingElementDefinition(element_type.clone()),
                    ))?;
                let mut complete = definition_paragraph_baseline(definition);
                overlay_title_formatting(&mut complete, paragraph.formatting);
                if !paragraph_patch_changes(complete, patch, definition.font.size)? {
                    continue;
                }
                apply_paragraph_patch(&mut complete, patch, definition.font.size)?;
                let source_attributes = paragraph_source_attributes(
                    complete,
                    &definition.paragraph.leading,
                    definition.paragraph.spacing,
                );
                let has_source_provenance = candidate.title_page[index].source_provenance.is_some();
                if let Some(preservation) = candidate.preservation.as_mut() {
                    if preservation.has_managed_element(&id) {
                        preservation.set_managed_paragraph_attributes(&id, &source_attributes)?;
                    } else if has_source_provenance {
                        return Err(FormattedTextError::Preservation(
                            PreservationError::ManagedAnchorMissing,
                        ));
                    }
                }
                candidate.title_page[index].formatting = title_formatting_from_overrides(complete);
            }
        }
    }
    candidate.validate()?;
    if candidate != *project {
        *project = candidate;
    }
    Ok(())
}

fn definition_paragraph_baseline(
    definition: &crate::ElementDefinition,
) -> SourceParagraphOverrides {
    SourceParagraphOverrides {
        paragraph: SourceParagraphSpec {
            alignment: Some(definition.paragraph.alignment),
            first_indent: Some(definition.paragraph.first_indent),
            leading: ParagraphLeading::from_definition_value(&definition.paragraph.leading),
            left_indent: Some(definition.paragraph.left_indent),
            right_indent: Some(definition.paragraph.right_indent),
            space_before: Some(definition.paragraph.space_before),
            spacing: ParagraphSpacing::from_definition_value(definition.paragraph.spacing),
        },
        starts_new_page: Some(definition.paragraph.starts_new_page),
    }
}

fn overlay_paragraph_overrides(
    target: &mut SourceParagraphOverrides,
    source: SourceParagraphOverrides,
) {
    macro_rules! overlay {
        ($field:ident) => {
            if source.paragraph.$field.is_some() {
                target.paragraph.$field = source.paragraph.$field;
            }
        };
    }
    overlay!(alignment);
    overlay!(first_indent);
    overlay!(leading);
    overlay!(left_indent);
    overlay!(right_indent);
    overlay!(space_before);
    overlay!(spacing);
    if source.starts_new_page.is_some() {
        target.starts_new_page = source.starts_new_page;
    }
}

fn overlay_title_formatting(
    target: &mut SourceParagraphOverrides,
    source: crate::TitlePageParagraphFormatting,
) {
    target.paragraph.alignment = source.alignment.map(Into::into).or(target.paragraph.alignment);
    target.paragraph.first_indent = source.first_indent.or(target.paragraph.first_indent);
    target.paragraph.leading = source.leading.or(target.paragraph.leading);
    target.paragraph.left_indent = source.left_indent.or(target.paragraph.left_indent);
    target.paragraph.right_indent = source.right_indent.or(target.paragraph.right_indent);
    target.paragraph.space_before = source.space_before.or(target.paragraph.space_before);
    target.paragraph.spacing = source.spacing.or(target.paragraph.spacing);
    target.starts_new_page = source.starts_new_page.or(target.starts_new_page);
}

fn apply_paragraph_patch(
    target: &mut SourceParagraphOverrides,
    patch: ParagraphFormattingPatch,
    base_font_size: u8,
) -> Result<(), FormattedTextError> {
    if let Some(value) = patch.alignment { target.paragraph.alignment = Some(value); }
    if let Some(value) = patch.first_indent { target.paragraph.first_indent = Some(value); }
    if let Some(value) = patch.leading { target.paragraph.leading = Some(value); }
    if let Some(value) = patch.left_indent { target.paragraph.left_indent = Some(value); }
    if let Some(value) = patch.right_indent { target.paragraph.right_indent = Some(value); }
    if let Some(lines) = patch.space_before_lines {
        let points = i64::from(base_font_size)
            .checked_mul(i64::from(lines))
            .ok_or(FormattedTextError::Model(crate::ModelError::InvalidNumber))?;
        target.paragraph.space_before = Some(FixedPoint::from_points(points));
    }
    if let Some(value) = patch.spacing { target.paragraph.spacing = Some(value); }
    if let Some(value) = patch.starts_new_page { target.starts_new_page = Some(value); }
    Ok(())
}

fn paragraph_patch_changes(
    current: SourceParagraphOverrides,
    patch: ParagraphFormattingPatch,
    base_font_size: u8,
) -> Result<bool, FormattedTextError> {
    if patch.alignment.is_some_and(|value| current.paragraph.alignment != Some(value))
        || patch.first_indent.is_some_and(|value| current.paragraph.first_indent != Some(value))
        || patch.leading.is_some_and(|value| current.paragraph.leading != Some(value))
        || patch.left_indent.is_some_and(|value| current.paragraph.left_indent != Some(value))
        || patch.right_indent.is_some_and(|value| current.paragraph.right_indent != Some(value))
        || patch.spacing.is_some_and(|value| current.paragraph.spacing != Some(value))
        || patch.starts_new_page.is_some_and(|value| current.starts_new_page != Some(value))
    {
        return Ok(true);
    }
    if let Some(lines) = patch.space_before_lines {
        let points = i64::from(base_font_size)
            .checked_mul(i64::from(lines))
            .ok_or(FormattedTextError::Model(crate::ModelError::InvalidNumber))?;
        if current.paragraph.space_before != Some(FixedPoint::from_points(points)) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn title_formatting_from_overrides(
    overrides: SourceParagraphOverrides,
) -> crate::TitlePageParagraphFormatting {
    crate::TitlePageParagraphFormatting {
        alignment: overrides.paragraph.alignment.map(Into::into),
        first_indent: overrides.paragraph.first_indent,
        leading: overrides.paragraph.leading,
        left_indent: overrides.paragraph.left_indent,
        right_indent: overrides.paragraph.right_indent,
        space_before: overrides.paragraph.space_before,
        spacing: overrides.paragraph.spacing,
        starts_new_page: overrides.starts_new_page,
    }
}

fn paragraph_source_attributes(
    overrides: SourceParagraphOverrides,
    definition_leading: &str,
    definition_spacing: u8,
) -> Vec<(&'static str, Option<String>)> {
    let spec = overrides.paragraph;
    let attribute = |name, value: Option<String>| (name, value);
    vec![
        attribute("Alignment", spec.alignment.map(|value| value.fdx_value().to_owned())),
        attribute("FirstIndent", spec.first_indent.map(format_inches)),
        attribute("Leading", Some(spec.leading.map_or_else(|| definition_leading.to_owned(), |value| value.fdx_value().to_owned()))),
        attribute("LeftIndent", spec.left_indent.map(format_inches)),
        attribute("RightIndent", spec.right_indent.map(format_inches)),
        attribute("SpaceBefore", spec.space_before.map(FixedPoint::decimal_string)),
        attribute("Spacing", Some(spec.spacing.map_or_else(|| definition_spacing.to_string(), |value| value.fdx_value().to_owned()))),
        attribute("StartsNewPage", overrides.starts_new_page.map(|value| if value { "Yes" } else { "No" }.to_owned())),
    ]
}

fn format_inches(value: FixedPoint) -> String {
    let raw = value.raw();
    const PER_INCH: u64 = 7_200;
    let magnitude = raw.unsigned_abs();
    let mut whole = magnitude / PER_INCH;
    let mut fraction = (magnitude % PER_INCH * 1_000_000 + PER_INCH / 2) / PER_INCH;
    if fraction == 1_000_000 { whole += 1; fraction = 0; }
    let sign = if raw < 0 { "-" } else { "" };
    if fraction == 0 { return format!("{sign}{whole}"); }
    let mut fraction = format!("{fraction:06}");
    while fraction.ends_with('0') { fraction.pop(); }
    format!("{sign}{whole}.{fraction}")
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
