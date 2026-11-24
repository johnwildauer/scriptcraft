//! Lossy semantic Project-to-Fountain text serialization.

use std::collections::HashSet;

use scriptcraft_model::{
    dual_dialogue::active_dual_dialogue_projection,
    element::{ScreenplayElement, TextRun},
    formatted_text::{parse_style_attribute, RunStyleFlags},
    formatting::ParagraphAlignment,
    ElementTypeKey, Project, StableId, TitlePageAlignment, TitlePageParagraph,
};

use crate::ExportError;

const MAX_FOUNTAIN_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EffectiveStyle {
    bold: bool,
    italics: bool,
    underline: bool,
    all_caps: bool,
}

impl EffectiveStyle {
    fn from_flags(base: RunStyleFlags, run: RunStyleFlags) -> Self {
        Self {
            bold: base.bold || run.bold,
            italics: base.italics || run.italics,
            underline: base.underline || run.underline,
            all_caps: base.all_caps || run.all_caps,
        }
    }

    fn delimiters(self) -> (&'static str, &'static str) {
        let emphasis = match (self.bold, self.italics) {
            (true, true) => "***",
            (true, false) => "**",
            (false, true) => "*",
            (false, false) => "",
        };
        match (self.underline, emphasis) {
            (true, "") => ("_", "_"),
            (true, "*") => ("_*", "*_"),
            (true, "**") => ("_**", "**_"),
            (true, "***") => ("_***", "***_"),
            (false, value) => (value, value),
            (true, _) => ("_", "_"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RunView<'a> {
    text: &'a str,
    style: EffectiveStyle,
}

#[derive(Debug, Clone, Copy)]
struct StyledChar {
    value: char,
    style: EffectiveStyle,
}

#[derive(Debug)]
struct FountainWriter {
    text: String,
    title_entries: usize,
    body_blocks: usize,
    dialogue_live: bool,
    previous_was_page_break: bool,
}

impl FountainWriter {
    fn new() -> Self {
        Self {
            text: String::new(),
            title_entries: 0,
            body_blocks: 0,
            dialogue_live: false,
            previous_was_page_break: false,
        }
    }

    fn push(&mut self, value: &str) -> Result<(), ExportError> {
        append_bounded(&mut self.text, value)
    }

    fn push_title_entry(&mut self, key: &str, value: &str) -> Result<(), ExportError> {
        if self.title_entries != 0 {
            self.push("\n")?;
        }
        if value.contains('\n') {
            self.push(key)?;
            self.push(":")?;
            for line in value.split('\n') {
                self.push("\n    ")?;
                self.push(line)?;
            }
        } else if value.is_empty() {
            self.push(key)?;
            self.push(":")?;
        } else {
            self.push(key)?;
            self.push(": ")?;
            self.push(value)?;
        }
        self.title_entries += 1;
        Ok(())
    }

    fn push_body_block(&mut self, value: &str, contiguous_dialogue: bool) -> Result<(), ExportError> {
        if value.is_empty() && self.body_blocks != 0 {
            // Empty semantic paragraphs still need an explicit Fountain marker
            // from their type-specific writer; this is only a defensive guard.
            return Ok(());
        }
        if self.body_blocks == 0 {
            if self.title_entries != 0 {
                self.push("\n\n")?;
            }
        } else if contiguous_dialogue {
            self.push("\n")?;
        } else {
            self.push("\n\n")?;
        }
        self.push(value.trim_end_matches('\n'))?;
        self.body_blocks += 1;
        self.previous_was_page_break = value == "===";
        Ok(())
    }

    fn push_page_break(&mut self) -> Result<(), ExportError> {
        self.dialogue_live = false;
        if self.body_blocks == 0 || self.previous_was_page_break {
            return Ok(());
        }
        self.push_body_block("===", false)
    }

    fn finish(mut self) -> Result<String, ExportError> {
        while self.text.ends_with('\n') {
            self.text.pop();
        }
        if !self.text.is_empty() {
            self.push("\n")?;
        }
        Ok(self.text)
    }
}

/// Serializes the current semantic Project into bounded, LF-only Fountain text.
/// The Project and all native/session state are borrowed immutably.
pub fn export_fountain(project: &Project) -> Result<String, ExportError> {
    let right_cues = project
        .dual_dialogue_groups
        .iter()
        .filter_map(|group| active_dual_dialogue_projection(project, group))
        .map(|projection| projection.right.character)
        .collect::<HashSet<StableId>>();

    let mut writer = FountainWriter::new();
    write_title_page(project, &mut writer)?;

    for owner in &project.elements {
        let element = owner.as_element();
        let definition = project
            .element_definitions()
            .get(&element.element_type)
            .ok_or(ExportError::InvalidFountainSource)?;
        let base = body_base_style(project, element)?;
        let alignment = project
            .direct_paragraph_overrides
            .get(&element.id)
            .and_then(|override_value| override_value.paragraph.alignment)
            .unwrap_or(definition.paragraph.alignment);
        let starts_new_page = project
            .direct_paragraph_overrides
            .get(&element.id)
            .and_then(|override_value| override_value.starts_new_page)
            .unwrap_or(definition.paragraph.starts_new_page);

        if starts_new_page {
            writer.push_page_break()?;
        }

        let kind = element.element_type.as_str();
        let mut next_dialogue_live = false;
        let contiguous = writer.dialogue_live
            && matches!(kind, "Dialogue" | "Parenthetical");
        let serialized = match kind {
            "Scene Heading" => {
                let value = inline_text(&element.text, &element.runs, base)?;
                let flattened = value.replace('\n', " ");
                if has_natural_scene_prefix(&flattened) {
                    flattened
                } else {
                    force_scene_heading(&flattened)
                }
            }
            "Character" => {
                let value = inline_text(&element.text, &element.runs, base)?;
                let flattened = value.replace('\n', " ");
                let mut cue = format!("@{flattened}");
                if right_cues.contains(&element.id) {
                    cue.push_str(" ^");
                }
                next_dialogue_live = true;
                cue
            }
            "Dialogue" if writer.dialogue_live => {
                next_dialogue_live = true;
                dialogue_lines(&element.text, &element.runs, base)?
            }
            "Parenthetical" if writer.dialogue_live => {
                next_dialogue_live = true;
                let value = inline_text(&element.text, &element.runs, base)?;
                parenthetical(&value.replace('\n', " "))
            }
            "Dialogue" | "Parenthetical" => {
                forced_action(&element.text, &element.runs, base)?
            }
            "Transition" => {
                let value = inline_text(&element.text, &element.runs, base)?;
                format!(">{}", value.replace('\n', " "))
            }
            "Lyrics" => prefixed_lines(&element.text, &element.runs, base, "~", false)?,
            "Action" => action_text(&element.text, &element.runs, base, alignment)?,
            "Note" if !element.text.contains("]]" ) => {
                let mut value = inline_text(&element.text, &element.runs, base)?;
                value = blank_lines_two_spaces(&value);
                format!("[[{value}]]")
            }
            "Note" => forced_action(&element.text, &element.runs, base)?,
            "Summary" => prefixed_lines(&element.text, &element.runs, base, "= ", false)?,
            "Outline 1" => flattened_prefix(&element.text, &element.runs, base, "# ")?,
            "Outline 2" => flattened_prefix(&element.text, &element.runs, base, "## ")?,
            "Outline 3" => flattened_prefix(&element.text, &element.runs, base, "### ")?,
            _ => forced_unmapped_action(&element.text, &element.runs, base, alignment)?,
        };

        if kind == "Character" {
            writer.dialogue_live = true;
        } else if matches!(kind, "Dialogue" | "Parenthetical") {
            writer.dialogue_live = next_dialogue_live;
        } else {
            writer.dialogue_live = false;
        }
        writer.push_body_block(&serialized, contiguous)?;
    }

    writer.finish()
}

fn append_bounded(target: &mut String, value: &str) -> Result<(), ExportError> {
    let new_len = target
        .len()
        .checked_add(value.len())
        .ok_or(ExportError::OutputResourceLimitExceeded)?;
    if new_len > MAX_FOUNTAIN_BYTES {
        return Err(ExportError::OutputResourceLimitExceeded);
    }
    target
        .try_reserve(value.len())
        .map_err(|_| ExportError::OutputResourceLimitExceeded)?;
    target.push_str(value);
    Ok(())
}

fn inline_text(
    canonical: &str,
    runs: &[TextRun],
    base: EffectiveStyle,
) -> Result<String, ExportError> {
    let mut views = Vec::new();
    if runs.is_empty() {
        views.push(RunView {
            text: canonical,
            style: base,
        });
    } else {
        let mut cursor = 0usize;
        for run in runs {
            let end = cursor
                .checked_add(run.text.len())
                .ok_or(ExportError::InvalidFountainSource)?;
            if canonical.get(cursor..end) != Some(run.text.as_str()) {
                return Err(ExportError::InvalidFountainSource);
            }
            cursor = end;
            let parsed = parse_style_attribute(&run.attributes)
                .map_err(|_| ExportError::InvalidFountainSource)?;
            views.push(RunView {
                text: &run.text,
                style: EffectiveStyle::from_flags(base_flags(base), parsed.flags),
            });
        }
        if cursor != canonical.len() {
            return Err(ExportError::InvalidFountainSource);
        }
    }

    let mut lines: Vec<Vec<StyledChar>> = vec![Vec::new()];
    let mut pending_cr = false;
    for run in views {
        for value in run.text.chars() {
            if pending_cr {
                lines.push(Vec::new());
                pending_cr = false;
                if value == '\n' {
                    continue;
                }
            }
            if value == '\r' {
                pending_cr = true;
                continue;
            }
            if value == '\n' {
                lines.push(Vec::new());
                continue;
            }
            if run.style.all_caps {
                for uppercase in value.to_uppercase() {
                    lines.last_mut().ok_or(ExportError::InvalidFountainSource)?.push(StyledChar {
                        value: uppercase,
                        style: run.style,
                    });
                }
            } else {
                lines.last_mut().ok_or(ExportError::InvalidFountainSource)?.push(StyledChar {
                    value,
                    style: run.style,
                });
            }
        }
    }
    if pending_cr {
        lines.push(Vec::new());
    }

    let mut output = String::new();
    for (line_index, line) in lines.iter().enumerate() {
        if line_index != 0 {
            append_bounded(&mut output, "\n")?;
        }
        render_styled_line(&mut output, line)?;
    }
    Ok(output)
}

fn base_flags(style: EffectiveStyle) -> RunStyleFlags {
    RunStyleFlags {
        bold: style.bold,
        italics: style.italics,
        underline: style.underline,
        all_caps: style.all_caps,
        strikethrough: false,
    }
}

fn render_styled_line(output: &mut String, line: &[StyledChar]) -> Result<(), ExportError> {
    let mut cursor = 0usize;
    while cursor < line.len() {
        let style = line[cursor].style;
        let mut end = cursor + 1;
        while end < line.len() && line[end].style == style {
            end += 1;
        }
        let segment = &line[cursor..end];
        let leading = segment.iter().take_while(|item| item.value.is_whitespace()).count();
        let trailing_start = segment
            .iter()
            .rposition(|item| !item.value.is_whitespace())
            .map_or(leading, |index| index + 1);
        for item in &segment[..leading] {
            push_escaped(output, item.value)?;
        }
        if trailing_start > leading {
            let (open, close) = style.delimiters();
            append_bounded(output, open)?;
            for item in &segment[leading..trailing_start] {
                push_escaped(output, item.value)?;
            }
            append_bounded(output, close)?;
        }
        for item in &segment[trailing_start..] {
            push_escaped(output, item.value)?;
        }
        cursor = end;
    }
    Ok(())
}

fn push_escaped(output: &mut String, value: char) -> Result<(), ExportError> {
    if matches!(value, '\\' | '*' | '_') {
        append_bounded(output, "\\")?;
    }
    let mut buffer = [0u8; 4];
    append_bounded(output, value.encode_utf8(&mut buffer))
}

fn body_base_style(project: &Project, element: &ScreenplayElement) -> Result<EffectiveStyle, ExportError> {
    let definition = project
        .element_definitions()
        .get(&element.element_type)
        .ok_or(ExportError::InvalidFountainSource)?;
    Ok(EffectiveStyle {
        bold: definition.font.bold,
        italics: definition.font.italic,
        underline: definition.font.underline,
        all_caps: definition.font.all_caps,
    })
}

fn title_base_style(
    project: &Project,
    paragraph: &TitlePageParagraph,
) -> Result<EffectiveStyle, ExportError> {
    if let Some(key) = &paragraph.element_type {
        let definition = project
            .element_definitions()
            .get(key)
            .ok_or(ExportError::InvalidFountainSource)?;
        Ok(EffectiveStyle {
            bold: definition.font.bold,
            italics: definition.font.italic,
            underline: definition.font.underline,
            all_caps: definition.font.all_caps,
        })
    } else {
        Ok(EffectiveStyle {
            bold: false,
            italics: false,
            underline: false,
            all_caps: false,
        })
    }
}

fn inline_paragraph(
    project: &Project,
    paragraph: &TitlePageParagraph,
) -> Result<String, ExportError> {
    inline_text(&paragraph.text, &paragraph.runs, title_base_style(project, paragraph)?)
}

#[derive(Debug)]
enum TitleItem {
    Typed(usize),
    Separator,
    Untyped { indexes: Vec<usize>, centered: bool },
}

fn write_title_page(project: &Project, writer: &mut FountainWriter) -> Result<(), ExportError> {
    let mut items = Vec::new();
    let mut index = 0usize;
    while index < project.title_page.len() {
        let paragraph = &project.title_page[index];
        if paragraph.element_type.is_some() {
            items.push(TitleItem::Typed(index));
            index += 1;
            continue;
        }
        let centered = paragraph
            .formatting
            .alignment
            .is_some_and(|alignment| alignment == TitlePageAlignment::Center);
        if paragraph.text.is_empty() {
            items.push(TitleItem::Separator);
            index += 1;
            continue;
        }
        let mut indexes = vec![index];
        index += 1;
        while index < project.title_page.len() {
            let next = &project.title_page[index];
            if next.element_type.is_some() || next.text.is_empty() {
                break;
            }
            let next_centered = next
                .formatting
                .alignment
                .is_some_and(|alignment| alignment == TitlePageAlignment::Center);
            if centered != next_centered {
                break;
            }
            indexes.push(index);
            index += 1;
        }
        items.push(TitleItem::Untyped { indexes, centered });
    }

    let mut saw_centered = false;
    let mut previous_was_credit = false;
    let mut contact_assigned = false;
    for item in items {
        match item {
            TitleItem::Separator => {}
            TitleItem::Typed(index) => {
                let paragraph = &project.title_page[index];
                let key = paragraph
                    .element_type
                    .as_ref()
                    .map(title_key)
                    .unwrap_or_else(|| "Content".to_owned());
                let value = inline_paragraph(project, paragraph)?;
                writer.push_title_entry(&key, &value)?;
                previous_was_credit = false;
            }
            TitleItem::Untyped { indexes, centered } => {
                let mut value = String::new();
                let mut plain_value = String::new();
                for (position, paragraph_index) in indexes.iter().enumerate() {
                    if position != 0 {
                        append_bounded(&mut value, "\n")?;
                        append_bounded(&mut plain_value, "\n")?;
                    }
                    let paragraph = &project.title_page[*paragraph_index];
                    append_bounded(
                        &mut value,
                        &inline_paragraph(project, paragraph)?,
                    )?;
                    append_bounded(
                        &mut plain_value,
                        &paragraph.text.replace("\r\n", "\n").replace('\r', "\n"),
                    )?;
                }
                let trimmed = plain_value.trim();
                let lower = trimmed.to_lowercase();
                let key = if centered && !saw_centered {
                    saw_centered = true;
                    "Title"
                } else if centered && previous_was_credit {
                    "Author"
                } else if centered && (lower == "written by" || lower == "by" || lower.starts_with("by ")) {
                    "Credit"
                } else if centered && (lower.starts_with("based on") || lower.starts_with("story by")) {
                    "Source"
                } else if !centered && saw_centered && !contact_assigned {
                    contact_assigned = true;
                    "Contact"
                } else {
                    "Content"
                };
                previous_was_credit = key == "Credit";
                if key == "Credit" || key == "Author" || key == "Source" || key == "Title" {
                    saw_centered = true;
                }
                writer.push_title_entry(key, &value)?;
            }
        }
    }
    Ok(())
}

fn title_key(key: &ElementTypeKey) -> String {
    match key.as_str() {
        "Title" | "Credit" | "Author" | "Authors" | "Source" | "Draft date" | "Contact" => {
            key.as_str().to_owned()
        }
        custom => {
            let sanitized = custom
                .chars()
                .map(|character| {
                    if character == ':' || character == '\r' || character == '\n' || character.is_control() {
                        ' '
                    } else {
                        character
                    }
                })
                .collect::<String>();
            let sanitized = sanitized.trim();
            if sanitized.is_empty() {
                "Content".to_owned()
            } else {
                sanitized.to_owned()
            }
        }
    }
}

fn dialogue_lines(
    text: &str,
    runs: &[TextRun],
    base: EffectiveStyle,
) -> Result<String, ExportError> {
    let value = inline_text(text, runs, base)?;
    Ok(value
        .split('\n')
        .map(|line| if line.is_empty() { "  " } else { line })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn parenthetical(value: &str) -> String {
    let trimmed = value.trim();
    let leading = value.len().saturating_sub(value.trim_start().len());
    let trailing = value
        .len()
        .saturating_sub(value.trim_end().len())
        .min(value.len().saturating_sub(leading));
    let already_parenthesized = trimmed.starts_with('(') && trimmed.ends_with(')');
    let mut result = String::new();
    result.push_str(&value[..leading]);
    if already_parenthesized {
        result.push_str(trimmed);
    } else {
        result.push('(');
        result.push_str(trimmed);
        result.push(')');
    }
    if trailing != 0 {
        result.push_str(&value[value.len() - trailing..]);
    }
    result
}

fn prefixed_lines(
    text: &str,
    runs: &[TextRun],
    base: EffectiveStyle,
    prefix: &str,
    flatten: bool,
) -> Result<String, ExportError> {
    let value = inline_text(text, runs, base)?;
    let value = if flatten { value.replace('\n', " ") } else { value };
    Ok(value
        .split('\n')
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn flattened_prefix(
    text: &str,
    runs: &[TextRun],
    base: EffectiveStyle,
    prefix: &str,
) -> Result<String, ExportError> {
    let value = inline_text(text, runs, base)?.replace('\n', " ");
    Ok(format!("{prefix}{value}"))
}

fn blank_lines_two_spaces(value: &str) -> String {
    value
        .split('\n')
        .map(|line| if line.is_empty() { "  " } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}

fn action_text(
    text: &str,
    runs: &[TextRun],
    base: EffectiveStyle,
    alignment: ParagraphAlignment,
) -> Result<String, ExportError> {
    let canonical = normalized_action_text(text);
    let value = inline_text(text, runs, base)?.replace('\t', "    ");
    let lines = value.split('\n').collect::<Vec<_>>();
    if alignment == ParagraphAlignment::Center {
        return Ok(lines
            .iter()
            .map(|line| format!(">{line}<"))
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let canonical_lines = canonical.split('\n').collect::<Vec<_>>();
    if canonical_lines.len() != lines.len() {
        return Ok(lines
            .iter()
            .map(|line| format!("!{line}"))
            .collect::<Vec<_>>()
            .join("\n"));
    }
    Ok(lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if action_line_should_force(&canonical_lines, index)
                || action_line_should_force(&lines, index)
            {
                format!("!{line}")
            } else {
                (*line).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn forced_action(text: &str, runs: &[TextRun], base: EffectiveStyle) -> Result<String, ExportError> {
    let value = inline_text(text, runs, base)?.replace('\t', "    ");
    Ok(value
        .split('\n')
        .map(|line| format!("!{line}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn forced_unmapped_action(
    text: &str,
    runs: &[TextRun],
    base: EffectiveStyle,
    alignment: ParagraphAlignment,
) -> Result<String, ExportError> {
    if alignment == ParagraphAlignment::Center {
        let value = inline_text(text, runs, base)?.replace('\t', "    ");
        Ok(value
            .split('\n')
            .map(|line| format!(">{line}<"))
            .collect::<Vec<_>>()
            .join("\n"))
    } else {
        forced_action(text, runs, base)
    }
}

fn normalized_action_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ")
}

fn action_line_should_force(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    action_line_needs_force(line)
        || (is_uppercase_character_line(line)
            && lines[index + 1..]
                .iter()
                .any(|following| !following.trim().is_empty()))
}

fn force_scene_heading(value: &str) -> String {
    if let Some((index, _)) = value.char_indices().find(|(_, character)| character.is_alphanumeric()) {
        let mut forced = String::with_capacity(value.len() + 1);
        forced.push_str(&value[..index]);
        forced.push('.');
        forced.push_str(&value[index..]);
        forced
    } else {
        format!(".{value}")
    }
}

fn action_line_needs_force(line: &str) -> bool {
    let source = line.trim_start();
    if source.is_empty() {
        return true;
    }
    has_natural_scene_prefix(source)
        || is_natural_transition(source)
        || source.starts_with('>')
        || source.starts_with('~')
        || source.starts_with('=')
        || source.starts_with('#')
        || source.starts_with("[[")
        || source.starts_with("/*")
        || source == "==="
        || source.starts_with('@')
        || source.starts_with('!')
        || source
            .strip_prefix('.')
            .and_then(|rest| rest.chars().next())
            .is_some_and(char::is_alphanumeric)
}

fn is_natural_transition(line: &str) -> bool {
    line.ends_with("TO:")
        && line.chars().any(char::is_alphabetic)
        && line.chars().all(|character| {
            !character.is_alphabetic() || character.to_uppercase().eq(std::iter::once(character))
        })
}

fn is_uppercase_character_line(line: &str) -> bool {
    line.chars().any(char::is_alphabetic)
        && line.chars().all(|character| {
            !character.is_alphabetic() || character.to_uppercase().eq(std::iter::once(character))
        })
}

fn has_natural_scene_prefix(value: &str) -> bool {
    let value = value.trim_start();
    ["INT./EXT", "INT/EXT", "INT", "EXT", "EST", "I/E"]
        .iter()
        .any(|prefix| {
            value
                .get(..prefix.len())
                .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
                && value
                    .get(prefix.len()..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|next| next == '.' || next == ' ')
        })
}
