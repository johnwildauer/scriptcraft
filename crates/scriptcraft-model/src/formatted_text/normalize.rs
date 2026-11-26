//! Shared canonical inline-run projection primitives.

use super::runs::{parse_style_attribute, FormattedTextError, RunStyle};
use crate::{
    dialogue_alts::DialogueAlternative,
    element::{
        BodyOwner, RunAttribute, TextRun, MAX_RUN_ATTRIBUTES, MAX_RUN_ATTRIBUTE_BYTES,
        MAX_RUNS_PER_ELEMENT,
    },
    element_definition::ElementDefinition,
    formatting::FixedPoint,
    Project,
    MAX_TEXT_BYTES,
};

const MIN_INLINE_SIZE_RAW: i64 = FixedPoint::SCALE;
const MAX_INLINE_SIZE_RAW: i64 = 1_000 * FixedPoint::SCALE;

/// Semantic interpretation of a direct run Size attribute.
///
/// Valid values retain their original lexical spelling for source projection;
/// invalid values remain source-owned and compare only with the same spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineSizeProjection {
    Inherited,
    Valid {
        points: FixedPoint,
        source_spelling: String,
    },
    Invalid {
        source_spelling: String,
    },
}

impl InlineSizeProjection {
    pub fn parse(attributes: &[RunAttribute]) -> Result<Self, FormattedTextError> {
        let mut sizes = attributes.iter().filter(|attribute| attribute.name == "Size");
        let Some(size) = sizes.next() else {
            return Ok(Self::Inherited);
        };
        if sizes.next().is_some() {
            return Err(FormattedTextError::AmbiguousInlineAttribute);
        }

        let source_spelling = size.value.clone();
        Ok(match FixedPoint::parse_points(&source_spelling) {
            Some(points) if (MIN_INLINE_SIZE_RAW..=MAX_INLINE_SIZE_RAW).contains(&points.raw()) => {
                Self::Valid {
                    points,
                    source_spelling,
                }
            }
            _ => Self::Invalid { source_spelling },
        })
    }

    fn semantic_key(&self) -> InlineSizeKey<'_> {
        match self {
            Self::Inherited => InlineSizeKey::Inherited,
            Self::Valid { points, .. } => InlineSizeKey::Valid(*points),
            Self::Invalid { source_spelling } => InlineSizeKey::Invalid(source_spelling),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InlineSizeKey<'a> {
    Inherited,
    Valid(FixedPoint),
    Invalid(&'a str),
}

/// Produce deterministic decimal spelling for a newly assigned bounded Size.
/// Imported valid values should retain their `source_spelling` instead.
pub fn canonical_inline_size_spelling(points: FixedPoint) -> Option<String> {
    (MIN_INLINE_SIZE_RAW..=MAX_INLINE_SIZE_RAW)
        .contains(&points.raw())
        .then(|| points.decimal_string())
}

/// Canonical Style value, with known flags in the approved order and unknown
/// tokens alphabetized while retaining duplicates and exact spellings.
pub fn canonical_style_value(
    projection: &super::runs::RunStyleProjection,
) -> Option<String> {
    let mut tokens = Vec::new();
    for style in [
        RunStyle::Bold,
        RunStyle::Italics,
        RunStyle::Underline,
        RunStyle::Strikethrough,
        RunStyle::AllCaps,
    ] {
        if projection.flags.contains(style) {
            tokens.push(style.canonical_token().to_owned());
        }
    }
    let mut unknown = projection.unknown_tokens.clone();
    unknown.sort();
    tokens.extend(unknown);
    (!tokens.is_empty()).then(|| tokens.join("+"))
}

/// Canonicalize one ordered run attribute vector without changing the relative
/// order or spelling of non-Style attributes.
pub fn canonicalize_run_attributes(
    attributes: &[RunAttribute],
) -> Result<Vec<RunAttribute>, FormattedTextError> {
    validate_attributes(attributes)?;
    let projection = parse_style_attribute(attributes)?;
    InlineSizeProjection::parse(attributes)?;
    let canonical = canonical_style_value(&projection);
    let style_index = attributes
        .iter()
        .position(|attribute| attribute.name == "Style");
    let mut result = attributes.to_vec();
    if let Some(index) = style_index {
        match canonical {
            Some(value) => result[index].value = value,
            None => {
                result.remove(index);
            }
        }
    }
    validate_attributes(&result)?;
    Ok(result)
}

/// Insert a newly generated known Size or Style attribute at its deterministic
/// boundary. Existing attributes are never reordered; other new attributes
/// append after the retained sequence.
pub fn insert_run_attribute(
    attributes: &mut Vec<RunAttribute>,
    attribute: RunAttribute,
) -> Result<(), FormattedTextError> {
    if attributes.iter().any(|existing| existing.name == attribute.name) {
        return Err(if attribute.name == "Style" {
            FormattedTextError::AmbiguousStyleAttribute
        } else {
            FormattedTextError::AmbiguousInlineAttribute
        });
    }
    let index = match attribute.name.as_str() {
        "Size" => attributes
            .iter()
            .position(|existing| existing.name == "Style")
            .unwrap_or(attributes.len()),
        "Style" => attributes.len(),
        _ => attributes.len(),
    };
    attributes.insert(index, attribute);
    validate_attributes(attributes)
}

/// Normalize a paragraph's run vector without changing its concatenated text.
/// This is an owned, idempotent operation; a legacy nonempty plain-text
/// paragraph with no runs becomes one unformatted run.
pub fn normalize_runs(
    canonical_text: &str,
    runs: &[TextRun],
) -> Result<Vec<TextRun>, FormattedTextError> {
    if canonical_text.len() > MAX_TEXT_BYTES {
        return Err(FormattedTextError::TextTooLong);
    }

    if runs.is_empty() {
        return Ok(if canonical_text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun {
                text: canonical_text.to_owned(),
                attributes: Vec::new(),
            }]
        });
    }

    let mut source_text = String::with_capacity(canonical_text.len());
    for run in runs {
        let projected_len = source_text.len().saturating_add(run.text.len());
        if projected_len > MAX_TEXT_BYTES {
            return Err(FormattedTextError::TextTooLong);
        }
        if projected_len > canonical_text.len() {
            return Err(FormattedTextError::InvalidRunProjection);
        }
        source_text.push_str(&run.text);
    }
    if source_text != canonical_text {
        return Err(FormattedTextError::InvalidRunProjection);
    }

    let mut normalized: Vec<TextRun> = Vec::with_capacity(runs.len());
    for run in runs.iter().filter(|run| !run.text.is_empty()) {
        let attributes = canonicalize_run_attributes(&run.attributes)?;
        if let Some(previous) = normalized.last_mut() {
            if equivalent_attributes(&previous.attributes, &attributes)? {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        normalized.push(TextRun {
            text: run.text.clone(),
            attributes,
        });
        if normalized.len() > MAX_RUNS_PER_ELEMENT {
            return Err(FormattedTextError::TooManyRuns);
        }
    }

    let mut normalized_text = String::with_capacity(canonical_text.len());
    for run in &normalized {
        normalized_text.push_str(&run.text);
    }
    if normalized_text != canonical_text {
        return Err(FormattedTextError::InvalidRunProjection);
    }
    if normalized
        .iter()
        .fold(0usize, |count, run| count.saturating_add(run.attributes.len()))
        > MAX_RUN_ATTRIBUTES
    {
        return Err(FormattedTextError::TooManyRunAttributes);
    }
    Ok(normalized)
}

/// Returns complete Text defaults in the established AltCollection attribute
/// order. Color is present only when the Dialogue definition owns a color.
pub fn dialogue_default_run_attributes(definition: &ElementDefinition) -> Vec<RunAttribute> {
    let font = &definition.font;
    let style = [
        (font.bold, "Bold"),
        (font.italic, "Italic"),
        (font.underline, "Underline"),
        (font.all_caps, "AllCaps"),
        (font.hidden_text, "HiddenText"),
    ]
    .into_iter()
    .filter_map(|(enabled, token)| enabled.then_some(token))
    .collect::<Vec<_>>()
    .join("+");
    let mut attributes = vec![
        RunAttribute { name: "AdornmentStyle".into(), value: font.adornment_style.to_string() },
        RunAttribute { name: "Font".into(), value: font.family.clone() },
        RunAttribute { name: "RevisionID".into(), value: font.revision_id.to_string() },
        RunAttribute { name: "Size".into(), value: font.size.to_string() },
        RunAttribute { name: "Style".into(), value: style },
    ];
    if let Some(color) = &font.color {
        attributes.push(RunAttribute { name: "Color".into(), value: color.clone() });
    }
    attributes
}

/// Projects owner/canonical Text runs into the sparse semantic Alt form.
/// Known defaults are removed only on exact effective-value equality.
pub fn sparse_alt_run_projection(
    definition: &ElementDefinition,
    canonical_text: &str,
    runs: &[TextRun],
) -> Result<Vec<TextRun>, FormattedTextError> {
    let defaults = dialogue_default_run_attributes(definition);
    let normalized = normalize_runs(canonical_text, runs)?;
    let sparse = normalized
        .into_iter()
        .map(|mut run| {
            run.attributes.retain(|attribute| {
                !defaults.iter().any(|default| {
                    default.name == attribute.name && default.value == attribute.value
                })
            });
            run
        })
        .collect::<Vec<_>>();
    normalize_runs(canonical_text, &sparse)
}

/// Materializes a sparse semantic Alt run using complete Dialogue Text
/// defaults, overlaying actual attributes without losing unknown source data.
pub fn complete_alt_run_projection(
    definition: &ElementDefinition,
    canonical_text: &str,
    runs: &[TextRun],
) -> Result<Vec<TextRun>, FormattedTextError> {
    let sparse = sparse_alt_run_projection(definition, canonical_text, runs)?;
    let defaults = dialogue_default_run_attributes(definition);
    let mut complete = Vec::with_capacity(sparse.len());
    for run in sparse {
        let mut attributes = defaults.clone();
        let mut replaced = std::collections::BTreeSet::new();
        let mut trailing = Vec::new();
        for actual in run.attributes {
            if let Some(index) = attributes
                .iter()
                .position(|default| default.name == actual.name)
            {
                if replaced.insert(actual.name.clone()) {
                    attributes[index] = actual;
                } else {
                    trailing.push(actual);
                }
            } else {
                trailing.push(actual);
            }
        }
        attributes.extend(trailing);
        complete.push(TextRun { text: run.text, attributes });
    }
    normalize_runs(canonical_text, &complete)
}

/// Build a detached, whole-Project canonical run candidate and project it into
/// retained managed Text nodes. The input Project and its shared Body owners
/// are never mutated; callers publish or serialize only the returned candidate.
pub fn normalize_project_runs(project: &Project) -> Result<Project, FormattedTextError> {
    let mut candidate = project.clone();
    let dialogue_definition = candidate
        .element_definitions()
        .get_by_label("Dialogue")
        .cloned();
    for owner in &mut candidate.elements {
        let normalized = normalize_runs(&owner.text, &owner.runs)?;
        let alternatives = owner
            .dialogue_alternatives
            .as_ref()
            .map(|alternatives| {
                alternatives
                    .inactive
                    .iter()
                    .map(|alternative| {
                        let runs = if let Some(definition) = &dialogue_definition {
                            sparse_alt_run_projection(definition, &alternative.text, &alternative.runs)?
                        } else {
                            normalize_runs(&alternative.text, &alternative.runs)?
                        };
                        Ok(DialogueAlternative {
                            id: alternative.id,
                            text: alternative.text.clone(),
                            runs,
                        })
                    })
                    .collect::<Result<Vec<_>, FormattedTextError>>()
            })
            .transpose()?;
        BodyOwner::update(owner, |element| {
            element.runs = normalized;
            if let (Some(existing), Some(normalized)) =
                (element.dialogue_alternatives.as_mut(), alternatives)
            {
                existing.inactive = normalized;
            }
        });
    }
    for paragraph in &mut candidate.title_page {
        paragraph.runs = normalize_runs(&paragraph.text, &paragraph.runs)?;
    }
    let mut script_notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
    for note in &mut script_notes {
        for paragraph in &mut note.paragraphs {
            paragraph.normalize()?;
        }
    }
    candidate
        .replace_script_notes(crate::script_notes::ScriptNotes::new(script_notes))
        .map_err(FormattedTextError::Model)?;

    if let Some(preservation) = candidate.preservation.as_mut() {
        for owner in &candidate.elements {
            if preservation.has_managed_element(&owner.id) {
                preservation.sync_managed_text_runs(&owner.id, &owner.text, &owner.runs)?;
            }
        }
        for paragraph in &candidate.title_page {
            if preservation.has_managed_element(&paragraph.id) {
                preservation.sync_managed_text_runs(
                    &paragraph.id,
                    &paragraph.text,
                    &paragraph.runs,
                )?;
            }
        }
    }
    Ok(candidate)
}

/// Compare run attributes using the canonical shared Style and Size semantics.
/// Other retained attributes still require exact ordered equality.
pub fn run_attributes_equivalent(
    left: &[RunAttribute],
    right: &[RunAttribute],
) -> Result<bool, FormattedTextError> {
    equivalent_attributes(left, right)
}

/// Determine whether a source run's unknown attributes and unknown Style
/// tokens remain represented by a canonical output run. Supported formatting
/// fields may legitimately differ after an edit; retained unknown data may not.
pub fn run_attributes_preserve_unknown_data(
    source: &[RunAttribute],
    output: &[RunAttribute],
) -> Result<bool, FormattedTextError> {
    let source_style = parse_style_attribute(source)?;
    let output_style = parse_style_attribute(output)?;
    if sorted_unknowns(source_style.unknown_tokens)
        != sorted_unknowns(output_style.unknown_tokens)
    {
        return Ok(false);
    }
    let source_retained = source
        .iter()
        .filter(|attribute| {
            !matches!(attribute.name.as_str(), "Style" | "Size" | "Font" | "Color" | "Background")
        })
        .map(|attribute| (attribute.name.as_str(), attribute.value.as_str()))
        .collect::<Vec<_>>();
    let output_retained = output
        .iter()
        .filter(|attribute| {
            !matches!(attribute.name.as_str(), "Style" | "Size" | "Font" | "Color" | "Background")
        })
        .map(|attribute| (attribute.name.as_str(), attribute.value.as_str()))
        .collect::<Vec<_>>();
    Ok(source_retained == output_retained)
}

fn equivalent_attributes(
    left: &[RunAttribute],
    right: &[RunAttribute],
) -> Result<bool, FormattedTextError> {
    let left_style = parse_style_attribute(left)?;
    let right_style = parse_style_attribute(right)?;
    let left_size = InlineSizeProjection::parse(left)?;
    let right_size = InlineSizeProjection::parse(right)?;
    if left_style.flags != right_style.flags
        || sorted_unknowns(left_style.unknown_tokens) != sorted_unknowns(right_style.unknown_tokens)
        || left_size.semantic_key() != right_size.semantic_key()
    {
        return Ok(false);
    }
    let left_retained = retained_attributes(left);
    let right_retained = retained_attributes(right);
    Ok(left_retained == right_retained)
}

fn sorted_unknowns(mut tokens: Vec<String>) -> Vec<String> {
    tokens.sort();
    tokens
}

fn retained_attributes(attributes: &[RunAttribute]) -> Vec<(&str, &str)> {
    attributes
        .iter()
        .filter(|attribute| attribute.name != "Style" && attribute.name != "Size")
        .map(|attribute| (attribute.name.as_str(), attribute.value.as_str()))
        .collect()
}

fn validate_attributes(attributes: &[RunAttribute]) -> Result<(), FormattedTextError> {
    if attributes.len() > MAX_RUN_ATTRIBUTES {
        return Err(FormattedTextError::TooManyRunAttributes);
    }
    if attributes.iter().any(|attribute| {
        attribute.name.len() > MAX_RUN_ATTRIBUTE_BYTES
            || attribute.value.len() > MAX_RUN_ATTRIBUTE_BYTES
    }) {
        return Err(FormattedTextError::RunAttributeTooLarge);
    }
    Ok(())
}
