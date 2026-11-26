//! Transactional ScriptNote editing over detached Project candidates.

use super::{history::Snapshot, EditError, EditingSession};
use crate::editing::rich_paragraph::{
    attributes_at, compose_replacement_chunks, effective_runs, next_boundary, previous_boundary,
    set_normalized_text, slice_runs,
};
use scriptcraft_model::body_offsets::selection_to_flat_offsets;
use scriptcraft_model::formatting::{
    FixedPoint, ParagraphAlignment, ParagraphLeading, ParagraphSpacing, SourceParagraphOverrides,
};
use scriptcraft_model::formatted_text::InlineColor;
use scriptcraft_model::formatted_text::{
    clear_supported_styles_over_ranges, parse_style_attribute, project_style_at_caret,
    set_inline_run_formatting_for_range, toggle_style_over_ranges, InlineRunFormattingPatch,
    RunBoundaryAffinity, RunStyle, RunStyleFlags, RunStyleRange,
};
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::preservation::{
    AnchorTarget, ForeignIdDomain, ForeignIdentity, ReferencePolicy,
};
use scriptcraft_model::{
    Project, RunAttribute, ScriptNote, ScriptNoteParagraph, ScriptNoteRange, ScriptNotes, StableId,
    DEFAULT_SCRIPT_NOTE_COLOR_RAW, MAX_TEXT_BYTES,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptNoteField { Title, Body }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptNoteTextPosition { pub paragraph: usize, pub offset: usize }

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScriptNoteSelection {
    #[default]
    None,
    Text { note: StableId, field: ScriptNoteField, anchor: ScriptNoteTextPosition, focus: ScriptNoteTextPosition },
}

impl ScriptNoteSelection {
    pub fn ordered(&self) -> Option<(ScriptNoteTextPosition, ScriptNoteTextPosition)> {
        match self {
            Self::Text { anchor, focus, .. } => Some(if anchor <= focus { (*anchor, *focus) } else { (*focus, *anchor) }),
            Self::None => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptNoteHorizontalDirection { Left, Right }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptNoteLineEdge { Start, End }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptNoteTextEditOperation {
    Insert { text: String },
    Paste { text: String },
    DeleteBackward,
    DeleteForward,
    Enter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNoteCreationInput {
    pub color: Option<InlineColor>,
    pub note_type: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptNoteColorUpdate {
    Preserve,
    ThemeDefault,
    Explicit(InlineColor),
}

pub(super) fn validate_script_note_selection(project: &Project, selection: &ScriptNoteSelection) -> Result<(), EditError> {
    let ScriptNoteSelection::Text { note, field, anchor, focus } = selection else { return Ok(()) };
    let note = project.script_notes().get(note).ok_or(EditError::InvalidSelection)?;
    let valid = |position: ScriptNoteTextPosition| -> Result<(), EditError> {
        let text = match field {
            ScriptNoteField::Title if position.paragraph == 0 => Some(note.title.as_str()),
            ScriptNoteField::Body => note.paragraphs.get(position.paragraph).map(|p| p.text.as_str()),
            _ => None,
        }.ok_or(EditError::InvalidSelection)?;
        if position.offset > text.len() || !text.is_char_boundary(position.offset) {
            return Err(EditError::InvalidUtf8Boundary);
        }
        Ok(())
    };
    valid(*anchor)?;
    valid(*focus)
}

fn generated_note_settings() -> SourceParagraphOverrides {
    let mut settings = SourceParagraphOverrides::default();
    settings.paragraph.alignment = Some(ParagraphAlignment::Left);
    settings.paragraph.first_indent = Some(FixedPoint::from_raw(0));
    settings.paragraph.left_indent = Some(FixedPoint::from_raw(0));
    settings.paragraph.right_indent = Some(FixedPoint::from_raw(61_200)); // 8.50 points.
    settings.paragraph.leading = Some(ParagraphLeading::Regular);
    settings.paragraph.space_before = Some(FixedPoint::from_raw(0));
    settings.paragraph.spacing = Some(ParagraphSpacing::Single);
    settings.starts_new_page = Some(false);
    settings
}

fn generated_identity_allocator(project: &Project) -> ParagraphIdAllocator {
    let mut used = BTreeSet::new();
    if let Some(state) = &project.preservation {
        used.extend(state.identities.iter().map(|identity| identity.value.clone()));
        used.extend(state.board_graph.entities.iter().map(|entity| entity.identity.value.clone()));
        for reference in &state.references {
            used.insert(reference.source.value.clone());
            used.insert(reference.target.value.clone());
        }
        for reference in &state.board_graph.references {
            used.insert(reference.source.value.clone());
            used.insert(reference.target.value.clone());
        }
        for section in &state.sections { collect_raw_ids(&section.children, &mut used); }
    }
    for owner in project.elements.iter().filter_map(|element| element.source_provenance.as_ref())
        .chain(project.title_page.iter().filter_map(|paragraph| paragraph.source_provenance.as_ref())) {
        used.extend(owner.foreign_id.iter().cloned());
    }
    used.extend(project.dual_dialogue_groups.iter().filter_map(|group| group.wrapper.outer_foreign_id.clone()));
    for note in project.script_notes().iter() {
        used.extend(note.numeric_id.iter().cloned());
        used.extend(note.ref_id.iter().cloned());
        used.extend(note.paragraphs.iter().filter_map(|paragraph| paragraph.durable_id.clone()));
    }
    ParagraphIdAllocator::new(used)
}

fn collect_raw_ids(nodes: &[scriptcraft_model::preservation::PreservedNode], used: &mut BTreeSet<String>) {
    for node in nodes {
        if let scriptcraft_model::preservation::PreservedNode::Element(element) = node {
            used.extend(element.attributes.iter().filter(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && (attribute.name.local_name.eq_ignore_ascii_case("id")
                        || attribute.name.local_name.eq_ignore_ascii_case("refid"))
            }).map(|attribute| attribute.value.clone()));
            collect_raw_ids(&element.children, used);
        }
    }
}

fn collect_raw_note_ids(nodes: &[scriptcraft_model::preservation::PreservedNode], used: &mut BTreeSet<String>) {
    for node in nodes {
        if let scriptcraft_model::preservation::PreservedNode::Element(element) = node {
            if element.name.local_name == "ScriptNote" {
                used.extend(element.attributes.iter().filter(|attribute| {
                    attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Id"
                }).map(|attribute| attribute.value.clone()));
            }
            collect_raw_note_ids(&element.children, used);
        }
    }
}

fn register_new_note_paragraph(project: &mut Project, durable_id: &str) -> Result<(), EditError> {
    let Some(state) = project.preservation.as_mut() else { return Ok(()) };
    let identity = ForeignIdentity::new(ForeignIdDomain::Paragraph, durable_id.to_owned()).map_err(EditError::Preservation)?;
    if !state.identities.contains(&identity) { state.identities.push(identity); }
    state.rebuild_indexes();
    Ok(())
}

fn unregister_deleted_note_paragraphs(project: &mut Project, note: &ScriptNote) -> Result<(), EditError> {
    let Some(state) = project.preservation.as_mut() else { return Ok(()) };
    let mut deleted = BTreeSet::new();
    for paragraph in &note.paragraphs {
        if let Some(id) = paragraph.durable_id.as_deref().filter(|id| !id.is_empty()) {
            deleted.insert(ForeignIdentity::new(ForeignIdDomain::Paragraph, id.to_owned()).map_err(EditError::Preservation)?);
        }
    }
    let note_owner = note.numeric_id.as_deref().filter(|id| !id.is_empty())
        .map(|id| ForeignIdentity::new(ForeignIdDomain::Other("ScriptNote".to_owned()), id.to_owned()))
        .transpose().map_err(EditError::Preservation)?;
    let owns = |identity: &ForeignIdentity| deleted.contains(identity) || note_owner.as_ref() == Some(identity);
    if state.anchors.iter().any(|anchor| matches!(&anchor.target, AnchorTarget::Foreign(target) if owns(target)))
        || state.references.iter().any(|reference| {
            !owns(&reference.source) && owns(&reference.target) && reference.policy == ReferencePolicy::RequiredInternal
        })
        || state.board_graph.references.iter().any(|reference| {
            !owns(&reference.source) && owns(&reference.target) && reference.policy == ReferencePolicy::RequiredInternal
        })
        || state.board_graph.entities.iter().any(|entity| owns(&entity.identity))
    {
        return Err(EditError::Preservation(scriptcraft_model::preservation::PreservationError::DanglingReference));
    }
    state.identities.retain(|identity| !deleted.contains(identity));
    state.references.retain(|reference| !owns(&reference.source));
    state.board_graph.references.retain(|reference| !owns(&reference.source));
    state.rebuild_indexes();
    Ok(())
}

fn numeric_note_id(project: &Project) -> Result<String, EditError> {
    let mut max = None;
    let mut parsed = BTreeSet::new();
    let mut used = project.script_notes().iter().filter_map(|note| note.numeric_id.as_deref())
        .filter(|id| !id.is_empty()).map(str::to_owned).collect::<BTreeSet<_>>();
    let mut raw_note_ids = BTreeSet::new();
    if let Some(state) = &project.preservation {
        used.extend(state.identities.iter().filter(|identity| {
            identity.domain == ForeignIdDomain::Other("ScriptNote".to_owned())
        }).map(|identity| identity.value.clone()));
        for section in &state.sections { collect_raw_note_ids(&section.children, &mut raw_note_ids); }
    }
    for id in &used {
        if id.bytes().all(|byte| byte.is_ascii_digit()) {
            let value = id.parse::<u64>().map_err(|_| EditError::InvalidElement)?;
            max = Some(max.map_or(value, |current: u64| current.max(value)));
            parsed.insert(value);
        }
    }
    let next = match max { Some(value) => value.checked_add(1).ok_or(EditError::InvalidElement)?, None => 0 };
    let lexical = next.to_string();
    if parsed.contains(&next) || used.contains(&lexical) || raw_note_ids.contains(&lexical) {
        return Err(EditError::InvalidElement);
    }
    Ok(lexical)
}

impl EditingSession {
    pub fn script_note_selection(&self) -> &ScriptNoteSelection { &self.script_note_selection }

    pub fn script_note_selection_is_valid(&self) -> bool {
        validate_script_note_selection(&self.project, &self.script_note_selection).is_ok()
    }

    /// One publication boundary for semantic note edits and selection-only changes.
    pub(super) fn publish_script_note(&mut self, candidate: Project, selection: ScriptNoteSelection) -> Result<(), EditError> {
        candidate.validate()?;
        validate_script_note_selection(&candidate, &selection)?;
        let semantic_change = candidate != self.project;
        let selection_change = selection != self.script_note_selection;
        if !semantic_change && !selection_change { return Ok(()); }
        let previous_selection = self.script_note_selection.clone();
        if semantic_change {
            self.push_history(Snapshot {
                project: self.project.clone(), selection: self.selection, title_selection: self.title_selection,
                beat_selection: self.beat_selection.clone(), beat_typing_style: self.beat_typing_style,
                beat_typing_formatting: self.beat_typing_formatting,
                script_note_typing_style: self.script_note_typing_style,
                script_note_typing_formatting: self.script_note_typing_formatting,
                title_typing_style: self.title_typing_style,
                body_typing_style: self.body_typing_style, body_typing_formatting: self.body_typing_formatting,
                generated_parentheticals: self.generated_parentheticals.clone(),
            });
            self.project = candidate;
            self.redo.clear();
            self.redo_note_selections.clear();
            self.revision = self.revision.saturating_add(1);
            self.dirty = self.project != self.clean_project;
            self.validated_baseline = true;
        }
        self.script_note_selection = selection;
        self.mark_mutated();
        self.update_script_note_typing_from_selection(&previous_selection)?;
        Ok(())
    }

    fn update_script_note_typing_from_selection(&mut self, previous: &ScriptNoteSelection) -> Result<(), EditError> {
        let old_style = self.script_note_typing_style;
        let old_formatting = self.script_note_typing_formatting;
        let style = match &self.script_note_selection {
            ScriptNoteSelection::Text { note, field: ScriptNoteField::Body, anchor, focus } if anchor == focus => {
                let paragraph = self.project.script_notes().get(note)
                    .and_then(|note| note.paragraphs.get(focus.paragraph)).ok_or(EditError::InvalidSelection)?;
                project_style_at_caret(&paragraph.text, &paragraph.runs, focus.offset,
                    RunBoundaryAffinity::Right, old_style).map_err(EditError::FormattedText)?
            }
            _ => RunStyleFlags::default(),
        };
        self.script_note_typing_style = style;
        let same_body_caret = matches!((previous, &self.script_note_selection),
            (ScriptNoteSelection::Text { note: old_note, field: ScriptNoteField::Body, anchor: old_anchor, focus: old_focus },
             ScriptNoteSelection::Text { note, field: ScriptNoteField::Body, anchor, focus })
             if old_note == note && old_anchor == old_focus && anchor == focus && old_focus == focus);
        if !same_body_caret {
            self.script_note_typing_formatting = InlineRunFormattingPatch::default();
        }
        if old_style != self.script_note_typing_style || old_formatting != self.script_note_typing_formatting {
            self.mark_mutated();
        }
        Ok(())
    }

    /// Toggle a supported inline style in an open ScriptNote Body selection.
    /// A collapsed caret updates only ephemeral typing state.
    pub fn toggle_script_note_run_style(&mut self, style: RunStyle, timestamp: String) -> Result<(), EditError> {
        let ScriptNoteSelection::Text { note: note_id, field: ScriptNoteField::Body, anchor, focus } = self.script_note_selection.clone()
            else { return Err(EditError::InvalidSelection) };
        if !self.script_note_selection_is_valid() { return Err(EditError::InvalidSelection); }
        if anchor == focus {
            let paragraph = self.project.script_notes().get(&note_id)
                .and_then(|note| note.paragraphs.get(focus.paragraph)).ok_or(EditError::InvalidSelection)?;
            let current = project_style_at_caret(&paragraph.text, &paragraph.runs, focus.offset,
                RunBoundaryAffinity::Right, self.script_note_typing_style).map_err(EditError::FormattedText)?;
            self.script_note_typing_style = current.toggled(style);
            self.mark_mutated();
            return Ok(());
        }
        let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let source = self.project.script_notes().get(&note_id).ok_or(EditError::InvalidSelection)?;
        if end.paragraph >= source.paragraphs.len() { return Err(EditError::InvalidSelection); }
        let ranges = (start.paragraph..=end.paragraph).map(|index| {
            let paragraph = &source.paragraphs[index];
            RunStyleRange { canonical_text: &paragraph.text, runs: &paragraph.runs,
                range: (if index == start.paragraph { start.offset } else { 0 })..
                    (if index == end.paragraph { end.offset } else { paragraph.text.len() }) }
        }).collect::<Vec<_>>();
        let edits = toggle_style_over_ranges(&ranges, self.script_note_typing_style, style)
            .map_err(EditError::FormattedText)?;
        let mut candidate = self.project.clone();
        let mut notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
        let note = notes.iter_mut().find(|note| note.id == note_id).ok_or(EditError::InvalidSelection)?;
        let mut changed = false;
        for (offset, edit) in edits.into_iter().enumerate() {
            let paragraph = &mut note.paragraphs[start.paragraph + offset];
            let normalized = effective_runs(&paragraph.text, &edit.runs);
            changed |= normalized != effective_runs(&paragraph.text, &paragraph.runs);
            paragraph.runs = normalized;
        }
        if !changed { return Ok(()); }
        note.date_modified = timestamp;
        candidate.replace_script_notes(ScriptNotes::new(notes))?;
        self.publish_script_note(candidate, self.script_note_selection.clone())
    }

    /// Clear supported run styles while preserving unknown tokens and attributes.
    pub fn clear_script_note_run_styles(&mut self, timestamp: String) -> Result<(), EditError> {
        let ScriptNoteSelection::Text { note: note_id, field: ScriptNoteField::Body, anchor, focus } = self.script_note_selection.clone()
            else { return Err(EditError::InvalidSelection) };
        if !self.script_note_selection_is_valid() { return Err(EditError::InvalidSelection); }
        if anchor == focus {
            if self.script_note_typing_style != RunStyleFlags::default() {
                self.script_note_typing_style = RunStyleFlags::default();
                self.mark_mutated();
            }
            return Ok(());
        }
        let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let source = self.project.script_notes().get(&note_id).ok_or(EditError::InvalidSelection)?;
        if end.paragraph >= source.paragraphs.len() { return Err(EditError::InvalidSelection); }
        let ranges = (start.paragraph..=end.paragraph).map(|index| {
            let paragraph = &source.paragraphs[index];
            RunStyleRange { canonical_text: &paragraph.text, runs: &paragraph.runs,
                range: (if index == start.paragraph { start.offset } else { 0 })..
                    (if index == end.paragraph { end.offset } else { paragraph.text.len() }) }
        }).collect::<Vec<_>>();
        let edits = clear_supported_styles_over_ranges(&ranges).map_err(EditError::FormattedText)?;
        let mut candidate = self.project.clone();
        let mut notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
        let note = notes.iter_mut().find(|note| note.id == note_id).ok_or(EditError::InvalidSelection)?;
        let mut changed = false;
        for (offset, edit) in edits.into_iter().enumerate() {
            let paragraph = &mut note.paragraphs[start.paragraph + offset];
            let normalized = effective_runs(&paragraph.text, &edit.runs);
            changed |= normalized != effective_runs(&paragraph.text, &paragraph.runs);
            paragraph.runs = normalized;
        }
        if !changed { return Ok(()); }
        note.date_modified = timestamp;
        candidate.replace_script_notes(ScriptNotes::new(notes))?;
        self.publish_script_note(candidate, self.script_note_selection.clone())
    }

    /// Set bounded inline attributes over a ScriptNote Body range, or merge
    /// them into collapsed-caret typing state without changing the Project.
    pub fn set_script_note_run_formatting(&mut self, patch: InlineRunFormattingPatch, timestamp: String) -> Result<(), EditError> {
        if patch.is_empty() { return Ok(()); }
        let ScriptNoteSelection::Text { note: note_id, field: ScriptNoteField::Body, anchor, focus } = self.script_note_selection.clone()
            else { return Err(EditError::InvalidSelection) };
        if !self.script_note_selection_is_valid() { return Err(EditError::InvalidSelection); }
        if anchor == focus {
            let mut next = self.script_note_typing_formatting;
            if let Some(value) = patch.font { next.font = Some(value); }
            if let Some(value) = patch.size { next.size = Some(value); }
            if let Some(value) = patch.color { next.color = Some(value); }
            if let Some(value) = patch.background { next.background = Some(value); }
            if next != self.script_note_typing_formatting {
                self.script_note_typing_formatting = next;
                self.mark_mutated();
            }
            return Ok(());
        }
        let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let source = self.project.script_notes().get(&note_id).ok_or(EditError::InvalidSelection)?;
        if end.paragraph >= source.paragraphs.len() { return Err(EditError::InvalidSelection); }
        let mut candidate = self.project.clone();
        let mut notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
        let note = notes.iter_mut().find(|note| note.id == note_id).ok_or(EditError::InvalidSelection)?;
        let mut changed = false;
        for index in start.paragraph..=end.paragraph {
            let paragraph = &mut note.paragraphs[index];
            let range = (if index == start.paragraph { start.offset } else { 0 })..
                (if index == end.paragraph { end.offset } else { paragraph.text.len() });
            if range.start > range.end || range.end > paragraph.text.len()
                || !paragraph.text.is_char_boundary(range.start) || !paragraph.text.is_char_boundary(range.end) {
                return Err(EditError::InvalidUtf8Boundary);
            }
            if range.is_empty() { continue; }
            let source_runs = effective_runs(&paragraph.text, &paragraph.runs);
            let normalized = set_inline_run_formatting_for_range(&paragraph.text, &source_runs, range, patch)
                .map_err(EditError::FormattedText)?;
            changed |= normalized != source_runs;
            paragraph.runs = normalized;
        }
        if !changed { return Ok(()); }
        note.date_modified = timestamp;
        candidate.replace_script_notes(ScriptNotes::new(notes))?;
        self.publish_script_note(candidate, self.script_note_selection.clone())
    }

    pub fn set_script_note_selection(&mut self, selection: ScriptNoteSelection) -> Result<(), EditError> {
        self.publish_script_note(self.project.clone(), selection)
    }

    pub fn create_script_note(&mut self, input: ScriptNoteCreationInput) -> Result<StableId, EditError> {
        let (start, end) = selection_to_flat_offsets(&self.project, self.selection).ok_or(EditError::InvalidSelection)?;
        let range = ScriptNoteRange::new(start, end).ok_or(EditError::InvalidSelection)?;
        let ScriptNoteCreationInput { color, note_type, timestamp } = input;
        let mut candidate = self.project.clone();
        let mut stable_ids = candidate.stable_id_allocator()?;
        let stable_id = stable_ids.allocate("script-note")?;
        let paragraph_stable_id = stable_ids.allocate("script-note-paragraph")?;
        let mut durable = generated_identity_allocator(&candidate);
        let ref_id = durable.allocate().map_err(|_| EditError::InvalidElement)?;
        let paragraph_durable = durable.allocate().map_err(|_| EditError::InvalidElement)?;
        let numeric_id = numeric_note_id(&candidate)?;
        let paragraph = ScriptNoteParagraph::empty(paragraph_stable_id, Some(paragraph_durable.clone()), None, generated_note_settings());
        let note = ScriptNote {
            id: stable_id.clone(), numeric_id: Some(numeric_id), ref_id: Some(ref_id), source_note_ordinal: None,
            range, color: color.map_or_else(|| DEFAULT_SCRIPT_NOTE_COLOR_RAW.to_owned(), |color| color.fdx_value().to_owned()), title: String::new(), note_type,
            writer_id: String::new(), writer_name: String::new(), date_time: timestamp.clone(),
            date_modified: timestamp, paragraphs: vec![paragraph],
        };
        let mut notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
        notes.push(note);
        candidate.replace_script_notes(ScriptNotes::new(notes))?;
        register_new_note_paragraph(&mut candidate, &paragraph_durable)?;
        self.publish_script_note(candidate, ScriptNoteSelection::Text {
            note: stable_id.clone(), field: ScriptNoteField::Body,
            anchor: ScriptNoteTextPosition { paragraph: 0, offset: 0 },
            focus: ScriptNoteTextPosition { paragraph: 0, offset: 0 },
        })?;
        Ok(stable_id)
    }

    pub fn delete_script_note(&mut self, note_id: StableId) -> Result<(), EditError> {
        let mut candidate = self.project.clone();
        let notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
        let index = notes.iter().position(|note| note.id == note_id).ok_or(EditError::InvalidSelection)?;
        let deleted = notes[index].clone();
        unregister_deleted_note_paragraphs(&mut candidate, &deleted)?;
        let mut retained = notes;
        retained.remove(index);
        candidate.replace_script_notes(ScriptNotes::new(retained))?;
        self.publish_script_note(candidate, ScriptNoteSelection::None)
    }

    pub fn set_script_note_properties(&mut self, note_id: StableId, color: ScriptNoteColorUpdate, note_type: Option<String>, timestamp: String) -> Result<(), EditError> {
        let mut candidate = self.project.clone();
        let mut notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
        let note = notes.iter_mut().find(|note| note.id == note_id).ok_or(EditError::InvalidSelection)?;
        let color = match color {
            ScriptNoteColorUpdate::Preserve => note.color.clone(),
            ScriptNoteColorUpdate::ThemeDefault => DEFAULT_SCRIPT_NOTE_COLOR_RAW.to_owned(),
            ScriptNoteColorUpdate::Explicit(color) => color.fdx_value().to_owned(),
        };
        if note.color == color && note.note_type == note_type { return Ok(()); }
        note.color = color;
        note.note_type = note_type;
        note.date_modified = timestamp;
        candidate.replace_script_notes(ScriptNotes::new(notes))?;
        self.publish_script_note(candidate, self.script_note_selection.clone())
    }

    pub fn selected_script_note_text(&self) -> Result<Option<String>, EditError> {
        validate_script_note_selection(&self.project, &self.script_note_selection)?;
        let ScriptNoteSelection::Text { note, field, anchor, focus } = &self.script_note_selection else { return Ok(None) };
        let note = self.project.script_notes().get(note).ok_or(EditError::InvalidSelection)?;
        let (start, end) = if anchor <= focus { (*anchor, *focus) } else { (*focus, *anchor) };
        match field {
            ScriptNoteField::Title => Ok(Some(note.title[start.offset..end.offset].to_owned())),
            ScriptNoteField::Body => {
                let mut selected = String::new();
                for index in start.paragraph..=end.paragraph {
                    if index > start.paragraph { selected.push('\n'); }
                    let paragraph = note.paragraphs.get(index).ok_or(EditError::InvalidSelection)?;
                    let from = if index == start.paragraph { start.offset } else { 0 };
                    let to = if index == end.paragraph { end.offset } else { paragraph.text.len() };
                    selected.push_str(&paragraph.text[from..to]);
                }
                Ok(Some(selected))
            }
        }
    }

    pub fn select_all_script_note_text(&mut self, field: ScriptNoteField) -> Result<(), EditError> {
        let (note_id, note) = match &self.script_note_selection {
            ScriptNoteSelection::Text { note, .. } => (note.clone(), self.project.script_notes().get(note)),
            ScriptNoteSelection::None => return Err(EditError::InvalidSelection),
        };
        let note = note.ok_or(EditError::InvalidSelection)?;
        let (anchor, focus) = match field {
            ScriptNoteField::Title => (ScriptNoteTextPosition { paragraph: 0, offset: 0 }, ScriptNoteTextPosition { paragraph: 0, offset: note.title.len() }),
            ScriptNoteField::Body => {
                let last = note.paragraphs.last().ok_or(EditError::InvalidSelection)?;
                (ScriptNoteTextPosition { paragraph: 0, offset: 0 }, ScriptNoteTextPosition { paragraph: note.paragraphs.len() - 1, offset: last.text.len() })
            }
        };
        self.set_script_note_selection(ScriptNoteSelection::Text { note: note_id, field, anchor, focus })
    }

    pub fn edit_script_note_text(&mut self, operation: ScriptNoteTextEditOperation, timestamp: String) -> Result<(), EditError> {
        let ScriptNoteSelection::Text { note: note_id, field, anchor, focus } = self.script_note_selection.clone() else { return Err(EditError::InvalidSelection) };
        let (mut start, mut end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let typing_style = self.script_note_typing_style;
        let typing_formatting = self.script_note_typing_formatting;
        if operation == ScriptNoteTextEditOperation::Enter && field == ScriptNoteField::Title {
            return self.set_script_note_selection(ScriptNoteSelection::Text {
                note: note_id, field: ScriptNoteField::Body,
                anchor: ScriptNoteTextPosition { paragraph: 0, offset: 0 }, focus: ScriptNoteTextPosition { paragraph: 0, offset: 0 },
            });
        }
        let text = match &operation {
            ScriptNoteTextEditOperation::Insert { text } => {
                if text.contains('\r') || text.contains('\n') { return Err(EditError::InvalidSelection); }
                text.clone()
            }
            ScriptNoteTextEditOperation::Paste { text } if field == ScriptNoteField::Title => {
                text.replace("\r\n", "\n").chars().map(|ch| if matches!(ch, '\r' | '\n') { ' ' } else { ch }).collect()
            }
            ScriptNoteTextEditOperation::Paste { text } => text.replace("\r\n", "\n").replace('\r', "\n"),
            _ => String::new(),
        };
        if text.is_empty() && matches!(operation, ScriptNoteTextEditOperation::Insert { .. } | ScriptNoteTextEditOperation::Paste { .. }) { return Ok(()); }

        let mut candidate = self.project.clone();
        let mut notes = candidate.script_notes().iter().cloned().collect::<Vec<_>>();
        let note_index = notes.iter().position(|note| note.id == note_id).ok_or(EditError::InvalidSelection)?;
        let mut note = notes[note_index].clone();
        let final_position = match field {
            ScriptNoteField::Title => {
                let current = note.title.clone();
                if start.paragraph != 0 || end.paragraph != 0 { return Err(EditError::InvalidSelection); }
                if start == end && text.is_empty() {
                    if operation == ScriptNoteTextEditOperation::DeleteBackward && start.offset > 0 { start.offset = previous_boundary(&current, start.offset); }
                    else if operation == ScriptNoteTextEditOperation::DeleteForward && start.offset < current.len() { end.offset = next_boundary(&current, start.offset); }
                    else { return Ok(()); }
                }
                let updated = format!("{}{}{}", &current[..start.offset], text, &current[end.offset..]);
                if updated.len() > MAX_TEXT_BYTES { return Err(EditError::TextTooLong); }
                if updated == current { return Ok(()); }
                note.title = updated;
                ScriptNoteTextPosition { paragraph: 0, offset: start.offset + text.len() }
            }
            ScriptNoteField::Body => {
                if start == end && text.is_empty() {
                    let paragraph = note.paragraphs.get(start.paragraph).ok_or(EditError::InvalidSelection)?;
                    if operation == ScriptNoteTextEditOperation::DeleteBackward {
                        if start.offset > 0 { start.offset = previous_boundary(&paragraph.text, start.offset); }
                        else if start.paragraph > 0 { start.paragraph -= 1; start.offset = note.paragraphs[start.paragraph].text.len(); }
                        else { return Ok(()); }
                    } else if operation == ScriptNoteTextEditOperation::DeleteForward {
                        if start.offset < paragraph.text.len() { end.offset = next_boundary(&paragraph.text, start.offset); }
                        else if start.paragraph + 1 < note.paragraphs.len() { end.paragraph += 1; end.offset = 0; }
                        else { return Ok(()); }
                    }
                }
                let inserted_pieces = if operation == ScriptNoteTextEditOperation::Enter {
                    vec![String::new()]
                } else {
                    text.split('\n').map(str::to_owned).collect::<Vec<_>>()
                };
                let replacement_position = replace_note_body_range(
                    &mut candidate, &mut note, start, end, &inserted_pieces,
                    if anchor == focus { typing_style } else { RunStyleFlags::default() },
                    if anchor == focus { typing_formatting } else { InlineRunFormattingPatch::default() },
                    anchor == focus,
                )?;
                if operation == ScriptNoteTextEditOperation::Enter {
                    split_note_paragraph(&mut candidate, &mut note, replacement_position)?
                } else { replacement_position }
            }
        };
        if field == ScriptNoteField::Title { note.date_modified = timestamp; }
        else { note.date_modified = timestamp; }
        notes[note_index] = note;
        candidate.replace_script_notes(ScriptNotes::new(notes))?;
        self.publish_script_note(candidate, ScriptNoteSelection::Text {
            note: note_id, field, anchor: final_position, focus: final_position,
        })?;
        if field == ScriptNoteField::Body && operation == ScriptNoteTextEditOperation::Enter && anchor == focus {
            if self.script_note_typing_style != typing_style || self.script_note_typing_formatting != typing_formatting {
                self.script_note_typing_style = typing_style;
                self.script_note_typing_formatting = typing_formatting;
                self.mark_mutated();
            }
        }
        Ok(())
    }

    pub fn navigate_script_note_horizontal(&mut self, direction: ScriptNoteHorizontalDirection, extend: bool) -> Result<(), EditError> {
        let ScriptNoteSelection::Text { note, field, anchor, focus } = self.script_note_selection.clone() else { return Err(EditError::InvalidSelection) };
        let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let mut target = focus;
        if !extend && anchor != focus {
            target = if direction == ScriptNoteHorizontalDirection::Left { start } else { end };
        } else {
            let text = self.script_note_text_at(&note, field, target.paragraph)?;
            match direction {
                ScriptNoteHorizontalDirection::Left if target.offset > 0 => target.offset = previous_boundary(text, target.offset),
                ScriptNoteHorizontalDirection::Right if target.offset < text.len() => target.offset = next_boundary(text, target.offset),
                ScriptNoteHorizontalDirection::Left if field == ScriptNoteField::Body && target.paragraph > 0 => {
                    target.paragraph -= 1;
                    target.offset = self.script_note_text_at(&note, field, target.paragraph)?.len();
                }
                ScriptNoteHorizontalDirection::Right if field == ScriptNoteField::Body => {
                    let count = self.project.script_notes().get(&note).ok_or(EditError::InvalidSelection)?.paragraphs.len();
                    if target.paragraph + 1 < count { target.paragraph += 1; target.offset = 0; }
                }
                _ => {}
            }
        }
        self.set_script_note_selection(ScriptNoteSelection::Text { note, field, anchor: if extend { anchor } else { target }, focus: target })
    }

    pub fn navigate_script_note_line_edge(&mut self, edge: ScriptNoteLineEdge, extend: bool) -> Result<(), EditError> {
        let ScriptNoteSelection::Text { note, field, anchor, focus } = self.script_note_selection.clone() else { return Err(EditError::InvalidSelection) };
        let text = self.script_note_text_at(&note, field, focus.paragraph)?;
        let target = ScriptNoteTextPosition { paragraph: focus.paragraph, offset: if edge == ScriptNoteLineEdge::Start { 0 } else { text.len() } };
        self.set_script_note_selection(ScriptNoteSelection::Text { note, field, anchor: if extend { anchor } else { target }, focus: target })
    }

    fn script_note_text_at(&self, note: &StableId, field: ScriptNoteField, paragraph: usize) -> Result<&str, EditError> {
        let note = self.project.script_notes().get(note).ok_or(EditError::InvalidSelection)?;
        match field {
            ScriptNoteField::Title if paragraph == 0 => Ok(&note.title),
            ScriptNoteField::Body => note.paragraphs.get(paragraph).map(|item| item.text.as_str()).ok_or(EditError::InvalidSelection),
            _ => Err(EditError::InvalidSelection),
        }
    }
}

fn replace_note_body_range(
    project: &mut Project,
    note: &mut ScriptNote,
    start: ScriptNoteTextPosition,
    end: ScriptNoteTextPosition,
    chunks: &[String],
    typing: RunStyleFlags,
    patch: InlineRunFormattingPatch,
    apply_typing: bool,
) -> Result<ScriptNoteTextPosition, EditError> {
    if start.paragraph > end.paragraph || end.paragraph >= note.paragraphs.len() { return Err(EditError::InvalidSelection); }
    let left = note.paragraphs[start.paragraph].clone();
    let right = note.paragraphs[end.paragraph].clone();
    if start.offset > left.text.len() || !left.text.is_char_boundary(start.offset) || end.offset > right.text.len() || !right.text.is_char_boundary(end.offset) {
        return Err(EditError::InvalidUtf8Boundary);
    }
    let left_runs = effective_runs(&left.text, &left.runs);
    let right_runs = effective_runs(&right.text, &right.runs);
    let insertion_attributes = styled_note_attributes(
        attributes_at(&left_runs, start.offset, left.text.len()), typing, patch, apply_typing,
    )?;
    let prefix = left.text[..start.offset].to_owned();
    let mut allocated = Vec::new();
    let mut durable = generated_identity_allocator(project);
    let mut stable = project.stable_id_allocator()?;
    let empty_chunks = vec![String::new()];
    let chunks = if chunks.is_empty() { empty_chunks.as_slice() } else { chunks };
    let mut replacements = Vec::with_capacity(chunks.len());
    let composed = compose_replacement_chunks(
        &left.text, &left_runs, start.offset, &right.text, &right_runs, end.offset, chunks, &insertion_attributes,
    );
    for index in 0..chunks.len() {
        let mut paragraph = if index == 0 {
            left.clone()
        } else {
            let stable_id = stable.allocate("script-note-paragraph")?;
            let durable_id = durable.allocate().map_err(|_| EditError::InvalidElement)?;
            allocated.push(durable_id.clone());
            ScriptNoteParagraph::empty(stable_id, Some(durable_id), None, generated_note_settings())
        };
        let (text, runs) = composed[index].clone();
        set_normalized_text(&mut paragraph.text, &mut paragraph.runs, text, runs)?;
        replacements.push(paragraph);
    }
    let removed = note.paragraphs[start.paragraph..=end.paragraph].iter()
        .filter(|old| !replacements.iter().any(|new| new.id == old.id)).cloned().collect::<Vec<_>>();
    for old in &removed { unregister_paragraph_identity(project, old.durable_id.as_deref())?; }
    note.paragraphs.splice(start.paragraph..=end.paragraph, replacements);
    for id in allocated { register_new_note_paragraph(project, &id)?; }
    let offset = if chunks.len() == 1 { prefix.len() + chunks[0].len() } else { chunks.last().map_or(0, String::len) };
    Ok(ScriptNoteTextPosition { paragraph: start.paragraph + chunks.len() - 1, offset })
}

fn styled_note_attributes(
    mut attributes: Vec<RunAttribute>,
    style: RunStyleFlags,
    patch: InlineRunFormattingPatch,
    apply_typing: bool,
) -> Result<Vec<RunAttribute>, EditError> {
    if apply_typing {
        let parsed = parse_style_attribute(&attributes).map_err(EditError::FormattedText)?;
        let supported = [
            (RunStyle::Bold, "Bold"), (RunStyle::Italics, "Italic"),
            (RunStyle::Underline, "Underline"), (RunStyle::Strikethrough, "Strikeout"),
            (RunStyle::AllCaps, "AllCaps"),
        ];
        let mut tokens = supported.iter().filter_map(|(flag, token)| style.contains(*flag).then_some(*token))
            .map(str::to_owned).collect::<Vec<_>>();
        tokens.extend(parsed.unknown_tokens);
        set_note_attribute(&mut attributes, "Style", (!tokens.is_empty()).then(|| tokens.join("+")));
    }
    if let Some(value) = patch.font { set_note_attribute(&mut attributes, "Font", value.map(|font| font.fdx_value().to_owned())); }
    if let Some(value) = patch.size { set_note_attribute(&mut attributes, "Size", value.map(|size| size.points().to_string())); }
    if let Some(value) = patch.color { set_note_attribute(&mut attributes, "Color", value.map(|color| color.fdx_value().to_owned())); }
    if let Some(value) = patch.background { set_note_attribute(&mut attributes, "Background", value.map(|color| color.fdx_value().to_owned())); }
    Ok(attributes)
}

fn set_note_attribute(attributes: &mut Vec<RunAttribute>, name: &str, value: Option<String>) {
    if let Some(index) = attributes.iter().position(|attribute| attribute.name == name) {
        if let Some(value) = value { attributes[index].value = value; } else { attributes.remove(index); }
    } else if let Some(value) = value {
        attributes.push(RunAttribute { name: name.to_owned(), value });
    }
}

fn split_note_paragraph(project: &mut Project, note: &mut ScriptNote, position: ScriptNoteTextPosition) -> Result<ScriptNoteTextPosition, EditError> {
    let source = note.paragraphs.get(position.paragraph).ok_or(EditError::InvalidSelection)?.clone();
    if position.offset > source.text.len() || !source.text.is_char_boundary(position.offset) { return Err(EditError::InvalidUtf8Boundary); }
    let runs = effective_runs(&source.text, &source.runs);
    let mut durable = generated_identity_allocator(project);
    let durable_id = durable.allocate().map_err(|_| EditError::InvalidElement)?;
    let stable_id = project.stable_id_allocator()?.allocate("script-note-paragraph")?;
    let mut left = source.clone();
    let mut right = ScriptNoteParagraph::empty(stable_id, Some(durable_id.clone()), None, generated_note_settings());
    set_normalized_text(&mut left.text, &mut left.runs, source.text[..position.offset].to_owned(), slice_runs(&runs, 0, position.offset))?;
    set_normalized_text(&mut right.text, &mut right.runs, source.text[position.offset..].to_owned(), slice_runs(&runs, position.offset, source.text.len()))?;
    note.paragraphs[position.paragraph] = left;
    note.paragraphs.insert(position.paragraph + 1, right);
    register_new_note_paragraph(project, &durable_id)?;
    Ok(ScriptNoteTextPosition { paragraph: position.paragraph + 1, offset: 0 })
}

fn unregister_paragraph_identity(project: &mut Project, durable_id: Option<&str>) -> Result<(), EditError> {
    let Some(durable_id) = durable_id.filter(|id| !id.is_empty()) else { return Ok(()) };
    let Some(state) = project.preservation.as_mut() else { return Ok(()) };
    let identity = ForeignIdentity::new(ForeignIdDomain::Paragraph, durable_id.to_owned()).map_err(EditError::Preservation)?;
    if state.anchors.iter().any(|anchor| matches!(&anchor.target, AnchorTarget::Foreign(target) if target == &identity))
        || state.references.iter().any(|reference| reference.source != identity && reference.target == identity && reference.policy == ReferencePolicy::RequiredInternal)
        || state.board_graph.references.iter().any(|reference| reference.source != identity && reference.target == identity && reference.policy == ReferencePolicy::RequiredInternal)
        || state.board_graph.entities.iter().any(|entity| entity.identity == identity)
    {
        return Err(EditError::Preservation(scriptcraft_model::preservation::PreservationError::DanglingReference));
    }
    state.identities.retain(|item| item != &identity);
    state.references.retain(|reference| reference.source != identity);
    state.board_graph.references.retain(|reference| reference.source != identity);
    state.rebuild_indexes();
    Ok(())
}
