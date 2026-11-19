use super::*;
use super::{
    cache::{add_elapsed_nanos, ElementWrapCache},
    inputs::{EffectiveGeometryMode, EffectiveParagraph},
};
use scriptcraft_model::formatting::{
    EffectiveTextStyle, FixedPoint, FontFallback, ParagraphAlignment,
};
use scriptcraft_model::{LayoutInputs, Project, Role};
use std::time::Instant;

#[allow(clippy::too_many_arguments)]
pub(super) fn compose_title_pages(
    layout: &mut ScreenplayLayout,
    project: &Project,
    inputs: &LayoutInputs,
    purpose: LayoutPurpose,
    include_title_page: bool,
    include_body: bool,
    wrap_cache: &mut Option<&mut ElementWrapCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) {
    if include_title_page && !project.title_page.is_empty() {
        let mut page = layout.new_page(true);
        for (index, element) in project.title_page.iter().enumerate() {
            if element.formatting.starts_new_page == Some(true) && !page.lines.is_empty() {
                layout.pages.push(page);
                page = layout.new_page(true);
            }
            let default_left = FixedPoint::from_points(i64::from(layout.inputs.margin_left));
            let default_right = FixedPoint::from_points(i64::from(
                layout
                    .inputs
                    .paper_width
                    .saturating_sub(layout.inputs.margin_right),
            ));
            let requested_left = element.formatting.left_indent.unwrap_or(default_left);
            let requested_right = element.formatting.right_indent.unwrap_or(default_right);
            let (title_left, title_right) = if requested_left.raw() >= 0
                && requested_right > requested_left
                && requested_right <= FixedPoint::from_points(i64::from(layout.inputs.paper_width))
            {
                (requested_left, requested_right)
            } else {
                (default_left, default_right)
            };
            let requested_indent = element.formatting.first_indent.unwrap_or_default();
            let title_indent = if title_right
                .raw()
                .saturating_sub(title_left.raw())
                .saturating_sub(requested_indent.raw())
                >= i64::from(layout.inputs.char_width) * FixedPoint::SCALE
            {
                requested_indent
            } else {
                FixedPoint::default()
            };
            let paragraph = EffectiveParagraph {
                left: title_left,
                right: title_right,
                first_indent: title_indent,
                alignment: element
                    .formatting
                    .alignment
                    .map(Into::into)
                    .unwrap_or(ParagraphAlignment::Left),
                space_before: element.formatting.space_before.unwrap_or_default(),
                line_advance: FixedPoint::from_raw(
                    i64::from(layout.inputs.line_advance)
                        .saturating_mul(i64::from(inputs.font_size.clamp(1, 72)))
                        .saturating_mul(FixedPoint::SCALE)
                        .saturating_add(6)
                        .div_euclid(12),
                ),
                char_width: FixedPoint::from_raw(
                    i64::from(layout.inputs.char_width)
                        .saturating_mul(i64::from(inputs.font_size.clamp(1, 72)))
                        .saturating_mul(FixedPoint::SCALE)
                        .saturating_add(6)
                        .div_euclid(12),
                ),
                starts_new_page: element.formatting.starts_new_page.unwrap_or(false),
                pagination: super::inputs::PaginationPolicy::Ordinary,
            };
            let style = plain_title_style(inputs);
            // Phase 5 replaces this static-layout adapter; General remains the type-less fallback.
            let role = element.transitional_role().unwrap_or(Role::General);
            let lines = wrap::wrap_for_layout(
                wrap_cache.as_deref_mut(),
                diagnostics.as_deref_mut(),
                &element.id,
                &element.text,
                index,
                role,
                element.element_type.as_ref(),
                paragraph,
                style,
                &element.runs,
                paragraph.char_width,
                purpose,
                true,
                EffectiveGeometryMode::Sequential,
            );
            if !page.lines.is_empty() {
                page.next_line_y = FixedPoint::from_raw(
                    page.next_line_y
                        .raw()
                        .saturating_add(paragraph.space_before.raw().max(0)),
                );
            }
            let placement_started = diagnostics.as_ref().map(|_| Instant::now());
            for line in lines {
                let line_advance = line.paragraph.line_advance.raw();
                let content_bottom = FixedPoint::from_points(i64::from(
                    layout
                        .inputs
                        .paper_height
                        .saturating_sub(layout.inputs.margin_bottom),
                ));
                if !page.lines.is_empty()
                    && page.next_line_y.raw().saturating_add(line_advance) > content_bottom.raw()
                {
                    layout.pages.push(page);
                    page = layout.new_page(true);
                }
                let laid_out = layout.make_line(page.number, line, true, page.next_line_y);
                page.lines.push(laid_out);
                page.occupied_lines = page.occupied_lines.saturating_add(1);
                page.next_line_y =
                    FixedPoint::from_raw(page.next_line_y.raw().saturating_add(line_advance));
            }
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
            }
        }
        layout.pages.push(page);
    }
    if include_title_page && include_body && layout.pages.last().is_some_and(|page| page.title_page)
    {
        layout.pages.push(layout.new_page(false));
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
