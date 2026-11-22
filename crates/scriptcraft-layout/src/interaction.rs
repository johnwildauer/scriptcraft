use super::{
    cache::add_elapsed_nanos,
    geometry::{
        Caret, FixedRect, LayoutError, LayoutErrorKind, LayoutLine, LocatedLayoutLine, Rect,
        ScreenplayLayout, SourceRange,
    },
    inputs::LayoutComposition,
    paginate, LayoutBuildDiagnostics,
};
use scriptcraft_model::formatting::FixedPoint;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LineLocation {
    page: usize,
    line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BodyLineIndexEntry {
    location: LineLocation,
    absolute_y: i64,
    absolute_bottom_y: i64,
    element: usize,
    source: SourceRange,
}

/// Value-free work count for one indexed interaction query. It counts index
/// entries actually inspected, including binary-search probes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InteractionQueryDiagnostics {
    pub index_entries_examined: u64,
    pub index_queries: u64,
    pub equal_y_candidates_examined: u64,
}

impl InteractionQueryDiagnostics {
    fn query(&mut self) {
        self.index_queries = self.index_queries.saturating_add(1);
    }

    fn examined(&mut self) {
        self.index_entries_examined = self.index_entries_examined.saturating_add(1);
    }

    fn equal_y_candidate(&mut self) {
        self.equal_y_candidates_examined = self.equal_y_candidates_examined.saturating_add(1);
        self.examined();
    }
}

impl LocatedLayoutLine<'_> {
    /// Applies the canonical span-boundary x-to-source mapping to this already
    /// located line. Midpoint ties deterministically select the trailing side.
    pub fn fixed_caret_for_x(&self, x: FixedPoint) -> Caret {
        let display_char = display_char_for_x(self.line, x);
        Caret {
            element: self.line.element,
            offset: self.line.source_for_display_char(display_char),
        }
    }

    /// Applies the canonical span-boundary source-to-x mapping to a caret owned by
    /// this line. This does not perform another line-ownership query.
    pub fn fixed_caret_geometry(&self, caret: &Caret) -> Result<FixedRect, LayoutError> {
        if caret.element != self.line.element
            || caret.offset < self.line.source.start
            || caret.offset > self.line.source.end
            || !is_boundary_in_line(self.line, caret.offset)
        {
            return Err(LayoutError {
                kind: LayoutErrorKind::InvalidPosition,
            });
        }
        let display_char = self.line.display_char_for_source(caret.offset, false);
        Ok(FixedRect {
            x: FixedPoint::from_raw(x_for_display_char(self.line, display_char)),
            y: self.line.geometry.y,
            width: FixedPoint::from_points(1),
            height: self.line.geometry.height,
        })
    }
}

impl LayoutLine {
    /// Maps a canonical UTF-8 boundary to a display-character boundary. The
    /// optional trailing choice is used only for an offset inside an expanded
    /// source scalar; normal editing offsets are always scalar boundaries.
    pub fn display_char_for_source(&self, source_offset: usize, trailing: bool) -> usize {
        for segment in self.display_source_map.iter() {
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
        for segment in self.display_source_map.iter() {
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

impl ScreenplayLayout {
    pub fn hit_test(&self, page: usize, x: u32, y: u32) -> Option<Caret> {
        self.hit_test_fixed(
            page,
            FixedPoint::from_points(i64::from(x)),
            FixedPoint::from_points(i64::from(y)),
        )
    }

    pub fn hit_test_fixed(&self, page: usize, x: FixedPoint, y: FixedPoint) -> Option<Caret> {
        let page = self.pages.get(page)?;
        if page.title_page && self.composition != LayoutComposition::TitlePageEditor {
            return None;
        }
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
        if self.pages.get(page)?.title_page
            && self.composition != LayoutComposition::TitlePageEditor
        {
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
        self.nearest_line_hit_test_fixed_with_diagnostics(
            page,
            x,
            y,
            &mut InteractionQueryDiagnostics::default(),
        )
    }

    pub fn nearest_line_hit_test_fixed_with_diagnostics(
        &self,
        page: usize,
        x: FixedPoint,
        y: FixedPoint,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<Caret> {
        self.nearest_line_location_fixed_with_diagnostics(page, x, y, diagnostics)
            .map(|location| location.fixed_caret_for_x(x))
    }

    /// Selects the same nearest core line as fixed hit testing without mapping
    /// the pointer's x coordinate to a caret.
    pub fn nearest_line_location_fixed_with_diagnostics(
        &self,
        page: usize,
        x: FixedPoint,
        y: FixedPoint,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<LocatedLayoutLine<'_>> {
        diagnostics.query();
        let horizontal_distance = |line: &LayoutLine| {
            let left = line.paragraph_bounds.x.raw();
            let right = left.saturating_add(line.paragraph_bounds.width.raw());
            if x.raw() < left {
                left.saturating_sub(x.raw())
            } else if x.raw() > right {
                x.raw().saturating_sub(right)
            } else {
                0
            }
        };
        if self.pages.get(page)?.title_page {
            if self.composition != LayoutComposition::TitlePageEditor {
                return None;
            }
            let page_height = i64::from(self.inputs.paper_height).saturating_mul(FixedPoint::SCALE);
            let pointer_y = i64::try_from(page)
                .ok()?
                .saturating_mul(page_height)
                .saturating_add(y.raw());
            let entry = self
                .title_line_index
                .iter()
                .enumerate()
                .min_by_key(|(index, entry)| {
                    diagnostics.examined();
                    (
                        interval_distance(
                            pointer_y,
                            entry.absolute_y,
                            entry.absolute_bottom_y,
                        ),
                        horizontal_distance(
                            &self.pages[entry.location.page].lines[entry.location.line],
                        ),
                        *index,
                    )
                })?
                .1;
            let line = &self.pages[entry.location.page].lines[entry.location.line];
            return Some(LocatedLayoutLine {
                page_index: entry.location.page,
                line,
            });
        }
        let page_height = i64::from(self.inputs.paper_height).saturating_mul(FixedPoint::SCALE);
        let pointer_y = i64::try_from(page)
            .ok()?
            .saturating_mul(page_height)
            .saturating_add(y.raw());
        let insertion = self.body_line_y_order.partition_point(|index| {
            diagnostics.examined();
            self.body_line_index[*index].absolute_y < pointer_y
        });
        let neighbor_coordinates = [
            insertion.checked_sub(1),
            (insertion < self.body_line_y_order.len()).then_some(insertion),
        ];
        let mut best: Option<(i64, i64, usize)> = None;
        for coordinate_index in neighbor_coordinates.into_iter().flatten() {
            diagnostics.examined();
            let y = self.body_line_index[self.body_line_y_order[coordinate_index]].absolute_y;
            let start = self.body_line_y_order.partition_point(|index| {
                diagnostics.examined();
                self.body_line_index[*index].absolute_y < y
            });
            let end = self.body_line_y_order.partition_point(|index| {
                diagnostics.examined();
                self.body_line_index[*index].absolute_y <= y
            });
            for coordinate_index in start..end {
                let index = self.body_line_y_order[coordinate_index];
                diagnostics.equal_y_candidate();
                let entry = self.body_line_index[index];
                let line = &self.pages[entry.location.page].lines[entry.location.line];
                let candidate = (
                    interval_distance(pointer_y, entry.absolute_y, entry.absolute_bottom_y),
                    horizontal_distance(line),
                    index,
                );
                if best.is_none_or(|current| candidate < current) {
                    best = Some(candidate);
                }
            }
        }
        let (_, _, best_index) = best?;
        let entry = self.body_line_index[best_index];
        Some(LocatedLayoutLine {
            page_index: entry.location.page,
            line: &self.pages[entry.location.page].lines[entry.location.line],
        })
    }

    fn caret_for_line(&self, line: &LayoutLine, x: FixedPoint) -> Caret {
        let cells = display_char_for_x(line, x);
        Caret {
            element: line.element,
            offset: line.source_for_display_char(cells),
        }
    }

    pub fn caret_rect(&self, caret: Caret) -> Result<Rect, LayoutError> {
        self.caret_geometry(caret).map(paginate::compat_rect)
    }

    pub fn caret_geometry(&self, caret: Caret) -> Result<FixedRect, LayoutError> {
        self.caret_geometry_with_diagnostics(caret, &mut InteractionQueryDiagnostics::default())
    }

    pub fn caret_geometry_with_diagnostics(
        &self,
        caret: Caret,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Result<FixedRect, LayoutError> {
        self.line_for_caret_with_diagnostics(&caret, diagnostics)?
            .fixed_caret_geometry(&caret)
    }

    /// Finds the core line that owns a canonical caret without computing its x
    /// coordinate.
    pub fn line_for_caret_with_diagnostics(
        &self,
        caret: &Caret,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Result<LocatedLayoutLine<'_>, LayoutError> {
        diagnostics.query();
        let Some(locations) = self.element_line_index.get(caret.element) else {
            return Err(LayoutError {
                kind: LayoutErrorKind::InvalidPosition,
            });
        };
        for location in locations {
            diagnostics.examined();
            let line = &self.pages[location.page].lines[location.line];
            if caret.offset >= line.source.start
                && caret.offset <= line.source.end
                && is_boundary_in_line(line, caret.offset)
            {
                return Ok(LocatedLayoutLine {
                    page_index: location.page,
                    line,
                });
            }
        }
        Err(LayoutError {
            kind: LayoutErrorKind::InvalidPosition,
        })
    }

    /// First page owning this element/source range in the same page-and-line
    /// order used by the former complete-layout owner scan.
    pub fn page_for_position_with_diagnostics(
        &self,
        element: usize,
        offset: usize,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<usize> {
        self.page_for_position_in_range_with_diagnostics(element, offset, None, diagnostics)
    }

    pub fn page_for_position_in_range_with_diagnostics(
        &self,
        element: usize,
        offset: usize,
        page_range: Option<(usize, usize)>,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<usize> {
        diagnostics.query();
        self.element_line_index
            .get(element)?
            .iter()
            .find_map(|location| {
                diagnostics.examined();
                if page_range
                    .is_some_and(|(first, last)| location.page < first || location.page > last)
                {
                    return None;
                }
                let line = &self.pages[location.page].lines[location.line];
                (offset >= line.source.start && offset <= line.source.end).then_some(location.page)
            })
    }

    /// Locates the current body line in document order for vertical movement.
    /// Title-page lines are deliberately outside this navigation domain.
    pub fn body_line_index_for_position_with_diagnostics(
        &self,
        element: usize,
        offset: usize,
        diagnostics: &mut InteractionQueryDiagnostics,
    ) -> Option<usize> {
        diagnostics.query();
        self.body_lines_by_element
            .get(element)?
            .iter()
            .copied()
            .find(|index| {
                diagnostics.examined();
                let entry = self.body_line_index[*index];
                offset >= entry.source.start && offset <= entry.source.end
            })
    }

    pub fn body_line(&self, index: usize) -> Option<&LayoutLine> {
        let entry = self.body_line_index.get(index)?;
        self.pages
            .get(entry.location.page)?
            .lines
            .get(entry.location.line)
    }

    pub fn body_line_count(&self) -> usize {
        self.body_line_index.len()
    }

    /// Surface-local title navigation. It is intentionally unavailable on a
    /// body editor or a print composition, even when print includes title pages.
    pub fn title_line_index_for_position(&self, element: usize, offset: usize) -> Option<usize> {
        if self.composition != LayoutComposition::TitlePageEditor {
            return None;
        }
        self.title_lines_by_element
            .get(element)?
            .iter()
            .copied()
            .find(|index| {
                let entry = self.title_line_index[*index];
                offset >= entry.source.start && offset <= entry.source.end
            })
    }

    pub fn title_line(&self, index: usize) -> Option<&LayoutLine> {
        if self.composition != LayoutComposition::TitlePageEditor {
            return None;
        }
        let entry = self.title_line_index.get(index)?;
        self.pages
            .get(entry.location.page)?
            .lines
            .get(entry.location.line)
    }

    pub fn title_page_for_position(&self, element: usize, offset: usize) -> Option<usize> {
        if self.composition != LayoutComposition::TitlePageEditor {
            return None;
        }
        self.page_for_position_with_diagnostics(
            element,
            offset,
            &mut InteractionQueryDiagnostics::default(),
        )
        .filter(|page| self.pages.get(*page).is_some_and(|page| page.title_page))
    }

    pub fn title_caret_geometry(&self, caret: Caret) -> Result<FixedRect, LayoutError> {
        if self.composition != LayoutComposition::TitlePageEditor {
            return Err(LayoutError {
                kind: LayoutErrorKind::InvalidPosition,
            });
        }
        self.caret_geometry(caret)
    }

    pub(super) fn rebuild_interaction_indexes(
        &mut self,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) {
        let started = diagnostics.as_ref().map(|_| Instant::now());
        let max_element = self
            .pages
            .iter()
            .flat_map(|page| page.lines.iter().map(|line| line.element))
            .max();
        let count = max_element.map_or(0, |element| element.saturating_add(1));
        self.element_line_index = vec![Vec::new(); count];
        self.body_line_index.clear();
        self.body_line_y_order.clear();
        self.body_lines_by_element = vec![Vec::new(); count];
        self.title_line_index.clear();
        self.title_lines_by_element = vec![Vec::new(); count];
        let page_height = i64::from(self.inputs.paper_height).saturating_mul(FixedPoint::SCALE);
        for (page_index, page) in self.pages.iter().enumerate() {
            for (line_index, line) in page.lines.iter().enumerate() {
                let location = LineLocation {
                    page: page_index,
                    line: line_index,
                };
                if let Some(element_lines) = self.element_line_index.get_mut(line.element) {
                    element_lines.push(location);
                }
                if page.title_page {
                    let page_offset = i64::try_from(page_index)
                        .unwrap_or(i64::MAX)
                        .saturating_mul(page_height);
                    let absolute_y = page_offset.saturating_add(line.geometry.y.raw());
                    let absolute_bottom_y = page_offset.saturating_add(line.baseline_y().raw());
                    let index = self.title_line_index.len();
                    self.title_line_index.push(BodyLineIndexEntry {
                        location,
                        absolute_y,
                        absolute_bottom_y,
                        element: line.element,
                        source: line.source,
                    });
                    if let Some(element_lines) = self.title_lines_by_element.get_mut(line.element) {
                        element_lines.push(index);
                    }
                    continue;
                }
                let page_offset = i64::try_from(page_index)
                    .unwrap_or(i64::MAX)
                    .saturating_mul(page_height);
                let absolute_y = page_offset.saturating_add(line.geometry.y.raw());
                let absolute_bottom_y = page_offset.saturating_add(line.baseline_y().raw());
                let index = self.body_line_index.len();
                self.body_line_index.push(BodyLineIndexEntry {
                    location,
                    absolute_y,
                    absolute_bottom_y,
                    element: line.element,
                    source: line.source,
                });
                if let Some(element_lines) = self.body_lines_by_element.get_mut(line.element) {
                    element_lines.push(index);
                }
            }
        }
        let sort_started = diagnostics.as_ref().map(|_| Instant::now());
        self.body_line_y_order = (0..self.body_line_index.len()).collect();
        self.body_line_y_order.sort_by_key(|index| {
            let entry = self.body_line_index[*index];
            (entry.absolute_y, entry.location.page, entry.location.line)
        });
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            add_elapsed_nanos(&mut diagnostics.body_y_sort_nanos, sort_started);
            add_elapsed_nanos(&mut diagnostics.interaction_index_nanos, started);
        }
    }
}

fn interval_distance(pointer: i64, top: i64, bottom: i64) -> i64 {
    if pointer < top {
        top.saturating_sub(pointer)
    } else if pointer > bottom {
        pointer.saturating_sub(bottom)
    } else {
        0
    }
}

fn display_char_for_x(line: &LayoutLine, x: FixedPoint) -> usize {
    if line.spans.is_empty() {
        // Empty lines have exactly one canonical/display boundary. There is no
        // measured span from which to derive another x boundary.
        let _ = x;
        return 0;
    }
    let mut display_offset = 0usize;
    for span in line.spans.iter() {
        let count = span.display_text.chars().count();
        let offsets = &span.display_boundary_offsets;
        if x.raw() <= span.geometry.x.raw() {
            return display_offset;
        }
        for index in 0..count {
            let left = offsets.get(index).map_or(0, |offset| offset.raw());
            let right = offsets
                .get(index.saturating_add(1))
                .map_or(span.geometry.width.raw(), |offset| offset.raw());
            let midpoint = left.saturating_add(right.saturating_sub(left).div_euclid(2));
            if x.raw() < span.geometry.x.raw().saturating_add(midpoint) {
                return display_offset.saturating_add(index);
            }
        }
        if x.raw()
            <= span
                .geometry
                .x
                .raw()
                .saturating_add(span.geometry.width.raw())
        {
            return display_offset.saturating_add(count);
        }
        display_offset = display_offset.saturating_add(count);
    }
    display_offset
}

fn x_for_display_char(line: &LayoutLine, display_char: usize) -> i64 {
    if line.spans.is_empty() {
        return line.geometry.x.raw();
    }
    let mut display_offset = 0usize;
    for span in line.spans.iter() {
        let count = span.display_text.chars().count();
        if display_char <= display_offset.saturating_add(count) {
            let local = display_char.saturating_sub(display_offset).min(count);
            let offset = span
                .display_boundary_offsets
                .get(local)
                .map_or(span.geometry.width.raw(), |offset| offset.raw());
            return span.geometry.x.raw().saturating_add(offset);
        }
        display_offset = display_offset.saturating_add(count);
    }
    line.geometry
        .x
        .raw()
        .saturating_add(line.geometry.width.raw())
}

fn is_boundary_in_line(line: &LayoutLine, offset: usize) -> bool {
    if offset == line.source.start || offset == line.source.end {
        return true;
    }
    line.text.is_char_boundary(offset - line.source.start)
}
