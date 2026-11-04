//! Reversible ScriptCraft mapping for disposable FDX TextState offsets.

use crate::{
    editing::{Position, Selection},
    Project,
};

pub const MIN_TEXT_SCALING: u16 = 75;
pub const MAX_TEXT_SCALING: u16 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextState {
    pub scaling: u16,
    pub selection: Selection,
}

impl TextState {
    pub const DEFAULT: Self = Self {
        scaling: 100,
        selection: Selection::caret(Position {
            element: 0,
            offset: 0,
        }),
    };

    pub fn normalized_scaling(value: u64) -> Option<u16> {
        (value > 0)
            .then_some(value.clamp(u64::from(MIN_TEXT_SCALING), u64::from(MAX_TEXT_SCALING)) as u16)
    }
}

/// Map a flat UTF-16 code-unit offset into a canonical body paragraph and UTF-8 byte boundary.
/// One code unit separates adjacent semantic body paragraphs.
pub fn flat_offset_to_position(project: &Project, offset: usize) -> Option<Position> {
    if project.elements.is_empty() {
        return None;
    }
    let mut base = 0usize;
    for (element_index, element) in project.elements.iter().enumerate() {
        if offset == base {
            return Some(Position {
                element: element_index,
                offset: 0,
            });
        }
        for (byte, character) in element.text.char_indices() {
            let next = base.checked_add(character.len_utf16())?;
            if offset < next {
                return None;
            }
            if offset == next {
                return Some(Position {
                    element: element_index,
                    offset: byte + character.len_utf8(),
                });
            }
            base = next;
        }
        if element_index + 1 < project.elements.len() {
            base = base.checked_add(1)?;
        }
    }
    (offset == base).then(|| Position {
        element: project.elements.len() - 1,
        offset: project
            .elements
            .last()
            .map_or(0, |element| element.text.len()),
    })
}

/// Map an editor selection to ordered flat UTF-16 offsets without changing its direction.
pub fn selection_to_flat_offsets(
    project: &Project,
    selection: Selection,
) -> Option<(usize, usize)> {
    fn position_to_flat(project: &Project, position: Position) -> Option<usize> {
        let element = project.elements.get(position.element)?;
        if position.offset > element.text.len() || !element.text.is_char_boundary(position.offset) {
            return None;
        }
        let mut flat = 0usize;
        for (index, candidate) in project.elements.iter().enumerate() {
            if index == position.element {
                return flat.checked_add(candidate.text[..position.offset].encode_utf16().count());
            }
            flat = flat.checked_add(candidate.text.encode_utf16().count())?;
            flat = flat.checked_add(1)?;
        }
        None
    }

    let anchor = position_to_flat(project, selection.anchor)?;
    let focus = position_to_flat(project, selection.focus)?;
    Some((anchor.min(focus), anchor.max(focus)))
}

/// Parse exactly two nonnegative decimal UTF-16 offsets and map them atomically.
pub fn parse_flat_selection(project: &Project, value: &str) -> Option<Selection> {
    let (start, end) = value.split_once(',')?;
    if end.contains(',') {
        return None;
    }
    if start.is_empty()
        || end.is_empty()
        || !start.bytes().all(|byte| byte.is_ascii_digit())
        || !end.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let start = start.parse::<usize>().ok()?;
    let end = end.parse::<usize>().ok()?;
    if start > end {
        return None;
    }
    Some(Selection {
        anchor: flat_offset_to_position(project, start)?,
        focus: flat_offset_to_position(project, end)?,
    })
}
