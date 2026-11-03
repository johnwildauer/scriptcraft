//! Reversible ScriptCraft mapping for disposable FDX TextState offsets.

use crate::{editing::{Position, Selection}, Project};

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
        selection: Selection::caret(Position { element: 0, offset: 0 }),
    };

    pub fn normalized_scaling(value: u64) -> Option<u16> {
        (value > 0).then_some(value.clamp(u64::from(MIN_TEXT_SCALING), u64::from(MAX_TEXT_SCALING)) as u16)
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
            return Some(Position { element: element_index, offset: 0 });
        }
        for (byte, character) in element.text.char_indices() {
            let next = base.checked_add(character.len_utf16())?;
            if offset < next {
                return None;
            }
            if offset == next {
                return Some(Position { element: element_index, offset: byte + character.len_utf8() });
            }
            base = next;
        }
        if element_index + 1 < project.elements.len() {
            base = base.checked_add(1)?;
        }
    }
    (offset == base).then(|| Position {
        element: project.elements.len() - 1,
        offset: project.elements.last().map_or(0, |element| element.text.len()),
    })
}

/// Map an editor selection to ordered flat UTF-16 offsets without changing its direction.
pub fn selection_to_flat_offsets(project: &Project, selection: Selection) -> Option<(usize, usize)> {
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
    if start.is_empty() || end.is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Role, ScreenplayElement};

    fn project(paragraphs: &[&str]) -> Project {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = paragraphs.iter().enumerate().map(|(index, text)| {
            ScreenplayElement::new(format!("p{index}"), Role::Action, (*text).to_owned())
                .unwrap_or_else(|_| unreachable!())
        }).collect();
        project
    }

    #[test]
    fn v6_wo0013_flat_selection_round_trips_utf16_and_utf8_boundaries() {
        let project = project(&["Aé🙂", "", "z"]);
        for (element, text) in project.elements.iter().enumerate() {
            for (byte, _) in text.text.char_indices().chain(std::iter::once((text.text.len(), '\0'))) {
                let selection = Selection::caret(Position { element, offset: byte });
                let flat = selection_to_flat_offsets(&project, selection).unwrap_or_else(|| unreachable!()).0;
                assert_eq!(flat_offset_to_position(&project, flat), Some(selection.anchor));
            }
        }
        assert_eq!(flat_offset_to_position(&project, 4), Some(Position { element: 0, offset: 4 }));
        assert_eq!(flat_offset_to_position(&project, 5), Some(Position { element: 1, offset: 0 }));
        assert_eq!(flat_offset_to_position(&project, 6), Some(Position { element: 2, offset: 0 }));
        assert_eq!(flat_offset_to_position(&project, 3), None); // surrogate split
    }

    #[test]
    fn v6_wo0013_selection_rejects_invalid_pairs_atomically_and_orders_save_offsets() {
        let project = project(&["abc", "def"]);
        for value in ["2,1", "-1,2", "1,", "1,2,3", "0,999", " 1,2"] {
            assert_eq!(parse_flat_selection(&project, value), None);
        }
        let backward = Selection {
            anchor: Position { element: 1, offset: 2 },
            focus: Position { element: 0, offset: 1 },
        };
        assert_eq!(selection_to_flat_offsets(&project, backward), Some((1, 7)));
        assert_eq!(backward.anchor, Position { element: 1, offset: 2 });
    }
}
