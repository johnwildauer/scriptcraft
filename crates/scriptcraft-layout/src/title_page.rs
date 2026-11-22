use super::*;
use super::{
    cache::{add_elapsed_nanos, ElementWrapCache},
    inputs::EffectiveGeometryMode,
};
use crate::geometry::LayoutError;
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::Project;
use std::time::Instant;

#[allow(clippy::too_many_arguments)]
pub(super) fn compose_title_pages(
    layout: &mut ScreenplayLayout,
    project: &Project,
    purpose: LayoutPurpose,
    include_title_page: bool,
    include_body: bool,
    wrap_cache: &mut Option<&mut ElementWrapCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) -> Result<(), LayoutError> {
    if include_title_page && !project.title_page.is_empty() {
        let environment = layout.layout_environment.clone();
        compose_resolved_title_pages(
            layout,
            project,
            purpose,
            &environment,
            wrap_cache,
            diagnostics,
        )?;
    }
    if include_title_page && include_body && layout.pages.last().is_some_and(|page| page.title_page)
    {
        layout.pages.push(layout.new_page(false));
    }
    Ok(())
}

fn compose_resolved_title_pages(
    layout: &mut ScreenplayLayout,
    project: &Project,
    purpose: LayoutPurpose,
    environment: &super::inputs::LayoutEnvironment,
    wrap_cache: &mut Option<&mut ElementWrapCache>,
    diagnostics: &mut Option<&mut LayoutBuildDiagnostics>,
) -> Result<(), LayoutError> {
    let mut page = layout.new_page(true);
    for (index, title) in project.title_page.iter().enumerate() {
        let (paragraph, resolved_style) =
            super::inputs::resolve_effective_title(project, title, environment)?;
        if purpose == LayoutPurpose::Print && resolved_style.hidden_text {
            continue;
        }
        if paragraph.starts_new_page && !page.lines.is_empty() {
            layout.pages.push(page);
            page = layout.new_page(true);
        }
        let element_type = title.element_type.clone();
        let lines = if let Some(cache) = wrap_cache.as_deref_mut() {
            cache.wrap_resolved(
                &title.id,
                title.element_type.as_ref(),
                &title.text,
                index,
                element_type,
                paragraph,
                resolved_style,
                &title.runs,
                purpose,
                true,
                EffectiveGeometryMode::Sequential,
                environment,
                diagnostics.as_deref_mut(),
            )?
        } else {
            let wrap_started = diagnostics.as_ref().map(|_| Instant::now());
            let lines = wrap::wrap_resolved_for_layout(
                &title.text,
                index,
                element_type,
                paragraph,
                &resolved_style,
                environment,
                &title.runs,
                true,
                0,
                0,
            )?;
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                add_elapsed_nanos(&mut diagnostics.element_wrap_nanos, wrap_started);
            }
            lines
        };
        if !page.lines.is_empty() {
            page.next_line_top = FixedPoint::from_raw(
                page.next_line_top
                    .raw()
                    .saturating_add(paragraph.space_before.raw().max(0)),
            );
        }
        let placement_started = diagnostics.as_ref().map(|_| Instant::now());
        for line in lines {
            let line_leading = line.vertical_metrics.leading.raw();
            let content_bottom = FixedPoint::from_points(i64::from(
                layout
                    .inputs
                    .paper_height
                    .saturating_sub(layout.inputs.margin_bottom),
            ));
            if !page.lines.is_empty()
                && page.next_line_top.raw().saturating_add(line_leading) > content_bottom.raw()
            {
                layout.pages.push(page);
                page = layout.new_page(true);
            }
            let laid_out = layout.make_line(page.number, line, true, page.next_line_top);
            let line_leading = laid_out.vertical_metrics.leading.raw();
            let mut laid_out = laid_out;
            laid_out.oversized |= laid_out.geometry.bottom() > content_bottom;
            page.lines.push(laid_out);
            page.occupied_lines = page.occupied_lines.saturating_add(1);
            page.next_line_top = FixedPoint::from_raw(page.next_line_top.raw().saturating_add(line_leading));
        }
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            add_elapsed_nanos(&mut diagnostics.placement_nanos, placement_started);
        }
    }
    layout.pages.push(page);
    Ok(())
}
