//! Transient title caret-follow and focused body reveal within card clips.

use super::text_state::BeatTextState;
use scriptcraft_layout::{BeatCardLayout, Caret};
use scriptcraft_session::editing::{BeatSelection, BeatTextField};

pub(super) fn reveal(layout: &BeatCardLayout, selection: &BeatSelection, state: &mut BeatTextState) {
    let scriptcraft_session::editing::BeatSelection::Text { card, field, focus, .. } = selection else { return };
    let Some((active, active_field)) = state.field.as_ref() else { return };
    if active != card || active_field != field || !state.focused { return; }
    match field {
        BeatTextField::Title => {
            let Ok(caret) = layout.title_caret_geometry(focus.offset) else { return };
            let left = layout.title_rect.x.to_points_f32();
            let right = left + layout.title_rect.width.to_points_f32();
            let x = caret.x.to_points_f32();
            if x - state.title_offset < left { state.title_offset = x - left; }
            if x - state.title_offset > right { state.title_offset = x - right; }
            state.title_offset = state.title_offset.max(0.0).min((layout.title.geometry.width.to_points_f32() - layout.title_rect.width.to_points_f32()).max(0.0));
        }
        BeatTextField::Body => {
            let Some(index) = layout.body_lines.iter().position(|line| line.element == focus.paragraph && line.source.start <= focus.offset && focus.offset <= line.source.end) else { return };
            let Ok(caret) = layout.body_caret_geometry(index, &Caret { element: focus.paragraph, offset: focus.offset }) else { return };
            let top = layout.body_rect.y.to_points_f32();
            let bottom = top + layout.body_rect.height.to_points_f32();
            let y = caret.y.to_points_f32();
            let height = caret.height.to_points_f32();
            if y - state.body_offset < top { state.body_offset = y - top; }
            if y + height - state.body_offset > bottom { state.body_offset = y + height - bottom; }
            state.body_offset = state.body_offset.max(0.0).min((layout.body_content_height.to_points_f32() - layout.body_rect.height.to_points_f32()).max(0.0));
        }
    }
}
