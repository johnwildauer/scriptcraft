//! Snapshot ownership, bounded history, and undo/redo restoration.

use super::selection::{validate_selection, validate_title_selection};
use super::derived::PendingDerivedInvalidation;
use super::{validate_full_candidate, EditError, EditingSession, Selection};
use scriptcraft_model::{cast_list, formatted_text::RunStyleFlags, Project};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    // Project includes semantic runs and the complete preservation graph, so
    // undo/redo restores both authorities rather than only the text projection.
    pub(super) project: Project,
    pub(super) selection: Selection,
    pub(super) title_selection: Selection,
    pub(super) title_typing_style: RunStyleFlags,
    pub(super) body_typing_style: RunStyleFlags,
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
        // Undo must remain available even when a derived list's source mapping
        // cannot be changed safely; retain that snapshot unchanged and let the
        // scheduled refresh/save preflight report the mapping limitation.
        let _ = cast_list::correct_cast_lists(&mut snapshot.project);
        validate_full_candidate(&snapshot.project)?;
        if let Some(preservation) = snapshot.project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        validate_selection(&snapshot.project, snapshot.selection)?;
        validate_title_selection(&snapshot.project, snapshot.title_selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.history.pop();
        self.redo.push(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.title_selection = snapshot.title_selection;
        self.title_typing_style = snapshot.title_typing_style;
        self.body_typing_style = snapshot.body_typing_style;
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
        let _ = cast_list::correct_cast_lists(&mut snapshot.project);
        validate_full_candidate(&snapshot.project)?;
        if let Some(preservation) = snapshot.project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        validate_selection(&snapshot.project, snapshot.selection)?;
        validate_title_selection(&snapshot.project, snapshot.title_selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.redo.pop();
        self.push_history(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.title_selection = snapshot.title_selection;
        self.title_typing_style = snapshot.title_typing_style;
        self.body_typing_style = snapshot.body_typing_style;
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
            return;
        }
        self.history.push(snapshot);
        if self.history.len() > self.history_limit {
            self.history.remove(0);
        }
    }
}
