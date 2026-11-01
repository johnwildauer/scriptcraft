//! Deterministic, backend-neutral screenplay layout.
//!
//! Layout uses explicit effective inputs and fixed integer metrics rather than
//! a platform font or viewport. The deterministic defaults therefore have stable structure on every host: one
//! character cell is seven units wide and one line is twelve units high.

use crate::{
    formatting::{
        formatting_for_role, EffectiveTextStyle, FixedPoint, FontFallback, ParagraphAlignment,
    },
    validate_layout_inputs, LayoutInputs, ModelError, PrintVisibility, Project, Role,
    SourceLayoutSettings, StableId,
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

/// Precise signed rectangle in hundredths of a point. `Rect` remains as the
/// integer compatibility projection; all layout and output consumers use this
/// representation and its single conversion rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRect {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub width: FixedPoint,
    pub height: FixedPoint,
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
    /// Display-only text; canonical `text` and source byte ranges are untouched.
    pub display_text: String,
    /// Source-character to display-character spans, relative to this line's
    /// display text and absolute within the canonical element text.
    pub display_source_map: Vec<DisplaySourceSegment>,
    pub text_style: EffectiveTextStyle,
    pub char_width: FixedPoint,
    /// Text extent after first-line indent and alignment.
    pub geometry: FixedRect,
    /// Full paragraph bounds, retained separately for hit testing and measure.
    pub paragraph_bounds: FixedRect,
    /// Rounded integer-point compatibility geometry for older callers.
    pub rect: Rect,
    pub keep_with_next: bool,
    pub oversized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPurpose {
    Editor,
    Print,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySourceSegment {
    pub display_start: usize,
    pub display_end: usize,
    pub source_start: usize,
    pub source_end: usize,
}

impl LayoutLine {
    /// Maps a canonical UTF-8 boundary to a display-character boundary. The
    /// optional trailing choice is used only for an offset inside an expanded
    /// source scalar; normal editing offsets are always scalar boundaries.
    pub fn display_char_for_source(&self, source_offset: usize, trailing: bool) -> usize {
        for segment in &self.display_source_map {
            if source_offset <= segment.source_start {
                return segment.display_start;
            }
            if source_offset < segment.source_end {
                return if trailing {
                    segment.display_end
                } else {
                    segment.display_start
                };
            }
            if source_offset == segment.source_end {
                return segment.display_end;
            }
        }
        self.display_text.chars().count()
    }

    /// Maps a display caret boundary to the nearest valid canonical boundary.
    /// Boundaries inside a Unicode uppercase expansion deterministically map
    /// before the source scalar; the boundary after its final cell maps after.
    pub fn source_for_display_char(&self, display_char: usize) -> usize {
        for segment in &self.display_source_map {
            if display_char <= segment.display_start {
                return segment.source_start;
            }
            if display_char < segment.display_end {
                return segment.source_start;
            }
            if display_char == segment.display_end {
                return segment.source_end;
            }
        }
        self.source.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPage {
    pub number: usize,
    pub title_page: bool,
    pub bounds: Rect,
    pub content_bounds: Rect,
    pub lines: Vec<LayoutLine>,
    pub occupied_lines: u32,
    next_line_y: FixedPoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayLayout {
    pub inputs: LayoutInputs,
    pub source_settings: SourceLayoutSettings,
    pub font_family: String,
    pub char_width: u32,
    pub line_height: u32,
    pub purpose: LayoutPurpose,
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
    purpose: Option<LayoutPurpose>,
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
        self.get_or_build_with_inputs_for_purpose(
            revision,
            project,
            inputs,
            LayoutPurpose::Editor,
        )
    }

    pub fn get_or_build_with_inputs_for_purpose(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        let source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings.clone())
            .unwrap_or_default();
        if self.revision == Some(revision)
            && self.inputs.as_ref() == Some(inputs)
            && self.source_settings.as_ref() == Some(&source_settings)
            && self.purpose == Some(purpose)
        {
            return self
                .layout
                .clone()
                .ok_or(LayoutError {
                    kind: LayoutErrorKind::NoGeometry,
                })
                .map(|layout| (layout, true));
        }
        let layout = Arc::new(ScreenplayLayout::build_with_inputs_for_purpose(
            project, inputs, purpose,
        )?);
        self.revision = Some(revision);
        self.inputs = Some(inputs.clone());
        self.source_settings = Some(source_settings);
        self.purpose = Some(purpose);
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
        Self::build_with_inputs_for_purpose(project, inputs, LayoutPurpose::Editor)
    }

    pub fn build_with_inputs_for_purpose(
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
    ) -> Result<Self, LayoutError> {
        let source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings.clone())
            .unwrap_or_default();
        Self::build_with_effective_inputs_for_purpose(project, inputs, &source_settings, purpose)
    }

    pub fn build_with_effective_inputs(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
    ) -> Result<Self, LayoutError> {
        Self::build_with_effective_inputs_for_purpose(
            project,
            inputs,
            source_settings,
            LayoutPurpose::Editor,
        )
    }

    pub fn build_with_effective_inputs_for_purpose(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        purpose: LayoutPurpose,
    ) -> Result<Self, LayoutError> {
        project.validate()?;
        validate_layout_inputs(inputs).map_err(|_| LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        })?;
        validate_profile(inputs)?;
        let char_width = inputs.char_width;
        let font_family = inputs.font_family.clone();
        let mut output = Self {
            inputs: inputs.clone(),
            source_settings: source_settings.clone(),
            font_family,
            char_width,
            line_height: inputs.line_advance,
            purpose,
            pages: Vec::new(),
        };
        if !project.title_page.is_empty() {
            let mut page = output.new_page(true);
            for (index, element) in project.title_page.iter().enumerate() {
                let paragraph = EffectiveParagraph {
                    left: FixedPoint::from_points(i64::from(output.inputs.margin_left)),
                    right: FixedPoint::from_points(i64::from(
                        output.inputs.paper_width - output.inputs.margin_right,
                    )),
                    first_indent: FixedPoint::default(),
                    alignment: ParagraphAlignment::Left,
                    space_before: FixedPoint::default(),
                    line_advance: FixedPoint::from_points(i64::from(output.inputs.line_advance)),
                    char_width: FixedPoint::from_points(i64::from(output.inputs.char_width)),
                };
                let style = plain_title_style(inputs);
                let lines = wrap_element(
                    &element.text,
                    index,
                    element.role,
                    paragraph,
                    style,
                    FixedPoint::from_points(i64::from(output.char_width)),
                    output.inputs.chars_per_line,
                );
                for line in lines {
                    let laid_out = output.make_line(&page, line, true, page.next_line_y);
                    page.lines.push(laid_out);
                    page.occupied_lines = page.occupied_lines.saturating_add(1);
                    page.next_line_y = FixedPoint::from_raw(
                        page.next_line_y
                            .raw()
                            .saturating_add(i64::from(output.inputs.line_advance) * FixedPoint::SCALE),
                    );
                }
            }
            output.pages.push(page);
        }
        if output.pages.last().is_some_and(|page| page.title_page) {
            output.pages.push(output.new_page(false));
        }
        for (index, element) in project.elements.iter().enumerate() {
            let text_style = output.source_settings.text_style(element.role);
            if purpose == LayoutPurpose::Print && text_style.hidden_text {
                continue;
            }
            if output
                .source_settings
                .starts_new_page_for_element(element.role, &element.id)
                && output
                    .pages
                    .last()
                    .is_some_and(|page| !page.title_page && !page.lines.is_empty())
            {
                output.pages.push(output.new_page(false));
            }
            let paragraph = effective_paragraph(
                &output.inputs,
                source_settings,
                element.role,
                &element.id,
                project.legacy_json_state.is_some(),
                &text_style,
            );
            let lines = wrap_element(
                element.text.as_str(),
                index,
                element.role,
                paragraph,
                text_style,
                paragraph.char_width,
                output.inputs.chars_per_line,
            );
            output.place_element(lines, element.role, paragraph);
        }
        if output.pages.is_empty() {
            output.pages.push(output.new_page(false));
        }
        Ok(output)
    }

    pub fn hit_test(&self, page: usize, x: u32, y: u32) -> Option<Caret> {
        self.hit_test_fixed(
            page,
            FixedPoint::from_points(i64::from(x)),
            FixedPoint::from_points(i64::from(y)),
        )
    }

    pub fn hit_test_fixed(&self, page: usize, x: FixedPoint, y: FixedPoint) -> Option<Caret> {
        let page = self.pages.get(page)?;
        let line = page.lines.iter().find(|line| {
            let paragraph_left = line.paragraph_bounds.x.raw();
            let paragraph_right = paragraph_left.saturating_add(line.paragraph_bounds.width.raw());
            let text_left = line.geometry.x.raw();
            let text_right = text_left.saturating_add(line.geometry.width.raw());
            x.raw() >= paragraph_left.min(text_left)
                && x.raw() <= paragraph_right.max(text_right)
                && y.raw() >= line.paragraph_bounds.y.raw()
                && y.raw()
                    < line
                        .paragraph_bounds
                        .y
                        .raw()
                        .saturating_add(line.paragraph_bounds.height.raw())
        })?;
        Some(self.caret_for_line(line, x))
    }

    /// Hit-test the body as one document-flow domain. `page` and `y` describe
    /// the pointer in page-local coordinates; the page height is used only to
    /// compare lines across pages. Horizontal placement is deliberately
    /// clamped to the selected line's source range.
    pub fn nearest_line_hit_test(&self, page: usize, x: i64, y: i64) -> Option<Caret> {
        if self.pages.get(page)?.title_page {
            return None;
        }
        self.nearest_line_hit_test_fixed(
            page,
            FixedPoint::from_points(x),
            FixedPoint::from_points(y),
        )
    }

    pub fn nearest_line_hit_test_fixed(
        &self,
        page: usize,
        x: FixedPoint,
        y: FixedPoint,
    ) -> Option<Caret> {
        if self.pages.get(page)?.title_page {
            return None;
        }
        let page_height = i64::from(self.inputs.paper_height).saturating_mul(FixedPoint::SCALE);
        let pointer_y = i64::try_from(page)
            .ok()?
            .saturating_mul(page_height)
            .saturating_add(y.raw());
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
                            .saturating_add(line.geometry.y.raw());
                        let distance = pointer_y.saturating_sub(line_y).saturating_abs();
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

    fn caret_for_line(&self, line: &LayoutLine, x: FixedPoint) -> Caret {
        let cell = line.char_width.raw().max(1);
        let units = x.raw().saturating_sub(line.geometry.x.raw()).max(0);
        let cells = (units.saturating_add(cell / 2) / cell) as usize;
        Caret {
            element: line.element,
            offset: line.source_for_display_char(cells),
        }
    }

    pub fn caret_rect(&self, caret: Caret) -> Result<Rect, LayoutError> {
        self.caret_geometry(caret).map(compat_rect)
    }

    pub fn caret_geometry(&self, caret: Caret) -> Result<FixedRect, LayoutError> {
        for page in &self.pages {
            for line in &page.lines {
                if line.element == caret.element
                    && caret.offset >= line.source.start
                    && caret.offset <= line.source.end
                    && is_boundary_in_line(line, caret.offset)
                {
                    let columns = line.display_char_for_source(caret.offset, false);
                    return Ok(FixedRect {
                        x: FixedPoint::from_raw(
                            line.geometry.x.raw().saturating_add(
                                (columns as i64).saturating_mul(line.char_width.raw()),
                            ),
                        ),
                        y: line.geometry.y,
                        width: FixedPoint::from_points(1),
                        height: line.geometry.height,
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
            next_line_y: FixedPoint::from_points(i64::from(self.inputs.margin_top)),
        }
    }

    fn make_line(
        &self,
        page: &LayoutPage,
        line: WrappedLine,
        title_page: bool,
        y: FixedPoint,
    ) -> LayoutLine {
        let indent = if line.first_visual_line {
            line.paragraph.first_indent
        } else {
            FixedPoint::default()
        };
        let line_left = FixedPoint::from_raw(line.paragraph.left.raw().saturating_add(indent.raw()));
        let line_width = FixedPoint::from_raw(line.paragraph.right.raw().saturating_sub(line_left.raw()));
        let text_width = FixedPoint::from_raw(
            (line.display_text.chars().count() as i64)
                .saturating_mul(line.char_width.raw()),
        );
        let spare = line_width.raw().saturating_sub(text_width.raw()).max(0);
        let text_x = match line.paragraph.alignment {
            ParagraphAlignment::Left => line_left,
            ParagraphAlignment::Right => FixedPoint::from_raw(
                line.paragraph.right.raw().saturating_sub(text_width.raw()),
            ),
            ParagraphAlignment::Center => {
                FixedPoint::from_raw(line_left.raw().saturating_add(spare / 2))
            }
        };
        let height = line.paragraph.line_advance;
        let paragraph_bounds = FixedRect {
            x: line.paragraph.left,
            y,
            width: FixedPoint::from_raw(
                line.paragraph.right.raw().saturating_sub(line.paragraph.left.raw()),
            ),
            height,
        };
        let geometry = FixedRect {
            x: text_x,
            y,
            width: text_width,
            height,
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
            display_text: line.display_text,
            display_source_map: line.display_source_map,
            text_style: line.text_style,
            char_width: line.char_width,
            geometry,
            paragraph_bounds,
            rect: compat_rect(paragraph_bounds),
            keep_with_next: !title_page && keep_with_next(line.role),
            oversized: line.oversized,
        }
    }

    fn place_element(
        &mut self,
        lines: Vec<WrappedLine>,
        role: Role,
        paragraph: EffectiveParagraph,
    ) {
        let keep = keep_with_next(role);
        let minimum = if keep { 2 } else { 1 };
        if self.pages.is_empty() {
            self.pages.push(self.new_page(false));
        }
        let capacity = self.inputs.lines_per_page as usize;
        let has_boundary = self.pages.last().is_some_and(|page| !page.lines.is_empty());
        let extra_gap = if has_boundary {
            paragraph.space_before.raw().max(0)
        } else {
            0
        };
        let content_bottom = FixedPoint::from_points(
            i64::from(self.inputs.paper_height.saturating_sub(self.inputs.margin_bottom)),
        );
        let minimum_height = paragraph
            .line_advance
            .raw()
            .saturating_mul(minimum as i64);
        let vertical_fit = self.pages.last().is_some_and(|page| {
            page.next_line_y
                .raw()
                .saturating_add(extra_gap)
                .saturating_add(minimum_height)
                <= content_bottom.raw()
        });
        if !lines.is_empty()
            && !self.pages.last().is_some_and(|page| page.title_page)
            && has_boundary
            && (self.pages.last().map_or(0, |page| page.occupied_lines as usize)
                .saturating_add(minimum)
                > capacity
                || !vertical_fit)
        {
            self.pages.push(self.new_page(false));
        }
        let applied_gap = if has_boundary && self.pages.last().is_some_and(|page| !page.lines.is_empty()) {
            if let Some(page) = self.pages.last_mut() {
                page.next_line_y = FixedPoint::from_raw(
                    page.next_line_y.raw().saturating_add(extra_gap),
                );
            }
            extra_gap
        } else {
            0
        };
        let total_height = paragraph
            .line_advance
            .raw()
            .saturating_mul(lines.len().min(i64::MAX as usize) as i64)
            .saturating_add(applied_gap);
        let content_height = content_bottom.raw()
            - FixedPoint::from_points(i64::from(self.inputs.margin_top)).raw();
        let oversized_element = lines.len() > capacity || total_height > content_height;
        for wrapped in lines {
            let current_fits = self.pages.last().is_some_and(|page| {
                (page.occupied_lines as usize) < capacity
                    && (page.lines.is_empty()
                        || page.next_line_y.raw().saturating_add(paragraph.line_advance.raw())
                            <= content_bottom.raw())
            });
            if !current_fits {
                self.pages.push(self.new_page(false));
            }
            let page_index = self.pages.len() - 1;
            if self.pages[page_index].lines.is_empty() {
                self.pages[page_index].occupied_lines = 0;
            }
            let page = self.pages[page_index].clone();
            let mut wrapped = wrapped;
            wrapped.oversized |= oversized_element;
            let line = self.make_line(&page, wrapped, false, page.next_line_y);
            self.pages[page_index].lines.push(line);
            self.pages[page_index].occupied_lines =
                self.pages[page_index].occupied_lines.saturating_add(1);
            self.pages[page_index].next_line_y = FixedPoint::from_raw(
                self.pages[page_index]
                    .next_line_y
                    .raw()
                    .saturating_add(paragraph.line_advance.raw()),
            );
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
    display_text: String,
    display_source_map: Vec<DisplaySourceSegment>,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    first_visual_line: bool,
    paragraph: EffectiveParagraph,
    oversized: bool,
}

#[derive(Debug, Clone, Copy)]
struct EffectiveParagraph {
    left: FixedPoint,
    right: FixedPoint,
    first_indent: FixedPoint,
    alignment: ParagraphAlignment,
    space_before: FixedPoint,
    line_advance: FixedPoint,
    char_width: FixedPoint,
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

fn effective_paragraph(
    inputs: &LayoutInputs,
    source: &SourceLayoutSettings,
    role: Role,
    element_id: &StableId,
    legacy_json_profile: bool,
    text_style: &EffectiveTextStyle,
) -> EffectiveParagraph {
    let native = formatting_for_role(role).paragraph;
    let fallback_left = FixedPoint::from_points(i64::from(inputs.margin_left));
    let fallback_right = FixedPoint::from_points(
        i64::from(inputs.paper_width.saturating_sub(inputs.margin_right)),
    );
    let default_left = FixedPoint::parse_inches(native.left_indent).unwrap_or(fallback_left);
    let default_right = FixedPoint::parse_inches(native.right_indent).unwrap_or(fallback_right);
    let projected = source.paragraph_spec(role);
    let direct = source.direct_paragraph_spec(element_id).paragraph;
    let mut left = projected.left_indent.unwrap_or(default_left);
    let mut right = projected.right_indent.unwrap_or(default_right);

    // Legacy JSON's explicit role boxes remain a transient compatibility input.
    // The reference-derived defaults, rather than those historical boxes, drive
    // all FDX sessions and ordinary new documents.
    let legacy_defaults = LayoutInputs::default();
    let role_box = inputs.role_box(role);
    let legacy_box = role_box.filter(|box_| {
        legacy_json_profile
            || legacy_defaults
                .role_box(role)
                .map_or(true, |default| *box_ != default)
    });
    if let Some(box_) = legacy_box {
        left = FixedPoint::from_points(i64::from(box_.x));
        right = FixedPoint::from_points(i64::from(box_.x.saturating_add(box_.width)));
    }

    let page_right = FixedPoint::from_points(i64::from(inputs.paper_width));
    if left.raw() < 0 || right <= left || right > page_right {
        left = default_left;
        right = default_right;
    }
    if left.raw() < 0 || right <= left || right > page_right {
        left = fallback_left;
        right = fallback_right;
    }

    // Direct bounds are a pair at the geometry-validation boundary. If their
    // combination is unusable, discard that direct pair and retain the already
    // resolved role/default pair rather than losing a valid role override.
    let direct_left = direct.left_indent.unwrap_or(left);
    let direct_right = direct.right_indent.unwrap_or(right);
    if direct_left.raw() >= 0 && direct_right > direct_left && direct_right <= page_right {
        left = direct_left;
        right = direct_right;
    }

    let default_indent = FixedPoint::parse_inches(native.first_indent).unwrap_or_default();
    let char_width = i64::from(inputs.char_width.max(1)).saturating_mul(FixedPoint::SCALE);
    let usable = right.raw().saturating_sub(left.raw());
    let indent_fits = |indent: FixedPoint| {
        left.raw().saturating_add(indent.raw()) >= 0
            && usable.saturating_sub(indent.raw()) >= char_width
    };
    let role_indent = projected.first_indent.unwrap_or(default_indent);
    let role_indent = if indent_fits(role_indent) {
        role_indent
    } else if indent_fits(default_indent) {
        default_indent
    } else {
        FixedPoint::default()
    };
    let first_indent = direct
        .first_indent
        .filter(|indent| indent_fits(*indent))
        .unwrap_or(role_indent);

    let default_alignment = match native.alignment {
        "Right" => ParagraphAlignment::Right,
        "Center" => ParagraphAlignment::Center,
        _ => ParagraphAlignment::Left,
    };
    let alignment = direct
        .alignment
        .or(projected.alignment)
        .unwrap_or(default_alignment);
    let default_space = FixedPoint::from_points(i64::from(native.space_before));
    let space_before = direct
        .space_before
        .or(projected.space_before)
        .unwrap_or(default_space);
    let scaled_metric = |base: u32| {
        let numerator = i64::from(base)
            .saturating_mul(i64::from(text_style.size))
            .saturating_mul(FixedPoint::SCALE);
        FixedPoint::from_raw(numerator.saturating_add(6).div_euclid(12).max(1))
    };
    EffectiveParagraph {
        left,
        right,
        first_indent,
        alignment,
        space_before,
        line_advance: scaled_metric(inputs.line_advance),
        char_width: scaled_metric(inputs.char_width),
    }
}

fn plain_title_style(inputs: &LayoutInputs) -> EffectiveTextStyle {
    EffectiveTextStyle {
        declared_family: inputs.font_family.clone(),
        family: FontFallback::CourierPrime,
        size: inputs.font_size.clamp(1, 72) as u8,
        bold: false,
        italic: false,
        underline: false,
        all_caps: false,
        hidden_text: false,
        source_color: None,
    }
}

fn compat_rect(rect: FixedRect) -> Rect {
    let rounded = |value: FixedPoint| value.rounded_points().clamp(0, i64::from(u32::MAX)) as u32;
    Rect {
        x: rounded(rect.x),
        y: rounded(rect.y),
        width: rounded(rect.width),
        height: rounded(rect.height),
    }
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

fn wrap_element(
    text: &str,
    element: usize,
    role: Role,
    paragraph: EffectiveParagraph,
    text_style: EffectiveTextStyle,
    char_width: FixedPoint,
    max_chars: u32,
) -> Vec<WrappedLine> {
    let cell = char_width.raw().max(1);
    let first_left = paragraph.left.raw().saturating_add(paragraph.first_indent.raw());
    let first_limit = paragraph
        .right
        .raw()
        .saturating_sub(first_left)
        .div_euclid(cell)
        .min(max_chars.max(1) as i64)
        .max(1) as usize;
    let rest_limit = paragraph
        .right
        .raw()
        .saturating_sub(paragraph.left.raw())
        .div_euclid(cell)
        .min(max_chars.max(1) as i64)
        .max(1) as usize;
    let mut output = Vec::new();
    let mut first_visual_line = true;
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
            first_limit,
            rest_limit,
            paragraph,
            &text_style,
            char_width,
            &mut first_visual_line,
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
        first_limit,
        rest_limit,
        paragraph,
        &text_style,
        char_width,
        &mut first_visual_line,
        &mut output,
    );
    if output.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: 0,
            end: 0,
            text: String::new(),
            display_text: String::new(),
            display_source_map: Vec::new(),
            text_style,
            char_width,
            first_visual_line: true,
            paragraph,
            oversized: false,
        });
    }
    output
}

#[derive(Debug, Clone, Copy)]
struct DisplayCell {
    character: char,
    local_start: usize,
    local_end: usize,
    source_start: usize,
    source_end: usize,
}

fn wrap_segment(
    text: &str,
    base: usize,
    element: usize,
    role: Role,
    first_limit: usize,
    rest_limit: usize,
    paragraph: EffectiveParagraph,
    text_style: &EffectiveTextStyle,
    char_width: FixedPoint,
    first_visual_line: &mut bool,
    output: &mut Vec<WrappedLine>,
) {
    if text.is_empty() {
        output.push(WrappedLine {
            element,
            role,
            start: base,
            end: base,
            text: String::new(),
            display_text: String::new(),
            display_source_map: Vec::new(),
            text_style: text_style.clone(),
            char_width,
            first_visual_line: *first_visual_line,
            paragraph,
            oversized: false,
        });
        *first_visual_line = false;
        return;
    }
    let mut cells = Vec::new();
    for (local_start, source_character) in text.char_indices() {
        let local_end = local_start + source_character.len_utf8();
        let source_start = base + local_start;
        let source_end = base + local_end;
        if text_style.all_caps {
            cells.extend(source_character.to_uppercase().map(|character| DisplayCell {
                character,
                local_start,
                local_end,
                source_start,
                source_end,
            }));
        } else {
            cells.push(DisplayCell {
                character: source_character,
                local_start,
                local_end,
                source_start,
                source_end,
            });
        }
    }
    let mut start = 0;
    while start < cells.len() && output.len() < MAX_LINES_PER_ELEMENT {
        let first_line = *first_visual_line;
        let limit = if first_line { first_limit } else { rest_limit };
        let mut hard_end = (start + limit).min(cells.len());
        while hard_end < cells.len()
            && hard_end > start
            && cells[hard_end - 1].source_start == cells[hard_end].source_start
        {
            hard_end += 1;
        }
        let mut end = hard_end;
        if hard_end < cells.len() {
            if let Some(space) = (start..hard_end)
                .rev()
                .find(|index| cells[*index].character.is_whitespace())
            {
                end = space + 1;
            }
        }
        if end <= start {
            end = (start + 1).min(cells.len());
        }
        let selected = &cells[start..end];
        let first_cell = selected.first().copied().unwrap_or(DisplayCell {
            character: '\0',
            local_start: 0,
            local_end: 0,
            source_start: base,
            source_end: base,
        });
        let last_cell = selected.last().copied().unwrap_or(first_cell);
        let source_start = first_cell.source_start;
        let source_end = last_cell.source_end;
        let mut display_text = String::new();
        let mut display_source_map: Vec<DisplaySourceSegment> = Vec::new();
        for (display_index, cell) in selected.iter().enumerate() {
            display_text.push(cell.character);
            if let Some(previous) = display_source_map.last_mut() {
                if previous.source_start == cell.source_start
                    && previous.source_end == cell.source_end
                {
                    previous.display_end = display_index + 1;
                    continue;
                }
            }
            display_source_map.push(DisplaySourceSegment {
                display_start: display_index,
                display_end: display_index + 1,
                source_start: cell.source_start,
                source_end: cell.source_end,
            });
        }
        output.push(WrappedLine {
            element,
            role,
            start: source_start,
            end: source_end,
            text: text[first_cell.local_start..last_cell.local_end].to_owned(),
            display_text,
            display_source_map,
            text_style: text_style.clone(),
            char_width,
            first_visual_line: first_line,
            paragraph,
            oversized: selected.len() > limit,
        });
        *first_visual_line = false;
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
        assert_eq!(page.lines[0].rect.x, 252);
        assert!(page
            .lines
            .iter()
            .all(|line| line.rect.x + line.rect.width <= 540));
    }

    #[test]
    fn native_role_paragraph_bounds_use_fractional_reference_geometry() {
        let p = project(&[
            (Role::SceneHeading, "scene"),
            (Role::Action, "action"),
            (Role::Note, "note"),
            (Role::Character, "BOB"),
            (Role::Parenthetical, "(quiet)"),
            (Role::Dialogue, "hello"),
            (Role::Transition, "CUT TO:"),
        ]);
        let mut visible = SourceLayoutSettings::default();
        visible.role_print_visibility[crate::formatting::role_index(Role::Note)] =
            Some(PrintVisibility::Visible);
        let layout = ScreenplayLayout::build_with_effective_inputs(
            &p,
            &LayoutInputs::default(),
            &visible,
        )
        .unwrap_or_else(|_| unreachable!());
        let boxes: Vec<_> = layout.pages.iter().flat_map(|p| &p.lines).collect();
        assert_eq!((boxes[0].rect.x, boxes[0].rect.width), (108, 432));
        assert_eq!((boxes[1].rect.x, boxes[1].rect.width), (108, 432));
        assert_eq!((boxes[2].rect.x, boxes[2].rect.width), (117, 423));
        assert_eq!((boxes[3].rect.x, boxes[3].rect.width), (252, 270));
        assert_eq!((boxes[4].rect.x, boxes[4].rect.width), (216, 180));
        assert_eq!((boxes[5].rect.x, boxes[5].rect.width), (180, 252));
        assert_eq!((boxes[6].rect.x, boxes[6].rect.width), (396, 115));
    }

    #[test]
    fn every_role_uses_its_typed_native_paragraph_measure() {
        let roles_and_text = Role::ALL.map(|role| (role, "x"));
        let p = project(&roles_and_text);
        let mut all_visible = SourceLayoutSettings::default();
        for role in Role::ALL {
            all_visible.role_print_visibility[crate::formatting::role_index(role)] =
                Some(PrintVisibility::Visible);
        }
        let layout = ScreenplayLayout::build_with_effective_inputs(
            &p,
            &LayoutInputs::default(),
            &all_visible,
        )
        .unwrap_or_else(|_| unreachable!());
        for role in Role::ALL {
            let line = layout
                .pages
                .iter()
                .flat_map(|page| &page.lines)
                .find(|line| line.role == role)
                .unwrap_or_else(|| unreachable!());
            let spec = formatting_for_role(role).paragraph;
            let left = FixedPoint::parse_inches(spec.left_indent).unwrap_or_default();
            let right = FixedPoint::parse_inches(spec.right_indent).unwrap_or_default();
            assert_eq!(line.paragraph_bounds.x, left, "{role:?} left bound");
            assert_eq!(line.paragraph_bounds.width.raw(), right.raw() - left.raw());
        }
    }

    #[test]
    fn alignment_first_indent_and_space_before_use_precise_text_geometry() {
        let p = project(&[
            (Role::Transition, "CUT"),
            (Role::NewAct, "ACT"),
            (Role::Parenthetical, "abcdefghijklmnopqrstuvwxyz1234"),
        ]);
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let transition = layout
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .find(|line| line.role == Role::Transition)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(transition.geometry.x.raw(), 51_120 - 3 * 7 * 100);
        assert_eq!(transition.source.start, 0);

        let new_act = layout
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .find(|line| line.role == Role::NewAct)
            .unwrap_or_else(|| unreachable!());
        let center_left = FixedPoint::from_points(72).raw();
        let center_right = FixedPoint::from_points(540).raw();
        let text_width = 3 * 7 * 100;
        assert_eq!(
            new_act.geometry.x.raw(),
            center_left + (center_right - center_left - text_width) / 2
        );
        assert_eq!(new_act.page, 1);
        assert_eq!(new_act.geometry.y, FixedPoint::from_points(72));

        let parenthetical = layout
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .filter(|line| line.role == Role::Parenthetical)
            .collect::<Vec<_>>();
        assert_eq!(parenthetical.len(), 2);
        assert_eq!(parenthetical[0].geometry.x.raw(), 20_880);
        assert_eq!(parenthetical[1].geometry.x.raw(), 21_600);
        assert_eq!(parenthetical[0].source.start, 0);
        assert_eq!(parenthetical[1].source.end, p.elements[2].text.len());
    }

    #[test]
    fn empty_aligned_paragraphs_have_stable_caret_anchors() {
        let p = project(&[(Role::NewAct, ""), (Role::Transition, "")]);
        let layout = ScreenplayLayout::build(&p).unwrap_or_else(|_| unreachable!());
        let center = layout.pages[0].lines[0].geometry.x;
        assert_eq!(center, FixedPoint::from_points(306));
        let center_caret = layout
            .caret_geometry(Caret {
                element: 0,
                offset: 0,
            })
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(center_caret.x, center);

        let transition = layout
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .find(|line| line.role == Role::Transition)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(transition.geometry.x.raw(), 51_120);
        let right_caret = layout
            .caret_geometry(Caret {
                element: transition.element,
                offset: 0,
            })
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(right_caret.x, transition.geometry.x);
    }

    #[test]
    fn incoming_role_space_before_is_used_and_suppressed_on_new_page() {
        for role in Role::ALL {
            let p = project(&[(Role::General, "before"), (role, "after")]);
            let mut all_visible = SourceLayoutSettings::default();
            for visible_role in Role::ALL {
                all_visible.role_print_visibility[crate::formatting::role_index(visible_role)] =
                    Some(PrintVisibility::Visible);
            }
            let layout = ScreenplayLayout::build_with_effective_inputs(
                &p,
                &LayoutInputs::default(),
                &all_visible,
            )
            .unwrap_or_else(|_| unreachable!());
            let lines = layout
                .pages
                .iter()
                .flat_map(|page| &page.lines)
                .collect::<Vec<_>>();
            let incoming = lines
                .iter()
                .find(|line| line.element == 1)
                .unwrap_or_else(|| unreachable!());
            if formatting_for_role(role).paragraph.starts_new_page {
                assert_eq!(incoming.page, 1);
                assert_eq!(incoming.geometry.y, FixedPoint::from_points(72));
            } else {
                let previous = lines
                    .iter()
                    .find(|line| line.element == 0)
                    .unwrap_or_else(|| unreachable!());
                assert_eq!(incoming.page, previous.page);
                assert_eq!(
                    incoming.geometry.y.raw() - previous.geometry.y.raw(),
                    (12 + i64::from(formatting_for_role(role).paragraph.space_before))
                        * FixedPoint::SCALE
                );
            }
        }
    }

    #[test]
    fn imported_page_start_yes_and_no_override_role_defaults() {
        let imported = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec StartsNewPage='Yes'/></ElementSettings><ElementSettings Type='New Act'><ParagraphSpec StartsNewPage='No'/></ElementSettings><Content><Paragraph Type='General' id='g'><Text>before</Text></Paragraph><Paragraph Type='Action' id='a'><Text>forced</Text></Paragraph><Paragraph Type='New Act' id='n'><Text>continued</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let layout = ScreenplayLayout::build(&imported.project).unwrap_or_else(|_| unreachable!());
        let action = layout
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .find(|line| line.role == Role::Action)
            .unwrap_or_else(|| unreachable!());
        let new_act = layout
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .find(|line| line.role == Role::NewAct)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(action.page, 1);
        assert_eq!(action.geometry.y, FixedPoint::from_points(72));
        assert_eq!(new_act.page, action.page);
        assert_eq!(new_act.geometry.y.raw() - action.geometry.y.raw(), 24 * FixedPoint::SCALE);
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
        let mut all_roles_visible = SourceLayoutSettings::default();
        for role in Role::ALL {
            let index = Role::ALL
                .iter()
                .position(|candidate| *candidate == role)
                .unwrap_or_else(|| unreachable!());
            all_roles_visible.role_print_visibility[index] = Some(PrintVisibility::Visible);
        }

        let layout = ScreenplayLayout::build_with_effective_inputs(
            &p,
            &inputs,
            &all_roles_visible,
        )
        .unwrap_or_else(|_| unreachable!());
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
            .get_or_build_with_inputs_for_purpose(
                4,
                &visible.project,
                &inputs,
                LayoutPurpose::Editor,
            )
            .unwrap_or_else(|_| unreachable!());
        assert!(!hit);
        assert_eq!(visible_layout.pages[0].lines.len(), 1);

        let (hidden_layout, hit) = cache
            .get_or_build_with_inputs_for_purpose(
                4,
                &hidden.project,
                &inputs,
                LayoutPurpose::Editor,
            )
            .unwrap_or_else(|_| unreachable!());
        assert!(!hit);
        assert_eq!(hidden_layout.pages[0].lines.len(), 1);
        assert!(hidden_layout.pages[0].lines[0].text_style.hidden_text);
        let (print_layout, print_hit) = cache
            .get_or_build_with_inputs_for_purpose(
                4,
                &hidden.project,
                &inputs,
                LayoutPurpose::Print,
            )
            .unwrap_or_else(|_| unreachable!());
        assert!(!print_hit);
        assert!(print_layout.pages[0].lines.is_empty());
    }

    #[test]
    fn cache_key_tracks_imported_paragraph_geometry_projection() {
        let wider = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec LeftIndent='1.50' RightIndent='7.50'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>same</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let indented = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec LeftIndent='0.88' RightIndent='7.50'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>same</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(wider.project.elements, indented.project.elements);

        let mut cache = LayoutCache::default();
        let inputs = LayoutInputs::default();
        let (first, first_hit) = cache
            .get_or_build_with_inputs(9, &wider.project, &inputs)
            .unwrap_or_else(|_| unreachable!());
        let (second, second_hit) = cache
            .get_or_build_with_inputs(9, &indented.project, &inputs)
            .unwrap_or_else(|_| unreachable!());
        assert!(!first_hit);
        assert!(!second_hit);
        assert_ne!(first.pages[0].lines[0].geometry.x, second.pages[0].lines[0].geometry.x);
        assert_eq!(second.pages[0].lines[0].geometry.x.raw(), 6_336);
    }

    #[test]
    fn invalid_imported_bound_pair_falls_back_to_both_native_bounds() {
        let imported = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec LeftIndent='5.00' RightIndent='4.00' FirstIndent='NaN'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>same</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let layout = ScreenplayLayout::build(&imported.project).unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages[0].lines[0].paragraph_bounds.x.raw(), 10_800);
        assert_eq!(layout.pages[0].lines[0].paragraph_bounds.width.raw(), 43_200);
        assert_eq!(
            imported
                .project
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .source_layout_settings()
                .paragraph_spec(Role::Action)
                .first_indent,
            None
        );
    }

    #[test]
    fn imported_alignment_fractional_indent_and_decimal_spacing_are_effective() {
        let imported = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec Alignment='Center' FirstIndent='-0.10' LeftIndent='0.88' RightIndent='7.50' SpaceBefore='14.25' Leading='Regular' Spacing='1'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>a</Text></Paragraph><Paragraph Type='Action' id='b'><Text>b</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let canonical = imported.project.elements[0].text.clone();
        let layout = ScreenplayLayout::build(&imported.project).unwrap_or_else(|_| unreachable!());
        let first = &layout.pages[0].lines[0];
        let second = &layout.pages[0].lines[1];
        assert_eq!(first.text, canonical);
        assert_eq!(first.source.start, 0);
        assert_eq!(first.source.end, 1);
        assert_eq!(first.paragraph_bounds.x.raw(), 6_336);
        assert_eq!(first.geometry.x.raw(), 29_458);
        assert_eq!(second.geometry.y.raw() - first.geometry.y.raw(), 2_625);
        assert_eq!(
            imported.project.preservation.as_ref().unwrap_or_else(|| unreachable!())
                .source.original_bytes,
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec Alignment='Center' FirstIndent='-0.10' LeftIndent='0.88' RightIndent='7.50' SpaceBefore='14.25' Leading='Regular' Spacing='1'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>a</Text></Paragraph><Paragraph Type='Action' id='b'><Text>b</Text></Paragraph></Content></FinalDraft>"
        );
    }

    #[test]
    fn native_role_space_before_is_preserved_as_point_geometry() {
        let p = project(&[(Role::Action, "first"), (Role::Action, "second")]);
        let inputs = LayoutInputs {
            line_advance: 14,
            ..LayoutInputs::default()
        };
        let layout =
            ScreenplayLayout::build_with_inputs(&p, &inputs).unwrap_or_else(|_| unreachable!());
        let lines = &layout.pages[0].lines;
        assert_eq!(lines[0].rect.y, 72);
        assert_eq!(lines[1].rect.y - lines[0].rect.y, 26);
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
    fn native_dialogue_and_parenthetical_space_before_is_zero() {
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
                let mut visible = SourceLayoutSettings::default();
                visible.role_print_visibility[crate::formatting::role_index(Role::Note)] =
                    Some(PrintVisibility::Visible);
                let layout = ScreenplayLayout::build_with_effective_inputs(
                    &p,
                    &LayoutInputs::default(),
                    &visible,
                )
                .unwrap_or_else(|_| unreachable!());
                let lines = &layout.pages[0].lines;
                assert_eq!(
                    lines[1].rect.y - lines[0].rect.y,
                    layout.inputs.line_advance
                );
            }
        }
    }

    #[test]
    fn other_incoming_roles_use_their_native_space_before() {
        let pairs = [
            (Role::Action, Role::Action),
            (Role::Dialogue, Role::Action),
            (Role::Parenthetical, Role::Character),
            (Role::Note, Role::SceneHeading),
        ];
        for (previous, next) in pairs {
            let p = project(&[(previous, "before"), (next, "after")]);
            let mut visible = SourceLayoutSettings::default();
            visible.role_print_visibility[crate::formatting::role_index(Role::Note)] =
                Some(PrintVisibility::Visible);
            let layout = ScreenplayLayout::build_with_effective_inputs(
                &p,
                &LayoutInputs::default(),
                &visible,
            )
            .unwrap_or_else(|_| unreachable!());
            let lines = &layout.pages[0].lines;
            let expected_space = match next {
                Role::SceneHeading | Role::Sequence | Role::EndOfAct => 24,
                Role::Action | Role::Character | Role::Shot | Role::Transition => 12,
                _ => 0,
            };
            assert_eq!(
                lines[1].rect.y - lines[0].rect.y,
                layout.inputs.line_advance + expected_space
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
        assert_eq!(layout.pages.len(), 3);
        assert!(layout.pages.iter().all(|page| page.occupied_lines <= 55));
        assert_eq!(layout.pages[0].lines[0].rect.y, 72);
        assert!(layout.pages[0]
            .lines
            .last()
            .is_some_and(|line| line.rect.y <= 792 - 72));
        assert_eq!(layout.pages[1].lines[0].rect.y, 72);
        assert_eq!(layout.pages[1].lines[0].page, 1);
        assert_eq!(layout.pages[2].lines[0].rect.y, 72);
        assert_eq!(layout.pages[2].lines[0].page, 2);
    }

    #[test]
    fn pagination_uses_fixed_vertical_fit_in_addition_to_logical_capacity() {
        let p = project(&[
            (Role::Action, "one"),
            (Role::Action, "two"),
            (Role::Action, "three"),
            (Role::Action, "four"),
            (Role::Action, "five"),
        ]);
        let inputs = LayoutInputs {
            paper_height: 80,
            margin_top: 10,
            margin_bottom: 10,
            line_advance: 15,
            lines_per_page: 55,
            ..LayoutInputs::default()
        };
        let layout = ScreenplayLayout::build_with_inputs(&p, &inputs)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(layout.pages.len(), 3);
        let page_bottom = FixedPoint::from_points(70).raw();
        for page in &layout.pages {
            assert!(!page.lines.is_empty());
            assert!(page.lines.iter().all(|line| {
                line.geometry.y.raw() + line.geometry.height.raw() <= page_bottom
            }));
        }
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

        let layout = ScreenplayLayout::build_with_effective_inputs_for_purpose(
            &p,
            &LayoutInputs::default(),
            &settings,
            LayoutPurpose::Print,
        )
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

    #[test]
    fn imported_fontspec_projects_supported_tokens_and_maps_unicode_allcaps() {
        let imported = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Scene Heading'><FontSpec Font='Arial' Size='16' Style='Bold+Italic+Underline+AllCaps+FutureToken' Color='unmapped'/></ElementSettings><Content><Paragraph Type='Scene Heading' id='heading'><Text>Stra\xc3\x9fe</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let canonical = imported.project.elements[0].text.clone();
        let inputs = LayoutInputs {
            chars_per_line: 5,
            ..LayoutInputs::default()
        };
        let layout = ScreenplayLayout::build_with_inputs(&imported.project, &inputs)
            .unwrap_or_else(|_| unreachable!());
        let lines = layout.pages.iter().flat_map(|page| &page.lines).collect::<Vec<_>>();
        assert_eq!(canonical, "Stra\u{00df}e");
        assert_eq!(
            lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join(""),
            canonical
        );
        assert_eq!(
            lines.iter().map(|line| line.display_text.as_str()).collect::<Vec<_>>().join(""),
            "STRASSE"
        );
        let styled = lines[0];
        assert_eq!(styled.text_style.declared_family, "Arial");
        assert_eq!(styled.text_style.family, FontFallback::CourierPrime);
        assert_eq!(styled.text_style.size, 16);
        assert!(styled.text_style.bold && styled.text_style.italic && styled.text_style.underline);
        assert!(styled.text_style.all_caps);
        assert!(!styled.text_style.hidden_text);
        assert_eq!(styled.geometry.height, FixedPoint::from_points(16));
        let expansion = styled
            .display_source_map
            .iter()
            .find(|segment| segment.source_start == 4)
            .unwrap_or_else(|| unreachable!());
        assert_eq!((expansion.display_start, expansion.display_end), (4, 6));
        assert_eq!((expansion.source_start, expansion.source_end), (4, 6));
        assert_eq!(styled.source_for_display_char(5), 4);
        assert_eq!(styled.source_for_display_char(6), 6);
        assert_eq!(styled.display_char_for_source(6, false), 6);
        let expansion_middle_x = FixedPoint::from_raw(
            styled.geometry.x.raw()
                + 5 * styled.char_width.raw()
                + styled.char_width.raw() / 4,
        );
        let middle_hit = layout
            .hit_test_fixed(0, expansion_middle_x, styled.geometry.y)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(middle_hit.offset, 4);
        let after_expansion_caret = layout
            .caret_geometry(Caret {
                element: styled.element,
                offset: 6,
            })
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(
            after_expansion_caret.x.raw(),
            styled.geometry.x.raw() + 6 * styled.char_width.raw()
        );
        assert_eq!(
            lines
                .iter()
                .flat_map(|line| &line.display_source_map)
                .filter(|segment| segment.source_start == 4 && segment.source_end == 6)
                .count(),
            1
        );
        assert_eq!(
            imported.project.preservation.as_ref().unwrap_or_else(|| unreachable!())
                .source.original_bytes,
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Scene Heading'><FontSpec Font='Arial' Size='16' Style='Bold+Italic+Underline+AllCaps+FutureToken' Color='unmapped'/></ElementSettings><Content><Paragraph Type='Scene Heading' id='heading'><Text>Stra\xc3\x9fe</Text></Paragraph></Content></FinalDraft>"
        );
    }

    #[test]
    fn unsupported_font_values_fallback_per_field_and_duplicate_specs_are_ambiguous() {
        let unsupported = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><FontSpec Font='Uninstalled Family' Size='0' Style='Bold+UnknownToken'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>x</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let unsupported_layout = ScreenplayLayout::build(&unsupported.project)
            .unwrap_or_else(|_| unreachable!());
        let unsupported_line = &unsupported_layout.pages[0].lines[0];
        assert_eq!(unsupported_line.text_style.declared_family, "Uninstalled Family");
        assert_eq!(unsupported_line.text_style.size, 12);
        assert!(unsupported_line.text_style.bold);
        assert!(!unsupported_line.text_style.italic);

        let ambiguous = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><FontSpec Size='16' Style='Bold'/><FontSpec Size='20' Style='Italic'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>x</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let ambiguous_layout = ScreenplayLayout::build(&ambiguous.project)
            .unwrap_or_else(|_| unreachable!());
        let ambiguous_line = &ambiguous_layout.pages[0].lines[0];
        assert_eq!(ambiguous_line.text_style.size, 12);
        assert!(!ambiguous_line.text_style.bold && !ambiguous_line.text_style.italic);
    }
}
