//! Durable correspondence for ScriptNote and note-Body identities after reread.

use super::EditError;
use scriptcraft_model::{Project, StableId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct ScriptNoteIdentityRebase {
    notes: BTreeMap<StableId, StableId>,
    paragraphs: BTreeMap<StableId, StableId>,
}

impl ScriptNoteIdentityRebase {
    /// Build a strict candidate-to-reread identity map before native publication.
    pub fn new(candidate: &Project, reread: &Project) -> Result<Self, EditError> {
        let candidate_notes = candidate.script_notes().iter().collect::<Vec<_>>();
        let reread_notes = reread.script_notes().iter().collect::<Vec<_>>();
        let mut result = Self::default();
        let mut used_reread = BTreeSet::new();
        let mut candidate_ordinals = BTreeSet::new();
        let mut candidate_paragraph_ids = BTreeSet::new();
        let mut reread_paragraph_ids = BTreeSet::new();
        if candidate_notes.len() != reread_notes.len() { return Err(EditError::InvalidSelection); }
        for note in &candidate_notes {
            for paragraph in &note.paragraphs {
                if let Some(id) = paragraph.durable_id.as_deref().filter(|value| !value.is_empty()) {
                    if !candidate_paragraph_ids.insert(id.to_owned()) { return Err(EditError::InvalidSelection); }
                }
            }
        }
        for note in &reread_notes {
            for paragraph in &note.paragraphs {
                if let Some(id) = paragraph.durable_id.as_deref().filter(|value| !value.is_empty()) {
                    if !reread_paragraph_ids.insert(id.to_owned()) { return Err(EditError::InvalidSelection); }
                }
            }
        }
        for note in &candidate_notes {
            let mut matches = Vec::new();
            if let Some(ref_id) = note.ref_id.as_deref().filter(|value| !value.is_empty()) {
                matches.extend(reread_notes.iter().enumerate().filter(|(_, actual)| {
                    actual.ref_id.as_deref() == Some(ref_id)
                }).map(|(index, _)| index));
                if matches.is_empty() && reread_notes.iter().any(|actual| {
                    actual.ref_id.as_deref().is_some_and(|value| !value.is_empty())
                }) {
                    let numeric = note.numeric_id.as_deref().filter(|value| !value.is_empty());
                    if numeric.is_none_or(|id| !reread_notes.iter().any(|actual| actual.numeric_id.as_deref() == Some(id))) {
                        return Err(EditError::InvalidSelection);
                    }
                }
            }
            if matches.is_empty() {
                if let Some(numeric_id) = note.numeric_id.as_deref().filter(|value| !value.is_empty()) {
                    matches.extend(reread_notes.iter().enumerate().filter(|(_, actual)| {
                        actual.numeric_id.as_deref() == Some(numeric_id)
                    }).map(|(index, _)| index));
                }
            }
            if matches.is_empty() && note.ref_id.as_deref().is_none_or(str::is_empty)
                && note.numeric_id.as_deref().is_none_or(str::is_empty)
            {
                let Some(ordinal) = note.source_note_ordinal else { return Err(EditError::InvalidSelection) };
                if !candidate_ordinals.insert(ordinal) { return Err(EditError::InvalidSelection); }
                matches.extend(reread_notes.iter().enumerate().filter(|(_, actual)| {
                    actual.source_note_ordinal == Some(ordinal)
                }).map(|(index, _)| index));
            }
            if matches.len() != 1 { return Err(EditError::InvalidSelection); }
            let index = matches[0];
            if !used_reread.insert(index) { return Err(EditError::InvalidSelection); }
            let actual = reread_notes[index];
            if note.numeric_id.as_deref().filter(|value| !value.is_empty())
                != actual.numeric_id.as_deref().filter(|value| !value.is_empty())
                || note.ref_id.as_deref().filter(|value| !value.is_empty())
                    != actual.ref_id.as_deref().filter(|value| !value.is_empty())
            { return Err(EditError::InvalidSelection); }
            if result.notes.insert(note.id.clone(), actual.id.clone()).is_some() {
                return Err(EditError::InvalidSelection);
            }
            let mut paragraph_ids = BTreeSet::new();
            for paragraph in &note.paragraphs {
                let paragraph_matches = if let Some(durable) = paragraph.durable_id.as_deref().filter(|value| !value.is_empty()) {
                    if !paragraph_ids.insert(durable) { return Err(EditError::InvalidSelection); }
                    actual.paragraphs.iter().filter(|candidate| candidate.durable_id.as_deref() == Some(durable)).collect::<Vec<_>>()
                } else {
                    actual.paragraphs.iter().filter(|candidate| candidate.text == paragraph.text).collect::<Vec<_>>()
                };
                let [mapped] = paragraph_matches.as_slice() else { return Err(EditError::InvalidSelection); };
                if result.paragraphs.insert(paragraph.id.clone(), mapped.id.clone()).is_some() {
                    return Err(EditError::InvalidSelection);
                }
            }
            if note.paragraphs.len() != actual.paragraphs.len() { return Err(EditError::InvalidSelection); }
        }
        Ok(result)
    }

    pub fn note_id(&self, candidate_id: &StableId) -> Option<&StableId> {
        self.notes.get(candidate_id)
    }

    pub fn paragraph_id(&self, candidate_id: &StableId) -> Option<&StableId> {
        self.paragraphs.get(candidate_id)
    }

    /// Rebase active text selection through durable note/paragraph identity.
    /// Paragraphs lacking durable IDs are matched only by unique exact content
    /// within their already-mapped note; no vector-position fallback is used.
    pub fn map_selection(
        &self,
        candidate: &Project,
        reread: &Project,
        selection: &super::ScriptNoteSelection,
    ) -> Result<super::ScriptNoteSelection, EditError> {
        use super::{ScriptNoteField, ScriptNoteSelection, ScriptNoteTextPosition};
        let ScriptNoteSelection::Text { note: candidate_id, field, anchor, focus } = selection else {
            return Ok(ScriptNoteSelection::None);
        };
        let Some(mapped_note_id) = self.note_id(candidate_id) else { return Ok(ScriptNoteSelection::None) };
        let old_note = candidate.script_notes().get(candidate_id).ok_or(EditError::InvalidSelection)?;
        let Some(new_note) = reread.script_notes().get(mapped_note_id) else { return Ok(ScriptNoteSelection::None) };
        let map_position = |position: ScriptNoteTextPosition| -> Result<ScriptNoteTextPosition, EditError> {
            match field {
                ScriptNoteField::Title => Ok(ScriptNoteTextPosition { paragraph: 0, offset: clamp_utf8_offset(&new_note.title, position.offset) }),
                ScriptNoteField::Body => {
                    let old_paragraph = old_note.paragraphs.get(position.paragraph).ok_or(EditError::InvalidSelection)?;
                    let new_index = if let Some(durable_id) = old_paragraph.durable_id.as_deref().filter(|id| !id.is_empty()) {
                        let matches = new_note.paragraphs.iter().enumerate().filter(|(_, paragraph)| paragraph.durable_id.as_deref() == Some(durable_id)).map(|(index, _)| index).collect::<Vec<_>>();
                        let [index] = matches.as_slice() else { return Err(EditError::InvalidSelection) };
                        *index
                    } else {
                        let matches = new_note.paragraphs.iter().enumerate().filter(|(_, paragraph)| paragraph.text == old_paragraph.text).map(|(index, _)| index).collect::<Vec<_>>();
                        let [index] = matches.as_slice() else { return Err(EditError::InvalidSelection) };
                        *index
                    };
                    let text = &new_note.paragraphs[new_index].text;
                    Ok(ScriptNoteTextPosition { paragraph: new_index, offset: clamp_utf8_offset(text, position.offset) })
                }
            }
        };
        Ok(ScriptNoteSelection::Text { note: mapped_note_id.clone(), field: *field, anchor: map_position(*anchor)?, focus: map_position(*focus)? })
    }
}

fn clamp_utf8_offset(text: &str, mut offset: usize) -> usize {
    offset = offset.min(text.len());
    while !text.is_char_boundary(offset) { offset -= 1; }
    offset
}
