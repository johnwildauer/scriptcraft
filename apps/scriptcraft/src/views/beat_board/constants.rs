use crate::chrome::theme::ScriptWorkspacePalette;
use eframe::egui::Color32;

pub(super) const GRID_CELL: f64 = 40.0;
pub(super) const GRID_FIRST_LINE: f64 = 20.0;
pub(super) const GRID_STROKE_PIXELS: f32 = 1.0;
pub(super) const VIEWPORT_ID: &str = "beat-board-viewport";
pub(crate) const WHEEL_POINTS_PER_LINE: f32 = 40.0;

// Keep the Beat grid appearance-specific; the shared page border belongs to Script/Title views.
pub(super) fn colors(palette: ScriptWorkspacePalette) -> (Color32, Color32, Color32) {
    (palette.background, palette.page.paper, palette.beat.grid)
}
