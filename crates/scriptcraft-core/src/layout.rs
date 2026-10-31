//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses explicit effective inputs and fixed integer metrics rather than
//! a platform font or viewport. The deterministic defaults therefore have stable structure on every host: one
//! character cell is seven units wide and one line is twelve units high.

use crate::{
    validate_layout_inputs, LayoutInputs, ModelError, PrintVisibility, Project, Role,
    SourceLayoutSettings,
};
use std::sync::Arc;

pub const FONT_FAMILY: &str = "Courier-family fixed metrics (12 pt target)";
pub const CHAR_WIDTH: u32 = 7;
pub const LINE_HEIGHT: u32 = 12;
pub const POINTS_PER_INCH: u32 = 72;
const MAX_LINES_PER_ELEMENT: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub element: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutLine {
    pub page: usize,
    pub role: Role,
    pub element: usize,
    pub source: SourceRange,
    pub text: String,
    pub rect: Rect,
    pub keep_with_next: bool,
    pub oversized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPage {
    pub number: usize,
    pub title_page: bool,
    pub bounds: Rect,
    pub content_bounds: Rect,
    pub lines: Vec<LayoutLine>,
    pub occupied_lines: u32,
    next_line_y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayLayout {
    pub inputs: LayoutInputs,
    pub source_settings: SourceLayoutSettings,
    pub font_family: String,
    pub char_width: u32,
    pub line_height: u32,
    pub pages: Vec<LayoutPage>,
}

/// Session-owned immutable layout cache shared by desktop adapters and
/// headless measurements. The revision, effective geometry and source-derived
/// layout settings are the cache key; viewport presentation state is excluded.
#[derive(Debug, Default)]
pub struct LayoutCache {
    revision: Option<u64>,
    inputs: Option<LayoutInputs>,
    source_settings: Option<SourceLayoutSettings>,
    layout: Option<Arc<ScreenplayLayout>>,
}

impl LayoutCache {
    pub fn get_or_build(
        &mut self,
        revision: u64,
        project: &Project,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        let inputs = project.session_layout_inputs();
        self.get_or_build_with_inputs(revision, project, &inputs)
    }

    pub fn get_or_build_with_inputs(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        let source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings.clone())
            .unwrap_or_default();
        if self.revision == Some(revision)
            && self.inputs.as_ref() == Some(inputs)
            && self.source_settings.as_ref() == Some(&source_settings)
        {
            return self
                .layout
                .clone()
                .ok_or(LayoutError {
                    kind: LayoutErrorKind::NoGeometry,
                })
                .map(|layout| (layout, true));
        }
        let layout = Arc::new(ScreenplayLayout::build_with_inputs(project, inputs)?);
        self.revision = Some(revision);
        self.inputs = Some(inputs.clone());
        self.source_settings = Some(source_settings);
        self.layout = Some(layout.clone());
        Ok((layout, false))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caret {
    pub element: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutError {
    pub kind: LayoutErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutErrorKind {
    Model(ModelError),
    InvalidProfile,
    InvalidPosition,
    NoGeometry,
}

impl From<ModelError> for LayoutError {
    fn from(error: ModelError) -> Self {
        Self {
            kind: LayoutErrorKind::Model(error),
        }
    }
}

impl ScreenplayLayout {
    pub fn build(project: &Project) -> Result<Self, LayoutError> {
        let inputs = project.session_layout_inputs();
        Self::build_with_inputs(project, &inputs)
    }

    pub fn build_with_inputs(
        project: &Project,
        inputs: &LayoutInputs,
    ) -> Result<Self, LayoutError> {
        let source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings.clone())
            .unwrap_or_default();
        Self::build_with_effective_inputs(project, inputs, &source_settings)
    }

    pub fn build_with_effective_inputs(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
    ) -> Result<Self, LayoutError> {
        project.validate()?;
        validate_layout_inputs(inputs).map_err(|_| LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        })?;
        validate_profile(inputs)?;
        let line_advance = inputs.line_advance;
        let char_width = inputs.char_width;
        let font_family = inputs.font_family.clone();
        let mut output = Self {
            inputs: inputs.clone(),
            source_settings: source_settings.clone(),
            font_family,
            char_width,
            line_height: line_advance,
            pages: Vec::new(),
        };
        if !project.title_page.is_empty() {
            let mut page = output.new_page(true);
            for (index, element) in project.title_page.iter().enumerate() {
                let lines = wrap_element(
                    element.text.as_str(),
                    index,
                    element.role,
                    output
                        .inputs
                        .paper_width
                        .saturating_sub(output.inputs.margin_left)
                        .saturating_sub(output.inputs.margin_right)
                        / output.inputs.char_width.max(1),
                );
                for line in lines {
                    page.lines.push(output.make_line(&page, line, true));
                    page.occupied_lines = page.occupied_lines.saturating_add(1);
                }
            }
            output.pages.push(page);
        }
        if output.pages.last().is_some_and(|page| page.title_page) {
            output.pages.push(output.new_page(false));
        }
        for (index, element) in project.elements.iter().enumerate() {
            if output.source_settings.print_visibility(element.role) == PrintVisibility::Hidden {
                continue;
            }
            if output.source_settings.starts_new_page(element.role)
                && output
                    .pages
                    .last()
                    .is_some_and(|page| !page.title_page && !page.lines.is_empty())
            {
                output.pages.push(output.new_page(false));
            }
            let width = role_width(&output.inputs, element.role);
            let lines = wrap_element(element.text.as_str(), index, element.role, width);
            output.place_element(lines, element.role);
        }
        if output.pages.is_empty() {
            output.pages.push(output.new_page(false));
        }
        Ok(output)
    }

    pub fn hit_test(&self, page: usize, x: u32, y: u32) -> Option<Caret> {
        let page = self.pages.get(page)?;
        let line = page.lines.iter().find(|line| {
            x >= line.rect.x
                && x <= line.rect.x.saturating_add(line.rect.width)
                && y >= line.rect.y
                && y < line.rect.y.saturating_add(line.rect.height)
        })?;
        Some(self.caret_for_line(line, x as i64))
    }

    /// Hit-test the body as one document-flow domain. `page` and `y` describe
    /// the pointer in page-local coordinates; the page height is used only to
    /// compare lines across pages. Horizontal placement is deliberately
    /// clamped to the selected line's source range.
    pub fn nearest_line_hit_test(&self, page: usize, x: i64, y: i64) -> Option<Caret> {
        if self.pages.get(page)?.title_page {
            return None;
        }
        let page_height = i64::from(self.inputs.paper_height);
        let pointer_y = i64::try_from(page)
            .ok()?
            .saturating_mul(page_height)
            .saturating_add(y);
        self.pages
            .iter()
            .enumerate()
            .filter(|(_, candidate)| !candidate.title_page)
            .flat_map(|(page_index, candidate)| {
                candidate
                    .lines
                    .iter()
                    .enumerate()
                    .map(move |(line_index, line)| {
                        let line_y = i64::try_from(page_index)
                            .unwrap_or(i64::MAX)
                            .saturating_mul(page_height)
                            .saturating_add(i64::from(line.rect.y));
                        let distance = (pointer_y - line_y).abs();
                        (distance, page_index, line_index, line)
                    })
            })
            .min_by_key(|(distance, page_index, line_index, _)| {
                // Stable document order is the tie-breaker: earlier lines,
                // then lower page indexes, win exact vertical ties.
                (*distance, *page_index, *line_index)
            })
            .map(|(_, _, _, line)| self.caret_for_line(line, x))
    }

    fn caret_for_line(&self, line: &LayoutLine, x: i64) -> Caret {
        let units = x.saturating_sub(i64::from(line.rect.x)).max(0) as u32;
        let cells = (units.saturating_add(self.char_width / 2)) / self.char_width.max(1);
        let offset = line
            .text
            .char_indices()
            .nth(cells as usize)
            .map_or(line.source.end, |(at, _)| line.source.start + at);
        Caret {
            element: line.element,
            offset,
        }
    }

    pub fn caret_rect(&self, caret: Caret) -> Result<Rect, LayoutError> {
        for page in &self.pages {
            for line in &page.lines {
                if line.element == caret.element
                    && caret.offset >= line.source.start
                    && caret.offset <= line.source.end
                    && is_boundary_in_line(line, caret.offset)
                {
                    let columns = line.text[..caret.offset - line.source.start]
                        .chars()
                        .count() as u32;
                    return Ok(Rect {
                        x: line.rect.x + columns.saturating_mul(self.char_width),
                        y: line.rect.y,
                        width: 1,
                        height: line.rect.height,
                    });
                }
            }
        }
        Err(LayoutError {
            kind: LayoutErrorKind::InvalidPosition,
        })
    }

    fn new_page(&self, title_page: bool) -> LayoutPage {
        LayoutPage {
            number: self.pages.len(),
            title_page,
            bounds: Rect {
                x: 0,
                y: 0,
                width: self.inputs.paper_width,
                height: self.inputs.paper_height,
            },
            content_bounds: Rect {
                x: self.inputs.margin_left,
                y: self.inputs.margin_top,
                width: self.inputs.paper_width - self.inputs.margin_left - self.inputs.margin_right,
                height: self.inputs.paper_height
                    - self.inputs.margin_top
                    - self.inputs.margin_bottom,
            },
            lines: Vec::new(),
            occupied_lines: 0,
            next_line_y: self.inputs.margin_top,
        }
    }

    fn make_line(&self, page: &LayoutPage, line: WrappedLine, title_page: bool) -> LayoutLine {
        let (x, width) = if title_page {
            (
                self.inputs.margin_left,
                self.inputs.paper_width - self.inputs.margin_left - self.inputs.margin_right,
            )
        } else {
            role_box(&self.inputs, line.role)
        };
        LayoutLine {
            page: page.number,
            role: line.role,
            element: line.element,
            source: SourceRange {
                element: line.element,
                start: line.start,
                end: line.end,
            },
            text: line.text,
            rect: Rect {
                x,
                y: page.next_line_y,
                width,
                height: self.inputs.line_advance,
            },
            keep_with_next: !title_page && keep_with_next(line.role),
            oversized: line.oversized,
        }
    }

    fn place_element(&mut self, lines: Vec<WrappedLine>, role: Role) {
        let keep = keep_with_next(role);
        let minimum = if keep { 2 } else { 1 };
        if self.pages.is_empty() {
            self.pages.push(self.new_page(false));
        }
        let capacity = self.inputs.lines_per_page as usize;
        let has_boundary = self.pages.last().is_some_and(|page| !page.lines.is_empty());
        let previous_role = self
            .pages
            .last()
            .and_then(|page| page.lines.last())
            .map(|line| line.role);
        let extra_gap = previous_role
            .map(|previous| role_boundary_gap(&self.inputs, previous, role))
            .unwrap_or(0);
        if !lines.is_empty()
            && !self.pages.last().is_some_and(|page| page.title_page)
            && has_boundary
            && self
                .pages
                .last()
                .map_or(0, |page| page.occupied_lines as usize)
                .saturating_add(minimum)
                .saturating_add(usize::from(extra_gap > 0))
                > capacity
        {
            self.pages.push(self.new_page(false));
        }
        if has_boundary && self.pages.last().is_some_and(|page| !page.lines.is_empty()) {
            // The boundary gap is point geometry, not a count of whole line
            // boxes. Keep it exact even when it is not a multiple of the
            // profile's line advance.
            if let Some(page) = self.pages.last_mut() {
                page.next_line_y = page.next_line_y.saturating_add(extra_gap);
                // Keep pagination's existing logical line-capacity contract
                // while the cursor above retains the exact point distance.
                page.occupied_lines = page
                    .occupied_lines
                    .saturating_add(usize::from(extra_gap > 0) as u32);
            }
        }
        let oversized_element = lines.len() > capacity;
        for wrapped in lines {
            let full = self
                .pages
                .last()
                .map_or(0, |page| page.occupied_lines as usize)
                >= capacity;
            if full {
                self.pages.push(self.new_page(false));
            }
            let page_index = self.pages.len() - 1;
            if self.pages[page_index].lines.is_empty() {
                self.pages[page_index].occupied_lines = 0;
            }
            let page = self.pages[page_index].clone();
            let mut wrapped = wrapped;
            wrapped.oversized |= oversized_element;
            let line = self.make_line(&page, wrapped, false);
            self.pages[page_index].lines.push(line);
            self.pages[page_index].occupied_lines =
                self.pages[page_index].occupied_lines.saturating_add(1);
            self.pages[page_index].next_line_y = self.pages[page_index]
                .next_line_y
                .saturating_add(self.inputs.line_advance);
        }
    }
}

#[derive(Debug)]
struct WrappedLine {
    element: usize,
    role: Role,
    start: usize,
    end: usize,
    text: String,
    oversized: bool,
}

fn validate_profile(inputs: &LayoutInputs) -> Result<(), LayoutError> {
    if inputs.margin_left.saturating_add(inputs.margin_right) >= inputs.paper_width
        || inputs.margin_top.saturating_add(inputs.margin_bottom) >= inputs.paper_height
        || inputs.lines_per_page == 0
        || inputs.chars_per_line == 0
        || inputs.line_advance == 0
        || inputs
            .role_boxes
            .iter()
            .any(|box_| box_.width == 0 || box_.x.saturating_add(box_.width) > inputs.paper_width)
        || inputs
            .additional_role_boxes
            .iter()
            .flatten()
            .any(|box_| box_.width == 0 || box_.x.saturating_add(box_.width) > inputs.paper_width)
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok(())
}

fn role_width(inputs: &LayoutInputs, role: Role) -> u32 {
    let (_, width) = role_box(inputs, role);
    inputs
        .chars_per_line
        .min(width / inputs.char_width.max(1))
        .max(1)
}

fn role_box(inputs: &LayoutInputs, role: Role) -> (u32, u32) {
    if let Some(box_) = inputs.role_box(role) {
        return (box_.x, box_.width);
    }
    // New roles have no asserted ScriptCraft geometry yet. Use the full text
    // area as a neutral layout fallback without changing their semantic role.
    (
        inputs.margin_left,
        inputs
            .paper_width
            .saturating_sub(inputs.margin_left)
            .saturating_sub(inputs.margin_right)
            .max(1),
    )
}

fn keep_with_next(role: Role) -> bool {
    matches!(
        role,
        Role::SceneHeading | Role::Character | Role::Parenthetical
    )
}

fn role_boundary_gap(inputs: &LayoutInputs, _previous: Role, next: Role) -> u32 {
    if matches!(next, Role::Dialogue | Role::Parenthetical) {
        0
    } else {
        inputs.boundary_gap
    }
}

fn is_boundary_in_line(line: &LayoutLine, offset: usize) -> bool {
    if offset == line.source.start || offset == line.source.end {
        return true;
    }
    line.text.is_char_boundary(offset - line.source.start)
}

fn wrap_element(text: &str, element: usize, role: Role, width: u32) -> Vec<WrappedLine> {
    let limit = width.max(1) as usize;
    let mut output = Vec::new();
    let mut paragraph_start = 0;
    let mut delimiters = text.char_indices().peekable();
    while let Some((newline, delimiter)) = delimiters.next() {
        if delimiter != '\n' && delimiter != '\r' {
            continue;
        }
        wrap_segment(
            &text[paragraph_start..newline],
            paragraph_start,
            element,
            role,
            limit,
            &mut output,
        );
        paragraph_start = newline + delimiter.len_utf8();
        if delimiter == '\r' && delimiters.peek().is_some_and(|(_, next)| *next == '\n') {
            let _ = delimiters.next();
            paragraph_start += 1;
        }
    }
    wrap_segment(
        text[paragraph_start..].as_ref(),
        paragraph_start,
        element,
        role,
        limit,
        &mut output,
    );
    if output.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: 0,
            end: 0,
            text: String::new(),
            oversized: false,
        });
    }
    output
}

fn wrap_segment(
    text: &str,
    base: usize,
    element: usize,
    role: Role,
    limit: usize,
    output: &mut Vec<WrappedLine>,
) {
    if text.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: base,
            end: base,
            text: String::new(),
            oversized: false,
        });
        return;
    }
    let mut start = 0;
    while start < text.len() && output.len() < MAX_LINES_PER_ELEMENT {
        let mut end = text.len();
        let mut count = 0;
        for (at, _) in text[start..].char_indices() {
            if count == limit {
                end = start + at;
                break;
            }
            count += 1;
        }
        if end == text.len() {
            end = text.len();
        } else if let Some(space) = text[start..end].rfind(char::is_whitespace) {
            let candidate = start + space + 1;
            if candidate > start {
                end = candidate;
            }
        }
        if end <= start {
            end = text[start..]
                .char_indices()
                .nth(1)
                .map_or(text.len(), |(at, _)| start + at);
        }
        let source = &text[start..end];
        output.push(WrappedLine {
            element,
            role,
            start: base + start,
            end: base + end,
            text: source.to_owned(),
            oversized: count > limit,
        });
        start = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Project, ScreenplayElement};

    fn project(elements: &[(Role, &str)]) -> Project {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = elements
            .iter()
            .enumerate()
            .map(|(index, (role, text))| {
                ScreenplayElement::new(format!("e-{index}"), *role, *text)
                    .unwrap_or_else(|_| unreachable!())
            })
            .collect();
        project
    }

    #[test]
    fn wrapping_is_bounded_and_source_mapped() {
        let p = project(&[(Role::Action, "abcdefghij")]);
        let inputs = LayoutInputs {
            chars_per_line: 4,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages.len(), 1);
        assert!(layout.pages[0]
            .lines
            .iter()
            .all(|line| line.text.chars().count() <= 4));
        assert_eq!(
            layout.pages[0].lines[0].source,
            SourceRange {
                element: 0,
                start: 0,
                end: 4
            }
        );
        assert_eq!(layout.pages[0].lines[2].source.end, 10);
    }

    #[test]
    fn margins_bounds_and_role_boxes_are_fixed() {
        let p = project(&[(Role::Character, "BOB"), (Role::Dialogue, "Hello")]);
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let page = &layout.pages[0];
        assert_eq!(page.bounds.width, 612);
        assert_eq!(page.bounds.height, 792);
        assert_eq!(page.content_bounds.x, 108);
        assert_eq!(page.content_bounds.y, 72);
        assert_eq!(page.content_bounds.width, 432);
        assert_eq!(page.content_bounds.height, 648);
        assert_eq!(page.lines[0].rect.x, 259);
        assert!(page
            .lines
            .iter()
            .all(|line| line.rect.x + line.rect.width <= 540));
    }

    #[test]
    fn approved_role_edges_are_point_based_and_notes_inherit_action() {
        let p = project(&[
            (Role::SceneHeading, "scene"),
            (Role::Action, "action"),
            (Role::Note, "note"),
            (Role::Character, "BOB"),
            (Role::Parenthetical, "(quiet)"),
            (Role::Dialogue, "hello"),
            (Role::Transition, "CUT TO:"),
        ]);
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let boxes: Vec<_> = layout.pages.iter().flat_map(|p| &p.lines).collect();
        assert_eq!((boxes[0].rect.x, boxes[0].rect.width), (108, 432));
        assert_eq!((boxes[1].rect.x, boxes[1].rect.width), (108, 432));
        assert_eq!((boxes[2].rect.x, boxes[2].rect.width), (108, 432));
        assert_eq!((boxes[3].rect.x, boxes[3].rect.width), (259, 175));
        assert_eq!((boxes[4].rect.x, boxes[4].rect.width), (216, 180));
        assert_eq!((boxes[5].rect.x, boxes[5].rect.width), (180, 216));
        assert_eq!((boxes[6].rect.x, boxes[6].rect.width), (432, 108));
    }

    #[test]
    fn alternate_profile_drives_role_wrap_line_and_caret_geometry() {
        let p = project(&[(Role::Action, "abcdefghijk"), (Role::Dialogue, "second")]);
        let mut inputs = LayoutInputs {
            char_width: 5,
            line_advance: 15,
            boundary_gap: 15,
            ..LayoutInputs::default()
        };
        inputs.role_boxes[1] = crate::RoleBox { x: 120, width: 50 };
        inputs.role_boxes[3] = crate::RoleBox { x: 200, width: 40 };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.char_width, 5);
        assert_eq!(layout.line_height, 15);
        assert_eq!(
            (
                layout.pages[0].lines[0].rect.x,
                layout.pages[0].lines[0].rect.width
            ),
            (120, 50)
        );
        assert_eq!(
            (
                layout.pages[0].lines[1].rect.x,
                layout.pages[0].lines[1].rect.width
            ),
            (120, 50)
        );
        assert_eq!(layout.pages[0].lines[2].rect.x, 200);
        assert_eq!(layout.pages[0].lines[2].rect.height, 15);
        assert_eq!(
            layout.pages[0].lines[2].rect.y - layout.pages[0].lines[1].rect.y,
            15
        );
        let caret = layout
            .caret_rect(Caret {
                element: 1,
                offset: 0,
            })
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(caret.height, 15);
        assert_eq!(
            layout.hit_test(0, 200 + 2 * 5, caret.y + 1),
            Some(Caret {
                element: 1,
                offset: 2
            })
        );
    }

    #[test]
    fn effective_layout_inputs_cover_profile_metrics_and_page_geometry() {
        let p = project(&[(Role::Action, "a screenplay line")]);
        let mut inputs = LayoutInputs {
            name: "runtime-alternate".into(),
            format_version: 2,
            paper_width: 720,
            paper_height: 900,
            margin_top: 30,
            margin_bottom: 40,
            margin_left: 50,
            margin_right: 60,
            chars_per_line: 40,
            lines_per_page: 40,
            line_advance: 14,
            boundary_gap: 9,
            font_family: "Courier".into(),
            font_size: 11,
            char_width: 8,
            ..LayoutInputs::default()
        };
        inputs.role_boxes[1] = crate::RoleBox { x: 160, width: 320 };

        let first =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        let second =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        assert_eq!(first, second);
        assert_eq!(first.inputs, inputs);
        assert_eq!(first.pages[0].bounds.width, 720);
        assert_eq!(first.pages[0].bounds.height, 900);
        assert_eq!(first.pages[0].content_bounds.x, 50);
        assert_eq!(first.pages[0].content_bounds.y, 30);
        assert_eq!(first.pages[0].content_bounds.width, 610);
        assert_eq!(first.pages[0].content_bounds.height, 830);
        assert_eq!(first.pages[0].lines[0].rect.x, 160);
        assert_eq!(first.pages[0].lines[0].rect.y, 30);
        assert_eq!(first.pages[0].lines[0].rect.height, 14);
    }

    #[test]
    fn every_role_accepts_independent_runtime_geometry() {
        let roles_and_text = Role::ALL.map(|role| (role, "x"));
        let p = project(&roles_and_text);
        let mut inputs = LayoutInputs::default();
        let boxes = Role::ALL.map(|_| crate::RoleBox { x: 40, width: 300 });
        inputs
            .role_boxes
            .copy_from_slice(&boxes[..crate::LEGACY_ROLE_BOX_COUNT]);
        for (index, box_) in boxes[crate::LEGACY_ROLE_BOX_COUNT..].iter().enumerate() {
            inputs.additional_role_boxes[index] = Some(*box_);
        }

        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        let lines = layout
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), Role::ALL.len());
        for (index, role) in Role::ALL.iter().enumerate() {
            let line = lines
                .iter()
                .find(|line| line.role == *role)
                .unwrap_or_else(|| unreachable!());
            assert_eq!(line.rect.x, 40, "role index {index}");
            assert_eq!(line.rect.width, 300, "role index {index}");
        }
    }

    #[test]
    fn cache_key_tracks_source_derived_visibility_settings() {
        let visible = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Note'><FontSpec Style='Italic'/></ElementSettings><Content><Paragraph Type='Note' id='note'><Text>note</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let hidden = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Note'><FontSpec Style='Italic+HiddenText'/></ElementSettings><Content><Paragraph Type='Note' id='note'><Text>note</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(visible.project.elements, hidden.project.elements);

        let mut cache = LayoutCache::default();
        let inputs = LayoutInputs::default();
        let (visible_layout, hit) = cache
            .get_or_build_with_inputs(4, &visible.project, &inputs)
            .unwrap_or_else(|_| unreachable!());
        assert!(!hit);
        assert_eq!(visible_layout.pages[0].lines.len(), 1);

        let (hidden_layout, hit) = cache
            .get_or_build_with_inputs(4, &hidden.project, &inputs)
            .unwrap_or_else(|_| unreachable!());
        assert!(!hit);
        assert!(hidden_layout.pages[0].lines.is_empty());
    }

    #[test]
    fn non_multiple_boundary_gap_is_preserved_as_point_geometry() {
        let p = project(&[(Role::Action, "first"), (Role::Action, "second")]);
        let inputs = LayoutInputs {
            line_advance: 14,
            boundary_gap: 10,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        let lines = &layout.pages[0].lines;
        assert_eq!(lines[0].rect.y, 72);
        assert_eq!(lines[1].rect.y - lines[0].rect.y, 24);
        assert_eq!(
            lines[1].rect.y,
            lines[0].rect.y + inputs.line_advance + inputs.boundary_gap
        );
    }

    #[test]
    fn fixed_metric_line_boxes_and_carets_have_twelve_unit_spacing() {
        let p = project(&[(Role::Action, "first"), (Role::Action, "second")]);
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let lines = &layout.pages[0].lines;
        assert_eq!(layout.line_height, 12);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].rect.y, 72);
        assert_eq!(lines[1].rect.y, 96);
        assert_eq!(lines[1].rect.y - lines[0].rect.y, layout.line_height * 2);
        assert!(lines.iter().all(|line| line.rect.height == LINE_HEIGHT));
        for line in lines {
            let caret = layout
                .caret_rect(Caret {
                    element: line.element,
                    offset: line.source.start,
                })
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(caret.height, LINE_HEIGHT);
            assert_eq!(caret.y, line.rect.y);
        }
    }

    #[test]
    fn every_incoming_dialogue_or_parenthetical_has_no_extra_gap() {
        let predecessors = [
            Role::SceneHeading,
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Parenthetical,
            Role::Transition,
            Role::Note,
        ];
        for next in [Role::Dialogue, Role::Parenthetical] {
            for previous in predecessors {
                let p = project(&[(previous, "before"), (next, "after")]);
                let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
                let lines = &layout.pages[0].lines;
                assert_eq!(
                    lines[1].rect.y - lines[0].rect.y,
                    layout.inputs.line_advance
                );
            }
        }
    }

    #[test]
    fn other_incoming_roles_use_boundary_gap() {
        let pairs = [
            (Role::Action, Role::Action),
            (Role::Dialogue, Role::Action),
            (Role::Parenthetical, Role::Character),
            (Role::Note, Role::SceneHeading),
        ];
        for (previous, next) in pairs {
            let p = project(&[(previous, "before"), (next, "after")]);
            let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
            let lines = &layout.pages[0].lines;
            assert_eq!(
                lines[1].rect.y - lines[0].rect.y,
                layout.inputs.line_advance + layout.inputs.boundary_gap
            );
        }
    }

    #[test]
    fn zero_gap_does_not_consume_page_capacity() {
        let p = project(&[(Role::Action, "before"), (Role::Dialogue, "after")]);
        let inputs = LayoutInputs {
            lines_per_page: 2,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages.len(), 1);
        assert_eq!(layout.pages[0].occupied_lines, 2);
    }

    #[test]
    fn empty_wrapped_and_keep_lines_preserve_geometry_at_page_boundaries() {
        let p = project(&[
            (Role::SceneHeading, "INT. ROOM"),
            (Role::Action, ""),
            (Role::Action, "abcdefghij"),
            (Role::Character, "BOB"),
            (Role::Dialogue, "Hello"),
        ]);
        let inputs = LayoutInputs {
            chars_per_line: 4,
            lines_per_page: 3,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());

        assert!(layout.pages.len() > 1);
        assert!(layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .any(|line| line.text.is_empty()));
        assert!(layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .any(|line| line.text == "abcd"));
        assert!(layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .any(|line| line.text == "efgh"));
        assert!(layout.pages.iter().any(|page| {
            page.lines
                .iter()
                .any(|line| line.role == Role::Character && line.keep_with_next)
        }));
        assert!(layout
            .pages
            .iter()
            .all(|page| { page.lines.first().is_some_and(|line| line.rect.y == 72) }));
        assert!(layout.pages.iter().all(|page| {
            page.lines.windows(2).all(|lines| {
                let advance = lines[1].rect.y - lines[0].rect.y;
                advance == LINE_HEIGHT || advance == LINE_HEIGHT * 2
            })
        }));
    }

    #[test]
    fn nominal_fifty_five_line_page_resets_to_top_margin() {
        let mut p = project(&[(Role::Action, "line")]);
        p.elements = (0..56)
            .map(|index| {
                ScreenplayElement::new(format!("e-{index}"), Role::Action, "line")
                    .unwrap_or_else(|_| unreachable!())
            })
            .collect();
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages.len(), 2);
        assert!(layout.pages[0].occupied_lines <= 55);
        assert_eq!(layout.pages[0].lines[0].rect.y, 72);
        assert!(layout.pages[0]
            .lines
            .last()
            .is_some_and(|line| line.rect.y <= 792 - 72));
        assert_eq!(layout.pages[1].lines[0].rect.y, 72);
        assert_eq!(layout.pages[1].lines[0].page, 1);
    }

    #[test]
    fn title_page_is_separate_and_keep_rules_are_applied() {
        let mut p = project(&[(Role::SceneHeading, "INT."), (Role::Action, "Action")]);
        p.title_page.push(
            ScreenplayElement::new("title", Role::Action, "Title")
                .unwrap_or_else(|_| unreachable!()),
        );
        let inputs = LayoutInputs {
            lines_per_page: 2,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        assert!(layout.pages[0].title_page);
        assert_eq!(layout.pages[1].lines[0].role, Role::SceneHeading);
        assert!(layout.pages[1].lines[0].keep_with_next);
    }

    #[test]
    fn oversized_elements_terminate_without_blank_pages() {
        let p = project(&[(Role::Action, &"x".repeat(100))]);
        let inputs = LayoutInputs {
            chars_per_line: 3,
            lines_per_page: 2,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages.len(), 17);
        assert!(layout.pages.iter().all(|page| !page.lines.is_empty()));
    }

    #[test]
    fn deterministic_hit_and_caret_round_trip() {
        let p = project(&[(Role::Action, "hello")]);
        let a = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let b = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        assert_eq!(a, b);
        let line = &a.pages[0].lines[0];
        let caret = a.hit_test(0, line.rect.x + 2 * CHAR_WIDTH, line.rect.y + 1);
        assert_eq!(
            caret,
            Some(Caret {
                element: 0,
                offset: 2
            })
        );
        assert!(a
            .caret_rect(caret.unwrap_or(Caret {
                element: 0,
                offset: 0
            }))
            .is_ok());
        assert!(a
            .caret_rect(Caret {
                element: 0,
                offset: 1
            })
            .is_ok());
    }

    #[test]
    fn hidden_roles_are_omitted_without_shifting_source_element_indexes() {
        let p = project(&[
            (Role::Action, "visible before"),
            (Role::Note, "private note"),
            (Role::Outline1, "private outline"),
            (Role::Summary, "private summary"),
            (Role::Action, "visible after"),
        ]);
        let mut settings = SourceLayoutSettings::default();
        for role in [Role::Note, Role::Outline1, Role::Summary] {
            let index = Role::ALL
                .iter()
                .position(|candidate| *candidate == role)
                .unwrap_or_else(|| unreachable!());
            settings.role_print_visibility[index] = Some(PrintVisibility::Hidden);
        }

        let layout =
            ScreenplayLayout::build_with_effective_inputs(&p, &LayoutInputs::default(), &settings)
                .unwrap_or_else(|_| unreachable!());
        let lines = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].element, 0);
        assert_eq!(lines[1].element, 4);
        assert_eq!(lines[0].text, "visible before");
        assert_eq!(lines[1].text, "visible after");
    }

    #[test]
    fn explicit_role_page_start_starts_a_new_body_page() {
        let p = project(&[(Role::Action, "before"), (Role::NewAct, "ACT TWO")]);
        let mut settings = SourceLayoutSettings::default();
        let index = Role::ALL
            .iter()
            .position(|role| *role == Role::NewAct)
            .unwrap_or_else(|| unreachable!());
        settings.role_starts_new_page[index] = Some(true);

        let layout =
            ScreenplayLayout::build_with_effective_inputs(&p, &LayoutInputs::default(), &settings)
                .unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages.len(), 2);
        assert_eq!(layout.pages[0].lines[0].text, "before");
        assert!(layout.pages[1]
            .lines
            .iter()
            .any(|line| line.text == "ACT TWO"));
        assert!(layout.pages[1].lines.iter().all(|line| line.element == 1));
    }

    #[test]
    fn nearest_pointer_clamps_margins_and_round_trips_all_body_roles() {
        let p = project(&[
            (Role::SceneHeading, "INT. ROOM"),
            (Role::Character, "BOB"),
            (Role::Parenthetical, "(quietly)"),
            (Role::Dialogue, "café🙂"),
            (Role::Transition, "CUT TO:"),
            (Role::Note, "note"),
        ]);
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        for page in &layout.pages {
            for line in &page.lines {
                let start = layout
                    .nearest_line_hit_test(
                        page.number,
                        i64::from(line.rect.x).saturating_sub(100),
                        i64::from(line.rect.y),
                    )
                    .unwrap_or_else(|| unreachable!());
                assert_eq!(
                    (start.element, start.offset),
                    (line.element, line.source.start)
                );
                let end = layout
                    .nearest_line_hit_test(
                        page.number,
                        i64::from(line.rect.x.saturating_add(line.rect.width)).saturating_add(100),
                        i64::from(line.rect.y),
                    )
                    .unwrap_or_else(|| unreachable!());
                assert_eq!((end.element, end.offset), (line.element, line.source.end));
                let caret = Caret {
                    element: line.element,
                    offset: line.source.end,
                };
                let rect = layout
                    .caret_rect(caret.clone())
                    .unwrap_or_else(|_| unreachable!());
                assert_eq!(
                    layout.nearest_line_hit_test(page.number, i64::from(rect.x), i64::from(rect.y)),
                    Some(caret)
                );
            }
        }
    }

    #[test]
    fn nearest_pointer_clamps_document_and_ties_to_earlier_line() {
        let p = project(&[(Role::Action, "first"), (Role::Action, "second")]);
        let inputs = LayoutInputs {
            lines_per_page: 1,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        assert!(layout.pages.len() >= 2);
        let first = &layout.pages[0].lines[0];
        let last_page = layout.pages.len() - 1;
        let last = layout.pages[last_page]
            .lines
            .last()
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            layout
                .nearest_line_hit_test(0, 0, -10_000)
                .unwrap_or_else(|| unreachable!())
                .offset,
            first.source.start
        );
        let below = layout
            .nearest_line_hit_test(last_page, i64::MAX, 10_000)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            (below.element, below.offset),
            (last.element, last.source.end)
        );

        let page_height = i64::from(layout.inputs.paper_height);
        let previous_y = i64::from(first.rect.y);
        let next_y = page_height + i64::from(layout.pages[1].lines[0].rect.y);
        let midpoint = (previous_y + next_y) / 2;
        let tie = layout
            .nearest_line_hit_test(1, 0, midpoint - page_height)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(tie.element, first.element);
    }

    #[test]
    fn title_page_is_not_a_body_hit_test_domain() {
        let mut p = project(&[(Role::Action, "body")]);
        p.title_page.push(
            ScreenplayElement::new("title", Role::Action, "title")
                .unwrap_or_else(|_| unreachable!()),
        );
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        assert!(layout.pages[0].title_page);
        assert_eq!(layout.nearest_line_hit_test(0, 0, 0), None);
        assert!(layout.nearest_line_hit_test(1, 0, -10_000).is_some());
    }
}
