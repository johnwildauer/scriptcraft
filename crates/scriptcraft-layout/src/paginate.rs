use super::*;
use super::{
    cache::{
        add_elapsed_nanos, prepare_body_for_layout, ElementWrapCache, PreparedBodyCache,
        PreparedBodyEnvironment, PreparedBodyParagraph,
    },
    inputs::{effective_paragraph, validate_profile, EffectiveGeometryMode, EffectiveParagraph},
    wrap::WrappedLine,
};
use scriptcraft_model::{
    formatting::{FixedPoint, ParagraphAlignment, SourceLayoutSettings},
    validate_layout_inputs, LayoutInputs, Project, Role,
};
use std::{sync::Arc, time::Instant};

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
            .map(|preservation| preservation.source_layout_settings().clone())
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
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            source_settings,
            purpose,
            match purpose {
                LayoutPurpose::Print => LayoutComposition::Print { include_title_page: true },
                LayoutPurpose::Editor => LayoutComposition::BodyEditor,
            },
            None,
            None,
            None,
            false,
            &BodyEditorPresentation::default(),
        )
    }

    pub fn build_with_composition(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        composition: LayoutComposition,
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
        )
    }

    /// Builds Body-editor geometry with one optional, transient active group.
    pub fn build_with_body_presentation(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        presentation: &BodyEditorPresentation,
    ) -> Result<Self, LayoutError> {
        let mut effective_settings = source_settings.clone();
        effective_settings
            .direct_paragraph_overrides
            .extend(project.direct_paragraph_overrides.clone());
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            &effective_settings,
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            None,
            None,
            None,
            false,
            presentation,
        )
    }

    /// Builds the same deterministic layout while returning value-free work
    /// measurements separately from the layout result.
    pub fn build_with_effective_inputs_for_purpose_and_diagnostics(
        project: &Project,
        inputs: &LayoutInputs,
        source_settings: &SourceLayoutSettings,
        purpose: LayoutPurpose,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<Self, LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        diagnostics.complete_project_validations = 1;
        Self::build_with_effective_inputs_for_purpose_internal(
            project,
            inputs,
            source_settings,
            purpose,
            match purpose {
                LayoutPurpose::Print => LayoutComposition::Print { include_title_page: true },
                LayoutPurpose::Editor => LayoutComposition::BodyEditor,
            },
            Some(diagnostics),
            None,
            None,
            false,
            &BodyEditorPresentation::default(),
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
    ) -> Result<Self, LayoutError> {
        if !validated_session {
            project.validate()?;
        }
        validate_layout_inputs(inputs).map_err(|_| LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        })?;
        validate_profile(inputs)?;
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.layout_builds = diagnostics.layout_builds.saturating_add(1);
        }
        let active_group_id = if composition == LayoutComposition::BodyEditor {
            body_presentation
                .active_group_id
                .as_ref()
                .filter(|_| body_presentation.is_for_project(project))
                .filter(|requested| project.dual_dialogue_groups.iter().any(|group| &group.id == *requested))
                .cloned()
        } else {
            None
        };
        let char_width = inputs.char_width;
        let font_family = inputs.font_family.clone();
        let mut output = Self {
            inputs: inputs.clone(),
            source_settings: source_settings.clone(),
            font_family,
            char_width,
            line_height: inputs.line_advance,
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
            inputs,
            purpose,
            include_title_page,
            include_body,
            &mut wrap_cache,
            &mut diagnostics,
        );

        let mut index = 0;
        while include_body && index < project.elements.len() {
            let element = &project.elements[index];
            if let Some(next_index) = dual_dialogue::prepare_group(
                &mut output,
                project,
                index,
                source_settings,
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
            let text_style = output.source_settings.text_style(element.role);
            if purpose == LayoutPurpose::Print && text_style.hidden_text {
                index = index.saturating_add(1);
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
                project.uses_legacy_layout_defaults(),
                &text_style,
            );
            let environment = PreparedBodyEnvironment {
                purpose,
                composition,
                geometry_mode: EffectiveGeometryMode::Sequential,
                paragraph,
                text_style,
                char_width: paragraph.char_width,
            };
            let prepared = prepare_body_for_layout(
                prepared_body_cache.as_deref_mut(),
                wrap_cache.as_deref_mut(),
                diagnostics.as_deref_mut(),
                &output,
                element,
                environment,
                prepared_lookup_started,
            );
            let placement_started = diagnostics.as_ref().map(|_| Instant::now());
            output.place_element(prepared, index, element.role, paragraph, &mut diagnostics);
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
            }
            index = index.saturating_add(1);
        }
        if output.pages.is_empty() {
            output.pages.push(output.new_page(composition == LayoutComposition::TitlePageEditor));
        }
        if let Some(active_group_id) = output.active_group_id.clone() {
            if let Some(group) = project.dual_dialogue_groups.iter().find(|group| group.id == active_group_id) {
                output.editor_group_boundaries = dual_dialogue::active_group_boundaries(&output, project, group);
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
            next_line_y: FixedPoint::from_points(i64::from(self.inputs.margin_top)),
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
        let mut spans = spans::positioned_spans(&line, FixedRect {
            x: line_left,
            y,
            width: FixedPoint::default(),
            height: line.paragraph.line_advance,
        });
        let text_width = FixedPoint::from_raw(
            spans.iter().map(|span| span.geometry.width.raw()).sum::<i64>(),
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
        let gap_width = if distribute { spare.div_euclid(justified_gaps.len() as i64) } else { 0 };
        let gap_remainder = if distribute { spare.rem_euclid(justified_gaps.len() as i64) as usize } else { 0 };
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
            span.geometry.x = FixedPoint::from_raw(span.geometry.x.raw().saturating_add(alignment_shift));
        }
        let height = line.paragraph.line_advance;
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
                justified_gaps.iter().position(|gap| *gap == index).map(|rank| {
                    gap_width.saturating_add(if rank < gap_remainder { 1 } else { 0 })
                }).unwrap_or(0)
            };
            let mut display_index = 0usize;
            let mut shift = 0i64;
            for span in &mut spans {
                let count = span.display_text.chars().count();
                let internal = (display_index..display_index.saturating_add(count))
                    .map(gap_adjustment).sum::<i64>();
                span.geometry.x = FixedPoint::from_raw(span.geometry.x.raw().saturating_add(shift));
                span.geometry.width = FixedPoint::from_raw(span.geometry.width.raw().saturating_add(internal));
                shift = shift.saturating_add(internal);
                display_index = display_index.saturating_add(count);
            }
        }
        LayoutLine {
            page: page_number,
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
            spans: spans.into(),
            text_style: line.text_style,
            char_width: line.char_width,
            alignment: line.paragraph.alignment,
            unaligned_line_left: line_left,
            geometry,
            paragraph_bounds,
            rect: compat_rect(paragraph_bounds),
            keep_with_next: !title_page && keep_with_next(line.role),
            oversized: line.oversized,
        }
    }

    fn place_element(
        &mut self,
        prepared: Arc<PreparedBodyParagraph>,
        element: usize,
        role: Role,
        paragraph: EffectiveParagraph,
        diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
    ) {
        let lines = &prepared.lines;
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
        let content_bottom = FixedPoint::from_points(i64::from(
            self.inputs
                .paper_height
                .saturating_sub(self.inputs.margin_bottom),
        ));
        let minimum_height = paragraph.line_advance.raw().saturating_mul(minimum as i64);
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
            && (self
                .pages
                .last()
                .map_or(0, |page| page.occupied_lines as usize)
                .saturating_add(minimum)
                > capacity
                || !vertical_fit)
        {
            self.pages.push(self.new_page(false));
        }
        let applied_gap =
            if has_boundary && self.pages.last().is_some_and(|page| !page.lines.is_empty()) {
                if let Some(page) = self.pages.last_mut() {
                    page.next_line_y =
                        FixedPoint::from_raw(page.next_line_y.raw().saturating_add(extra_gap));
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
        let content_height =
            content_bottom.raw() - FixedPoint::from_points(i64::from(self.inputs.margin_top)).raw();
        let oversized_element = lines.len() > capacity || total_height > content_height;
        for wrapped in lines.iter() {
            let current_fits = self.pages.last().is_some_and(|page| {
                (page.occupied_lines as usize) < capacity
                    && (page.lines.is_empty()
                        || page
                            .next_line_y
                            .raw()
                            .saturating_add(paragraph.line_advance.raw())
                            <= content_bottom.raw())
            });
            if !current_fits {
                self.pages.push(self.new_page(false));
            }
            let page_index = self.pages.len() - 1;
            if self.pages[page_index].lines.is_empty() {
                self.pages[page_index].occupied_lines = 0;
            }
            let page_number = self.pages[page_index].number;
            let next_line_y = self.pages[page_index].next_line_y;
            let mut line = rebase_prepared_line(
                wrapped,
                element,
                page_number,
                next_line_y,
                diagnostics.as_deref_mut(),
            );
            line.oversized |= oversized_element;
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
    line.geometry.y = FixedPoint::from_raw(prepared.geometry.y.raw().saturating_add(vertical_delta));
    line.paragraph_bounds.y = y;
    line.rect = compat_rect(line.paragraph_bounds);
    for span in Arc::make_mut(&mut line.spans) {
        span.source.element = element;
        span.geometry.y = FixedPoint::from_raw(span.geometry.y.raw().saturating_add(vertical_delta));
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

fn keep_with_next(role: Role) -> bool {
    matches!(
        role,
        Role::SceneHeading | Role::Character | Role::Parenthetical
    )
}
