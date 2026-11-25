//! Disposable cache for measured ScriptNote text geometry.

use scriptcraft_layout::{LayoutEnvironment, ScriptNoteLayout, ScriptNoteMetrics, SCRIPT_NOTE_BODY_PLACEHOLDER, SCRIPT_NOTE_TITLE_PLACEHOLDER};
use scriptcraft_model::{ScriptNote, ScriptNoteParagraph, StableId, TextRun};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParagraphFlowKey {
    text: String,
    runs: Vec<TextRun>,
    direct_settings: scriptcraft_model::formatting::SourceParagraphOverrides,
}

impl From<&ScriptNoteParagraph> for ParagraphFlowKey {
    fn from(paragraph: &ScriptNoteParagraph) -> Self {
        Self {
            text: paragraph.text.clone(),
            runs: paragraph.runs.clone(),
            direct_settings: paragraph.direct_settings.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedFaceKey {
    id: String,
    bytes: Vec<u8>,
    index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NoteLayoutKey {
    title: String,
    paragraphs: Vec<ParagraphFlowKey>,
    metrics: ScriptNoteMetrics,
    environment: LayoutEnvironment,
    prepared_faces: Vec<PreparedFaceKey>,
}

#[derive(Debug, Default)]
pub(crate) struct ScriptNoteLayoutCache {
    entries: HashMap<StableId, (NoteLayoutKey, ScriptNoteCardLayouts)>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptNoteCardLayouts {
    pub(crate) actual: ScriptNoteLayout,
    pub(crate) placeholder: Option<ScriptNoteLayout>,
}

impl ScriptNoteLayoutCache {
    pub(crate) fn get_or_build(
        &mut self,
        note: &ScriptNote,
        environment: &LayoutEnvironment,
        prepared: &crate::presentation::document_font::PreparedDocumentFonts,
        metrics: ScriptNoteMetrics,
    ) -> Result<ScriptNoteCardLayouts, scriptcraft_layout::LayoutError> {
        let prepared_faces = prepared
            .resolved_ids()
            .filter_map(|id| Some(PreparedFaceKey {
                id: id.as_str().to_owned(),
                bytes: prepared.face_bytes(id)?.as_ref().to_vec(),
                index: prepared.face_index(id)?,
            }))
            .collect();
        let key = NoteLayoutKey {
            title: note.title.clone(),
            paragraphs: note.paragraphs.iter().map(ParagraphFlowKey::from).collect(),
            metrics,
            environment: environment.clone(),
            prepared_faces,
        };
        if let Some((old_key, layout)) = self.entries.get(&note.id) {
            if old_key == &key {
                return Ok(layout.clone());
            }
        }
        let actual = ScriptNoteLayout::new(note, environment, metrics)?;
        let placeholder = if note.title.is_empty() || actual.body_is_empty {
            let mut sample = note.clone();
            if sample.title.is_empty() {
                sample.title = SCRIPT_NOTE_TITLE_PLACEHOLDER.to_owned();
            }
            if actual.body_is_empty {
                if let Some(first) = sample.paragraphs.first_mut() {
                    first.text = SCRIPT_NOTE_BODY_PLACEHOLDER.to_owned();
                    first.runs.clear();
                    sample.paragraphs.truncate(1);
                } else {
                    let mut paragraph = ScriptNoteParagraph::empty(
                        note.id.clone(), None, None, Default::default(),
                    );
                    paragraph.text = SCRIPT_NOTE_BODY_PLACEHOLDER.to_owned();
                    sample.paragraphs.push(paragraph);
                }
            }
            Some(ScriptNoteLayout::new(&sample, environment, metrics)?)
        } else {
            None
        };
        let layouts = ScriptNoteCardLayouts { actual, placeholder };
        if self.entries.len() >= 64 {
            self.entries.clear();
        }
        self.entries.insert(note.id.clone(), (key, layouts.clone()));
        Ok(layouts)
    }

    pub(crate) fn retain_notes(&mut self, notes: impl Iterator<Item = StableId>) {
        let retained = notes.collect::<std::collections::HashSet<_>>();
        self.entries.retain(|id, _| retained.contains(id));
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}
