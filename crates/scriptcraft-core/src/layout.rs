//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses fixed integer metrics rather than a platform font or viewport.
//! The initial profile therefore has stable structure on every host: one
//! character cell is seven units wide and one line is twelve units high.

use crate::{ModelError, PageProfile, Project, Role};
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
    pub profile: PageProfile,
    pub font_family: String,
    pub char_width: u32,
    pub line_height: u32,
    pub pages: Vec<LayoutPage>,
}

/// Session-owned immutable layout cache shared by desktop adapters and
/// headless measurements.  The revision and complete persisted profile are
/// the cache key; viewport presentation state is intentionally not included.
#[derive(Debug, Default)]
pub struct LayoutCache {
    revision: Option<u64>,
    profile: Option<PageProfile>,
    layout: Option<Arc<ScreenplayLayout>>,
}

impl LayoutCache {
    pub fn get_or_build(
        &mut self,
        revision: u64,
        project: &Project,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        if self.revision == Some(revision) && self.profile.as_ref() == Some(&project.page_profile) {
            return self
                .layout
                .clone()
                .ok_or(LayoutError {
                    kind: LayoutErrorKind::NoGeometry,
                })
                .map(|layout| (layout, true));
        }
        let layout = Arc::new(ScreenplayLayout::build(project)?);
        self.revision = Some(revision);
        self.profile = Some(project.page_profile.clone());
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
        project.validate()?;
        let profile = project.page_profile.clone();
        validate_profile(&profile)?;
        let line_advance = profile.line_advance;
        let char_width = profile.char_width;
        let font_family = profile.font_family.clone();
        let mut output = Self {
            profile,
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
                        .profile
                        .paper_width
                        .saturating_sub(output.profile.margin_left)
                        .saturating_sub(output.profile.margin_right)
                        / output.profile.char_width.max(1),
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
        let page_height = i64::from(self.profile.paper_height);
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
            occupied_lines: 0,
            next_line_y: self.profile.margin_top,
        }
    }

    fn make_line(&self, page: &LayoutPage, line: WrappedLine, title_page: bool) -> LayoutLine {
        let (x, width) = if title_page {
            (
                self.profile.margin_left,
                self.profile.paper_width - self.profile.margin_left - self.profile.margin_right,
            )
        } else {
            role_box(&self.profile, line.role)
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
                height: self.profile.line_advance,
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
        let has_boundary = self.pages.last().is_some_and(|page| !page.lines.is_empty());
        let previous_role = self
            .pages
            .last()
            .and_then(|page| page.lines.last())
            .map(|line| line.role);
        let extra_gap = previous_role
            .map(|previous| role_boundary_gap(&self.profile, previous, role))
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
                .saturating_add(self.profile.line_advance);
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
        || profile.line_advance == 0
        || profile
            .role_boxes
            .iter()
            .any(|box_| box_.width == 0 || box_.x.saturating_add(box_.width) > profile.paper_width)
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok(())
}

fn role_width(profile: &PageProfile, role: Role) -> u32 {
    let (_, width) = role_box(profile, role);
    profile
        .chars_per_line
        .min(width / profile.char_width.max(1))
        .max(1)
}

fn role_box(profile: &PageProfile, role: Role) -> (u32, u32) {
    let index = match role {
        Role::SceneHeading => 0,
        Role::Action => 1,
        Role::Character => 2,
        Role::Dialogue => 3,
        Role::Parenthetical => 4,
        Role::Transition => 5,
        Role::Note => 6,
    };
    let box_ = &profile.role_boxes[index];
    (box_.x, box_.width)
}

fn keep_with_next(role: Role) -> bool {
    matches!(
        role,
        Role::SceneHeading | Role::Character | Role::Parenthetical
    )
}

fn role_boundary_gap(profile: &PageProfile, _previous: Role, next: Role) -> u32 {
    if matches!(next, Role::Dialogue | Role::Parenthetical) {
        0
    } else {
        profile.boundary_gap
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
        let mut p = project(&[(Role::Action, "abcdefghijk"), (Role::Dialogue, "second")]);
        p.page_profile.char_width = 5;
        p.page_profile.line_advance = 15;
        p.page_profile.boundary_gap = 15;
        p.page_profile.role_boxes[1] = crate::RoleBox { x: 120, width: 50 };
        p.page_profile.role_boxes[3] = crate::RoleBox { x: 200, width: 40 };
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
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
    fn non_multiple_boundary_gap_is_preserved_as_point_geometry() {
        let mut p = project(&[(Role::Action, "first"), (Role::Action, "second")]);
        p.page_profile.line_advance = 14;
        p.page_profile.boundary_gap = 10;
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let lines = &layout.pages[0].lines;
        assert_eq!(lines[0].rect.y, 72);
        assert_eq!(lines[1].rect.y - lines[0].rect.y, 24);
        assert_eq!(
            lines[1].rect.y,
            lines[0].rect.y + p.page_profile.line_advance + p.page_profile.boundary_gap
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
                    p.page_profile.line_advance
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
                p.page_profile.line_advance + p.page_profile.boundary_gap
            );
        }
    }

    #[test]
    fn zero_gap_does_not_consume_page_capacity() {
        let mut p = project(&[(Role::Action, "before"), (Role::Dialogue, "after")]);
        p.page_profile.lines_per_page = 2;
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages.len(), 1);
        assert_eq!(layout.pages[0].occupied_lines, 2);
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
        let mut p = project(&[(Role::Action, "first"), (Role::Action, "second")]);
        p.page_profile.lines_per_page = 1;
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
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

        let page_height = i64::from(layout.profile.paper_height);
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
