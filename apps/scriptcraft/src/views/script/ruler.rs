//! Pure geometry and paper-width tick planning for the Script View ruler.
//!
//! The tick plan is the only cached ruler data. Paragraph markers are derived
//! from the current canonical first line so direct formatting and active
//! dual-dialogue geometry are reflected without a second resolver.

use eframe::egui::Color32;
use scriptcraft_layout::{LocatedLayoutLine, ScreenplayLayout, POINTS_PER_INCH};
use scriptcraft_model::formatting::FixedPoint;

pub(super) const RULER_BAR_HEIGHT: f32 = 12.0;
pub(super) const RULER_MARKER_OVERFLOW: f32 = 5.0;
pub(super) const RULER_ALLOCATED_HEIGHT: f32 = RULER_BAR_HEIGHT + 2.0 * RULER_MARKER_OVERFLOW;

pub(super) const RULER_TICK_WIDTH: f32 = 2.0;
pub(super) const RULER_EIGHTH_TICK_HEIGHT: f32 = 3.0;
pub(super) const RULER_HALF_TICK_HEIGHT: f32 = 6.0;
pub(super) const RULER_LABEL_FONT_SIZE: f32 = 10.0;
pub(super) const RULER_LABEL_Y_OFFSET: f32 = 0.0;

pub(super) const RULER_MARKER_WIDTH: f32 = 7.0;
pub(super) const RULER_MARKER_BODY_HEIGHT: f32 = 4.0;
pub(super) const RULER_MARKER_TIP_HEIGHT: f32 = 4.0;
pub(super) const RULER_MARKER_STEM_WIDTH: f32 = 1.0;
pub(super) const RULER_FIRST_LINE_CENTER_OFFSET: f32 = 8.0;
pub(super) const RULER_BOTTOM_MARKER_CENTER_OFFSET: f32 = 10.0;
pub(super) const RULER_LEFT_FOOT_HEIGHT: f32 = 4.0;

pub(super) const RULER_DAY_TICK_COLOR: Color32 = Color32::from_gray(112);
pub(super) const RULER_DAY_LABEL_COLOR: Color32 = Color32::from_gray(80);
pub(super) const RULER_DAY_MARKER_FILL: Color32 = Color32::from_gray(255);
pub(super) const RULER_DAY_MARKER_OUTLINE: Color32 = Color32::from_gray(64);
pub(super) const RULER_DARK_TICK_COLOR: Color32 = Color32::from_gray(144);
pub(super) const RULER_DARK_LABEL_COLOR: Color32 = Color32::from_gray(224);
pub(super) const RULER_DARK_MARKER_FILL: Color32 = Color32::from_gray(238);
pub(super) const RULER_DARK_MARKER_OUTLINE: Color32 = Color32::from_gray(20);

pub(super) const RULER_TICK_STEP_POINTS: u32 = POINTS_PER_INCH / 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RulerTickKind {
    Eighth,
    Half,
    WholeInch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RulerTick {
    pub(super) document_x: u32,
    pub(super) kind: RulerTickKind,
    pub(super) label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RulerTickPlan {
    pub(super) paper_width: u32,
    pub(super) ticks: Vec<RulerTick>,
}

impl RulerTickPlan {
    fn for_paper_width(paper_width: u32) -> Self {
        let mut ticks = Vec::new();
        let mut document_x = 0;
        let mut ordinal = 0_u32;

        loop {
            let kind = if ordinal % 8 == 0 {
                RulerTickKind::WholeInch
            } else if ordinal % 4 == 0 {
                RulerTickKind::Half
            } else {
                RulerTickKind::Eighth
            };
            let label = (ordinal != 0 && kind == RulerTickKind::WholeInch)
                .then(|| (ordinal / 8).to_string());
            if document_x != 0 && document_x != paper_width {
                ticks.push(RulerTick {
                    document_x,
                    kind,
                    label,
                });
            }

            if paper_width - document_x < RULER_TICK_STEP_POINTS {
                break;
            }
            document_x += RULER_TICK_STEP_POINTS;
            ordinal += 1;
        }

        Self { paper_width, ticks }
    }
}

/// Document-space marker positions for one paragraph's first canonical line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RulerMarkerPositions {
    pub(super) first_line_x: FixedPoint,
    pub(super) left_x: FixedPoint,
    pub(super) right_x: FixedPoint,
}

/// Uses resolved canonical geometry, preserving first-line indents, direct
/// paragraph overrides, and active dual-dialogue column bounds.
pub(super) fn marker_positions(line: &LocatedLayoutLine<'_>) -> RulerMarkerPositions {
    let bounds = line.line.paragraph_bounds;
    RulerMarkerPositions {
        first_line_x: line.line.unaligned_line_left,
        left_x: bounds.x,
        right_x: FixedPoint::from_raw(bounds.x.raw().saturating_add(bounds.width.raw())),
    }
}

/// Projects document X using only the page-left coordinate and presentation
/// scale supplied by the current renderer frame.
pub(super) fn document_x_to_screen(page_left: f32, scale: f32, document_x: FixedPoint) -> f32 {
    page_left + document_x.to_points_f32() * scale
}

#[derive(Debug, Default)]
pub(crate) struct RulerState {
    tick_plan: Option<RulerTickPlan>,
}

impl RulerState {
    /// Returns the plan for this effective width, rebuilding only on a width
    /// change. Zoom, origin, focus, palette, and visibility are deliberately
    /// absent from this cache key.
    pub(super) fn tick_plan(&mut self, paper_width: u32) -> &RulerTickPlan {
        if self
            .tick_plan
            .as_ref()
            .map_or(true, |plan| plan.paper_width != paper_width)
        {
            self.tick_plan = Some(RulerTickPlan::for_paper_width(paper_width));
        }
        self.tick_plan
            .get_or_insert_with(|| RulerTickPlan::for_paper_width(paper_width))
    }
}

fn ruler_bar_rect(
    strip: eframe::egui::Rect,
    page_left: f32,
    page_width: f32,
) -> eframe::egui::Rect {
    let top = strip.top() + RULER_MARKER_OVERFLOW;
    eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(page_left, top),
        eframe::egui::vec2(page_width, RULER_BAR_HEIGHT),
    )
}

/// Paints from the completed frame's page geometry. The strip has no response
/// or interaction state; geometry comes from the actual page origin and scale.
pub(super) fn paint_ruler(
    ui: &eframe::egui::Ui,
    strip: eframe::egui::Rect,
    page_left: f32,
    page_width: f32,
    paper_color: Color32,
    scale: f32,
    paper_width: u32,
    focus_element: usize,
    layout: &ScreenplayLayout,
    state: &mut RulerState,
    dark_mode: bool,
) {
    use eframe::egui::{self, Align2, Stroke, Vec2};

    let strip_painter = ui.painter().with_clip_rect(strip);
    let bar_rect = ruler_bar_rect(strip, page_left, page_width);
    if bar_rect.is_positive() {
        strip_painter.rect_filled(bar_rect, 0.0, paper_color);
    }
    let tick_clip_rect = strip.intersect(bar_rect);
    let tick_painter = tick_clip_rect
        .is_positive()
        .then(|| strip_painter.with_clip_rect(tick_clip_rect));
    let pixels_per_point = ui.ctx().pixels_per_point().max(f32::MIN_POSITIVE);
    let (tick_color, label_color, marker_fill, marker_outline) = if dark_mode {
        (
            RULER_DARK_TICK_COLOR,
            RULER_DARK_LABEL_COLOR,
            RULER_DARK_MARKER_FILL,
            RULER_DARK_MARKER_OUTLINE,
        )
    } else {
        (
            RULER_DAY_TICK_COLOR,
            RULER_DAY_LABEL_COLOR,
            RULER_DAY_MARKER_FILL,
            RULER_DAY_MARKER_OUTLINE,
        )
    };
    let page_right = page_left + page_width;
    let bar_center_y = bar_rect.center().y;
    let label_y = bar_center_y + RULER_LABEL_Y_OFFSET;
    let plan = state.tick_plan(paper_width);

    if let Some(tick_painter) = tick_painter.as_ref() {
        for tick in &plan.ticks {
            let document_x = FixedPoint::from_points(i64::from(tick.document_x));
            let x = document_x_to_screen(page_left, scale, document_x);
            if x < page_left || x > page_right {
                continue;
            }
            let height = match tick.kind {
                RulerTickKind::Eighth => Some(RULER_EIGHTH_TICK_HEIGHT),
                RulerTickKind::Half => Some(RULER_HALF_TICK_HEIGHT),
                RulerTickKind::WholeInch => None,
            };
            if let Some(height) = height {
                paint_snapped_rect(
                    tick_painter,
                    egui::Rect::from_center_size(
                        egui::pos2(x, bar_center_y),
                        Vec2::new(RULER_TICK_WIDTH, height),
                    ),
                    pixels_per_point,
                    tick_color,
                );
            }

            if let Some(label) = tick.label.as_deref() {
                tick_painter.text(
                    egui::pos2(x, label_y),
                    Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(RULER_LABEL_FONT_SIZE),
                    label_color,
                );
            }
        }
    }

    let Some(line) = layout.first_line_for_element(focus_element) else {
        return;
    };
    let positions = marker_positions(&line);
    let first_x = document_x_to_screen(page_left, scale, positions.first_line_x);
    let left_x = document_x_to_screen(page_left, scale, positions.left_x);
    let right_x = document_x_to_screen(page_left, scale, positions.right_x);
    let marker_stroke = Stroke::new(2.0 / pixels_per_point, marker_outline);
    let marker_half_width = RULER_MARKER_WIDTH * 0.5;
    let first_top = bar_center_y - RULER_FIRST_LINE_CENTER_OFFSET;
    let common_tip_y = first_top + RULER_MARKER_BODY_HEIGHT + RULER_MARKER_TIP_HEIGHT;
    let shoulder_y = common_tip_y + RULER_MARKER_TIP_HEIGHT;
    let right_base_y = common_tip_y + RULER_MARKER_TIP_HEIGHT + RULER_MARKER_BODY_HEIGHT;
    let left_bottom_y = bar_center_y + RULER_BOTTOM_MARKER_CENTER_OFFSET;
    let left_base_y = left_bottom_y - RULER_LEFT_FOOT_HEIGHT;

    if first_x >= page_left && first_x <= page_right {
        paint_marker_stem(
            &strip_painter,
            first_x,
            common_tip_y,
            bar_center_y,
            pixels_per_point,
            marker_outline,
        );
    }

    if left_x >= page_left && left_x <= page_right {
        paint_marker_stem(
            &strip_painter,
            left_x,
            bar_center_y,
            common_tip_y,
            pixels_per_point,
            marker_outline,
        );
    }

    if right_x >= page_left && right_x <= page_right {
        paint_marker_stem(
            &strip_painter,
            right_x,
            bar_center_y,
            common_tip_y,
            pixels_per_point,
            marker_outline,
        );
    }

    if right_x >= page_left && right_x <= page_right {
        paint_marker_polygon(
            &strip_painter,
            [
                egui::pos2(right_x, common_tip_y),
                egui::pos2(right_x + marker_half_width, shoulder_y),
                egui::pos2(right_x + marker_half_width, right_base_y),
                egui::pos2(right_x - marker_half_width, right_base_y),
                egui::pos2(right_x - marker_half_width, shoulder_y),
            ],
            pixels_per_point,
            marker_fill,
            marker_stroke,
        );
    }

    if left_x >= page_left && left_x <= page_right {
        paint_marker_polygon(
            &strip_painter,
            [
                egui::pos2(left_x, common_tip_y),
                egui::pos2(left_x + marker_half_width, shoulder_y),
                egui::pos2(left_x + marker_half_width, left_bottom_y),
                egui::pos2(left_x - marker_half_width, left_bottom_y),
                egui::pos2(left_x - marker_half_width, shoulder_y),
            ],
            pixels_per_point,
            marker_fill,
            marker_stroke,
        );
        let divider = [
            egui::pos2(left_x - marker_half_width, left_base_y),
            egui::pos2(left_x + marker_half_width, left_base_y),
        ]
        .map(|point| snap_pixel_center(point, pixels_per_point));
        strip_painter.line_segment(divider, marker_stroke);
    }

    if first_x >= page_left && first_x <= page_right {
        paint_marker_polygon(
            &strip_painter,
            [
                egui::pos2(first_x - marker_half_width, first_top),
                egui::pos2(first_x + marker_half_width, first_top),
                egui::pos2(
                    first_x + marker_half_width,
                    first_top + RULER_MARKER_BODY_HEIGHT,
                ),
                egui::pos2(first_x, common_tip_y),
                egui::pos2(
                    first_x - marker_half_width,
                    first_top + RULER_MARKER_BODY_HEIGHT,
                ),
            ],
            pixels_per_point,
            marker_fill,
            marker_stroke,
        );
    }
}

fn paint_marker_stem(
    painter: &eframe::egui::Painter,
    x: f32,
    y_start: f32,
    y_end: f32,
    pixels_per_point: f32,
    color: Color32,
) {
    let min_y = y_start.min(y_end);
    let max_y = y_start.max(y_end);
    if max_y <= min_y {
        return;
    }
    paint_snapped_rect(
        painter,
        eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(x - RULER_MARKER_STEM_WIDTH * 0.5, min_y),
            eframe::egui::pos2(x + RULER_MARKER_STEM_WIDTH * 0.5, max_y),
        ),
        pixels_per_point,
        color,
    );
}

fn paint_marker_polygon(
    painter: &eframe::egui::Painter,
    points: [eframe::egui::Pos2; 5],
    pixels_per_point: f32,
    fill: Color32,
    stroke: eframe::egui::Stroke,
) {
    let points = points.map(|point| snap_pixel_center(point, pixels_per_point));
    painter.add(eframe::egui::Shape::convex_polygon(
        points.to_vec(),
        fill,
        stroke,
    ));
}

fn paint_snapped_rect(
    painter: &eframe::egui::Painter,
    rect: eframe::egui::Rect,
    pixels_per_point: f32,
    color: Color32,
) {
    let snap_edge = |coordinate: f32| (coordinate * pixels_per_point).round() / pixels_per_point;
    let snapped = eframe::egui::Rect::from_min_max(
        eframe::egui::pos2(snap_edge(rect.min.x), snap_edge(rect.min.y)),
        eframe::egui::pos2(snap_edge(rect.max.x), snap_edge(rect.max.y)),
    );
    if snapped.is_positive() {
        painter.rect_filled(snapped, 0.0, color);
    }
}

fn snap_pixel_center(point: eframe::egui::Pos2, pixels_per_point: f32) -> eframe::egui::Pos2 {
    let snap = |coordinate: f32| ((coordinate * pixels_per_point).floor() + 0.5) / pixels_per_point;
    eframe::egui::pos2(snap(point.x), snap(point.y))
}
