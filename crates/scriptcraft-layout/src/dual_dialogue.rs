use super::*;
use super::{
    cache::{
        add_elapsed_nanos, prepare_body_for_layout, ElementWrapCache, PreparedBodyCache,
        PreparedBodyEnvironment, PreparedBodyParagraph,
    },
    inputs::{
        resolve_effective_dual_wrapper_paragraph, resolve_effective_paragraph,
        resolve_effective_text_style, EffectiveGeometryMode,
    },
};
use scriptcraft_model::dual_dialogue::{
    self as model_dual_dialogue, DualDialogueGroup, DualDialogueSide,
};
use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
use scriptcraft_model::{Project, StableId};
use std::{sync::Arc, time::Instant};

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_group(
    layout: &mut ScreenplayLayout,
    project: &Project,
    index: usize,
    purpose: LayoutPurpose,
    composition: LayoutComposition,
    prepared_body_cache: &mut Option<&mut PreparedBodyCache>,
    wrap_cache: &mut Option<&mut ElementWrapCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) -> Result<Option<usize>, LayoutError> {
    layout.prepare_group(
        project,
        index,
        purpose,
        composition,
        prepared_body_cache,
        wrap_cache,
        diagnostics,
    )
}

impl ScreenplayLayout {
    #[allow(clippy::too_many_arguments)]
    fn prepare_group(
        &mut self,
        project: &Project,
        index: usize,
        purpose: LayoutPurpose,
        composition: LayoutComposition,
        prepared_body_cache: &mut Option<&mut PreparedBodyCache>,
        wrap_cache: &mut Option<&mut ElementWrapCache>,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) -> Result<Option<usize>, LayoutError> {
        let Some(group) = model_dual_dialogue::group_at_element_index(project, index)
            .filter(|group| model_dual_dialogue::first_member(project, group) == Some(index))
        else {
            return Ok(None);
        };
        let Some(projection) = model_dual_dialogue::active_dual_dialogue_projection(project, group)
        else {
            return Ok(None);
        };
        // The transient active Body group follows the ordinary sequential path.
        if self.active_group_id.as_ref() == Some(&group.id) {
            return Ok(None);
        }

        let resolved_wrapper = resolve_effective_dual_wrapper_paragraph(
            project,
            &group.id,
            &self.layout_environment,
        )?;
        let (outer_left, outer_right) = (resolved_wrapper.left, resolved_wrapper.right);
        let gap = FixedPoint::from_points(geometry::DUAL_DIALOGUE_COLUMN_GAP_POINTS);
        let column_width = FixedPoint::from_raw(
            outer_right
                .raw()
                .saturating_sub(outer_left.raw())
                .saturating_sub(gap.raw())
                .div_euclid(2),
        );
        if column_width.raw() <= 0 {
            return Err(LayoutError {
                kind: LayoutErrorKind::InvalidProfile,
            });
        }
        let left_box = (
            outer_left,
            FixedPoint::from_raw(outer_left.raw().saturating_add(column_width.raw())),
        );
        let right_box = (
            FixedPoint::from_raw(left_box.1.raw().saturating_add(gap.raw())),
            FixedPoint::from_raw(
                left_box
                    .1
                    .raw()
                    .saturating_add(gap.raw())
                    .saturating_add(column_width.raw()),
            ),
        );
        let left = wrap_dual_side(
            project,
            &projection.left,
            left_box,
            purpose,
            composition,
            self,
            prepared_body_cache,
            wrap_cache,
            diagnostics,
        )?;
        let right = wrap_dual_side(
            project,
            &projection.right,
            right_box,
            purpose,
            composition,
            self,
            prepared_body_cache,
            wrap_cache,
            diagnostics,
        )?;

        // Print-hidden members remain semantic group members but do not create
        // a visible block or apply wrapper-owned vertical fields.
        if left.line_count > 0 || right.line_count > 0 {
            let (starts_new_page, space_before) =
                (resolved_wrapper.starts_new_page, resolved_wrapper.space_before);
            let first_visible_space_before = |side: &WrappedDualSide| {
                side.paragraphs
                    .iter()
                    .find_map(|paragraph| {
                        paragraph
                            .prepared
                            .lines
                            .first()
                            .map(|_| paragraph.prepared.paragraph.space_before.raw())
                    })
                    .unwrap_or_default()
            };
            let space_before = FixedPoint::from_raw(
                space_before
                    .raw()
                    .max(first_visible_space_before(&left))
                    .max(first_visible_space_before(&right)),
            );
            let placement_started = diagnostics.as_ref().map(|_| Instant::now());
            self.place_dual_block(&left, &right, starts_new_page, space_before, diagnostics);
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
            }
        }

        let next_index = model_dual_dialogue::last_member(project, group)
            .map_or(index.saturating_add(1), |last| last.saturating_add(1));
        Ok(Some(next_index))
    }
    fn place_dual_side(
        &self,
        page_number: usize,
        side: &WrappedDualSide,
        block_top: FixedPoint,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) -> Vec<LayoutLine> {
        let mut y = block_top;
        let mut placed = Vec::with_capacity(side.line_count);
        for (paragraph_index, paragraph) in side.paragraphs.iter().enumerate() {
            if paragraph_index > 0 && !paragraph.prepared.lines.is_empty() {
                y = FixedPoint::from_raw(
                    y.raw()
                        .saturating_add(paragraph.prepared.paragraph.space_before.raw().max(0)),
                );
            }
            for wrapped in paragraph.prepared.lines.iter() {
                let advance = wrapped.vertical_metrics.leading;
                let mut line = paginate::rebase_prepared_line(
                    wrapped,
                    paragraph.element,
                    page_number,
                    y,
                    diagnostics.as_deref_mut(),
                );
                line.keep_with_next = false;
                placed.push(line);
                y = FixedPoint::from_raw(y.raw().saturating_add(advance.raw()));
            }
        }
        placed
    }

    fn place_dual_block(
        &mut self,
        left: &WrappedDualSide,
        right: &WrappedDualSide,
        starts_new_page: bool,
        space_before: FixedPoint,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) {
        if self.pages.is_empty() || self.pages.last().is_some_and(|page| page.title_page) {
            self.pages.push(self.new_page(false));
        }
        if starts_new_page && self.pages.last().is_some_and(|page| !page.lines.is_empty()) {
            self.pages.push(self.new_page(false));
        }

        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let page_top = FixedPoint::from_points(i64::from(self.inputs.margin_top));
        let content_height = content_bottom.raw().saturating_sub(page_top.raw());
        let block_height = left.height.raw().max(right.height.raw()).max(0);
        let left_fit_extent = if left.line_count == 0 {
            left.height.raw()
        } else {
            left.fit_extent.raw()
        };
        let right_fit_extent = if right.line_count == 0 {
            right.height.raw()
        } else {
            right.fit_extent.raw()
        };
        let fit_extent = left_fit_extent.max(right_fit_extent).max(0);
        // A saturated sum remains larger than any validated printable extent,
        // so overflow cannot be mistaken for a fitting block.
        let fits_empty_page = fit_extent <= content_height;

        let current_page = self.pages.len() - 1;
        let has_boundary = !self.pages[current_page].lines.is_empty();
        let applied_gap = if has_boundary {
            space_before.raw().max(0)
        } else {
            0
        };
        let block_top = FixedPoint::from_raw(
            self.pages[current_page]
                .next_line_top
                .raw()
                .saturating_add(applied_gap),
        );
        let vertical_end = block_top.raw().saturating_add(fit_extent);
        let fits_remaining = vertical_end <= content_bottom.raw();

        if !fits_empty_page {
            // The exception is limited to a block that cannot fit on a fresh
            // page. Start it there once; each fallback line is consumed exactly
            // once and a page is advanced only after a prior line made progress.
            if !self.pages[current_page].lines.is_empty() {
                self.pages.push(self.new_page(false));
            }
            self.spill_oversized_dual_block(left, right, diagnostics);
            return;
        }

        let page_index = if !fits_remaining && !self.pages[current_page].lines.is_empty() {
            self.pages.push(self.new_page(false));
            self.pages.len() - 1
        } else {
            current_page
        };
        let applied_gap = if self.pages[page_index].lines.is_empty() {
            0
        } else {
            applied_gap
        };
        let block_top = FixedPoint::from_raw(
            self.pages[page_index]
                .next_line_top
                .raw()
                .saturating_add(applied_gap),
        );
        let page_number = self.pages[page_index].number;
        let left_lines = self.place_dual_side(page_number, left, block_top, diagnostics);
        let right_lines = self.place_dual_side(page_number, right, block_top, diagnostics);
        self.pages[page_index].lines.extend(left_lines);
        self.pages[page_index].lines.extend(right_lines);
        self.pages[page_index].occupied_lines = self.pages[page_index]
            .occupied_lines
            .saturating_add(left.line_count.max(right.line_count).min(u32::MAX as usize) as u32);
        self.pages[page_index].next_line_top =
            FixedPoint::from_raw(block_top.raw().saturating_add(block_height));
    }

    fn spill_oversized_dual_block(
        &mut self,
        left: &WrappedDualSide,
        right: &WrappedDualSide,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) {
        let start_page = self.pages.len() - 1;
        let page_top = FixedPoint::from_points(i64::from(self.inputs.margin_top));
        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let left_placements = spill_side_lines(left, page_top, content_bottom);
        let right_placements = spill_side_lines(right, page_top, content_bottom);
        let page_count = left_placements
            .page_count
            .max(right_placements.page_count)
            .max(1);
        while self.pages.len() < start_page.saturating_add(page_count) {
            self.pages.push(self.new_page(false));
        }

        let mut occupied = vec![0usize; page_count];
        let mut bottoms = vec![page_top.raw(); page_count];
        for placements in [&left_placements, &right_placements] {
            for (page_offset, count) in placements.lines_per_page.iter().enumerate() {
                occupied[page_offset] = occupied[page_offset].max(*count);
            }
            for (page_offset, bottom) in placements.bottoms.iter().enumerate() {
                bottoms[page_offset] = bottoms[page_offset].max(*bottom);
            }
        }

        // Append every left line before every right line, including pages where
        // an oversized side has spilled forward.
        for placements in [&left_placements, &right_placements] {
            for placement in &placements.lines {
                let page_index = start_page.saturating_add(placement.page_offset);
                let page_number = self.pages[page_index].number;
                let mut line = paginate::rebase_prepared_line(
                    &placement.line,
                    placement.element,
                    page_number,
                    placement.y,
                    diagnostics.as_deref_mut(),
                );
                line.oversized = true;
                line.keep_with_next = false;
                self.pages[page_index].lines.push(line);
            }
        }
        for page_offset in 0..page_count {
            let page = &mut self.pages[start_page + page_offset];
            page.occupied_lines = page
                .occupied_lines
                .saturating_add(occupied[page_offset].min(u32::MAX as usize) as u32);
            page.next_line_top = FixedPoint::from_raw(bottoms[page_offset]);
        }
    }
}

struct PreparedDualParagraph {
    element: usize,
    prepared: Arc<PreparedBodyParagraph>,
}

struct WrappedDualSide {
    paragraphs: Vec<PreparedDualParagraph>,
    height: FixedPoint,
    /// Relative final natural-bottom extent, separate from leading advancement.
    fit_extent: FixedPoint,
    line_count: usize,
}

struct SpilledDualLine {
    element: usize,
    line: LayoutLine,
    page_offset: usize,
    y: FixedPoint,
}

struct SpilledDualSide {
    lines: Vec<SpilledDualLine>,
    lines_per_page: Vec<usize>,
    bottoms: Vec<i64>,
    page_count: usize,
}

fn spill_side_lines(
    side: &WrappedDualSide,
    page_top: FixedPoint,
    content_bottom: FixedPoint,
) -> SpilledDualSide {
    let mut result = SpilledDualSide {
        lines: Vec::with_capacity(side.line_count),
        lines_per_page: vec![0],
        bottoms: vec![page_top.raw()],
        page_count: 1,
    };
    let mut page_offset = 0usize;
    let mut y = page_top;

    for (paragraph_index, paragraph) in side.paragraphs.iter().enumerate() {
        let mut gap_pending = paragraph_index > 0;
        for wrapped in paragraph.prepared.lines.iter() {
            let advance = wrapped.vertical_metrics.leading.raw();
            let current_count = result.lines_per_page[page_offset];
            let gap = if gap_pending && current_count > 0 {
                paragraph.prepared.paragraph.space_before.raw().max(0)
            } else {
                0
            };
            let proposed_y = FixedPoint::from_raw(y.raw().saturating_add(gap));
            let fits_page = super::geometry::natural_bottom(proposed_y, wrapped.vertical_metrics)
                .raw()
                <= content_bottom.raw();

            if !fits_page && current_count > 0 {
                page_offset = page_offset.saturating_add(1);
                if result.lines_per_page.len() <= page_offset {
                    result.lines_per_page.push(0);
                    result.bottoms.push(page_top.raw());
                }
                result.page_count = page_offset.saturating_add(1);
                y = page_top;
                gap_pending = false;
            }

            let count = result.lines_per_page[page_offset];
            let line_gap = if gap_pending && count > 0 {
                paragraph.prepared.paragraph.space_before.raw().max(0)
            } else {
                0
            };
            let line_y = FixedPoint::from_raw(y.raw().saturating_add(line_gap));
            let line_bottom = line_y.raw().saturating_add(advance);
            result.lines.push(SpilledDualLine {
                element: paragraph.element,
                line: wrapped.clone(),
                page_offset,
                y: line_y,
            });
            result.lines_per_page[page_offset] = count.saturating_add(1);
            result.bottoms[page_offset] = result.bottoms[page_offset].max(line_bottom);
            y = FixedPoint::from_raw(line_bottom);
            gap_pending = false;
        }
    }

    result
}

pub(super) fn active_group_boundaries(
    layout: &ScreenplayLayout,
    project: &Project,
    group: &DualDialogueGroup,
) -> Vec<EditorGroupBoundary> {
    let Some(first_element) = model_dual_dialogue::first_member(project, group) else {
        return Vec::new();
    };
    let Some(last_element) = model_dual_dialogue::last_member(project, group) else {
        return Vec::new();
    };
    let first = layout
        .pages
        .iter()
        .enumerate()
        .find_map(|(page_index, page)| {
            page.lines
                .iter()
                .find(|line| line.element == first_element)
                .map(|line| (page_index, line))
        });
    let last = layout
        .pages
        .iter()
        .enumerate()
        .rev()
        .find_map(|(page_index, page)| {
            page.lines
                .iter()
                .rev()
                .find(|line| line.element == last_element)
                .map(|line| (page_index, line))
        });
    let (Some((upper_page, first_line)), Some((lower_page, last_line))) = (first, last) else {
        return Vec::new();
    };
    let content = &layout.pages[upper_page].content_bounds;
    let x = FixedPoint::from_points(i64::from(content.x));
    let width = FixedPoint::from_points(i64::from(content.width));
    let upper = FixedRect {
        x,
        y: first_line.paragraph_bounds.y,
        width,
        height: FixedPoint::default(),
    };
    let lower = FixedRect {
        x,
        y: FixedPoint::from_raw(
            last_line
                .paragraph_bounds
                .y
                .raw()
                .saturating_add(last_line.paragraph_bounds.height.raw()),
        ),
        width,
        height: FixedPoint::default(),
    };
    vec![EditorGroupBoundary {
        group_id: group.id.clone(),
        upper_page,
        upper,
        lower_page,
        lower,
    }]
}

#[allow(clippy::too_many_arguments)]
fn wrap_dual_side(
    project: &Project,
    side: &DualDialogueSide,
    column: (FixedPoint, FixedPoint),
    purpose: LayoutPurpose,
    composition: LayoutComposition,
    layout: &ScreenplayLayout,
    prepared_cache: &mut Option<&mut PreparedBodyCache>,
    cache: &mut Option<&mut ElementWrapCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) -> Result<WrappedDualSide, LayoutError> {
    let mut paragraphs = Vec::new();
    let mut height = 0i64;
    let mut fit_top = 0i64;
    let mut fit_extent = 0i64;
    let mut line_count = 0usize;
    for id in side.member_ids() {
        let Some((index, element)) = project
            .elements
            .iter()
            .enumerate()
            .find(|(_, element)| &element.id == id)
        else {
            continue;
        };
        let prepared_lookup_started = diagnostics.as_ref().map(|_| Instant::now());
        let element_key = element.element_type();
        let resolved_text_style =
            resolve_effective_text_style(project, &element_key, &layout.layout_environment)?;
        let mut paragraph = resolve_effective_paragraph(
            project,
            &element_key,
            &element.id,
            &element.runs,
            &layout.layout_environment,
        )?;
        if purpose == LayoutPurpose::Print && resolved_text_style.hidden_text {
            continue;
        }
        paragraph.left = column.0;
        paragraph.right = column.1;
        paragraph.first_indent = FixedPoint::default();
        if project
            .element_semantics(&element.element_type())
            .is_some_and(|semantics| semantics.capabilities().character_cue)
        {
            paragraph.alignment = ParagraphAlignment::Center;
        }
        let environment = PreparedBodyEnvironment {
            purpose,
            composition,
            geometry_mode: EffectiveGeometryMode::DualColumn,
            paragraph,
            resolved_text_style,
            element_key,
            layout_environment: layout.layout_environment.clone(),
        };
        let prepared = prepare_body_for_layout(
            prepared_cache.as_deref_mut(),
            cache.as_deref_mut(),
            diagnostics.as_deref_mut(),
            layout,
            element,
            environment,
            prepared_lookup_started,
        )?;
        if !paragraphs.is_empty() && !prepared.lines.is_empty() {
            fit_top = fit_top.saturating_add(paragraph.space_before.raw().max(0));
        }
        for line in prepared.lines.iter() {
            fit_extent = super::geometry::natural_bottom(
                FixedPoint::from_raw(fit_top),
                line.vertical_metrics,
            )
            .raw();
            fit_top = fit_top.saturating_add(line.vertical_metrics.leading.raw());
        }
        if !paragraphs.is_empty() {
            height = height
                .checked_add(paragraph.space_before.raw().max(0))
                .ok_or(LayoutError { kind: super::geometry::LayoutErrorKind::InvalidEffectiveLeading })?;
        }
        height = prepared.lines.iter().try_fold(height, |sum, line| {
            sum.checked_add(line.vertical_metrics.leading.raw())
        }).ok_or(LayoutError { kind: super::geometry::LayoutErrorKind::InvalidEffectiveLeading })?;
        line_count = line_count.saturating_add(prepared.lines.len());
        paragraphs.push(PreparedDualParagraph {
            element: index,
            prepared,
        });
    }
    Ok(WrappedDualSide {
        paragraphs,
        height: FixedPoint::from_raw(height),
        fit_extent: FixedPoint::from_raw(fit_extent),
        line_count,
    })
}
