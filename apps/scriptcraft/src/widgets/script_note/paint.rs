//! Fixed card chrome and measured rich-text painting.

use super::{geometry::CardRegions, state::ScriptNoteWorkspaceState};
use crate::{chrome::theme::PagePalette, presentation::{document_font::PreparedDocumentFonts, span_paint::resolve_span_paint}};
use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke, StrokeKind};
use scriptcraft_layout::{LayoutLine, ScriptNoteLayout, ScriptNoteTitleProjection};
use scriptcraft_model::ScriptNote;
use scriptcraft_session::editing::ScriptNoteSelection;

pub(crate) fn paint(
    painter: &Painter,
    regions: CardRegions,
    note: &ScriptNote,
    layout: &ScriptNoteLayout,
    placeholder: Option<&ScriptNoteLayout>,
    prepared: &PreparedDocumentFonts,
    palette: PagePalette,
    state: &ScriptNoteWorkspaceState,
    note_id: &scriptcraft_model::StableId,
    selection: &ScriptNoteSelection,
    previous_enabled: bool,
    next_enabled: bool,
) {
    let card = painter.with_clip_rect(regions.card);
    card.rect_filled(regions.card, super::geometry::CARD_RADIUS, palette.paper);
    card.rect_stroke(regions.card, super::geometry::CARD_RADIUS, Stroke::new(1.0, palette.border), StrokeKind::Inside);
    card.line_segment(
        [regions.footer.left_bottom(), regions.footer.right_bottom()],
        Stroke::new(1.0, palette.border),
    );
    card.line_segment(
        [regions.top_bar.left_bottom(), regions.top_bar.right_bottom()],
        Stroke::new(1.0, palette.note_internal_separator),
    );
    super::text_paint::paint_selection(painter, regions, layout, note_id, selection, state, palette);
    card.line_segment(
        [regions.footer.left_top(), regions.footer.right_top()],
        Stroke::new(1.0, palette.note_internal_separator),
    );

    let writer = if note.writer_name.is_empty() { "" } else { note.writer_name.as_str() };
    let date = super::geometry::parse_modified_timestamp(&note.date_modified).unwrap_or_default();
    paint_clipped_text(&card, regions.top_bar.shrink2(egui::vec2(8.0, 4.0)), writer, palette.subdued_text, 11.0, false);
    let date_rect = Rect::from_min_max(
        Pos2::new(regions.top_bar.left() + regions.top_bar.width() * 0.44, regions.top_bar.top() + 5.0),
        Pos2::new(regions.top_bar.right() - 8.0, regions.top_bar.bottom() - 4.0),
    );
    paint_clipped_text(&card, date_rect, &date, palette.subdued_text, 11.0, true);

    if note.title.is_empty() {
        if let Some(sample) = placeholder {
            paint_line(&card, &sample.title, Pos2::ZERO, 0.0, prepared, palette, Some(regions.title), true);
        }
    } else if state.focused && state.field == Some(scriptcraft_session::editing::ScriptNoteField::Title) {
        paint_line(&card, &layout.title, Pos2::new(-state.title_offset, 0.0), 0.0, prepared, palette, Some(regions.title), false);
    } else if let Some(projection) = &layout.truncated_title {
        paint_title_projection(&card, projection, prepared, palette, regions.title);
    } else {
        paint_line(&card, &layout.title, Pos2::ZERO, 0.0, prepared, palette, Some(regions.title), false);
    }

    let swatch = Rect::from_min_size(
        Pos2::new(regions.color_region.left(), regions.color_region.center().y - 7.0),
        egui::vec2(24.0, 14.0),
    );
    let color = super::properties::resolve_color(note.color_projection(), palette.text);
    card.rect_filled(swatch, 3.0, color.color);
    card.rect_stroke(swatch, 3.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);
    let type_text = note.note_type.as_deref().unwrap_or("None");
    paint_type_group(&card, regions.type_region, type_text, palette.text);

    let body_painter = card.with_clip_rect(regions.body);
    if layout.body_is_empty {
        if let Some(line) = placeholder.and_then(|sample| sample.body_lines.first()) {
            paint_line(&body_painter, line, Pos2::ZERO, state.body_offset, prepared, palette, Some(regions.body), true);
        }
    } else {
        for line in &layout.body_lines {
            if line.geometry.y.to_points_f32() - state.body_offset > layout.body_rect.bottom().to_points_f32() {
                break;
            }
            paint_line(&body_painter, line, Pos2::ZERO, state.body_offset, prepared, palette, Some(regions.body), false);
        }
    }

    let delete_color = if state.delete_hovered { Color32::from_rgb(255, 0, 0) } else { palette.text };
    let delete_glyph_rect = Rect::from_center_size(
        Pos2::new(
            regions.delete.center().x,
            layout.title.geometry.y.to_points_f32() + layout.title.geometry.height.to_points_f32() * 0.5,
        ),
        egui::vec2(super::geometry::DELETE_SIZE, super::geometry::DELETE_SIZE),
    );
    paint_icon_sized(&card, delete_glyph_rect, egui_material_icons::icons::ICON_DELETE, delete_color, super::geometry::DELETE_SIZE);
    paint_icon(&card, regions.previous, egui_material_icons::icons::ICON_CHEVRON_LEFT, if previous_enabled { palette.text } else { palette.subdued_text });
    paint_icon(&card, regions.next, egui_material_icons::icons::ICON_CHEVRON_RIGHT, if next_enabled { palette.text } else { palette.subdued_text });
}

fn paint_line(
    painter: &Painter,
    line: &LayoutLine,
    offset: Pos2,
    vertical_scroll: f32,
    prepared: &PreparedDocumentFonts,
    palette: PagePalette,
    clip: Option<Rect>,
    subdued: bool,
) {
    let painter = clip.map_or_else(|| painter.clone(), |rect| painter.with_clip_rect(rect));
    for span in line.spans.iter() {
        let Some(style) = resolve_span_paint(
            &span.resolved_text_style,
            span.resolved_text_style.size.to_points_f32(),
            palette,
            prepared,
        ) else { continue };
        let color = if subdued { palette.subdued_text } else { style.color };
        let galley = painter.layout_no_wrap(span.display_text.to_string(), style.font, color);
        let pos = Pos2::new(
            offset.x + span.geometry.x.to_points_f32(),
            offset.y + span.geometry.y.to_points_f32() - vertical_scroll,
        );
        let galley_height = galley.size().y;
        painter.galley(pos, galley, color);
        let baseline = pos.y + galley_height * 0.78;
        if span.underline {
            painter.line_segment(
                [Pos2::new(pos.x, baseline + 1.0), Pos2::new(pos.x + span.geometry.width.to_points_f32(), baseline + 1.0)],
                Stroke::new(1.0, color),
            );
        }
        if span.strikethrough {
            painter.line_segment(
                [Pos2::new(pos.x, pos.y + galley_height * 0.48), Pos2::new(pos.x + span.geometry.width.to_points_f32(), pos.y + galley_height * 0.48)],
                Stroke::new(1.0, color),
            );
        }
    }
}

fn paint_title_projection(
    painter: &Painter,
    projection: &ScriptNoteTitleProjection,
    prepared: &PreparedDocumentFonts,
    palette: PagePalette,
    clip: Rect,
) {
    paint_line(painter, &projection.prefix, Pos2::ZERO, 0.0, prepared, palette, Some(clip), false);
    let ellipsis = &projection.ellipsis;
    let Some(style) = resolve_span_paint(
        &ellipsis.resolved_text_style,
        ellipsis.resolved_text_style.size.to_points_f32(),
        palette,
        prepared,
    ) else { return };
    let galley = painter.layout_no_wrap(ellipsis.text.to_owned(), style.font, style.color);
    let position = Pos2::new(ellipsis.geometry.x.to_points_f32(), ellipsis.geometry.y.to_points_f32());
    painter.with_clip_rect(clip).galley(position, galley, style.color);
}

fn paint_clipped_text(painter: &Painter, rect: Rect, text: &str, color: Color32, size: f32, right: bool) {
    if text.is_empty() { return; }
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::new(size, FontFamily::Proportional), color);
    let x = if right { rect.right() - galley.size().x } else { rect.left() };
    let y = rect.center().y - galley.size().y * 0.5;
    painter.with_clip_rect(rect).galley(Pos2::new(x, y), galley, color);
}

fn paint_type_group(painter: &Painter, region: Rect, text: &str, color: Color32) {
    if region.width() <= 0.0 { return; }
    let font = FontId::new(12.0, FontFamily::Proportional);
    let icon = egui_material_icons::icons::ICON_UNFOLD_MORE.outlined();
    let icon_font = FontId::new(18.0, icon.font_family());
    let icon_galley = painter.layout_no_wrap(icon.codepoint.to_owned(), icon_font, color);
    let gap = 4.0;
    let max_text_width = (region.width() - icon_galley.size().x - gap).max(0.0);
    let mut display = text.to_owned();
    let mut galley = painter.layout_no_wrap(display.clone(), font.clone(), color);
    if galley.size().x > max_text_width {
        let ellipsis = "…";
        let ellipsis_width = painter.layout_no_wrap(ellipsis.to_owned(), font.clone(), color).size().x;
        let mut boundary = display.len();
        while boundary > 0 {
            boundary -= 1;
            while !display.is_char_boundary(boundary) { boundary -= 1; }
            let candidate = format!("{}{}", &display[..boundary], ellipsis);
            let candidate_galley = painter.layout_no_wrap(candidate.clone(), font.clone(), color);
            if candidate_galley.size().x <= max_text_width || boundary == 0 {
                display = candidate;
                galley = candidate_galley;
                break;
            }
        }
        if max_text_width < ellipsis_width {
            display.clear();
            galley = painter.layout_no_wrap(String::new(), font, color);
        }
    }
    let group_width = (galley.size().x + if display.is_empty() { 0.0 } else { gap } + icon_galley.size().x)
        .min(region.width());
    let left = region.center().x - group_width * 0.5;
    let top = region.center().y - galley.size().y * 0.5;
    painter.with_clip_rect(region).galley(Pos2::new(left, top), galley.clone(), color);
    let icon_x = left + galley.size().x + if display.is_empty() { 0.0 } else { gap };
    painter.with_clip_rect(region).galley(
        Pos2::new(icon_x, region.center().y - icon_galley.size().y * 0.5),
        icon_galley,
        color,
    );
}

fn paint_icon(painter: &Painter, rect: Rect, icon: egui_material_icons::MaterialIcon, color: Color32) {
    paint_icon_sized(painter, rect, icon, color, 16.0);
}

fn paint_icon_sized(painter: &Painter, rect: Rect, icon: egui_material_icons::MaterialIcon, color: Color32, size: f32) {
    let galley = painter.layout_no_wrap(icon.filled().codepoint.to_owned(), FontId::new(size, icon.font_family()), color);
    painter.galley(rect.center() - galley.size() * 0.5, galley, color);
}
