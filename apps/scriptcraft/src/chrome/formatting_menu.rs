//! Lightweight effective-value projections for the fixed Format submenus.
//!
//! This deliberately reads semantic definitions, direct values, runs, and the
//! transient Body typing patch only. It does not resolve host fonts or trigger
//! any layout work.

use super::menu_items::MenuAction;
use scriptcraft_commands::{
    EditorSurface, InlineColorValue, InlineFontValue, InlineSizeValue,
    ParagraphAlignmentValue, ParagraphLeadingValue, ParagraphSpacingValue, SpaceBeforeLines,
    TitleTextStyle,
};
use scriptcraft_model::element::{RunAttribute, TextRun};
use scriptcraft_model::formatted_text::{parse_style_attribute, InlineSizeProjection, RunStyle, RunStyleFlags};
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::formatting::{ParagraphAlignment, ParagraphLeading, ParagraphSpacing};
use scriptcraft_model::{ElementFontSpec, Project};
use scriptcraft_session::editing::{EditingSession, Position, Selection};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FormattingMenuProjection {
    checked_actions: Vec<MenuAction>,
}

impl FormattingMenuProjection {
    pub(crate) fn is_checked(&self, action: MenuAction) -> bool {
        self.checked_actions.contains(&action)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunValues {
    font: String,
    base_font: String,
    font_direct: bool,
    size: Option<FixedPoint>,
    size_direct: bool,
    color: Option<String>,
    base_color: Option<String>,
    color_direct: bool,
    styles: RunStyleFlags,
}

pub(crate) fn project(session: &EditingSession, surface: EditorSurface) -> FormattingMenuProjection {
    let mut projection = FormattingMenuProjection::default();
    let project = session.project();
    let (selection, title_surface) = match surface {
        EditorSurface::Body => (session.selection(), false),
        EditorSurface::TitlePage => (session.title_selection(), true),
    };

    let mut runs = selected_run_values(project, selection, title_surface);
    if selection.anchor == selection.focus && !title_surface {
        if let Some(values) = runs.first_mut() {
            apply_typing_patch(values, session.body_typing_formatting());
            values.styles = session.body_typing_style();
        }
    }
    if selection.anchor == selection.focus && title_surface {
        if let Some(values) = runs.first_mut() {
            values.styles = session.title_typing_style();
        }
    }

    if surface == EditorSurface::Body {
      if uniform_field(&runs, |value| Some(value.size_direct)) == Some(false) {
          projection.checked_actions.push(MenuAction::ClearInlineSize);
      }
      if let Some(values) = uniform_field(&runs, |value| value.size.filter(|_| value.size_direct)) {
        for size in [
            InlineSizeValue::Seven,
            InlineSizeValue::Eight,
            InlineSizeValue::Nine,
            InlineSizeValue::Ten,
            InlineSizeValue::Twelve,
            InlineSizeValue::Fourteen,
            InlineSizeValue::Eighteen,
            InlineSizeValue::TwentyFour,
        ] {
            if FixedPoint::from_points(i64::from(size.as_core().points())) == values {
                projection.checked_actions.push(MenuAction::SetInlineSize(size));
            }
        }
      }
    let font_values = uniform_field(&runs, |value| Some(value.font.clone()));
    if uniform_field(&runs, |value| Some(value.font_direct)) == Some(false) {
        projection.checked_actions.push(MenuAction::SetInlineFont(InlineFontValue::ElementDefault));
    }
    if let Some(font) = font_values {
        for value in [
            InlineFontValue::CourierFinalDraft,
            InlineFontValue::CourierPrime,
            InlineFontValue::Arial,
            InlineFontValue::TimesNewRoman,
        ] {
            if font == inline_font_name(value).unwrap_or_default() {
                projection.checked_actions.push(MenuAction::SetInlineFont(value));
            }
        }
      }
    let color_values = uniform_field(&runs, |value| Some(value.color.clone()));
    if uniform_field(&runs, |value| Some(value.color_direct)) == Some(false) {
        projection.checked_actions.push(MenuAction::SetInlineColor(InlineColorValue::ElementDefault));
    }
    if let Some(color) = color_values {
        for value in [
            InlineColorValue::Black,
            InlineColorValue::White,
            InlineColorValue::Red,
            InlineColorValue::Green,
            InlineColorValue::Blue,
        ] {
            if color.as_deref() == value.as_core().map(|value| value.fdx_value()) {
                projection.checked_actions.push(MenuAction::SetInlineColor(value));
            }
        }
      }
    }
    for (style, value) in [
        (TitleTextStyle::Bold, RunStyle::Bold),
        (TitleTextStyle::Italics, RunStyle::Italics),
        (TitleTextStyle::Underline, RunStyle::Underline),
        (TitleTextStyle::AllCaps, RunStyle::AllCaps),
        (TitleTextStyle::Strikethrough, RunStyle::Strikethrough),
    ] {
        if uniform_field(&runs, |run| Some(run.styles.contains(value))) == Some(true) {
            projection.checked_actions.push(MenuAction::ToggleStyle(style));
        }
    }
    if uniform_field(&runs, |run| Some(run.styles == RunStyleFlags::default())) == Some(true) {
        projection.checked_actions.push(MenuAction::ClearStyles);
    }

    let targets = selected_paragraphs(project, selection, title_surface);
    let paragraphs = targets
        .iter()
        .filter_map(|index| paragraph_values(project, *index, title_surface))
        .collect::<Vec<_>>();
    if let Some(alignment) = uniform_field(&paragraphs, |value| Some(value.alignment)) {
        for value in [
            ParagraphAlignmentValue::Left,
            ParagraphAlignmentValue::Center,
            ParagraphAlignmentValue::Right,
            ParagraphAlignmentValue::Justified,
        ] {
            if alignment == Some(value.as_core()) {
                projection.checked_actions.push(MenuAction::SetAlignment(value));
            }
        }
    }
    if surface == EditorSurface::Body {
        if let Some(spacing) = uniform_field(&paragraphs, |value| Some(value.spacing)) {
            for value in [
                ParagraphSpacingValue::Single,
                ParagraphSpacingValue::OneAndHalf,
                ParagraphSpacingValue::Double,
            ] {
                if spacing == Some(value.as_core()) {
                    projection.checked_actions.push(MenuAction::SetParagraphSpacing(value));
                }
            }
        }
        if let Some(leading) = uniform_field(&paragraphs, |value| Some(value.leading)) {
            for value in [
                ParagraphLeadingValue::Regular,
                ParagraphLeadingValue::Tight,
                ParagraphLeadingValue::Loose,
            ] {
                if leading == Some(value.as_core()) {
                    projection.checked_actions.push(MenuAction::SetParagraphLeading(value));
                }
            }
        }
        for lines in 0..=3 {
            if uniform_field(&paragraphs, |value| Some(value.space_before_lines)) == Some(Some(lines)) {
                let typed = SpaceBeforeLines::new(lines).expect("menu line count is bounded");
                projection.checked_actions.push(MenuAction::SetSpaceBefore(typed));
            }
        }
    }
    projection
}

#[derive(Debug, Clone, Copy)]
struct ParagraphValues {
    alignment: Option<ParagraphAlignment>,
    spacing: Option<ParagraphSpacing>,
    leading: Option<ParagraphLeading>,
    space_before_lines: Option<u8>,
}

fn selected_paragraphs(project: &Project, selection: Selection, title_surface: bool) -> Vec<usize> {
    let len = if title_surface { project.title_page.len() } else { project.elements.len() };
    let (start, end) = ordered_positions(selection);
    if start.element >= len || end.element >= len || start.element > end.element {
        return Vec::new();
    }
    let last = if start.element < end.element && end.offset == 0 {
        end.element.saturating_sub(1)
    } else {
        end.element
    };
    if last < start.element { Vec::new() } else { (start.element..=last).collect() }
}

fn paragraph_values(project: &Project, index: usize, title_surface: bool) -> Option<ParagraphValues> {
    if title_surface {
        let paragraph = project.title_page.get(index)?;
        let (alignment, spacing, leading, space_before, font_size) = if let Some(key) = &paragraph.element_type {
            let definition = project.element_definitions().get(key)?;
            (
                paragraph.formatting.alignment.map(|value| match value {
                    scriptcraft_model::title_page::TitlePageAlignment::Left => ParagraphAlignment::Left,
                    scriptcraft_model::title_page::TitlePageAlignment::Center => ParagraphAlignment::Center,
                    scriptcraft_model::title_page::TitlePageAlignment::Right => ParagraphAlignment::Right,
                    scriptcraft_model::title_page::TitlePageAlignment::Justified => ParagraphAlignment::Justified,
                }).or(Some(definition.paragraph.alignment)),
                paragraph.formatting.spacing.or(ParagraphSpacing::from_definition_value(definition.paragraph.spacing)),
                paragraph.formatting.leading.or_else(|| ParagraphLeading::from_definition_value(&definition.paragraph.leading)),
                paragraph.formatting.space_before.unwrap_or(definition.paragraph.space_before),
                definition.font.size,
            )
        } else {
            (paragraph.formatting.alignment.map(|value| match value {
                scriptcraft_model::title_page::TitlePageAlignment::Left => ParagraphAlignment::Left,
                scriptcraft_model::title_page::TitlePageAlignment::Center => ParagraphAlignment::Center,
                scriptcraft_model::title_page::TitlePageAlignment::Right => ParagraphAlignment::Right,
                scriptcraft_model::title_page::TitlePageAlignment::Justified => ParagraphAlignment::Justified,
            }), paragraph.formatting.spacing, paragraph.formatting.leading, paragraph.formatting.space_before.unwrap_or_default(), 0)
        };
        Some(ParagraphValues { alignment, spacing, leading, space_before_lines: exact_line_count(space_before, font_size) })
    } else {
        let element = project.elements.get(index)?;
        let definition = project.element_definitions().get(&element.element_type())?;
        let direct = project.direct_paragraph_overrides.get(&element.id).copied().unwrap_or_default();
        let space_before = direct.paragraph.space_before.unwrap_or(definition.paragraph.space_before);
        Some(ParagraphValues {
            alignment: Some(direct.paragraph.alignment.unwrap_or(definition.paragraph.alignment)),
            spacing: direct.paragraph.spacing.or(ParagraphSpacing::from_definition_value(definition.paragraph.spacing)),
            leading: direct.paragraph.leading.or_else(|| ParagraphLeading::from_definition_value(&definition.paragraph.leading)),
            space_before_lines: exact_line_count(space_before, definition.font.size),
        })
    }
}

fn exact_line_count(
    space_before: scriptcraft_model::formatting::FixedPoint,
    font_size: u8,
) -> Option<u8> {
    if font_size == 0 || space_before.raw() < 0 { return None; }
    let size = i64::from(font_size)
        .saturating_mul(scriptcraft_model::formatting::FixedPoint::SCALE);
    if size == 0 || space_before.raw() % size != 0 { return None; }
    u8::try_from(space_before.raw() / size).ok().filter(|lines| *lines <= 3)
}

fn selected_run_values(project: &Project, selection: Selection, title_surface: bool) -> Vec<RunValues> {
    let (start, end) = ordered_positions(selection);
    let paragraph_count = if title_surface { project.title_page.len() } else { project.elements.len() };
    if start.element != end.element || start.element >= paragraph_count {
        if start.element == end.element { return run_values_at(project, start, title_surface).into_iter().collect(); }
        let mut values = Vec::new();
        for index in start.element..=end.element {
            let (text, runs, key, range) = if title_surface {
                let Some(paragraph) = project.title_page.get(index) else { continue };
                (paragraph.text.as_str(), paragraph.runs.as_slice(), paragraph.element_type.as_ref(),
                    (if index == start.element { start.offset } else { 0 })..(if index == end.element { end.offset } else { paragraph.text.len() }))
            } else {
                let Some(paragraph) = project.elements.get(index) else { continue };
                let paragraph = paragraph.as_element();
                (paragraph.text.as_str(), paragraph.runs.as_slice(), Some(&paragraph.element_type),
                    (if index == start.element { start.offset } else { 0 })..(if index == end.element { end.offset } else { paragraph.text.len() }))
            };
            let Some(definition) = key.and_then(|key| project.element_definitions().get(key)) else { continue };
                let Some(selected) = values_for_range(text, runs, range, &definition.font) else {
                    return Vec::new();
                };
                values.extend(selected);
        }
        return values;
    }
    if start == end { return run_values_at(project, start, title_surface).into_iter().collect(); }
    let index = start.element;
    let (text, runs, key) = if title_surface {
        let paragraph = &project.title_page[index];
        (paragraph.text.as_str(), paragraph.runs.as_slice(), paragraph.element_type.as_ref())
    } else {
        let paragraph = project.elements[index].as_element();
        (paragraph.text.as_str(), paragraph.runs.as_slice(), Some(&paragraph.element_type))
    };
    let Some(definition) = key.and_then(|key| project.element_definitions().get(key)) else { return Vec::new() };
    values_for_range(text, runs, start.offset..end.offset, &definition.font).unwrap_or_default()
}

fn run_values_at(project: &Project, position: Position, title_surface: bool) -> Option<RunValues> {
    let (text, runs, key) = if title_surface {
        let paragraph = project.title_page.get(position.element)?;
        (paragraph.text.as_str(), paragraph.runs.as_slice(), paragraph.element_type.as_ref())
    } else {
        let paragraph = project.elements.get(position.element)?.as_element();
        (paragraph.text.as_str(), paragraph.runs.as_slice(), Some(&paragraph.element_type))
    };
    let font = key.and_then(|key| project.element_definitions().get(key)).map(|definition| &definition.font)?;
    let attrs = attributes_at(text, runs, position.offset);
    run_values(&attrs, font)
}

fn attributes_at<'a>(text: &str, runs: &'a [TextRun], offset: usize) -> &'a [RunAttribute] {
    let mut cursor = 0usize;
    let mut last = None;
    for run in runs {
        let end = cursor.saturating_add(run.text.len());
        if offset < end { return &run.attributes; }
        if offset == cursor { return &run.attributes; }
        last = Some(run.attributes.as_slice());
        cursor = end;
    }
    let _ = text;
    last.unwrap_or(&[])
}

fn values_for_range(
    text: &str,
    runs: &[TextRun],
    range: std::ops::Range<usize>,
    font: &ElementFontSpec,
) -> Option<Vec<RunValues>> {
    if range.start >= range.end { return Some(Vec::new()); }
    if runs.is_empty() { return Some(run_values(&[], font).into_iter().collect()); }
    let mut cursor = 0usize;
    let mut values = Vec::new();
    for run in runs {
        let end = cursor.saturating_add(run.text.len());
        if cursor < range.end && end > range.start {
            values.push(run_values(&run.attributes, font)?);
        }
        cursor = end;
    }
    let _ = text;
    Some(values)
}

fn unique_attribute<'a>(attributes: &'a [RunAttribute], name: &str) -> Result<Option<&'a str>, ()> {
    let mut matches = attributes.iter().filter(|attribute| attribute.name == name);
    let value = matches.next().map(|attribute| attribute.value.as_str());
    if matches.next().is_some() { Err(()) } else { Ok(value) }
}

fn run_values(attributes: &[RunAttribute], base: &ElementFontSpec) -> Option<RunValues> {
    let font = unique_attribute(attributes, "Font").ok()?;
    let color = unique_attribute(attributes, "Color").ok()?;
    let _style = unique_attribute(attributes, "Style").ok()?;
    let styles = parse_style_attribute(attributes).ok()?.flags;
    let size_projection = InlineSizeProjection::parse(attributes).ok()?;
    let (size, size_direct) = match size_projection {
        InlineSizeProjection::Inherited => (None, false),
        InlineSizeProjection::Valid { points, .. } => (Some(points), true),
        InlineSizeProjection::Invalid { .. } => (None, true),
    };
    Some(RunValues {
        font: font.filter(|value| !value.is_empty()).unwrap_or(&base.family).to_owned(),
        base_font: base.family.clone(),
        font_direct: font.is_some(),
        size,
        size_direct,
        color: color.map(str::to_owned).or_else(|| base.color.clone()),
        base_color: base.color.clone(),
        color_direct: color.is_some(),
        styles: RunStyleFlags {
            bold: base.bold || styles.bold,
            italics: base.italic || styles.italics,
            underline: base.underline || styles.underline,
            all_caps: base.all_caps || styles.all_caps,
            strikethrough: styles.strikethrough,
        },
    })
}

fn apply_typing_patch(values: &mut RunValues, patch: scriptcraft_model::formatted_text::InlineRunFormattingPatch) {
    if let Some(font) = patch.font {
        values.font = font
            .map(|font| font.fdx_value().to_owned())
            .unwrap_or_else(|| values.base_font.clone());
        values.font_direct = font.is_some();
    }
    if let Some(size) = patch.size {
        match size {
            Some(size) => {
                values.size = Some(FixedPoint::from_points(i64::from(size.points())));
                values.size_direct = true;
            }
            None => {
                values.size = None;
                values.size_direct = false;
            }
        }
    }
    if let Some(color) = patch.color {
        values.color = color
            .map(|color| color.fdx_value().to_owned())
            .or_else(|| values.base_color.clone());
        values.color_direct = color.is_some();
    }
}

fn uniform_field<S, T: PartialEq>(values: &[S], get: impl Fn(&S) -> Option<T>) -> Option<T> {
    let first = get(values.first()?)?;
    values.iter().skip(1).all(|value| get(value).as_ref() == Some(&first)).then_some(first)
}

fn inline_font_name(value: InlineFontValue) -> Option<&'static str> {
    match value {
        InlineFontValue::ElementDefault => None,
        InlineFontValue::CourierFinalDraft => Some("Courier Final Draft"),
        InlineFontValue::CourierPrime => Some("Courier Prime"),
        InlineFontValue::Arial => Some("Arial"),
        InlineFontValue::TimesNewRoman => Some("Times New Roman"),
    }
}

fn ordered_positions(selection: Selection) -> (Position, Position) {
    let key = |position: Position| (position.element, position.offset);
    if key(selection.anchor) <= key(selection.focus) {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}
