//! One transient, read-only foreground scene preview.

use eframe::egui::{self, Align, Area, Id, Layout, Order, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use scriptcraft_layout::{
    measure_scene_preview, LayoutEnvironment, SceneProjection, StoryMapPreviewLayout,
    StoryMapPreviewMetrics, StoryMapProjection,
};
use scriptcraft_model::{formatting::FixedPoint, DocumentElementDefinitions, Project, StableId};
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    chrome::theme::{PagePalette, StoryMapPalette},
    presentation::{
        document_font::PreparedDocumentFonts,
        geometry,
        span_paint::resolve_span_paint,
    },
};

use super::constants::{
    SCENE_PREVIEW_BORDER_WIDTH, SCENE_PREVIEW_BOTTOM_INSET, SCENE_PREVIEW_CORNER_RADIUS,
    SCENE_PREVIEW_FONT_SIZE, SCENE_PREVIEW_HORIZONTAL_INSET, SCENE_PREVIEW_MAX_HEIGHT,
    SCENE_PREVIEW_MAX_WIDTH, SCENE_PREVIEW_NOTCH_CORNER_SAFE_INSET,
    SCENE_PREVIEW_NOTCH_HEIGHT, SCENE_PREVIEW_NOTCH_WIDTH, SCENE_PREVIEW_TITLE_BODY_GAP,
    SCENE_PREVIEW_TOP_INSET,
};

#[derive(Debug, Clone)]
struct OpenScene {
    document_identity: Arc<()>,
    element_index: usize,
    owner_id: StableId,
    anchor: Rect,
}

#[derive(Debug, Clone)]
struct PreviewKey {
    document_identity: Arc<()>,
    scene: SceneProjection,
    heading_type: scriptcraft_model::ElementTypeKey,
    paragraph_type: Option<scriptcraft_model::ElementTypeKey>,
    width: FixedPoint,
    definitions: DocumentElementDefinitions,
    direct_paragraph_overrides:
        BTreeMap<StableId, scriptcraft_model::formatting::SourceParagraphOverrides>,
    environment: LayoutEnvironment,
}

impl PreviewKey {
    fn same_inputs(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document_identity, &other.document_identity)
            && self.scene == other.scene
            && self.heading_type == other.heading_type
            && self.paragraph_type == other.paragraph_type
            && self.width == other.width
            && self.definitions == other.definitions
            && self.direct_paragraph_overrides == other.direct_paragraph_overrides
            && self.environment == other.environment
    }
}

#[derive(Debug, Clone)]
struct MeasuredPreview {
    key: PreviewKey,
    layout: StoryMapPreviewLayout,
}

#[derive(Debug, Clone, Copy)]
struct PopupPlacement {
    body: Rect,
    anchor: Rect,
    notch_base_center: f32,
    area_origin: Pos2,
    area_size: Vec2,
}

struct PreviewPaintOp {
    position: Pos2,
    galley: Arc<egui::Galley>,
    color: egui::Color32,
    decorations: Vec<Rect>,
}

/// The single desktop Story Map preview owner. Source text always comes from
/// the retained projection snapshot; only its one measured flow is cached.
#[derive(Debug, Default)]
pub(crate) struct StoryMapPreviewState {
    open: Option<OpenScene>,
    measured: Option<MeasuredPreview>,
    failed_key: Option<PreviewKey>,
    placement: Option<PopupPlacement>,
    flow_failure_pending: bool,
}

impl StoryMapPreviewState {
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub(crate) fn take_flow_failure(&mut self) -> bool {
        std::mem::take(&mut self.flow_failure_pending)
    }

    pub(crate) fn open_scene(
        &mut self,
        document_identity: Arc<()>,
        element_index: usize,
        owner_id: StableId,
        anchor: Rect,
    ) {
        let same_scene = self.open.as_ref().is_some_and(|open| {
            Arc::ptr_eq(&open.document_identity, &document_identity)
                && open.element_index == element_index
                && open.owner_id == owner_id
        });
        if !same_scene {
            self.measured = None;
            self.failed_key = None;
            self.placement = None;
        }
        self.open = Some(OpenScene {
            document_identity,
            element_index,
            owner_id,
            anchor,
        });
    }

    pub(crate) fn close(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn contains_pointer(&self, point: Pos2) -> bool {
        let Some(placement) = self.placement else { return false };
        placement.body.contains(point) || point_in_notch(point, placement)
    }

    pub(crate) fn resolve<'a>(
        &mut self,
        project: &Project,
        snapshot: Option<&'a StoryMapProjection>,
        anchor_for_scene: impl FnOnce(&'a SceneProjection) -> Option<Rect>,
    ) -> Option<(&'a SceneProjection, Rect)> {
        let Some(open) = self.open.as_ref() else { return None };
        let Some(snapshot) = snapshot.filter(|snapshot| {
            Arc::ptr_eq(&snapshot.document_identity, &open.document_identity)
        }) else {
            self.close();
            return None;
        };
        let Some(scene) = snapshot.scenes.iter().find(|scene| {
            scene.element_index == open.element_index && scene.owner_id == open.owner_id
        }) else {
            self.close();
            return None;
        };
        if !source_exists(project, scene)
            || !project
                .elements
                .get(open.element_index)
                .and_then(|owner| project.element_semantics(&owner.element_type))
                .is_some_and(|semantics| semantics.capabilities().scene_boundary)
        {
            self.close();
            return None;
        }
        let Some(anchor) = anchor_for_scene(scene) else {
            self.close();
            return None;
        };
        if let Some(open) = self.open.as_mut() {
            open.anchor = anchor;
        }
        Some((scene, anchor))
    }

    pub(crate) fn scene_for_snapshot<'a>(
        &mut self,
        project: &Project,
        snapshot: Option<&'a StoryMapProjection>,
    ) -> Option<(&'a SceneProjection, Arc<()>, Rect)> {
        let Some(open) = self.open.as_ref() else { return None };
        let Some(snapshot) = snapshot.filter(|snapshot| {
            Arc::ptr_eq(&snapshot.document_identity, &open.document_identity)
        }) else {
            self.close();
            return None;
        };
        let Some(scene) = snapshot.scenes.iter().find(|scene| {
            scene.element_index == open.element_index && scene.owner_id == open.owner_id
        }) else {
            self.close();
            return None;
        };
        if !source_exists(project, scene)
            || !project
                .elements
                .get(open.element_index)
                .and_then(|owner| project.element_semantics(&owner.element_type))
                .is_some_and(|semantics| semantics.capabilities().scene_boundary)
        {
            self.close();
            return None;
        }
        Some((scene, open.document_identity.clone(), open.anchor))
    }

    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        root_bounds: Rect,
        document_identity: Arc<()>,
        scene: &SceneProjection,
        anchor: Rect,
        project: &Project,
        prepared_fonts: Option<&PreparedDocumentFonts>,
        story_map_palette: StoryMapPalette,
        page_palette: PagePalette,
    ) {
        let Some(open) = self.open.as_ref().filter(|open| {
            Arc::ptr_eq(&open.document_identity, &document_identity)
                && open.element_index == scene.element_index
                && open.owner_id == scene.owner_id
        }) else {
            self.close();
            return;
        };
        let _ = open;
        if !root_bounds.is_positive()
            || !finite_rect(root_bounds)
            || !anchor.is_positive()
            || !finite_rect(anchor)
        {
            self.close();
            return;
        }
        // The prior placement has already served outside-press hit testing in
        // the root composition. Publish a new placement only after all current
        // measurement and geometry gates succeed.
        self.placement = None;
        let max_width = root_bounds.width().min(SCENE_PREVIEW_MAX_WIDTH).floor();
        let width = FixedPoint::from_points(max_width as i64);
        if max_width <= SCENE_PREVIEW_NOTCH_CORNER_SAFE_INSET * 2.0 {
            self.close();
            return;
        }
        let Some(prepared) = prepared_fonts else {
            return;
        };
        let environment = LayoutEnvironment::new(
            project.session_layout_inputs().page_only(),
            prepared.environment.clone(),
        );
        let Some(heading_owner) = project.elements.get(scene.element_index) else {
            self.close();
            return;
        };
        let paragraph_type = scene.paragraph.as_ref().and_then(|paragraph| {
            project.elements.get(paragraph.element_index).map(|owner| owner.element_type.clone())
        });
        let key = PreviewKey {
            document_identity: document_identity.clone(),
            scene: scene.clone(),
            heading_type: heading_owner.element_type.clone(),
            paragraph_type,
            width,
            definitions: project.element_definitions().clone(),
            direct_paragraph_overrides: project.direct_paragraph_overrides.clone(),
            environment,
        };
        let cache_hit = self
            .measured
            .as_ref()
            .is_some_and(|cached| cached.key.same_inputs(&key));
        let failed_hit = self.failed_key.as_ref().is_some_and(|failed| failed.same_inputs(&key));
        if !cache_hit && !failed_hit {
            let metrics = StoryMapPreviewMetrics {
                horizontal_inset: FixedPoint::from_points(SCENE_PREVIEW_HORIZONTAL_INSET as i64),
                top_inset: FixedPoint::from_points(SCENE_PREVIEW_TOP_INSET as i64),
                title_body_gap: FixedPoint::from_points(SCENE_PREVIEW_TITLE_BODY_GAP as i64),
                bottom_inset: FixedPoint::from_points(SCENE_PREVIEW_BOTTOM_INSET as i64),
                font_size: FixedPoint::from_points(SCENE_PREVIEW_FONT_SIZE as i64),
            };
            if let Ok(layout) = measure_scene_preview(
                project,
                scene,
                &key.environment,
                width,
                metrics,
            ) {
                // Commit source and both measured regions as one cache value.
                self.measured = Some(MeasuredPreview { key, layout });
                self.failed_key = None;
            } else {
                // A same-scene last-good flow may remain visible; remember the
                // failed exact inputs to avoid a frame-by-frame retry loop.
                self.failed_key = Some(key);
                self.flow_failure_pending = true;
            }
        }
        let Some(measured) = self.measured.as_ref().filter(|cached| {
            Arc::ptr_eq(&cached.key.document_identity, &document_identity)
                && cached.key.scene.element_index == scene.element_index
                && cached.key.scene.owner_id == scene.owner_id
        }) else {
            // Font/flow failure omits the text rather than fabricating source.
            return;
        };
        let body_width = measured.key.width.to_points_f32();
        if !body_width.is_finite()
            || body_width <= 0.0
            || body_width > root_bounds.width()
            || body_width > SCENE_PREVIEW_MAX_WIDTH
        {
            // A failed constrained reflow must not paint the prior wider value
            // outside the newly available root bounds.
            return;
        }
        let body_x = (anchor.center().x - body_width * 0.5)
            .clamp(root_bounds.left(), root_bounds.right() - body_width);
        let body_y = anchor.bottom() + SCENE_PREVIEW_NOTCH_HEIGHT;
        let available_height = (root_bounds.bottom() - body_y).max(0.0);
        let total_measured = measured.layout.total_content_height.to_points_f32();
        if !total_measured.is_finite() {
            self.close();
            return;
        }
        let body_height = total_measured
            .max(36.0)
            .min(SCENE_PREVIEW_MAX_HEIGHT)
            .min(available_height);
        if !body_x.is_finite()
            || !body_y.is_finite()
            || !body_height.is_finite()
            || body_height < 36.0
        {
            self.close();
            return;
        }
        let body = Rect::from_min_size(Pos2::new(body_x, body_y), Vec2::new(body_width, body_height));
        let notch_safe_left = body.left() + SCENE_PREVIEW_NOTCH_CORNER_SAFE_INSET;
        let notch_safe_right = body.right() - SCENE_PREVIEW_NOTCH_CORNER_SAFE_INSET;
        if notch_safe_right < notch_safe_left {
            self.close();
            return;
        }
        let notch_base_center = anchor.center().x.clamp(notch_safe_left, notch_safe_right);
        let notch_left = notch_base_center - SCENE_PREVIEW_NOTCH_WIDTH * 0.5;
        let notch_right = notch_base_center + SCENE_PREVIEW_NOTCH_WIDTH * 0.5;
        let area_left = body.left().min(notch_left).min(anchor.center().x);
        let area_right = body.right().max(notch_right).max(anchor.center().x);
        let area_origin = Pos2::new(area_left, body.top() - SCENE_PREVIEW_NOTCH_HEIGHT);
        let area_size = Vec2::new(area_right - area_left, body.height() + SCENE_PREVIEW_NOTCH_HEIGHT);
        let placement = PopupPlacement { body, anchor, notch_base_center, area_origin, area_size };
        self.placement = Some(placement);

        let response = Area::new(Id::new("story-map-scene-preview"))
            .order(Order::Foreground)
            .fixed_pos(area_origin)
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_min_size(area_size);
                ui.set_max_size(area_size);
                let _ = ui.allocate_exact_size(area_size, Sense::hover());
                paint_frame(ui, placement, story_map_palette);
                paint_scrolling_content(
                    ui,
                    placement,
                    &measured.layout,
                    PagePalette { text: story_map_palette.scene_preview_text, ..page_palette },
                    prepared_fonts,
                    scene.element_index,
                    scene.owner_id.clone(),
                )
            });
        if !response.inner {
            self.flow_failure_pending = true;
        }
    }
}

fn paint_frame(ui: &egui::Ui, placement: PopupPlacement, palette: StoryMapPalette) {
    let painter = ui.painter().with_clip_rect(ui.ctx().content_rect());
    let body = placement.body;
    let base_left = placement.notch_base_center - SCENE_PREVIEW_NOTCH_WIDTH * 0.5;
    let base_right = placement.notch_base_center + SCENE_PREVIEW_NOTCH_WIDTH * 0.5;
    let tip = Pos2::new(placement.anchor.center().x, placement.anchor.bottom());
    let left = Pos2::new(base_left, body.top());
    let right = Pos2::new(base_right, body.top());
    painter.rect_filled(body, SCENE_PREVIEW_CORNER_RADIUS, palette.scene_preview_body);
    painter.rect_stroke(
        body,
        SCENE_PREVIEW_CORNER_RADIUS,
        Stroke::new(SCENE_PREVIEW_BORDER_WIDTH, palette.scene_preview_border),
        egui::StrokeKind::Inside,
    );
    painter.add(Shape::convex_polygon(
        vec![tip, right, left],
        palette.scene_preview_notch,
        Stroke::NONE,
    ));
    painter.line_segment(
        [left, right],
        Stroke::new(SCENE_PREVIEW_BORDER_WIDTH + 1.0, palette.scene_preview_body),
    );
    painter.line_segment([tip, left], Stroke::new(SCENE_PREVIEW_BORDER_WIDTH, palette.scene_preview_border));
    painter.line_segment([tip, right], Stroke::new(SCENE_PREVIEW_BORDER_WIDTH, palette.scene_preview_border));
    painter.line_segment(
        [Pos2::new(body.left(), body.top()), left],
        Stroke::new(SCENE_PREVIEW_BORDER_WIDTH, palette.scene_preview_border),
    );
    painter.line_segment(
        [right, Pos2::new(body.right(), body.top())],
        Stroke::new(SCENE_PREVIEW_BORDER_WIDTH, palette.scene_preview_border),
    );
}

fn paint_scrolling_content(
    ui: &mut egui::Ui,
    placement: PopupPlacement,
    layout: &StoryMapPreviewLayout,
    page_palette: PagePalette,
    prepared: Option<&PreparedDocumentFonts>,
    scene_index: usize,
    scene_identity: StableId,
) -> bool {
    let inner = placement.body.shrink2(Vec2::new(
        SCENE_PREVIEW_HORIZONTAL_INSET,
        SCENE_PREVIEW_BORDER_WIDTH,
    ));
    if !inner.is_positive() {
        return false;
    }
    let Some(prepared) = prepared else { return false };
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt(("story-map-preview-scroll", scene_index, scene_identity))
                .auto_shrink([false, false])
                .max_height(inner.height())
                .show(ui, |ui| {
                    ui.set_min_width(inner.width());
                    let content_height = layout.total_content_height.to_points_f32().max(inner.height());
                    let (content_rect, _) = ui.allocate_exact_size(
                        Vec2::new(inner.width(), content_height),
                        Sense::hover(),
                    );
                    let painter = ui.painter().with_clip_rect(inner);
                    let origin = content_rect.min;
                    let title_clip = fixed_rect_at(origin, layout.title_rect).intersect(inner);
                    let body_clip = fixed_rect_at(origin, layout.body_rect).intersect(inner);
                    let title_ops = prepare_lines(
                        &painter.with_clip_rect(title_clip),
                        origin,
                        &layout.title_lines,
                        page_palette,
                        prepared,
                    );
                    let body_ops = prepare_lines(
                        &painter.with_clip_rect(body_clip),
                        origin,
                        &layout.body_lines,
                        page_palette,
                        prepared,
                    );
                    if let (Some(title_ops), Some(body_ops)) = (title_ops, body_ops) {
                        paint_operations(&painter.with_clip_rect(title_clip), title_ops);
                        paint_operations(&painter.with_clip_rect(body_clip), body_ops);
                        true
                    } else {
                        false
                    }
                })
                .inner
        },
    ).inner
}

fn source_exists(project: &Project, scene: &SceneProjection) -> bool {
    let Some(owner) = project.elements.get(scene.element_index) else { return false };
    if owner.as_element().id != scene.owner_id {
        return false;
    }
    scene.paragraph.as_ref().is_none_or(|paragraph| {
        project
            .elements
            .get(paragraph.element_index)
            .is_some_and(|owner| owner.as_element().id == paragraph.owner_id)
    })
}

fn fixed_rect_at(origin: Pos2, rect: scriptcraft_layout::FixedRect) -> Rect {
    Rect::from_min_size(
        origin + egui::vec2(rect.x.to_points_f32(), rect.y.to_points_f32()),
        Vec2::new(rect.width.to_points_f32(), rect.height.to_points_f32()),
    )
}

fn prepare_lines(
    painter: &egui::Painter,
    origin: Pos2,
    lines: &[scriptcraft_layout::LayoutLine],
    palette: PagePalette,
    prepared: &PreparedDocumentFonts,
) -> Option<Vec<PreviewPaintOp>> {
    let mut operations = Vec::new();
    for line in lines {
        let Ok(projection) = geometry::project_line(line, 1.0) else { return None };
        for span in &projection.spans {
            let resolved = resolve_span_paint(
                &span.core.resolved_text_style,
                span.core.resolved_text_style.size.to_points_f32(),
                palette,
                prepared,
            )?;
            let color = resolved.color;
            let galley = geometry::canonical_singleline_galley(
                painter,
                &span.core.display_text,
                &resolved.font,
                &span.display_boundary_offsets,
                color,
                projection.line_box_height,
                projection.baseline_y - projection.line_box_top,
            ).ok()?;
            let mut decorations = Vec::new();
            for decoration in [span.core.underline_geometry, span.core.strikethrough_geometry]
                .into_iter()
                .flatten()
            {
                let rect = geometry::project_decoration_rect(
                    span,
                    decoration,
                    projection.baseline_y,
                    1.0,
                ).ok()?;
                if rect.is_positive() {
                    decorations.push(rect.translate(origin.to_vec2()));
                }
            }
            operations.push(PreviewPaintOp {
                position: origin + egui::vec2(span.origin_x, projection.line_box_top),
                galley,
                color,
                decorations,
            });
        }
    }
    Some(operations)
}

fn paint_operations(painter: &egui::Painter, operations: Vec<PreviewPaintOp>) {
    for operation in operations {
        painter.galley(operation.position, operation.galley, operation.color);
        for decoration in operation.decorations {
            painter.rect_filled(decoration, 0.0, operation.color);
        }
    }
}

fn point_in_notch(point: Pos2, placement: PopupPlacement) -> bool {
    let left = placement.notch_base_center - SCENE_PREVIEW_NOTCH_WIDTH * 0.5;
    let right = placement.notch_base_center + SCENE_PREVIEW_NOTCH_WIDTH * 0.5;
    let tip = Pos2::new(placement.anchor.center().x, placement.anchor.bottom());
    let base_y = placement.body.top();
    if point.y < tip.y || point.y > base_y || base_y <= tip.y {
        return false;
    }
    let fraction = (point.y - tip.y) / (base_y - tip.y);
    let half_width = (SCENE_PREVIEW_NOTCH_WIDTH * 0.5) * fraction;
    let center = tip.x + (placement.notch_base_center - tip.x) * fraction;
    point.x >= center - half_width && point.x <= center + half_width
        && point.x >= left.min(right) && point.x <= left.max(right)
}

fn finite_rect(rect: Rect) -> bool {
    rect.min.x.is_finite() && rect.min.y.is_finite() && rect.max.x.is_finite() && rect.max.y.is_finite()
}
