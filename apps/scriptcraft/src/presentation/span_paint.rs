//! Shared resolved font and color policy for document span painting.

use crate::{chrome::theme, presentation::document_font};
use eframe::egui::{Color32, FontFamily, FontId, Ui};
use scriptcraft_model::typography::{EffectiveTextColor, EffectiveTextStyle};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResolvedSpanPaint {
    pub(crate) font: FontId,
    pub(crate) color: Color32,
    pub(crate) unsupported_color: bool,
}

pub(crate) fn font_for_active_family(ui: &Ui, font: &FontId) -> FontId {
    let family = font.family.clone();
    let is_bound = ui.fonts(|fonts| {
        let definitions = fonts.definitions();
        definitions
            .families
            .get(&family)
            .is_some_and(|keys| keys.iter().any(|key| definitions.font_data.contains_key(key)))
    });
    if is_bound {
        font.clone()
    } else {
        FontId::new(font.size, FontFamily::Proportional)
    }
}

pub(crate) fn neutral_resolved_font(
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

pub(crate) fn resolve_span_paint(
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
    let (color, unsupported_color) = match style.color {
        EffectiveTextColor::ThemeDefault => (
            if style.hidden_text { palette.subdued_text } else { palette.text },
            false,
        ),
        EffectiveTextColor::ExplicitRgb8 { red, green, blue } => {
            (Color32::from_rgb(red, green, blue), false)
        }
        EffectiveTextColor::UnsupportedExplicit => (
            if style.hidden_text { palette.subdued_text } else { palette.text },
            true,
        ),
    };
    Some(ResolvedSpanPaint { font, color, unsupported_color })
}
