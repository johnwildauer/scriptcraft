//! Script View span styling, galley construction, and interaction projection.

use super::ScriptCraftApp;
use crate::presentation::geometry as presentation_geometry;
use eframe::egui;
use scriptcraft_layout::LayoutLine;

impl ScriptCraftApp {
    pub(crate) fn project_line_for_interaction<'a>(
        &mut self,
        _ctx: &egui::Context,
        line: &'a LayoutLine,
        scale: f32,
    ) -> Result<Option<presentation_geometry::PresentationLine<'a>>, ()> {
        presentation_geometry::project_line(line, scale)
            .map(Some)
            .map_err(|_| ())
    }
}
