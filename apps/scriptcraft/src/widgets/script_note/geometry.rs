//! Fixed card regions and display-coordinate placement helpers.

use eframe::egui::{pos2, Pos2, Rect, Vec2};

pub(crate) const CARD_SIZE: Vec2 = Vec2::new(296.0, 260.0);
pub(crate) const TOP_BAR_HEIGHT: f32 = 20.0;
pub(crate) const TITLE_TOP_GAP: f32 = 4.0;
pub(crate) const TITLE_HEIGHT: f32 = 32.0;
pub(crate) const BODY_TOP: f32 = TOP_BAR_HEIGHT + TITLE_TOP_GAP + TITLE_HEIGHT;
pub(crate) const FOOTER_HEIGHT: f32 = 28.0;
pub(crate) const FOOTER_TOP: f32 = CARD_SIZE.y - FOOTER_HEIGHT;
pub(crate) const HORIZONTAL_INSET: f32 = 10.0;
pub(crate) const BODY_VERTICAL_INSET: f32 = 0.0;
pub(crate) const DELETE_GAP: f32 = 3.0;
pub(crate) const CARD_RADIUS: f32 = 8.0;
pub(crate) const BORDER_WIDTH: f32 = 1.0;
pub(crate) const DELETE_SIZE: f32 = 18.0;
pub(crate) const DELETE_HIT_SIZE: f32 = 24.0;
pub(crate) const TITLE_TRAILING_RESERVE: f32 = DELETE_HIT_SIZE + DELETE_GAP;
pub(crate) const CHEVRON_HIT_SIZE: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CardRegions {
    pub(crate) card: Rect,
    pub(crate) top_bar: Rect,
    pub(crate) title: Rect,
    pub(crate) body: Rect,
    pub(crate) footer: Rect,
    pub(crate) color_region: Rect,
    pub(crate) type_region: Rect,
    pub(crate) delete: Rect,
    pub(crate) previous: Rect,
    pub(crate) next: Rect,
}

impl CardRegions {
    pub(crate) fn at(origin: Pos2) -> Self {
        let card = Rect::from_min_size(origin, CARD_SIZE);
        let top_bar = Rect::from_min_size(origin, Vec2::new(CARD_SIZE.x, TOP_BAR_HEIGHT));
        let title = Rect::from_min_max(
            pos2(origin.x + HORIZONTAL_INSET, origin.y + TOP_BAR_HEIGHT + TITLE_TOP_GAP),
            pos2(origin.x + CARD_SIZE.x - HORIZONTAL_INSET - TITLE_TRAILING_RESERVE, origin.y + BODY_TOP),
        );
        let body = Rect::from_min_max(
            pos2(origin.x + HORIZONTAL_INSET, origin.y + BODY_TOP + BODY_VERTICAL_INSET),
            pos2(origin.x + CARD_SIZE.x - HORIZONTAL_INSET, origin.y + FOOTER_TOP - BODY_VERTICAL_INSET),
        );
        let footer = Rect::from_min_size(
            pos2(origin.x, origin.y + FOOTER_TOP),
            Vec2::new(CARD_SIZE.x, FOOTER_HEIGHT),
        );
        let color_region = Rect::from_min_size(
            pos2(origin.x + HORIZONTAL_INSET, origin.y + FOOTER_TOP),
            Vec2::new(58.0, FOOTER_HEIGHT),
        );
        let previous = Rect::from_min_size(
            pos2(card.right() - CHEVRON_HIT_SIZE * 2.0, footer.center().y - CHEVRON_HIT_SIZE * 0.5),
            Vec2::splat(CHEVRON_HIT_SIZE),
        );
        let type_region = Rect::from_min_size(
            pos2(color_region.right(), origin.y + FOOTER_TOP),
            Vec2::new((previous.left() - color_region.right()).max(0.0), FOOTER_HEIGHT),
        );
        let delete = Rect::from_min_size(
            pos2(origin.x + CARD_SIZE.x - HORIZONTAL_INSET - DELETE_HIT_SIZE, title.center().y - DELETE_HIT_SIZE * 0.5),
            Vec2::splat(DELETE_HIT_SIZE),
        );
        let next = Rect::from_min_size(
            pos2(card.right() - CHEVRON_HIT_SIZE, footer.center().y - CHEVRON_HIT_SIZE * 0.5),
            Vec2::splat(CHEVRON_HIT_SIZE),
        );
        Self { card, top_bar, title, body, footer, color_region, type_region, delete, previous, next }
    }
}

/// Choose a display-coordinate card origin. Validated monitor work bounds own
/// full-card fitting and vertical clamping; root bounds cap the card's left
/// origin to keep it attached to the application window.
pub(crate) fn place_card(
    baseline: f32,
    marker_left: f32,
    marker_right: f32,
    root_bounds: Option<Rect>,
    work_bounds: Option<Rect>,
) -> Pos2 {
    let root_bounds = root_bounds.filter(|rect| rect.is_finite() && rect.is_positive());
    let work_bounds = work_bounds.filter(|rect| rect.is_finite() && rect.is_positive());
    // A valid work area is the screen constraint. Only when it is unavailable
    // do valid root bounds serve as the available horizontal constraint.
    let horizontal_bounds = work_bounds.or(root_bounds).map(|bounds| (bounds.left(), bounds.right()));
    let left_edge = horizontal_bounds.map_or(f32::NEG_INFINITY, |(left, _)| left);
    let right_edge = horizontal_bounds.map_or(f32::INFINITY, |(_, right)| right);
    let right_origin = root_bounds.map_or(marker_right + 8.0, |root| (marker_right + 8.0).min(root.right()));
    let left_origin = marker_left - 8.0 - CARD_SIZE.x;
    let left_space = (marker_left - 8.0 - left_edge).max(0.0);
    let right_space = (right_edge - marker_right - 8.0).max(0.0);
    let mut x = if right_origin >= left_edge && right_origin + CARD_SIZE.x <= right_edge {
        right_origin
    } else if left_origin >= left_edge {
        left_origin
    } else if right_space >= left_space {
        right_origin.min(right_edge - CARD_SIZE.x)
    } else {
        left_origin.max(left_edge)
    };
    let mut y = baseline - CARD_SIZE.y * 0.5;
    if let Some((left, right)) = horizontal_bounds {
        x = x.clamp(left, (right - CARD_SIZE.x).max(left));
    }
    if let Some(root) = root_bounds {
        x = x.min(root.right());
    }
    if let Some(bounds) = work_bounds {
        y = y.clamp(bounds.top(), (bounds.bottom() - CARD_SIZE.y).max(bounds.top()));
    }
    pos2(x, y)
}

/// Clamp a viewport origin to a validated work area while keeping oversized
/// viewports as far right/down inside it as their dimensions permit.
pub(crate) fn clamp_origin(origin: Pos2, size: Vec2, work_bounds: Option<Rect>) -> Pos2 {
    let Some(bounds) = work_bounds.filter(|rect| rect.is_finite() && rect.is_positive()) else {
        return origin;
    };
    pos2(
        origin.x.clamp(bounds.left(), (bounds.right() - size.x).max(bounds.left())),
        origin.y.clamp(bounds.top(), (bounds.bottom() - size.y).max(bounds.top())),
    )
}

pub(crate) fn parse_modified_timestamp(value: &str) -> Option<String> {
    if value.len() != 15 || value.as_bytes().get(8) != Some(&b'T') {
        return None;
    }
    let digits = value.as_bytes();
    if digits.iter().enumerate().any(|(index, byte)| index != 8 && !byte.is_ascii_digit()) {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        value.get(range)?.parse().ok()
    };
    let year = number(0..4)?;
    let month = number(4..6)?;
    let day = number(6..8)?;
    let hour = number(9..11)?;
    let minute = number(11..13)?;
    let second = number(13..15)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > month_days || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let hour12 = match hour % 12 { 0 => 12, value => value };
    Some(format!("{month:02}/{day:02}/{year:04} {hour12:02}:{minute:02} {}", if hour < 12 { "AM" } else { "PM" }))
}
