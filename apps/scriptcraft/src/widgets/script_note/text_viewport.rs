//! Native card pointer mapping and focused Body scrolling helpers.

use super::{geometry::CardRegions, state::ScriptNoteWorkspaceState};
use eframe::egui::{Pos2, Rect};
use scriptcraft_layout::{Caret, ScriptNoteLayout};
use scriptcraft_session::editing::{ScriptNoteField, ScriptNoteTextPosition};

pub(crate) fn selection_at(
    layout: &ScriptNoteLayout,
    regions: CardRegions,
    field: ScriptNoteField,
    position: Pos2,
    body_offset: f32,
    title_offset: f32,
    click_count: usize,
    title: &str,
    paragraphs: &[scriptcraft_model::ScriptNoteParagraph],
) -> Option<(ScriptNoteTextPosition, ScriptNoteTextPosition)> {
    match field {
        ScriptNoteField::Title => {
            let local_x = position.x - regions.card.left() + title_offset;
            let caret = layout.title_caret_for_x(scriptcraft_model::formatting::FixedPoint::from_points_f64(f64::from(local_x))?);
            let mut start = ScriptNoteTextPosition { paragraph: 0, offset: caret.offset };
            let mut end = start;
            if click_count >= 3 {
                start.offset = 0;
                end.offset = title.len();
            } else if click_count == 2 {
                (start.offset, end.offset) = word_bounds(title, start.offset);
            }
            Some((start, end))
        }
        ScriptNoteField::Body => {
            let local_y = position.y - regions.card.top() + body_offset;
            let line_index = layout.body_lines.iter().position(|line| {
                let top = line.geometry.y.to_points_f32();
                let bottom = top + line.geometry.height.to_points_f32();
                local_y >= top && local_y <= bottom
            }).or_else(|| {
                layout.body_lines.iter().enumerate().min_by(|(_, left), (_, right)| {
                    (left.geometry.y.to_points_f32() - local_y).abs().total_cmp(&(right.geometry.y.to_points_f32() - local_y).abs())
                }).map(|(index, _)| index)
            })?;
            let line = layout.body_lines.get(line_index)?;
            let paragraph = paragraphs.get(line.element)?;
            let local_x = position.x - regions.card.left();
            let caret = layout.body_caret_for_x(line_index, scriptcraft_model::formatting::FixedPoint::from_points_f64(f64::from(local_x))?)?;
            let mut start = ScriptNoteTextPosition { paragraph: line.element, offset: caret.offset };
            let mut end = start;
            if click_count >= 3 {
                start.offset = 0;
                end.offset = paragraph.text.len();
            } else if click_count == 2 {
                (start.offset, end.offset) = word_bounds(&paragraph.text, caret.offset);
            }
            Some((start, end))
        }
    }
}

pub(crate) fn reveal_title_offset(
    layout: &ScriptNoteLayout,
    selection: &scriptcraft_session::editing::ScriptNoteSelection,
    state: &mut ScriptNoteWorkspaceState,
) {
    let scriptcraft_session::editing::ScriptNoteSelection::Text {
        note,
        field: ScriptNoteField::Title,
        focus,
        ..
    } = selection else {
        state.title_offset = 0.0;
        return;
    };
    if state.open_note.as_ref() != Some(note)
        || !state.focused
        || state.field != Some(ScriptNoteField::Title)
    {
        state.title_offset = 0.0;
        return;
    }
    let Ok(caret) = layout.title_caret_geometry(focus.offset) else { return };
    let left = layout.title_rect.x.to_points_f32();
    let right = left + layout.title_rect.width.to_points_f32();
    let x = caret.x.to_points_f32();
    if x - state.title_offset < left { state.title_offset = x - left; }
    if x - state.title_offset > right { state.title_offset = x - right; }
    state.title_offset = state.title_offset.max(0.0)
        .min((layout.title.geometry.width.to_points_f32() - layout.title_rect.width.to_points_f32()).max(0.0));
}

fn word_bounds(text: &str, offset: usize) -> (usize, usize) {
    if text.is_empty() { return (0, 0); }
    let mut at = offset.min(text.len());
    if at == text.len() { at = text.char_indices().last().map_or(0, |(index, _)| index); }
    while !text.is_char_boundary(at) { at = at.saturating_sub(1); }
    let Some(character) = text.get(at..).and_then(|tail| tail.chars().next()) else { return (at, at) };
    let combining = |ch: char| matches!(ch as u32,
        0x0300..=0x036f | 0x0483..=0x0489 | 0x0591..=0x05bd | 0x05bf..=0x05c7 |
        0x0610..=0x061a | 0x064b..=0x065f | 0x0670 | 0x06d6..=0x06ed |
        0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x20d0..=0x20ff | 0xfe20..=0xfe2f);
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_' || combining(ch);
    let mut start = at;
    let mut end = at + character.len_utf8();
    if !is_word(character) && !character.is_whitespace() { return (start, end); }
    while start > 0 {
        let Some((index, previous)) = text[..start].char_indices().next_back() else { break };
        if is_word(character) {
            if !is_word(previous) { break; }
        } else if !previous.is_whitespace() {
            break;
        }
        start = index;
    }
    while end < text.len() {
        let Some(next) = text[end..].chars().next() else { break };
        if is_word(character) {
            if !is_word(next) { break; }
        } else if !next.is_whitespace() {
            break;
        }
        end += next.len_utf8();
    }
    (start, end)
}

pub(crate) fn body_wheel_scroll(
    state: &mut ScriptNoteWorkspaceState,
    layout: &ScriptNoteLayout,
    body_rect: Rect,
    pointer: Pos2,
    delta_y: f32,
) -> bool {
    if !state.focused || state.field != Some(ScriptNoteField::Body) || !body_rect.contains(pointer) {
        return false;
    }
    let overflow = (layout.body_content_height.to_points_f32() - layout.body_rect.height.to_points_f32()).max(0.0);
    if overflow <= 0.0 { return false; }
    let next = (state.body_offset - delta_y).clamp(0.0, overflow);
    if (next - state.body_offset).abs() <= f32::EPSILON { return false; }
    state.body_offset = next;
    true
}

pub(crate) fn reveal_body_offset(
    layout: &ScriptNoteLayout,
    line_index: usize,
    state: &mut ScriptNoteWorkspaceState,
) {
    let Some(line) = layout.body_lines.get(line_index) else { return };
    let top = line.geometry.y.to_points_f32() - layout.body_rect.y.to_points_f32();
    let bottom = top + line.geometry.height.to_points_f32();
    let height = layout.body_rect.height.to_points_f32();
    if top < state.body_offset { state.body_offset = top.max(0.0); }
    else if bottom > state.body_offset + height { state.body_offset = (bottom - height).max(0.0); }
    let max = (layout.body_content_height.to_points_f32() - height).max(0.0);
    state.body_offset = state.body_offset.clamp(0.0, max);
}
