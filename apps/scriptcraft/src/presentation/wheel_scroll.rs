//! Shared finite arithmetic for ordinary line-based wheel deltas.

/// Scales both axes of a Line-unit wheel delta by a valid effective multiplier.
pub(crate) fn scale_line_delta(
    delta: eframe::egui::Vec2,
    multiplier: f32,
) -> Option<eframe::egui::Vec2> {
    if !delta.x.is_finite()
        || !delta.y.is_finite()
        || !multiplier.is_finite()
        || multiplier <= 0.0
    {
        return None;
    }

    let scaled = delta * multiplier;
    (scaled.x.is_finite() && scaled.y.is_finite()).then_some(scaled)
}
