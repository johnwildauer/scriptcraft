//! Headless inline-style, source-range, and direct-alignment helpers shared by
//! screenplay body paragraphs and title-page paragraphs.

use crate::preservation::PreservationError;
use crate::{
    ModelError, ParagraphAlignment, Project, RunAttribute, TextRun, TitlePageAlignment,
    MAX_RUNS_PER_ELEMENT,
};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunStyle {
    Bold,
    Italics,
    Underline,
    AllCaps,
    Strikethrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunBoundaryAffinity {
    Left,
    Right,
}

impl RunStyle {
    pub const fn canonical_token(self) -> &'static str {
        match self {
            Self::Bold => "Bold",
            Self::Italics => "Italic",
            Self::Underline => "Underline",
            Self::AllCaps => "AllCaps",
            Self::Strikethrough => "Strikethrough",
        }
    }

    fn matches_token(self, token: &str) -> bool {
        token == self.canonical_token()
            || (self == Self::Strikethrough && token == "StrikeThrough")
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunStyleFlags {
    pub bold: bool,
    pub italics: bool,
    pub underline: bool,
    pub all_caps: bool,
    pub strikethrough: bool,
}

impl RunStyleFlags {
    pub const fn contains(self, style: RunStyle) -> bool {
        match style {
            RunStyle::Bold => self.bold,
            RunStyle::Italics => self.italics,
            RunStyle::Underline => self.underline,
            RunStyle::AllCaps => self.all_caps,
            RunStyle::Strikethrough => self.strikethrough,
        }
    }

    pub fn set(&mut self, style: RunStyle, enabled: bool) {
        match style {
            RunStyle::Bold => self.bold = enabled,
            RunStyle::Italics => self.italics = enabled,
            RunStyle::Underline => self.underline = enabled,
            RunStyle::AllCaps => self.all_caps = enabled,
            RunStyle::Strikethrough => self.strikethrough = enabled,
        }
    }

    pub fn toggled(mut self, style: RunStyle) -> Self {
        self.set(style, !self.contains(style));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStyleProjection {
    pub flags: RunStyleFlags,
    /// Unrecognized tokens retain their exact spelling and relative order.
    pub unknown_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormattedTextError {
    InvalidUtf8Boundary,
    InvalidRunProjection,
    AmbiguousStyleAttribute,
    TooManyRuns,
    ParagraphOutOfRange,
    EmptyParagraphSelection,
    DuplicateParagraphTarget,
    AmbiguousDirectAttribute,
    Preservation(PreservationError),
    Model(ModelError),
}

impl From<PreservationError> for FormattedTextError {
    fn from(error: PreservationError) -> Self {
        Self::Preservation(error)
    }
}

impl From<ModelError> for FormattedTextError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParagraphTarget {
    Body(usize),
    TitlePage(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParagraphAlignmentProjection {
    pub alignment: Option<ParagraphAlignment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStyleEdit {
    pub runs: Vec<TextRun>,
    /// Collapsed carets update this transient projection; callers own its
    /// lifetime and must not persist it or fabricate an empty Text child.
    pub active_style: RunStyleFlags,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStyleRange<'a> {
    pub canonical_text: &'a str,
    pub runs: &'a [TextRun],
    pub range: Range<usize>,
}

/// Parse source `Style` tokens without normalizing unknown token spellings.
pub fn parse_style_attribute(
    attributes: &[RunAttribute],
) -> Result<RunStyleProjection, FormattedTextError> {
    let mut style_values = attributes.iter().filter(|attribute| attribute.name == "Style");
    let Some(style_attribute) = style_values.next() else {
        return Ok(RunStyleProjection {
            flags: RunStyleFlags::default(),
            unknown_tokens: Vec::new(),
        });
    };
    if style_values.next().is_some() {
        return Err(FormattedTextError::AmbiguousStyleAttribute);
    }

    let mut projection = RunStyleProjection {
        flags: RunStyleFlags::default(),
        unknown_tokens: Vec::new(),
    };
    if style_attribute.value.is_empty() {
        return Ok(projection);
    }
    for token in style_attribute.value.split('+') {
        if let Some(style) = supported_style(token) {
            projection.flags.set(style, true);
        } else {
            projection.unknown_tokens.push(token.to_owned());
        }
    }
    Ok(projection)
}

fn supported_style(token: &str) -> Option<RunStyle> {
    [
        RunStyle::Bold,
        RunStyle::Italics,
        RunStyle::Underline,
        RunStyle::AllCaps,
        RunStyle::Strikethrough,
    ]
    .into_iter()
    .find(|style| style.matches_token(token))
}

fn update_run_style(
    attributes: &[RunAttribute],
    style: RunStyle,
    enabled: bool,
) -> Result<Vec<RunAttribute>, FormattedTextError> {
    let mut result = attributes.to_vec();
    let style_index = result
        .iter()
        .position(|attribute| attribute.name == "Style");
    if result
        .iter()
        .filter(|attribute| attribute.name == "Style")
        .count()
        > 1
    {
        return Err(FormattedTextError::AmbiguousStyleAttribute);
    }

    let original = style_index
        .map(|index| result[index].value.as_str())
        .unwrap_or("");
    let mut tokens = if original.is_empty() {
        Vec::new()
    } else {
        original.split('+').map(str::to_owned).collect::<Vec<_>>()
    };
    let had_style = tokens.iter().any(|token| style.matches_token(token));
    tokens.retain(|token| !style.matches_token(token));
    if enabled {
        // Preserve existing token order; append only the intentionally changed
        // style, using the bounded canonical FDX spelling.
        tokens.push(style.canonical_token().to_owned());
    } else if !had_style && style_index.is_none() {
        return Ok(result);
    }
    let value = tokens.join("+");
    if let Some(index) = style_index {
        result[index].value = value;
    } else {
        result.push(RunAttribute {
            name: "Style".to_owned(),
            value,
        });
    }
    Ok(result)
}

fn effective_runs(text: &str, runs: &[TextRun]) -> Result<Vec<TextRun>, FormattedTextError> {
    if runs.is_empty() {
        return Ok(if text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun {
                text: text.to_owned(),
                attributes: Vec::new(),
            }]
        });
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != text {
        return Err(FormattedTextError::InvalidRunProjection);
    }
    Ok(runs.to_vec())
}

fn push_run_coalescing(runs: &mut Vec<TextRun>, text: &str, attributes: &[RunAttribute]) {
    if text.is_empty() {
        return;
    }
    if let Some(previous) = runs.last_mut() {
        if previous.attributes == attributes {
            previous.text.push_str(text);
            return;
        }
    }
    runs.push(TextRun {
        text: text.to_owned(),
        attributes: attributes.to_vec(),
    });
}

fn any_or_all_selected_style(
    text: &str,
    runs: &[TextRun],
    range: &Range<usize>,
    style: RunStyle,
) -> Result<(bool, bool), FormattedTextError> {
    if range.start > range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return Err(FormattedTextError::InvalidUtf8Boundary);
    }
    if range.start == range.end {
        return Ok((false, false));
    }
    let mut cursor = 0usize;
    let mut any = false;
    let mut all = true;
    for run in runs {
        let run_end = cursor.saturating_add(run.text.len());
        let start = cursor.max(range.start);
        let end = run_end.min(range.end);
        if start < end {
            let has_style = parse_style_attribute(&run.attributes)?.flags.contains(style);
            any |= has_style;
            all &= has_style;
        }
        cursor = run_end;
    }
    Ok((any, all))
}

/// Apply the uniform toggle policy over a canonical UTF-8 byte range. A
/// collapsed range changes only transient active typing style. For a selection,
/// the style is removed iff every selected character has it; otherwise it is
/// added. Adjacent output runs merge only when their complete ordered attribute
/// vectors match.
pub fn toggle_style_for_range(
    canonical_text: &str,
    runs: &[TextRun],
    range: Range<usize>,
    active_style: RunStyleFlags,
    style: RunStyle,
) -> Result<RunStyleEdit, FormattedTextError> {
    let source_runs = effective_runs(canonical_text, runs)?;
    validate_range(canonical_text, &range)?;
    if range.is_empty() {
        return Ok(RunStyleEdit {
            runs: source_runs,
            active_style: active_style.toggled(style),
            changed: false,
        });
    }
    let (_, every_selected_character_has_style) =
        any_or_all_selected_style(canonical_text, &source_runs, &range, style)?;
    transform_style_range(
        canonical_text,
        &source_runs,
        range,
        active_style,
        style,
        !every_selected_character_has_style,
    )
}

fn validate_range(text: &str, range: &Range<usize>) -> Result<(), FormattedTextError> {
    if range.start > range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return Err(FormattedTextError::InvalidUtf8Boundary);
    }
    Ok(())
}

fn transform_style_range(
    canonical_text: &str,
    source_runs: &[TextRun],
    range: Range<usize>,
    active_style: RunStyleFlags,
    style: RunStyle,
    enabled: bool,
) -> Result<RunStyleEdit, FormattedTextError> {
    if range.is_empty() {
        return Ok(RunStyleEdit {
            runs: source_runs.to_vec(),
            active_style,
            changed: false,
        });
    }
    let mut transformed = Vec::with_capacity(source_runs.len().saturating_add(2));
    let mut global_start = 0usize;
    for run in source_runs {
        let global_end = global_start.saturating_add(run.text.len());
        let local_start = range.start.max(global_start).min(global_end) - global_start;
        let local_end = range.end.max(global_start).min(global_end) - global_start;
        if local_start == local_end {
            push_run_coalescing(&mut transformed, &run.text, &run.attributes);
        } else {
            push_run_coalescing(
                &mut transformed,
                &run.text[..local_start],
                &run.attributes,
            );
            let current_has_style = parse_style_attribute(&run.attributes)?.flags.contains(style);
            let updated = if current_has_style == enabled {
                run.attributes.clone()
            } else {
                update_run_style(&run.attributes, style, enabled)?
            };
            push_run_coalescing(
                &mut transformed,
                &run.text[local_start..local_end],
                &updated,
            );
            push_run_coalescing(
                &mut transformed,
                &run.text[local_end..],
                &run.attributes,
            );
        }
        global_start = global_end;
    }
    if transformed.len() > MAX_RUNS_PER_ELEMENT {
        return Err(FormattedTextError::TooManyRuns);
    }
    Ok(RunStyleEdit {
        runs: transformed,
        active_style,
        changed: true,
    })
}

/// Apply one all-on/all-off decision across every noncollapsed paragraph range.
/// This prevents individually mixed paragraphs from choosing contradictory
/// directions when one editor selection spans several paragraphs.
pub fn toggle_style_over_ranges(
    selections: &[RunStyleRange<'_>],
    active_style: RunStyleFlags,
    style: RunStyle,
) -> Result<Vec<RunStyleEdit>, FormattedTextError> {
    if selections.is_empty() {
        return Err(FormattedTextError::EmptyParagraphSelection);
    }
    let mut all_selected_have_style = true;
    let mut has_selected_characters = false;
    let mut effective = Vec::with_capacity(selections.len());
    for selection in selections {
        validate_range(selection.canonical_text, &selection.range)?;
        let runs = effective_runs(selection.canonical_text, selection.runs)?;
        if !selection.range.is_empty() {
            let (_, all_here) = any_or_all_selected_style(
                selection.canonical_text,
                &runs,
                &selection.range,
                style,
            )?;
            has_selected_characters = true;
            all_selected_have_style &= all_here;
        }
        effective.push(runs);
    }
    if !has_selected_characters {
        return Ok(effective
            .into_iter()
            .map(|runs| RunStyleEdit {
                runs,
                active_style: active_style.toggled(style),
                changed: false,
            })
            .collect());
    }
    let enabled = !all_selected_have_style;
    selections
        .iter()
        .zip(effective.iter())
        .map(|(selection, runs)| {
            transform_style_range(
                selection.canonical_text,
                runs,
                selection.range.clone(),
                active_style,
                style,
                enabled,
            )
        })
        .collect()
}

/// Removes only the five supported run styles from one canonical UTF-8 range.
/// Unknown Style tokens and all non-Style attributes are retained verbatim.
pub fn clear_supported_styles_for_range(
    canonical_text: &str,
    runs: &[TextRun],
    range: Range<usize>,
) -> Result<RunStyleEdit, FormattedTextError> {
    let source_runs = effective_runs(canonical_text, runs)?;
    validate_range(canonical_text, &range)?;
    if range.is_empty() {
        return Ok(RunStyleEdit {
            runs: source_runs,
            active_style: RunStyleFlags::default(),
            changed: false,
        });
    }

    let mut transformed = Vec::with_capacity(source_runs.len().saturating_add(2));
    let mut global_start = 0usize;
    for run in &source_runs {
        let global_end = global_start.saturating_add(run.text.len());
        let local_start = range.start.max(global_start).min(global_end) - global_start;
        let local_end = range.end.max(global_start).min(global_end) - global_start;
        if local_start == local_end {
            push_run_coalescing(&mut transformed, &run.text, &run.attributes);
        } else {
            push_run_coalescing(&mut transformed, &run.text[..local_start], &run.attributes);
            let mut attributes = run.attributes.clone();
            for style in [
                RunStyle::Bold,
                RunStyle::Italics,
                RunStyle::Underline,
                RunStyle::AllCaps,
                RunStyle::Strikethrough,
            ] {
                attributes = update_run_style(&attributes, style, false)?;
            }
            push_run_coalescing(
                &mut transformed,
                &run.text[local_start..local_end],
                &attributes,
            );
            push_run_coalescing(&mut transformed, &run.text[local_end..], &run.attributes);
        }
        global_start = global_end;
    }
    if transformed.len() > MAX_RUNS_PER_ELEMENT {
        return Err(FormattedTextError::TooManyRuns);
    }
    let changed = transformed != source_runs;
    Ok(RunStyleEdit {
        runs: transformed,
        active_style: RunStyleFlags::default(),
        changed,
    })
}

/// Clears supported styles for each paragraph range without deciding style
/// directions independently. Collapsed ranges carry the cleared typing style.
pub fn clear_supported_styles_over_ranges(
    selections: &[RunStyleRange<'_>],
) -> Result<Vec<RunStyleEdit>, FormattedTextError> {
    if selections.is_empty() {
        return Err(FormattedTextError::EmptyParagraphSelection);
    }
    selections
        .iter()
        .map(|selection| {
            clear_supported_styles_for_range(
                selection.canonical_text,
                selection.runs,
                selection.range.clone(),
            )
        })
        .collect()
}

/// Project toolbar lit state: any intersecting run lights a style; a collapsed
/// caret uses the caller-owned active run/typing flags.
pub fn project_lit_styles(
    canonical_text: &str,
    runs: &[TextRun],
    range: Range<usize>,
    active_style: RunStyleFlags,
) -> Result<RunStyleFlags, FormattedTextError> {
    let source_runs = effective_runs(canonical_text, runs)?;
    if range.start > range.end
        || range.end > canonical_text.len()
        || !canonical_text.is_char_boundary(range.start)
        || !canonical_text.is_char_boundary(range.end)
    {
        return Err(FormattedTextError::InvalidUtf8Boundary);
    }
    if range.is_empty() {
        return Ok(active_style);
    }
    let mut result = RunStyleFlags::default();
    let mut cursor = 0usize;
    for run in source_runs {
        let end = cursor.saturating_add(run.text.len());
        if cursor < range.end && end > range.start {
            let flags = parse_style_attribute(&run.attributes)?.flags;
            for style in [
                RunStyle::Bold,
                RunStyle::Italics,
                RunStyle::Underline,
                RunStyle::AllCaps,
                RunStyle::Strikethrough,
            ] {
                if flags.contains(style) {
                    result.set(style, true);
                }
            }
        }
        cursor = end;
    }
    Ok(result)
}

/// Resolve the run style at a collapsed byte caret. At an interior boundary,
/// `Left` chooses the run ending there and `Right` chooses the run starting
/// there; document endpoints fall back to the only adjacent run. Empty text
/// returns the caller's transient typing style.
pub fn project_style_at_caret(
    canonical_text: &str,
    runs: &[TextRun],
    byte_offset: usize,
    affinity: RunBoundaryAffinity,
    active_typing_style: RunStyleFlags,
) -> Result<RunStyleFlags, FormattedTextError> {
    if byte_offset > canonical_text.len() || !canonical_text.is_char_boundary(byte_offset) {
        return Err(FormattedTextError::InvalidUtf8Boundary);
    }
    if canonical_text.is_empty() {
        return Ok(active_typing_style);
    }
    let source_runs = effective_runs(canonical_text, runs)?;
    let mut cursor = 0usize;
    let mut left_style = None;
    let mut right_style = None;
    for run in source_runs {
        let end = cursor.saturating_add(run.text.len());
        if cursor < byte_offset && byte_offset < end {
            return Ok(parse_style_attribute(&run.attributes)?.flags);
        }
        if end == byte_offset && !run.text.is_empty() {
            left_style = Some(parse_style_attribute(&run.attributes)?.flags);
        }
        if cursor == byte_offset && !run.text.is_empty() {
            right_style = Some(parse_style_attribute(&run.attributes)?.flags);
        }
        cursor = end;
    }
    Ok(match affinity {
        RunBoundaryAffinity::Left => left_style.or(right_style),
        RunBoundaryAffinity::Right => right_style.or(left_style),
    }
    .unwrap_or(active_typing_style))
}

/// Union-style projection across a cross-paragraph selection. Collapsed ranges
/// defer to active typing style only when no range selects characters.
pub fn project_lit_styles_over_ranges(
    selections: &[RunStyleRange<'_>],
    active_style: RunStyleFlags,
) -> Result<RunStyleFlags, FormattedTextError> {
    if selections.is_empty() {
        return Ok(active_style);
    }
    let mut result = RunStyleFlags::default();
    let mut has_selected_characters = false;
    for selection in selections {
        let projected = project_lit_styles(
            selection.canonical_text,
            selection.runs,
            selection.range.clone(),
            active_style,
        )?;
        if !selection.range.is_empty() {
            has_selected_characters = true;
            result.bold |= projected.bold;
            result.italics |= projected.italics;
            result.underline |= projected.underline;
            result.all_caps |= projected.all_caps;
            result.strikethrough |= projected.strikethrough;
        }
    }
    Ok(if has_selected_characters {
        result
    } else {
        active_style
    })
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
                    .get_mut(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                let id = element.id.clone();
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
        alignments.len() > 0 && alignments.iter().all(|alignment| *alignment == Some(*first))
    });
    ParagraphAlignmentProjection { alignment: uniform }
}

impl ParagraphAlignment {
    pub const fn fdx_value(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Center => "Center",
            Self::Justified => "Full",
        }
    }

    pub fn from_fdx(value: &str) -> Option<Self> {
        match value {
            "Left" => Some(Self::Left),
            "Right" => Some(Self::Right),
            "Center" => Some(Self::Center),
            "Full" => Some(Self::Justified),
            _ => None,
        }
    }
}

impl From<ParagraphAlignment> for TitlePageAlignment {
    fn from(value: ParagraphAlignment) -> Self {
        match value {
            ParagraphAlignment::Left => Self::Left,
            ParagraphAlignment::Right => Self::Right,
            ParagraphAlignment::Center => Self::Center,
            ParagraphAlignment::Justified => Self::Justified,
        }
    }
}

impl From<TitlePageAlignment> for ParagraphAlignment {
    fn from(value: TitlePageAlignment) -> Self {
        match value {
            TitlePageAlignment::Left => Self::Left,
            TitlePageAlignment::Right => Self::Right,
            TitlePageAlignment::Center => Self::Center,
            TitlePageAlignment::Justified => Self::Justified,
        }
    }
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
            (element.id.clone(), element.text.clone(), element.runs.clone())
        }
        ParagraphTarget::TitlePage(index) => {
            let paragraph = candidate
                .title_page
                .get(index)
                .ok_or(FormattedTextError::ParagraphOutOfRange)?;
            (paragraph.id.clone(), paragraph.text.clone(), paragraph.runs.clone())
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
                candidate.elements[index].runs = edit.runs.clone();
                candidate.elements[index].text = text.clone();
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
                (element.id.clone(), element.text.clone(), element.runs.clone())
            }
            ParagraphTarget::TitlePage(index) => {
                let paragraph = candidate
                    .title_page
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (paragraph.id.clone(), paragraph.text.clone(), paragraph.runs.clone())
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
                candidate.elements[index].text = text.clone();
                candidate.elements[index].runs = edit.runs.clone();
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
                (element.id.clone(), element.text.clone(), element.runs.clone())
            }
            ParagraphTarget::TitlePage(index) => {
                let paragraph = candidate
                    .title_page
                    .get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (paragraph.id.clone(), paragraph.text.clone(), paragraph.runs.clone())
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
                candidate.elements[index].text = text.clone();
                candidate.elements[index].runs = edit.runs.clone();
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
