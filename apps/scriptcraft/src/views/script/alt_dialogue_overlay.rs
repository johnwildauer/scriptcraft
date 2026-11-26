//! Frame-local Script Body overlay geometry for alternate Dialogue variants.

use crate::chrome::status_bar::PresentedPage;
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke};
use scriptcraft_layout::ScreenplayLayout;
use scriptcraft_model::{dialogue_alts::DialogueAlternatives, Project};

pub(super) const REST_OPACITY: f32 = 0.69;
pub(super) const HOVER_COLOR: Color32 = Color32::from_rgb(112, 170, 220);
pub(super) const ICON_SIZE: f32 = 15.0;
pub(super) const ARROW_SIZE: f32 = 28.0;
pub(super) const ARROW_HEIGHT_OFFSET: f32 = -4.0;
pub(super) const ICON_HOVER_SIZE: f32 = ICON_SIZE + 1.0;
pub(super) const ARROW_HOVER_SIZE: f32 = ARROW_SIZE + 1.0;
pub(super) const INDENT_GAP: f32 = 10.0;
pub(super) const ARROW_GAP: f32 = 0.0;
pub(super) const COUNT_TEXT_SIZE: f32 = 11.0;
pub(super) const COUNT_HOVER_TEXT_SIZE: f32 = 12.0;
pub(super) const COUNT_BOX_SIZE: f32 = 16.0;
pub(super) const ORDINAL_TEXT_SIZE: f32 = 11.0;
const ORDINAL_INTERNAL_GAP: f32 = 0.0;
const COUNT_BORDER_WIDTH: f32 = 1.0;
const COUNT_HOVER_BORDER_WIDTH: f32 = 1.0;
const COUNT_CORNER_RADIUS: f32 = 1.5;
const COUNT_GLOW_INSET: f32 = 1.0;
const COUNT_GLOW_CORNER_RADIUS: f32 = 2.0;
const COUNT_GLOW_ALPHA: u8 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AltOverlayAction {
    Add,
    Delete,
    Previous,
    Next,
    FocusAtTextEnd { element_index: usize, text_end: usize },
}

#[derive(Debug, Clone, Copy)]
enum AltGlyph {
    Add,
    Delete,
    Previous,
    Next,
}

#[derive(Debug, Clone)]
struct IconControl {
    action: AltOverlayAction,
    glyph: AltGlyph,
    rest_size: f32,
    hover_size: f32,
    rect: Rect,
    hit_rect: Rect,
}

#[derive(Debug, Clone)]
struct CountControl {
    action: AltOverlayAction,
    rect: Rect,
    center: Pos2,
    text: String,
}

#[derive(Debug, Clone)]
struct OrdinalLabel {
    center: Pos2,
    text: String,
}

/// Controls for one canonical first Dialogue line, shared by paint and hit testing.
#[derive(Debug, Clone)]
pub(super) struct AltOverlayGeometry {
    pub(super) page_index: usize,
    icons: Vec<IconControl>,
    count: Option<CountControl>,
    ordinal: Option<OrdinalLabel>,
}

fn centered_rect(center: Pos2, size: f32) -> Rect {
    Rect::from_center_size(center, egui::vec2(size, size))
}

fn text_width(painter: &Painter, text: &str, size: f32, scale: f32) -> f32 {
    painter
        .layout_no_wrap(
            text.to_owned(),
            FontId::new(size * scale, FontFamily::Name("Courier Prime".into())),
            Color32::WHITE,
        )
        .size()
        .x
}

fn project_icon(
    action: AltOverlayAction,
    glyph: AltGlyph,
    center: Pos2,
    page_rect: Rect,
    editor_clip: Rect,
    scale: f32,
) -> IconControl {
    let (rest_size, hover_size) = match glyph {
        AltGlyph::Add | AltGlyph::Delete => (ICON_SIZE, ICON_HOVER_SIZE),
        AltGlyph::Previous | AltGlyph::Next => (ARROW_SIZE, ARROW_HOVER_SIZE),
    };
    let hit_rect = centered_rect(center, hover_size * scale)
        .intersect(page_rect)
        .intersect(editor_clip);
    IconControl {
        action,
        glyph,
        rest_size,
        hover_size,
        rect: centered_rect(center, rest_size * scale),
        hit_rect,
    }
}

/// Projects only requested page owners, using their existing canonical first-line geometry.
pub(super) fn project_visible_controls(
    layout: &ScreenplayLayout,
    project: &Project,
    presented_pages: &[PresentedPage],
    projected_pages: &[usize],
    scale: f32,
    editor_clip: Rect,
    active_element: Option<usize>,
    painter: &Painter,
) -> Vec<AltOverlayGeometry> {
    if projected_pages.is_empty() {
        return Vec::new();
    }

    let mut controls = Vec::new();
    for (element_index, owner) in project.elements.iter().enumerate() {
        let Some(semantics) = project.element_semantics(&owner.element_type()) else {
            continue;
        };
        if !semantics.capabilities().dialogue {
            continue;
        }
        let alternatives = owner.dialogue_alternatives.as_ref();
        let is_active = active_element == Some(element_index);
        if !is_active && !alternatives.is_some_and(|alternatives| alternatives.total_count() > 1) {
            continue;
        }

        let Some(first_line) = layout.first_line_for_element(element_index) else {
            continue;
        };
        if !projected_pages.contains(&first_line.page_index) {
            continue;
        }
        let Some(presented_page) = presented_pages.get(first_line.page_index).copied() else {
            continue;
        };
        let line = first_line.line;
        let page_rect = presented_page.rect;
        let paragraph_right = page_rect.left()
            + (line.paragraph_bounds.x.to_points_f32()
                + line.paragraph_bounds.width.to_points_f32())
                * scale;
        let first_indent_left = page_rect.left() + line.unaligned_line_left.to_points_f32() * scale;
        let line_center_y = crate::views::script::page_policy::document_y_to_screen(
            presented_page,
            line.paragraph_bounds.y.to_points_f32()
                + line.paragraph_bounds.height.to_points_f32() * 0.5,
            scale,
        );
        let center_y = line_center_y;
        let mut geometry = AltOverlayGeometry {
            page_index: first_line.page_index,
            icons: Vec::new(),
            count: None,
            ordinal: None,
        };

        if is_active {
            let total = alternatives.map_or(1, DialogueAlternatives::total_count);
            let add_center = Pos2::new(
                paragraph_right + (INDENT_GAP + ICON_SIZE * 0.5) * scale,
                center_y,
            );
            geometry.icons.push(project_icon(
                AltOverlayAction::Add,
                AltGlyph::Add,
                add_center,
                page_rect,
                editor_clip,
                scale,
            ));
            if total > 1 {
                let delete_center = Pos2::new(
                    add_center.x + (ICON_SIZE + INDENT_GAP) * scale,
                    center_y,
                );
                geometry.icons.push(project_icon(
                    AltOverlayAction::Delete,
                    AltGlyph::Delete,
                    delete_center,
                    page_rect,
                    editor_clip,
                    scale,
                ));

                let Some(alternatives) = alternatives else {
                    continue;
                };
                let ordinal = alternatives.active_ordinal();
                let ordinal_text = format!("{ordinal}/{total}");
                let ordinal_width = text_width(painter, &ordinal_text, ORDINAL_TEXT_SIZE, scale);
                let icon_width = ARROW_SIZE * scale;
                let group_gap = ORDINAL_INTERNAL_GAP * scale;
                let group_width = icon_width * 2.0 + ordinal_width + group_gap * 2.0;
                let group_right = first_indent_left - ARROW_GAP * scale;
                let group_left = group_right - group_width;
                let previous_center = Pos2::new(group_left + icon_width * 0.5, center_y + ARROW_HEIGHT_OFFSET);
                let ordinal_center = Pos2::new(
                    group_left + icon_width + group_gap + ordinal_width * 0.5,
                    center_y,
                );
                let next_center = Pos2::new(group_right - icon_width * 0.5, center_y + ARROW_HEIGHT_OFFSET);
                geometry.icons.push(project_icon(
                    AltOverlayAction::Previous,
                    AltGlyph::Previous,
                    previous_center,
                    page_rect,
                    editor_clip,
                    scale,
                ));
                geometry.ordinal = Some(OrdinalLabel {
                    center: ordinal_center,
                    text: ordinal_text,
                });
                geometry.icons.push(project_icon(
                    AltOverlayAction::Next,
                    AltGlyph::Next,
                    next_center,
                    page_rect,
                    editor_clip,
                    scale,
                ));
            }
        } else if let Some(alternatives) = alternatives {
            let total = alternatives.total_count();
            if total > 1 {
                let center = Pos2::new(
                    first_indent_left - (INDENT_GAP + COUNT_BOX_SIZE * 0.5) * scale,
                    center_y,
                );
                let rect = centered_rect(center, COUNT_BOX_SIZE * scale);
                geometry.count = Some(CountControl {
                    action: AltOverlayAction::FocusAtTextEnd {
                        element_index,
                        text_end: owner.text.len(),
                    },
                    rect: rect.intersect(page_rect).intersect(editor_clip),
                    center,
                    text: total.to_string(),
                });
            }
        }

        controls.push(geometry);
    }
    controls
}

pub(super) fn hovered_control(
    controls: &[AltOverlayGeometry],
    pointer: Option<Pos2>,
) -> Option<AltOverlayAction> {
    let pointer = pointer?;
    controls.iter().find_map(|geometry| {
        geometry
            .icons
            .iter()
            .find(|control| control.hit_rect.is_positive() && control.hit_rect.contains(pointer))
            .map(|control| control.action)
            .or_else(|| {
                geometry.count.as_ref().and_then(|count| {
                    (count.rect.is_positive() && count.rect.contains(pointer)).then_some(count.action)
                })
            })
    })
}

fn rest_color(color: Color32) -> Color32 {
    let alpha = (f32::from(color.a()) * REST_OPACITY).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn glyph(glyph: AltGlyph) -> egui_material_icons::MaterialIcon {
    use egui_material_icons::icons;
    match glyph {
        AltGlyph::Add => icons::ICON_ADD_CIRCLE,
        AltGlyph::Delete => icons::ICON_DO_NOT_DISTURB_ON,
        AltGlyph::Previous => icons::ICON_ARROW_LEFT,
        AltGlyph::Next => icons::ICON_ARROW_RIGHT,
    }
}

/// Paints after screenplay glyphs with page/editor clips; hover only changes centered visuals.
pub(super) fn paint_controls(
    painter: &Painter,
    page_rect: Rect,
    editor_clip: Rect,
    scale: f32,
    subdued_text: Color32,
    controls: &[AltOverlayGeometry],
    hovered: Option<AltOverlayAction>,
) {
    let clipped = painter.with_clip_rect(
        painter
            .clip_rect()
            .intersect(page_rect)
            .intersect(editor_clip),
    );
    for geometry in controls {
        let hovered_in_group = geometry
            .icons
            .iter()
            .any(|control| hovered == Some(control.action));
        for control in &geometry.icons {
            let is_hovered = hovered == Some(control.action);
            let color = if is_hovered {
                HOVER_COLOR
            } else {
                rest_color(subdued_text)
            };
            let icon = glyph(control.glyph).outlined();
            clipped.text(
                control.rect.center(),
                Align2::CENTER_CENTER,
                icon.codepoint,
                FontId::new(
                    (if is_hovered {
                        control.hover_size
                    } else {
                        control.rest_size
                    }) * scale,
                    icon.font_family(),
                ),
                color,
            );
        }
        if let Some(ordinal) = &geometry.ordinal {
            let color = if hovered_in_group {
                HOVER_COLOR
            } else {
                rest_color(subdued_text)
            };
            clipped.text(
                ordinal.center,
                Align2::CENTER_CENTER,
                ordinal.text.clone(),
                FontId::new(
                    ORDINAL_TEXT_SIZE * scale,
                    FontFamily::Name("Courier Prime".into()),
                ),
                color,
            );
        }
        if let Some(count) = &geometry.count {
            let is_hovered = hovered == Some(count.action);
            let color = if is_hovered {
                HOVER_COLOR
            } else {
                rest_color(subdued_text)
            };
            if is_hovered {
                let glow = count.rect.expand(COUNT_GLOW_INSET * scale);
                clipped.rect_filled(
                    glow,
                    COUNT_GLOW_CORNER_RADIUS * scale,
                    Color32::from_rgba_unmultiplied(
                        HOVER_COLOR.r(),
                        HOVER_COLOR.g(),
                        HOVER_COLOR.b(),
                        COUNT_GLOW_ALPHA,
                    ),
                );
            }
            clipped.rect_stroke(
                count.rect,
                COUNT_CORNER_RADIUS * scale,
                Stroke::new(
                    (if is_hovered {
                        COUNT_HOVER_BORDER_WIDTH
                    } else {
                        COUNT_BORDER_WIDTH
                    }) * scale,
                    color,
                ),
                egui::StrokeKind::Inside,
            );
            clipped.text(
                count.center + egui::vec2(0.0, 1.0),
                Align2::CENTER_CENTER,
                count.text.clone(),
                FontId::new(
                    (if is_hovered {
                        COUNT_HOVER_TEXT_SIZE
                    } else {
                        COUNT_TEXT_SIZE
                    }) * scale,
                    FontFamily::Name("Courier Prime".into()),
                ),
                color,
            );
        }
    }
}
