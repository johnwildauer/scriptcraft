//! Canonical note selection, caret, and transient IME preedit painting.

use super::state::ScriptNoteWorkspaceState;
use crate::chrome::theme::PagePalette;
use eframe::egui::{self, Pos2, Rect};
use scriptcraft_layout::{Caret, ScriptNoteLayout};
use scriptcraft_session::editing::{ScriptNoteField, ScriptNoteSelection, ScriptNoteTextPosition};
use std::time::Duration;

pub(crate) fn paint_selection(
    painter: &egui::Painter,
    regions: super::geometry::CardRegions,
    layout: &ScriptNoteLayout,
    note_id: &scriptcraft_model::StableId,
    selection: &ScriptNoteSelection,
    state: &ScriptNoteWorkspaceState,
    palette: PagePalette,
) {
    let ScriptNoteSelection::Text { note, field, anchor, focus } = selection else { return };
    if note != note_id { return; }
    let (start, end) = if anchor <= focus { (*anchor, *focus) } else { (*focus, *anchor) };
    let field_rect = match field {
        ScriptNoteField::Title => regions.title,
        ScriptNoteField::Body => regions.body,
    };
    let field_painter = painter.with_clip_rect(field_rect);
    if start != end {
        match field {
            ScriptNoteField::Title => {
                let a = layout.title_caret_geometry(start.offset).ok();
                let b = layout.title_caret_geometry(end.offset).ok();
                if let (Some(a), Some(b)) = (a, b) {
                    let r = Rect::from_min_max(
                        Pos2::new(a.x.to_points_f32().min(b.x.to_points_f32()) - state.title_offset, regions.title.top()),
                        Pos2::new(a.x.to_points_f32().max(b.x.to_points_f32()).max(a.x.to_points_f32() + 1.0) - state.title_offset, regions.title.bottom()),
                    ).intersect(regions.title);
                    field_painter.rect_filled(r, 0.0, palette.selection);
                }
            }
            ScriptNoteField::Body => {
                for (index, line) in layout.body_lines.iter().enumerate() {
                    if line.element < start.paragraph || line.element > end.paragraph { continue; }
                    let from = if line.element == start.paragraph { start.offset.max(line.source.start) } else { line.source.start };
                    let to = if line.element == end.paragraph { end.offset.min(line.source.end) } else { line.source.end };
                    if from >= to { continue; }
                    let (Some(a), Some(b)) = (
                        layout.body_caret_geometry(index, &Caret { element: line.element, offset: from }).ok(),
                        layout.body_caret_geometry(index, &Caret { element: line.element, offset: to }).ok(),
                    ) else { continue };
                    let min_x = a.x.to_points_f32();
                    let max_x = b.x.to_points_f32();
                    let y = a.y.to_points_f32() - state.body_offset;
                    let r = Rect::from_min_max(Pos2::new(min_x.min(max_x), y), Pos2::new(min_x.max(max_x).max(min_x + 1.0), y + line.geometry.height.to_points_f32())).intersect(regions.body);
                    if r.is_positive() { field_painter.rect_filled(r, 0.0, palette.selection); }
                }
            }
        }
    }
}

pub(crate) fn paint_caret(
    painter: &egui::Painter,
    regions: super::geometry::CardRegions,
    layout: &ScriptNoteLayout,
    note_id: &scriptcraft_model::StableId,
    selection: &ScriptNoteSelection,
    state: &ScriptNoteWorkspaceState,
    palette: PagePalette,
) {
    let ScriptNoteSelection::Text { note, field, anchor, focus } = selection else { return };
    if note != note_id || !state.focused || state.field != Some(*field) || anchor != focus { return; }
    let field_rect = match field { ScriptNoteField::Title => regions.title, ScriptNoteField::Body => regions.body };
    let field_painter = painter.with_clip_rect(field_rect);
    let caret = match field {
        ScriptNoteField::Title => layout.title_caret_geometry(focus.offset).ok(),
        ScriptNoteField::Body => layout.body_lines.iter().position(|line| line.element == focus.paragraph && line.source.start <= focus.offset && focus.offset <= line.source.end)
            .and_then(|index| layout.body_caret_geometry(index, &Caret { element: focus.paragraph, offset: focus.offset }).ok()),
    };
    let Some(caret) = caret else { return };
    let caret_rect = Rect::from_min_size(
        Pos2::new(
            caret.x.to_points_f32() - if *field == ScriptNoteField::Title { state.title_offset } else { 0.0 },
            caret.y.to_points_f32() - if *field == ScriptNoteField::Body { state.body_offset } else { 0.0 },
        ),
        egui::vec2(1.2, caret.height.to_points_f32().max(1.0)),
    );
    let now = painter.ctx().input(|input| input.time.max(0.0));
    let phase = ((now - state.caret_reset_time).max(0.0) / 0.5).floor() as u64;
    painter.ctx().request_repaint_after(Duration::from_millis(500));
    if phase.is_multiple_of(2) { field_painter.rect_filled(caret_rect.intersect(field_rect), 0.0, palette.caret); }
    if !state.ime_preedit.is_empty() {
        let galley = field_painter.layout_no_wrap(state.ime_preedit.clone(), egui::FontId::proportional(12.0), palette.ime_text);
        field_painter.galley(caret_rect.left_top(), galley, palette.ime_text);
        // egui's native viewport API does not expose a custom IME candidate-window
        // anchor rectangle. Composition paint follows the canonical caret only.
    }
    let _ = ScriptNoteTextPosition { paragraph: focus.paragraph, offset: focus.offset };
}
