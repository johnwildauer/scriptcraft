//! Project-owned semantic ScriptNotes, independent of FDX and presentation state.

use crate::{
    formatted_text::{normalize_runs, runs::InlineColor, FormattedTextError},
    formatting::SourceParagraphOverrides,
    identity::StableId,
    TextRun,
};

/// Exact raw FDX value used for a ScriptNote's theme-default color.
pub const DEFAULT_SCRIPT_NOTE_COLOR_RAW: &str = "#RRRRGGGGBBBB";

/// Read-only semantic projection of a ScriptNote's retained raw Color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptNoteColorProjection {
    ThemeDefault,
    Explicit(InlineColor),
    Unsupported,
}

/// Ordered half-open range in flattened Body UTF-16 code units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptNoteRange {
    start: usize,
    end: usize,
}

impl ScriptNoteRange {
    pub const fn new(start: usize, end: usize) -> Option<Self> {
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    pub const fn start(self) -> usize {
        self.start
    }

    pub const fn end(self) -> usize {
        self.end
    }
}

/// One logical replacement in flattened Body UTF-16 coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyTextReplacement {
    start: usize,
    end: usize,
    inserted_utf16_len: usize,
    pre_edit_document_len: usize,
}

impl BodyTextReplacement {
    /// Construct a replacement only when its removed interval is bounded by the old Body.
    pub const fn new(
        start: usize,
        end: usize,
        inserted_utf16_len: usize,
        pre_edit_document_len: usize,
    ) -> Option<Self> {
        if start <= end && end <= pre_edit_document_len {
            Some(Self {
                start,
                end,
                inserted_utf16_len,
                pre_edit_document_len,
            })
        } else {
            None
        }
    }

    pub const fn insertion(
        position: usize,
        inserted_utf16_len: usize,
        pre_edit_document_len: usize,
    ) -> Option<Self> {
        Self::new(
            position,
            position,
            inserted_utf16_len,
            pre_edit_document_len,
        )
    }

    pub const fn deletion(
        start: usize,
        end: usize,
        pre_edit_document_len: usize,
    ) -> Option<Self> {
        Self::new(start, end, 0, pre_edit_document_len)
    }

    pub const fn start(self) -> usize {
        self.start
    }

    pub const fn end(self) -> usize {
        self.end
    }

    pub const fn inserted_utf16_len(self) -> usize {
        self.inserted_utf16_len
    }

    pub const fn pre_edit_document_len(self) -> usize {
        self.pre_edit_document_len
    }

    pub fn post_edit_document_len(self) -> Option<usize> {
        self.pre_edit_document_len
            .checked_sub(self.end.checked_sub(self.start)?)?
            .checked_add(self.inserted_utf16_len)
    }
}

/// One semantically rich Body paragraph in a ScriptNote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNoteParagraph {
    pub id: StableId,
    pub durable_id: Option<String>,
    /// Import-only source coordinate; never a durable or presentation identity.
    pub source_paragraph_ordinal: Option<usize>,
    pub text: String,
    pub runs: Vec<TextRun>,
    pub direct_settings: SourceParagraphOverrides,
}

impl ScriptNoteParagraph {
    /// Construct the canonical empty paragraph used by an empty note Body.
    pub fn empty(
        id: StableId,
        durable_id: Option<String>,
        source_paragraph_ordinal: Option<usize>,
        direct_settings: SourceParagraphOverrides,
    ) -> Self {
        Self {
            id,
            durable_id,
            source_paragraph_ordinal,
            text: String::new(),
            runs: Vec::new(),
            direct_settings,
        }
    }

    /// Normalize the paragraph's runs using the shared canonical run normalizer.
    pub fn normalize(&mut self) -> Result<(), FormattedTextError> {
        self.runs = normalize_runs(&self.text, &self.runs)?;
        Ok(())
    }
}

/// Semantic note content and durable/source identity metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNote {
    pub id: StableId,
    /// Durable numeric FDX Id, retained lexically for source fidelity.
    pub numeric_id: Option<String>,
    pub ref_id: Option<String>,
    /// Import-only source coordinate, not note ordering or identity.
    pub source_note_ordinal: Option<usize>,
    pub range: ScriptNoteRange,
    /// Raw source Color value, retained even when the palette does not recognize it.
    pub color: String,
    pub title: String,
    /// `None` represents absent/cleared Type; empty strings are not semantic values.
    pub note_type: Option<String>,
    pub writer_id: String,
    pub writer_name: String,
    pub date_time: String,
    pub date_modified: String,
    pub paragraphs: Vec<ScriptNoteParagraph>,
}

impl ScriptNote {
    /// Resolve durable identity without falling back to source order or vector position.
    pub fn durable_identity(&self) -> Option<ScriptNoteIdentity<'_>> {
        self.ref_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(ScriptNoteIdentity::RefId)
            .or_else(|| {
                self.numeric_id
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .map(ScriptNoteIdentity::NumericId)
            })
    }

    /// Read-only known-color projection; unsupported source spellings remain unresolved.
    pub fn known_color(&self) -> Option<InlineColor> {
        InlineColor::from_normalized_fdx_value(&self.color)
    }

    /// Distinguish an absent/empty or sentinel theme default, an explicit
    /// palette color, and a nonempty source spelling outside the supported palette.
    pub fn color_projection(&self) -> ScriptNoteColorProjection {
        if self.color.is_empty() || self.color == DEFAULT_SCRIPT_NOTE_COLOR_RAW {
            ScriptNoteColorProjection::ThemeDefault
        } else if let Some(color) = self.known_color() {
            ScriptNoteColorProjection::Explicit(color)
        } else {
            ScriptNoteColorProjection::Unsupported
        }
    }
}

/// Resolved durable note identity, preserving which source field supplied it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScriptNoteIdentity<'a> {
    RefId(&'a str),
    NumericId(&'a str),
}

/// Creation/source-ordered semantic notes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptNotes {
    notes: Vec<ScriptNote>,
}

impl ScriptNotes {
    pub fn new(notes: Vec<ScriptNote>) -> Self {
        Self { notes }
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ScriptNote> {
        self.notes.iter()
    }

    pub fn get(&self, id: &StableId) -> Option<&ScriptNote> {
        self.notes.iter().find(|note| &note.id == id)
    }

    pub(crate) fn as_slice(&self) -> &[ScriptNote] {
        &self.notes
    }

    /// Returns a complete cloned collection with ranges replaced by stable note identity.
    /// Every requested identity must resolve exactly once; source and creation order remain
    /// unchanged, including when distinct notes receive the same range.
    pub(crate) fn with_replaced_ranges(
        &self,
        replacements: &[(StableId, ScriptNoteRange)],
    ) -> Option<Self> {
        for (index, (id, _)) in replacements.iter().enumerate() {
            if replacements[..index]
                .iter()
                .any(|(previous_id, _)| previous_id == id)
                || self.notes.iter().filter(|note| &note.id == id).count() != 1
            {
                return None;
            }
        }
        let mut notes = self.clone();
        let mut applied = vec![false; replacements.len()];
        for note in &mut notes.notes {
            for (index, (id, range)) in replacements.iter().enumerate() {
                if &note.id == id {
                    if applied[index] {
                        return None;
                    }
                    note.range = *range;
                    applied[index] = true;
                }
            }
        }
        applied.iter().all(|was_applied| *was_applied).then_some(notes)
    }

    /// Rebase all note ranges through one flattened Body replacement.
    ///
    /// Insertion at/before a nonzero start shifts both bounds; insertion strictly
    /// inside expands only the end; insertion at end/after is inert. At a point
    /// note, insertion at the point moves it, while later insertion does not.
    /// Deletion wholly before leaves a note unchanged, wholly after shifts it,
    /// boundary-touching deletion clips only the intersecting side, and overlap
    /// clips either edge. Covering a note or the complete document collapses it
    /// to the deletion start, never deleting the note. A point at deletion start
    /// stays; one at deletion end or strictly inside collapses to the start.
    /// Replacement deletes before inserting, so fully replaced notes collapse
    /// then move to the post-insertion boundary. All coordinates count UTF-16:
    /// supplementary characters before/inside edits are handled by the shared
    /// Body mapper, which rejects surrogate-interior boundaries.
    pub fn rebase_body_text(&self, replacement: BodyTextReplacement) -> Option<Self> {
        let post_edit_len = replacement.post_edit_document_len()?;
        let mut rebased = self.clone();
        for note in &mut rebased.notes {
            let range = note.range;
            if range.end > replacement.pre_edit_document_len {
                return None;
            }

            let (mut start, mut end) = (range.start, range.end);
            if replacement.start < replacement.end {
                let removed_len = replacement.end.checked_sub(replacement.start)?;
                if end <= replacement.start {
                    // Deletion wholly after this range, including a point at A.
                } else if start >= replacement.end {
                    start = start.checked_sub(removed_len)?;
                    end = end.checked_sub(removed_len)?;
                } else {
                    start = if start < replacement.start {
                        start
                    } else {
                        replacement.start
                    };
                    end = if end > replacement.end {
                        end.checked_sub(removed_len)?
                    } else {
                        replacement.start
                    };
                }
            }

            if replacement.inserted_utf16_len > 0 {
                if replacement.start <= start {
                    start = start.checked_add(replacement.inserted_utf16_len)?;
                    end = end.checked_add(replacement.inserted_utf16_len)?;
                } else if replacement.start < end {
                    end = end.checked_add(replacement.inserted_utf16_len)?;
                }
            }
            if start > end || end > post_edit_len {
                return None;
            }
            note.range = ScriptNoteRange::new(start, end)?;
        }
        Some(rebased)
    }
}
