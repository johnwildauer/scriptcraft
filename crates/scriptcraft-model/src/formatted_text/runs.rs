//! Canonical run-style primitives and UTF-8 range projections.

use crate::{
    element::{BodyOwner, RunAttribute, TextRun},
    error::ModelError,
    preservation::PreservationError,
    project::Project,
};
use super::normalize::{canonical_style_value, canonicalize_run_attributes, insert_run_attribute, normalize_runs};
use super::paragraphs::ParagraphTarget;
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
            Self::Strikethrough => "Strikeout",
        }
    }

    fn matches_token(self, token: &str) -> bool {
        token == self.canonical_token()
            || (self == Self::Strikethrough
                && matches!(token, "Strikethrough" | "StrikeThrough"))
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
    /// Unrecognized tokens retain exact source spelling/order until canonical emission.
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
    AmbiguousInlineAttribute,
    TooManyRunAttributes,
    RunAttributeTooLarge,
    TextTooLong,
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

/// Bounded inline font families accepted by the initial formatting controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineFont {
    CourierFinalDraft,
    CourierPrime,
    Arial,
    TimesNewRoman,
}

impl InlineFont {
    pub const fn fdx_value(self) -> &'static str {
        match self {
            Self::CourierFinalDraft => "Courier Final Draft",
            Self::CourierPrime => "Courier Prime",
            Self::Arial => "Arial",
            Self::TimesNewRoman => "Times New Roman",
        }
    }
}

/// Bounded inline sizes accepted by the initial formatting controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineSize {
    Seven,
    Eight,
    Nine,
    Ten,
    Twelve,
    Fourteen,
    Eighteen,
    TwentyFour,
}

impl InlineSize {
    pub const fn points(self) -> u8 {
        match self {
            Self::Seven => 7,
            Self::Eight => 8,
            Self::Nine => 9,
            Self::Ten => 10,
            Self::Twelve => 12,
            Self::Fourteen => 14,
            Self::Eighteen => 18,
            Self::TwentyFour => 24,
        }
    }
}

/// Bounded normalized colors accepted by inline formatting controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineColor {
    Black,
    White,
    Pink,
    Red,
    Orange,
    Yellow,
    Olive,
    Green,
    Mint,
    Blue,
    Violet,
    Purple,
    Brown,
    Gray,
}

impl InlineColor {
    /// Ordered reference palette exposed by the Color and Highlight menus.
    pub const MENU_PALETTE: [Self; 12] = [
        Self::Pink,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Olive,
        Self::Green,
        Self::Mint,
        Self::Blue,
        Self::Violet,
        Self::Purple,
        Self::Brown,
        Self::Gray,
    ];

    pub const fn fdx_value(self) -> &'static str {
        match self {
            Self::Black => "#000000000000",
            Self::White => "#FFFFFFFFFFFF",
            Self::Pink => "#E2E29898DDDD",
            Self::Red => "#EBEB62627B7B",
            Self::Orange => "#EFEFA4A46262",
            Self::Yellow => "#E5E5CBCB6C6C",
            Self::Olive => "#929290900000",
            Self::Green => "#8F8FC3C36A6A",
            Self::Mint => "#8888CACAB8B8",
            Self::Blue => "#6363A7A7EFEF",
            Self::Violet => "#9A9AAEAEDBDB",
            Self::Purple => "#AFAF9393E8E8",
            Self::Brown => "#B2B27C7C7373",
            Self::Gray => "#C0C0C0C0C0C0",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::White => "White",
            Self::Pink => "Pink",
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Olive => "Olive",
            Self::Green => "Green",
            Self::Mint => "Mint",
            Self::Blue => "Blue",
            Self::Violet => "Violet",
            Self::Purple => "Purple",
            Self::Brown => "Brown",
            Self::Gray => "Gray",
        }
    }

    pub const fn rgb8(self) -> [u8; 3] {
        match self {
            Self::Black => [0, 0, 0],
            Self::White => [255, 255, 255],
            Self::Pink => [226, 152, 221],
            Self::Red => [235, 98, 123],
            Self::Orange => [239, 164, 98],
            Self::Yellow => [229, 203, 108],
            Self::Olive => [146, 144, 0],
            Self::Green => [143, 195, 106],
            Self::Mint => [136, 202, 184],
            Self::Blue => [99, 167, 239],
            Self::Violet => [154, 174, 219],
            Self::Purple => [175, 147, 232],
            Self::Brown => [178, 124, 115],
            Self::Gray => [192, 192, 192],
        }
    }

    /// Resolve only bounded normalized FDX color values, allowing hexadecimal
    /// letter casing to differ without accepting malformed source strings.
    pub fn from_normalized_fdx_value(value: &str) -> Option<Self> {
        if value.len() != 13
            || !value.starts_with('#')
            || !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        {
            return None;
        }
        [Self::Black, Self::White]
            .into_iter()
            .chain(Self::MENU_PALETTE)
            .find(|color| value.eq_ignore_ascii_case(color.fdx_value()))
    }
}

/// `None` means unchanged; `Some(None)` clears Font, Size, Color, or Background
/// to inherit/absence, and `Some(Some(value))` writes a bounded explicit value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InlineRunFormattingPatch {
    pub font: Option<Option<InlineFont>>,
    pub size: Option<Option<InlineSize>>,
    pub color: Option<Option<InlineColor>>,
    pub background: Option<Option<InlineColor>>,
}

impl InlineRunFormattingPatch {
    pub const fn is_empty(self) -> bool {
        self.font.is_none()
            && self.size.is_none()
            && self.color.is_none()
            && self.background.is_none()
    }
}
/// Parse source `Style` tokens without normalizing unknown token spellings.
pub fn parse_style_attribute(
    attributes: &[RunAttribute],
) -> Result<RunStyleProjection, FormattedTextError> {
    let mut style_values = attributes
        .iter()
        .filter(|attribute| attribute.name == "Style");
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
        if token.is_empty() {
            continue;
        }
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
    let mut projection = parse_style_attribute(&result)?;
    projection.flags.set(style, enabled);
    let canonical = canonical_style_value(&projection);
    let style_index = result.iter().position(|attribute| attribute.name == "Style");
    match (style_index, canonical) {
        (Some(index), Some(value)) => result[index].value = value,
        (Some(index), None) => { result.remove(index); }
        (None, Some(value)) => insert_run_attribute(
            &mut result,
            RunAttribute { name: "Style".to_owned(), value },
        )?,
        (None, None) => {}
    }
    canonicalize_run_attributes(&result)
}

pub(super) fn effective_runs(
    text: &str,
    runs: &[TextRun],
) -> Result<Vec<TextRun>, FormattedTextError> {
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

fn push_run_fragment(runs: &mut Vec<TextRun>, text: &str, attributes: &[RunAttribute]) {
    if text.is_empty() {
        return;
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
            let has_style = parse_style_attribute(&run.attributes)?
                .flags
                .contains(style);
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
/// added. Adjacent output runs merge only when the shared semantic normalizer
/// considers their canonical Style, Size, and retained attributes equivalent.
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
            push_run_fragment(&mut transformed, &run.text, &run.attributes);
        } else {
            push_run_fragment(&mut transformed, &run.text[..local_start], &run.attributes);
            let current_has_style = parse_style_attribute(&run.attributes)?
                .flags
                .contains(style);
            let updated = if current_has_style == enabled {
                run.attributes.clone()
            } else {
                update_run_style(&run.attributes, style, enabled)?
            };
            push_run_fragment(
                &mut transformed,
                &run.text[local_start..local_end],
                &updated,
            );
            push_run_fragment(&mut transformed, &run.text[local_end..], &run.attributes);
        }
        global_start = global_end;
    }
    transformed = normalize_runs(canonical_text, &transformed)?;
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

/// Apply a typed inline attribute patch to exact UTF-8 ranges as one candidate.
/// Every affected source range is checked before run publication; attribute
/// slots stay in place, and the shared whole-paragraph normalizer determines
/// whether adjacent output runs are equivalent.
pub fn set_paragraph_run_formatting(
    project: &mut Project,
    selections: &[(ParagraphTarget, Range<usize>)],
    patch: InlineRunFormattingPatch,
) -> Result<bool, FormattedTextError> {
    if selections.is_empty() {
        return Err(FormattedTextError::EmptyParagraphSelection);
    }
    if patch.is_empty() {
        return Ok(false);
    }
    let mut seen = std::collections::BTreeSet::new();
    if selections.iter().any(|(target, _)| !seen.insert(*target)) {
        return Err(FormattedTextError::DuplicateParagraphTarget);
    }

    let mut candidate = project.clone();
    let mut snapshots = Vec::with_capacity(selections.len());
    for (target, range) in selections {
        let (id, text, runs, has_provenance) = match *target {
            ParagraphTarget::Body(index) => {
                let element = candidate.elements.get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (element.id.clone(), element.text.clone(), element.runs.clone(), element.source_provenance.is_some())
            }
            ParagraphTarget::TitlePage(index) => {
                let paragraph = candidate.title_page.get(index)
                    .ok_or(FormattedTextError::ParagraphOutOfRange)?;
                (paragraph.id.clone(), paragraph.text.clone(), paragraph.runs.clone(), paragraph.source_provenance.is_some())
            }
        };
        validate_range(&text, range)?;
        if !range.is_empty() {
            if let Some(preservation) = candidate.preservation.as_ref() {
                preservation.ensure_run_range_is_contiguous(&id, range.clone())?;
            }
        }
        snapshots.push((*target, id, text, runs, range.clone(), has_provenance));
    }

    let mut changed = false;
    for (target, id, text, runs, range, has_provenance) in snapshots {
        if range.is_empty() {
            continue;
        }
        let source_runs = effective_runs(&text, &runs)?;
        let mut transformed = Vec::with_capacity(source_runs.len().saturating_add(2));
        let mut global_start = 0usize;
        for run in &source_runs {
            let global_end = global_start.saturating_add(run.text.len());
            let local_start = range.start.max(global_start).min(global_end) - global_start;
            let local_end = range.end.max(global_start).min(global_end) - global_start;
            if local_start == local_end {
                push_run_fragment(&mut transformed, &run.text, &run.attributes);
            } else {
                push_run_fragment(&mut transformed, &run.text[..local_start], &run.attributes);
                let attributes = patch_inline_attributes(&run.attributes, patch)?;
                push_run_fragment(&mut transformed, &run.text[local_start..local_end], &attributes);
                push_run_fragment(&mut transformed, &run.text[local_end..], &run.attributes);
            }
            global_start = global_end;
        }
        transformed = normalize_runs(&text, &transformed)?;
        if transformed == source_runs {
            continue;
        }
        changed = true;
        match target {
            ParagraphTarget::Body(index) => BodyOwner::update(&mut candidate.elements[index], |element| {
                element.text = text.clone();
                element.runs = transformed.clone();
            }),
            ParagraphTarget::TitlePage(index) => {
                candidate.title_page[index].text = text.clone();
                candidate.title_page[index].runs = transformed.clone();
            }
        }
        if let Some(preservation) = candidate.preservation.as_mut() {
            if preservation.has_managed_element(&id) {
                preservation.sync_managed_text_runs(&id, &text, &transformed)?;
            } else if has_provenance {
                return Err(FormattedTextError::Preservation(PreservationError::ManagedAnchorMissing));
            }
        }
    }
    if changed {
        candidate.validate()?;
        *project = candidate;
    }
    Ok(changed)
}

fn patch_inline_attributes(
    source: &[RunAttribute],
    patch: InlineRunFormattingPatch,
) -> Result<Vec<RunAttribute>, FormattedTextError> {
    let mut result = source.to_vec();
    for (name, requested) in [
        ("Font", patch.font.map(|value| value.map(|font| font.fdx_value()))),
        ("Color", patch.color.map(|value| value.map(|color| color.fdx_value()))),
        (
            "Background",
            patch
                .background
                .map(|value| value.map(|color| color.fdx_value())),
        ),
    ] {
        let Some(value) = requested else { continue };
        let mut matches = result.iter().enumerate().filter(|(_, attribute)| attribute.name == name);
        let index = matches.next().map(|(index, _)| index);
        if matches.next().is_some() {
            return Err(FormattedTextError::AmbiguousInlineAttribute);
        }
        match (index, value) {
            (Some(index), Some(value)) => result[index].value = value.to_owned(),
            (Some(index), None) => { result.remove(index); }
            (None, Some(value)) => insert_run_attribute(&mut result, RunAttribute { name: name.to_owned(), value: value.to_owned() })?,
            (None, None) => {}
        }
    }
    if let Some(requested) = patch.size {
        let mut matches = result.iter().enumerate().filter(|(_, attribute)| attribute.name == "Size");
        let index = matches.next().map(|(index, _)| index);
        if matches.next().is_some() {
            return Err(FormattedTextError::AmbiguousInlineAttribute);
        }
        match (index, requested) {
            (Some(index), Some(size)) => result[index].value = size.points().to_string(),
            (Some(index), None) => { result.remove(index); }
            (None, Some(size)) => insert_run_attribute(
                &mut result,
                RunAttribute { name: "Size".to_owned(), value: size.points().to_string() },
            )?,
            (None, None) => {}
        }
    }
    canonicalize_run_attributes(&result)
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
            push_run_fragment(&mut transformed, &run.text, &run.attributes);
        } else {
            push_run_fragment(&mut transformed, &run.text[..local_start], &run.attributes);
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
            push_run_fragment(
                &mut transformed,
                &run.text[local_start..local_end],
                &attributes,
            );
            push_run_fragment(&mut transformed, &run.text[local_end..], &run.attributes);
        }
        global_start = global_end;
    }
    transformed = normalize_runs(canonical_text, &transformed)?;
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
