use super::*;
use super::{
    cache::{
        add_elapsed_nanos, prepare_body_for_layout, ElementWrapCache, PreparedBodyCache,
        PreparedBodyEnvironment, PreparedBodyParagraph,
    },
    inputs::{effective_paragraph, EffectiveGeometryMode, EffectiveParagraph},
};
use scriptcraft_model::dual_dialogue::{
    self as model_dual_dialogue, DualDialogueGroup, DualDialogueSide,
};
use scriptcraft_model::formatting::{
    formatting_for_role, EffectiveTextStyle, FixedPoint, ParagraphAlignment, SourceLayoutSettings,
    SourceParagraphOverrides,
};
use scriptcraft_model::{LayoutInputs, Project, Role, StableId};
use std::{sync::Arc, time::Instant};

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_group(
    layout: &mut ScreenplayLayout,
    project: &Project,
    index: usize,
    source_settings: &SourceLayoutSettings,
    purpose: LayoutPurpose,
    composition: LayoutComposition,
    prepared_body_cache: &mut Option<&mut PreparedBodyCache>,
    wrap_cache: &mut Option<&mut ElementWrapCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) -> Result<Option<usize>, LayoutError> {
    layout.prepare_group(
        project,
        index,
        source_settings,
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
        source_settings: &SourceLayoutSettings,
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

        let (outer_left, outer_right) =
            dual_outer_extent(&self.inputs, source_settings, project, group)?;
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
            &self.inputs,
            source_settings,
            purpose,
            composition,
            self,
            prepared_body_cache,
            wrap_cache,
            diagnostics,
        );
        let right = wrap_dual_side(
            project,
            &projection.right,
            right_box,
            &self.inputs,
            source_settings,
            purpose,
            composition,
            self,
            prepared_body_cache,
            wrap_cache,
            diagnostics,
        );

        // Print-hidden members remain semantic group members but do not create
        // a visible block or apply wrapper-owned vertical fields.
        if left.line_count > 0 || right.line_count > 0 {
            let wrapper = wrapper_overrides(project, source_settings, &group.id);
            let starts_new_page = wrapper
                .starts_new_page
                .unwrap_or_else(|| source_settings.starts_new_page(Role::General));
            let native_general = formatting_for_role(Role::General).paragraph;
            let space_before = wrapper
                .paragraph
                .space_before
                .or(source_settings.paragraph_spec(Role::General).space_before)
                .unwrap_or_else(|| FixedPoint::from_points(i64::from(native_general.space_before)));
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
                let advance = paragraph.prepared.paragraph.line_advance;
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
        let capacity = self.inputs.lines_per_page as usize;
        let block_height = left.height.raw().max(right.height.raw()).max(0);
        let required_lines = left.line_count.max(right.line_count);
        // A saturated sum remains larger than any validated printable extent,
        // so overflow cannot be mistaken for a fitting block.
        let fits_empty_page = block_height <= content_height && required_lines <= capacity;

        let current_page = self.pages.len() - 1;
        let has_boundary = !self.pages[current_page].lines.is_empty();
        let applied_gap = if has_boundary {
            space_before.raw().max(0)
        } else {
            0
        };
        let block_top = FixedPoint::from_raw(
            self.pages[current_page]
                .next_line_y
                .raw()
                .saturating_add(applied_gap),
        );
        let vertical_end = block_top.raw().saturating_add(block_height);
        let line_end =
            (self.pages[current_page].occupied_lines as usize).saturating_add(required_lines);
        let fits_remaining = line_end <= capacity && vertical_end <= content_bottom.raw();

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
                .next_line_y
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
            .saturating_add(required_lines.min(u32::MAX as usize) as u32);
        self.pages[page_index].next_line_y =
            FixedPoint::from_raw(block_top.raw().saturating_add(block_height));
    }

    fn spill_oversized_dual_block(
        &mut self,
        left: &WrappedDualSide,
        right: &WrappedDualSide,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) {
        let start_page = self.pages.len() - 1;
        let capacity = self.inputs.lines_per_page as usize;
        let page_top = FixedPoint::from_points(i64::from(self.inputs.margin_top));
        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let left_placements = spill_side_lines(left, page_top, content_bottom, capacity);
        let right_placements = spill_side_lines(right, page_top, content_bottom, capacity);
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
            page.next_line_y = FixedPoint::from_raw(bottoms[page_offset]);
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
    capacity: usize,
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
            let advance = paragraph.prepared.paragraph.line_advance.raw().max(0);
            let current_count = result.lines_per_page[page_offset];
            let gap = if gap_pending && current_count > 0 {
                paragraph.prepared.paragraph.space_before.raw().max(0)
            } else {
                0
            };
            let proposed_y = FixedPoint::from_raw(y.raw().saturating_add(gap));
            let proposed_end = proposed_y.raw().saturating_add(advance);
            let fits_page = current_count < capacity && proposed_end <= content_bottom.raw();

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

fn wrapper_overrides(
    project: &Project,
    source: &SourceLayoutSettings,
    group_id: &StableId,
) -> SourceParagraphOverrides {
    project
        .dual_dialogue_paragraph_overrides
        .get(group_id)
        .copied()
        .or_else(|| source.direct_paragraph_overrides.get(group_id).copied())
        .unwrap_or_default()
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

fn dual_outer_extent(
    inputs: &LayoutInputs,
    source: &SourceLayoutSettings,
    project: &Project,
    group: &DualDialogueGroup,
) -> Result<(FixedPoint, FixedPoint), LayoutError> {
    let native = formatting_for_role(Role::General).paragraph;
    let app_left = FixedPoint::from_points(i64::from(inputs.margin_left));
    let app_right = FixedPoint::from_points(i64::from(
        inputs.paper_width.saturating_sub(inputs.margin_right),
    ));
    let general = source.paragraph_spec(Role::General);
    let role_left = general
        .left_indent
        .unwrap_or_else(|| FixedPoint::parse_inches(native.left_indent).unwrap_or(app_left));
    let role_right = general
        .right_indent
        .unwrap_or_else(|| FixedPoint::parse_inches(native.right_indent).unwrap_or(app_right));
    let wrapper = wrapper_overrides(project, source, &group.id).paragraph;
    let left = wrapper.left_indent.unwrap_or(role_left);
    let right = wrapper.right_indent.unwrap_or(role_right);
    let page_right = FixedPoint::from_points(i64::from(inputs.paper_width));
    let gap = FixedPoint::from_points(geometry::DUAL_DIALOGUE_COLUMN_GAP_POINTS);
    if left.raw() < 0
        || right <= left
        || right > page_right
        || right.raw().saturating_sub(left.raw()) <= gap.raw()
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok((left, right))
}

#[allow(clippy::too_many_arguments)]
fn wrap_dual_side(
    project: &Project,
    side: &DualDialogueSide,
    column: (FixedPoint, FixedPoint),
    inputs: &LayoutInputs,
    source: &SourceLayoutSettings,
    purpose: LayoutPurpose,
    composition: LayoutComposition,
    layout: &ScreenplayLayout,
    prepared_cache: &mut Option<&mut PreparedBodyCache>,
    cache: &mut Option<&mut ElementWrapCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) -> WrappedDualSide {
    let mut paragraphs = Vec::new();
    let mut height = 0i64;
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
        // Phase 5 replaces this static-layout adapter with definition-driven layout.
        let role = element.transitional_role();
        let prepared_lookup_started = diagnostics.as_ref().map(|_| Instant::now());
        let text_style = source.text_style(role);
        if purpose == LayoutPurpose::Print && text_style.hidden_text {
            continue;
        }
        let mut paragraph = effective_paragraph(inputs, source, role, &element.id, &text_style);
        paragraph.left = column.0;
        paragraph.right = column.1;
        paragraph.first_indent = FixedPoint::default();
        if role == Role::Character {
            paragraph.alignment = ParagraphAlignment::Center;
        }
        let environment = PreparedBodyEnvironment {
            purpose,
            composition,
            geometry_mode: EffectiveGeometryMode::DualColumn,
            paragraph,
            text_style,
            char_width: paragraph.char_width,
        };
        let prepared = prepare_body_for_layout(
            prepared_cache.as_deref_mut(),
            cache.as_deref_mut(),
            diagnostics.as_deref_mut(),
            layout,
            element,
            environment,
            prepared_lookup_started,
        );
        if !paragraphs.is_empty() {
            height = height.saturating_add(paragraph.space_before.raw().max(0));
        }
        height = height.saturating_add(
            paragraph
                .line_advance
                .raw()
                .saturating_mul(prepared.lines.len().min(i64::MAX as usize) as i64),
        );
        line_count = line_count.saturating_add(prepared.lines.len());
        paragraphs.push(PreparedDualParagraph {
            element: index,
            prepared,
        });
    }
    WrappedDualSide {
        paragraphs,
        height: FixedPoint::from_raw(height),
        line_count,
    }
}
