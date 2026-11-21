//! Script View span styling, galley construction, and interaction projection.

use super::ScriptCraftApp;
use crate::{
    chrome::theme,
    presentation::{document_font, geometry as presentation_geometry},
};
use eframe::egui::{self, Color32, FontFamily, FontId};
use scriptcraft_layout::LayoutLine;
use scriptcraft_model::typography::{
    EffectiveTextColor, EffectiveTextStyle,
};

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

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ResolvedSpanPaint {
    pub(super) font: FontId,
    pub(super) color: Color32,
    pub(super) unsupported_color: bool,
}

pub(super) fn neutral_resolved_font(
    font_size: f32,
    prepared: &document_font::PreparedDocumentFonts,
) -> Option<FontId> {
    let id = prepared.environment.neutral_request().regular.to_owned();
    let family_name = prepared.egui_family_names.get(&id)?;
    Some(FontId::new(
        font_size.max(1.0),
        FontFamily::Name(family_name.clone().into()),
    ))
}

pub(super) fn resolve_span_paint(
    style: &EffectiveTextStyle,
    font_size: f32,
    palette: theme::PagePalette,
    prepared: &document_font::PreparedDocumentFonts,
) -> Option<ResolvedSpanPaint> {
    let family_name = prepared.egui_family_names.get(&style.resolved_font_id)?;
    let font = FontId::new(
        font_size.max(1.0),
        FontFamily::Name(family_name.clone().into()),
    );
    let color_resolution = style.color;
    let (color, unsupported_color) = match color_resolution {
        EffectiveTextColor::ThemeDefault => (
            if style.hidden_text {
                palette.subdued_text
            } else {
                palette.text
            },
            false,
        ),
        EffectiveTextColor::ExplicitRgb8 { red, green, blue } => {
            (Color32::from_rgb(red, green, blue), false)
        }
        EffectiveTextColor::UnsupportedExplicit => (
            if style.hidden_text {
                palette.subdued_text
            } else {
                palette.text
            },
            true,
        ),
    };
    Some(ResolvedSpanPaint {
        font,
        color,
        unsupported_color,
    })
}
