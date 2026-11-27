//! Backend-neutral Story Map scene and Body-caret projection.

use crate::{
    inputs::{resolve_effective_paragraph, resolve_effective_text_style, EffectiveParagraph, LayoutEnvironment, PaginationPolicy},
    wrap, Caret, FixedRect, LayoutComposition, LayoutError, LayoutErrorKind,
    LayoutLine, LocatedLayoutLine, ScreenplayLayout,
};
use scriptcraft_model::{
    formatting::FixedPoint,
    layout_inputs::{DocumentLeading, PageLayoutInputs},
    typography::{EffectiveTextStyle, FaceStyle, NEUTRAL_COURIER_PRIME_FAMILY},
    Position, Project, StableId, TextRun,
};
use std::sync::Arc;

/// Canonical source for the immediate Body paragraph following a scene heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneParagraphProjection {
    pub element_index: usize,
    pub owner_id: StableId,
    pub text: String,
    pub runs: Vec<TextRun>,
}

/// One exact scene boundary in Body order, with continuous page-fraction bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneProjection {
    pub element_index: usize,
    pub owner_id: StableId,
    pub heading: String,
    pub start_page: f64,
    pub end_page: f64,
    /// `None` means the immediately following Body paragraph does not exist
    /// before the next scene boundary; projection never searches farther ahead.
    pub paragraph: Option<SceneParagraphProjection>,
}

/// Immutable, same-document projection derived from an existing canonical Body layout.
#[derive(Debug, Clone)]
pub struct StoryMapProjection {
    pub document_identity: Arc<()>,
    pub scenes: Vec<SceneProjection>,
    pub caret_page: Option<f64>,
    pub final_body_line_bottom: Option<f64>,
}

/// Card-local, nonpaginating preview measurements. Line source ranges remain
/// canonical UTF-8 byte ranges; this value is presentation-only, not editable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMapPreviewLayout {
    pub title_rect: FixedRect,
    pub body_rect: FixedRect,
    pub title_lines: Vec<LayoutLine>,
    pub body_lines: Vec<LayoutLine>,
    pub title_content_height: FixedPoint,
    pub body_content_height: FixedPoint,
    pub total_content_height: FixedPoint,
    pub body_is_empty: bool,
}

/// Logical-point spacing used by the focused scene preview flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoryMapPreviewMetrics {
    pub horizontal_inset: FixedPoint,
    pub top_inset: FixedPoint,
    pub title_body_gap: FixedPoint,
    pub bottom_inset: FixedPoint,
    pub font_size: FixedPoint,
}

impl Default for StoryMapPreviewMetrics {
    fn default() -> Self {
        Self {
            horizontal_inset: FixedPoint::from_points(12),
            top_inset: FixedPoint::from_points(10),
            title_body_gap: FixedPoint::from_points(8),
            bottom_inset: FixedPoint::from_points(10),
            font_size: FixedPoint::from_points(12),
        }
    }
}

/// Measures only the retained scene heading and its immediate retained Body
/// paragraph using an already-prepared font environment. No paginator or
/// canonical layout lookup is performed.
pub fn measure_scene_preview(
    project: &Project,
    scene: &SceneProjection,
    environment: &LayoutEnvironment,
    outer_width: FixedPoint,
    metrics: StoryMapPreviewMetrics,
) -> Result<StoryMapPreviewLayout, LayoutError> {
    if outer_width.raw() <= 0 {
        return Err(LayoutError { kind: LayoutErrorKind::NoGeometry });
    }
    let inset = metrics.horizontal_inset.raw().max(0).min(outer_width.raw() / 2);
    let content_width = outer_width.raw().saturating_sub(inset.saturating_mul(2));
    if content_width <= 0 {
        return Err(LayoutError { kind: LayoutErrorKind::NoGeometry });
    }
    let heading_owner = project.elements.get(scene.element_index).ok_or(LayoutError {
        kind: LayoutErrorKind::NoGeometry,
    })?;
    if heading_owner.as_element().id != scene.owner_id {
        return Err(LayoutError { kind: LayoutErrorKind::NoGeometry });
    }
    let heading_key = &heading_owner.element_type;
    let mut local_environment = environment.clone();
    local_environment.page.document_leading = DocumentLeading::Normal;
    let title_style = courier_style(project, heading_key, &local_environment, metrics.font_size)?;
    let title_paragraph = local_paragraph(
        project,
        heading_key,
        &heading_owner.as_element().id,
        &[],
        &local_environment,
        content_width,
    )?;
    let title_rect = FixedRect {
        x: FixedPoint::from_raw(inset),
        y: FixedPoint::from_raw(metrics.top_inset.raw().max(0)),
        width: FixedPoint::from_raw(content_width),
        height: FixedPoint::default(),
    };
    let title_wrapped = wrap::wrap_resolved_for_layout(
        &scene.heading,
        scene.element_index,
        Some(heading_key.clone()),
        title_paragraph,
        &title_style,
        &local_environment,
        &[],
        true,
        0,
        0,
    )?;
    if title_wrapped
        .last()
        .is_some_and(|line| !line.final_visual_line)
    {
        return Err(LayoutError { kind: LayoutErrorKind::NoGeometry });
    }
    let mut title_lines = Vec::with_capacity(title_wrapped.len());
    let mut y = title_rect
        .y
        .raw()
        .checked_add(title_paragraph.space_before.raw().max(0))
        .ok_or(LayoutError { kind: LayoutErrorKind::NoGeometry })?;
    for wrapped in title_wrapped {
        let line = ScreenplayLayout::make_line(0, wrapped, false, FixedPoint::from_raw(y));
        y = y.checked_add(line.vertical_metrics.leading.raw()).ok_or(LayoutError {
            kind: LayoutErrorKind::NoGeometry,
        })?;
        title_lines.push(line);
    }
    let title_content_height = FixedPoint::from_raw(y.saturating_sub(title_rect.y.raw()));
    let title_body_top = y.checked_add(metrics.title_body_gap.raw().max(0)).ok_or(LayoutError {
        kind: LayoutErrorKind::NoGeometry,
    })?;
    let mut body_lines = Vec::new();
    let mut body_content_height = FixedPoint::default();
    let mut body_is_empty = true;
    if let Some(paragraph_source) = scene.paragraph.as_ref() {
        let body_owner = project.elements.get(paragraph_source.element_index).ok_or(LayoutError {
            kind: LayoutErrorKind::NoGeometry,
        })?;
        let body = body_owner.as_element();
        if body.id != paragraph_source.owner_id {
            return Err(LayoutError { kind: LayoutErrorKind::NoGeometry });
        }
        let body_style = courier_style(
            project,
            &body_owner.element_type,
            &local_environment,
            metrics.font_size,
        )?;
        let body_paragraph = local_paragraph(
            project,
            &body_owner.element_type,
            &body.id,
            &paragraph_source.runs,
            &local_environment,
            content_width,
        )?;
        let wrapped = wrap::wrap_resolved_for_layout(
            &paragraph_source.text,
            paragraph_source.element_index,
            Some(body_owner.element_type.clone()),
            body_paragraph,
            &body_style,
            &local_environment,
            &paragraph_source.runs,
            true,
            0,
            0,
        )?;
        if wrapped.last().is_some_and(|line| !line.final_visual_line) {
            return Err(LayoutError { kind: LayoutErrorKind::NoGeometry });
        }
        let mut body_y = title_body_top
            .checked_add(body_paragraph.space_before.raw().max(0))
            .ok_or(LayoutError { kind: LayoutErrorKind::NoGeometry })?;
        for wrapped in wrapped {
            let line = ScreenplayLayout::make_line(0, wrapped, false, FixedPoint::from_raw(body_y));
            body_y = body_y.checked_add(line.vertical_metrics.leading.raw()).ok_or(LayoutError {
                kind: LayoutErrorKind::NoGeometry,
            })?;
            body_lines.push(line);
        }
        body_content_height = FixedPoint::from_raw(body_y.saturating_sub(title_body_top));
        body_is_empty = paragraph_source.text.is_empty();
    }
    let body_rect = FixedRect {
        x: FixedPoint::from_raw(inset),
        y: FixedPoint::from_raw(title_body_top),
        width: FixedPoint::from_raw(content_width),
        height: body_content_height,
    };
    let total_height = title_body_top
        .checked_add(body_content_height.raw())
        .and_then(|height| height.checked_add(metrics.bottom_inset.raw().max(0)))
        .ok_or(LayoutError { kind: LayoutErrorKind::NoGeometry })?;
    Ok(StoryMapPreviewLayout {
        title_rect: FixedRect {
            height: title_content_height,
            ..title_rect
        },
        body_rect,
        title_lines,
        body_lines,
        title_content_height,
        body_content_height,
        total_content_height: FixedPoint::from_raw(total_height),
        body_is_empty,
    })
}

fn courier_style(
    project: &Project,
    key: &scriptcraft_model::ElementTypeKey,
    environment: &LayoutEnvironment,
    font_size: FixedPoint,
) -> Result<EffectiveTextStyle, LayoutError> {
    let source = resolve_effective_text_style(project, key, environment)?;
    let face_style = FaceStyle::from_flags(source.bold, source.italic);
    let resolved = environment
        .fonts
        .resolve_face_id(NEUTRAL_COURIER_PRIME_FAMILY, face_style, None)?
        .clone();
    Ok(EffectiveTextStyle::new(
        NEUTRAL_COURIER_PRIME_FAMILY,
        source.source_color,
        resolved,
        font_size,
        source.bold,
        source.italic,
        source.underline,
        source.all_caps,
        source.hidden_text,
    )?)
}

fn local_paragraph(
    project: &Project,
    key: &scriptcraft_model::ElementTypeKey,
    id: &StableId,
    runs: &[TextRun],
    environment: &LayoutEnvironment,
    content_width: i64,
) -> Result<EffectiveParagraph, LayoutError> {
    let mut paragraph = resolve_effective_paragraph(project, key, id, runs, environment)?;
    paragraph.left = FixedPoint::default();
    paragraph.right = FixedPoint::from_raw(content_width);
    paragraph.starts_new_page = false;
    paragraph.pagination = PaginationPolicy::Ordinary;
    Ok(paragraph)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryMapProjectionError {
    InvalidLayout,
    MissingSceneLine,
    InvalidGeometry,
}

/// Derives the complete projection from Project and one already-built canonical
/// Body layout. It does not build, modify, or paginate the layout.
pub fn project_story_map(
    project: &Project,
    body_focus: Position,
    layout: &ScreenplayLayout,
) -> Result<StoryMapProjection, StoryMapProjectionError> {
    if layout.composition != LayoutComposition::BodyEditor || layout.inputs.paper_height == 0 {
        return Err(StoryMapProjectionError::InvalidLayout);
    }
    let scene_indexes = project
        .elements
        .iter()
        .enumerate()
        .filter_map(|(index, owner)| {
            project
                .element_semantics(&owner.element_type)
                .is_some_and(|semantics| semantics.capabilities().scene_boundary)
                .then_some(index)
        })
        .collect::<Vec<_>>();

    let final_line = layout.last_body_line();
    let final_body_line_bottom = final_line
        .map(|located| {
            if !is_body_line(layout, located) || located.line.geometry.height.raw() <= 0 {
                return Err(StoryMapProjectionError::InvalidGeometry);
            }
            let bottom_raw = located
                .line
                .geometry
                .y
                .raw()
                .checked_add(located.line.geometry.height.raw())
                .ok_or(StoryMapProjectionError::InvalidGeometry)?;
            line_page_position(
                located,
                FixedPoint::from_raw(bottom_raw),
                &layout.layout_environment.page,
            )
        })
        .transpose()?;
    if !scene_indexes.is_empty() && final_body_line_bottom.is_none() {
        return Err(StoryMapProjectionError::InvalidGeometry);
    }

    let mut scene_starts = Vec::with_capacity(scene_indexes.len());
    for index in &scene_indexes {
        let located = layout
            .first_line_for_element(*index)
            .ok_or(StoryMapProjectionError::MissingSceneLine)?;
        if !is_body_line(layout, located) {
            return Err(StoryMapProjectionError::InvalidGeometry);
        }
        scene_starts.push(line_page_position(
            located,
            located.line.geometry.y,
            &layout.layout_environment.page,
        )?);
    }

    let mut scenes = Vec::with_capacity(scene_indexes.len());
    for (scene_offset, element_index) in scene_indexes.iter().copied().enumerate() {
        let start_page = scene_starts[scene_offset];
        let next_boundary = scene_indexes.get(scene_offset.saturating_add(1)).copied();
        let end_page = next_boundary
            .and_then(|_| scene_starts.get(scene_offset.saturating_add(1)).copied())
            .or(final_body_line_bottom)
            .ok_or(StoryMapProjectionError::InvalidGeometry)?;
        if !start_page.is_finite() || !end_page.is_finite() || end_page < start_page {
            return Err(StoryMapProjectionError::InvalidGeometry);
        }
        if scene_offset > 0 && start_page < scene_starts[scene_offset - 1] {
            return Err(StoryMapProjectionError::InvalidGeometry);
        }

        let owner = project
            .elements
            .get(element_index)
            .ok_or(StoryMapProjectionError::InvalidGeometry)?;
        let element = owner.as_element();
        let paragraph = element_index
            .checked_add(1)
            .filter(|next| next_boundary.is_none_or(|boundary| *next < boundary))
            .and_then(|next_index| project.elements.get(next_index).map(|paragraph_owner| {
                let paragraph = paragraph_owner.as_element();
                SceneParagraphProjection {
                    element_index: next_index,
                    owner_id: paragraph.id.clone(),
                    text: paragraph.text.clone(),
                    runs: paragraph.runs.clone(),
                }
            }));
        scenes.push(SceneProjection {
            element_index,
            owner_id: element.id.clone(),
            heading: element.text.clone(),
            start_page,
            end_page,
            paragraph,
        });
    }

    let caret_page = layout
        .line_for_caret_with_diagnostics(
            &Caret {
                element: body_focus.element,
                offset: body_focus.offset,
            },
            &mut crate::InteractionQueryDiagnostics::default(),
        )
        .ok()
        .filter(|located| is_body_line(layout, *located))
        .and_then(|located| {
            line_page_position(
                located,
                located.line.geometry.y,
                &layout.layout_environment.page,
            )
            .ok()
        });

    Ok(StoryMapProjection {
        document_identity: project.layout_identity().clone(),
        scenes,
        caret_page,
        final_body_line_bottom,
    })
}

impl StoryMapProjection {
    /// Re-derives the confirmed presentation domain without touching Body layout.
    pub fn displayed_domain_pages(&self, target_pages: u32) -> f64 {
        let target = f64::from(target_pages);
        self.final_body_line_bottom
            .filter(|bottom| bottom.is_finite() && *bottom >= 0.0)
            .map_or(target, |bottom| target.max(bottom.ceil()))
    }
}

fn is_body_line(layout: &ScreenplayLayout, located: LocatedLayoutLine<'_>) -> bool {
    located.line.page == located.page_index
        && layout
            .pages
            .get(located.page_index)
            .is_some_and(|page| !page.title_page)
}

fn line_page_position(
    located: LocatedLayoutLine<'_>,
    y: FixedPoint,
    inputs: &PageLayoutInputs,
) -> Result<f64, StoryMapProjectionError> {
    let page = u64::try_from(located.page_index)
        .map_err(|_| StoryMapProjectionError::InvalidGeometry)?;
    // All valid Story Map documents are far below f64's exact-integer limit;
    // reject rather than silently lose a page index if an invalid layout arrives.
    if page > (1_u64 << 53) {
        return Err(StoryMapProjectionError::InvalidGeometry);
    }
    let paper_height_raw = i64::from(inputs.paper_height)
        .checked_mul(FixedPoint::SCALE)
        .ok_or(StoryMapProjectionError::InvalidGeometry)?;
    let margin_top_raw = i64::from(inputs.margin_top)
        .checked_mul(FixedPoint::SCALE)
        .ok_or(StoryMapProjectionError::InvalidGeometry)?;
    let margin_bottom_raw = i64::from(inputs.margin_bottom)
        .checked_mul(FixedPoint::SCALE)
        .ok_or(StoryMapProjectionError::InvalidGeometry)?;
    let text_top_raw = margin_top_raw;
    let text_bottom_raw = paper_height_raw
        .checked_sub(margin_bottom_raw)
        .ok_or(StoryMapProjectionError::InvalidGeometry)?;
    let text_height_raw = text_bottom_raw
        .checked_sub(text_top_raw)
        .ok_or(StoryMapProjectionError::InvalidGeometry)?;
    if paper_height_raw <= 0 || text_height_raw <= 0 {
        return Err(StoryMapProjectionError::InvalidGeometry);
    }
    let clamped_y_raw = y.raw().clamp(text_top_raw, text_bottom_raw);
    let text_relative_y_raw = clamped_y_raw
        .checked_sub(text_top_raw)
        .ok_or(StoryMapProjectionError::InvalidGeometry)?;
    let fraction = text_relative_y_raw as f64 / text_height_raw as f64;
    let page_position = page as f64 + fraction;
    if page_position.is_finite() {
        Ok(page_position)
    } else {
        Err(StoryMapProjectionError::InvalidGeometry)
    }
}
