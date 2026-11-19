//! Validated-session target reads and transactional dual-dialogue toggling.

use super::history::Snapshot;
use super::{validate_full_candidate, validate_selection, EditError, EditingSession};
use scriptcraft_model::dual_dialogue;
use scriptcraft_model::{Project, SourceFormat, SourceProvenance, SourceSection};

impl EditingSession {
    /// Read-only resolution for the current selection on the requested surface.
    /// TitlePage is always ineligible for the Body-only dual-dialogue toggle.
    pub fn dual_dialogue_toggle_target(
        &self,
        surface: dual_dialogue::SelectionSurface,
    ) -> Result<Option<dual_dialogue::ToggleTarget>, EditError> {
        let selection = match surface {
            dual_dialogue::SelectionSurface::Body => self.selection,
            dual_dialogue::SelectionSurface::TitlePage => self.title_selection,
        };
        dual_dialogue::resolve_toggle_target_for_validated_session(
            &self.project,
            surface,
            selection,
        )
        .map_err(EditError::Model)
    }

    /// Atomically enables a valid ungrouped dual sequence or disables the group
    /// containing the collapsed Body caret. An ineligible selection is a no-op.
    pub fn toggle_dual_dialogue(&mut self) -> Result<(), EditError> {
        let Some(target) =
            self.dual_dialogue_toggle_target(dual_dialogue::SelectionSurface::Body)?
        else {
            return Ok(());
        };
        let mut project = self.project.clone();
        match target {
            dual_dialogue::ToggleTarget::Enable(pattern) => {
                let candidate_start = pattern.flat_range.start;
                let group_id = project.allocate_element_id()?;
                let member_ids = pattern
                    .left
                    .member_ids()
                    .chain(pattern.right.member_ids())
                    .cloned()
                    .collect::<Vec<_>>();
                let mut wrapper = dual_dialogue::DualDialogueWrapperProvenance::default();
                if let Some(preservation) = project.preservation.as_mut() {
                    let (foreign_id, anchor, ordinal) = preservation
                        .wrap_dual_dialogue_members(&group_id, &member_ids, None)
                        .map_err(EditError::Preservation)?;
                    wrapper.outer_foreign_id = Some(foreign_id.clone());
                    wrapper.source = Some(SourceProvenance {
                        format: SourceFormat::Fdx,
                        section: SourceSection::Content,
                        ordinal,
                        foreign_id: Some(foreign_id),
                        foreign_role: Some("General".to_owned()),
                    });
                    wrapper.preservation_anchor = Some(anchor);
                }
                let group = dual_dialogue::DualDialogueGroup {
                    id: group_id,
                    members: member_ids,
                    wrapper,
                };
                let insertion = project
                    .dual_dialogue_groups
                    .iter()
                    .position(|existing| {
                        dual_dialogue::first_member(&project, existing)
                            .is_some_and(|index| index > candidate_start)
                    })
                    .unwrap_or(project.dual_dialogue_groups.len());
                project.dual_dialogue_groups.insert(insertion, group);
            }
            dual_dialogue::ToggleTarget::Disable(group_id) => {
                super::structure::unwrap_dual_dialogue_groups(
                    &mut project,
                    std::slice::from_ref(&group_id),
                )?;
            }
        }

        validate_full_candidate(&project)?;
        if let Some(preservation) = project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        validate_selection(&project, self.selection)?;
        let snapshot = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.push_history(snapshot);
        self.project = project;
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.pending_derived_invalidation = super::derived::PendingDerivedInvalidation::Complete;
        self.deferred_derived_scenes.clear();
        self.failed_derived_complete = false;
        self.failed_derived_scenes.clear();
        self.mark_mutated();
        Ok(())
    }
}
