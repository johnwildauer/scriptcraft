use eframe::egui::{Pos2, Rect};

use super::constants::{
    LABEL_COLUMN_WIDTH, MAX_STRIP_HEIGHT, MIN_STRIP_HEIGHT, PAGE_RULER_ROW_HEIGHT,
    RESIZE_HIT_BAND_HEIGHT, STATIC_BEAT_VERTICAL_INSET,
};
use super::constants::{UN_SCALED_TARGET_WIDTH, ZOOM_PERCENT_PER_TARGET_PAGE};

pub(crate) fn clamp_height(requested: f32, available_height: f32) -> Option<u32> {
    if !requested.is_finite()
        || !available_height.is_finite()
        || available_height < MIN_STRIP_HEIGHT
    {
        return None;
    }
    let maximum = available_height.min(MAX_STRIP_HEIGHT).floor();
    if maximum < MIN_STRIP_HEIGHT {
        return None;
    }
    let clamped = requested
        .round()
        .clamp(MIN_STRIP_HEIGHT, maximum);
    (clamped.is_finite() && (MIN_STRIP_HEIGHT..=maximum).contains(&clamped))
        .then_some(clamped as u32)
}

#[derive(Debug, Clone)]
pub(crate) struct StripGeometry {
    pub(crate) rect: Rect,
    pub(crate) label_column: Rect,
    pub(crate) timeline_viewport: Rect,
    pub(crate) lane_rows: Vec<Rect>,
    pub(crate) ruler_row: Rect,
    pub(crate) script_row: Rect,
    pub(crate) scrollbar_row: Rect,
    pub(crate) resize_band: Rect,
}

/// Allocates the shell from one common origin. Invalid/nonpositive composition
/// bounds are rejected before any egui rectangle is produced.
pub(crate) fn allocate_strip(
    rect: Rect,
    lane_count: usize,
    scrollbar_height: f32,
) -> Option<StripGeometry> {
    let min = rect.min;
    let max = rect.max;
    if !finite_pos(min)
        || !finite_pos(max)
        || !scrollbar_height.is_finite()
        || scrollbar_height < 0.0
        || max.x <= min.x
        || max.y <= min.y
    {
        return None;
    }

    let width = max.x - min.x;
    let height = max.y - min.y;
    if !width.is_finite()
        || !height.is_finite()
        || height < PAGE_RULER_ROW_HEIGHT + scrollbar_height
    {
        return None;
    }
    let label_right = min.x + LABEL_COLUMN_WIDTH;
    if !label_right.is_finite() {
        return None;
    }
    let label_column = Rect::from_min_max(min, Pos2::new(label_right, max.y));
    let timeline_right = max.x.max(label_right);
    let timeline_viewport = Rect::from_min_max(
        Pos2::new(label_right, min.y),
        Pos2::new(timeline_right, max.y),
    );

    let remaining_height = (height - PAGE_RULER_ROW_HEIGHT - scrollbar_height).max(0.0);
    let row_count = lane_count.saturating_add(1);
    if row_count == 0 {
        return None;
    }
    let row_count_f = row_count as f32;
    if !row_count_f.is_finite() || row_count_f <= 0.0 {
        return None;
    }

    let mut lane_rows = Vec::with_capacity(lane_count);
    for index in 0..lane_count {
        let row_top = if index == 0 {
            min.y
        } else {
            min.y + remaining_height * index as f32 / row_count_f
        };
        let row_bottom = min.y + remaining_height * (index + 1) as f32 / row_count_f;
        lane_rows.push(Rect::from_min_max(
            Pos2::new(min.x, row_top),
            Pos2::new(max.x, row_bottom),
        ));
    }

    let dynamic_bottom = min.y + remaining_height * lane_count as f32 / row_count_f;
    let ruler_bottom = dynamic_bottom + PAGE_RULER_ROW_HEIGHT;
    // The last equal-share row ends exactly at the fixed scrollbar origin.
    let script_bottom = max.y - scrollbar_height;
    let ruler_row = Rect::from_min_max(
        Pos2::new(min.x, dynamic_bottom),
        Pos2::new(max.x, ruler_bottom.min(max.y)),
    );
    let script_row = Rect::from_min_max(
        Pos2::new(min.x, ruler_bottom.min(max.y)),
        Pos2::new(max.x, script_bottom.max(ruler_bottom.min(max.y))),
    );
    let scrollbar_top = script_row.max.y;
    let scrollbar_row = Rect::from_min_max(
        Pos2::new(min.x, scrollbar_top),
        Pos2::new(max.x, max.y),
    );
    let resize_top = (max.y - RESIZE_HIT_BAND_HEIGHT).max(min.y);
    let resize_band = Rect::from_min_max(Pos2::new(min.x, resize_top), max);

    Some(StripGeometry {
        rect,
        label_column,
        timeline_viewport,
        lane_rows,
        ruler_row,
        script_row,
        scrollbar_row,
        resize_band,
    })
}

fn finite_pos(pos: Pos2) -> bool {
    pos.x.is_finite() && pos.y.is_finite()
}

/// Immutable, shared transform for every Story Map timeline consumer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TimelineGeometry {
    pub(crate) target_pages: f64,
    pub(crate) domain_pages: f64,
    pub(crate) timeline_left: f64,
    pub(crate) viewport_width: f64,
    pub(crate) zoom_min: f64,
    pub(crate) zoom_max: f64,
    pub(crate) zoom_percent: f64,
    pub(crate) page_pitch: f64,
    pub(crate) unscaled_domain_width: f64,
    pub(crate) scaled_domain_width: f64,
    pub(crate) origin: f64,
    pub(crate) max_origin: f64,
}

impl TimelineGeometry {
    /// Derive and clamp a transform. `domain_pages` is explicit so Phase 3 can
    /// provide canonical Body overflow without changing the transform policy.
    pub(crate) fn new(
        target_pages: f64,
        domain_pages: f64,
        timeline_left: f64,
        viewport_width: f64,
        zoom_percent: f64,
        origin: f64,
    ) -> Option<Self> {
        if !target_pages.is_finite()
            || target_pages <= 0.0
            || !domain_pages.is_finite()
            || domain_pages < target_pages
            || !timeline_left.is_finite()
            || !viewport_width.is_finite()
            || viewport_width <= 0.0
            || !zoom_percent.is_finite()
            || !origin.is_finite()
        {
            return None;
        }
        let unscaled_domain_width = UN_SCALED_TARGET_WIDTH * domain_pages / target_pages;
        let zoom_min_fit = 100.0 * viewport_width / unscaled_domain_width;
        let zoom_max = ZOOM_PERCENT_PER_TARGET_PAGE * target_pages;
        let zoom_min = zoom_min_fit.min(zoom_max);
        let zoom_percent = zoom_percent.clamp(zoom_min, zoom_max);
        let page_pitch = (UN_SCALED_TARGET_WIDTH / target_pages) * zoom_percent / 100.0;
        let scaled_domain_width = unscaled_domain_width * zoom_percent / 100.0;
        let max_origin = (scaled_domain_width - viewport_width).max(0.0);
        let origin = origin.clamp(0.0, max_origin);
        let values = [
            unscaled_domain_width,
            zoom_min_fit,
            zoom_min,
            zoom_max,
            zoom_percent,
            page_pitch,
            scaled_domain_width,
            max_origin,
            origin,
        ];
        if values.iter().any(|value| !value.is_finite())
            || unscaled_domain_width <= 0.0
            || page_pitch <= 0.0
        {
            return None;
        }
        Some(Self {
            target_pages,
            domain_pages,
            timeline_left,
            viewport_width,
            zoom_min,
            zoom_max,
            zoom_percent,
            page_pitch,
            unscaled_domain_width,
            scaled_domain_width,
            origin,
            max_origin,
        })
    }

    pub(crate) fn page_to_screen(&self, page_position: f64) -> Option<f64> {
        let screen = self.timeline_left + page_position * self.page_pitch - self.origin;
        screen.is_finite().then_some(screen)
    }

    pub(crate) fn screen_to_page(&self, screen_x: f64) -> Option<f64> {
        let page = (screen_x - self.timeline_left + self.origin) / self.page_pitch;
        page.is_finite().then_some(page)
    }

    pub(crate) fn visible_page_range(&self) -> Option<(f64, f64)> {
        let start = self.origin / self.page_pitch;
        let end = (self.origin + self.viewport_width) / self.page_pitch;
        (start.is_finite() && end.is_finite())
            .then_some((start.max(0.0).min(self.domain_pages), end.max(0.0).min(self.domain_pages)))
    }

    /// Return a center-anchored transform at `zoom_percent`.
    pub(crate) fn zoom_around_center(&self, zoom_percent: f64) -> Option<Self> {
        if !zoom_percent.is_finite() {
            return None;
        }
        let center_page = (self.origin + self.viewport_width / 2.0) / self.page_pitch;
        let proposed_zoom = zoom_percent.clamp(self.zoom_min, self.zoom_max);
        let new_pitch = (UN_SCALED_TARGET_WIDTH / self.target_pages) * proposed_zoom / 100.0;
        let new_origin = center_page * new_pitch - self.viewport_width / 2.0;
        if !center_page.is_finite() || !new_pitch.is_finite() || !new_origin.is_finite() {
            return None;
        }
        Self::new(
            self.target_pages,
            self.domain_pages,
            self.timeline_left,
            self.viewport_width,
            proposed_zoom,
            new_origin,
        )
    }

    pub(crate) fn with_origin(&self, origin: f64) -> Option<Self> {
        Self::new(
            self.target_pages,
            self.domain_pages,
            self.timeline_left,
            self.viewport_width,
            self.zoom_percent,
            origin,
        )
    }
}

/// True when an exact half-open page interval intersects the current viewport.
/// Callers retain semantic integer endpoints and use this only for paint culling.
pub(crate) fn interval_intersects_visible_range(
    timeline: &TimelineGeometry,
    start_page: f64,
    end_page: f64,
) -> bool {
    if !start_page.is_finite() || !end_page.is_finite() || end_page <= start_page {
        return false;
    }
    timeline
        .visible_page_range()
        .is_some_and(|(visible_start, visible_end)| {
            end_page > visible_start && start_page < visible_end
        })
}

/// Projects one legal semantic placement to the same clipped rectangle used by
/// static Beat paint and Story Map-local hit testing.
pub(crate) fn placed_beat_rect(
    strip: &StripGeometry,
    lane_index: usize,
    start_eighths: u64,
    length_eighths: u64,
    target_pages: u32,
    timeline: &TimelineGeometry,
    clip: Rect,
) -> Option<Rect> {
    let row = strip.lane_rows.get(lane_index)?.intersect(strip.rect);
    let target_eighths = u64::from(target_pages).checked_mul(8)?;
    let end_eighths = start_eighths.checked_add(length_eighths)?;
    if length_eighths == 0 || end_eighths > target_eighths {
        return None;
    }
    let left = timeline.page_to_screen(start_eighths as f64 / 8.0)?;
    let right = timeline.page_to_screen(end_eighths as f64 / 8.0)?;
    if !left.is_finite()
        || !right.is_finite()
        || left < f64::from(f32::MIN)
        || left > f64::from(f32::MAX)
        || right < f64::from(f32::MIN)
        || right > f64::from(f32::MAX)
    {
        return None;
    }
    let y_inset = STATIC_BEAT_VERTICAL_INSET.min(row.height() * 0.5);
    let block = Rect::from_min_max(
        Pos2::new(left as f32, row.top() + y_inset),
        Pos2::new(right as f32, row.bottom() - y_inset),
    );
    let clipped = block
        .intersect(row)
        .intersect(strip.timeline_viewport)
        .intersect(strip.rect)
        .intersect(clip);
    clipped.is_positive().then_some(clipped)
}

/// Cull an exact half-open page interval through the shared transform, then
/// return its horizontally clipped screen bounds for presentation paint.
pub(crate) fn visible_interval_screen_bounds(
    timeline: &TimelineGeometry,
    start_page: f64,
    end_page: f64,
    clip: Rect,
) -> Option<(f32, f32)> {
    if !clip.is_positive()
        || !clip.min.x.is_finite()
        || !clip.max.x.is_finite()
        || !clip.min.y.is_finite()
        || !clip.max.y.is_finite()
        || !interval_intersects_visible_range(timeline, start_page, end_page)
    {
        return None;
    }
    let left = timeline.page_to_screen(start_page)?;
    let right = timeline.page_to_screen(end_page)?;
    if !left.is_finite() || !right.is_finite() || right <= left {
        return None;
    }
    let clipped_left = left.max(f64::from(clip.left()));
    let clipped_right = right.min(f64::from(clip.right()));
    if clipped_right <= clipped_left {
        return None;
    }
    Some((clipped_left as f32, clipped_right as f32))
}
