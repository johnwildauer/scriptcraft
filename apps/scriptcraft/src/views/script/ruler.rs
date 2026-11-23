//! Pure geometry and paper-width tick planning for the Script View ruler.
//!
//! The tick plan is the only cached ruler data. Paragraph markers are derived
//! from the current canonical first line so direct formatting and active
//! dual-dialogue geometry are reflected without a second resolver.

use eframe::egui::Color32;
use scriptcraft_layout::{LocatedLayoutLine, ScreenplayLayout, POINTS_PER_INCH};
use scriptcraft_model::formatting::FixedPoint;

pub(super) const RULER_HEIGHT: f32 = 12.0;
pub(super) const RULER_MARKER_OVERFLOW: f32 = 4.0;
pub(super) const RULER_STRIP_HEIGHT: f32 = RULER_HEIGHT + 2.0 * RULER_MARKER_OVERFLOW;

pub(super) const RULER_TICK_WIDTH: f32 = 1.0;
pub(super) const RULER_EIGHTH_TICK_HEIGHT: f32 = 2.0;
pub(super) const RULER_HALF_TICK_HEIGHT: f32 = 6.0;
pub(super) const RULER_INCH_TICK_HEIGHT: f32 = 6.0;
pub(super) const RULER_LABEL_FONT_SIZE: f32 = 10.0;
pub(super) const RULER_LABEL_Y_OFFSET: f32 = 0.0;

pub(super) const RULER_MARKER_WIDTH: f32 = 7.0;
pub(super) const RULER_MARKER_HEIGHT: f32 = 6.0;
pub(super) const RULER_MARKER_STEM_WIDTH: f32 = 1.0;

pub(super) const RULER_DAY_TICK_COLOR: Color32 = Color32::from_gray(112);
pub(super) const RULER_DAY_LABEL_COLOR: Color32 = Color32::from_gray(80);
pub(super) const RULER_DAY_MARKER_COLOR: Color32 = Color32::from_gray(64);
pub(super) const RULER_DARK_TICK_COLOR: Color32 = Color32::from_gray(144);
pub(super) const RULER_DARK_LABEL_COLOR: Color32 = Color32::from_gray(224);
pub(super) const RULER_DARK_MARKER_COLOR: Color32 = Color32::from_gray(238);

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
            ticks.push(RulerTick {
                document_x,
                kind,
                label,
            });

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

/// Paints from the completed frame's page geometry. The strip has no response
/// or interaction state; geometry comes from the actual page origin and scale.
pub(super) fn paint_ruler(
    ui: &eframe::egui::Ui,
    strip: eframe::egui::Rect,
    page_left: f32,
    page_width: f32,
    scale: f32,
    paper_width: u32,
    focus_element: usize,
    layout: &ScreenplayLayout,
    state: &mut RulerState,
    dark_mode: bool,
) {
    use eframe::egui::{self, Align2, Stroke, Vec2};

    let painter = ui.painter().with_clip_rect(strip);
    let pixels_per_point = ui.ctx().pixels_per_point().max(f32::MIN_POSITIVE);
    let (tick_color, label_color, marker_color) = if dark_mode {
        (
            RULER_DARK_TICK_COLOR,
            RULER_DARK_LABEL_COLOR,
            RULER_DARK_MARKER_COLOR,
        )
    } else {
        (
            RULER_DAY_TICK_COLOR,
            RULER_DAY_LABEL_COLOR,
            RULER_DAY_MARKER_COLOR,
        )
    };
    let page_right = page_left + page_width;
    let band_top = strip.top() + RULER_MARKER_OVERFLOW;
    let band_bottom = band_top + RULER_HEIGHT;
    let label_y = band_top + RULER_LABEL_FONT_SIZE * 0.5 + RULER_LABEL_Y_OFFSET;
    let plan = state.tick_plan(paper_width);

    for tick in &plan.ticks {
        let document_x = FixedPoint::from_points(i64::from(tick.document_x));
        let x = document_x_to_screen(page_left, scale, document_x);
        if x < page_left || x > page_right {
            continue;
        }
        let (width, height) = match tick.kind {
            RulerTickKind::Eighth => (RULER_TICK_WIDTH, RULER_EIGHTH_TICK_HEIGHT),
            RulerTickKind::Half => (RULER_TICK_WIDTH, RULER_HALF_TICK_HEIGHT),
            RulerTickKind::WholeInch => (RULER_TICK_WIDTH, RULER_INCH_TICK_HEIGHT),
        };
        paint_snapped_rect(
            &painter,
            egui::Rect::from_center_size(
                egui::pos2(x, band_bottom - height * 0.5),
                Vec2::new(width, height),
            ),
            pixels_per_point,
            tick_color,
        );

        if let Some(label) = tick.label.as_deref() {
            painter.text(
                egui::pos2(x, label_y),
                Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(RULER_LABEL_FONT_SIZE),
                label_color,
            );
        }
    }

    let Some(line) = layout.first_line_for_element(focus_element) else {
        return;
    };
    let positions = marker_positions(&line);
    let first_x = document_x_to_screen(page_left, scale, positions.first_line_x);
    let left_x = document_x_to_screen(page_left, scale, positions.left_x);
    let right_x = document_x_to_screen(page_left, scale, positions.right_x);
    let marker_stroke = Stroke::new(1.0 / pixels_per_point, marker_color);
    let top = strip.top();
    let bottom = strip.bottom();
    let marker_half_width = RULER_MARKER_WIDTH * 0.5;
    let marker_height = RULER_MARKER_HEIGHT;
    let band_center = (band_top + band_bottom) * 0.5;

    if first_x >= page_left && first_x <= page_right {
        paint_marker_stem(
            &painter,
            first_x,
            top + marker_height,
            band_center,
            pixels_per_point,
            marker_color,
        );
        paint_triangle_outline(
            &painter,
            [
                egui::pos2(first_x - marker_half_width, top),
                egui::pos2(first_x + marker_half_width, top),
                egui::pos2(first_x, top + marker_height),
            ],
            pixels_per_point,
            marker_stroke,
        );
    }

    for marker_x in [left_x, right_x] {
        if marker_x < page_left || marker_x > page_right {
            continue;
        }
        paint_marker_stem(
            &painter,
            marker_x,
            band_center,
            bottom - marker_height,
            pixels_per_point,
            marker_color,
        );
        paint_triangle_outline(
            &painter,
            [
                egui::pos2(marker_x, bottom - marker_height),
                egui::pos2(marker_x - marker_half_width, bottom),
                egui::pos2(marker_x + marker_half_width, bottom),
            ],
            pixels_per_point,
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

fn paint_triangle_outline(
    painter: &eframe::egui::Painter,
    points: [eframe::egui::Pos2; 3],
    pixels_per_point: f32,
    stroke: eframe::egui::Stroke,
) {
    let points = points.map(|point| snap_pixel_center(point, pixels_per_point));
    painter.line_segment([points[0], points[1]], stroke);
    painter.line_segment([points[1], points[2]], stroke);
    painter.line_segment([points[2], points[0]], stroke);
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
