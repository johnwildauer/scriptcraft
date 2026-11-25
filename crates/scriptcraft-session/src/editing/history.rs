//! Snapshot ownership, bounded history, and undo/redo restoration.

use super::derived::PendingDerivedInvalidation;
use super::selection::{validate_selection, validate_title_selection};
use super::{validate_full_candidate, EditError, EditingSession, Selection};
use scriptcraft_model::{formatted_text::{InlineRunFormattingPatch, RunStyleFlags}, Project};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    // Project includes semantic runs and the complete preservation graph, so
    // undo/redo restores both authorities rather than only the text projection.
    pub(super) project: Project,
    pub(super) selection: Selection,
    pub(super) title_selection: Selection,
    pub(super) beat_selection: super::BeatSelection,
    pub(super) beat_typing_style: RunStyleFlags,
    pub(super) beat_typing_formatting: InlineRunFormattingPatch,
    pub(super) title_typing_style: RunStyleFlags,
    pub(super) body_typing_style: RunStyleFlags,
    pub(super) body_typing_formatting: InlineRunFormattingPatch,
    pub(super) generated_parentheticals: BTreeSet<String>,
}

impl EditingSession {
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> Result<(), EditError> {
        let mut snapshot = self
            .history
            .last()
            .cloned()
            .ok_or(EditError::EmptyHistory)?;
        validate_full_candidate(&snapshot.project)?;
        if let Some(preservation) = snapshot.project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        validate_selection(&snapshot.project, snapshot.selection)?;
        validate_title_selection(&snapshot.project, snapshot.title_selection)?;
        super::beat_board::validate_beat_selection(&snapshot.project, &snapshot.beat_selection)?;
        let snapshot_note_selection = self.history_note_selections.last().cloned().unwrap_or_default();
        super::script_notes::validate_script_note_selection(&snapshot.project, &snapshot_note_selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            beat_selection: self.beat_selection.clone(),
            beat_typing_style: self.beat_typing_style,
            beat_typing_formatting: self.beat_typing_formatting,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            body_typing_formatting: self.body_typing_formatting,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.history.pop();
        self.history_note_selections.pop();
        self.redo.push(current);
        self.redo_note_selections.push(self.script_note_selection.clone());
        self.project = snapshot.project;
        self.refresh_layout_inputs_from_project();
        self.selection = snapshot.selection;
        self.title_selection = snapshot.title_selection;
        self.beat_selection = snapshot.beat_selection;
        self.script_note_selection = snapshot_note_selection;
        self.beat_typing_style = snapshot.beat_typing_style;
        self.beat_typing_formatting = snapshot.beat_typing_formatting;
        self.title_typing_style = snapshot.title_typing_style;
        self.body_typing_style = snapshot.body_typing_style;
        self.body_typing_formatting = snapshot.body_typing_formatting;
        self.generated_parentheticals = snapshot.generated_parentheticals;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.pending_derived_invalidation = PendingDerivedInvalidation::Complete;
        self.deferred_derived_scenes.clear();
        self.failed_derived_complete = false;
        self.failed_derived_scenes.clear();
        self.mark_mutated();
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), EditError> {
        let mut snapshot = self.redo.last().cloned().ok_or(EditError::EmptyHistory)?;
        validate_full_candidate(&snapshot.project)?;
        if let Some(preservation) = snapshot.project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        validate_selection(&snapshot.project, snapshot.selection)?;
        validate_title_selection(&snapshot.project, snapshot.title_selection)?;
        super::beat_board::validate_beat_selection(&snapshot.project, &snapshot.beat_selection)?;
        let snapshot_note_selection = self.redo_note_selections.last().cloned().unwrap_or_default();
        super::script_notes::validate_script_note_selection(&snapshot.project, &snapshot_note_selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            beat_selection: self.beat_selection.clone(),
            beat_typing_style: self.beat_typing_style,
            beat_typing_formatting: self.beat_typing_formatting,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            body_typing_formatting: self.body_typing_formatting,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.redo.pop();
        self.redo_note_selections.pop();
        let remaining_redo_note_selections = self.redo_note_selections.clone();
        self.push_history(current);
        self.redo_note_selections = remaining_redo_note_selections;
        self.project = snapshot.project;
        self.refresh_layout_inputs_from_project();
        self.selection = snapshot.selection;
        self.title_selection = snapshot.title_selection;
        self.beat_selection = snapshot.beat_selection;
        self.script_note_selection = snapshot_note_selection;
        self.beat_typing_style = snapshot.beat_typing_style;
        self.beat_typing_formatting = snapshot.beat_typing_formatting;
        self.title_typing_style = snapshot.title_typing_style;
        self.body_typing_style = snapshot.body_typing_style;
        self.body_typing_formatting = snapshot.body_typing_formatting;
        self.generated_parentheticals = snapshot.generated_parentheticals;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.pending_derived_invalidation = PendingDerivedInvalidation::Complete;
        self.deferred_derived_scenes.clear();
        self.failed_derived_complete = false;
        self.failed_derived_scenes.clear();
        self.mark_mutated();
        Ok(())
    }

    pub(super) fn push_history(&mut self, snapshot: Snapshot) {
        if self.history_limit == 0 {
            self.redo_note_selections.clear();
            return;
        }
        self.history.push(snapshot);
        self.history_note_selections.push(self.script_note_selection.clone());
        // All non-redo publishers clear the matching Project redo stack after
        // publishing; clear its parallel selection payload at the same boundary.
        self.redo_note_selections.clear();
        if self.history.len() > self.history_limit {
            self.history.remove(0);
            self.history_note_selections.remove(0);
        }
    }
}
