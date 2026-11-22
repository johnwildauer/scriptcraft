use super::*;
use super::{
    cache::{
        add_elapsed_nanos, prepare_body_for_layout, ElementWrapCache, PreparedBodyCache,
        PreparedBodyEnvironment, PreparedBodyParagraph,
    },
    inputs::{
        validate_page_profile, EffectiveGeometryMode, EffectiveParagraph, LayoutEnvironment,
    },
    wrap::WrappedLine,
};
use scriptcraft_model::{formatting::{FixedPoint, ParagraphAlignment, SourceLayoutSettings}, LayoutInputs, Project};
use std::{sync::Arc, time::Instant};

impl ScreenplayLayout {
    /// Builds composition with the explicit resolved-font environment used by
    /// desktop preparation. Every line and span uses that environment.
    pub fn build_with_composition_and_environment(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        composition: LayoutComposition,
        environment: &crate::inputs::LayoutEnvironment,
    ) -> Result<Self, LayoutError> {
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            source_settings,
            composition.purpose(),
            composition,
            None,
            None,
            None,
            false,
            &BodyEditorPresentation::default(),
            Arc::new(environment.clone()),
        )
    }

    pub(super) fn build_with_effective_inputs_for_purpose_internal(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        purpose: LayoutPurpose,
        composition: LayoutComposition,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
        mut wrap_cache: Option<&mut ElementWrapCache>,
        mut prepared_body_cache: Option<&mut PreparedBodyCache>,
        validated_session: bool,
        body_presentation: &BodyEditorPresentation,
        layout_environment: Arc<LayoutEnvironment>,
    ) -> Result<Self, LayoutError> {
        if !validated_session {
            project.validate()?;
        }
        validate_page_profile(&layout_environment.page)?;
        if layout_environment.page != inputs.page_only() {
            return Err(LayoutError {
                kind: LayoutErrorKind::InvalidProfile,
            });
        }
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.layout_builds = diagnostics.layout_builds.saturating_add(1);
        }
        let active_group_id = if composition == LayoutComposition::BodyEditor {
            body_presentation
                .active_group_id
                .as_ref()
                .filter(|_| body_presentation.is_for_project(project))
                .filter(|requested| {
                    project.dual_dialogue_groups.iter().any(|group| {
                        &group.id == *requested
                            && scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(
                                project, group,
                            )
                            .is_some()
                    })
                })
                .cloned()
        } else {
            None
        };
        let mut output = Self {
            inputs: inputs.clone(),
            layout_environment,
            source_settings: source_settings.clone(),
            purpose,
            composition,
            active_group_id,
            editor_group_boundaries: Vec::new(),
            pages: Vec::new(),
            element_line_index: Vec::new(),
            body_line_index: Vec::new(),
            body_line_y_order: Vec::new(),
            body_lines_by_element: Vec::new(),
            title_line_index: Vec::new(),
            title_lines_by_element: Vec::new(),
        };
        let include_title_page = match composition {
            LayoutComposition::BodyEditor => false,
            LayoutComposition::TitlePageEditor => true,
            LayoutComposition::Print { include_title_page } => include_title_page,
        };
        let include_body = !matches!(composition, LayoutComposition::TitlePageEditor);
        title_page::compose_title_pages(
            &mut output,
            project,
            purpose,
            include_title_page,
            include_body,
            &mut wrap_cache,
            &mut diagnostics,
        )?;

        let mut index = 0;
        while include_body && index < project.elements.len() {
            let element = &project.elements[index];
            if let Some(next_index) = dual_dialogue::prepare_group(
                &mut output,
                project,
                index,
                purpose,
                composition,
                &mut prepared_body_cache,
                &mut wrap_cache,
                &mut diagnostics,
            )? {
                index = next_index;
                continue;
            }

            let prepared_lookup_started = diagnostics.as_ref().map(|_| Instant::now());
            let element_key = element.element_type();
            let resolved_text_style = super::inputs::resolve_effective_text_style(
                project,
                &element_key,
                &output.layout_environment,
            )?;
            let paragraph = super::inputs::resolve_effective_paragraph(
                project,
                &element_key,
                &element.id,
                &element.runs,
                &output.layout_environment,
            )?;
            if purpose == LayoutPurpose::Print && resolved_text_style.hidden_text {
                index = index.saturating_add(1);
                continue;
            }
            let environment = PreparedBodyEnvironment {
                purpose,
                composition,
                geometry_mode: EffectiveGeometryMode::Sequential,
                paragraph,
                resolved_text_style,
                element_key,
                layout_environment: output.layout_environment.clone(),
            };
            let prepared = prepare_body_for_layout(
                prepared_body_cache.as_deref_mut(),
                wrap_cache.as_deref_mut(),
                diagnostics.as_deref_mut(),
                &output,
                element,
                environment,
                prepared_lookup_started,
            )?;
            let placement_started = diagnostics.as_ref().map(|_| Instant::now());
            output.place_element(prepared, index, paragraph, &mut diagnostics)?;
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
            }
            index = index.saturating_add(1);
        }
        if output.pages.is_empty() {
            output
                .pages
                .push(output.new_page(composition == LayoutComposition::TitlePageEditor));
        }
        if let Some(active_group_id) = output.active_group_id.clone() {
            if let Some(group) = project
                .dual_dialogue_groups
                .iter()
                .find(|group| group.id == active_group_id)
            {
                output.editor_group_boundaries =
                    dual_dialogue::active_group_boundaries(&output, project, group);
            }
        }
        output.rebuild_interaction_indexes(diagnostics.as_deref_mut());
        Ok(output)
    }

    pub(super) fn new_page(&self, title_page: bool) -> LayoutPage {
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
            next_line_top: FixedPoint::from_points(i64::from(self.inputs.margin_top)),
        }
    }

    pub(super) fn make_line(
        &self,
        page_number: usize,
        line: WrappedLine,
        title_page: bool,
        y: FixedPoint,
    ) -> LayoutLine {
        let indent = if line.first_visual_line {
            line.paragraph.first_indent
        } else {
            FixedPoint::default()
        };
        let line_left =
            FixedPoint::from_raw(line.paragraph.left.raw().saturating_add(indent.raw()));
        let line_width =
            FixedPoint::from_raw(line.paragraph.right.raw().saturating_sub(line_left.raw()));
        let line_geometry = FixedRect {
            x: line_left,
            y,
            width: FixedPoint::default(),
            height: line.vertical_metrics.leading,
        };
        let mut spans = spans::resolved_positioned_spans(
            &line.resolved_scalars,
            line.element,
            line_geometry,
        )
        .into_iter()
        .map(|span| PositionedStyledSpan {
            display_text: span.display_text,
            source: span.source,
            display_source_map: span.display_source_map,
            resolved_text_style: span.text_style,
            geometry: span.geometry,
            display_boundary_offsets: span.display_boundary_offsets,
            underline: span.underline,
            strikethrough: span.strikethrough,
        })
        .collect::<Vec<_>>();
        let text_width = FixedPoint::from_raw(
            spans
                .iter()
                .map(|span| span.geometry.width.raw())
                .sum::<i64>(),
        );
        let spare = line_width.raw().saturating_sub(text_width.raw()).max(0);
        let display_chars = line.display_text.chars().collect::<Vec<_>>();
        let justified_gaps = if line.paragraph.alignment == ParagraphAlignment::Justified
            && !line.final_visual_line
        {
            (1..display_chars.len().saturating_sub(1))
                .filter(|index| {
                    display_chars[*index].is_whitespace()
                        && !display_chars[*index - 1].is_whitespace()
                        && !display_chars[*index + 1].is_whitespace()
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let distribute = !justified_gaps.is_empty();
        let distributed_width = if distribute { spare } else { 0 };
        let gap_width = if distribute {
            spare.div_euclid(justified_gaps.len() as i64)
        } else {
            0
        };
        let gap_remainder = if distribute {
            spare.rem_euclid(justified_gaps.len() as i64) as usize
        } else {
            0
        };
        let text_x = match line.paragraph.alignment {
            ParagraphAlignment::Left => line_left,
            ParagraphAlignment::Right => {
                FixedPoint::from_raw(line.paragraph.right.raw().saturating_sub(text_width.raw()))
            }
            ParagraphAlignment::Center => {
                FixedPoint::from_raw(line_left.raw().saturating_add(spare / 2))
            }
            ParagraphAlignment::Justified => line_left,
        };
        let alignment_shift = text_x.raw().saturating_sub(line_left.raw());
        for span in &mut spans {
            span.geometry.x =
                FixedPoint::from_raw(span.geometry.x.raw().saturating_add(alignment_shift));
        }
        let height = line.vertical_metrics.leading;
        let paragraph_bounds = FixedRect {
            x: line.paragraph.left,
            y,
            width: FixedPoint::from_raw(
                line.paragraph
                    .right
                    .raw()
                    .saturating_sub(line.paragraph.left.raw()),
            ),
            height,
        };
        let geometry = FixedRect {
            x: text_x,
            y,
            width: FixedPoint::from_raw(text_width.raw().saturating_add(distributed_width)),
            height,
        };
        if distribute {
            let gap_adjustment = |index: usize| {
                justified_gaps
                    .iter()
                    .position(|gap| *gap == index)
                    .map(|rank| gap_width.saturating_add(if rank < gap_remainder { 1 } else { 0 }))
                    .unwrap_or(0)
            };
            let mut display_index = 0usize;
            let mut shift = 0i64;
            for span in &mut spans {
                let count = span.display_text.chars().count();
                let span_end = display_index.saturating_add(count);
                let before = (0..display_index).map(gap_adjustment).sum::<i64>();
                let internal = (display_index..span_end).map(gap_adjustment).sum::<i64>();
                span.geometry.x = FixedPoint::from_raw(span.geometry.x.raw().saturating_add(shift));
                let offsets = Arc::make_mut(&mut span.display_boundary_offsets);
                for (boundary, offset) in offsets.iter_mut().enumerate() {
                    let absolute_boundary = display_index.saturating_add(boundary);
                    let adjustment = (0..absolute_boundary).map(gap_adjustment).sum::<i64>();
                    *offset =
                        FixedPoint::from_raw(offset.raw().saturating_add(adjustment - before));
                }
                span.geometry.width =
                    FixedPoint::from_raw(offsets.last().map_or(0, |offset| offset.raw()));
                shift = shift.saturating_add(internal);
                display_index = span_end;
            }
        }
        LayoutLine {
            page: page_number,
            element_type: line.element_type,
            element: line.element,
            source: SourceRange {
                element: line.element,
                start: line.start,
                end: line.end,
            },
            text: line.text,
            display_text: line.display_text,
            display_source_map: line.display_source_map,
            spans: spans.into(),
            alignment: line.paragraph.alignment,
            unaligned_line_left: line_left,
            geometry,
            paragraph_bounds,
            vertical_metrics: line.vertical_metrics,
            rect: compat_rect(paragraph_bounds),
            keep_with_next: !title_page
                && line.paragraph.pagination == super::inputs::PaginationPolicy::KeepWithNext,
            oversized: line.oversized,
        }
    }

    fn place_element(
        &mut self,
        prepared: Arc<PreparedBodyParagraph>,
        element: usize,
        paragraph: EffectiveParagraph,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) -> Result<(), LayoutError> {
        let lines = &prepared.lines;
        let keep = paragraph.pagination == super::inputs::PaginationPolicy::KeepWithNext;
        if self.pages.is_empty() {
            self.pages.push(self.new_page(false));
        }
        if paragraph.starts_new_page
            && self
                .pages
                .last()
                .is_some_and(|page| !page.title_page && !page.lines.is_empty())
        {
            self.pages.push(self.new_page(false));
        }
        let has_boundary = self.pages.last().is_some_and(|page| !page.lines.is_empty());
        let extra_gap = if has_boundary {
            paragraph.space_before.raw().max(0)
        } else {
            0
        };
        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let vertical_fit = self.pages.last().is_some_and(|page| {
            let candidate = if keep {
                lines.get(1).or_else(|| lines.first())
            } else {
                lines.first()
            };
            candidate.map_or(true, |line| {
                let prior_advance = if keep {
                    lines[0].vertical_metrics.leading.raw()
                } else {
                    0
                };
                let candidate_top = FixedPoint::from_raw(
                    page.next_line_top
                        .raw()
                        .saturating_add(extra_gap)
                        .saturating_add(prior_advance),
                );
                super::geometry::natural_bottom(candidate_top, line.vertical_metrics).raw()
                    <= content_bottom.raw()
            })
        });
        if !lines.is_empty()
            && !self.pages.last().is_some_and(|page| page.title_page)
            && has_boundary
            && !vertical_fit
        {
            self.pages.push(self.new_page(false));
        }
        let applied_gap =
            if has_boundary && self.pages.last().is_some_and(|page| !page.lines.is_empty()) {
                if let Some(page) = self.pages.last_mut() {
                    page.next_line_top = FixedPoint::from_raw(
                        page.next_line_top.raw().saturating_add(extra_gap),
                    );
                }
                extra_gap
            } else {
                0
            };
        let line_height = lines.iter().try_fold(0_i64, |sum, line| {
            sum.checked_add(line.vertical_metrics.leading.raw())
        }).ok_or(LayoutError {
            kind: LayoutErrorKind::InvalidEffectiveLeading,
        })?;
        let _total_height = line_height.checked_add(applied_gap).ok_or(LayoutError {
            kind: LayoutErrorKind::InvalidEffectiveLeading,
        })?;
        let fresh_page_top = FixedPoint::from_raw(
            FixedPoint::from_points(i64::from(self.inputs.margin_top))
                .raw()
                .saturating_add(applied_gap),
        );
        let final_line_top = lines
            .iter()
            .take(lines.len().saturating_sub(1))
            .fold(fresh_page_top, |top, line| {
                FixedPoint::from_raw(
                    top.raw()
                        .saturating_add(line.vertical_metrics.leading.raw()),
                )
            });
        let oversized_element = lines.last().is_some_and(|line| {
            super::geometry::natural_bottom(final_line_top, line.vertical_metrics).raw()
                > content_bottom.raw()
        });
        for wrapped in lines.iter() {
            let line_leading = wrapped.vertical_metrics.leading.raw();
            let current_fits = self.pages.last().is_some_and(|page| {
                page.lines.is_empty()
                    || super::geometry::natural_bottom(
                        page.next_line_top,
                        wrapped.vertical_metrics,
                    )
                    .raw()
                        <= content_bottom.raw()
            });
            if !current_fits {
                self.pages.push(self.new_page(false));
            }
            let page_index = self.pages.len() - 1;
            if self.pages[page_index].lines.is_empty() {
                self.pages[page_index].occupied_lines = 0;
            }
            let page_number = self.pages[page_index].number;
            let next_line_top = self.pages[page_index].next_line_top;
            let mut line = rebase_prepared_line(
                wrapped,
                element,
                page_number,
                next_line_top,
                diagnostics.as_deref_mut(),
            );
            line.oversized |= oversized_element;
            self.pages[page_index].lines.push(line);
            self.pages[page_index].occupied_lines =
                self.pages[page_index].occupied_lines.saturating_add(1);
            self.pages[page_index].next_line_top = FixedPoint::from_raw(
                self.pages[page_index]
                    .next_line_top
                    .raw()
                    .saturating_add(line_leading),
            );
        }
        Ok(())
    }
}

pub(super) fn rebase_prepared_line(
    prepared: &LayoutLine,
    element: usize,
    page: usize,
    y: FixedPoint,
    diagnostics: Option<&mut LayoutBuildDiagnostics>,
) -> LayoutLine {
    // Per-line timing is intentionally limited to an active optional carrier.
    // Development readings include this measurement overhead.
    let started = diagnostics.as_ref().map(|_| Instant::now());
    let mut line = prepared.clone();
    let vertical_delta = y.raw().saturating_sub(prepared.paragraph_bounds.y.raw());
    line.page = page;
    line.element = element;
    line.source.element = element;
    line.geometry.y =
        FixedPoint::from_raw(prepared.geometry.y.raw().saturating_add(vertical_delta));
    line.paragraph_bounds.y = y;
    line.rect = compat_rect(line.paragraph_bounds);
    for span in Arc::make_mut(&mut line.spans) {
        span.source.element = element;
        span.geometry.y =
            FixedPoint::from_raw(span.geometry.y.raw().saturating_add(vertical_delta));
    }
    if let Some(diagnostics) = diagnostics {
        add_elapsed_nanos(&mut diagnostics.rebase_nanos, started);
    }
    line
}

pub(super) fn compat_rect(rect: FixedRect) -> Rect {
    let rounded = |value: FixedPoint| value.rounded_points().clamp(0, i64::from(u32::MAX)) as u32;
    Rect {
        x: rounded(rect.x),
        y: rounded(rect.y),
        width: rounded(rect.width),
        height: rounded(rect.height),
    }
}
