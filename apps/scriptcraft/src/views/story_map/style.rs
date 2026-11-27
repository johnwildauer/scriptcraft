//! Transient definition-driven text styles used by Story Map presentation.

use eframe::egui::FontId;
use scriptcraft_model::{
    element_definition::ElementDefinition,
    formatting::FixedPoint,
    typography::{EffectiveTextStyle, FaceStyle},
    Project, StoryMapLane,
};

use crate::{
    chrome::theme::PagePalette,
    presentation::{
        document_font::PreparedDocumentFonts,
        span_paint::{neutral_resolved_font, resolve_span_paint, ResolvedSpanPaint},
    },
};

use super::constants::STORY_MAP_TEXT_FONT_SIZE;

const SCENE_HEADING_DEFINITION: &str = "Scene Heading";

pub(crate) fn resolve_definition_style(
    definition: &ElementDefinition,
    prepared: &PreparedDocumentFonts,
    palette: PagePalette,
) -> Option<ResolvedSpanPaint> {
    let font = &definition.font;
    let face_style = FaceStyle::from_flags(font.bold, font.italic);
    let resolved_font_id = prepared
        .environment
        .resolve_face_id(&font.family, face_style, None)
        .ok()?
        .clone();
    let effective = EffectiveTextStyle::new(
        font.family.clone(),
        font.color.clone(),
        resolved_font_id,
        FixedPoint::from_points(STORY_MAP_TEXT_FONT_SIZE as i64),
        font.bold,
        font.italic,
        font.underline,
        font.all_caps,
        font.hidden_text,
    )
    .ok()?;
    resolve_span_paint(&effective, STORY_MAP_TEXT_FONT_SIZE, palette, prepared)
}

pub(crate) fn resolve_lane_style(
    project: &Project,
    lane: &StoryMapLane,
    prepared: Option<&PreparedDocumentFonts>,
    palette: PagePalette,
) -> ResolvedSpanPaint {
    prepared
        .and_then(|prepared| {
            project
                .element_definitions()
                .get_by_label(&lane.lane_type)
                .and_then(|definition| resolve_definition_style(definition, prepared, palette))
        })
        .unwrap_or_else(|| generic_fallback(palette))
}

pub(crate) fn resolve_scene_heading_style(
    project: &Project,
    prepared: Option<&PreparedDocumentFonts>,
    palette: PagePalette,
) -> ResolvedSpanPaint {
    prepared
        .and_then(|prepared| {
            project
                .element_definitions()
                .get_by_label(SCENE_HEADING_DEFINITION)
                .and_then(|definition| resolve_definition_style(definition, prepared, palette))
        })
        .unwrap_or_else(|| generic_fallback(palette))
}

pub(crate) fn neutral_courier_fallback(
    prepared: &PreparedDocumentFonts,
    palette: PagePalette,
) -> Option<ResolvedSpanPaint> {
    Some(ResolvedSpanPaint {
        font: neutral_resolved_font(STORY_MAP_TEXT_FONT_SIZE, prepared)?,
        color: palette.text,
        unsupported_color: false,
    })
}

pub(crate) fn generic_fallback(palette: PagePalette) -> ResolvedSpanPaint {
    ResolvedSpanPaint {
        font: FontId::proportional(STORY_MAP_TEXT_FONT_SIZE),
        color: palette.text,
        unsupported_color: false,
    }
}
