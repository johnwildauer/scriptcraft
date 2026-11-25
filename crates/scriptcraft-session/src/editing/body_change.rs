//! Range-aware primitives for flattened screenplay Body changes.

use super::{EditError, Position};
use scriptcraft_model::{
    script_notes::BodyTextReplacement,
    flat_document_len, position_to_flat_offset, Project,
};

pub(crate) fn text_replacement(
    project: &Project,
    start: Position,
    end: Position,
    inserted: &str,
) -> Result<BodyTextReplacement, EditError> {
    let start = position_to_flat_offset(project, start).ok_or(EditError::InvalidSelection)?;
    let end = position_to_flat_offset(project, end).ok_or(EditError::InvalidSelection)?;
    flat_replacement(project, start, end, inserted.encode_utf16().count())
}

pub(crate) fn flat_replacement(
    project: &Project,
    start: usize,
    end: usize,
    inserted_utf16_len: usize,
) -> Result<BodyTextReplacement, EditError> {
    let document_len = flat_document_len(project).ok_or(EditError::InvalidSelection)?;
    BodyTextReplacement::new(start, end, inserted_utf16_len, document_len)
        .ok_or(EditError::InvalidSelection)
}

pub(crate) fn apply(
    project: &mut Project,
    replacement: BodyTextReplacement,
) -> Result<(), EditError> {
    project
        .apply_body_text_replacement(replacement)
        .map_err(EditError::Model)
}
