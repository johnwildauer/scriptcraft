//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses fixed integer metrics rather than a platform font or viewport.
//! The initial profile therefore has stable structure on every host: one
//! character cell is seven units wide and one line is twelve units high.

use crate::{ModelError, PageProfile, Project, Role};

pub const FONT_FAMILY: &str = "ScriptCraft Fixed Metrics";
pub const CHAR_WIDTH: u32 = 7;
pub const LINE_HEIGHT: u32 = 12;
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayLayout {
    pub profile: PageProfile,
    pub font_family: &'static str,
    pub char_width: u32,
    pub line_height: u32,
    pub pages: Vec<LayoutPage>,
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
        project.validate()?;
        let profile = project.page_profile.clone();
        validate_profile(&profile)?;
        let mut output = Self {
            profile,
            font_family: FONT_FAMILY,
            char_width: CHAR_WIDTH,
            line_height: LINE_HEIGHT,
            pages: Vec::new(),
        };
        if !project.title_page.is_empty() {
            let mut page = output.new_page(true);
            for (index, element) in project.title_page.iter().enumerate() {
                let lines = wrap_element(element.text.as_str(), index, element.role, 55);
                for line in lines {
                    page.lines.push(output.make_line(&page, line, true));
                }
            }
            output.pages.push(page);
        }
        if output.pages.last().is_some_and(|page| page.title_page) {
            output.pages.push(output.new_page(false));
        }
        for (index, element) in project.elements.iter().enumerate() {
            let width = role_width(&output.profile, element.role);
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
        let units = x.saturating_sub(line.rect.x);
        let cells = (units.saturating_add(CHAR_WIDTH / 2)) / CHAR_WIDTH;
        let offset = line
            .text
            .char_indices()
            .nth(cells as usize)
            .map_or(line.source.end, |(at, _)| line.source.start + at);
        Some(Caret {
            element: line.element,
            offset,
        })
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
                        x: line.rect.x + columns.saturating_mul(CHAR_WIDTH),
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
                width: self.profile.paper_width,
                height: self.profile.paper_height,
            },
            content_bounds: Rect {
                x: self.profile.margin_left,
                y: self.profile.margin_top,
                width: self.profile.paper_width
                    - self.profile.margin_left
                    - self.profile.margin_right,
                height: self.profile.paper_height
                    - self.profile.margin_top
                    - self.profile.margin_bottom,
            },
            lines: Vec::new(),
        }
    }

    fn make_line(&self, page: &LayoutPage, line: WrappedLine, title_page: bool) -> LayoutLine {
        let (indent, width) = if title_page {
            (0, self.profile.chars_per_line)
        } else {
            (
                role_indent_for(&self.profile, line.role),
                role_width(&self.profile, line.role),
            )
        };
        let line_number = page.lines.len() as u32;
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
                x: self.profile.margin_left + indent.saturating_mul(CHAR_WIDTH),
                y: self.profile.margin_top + line_number.saturating_mul(LINE_HEIGHT),
                width: width.saturating_mul(CHAR_WIDTH),
                height: LINE_HEIGHT,
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
        let capacity = self.profile.lines_per_page as usize;
        if !lines.is_empty()
            && !self.pages.last().is_some_and(|page| page.title_page)
            && self.pages.last().map_or(0, |page| page.lines.len()) > 0
            && capacity.saturating_sub(self.pages.last().map_or(0, |page| page.lines.len()))
                < minimum
        {
            self.pages.push(self.new_page(false));
        }
        let oversized_element = lines.len() > capacity;
        for wrapped in lines {
            let full = self.pages.last().map_or(0, |page| page.lines.len()) >= capacity;
            if full {
                self.pages.push(self.new_page(false));
            }
            let page_index = self.pages.len() - 1;
            let page = self.pages[page_index].clone();
            let mut wrapped = wrapped;
            wrapped.oversized |= oversized_element;
            let line = self.make_line(&page, wrapped, false);
            self.pages[page_index].lines.push(line);
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

fn validate_profile(profile: &PageProfile) -> Result<(), LayoutError> {
    if profile.margin_left.saturating_add(profile.margin_right) >= profile.paper_width
        || profile.margin_top.saturating_add(profile.margin_bottom) >= profile.paper_height
        || profile.lines_per_page == 0
        || profile.chars_per_line == 0
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok(())
}

fn role_indent(role: Role) -> u32 {
    match role {
        Role::Character => 15,
        Role::Dialogue => 10,
        Role::Parenthetical => 12,
        Role::Transition => 30,
        _ => 0,
    }
}

fn role_measure(role: Role, profile_width: u32) -> u32 {
    let measure = match role {
        Role::Character => 25,
        Role::Dialogue => 35,
        Role::Parenthetical => 30,
        _ => 55,
    };
    measure.min(profile_width.max(1))
}

fn role_width(profile: &PageProfile, role: Role) -> u32 {
    let content_chars = (profile
        .paper_width
        .saturating_sub(profile.margin_left)
        .saturating_sub(profile.margin_right))
        / CHAR_WIDTH;
    role_measure(
        role,
        profile
            .chars_per_line
            .min(content_chars.saturating_sub(role_indent_for(profile, role))),
    )
}

fn role_indent_for(profile: &PageProfile, role: Role) -> u32 {
    let content_chars = (profile
        .paper_width
        .saturating_sub(profile.margin_left)
        .saturating_sub(profile.margin_right))
        / CHAR_WIDTH;
    role_indent(role).min(content_chars.saturating_sub(1))
}

fn keep_with_next(role: Role) -> bool {
    matches!(
        role,
        Role::SceneHeading | Role::Character | Role::Parenthetical
    )
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
        let mut project = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
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
        let mut p = project(&[(Role::Action, "abcdefghij")]);
        p.page_profile.chars_per_line = 4;
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
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
        assert_eq!(page.content_bounds.x, 90);
        assert_eq!(page.content_bounds.y, 72);
        assert_eq!(page.content_bounds.width, 450);
        assert_eq!(page.content_bounds.height, 648);
        assert_eq!(page.lines[0].rect.x, 90 + 15 * CHAR_WIDTH);
        assert!(page
            .lines
            .iter()
            .all(|line| line.rect.x + line.rect.width <= 540));
    }

    #[test]
    fn fixed_metric_line_boxes_and_carets_have_twelve_unit_spacing() {
        let p = project(&[(Role::Action, "first"), (Role::Dialogue, "second")]);
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let lines = &layout.pages[0].lines;
        assert_eq!(layout.line_height, 12);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].rect.y, 72);
        assert_eq!(lines[1].rect.y, 84);
        assert_eq!(lines[1].rect.y - lines[0].rect.y, layout.line_height);
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
    fn empty_wrapped_and_keep_lines_preserve_geometry_at_page_boundaries() {
        let mut p = project(&[
            (Role::SceneHeading, "INT. ROOM"),
            (Role::Action, ""),
            (Role::Action, "abcdefghij"),
            (Role::Character, "BOB"),
            (Role::Dialogue, "Hello"),
        ]);
        p.page_profile.chars_per_line = 4;
        p.page_profile.lines_per_page = 3;
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());

        assert!(layout.pages.len() > 1);
        assert!(layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .any(|line| line.text.is_empty()));
        assert!(layout.pages.iter().any(|page| {
            page.lines.iter().any(|line| line.text == "abcd")
                && page.lines.iter().any(|line| line.text == "efgh")
        }));
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
            page.lines
                .windows(2)
                .all(|lines| lines[1].rect.y - lines[0].rect.y == LINE_HEIGHT)
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
        assert_eq!(layout.pages[0].lines.len(), 55);
        assert_eq!(layout.pages[0].lines[0].rect.y, 72);
        assert_eq!(layout.pages[0].lines[54].rect.y, 72 + 54 * LINE_HEIGHT);
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
        p.page_profile.lines_per_page = 2;
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        assert!(layout.pages[0].title_page);
        assert_eq!(layout.pages[1].lines[0].role, Role::SceneHeading);
        assert!(layout.pages[1].lines[0].keep_with_next);
    }

    #[test]
    fn oversized_elements_terminate_without_blank_pages() {
        let mut p = project(&[(Role::Action, &"x".repeat(100))]);
        p.page_profile.chars_per_line = 3;
        p.page_profile.lines_per_page = 2;
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
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
}
