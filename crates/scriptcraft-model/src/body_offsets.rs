//! Shared flattened UTF-16 coordinates for semantic Body paragraphs.

use crate::{position::{Position, Selection}, Project};

/// Convert a Body position's UTF-8 byte boundary to its flattened UTF-16 offset.
pub fn position_to_flat_offset(project: &Project, position: Position) -> Option<usize> {
    let paragraph = project.elements.get(position.element)?;
    if position.offset > paragraph.text.len() || !paragraph.text.is_char_boundary(position.offset) {
        return None;
    }
    paragraph_flat_start(project, position.element)?.checked_add(
        paragraph.text[..position.offset].encode_utf16().count(),
    )
}

/// Convert a selection to ordered flattened UTF-16 bounds.
///
/// Selection direction is intentionally not represented in the returned range.
pub fn selection_to_flat_offsets(
    project: &Project,
    selection: Selection,
) -> Option<(usize, usize)> {
    let anchor = position_to_flat_offset(project, selection.anchor)?;
    let focus = position_to_flat_offset(project, selection.focus)?;
    Some((anchor.min(focus), anchor.max(focus)))
}

/// Convert a valid flattened UTF-16 boundary to a Body paragraph and UTF-8 byte offset.
///
/// A single UTF-16 code unit separates adjacent semantic paragraphs. Offsets
/// inside a surrogate pair and the separator itself are not valid boundaries.
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
                    offset: byte.checked_add(character.len_utf8())?,
                });
            }
            base = next;
        }
        if element_index.checked_add(1)? < project.elements.len() {
            base = base.checked_add(1)?;
        }
    }
    if offset != base {
        return None;
    }
    Some(Position {
        element: project.elements.len().checked_sub(1)?,
        offset: project.elements.last()?.text.len(),
    })
}

/// Return the checked flattened UTF-16 start of one Body paragraph.
pub fn paragraph_flat_start(project: &Project, paragraph_index: usize) -> Option<usize> {
    if paragraph_index >= project.elements.len() {
        return None;
    }
    let mut start = 0usize;
    for paragraph in project.elements.iter().take(paragraph_index) {
        start = start.checked_add(paragraph.text.encode_utf16().count())?;
        start = start.checked_add(1)?;
    }
    Some(start)
}

/// Return total flattened Body length in UTF-16 code units, including paragraph separators.
pub fn flat_document_len(project: &Project) -> Option<usize> {
    let mut length = 0usize;
    for (index, paragraph) in project.elements.iter().enumerate() {
        length = length.checked_add(paragraph.text.encode_utf16().count())?;
        if index.checked_add(1)? < project.elements.len() {
            length = length.checked_add(1)?;
        }
    }
    Some(length)
}
