//! Detached Body candidate validation and atomic publication.

use super::derived::{
    can_skip_derived_invalidation, classify_derived_invalidation, PendingDerivedInvalidation,
};
use super::diagnostics::{add_diagnostic_duration, add_optional_diagnostic_duration};
use super::history::Snapshot;
use super::selection::SelectionOrdering;
use super::structure::{
    groups_touched_by_selection, remap_selection_by_identity, unwrap_dual_dialogue_groups,
};
use super::{
    invalidate_changed_generated_parentheticals, validate_full_candidate, validate_selection,
    EditError, EditMutationDiagnostics, EditPolicy, EditingSession, Selection,
};
use scriptcraft_model::{dual_dialogue, BodyOwner, Project, StableId};
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MutationValidationScope {
    Complete,
    SingleParagraph,
}

impl EditingSession {
    pub(super) fn mutate<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        operation: F,
    ) -> Result<(), EditError>
    where
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.mutate_with_scene_heading_deletions(requested_scope, BTreeSet::new(), false, operation)
    }

    pub(super) fn mutate_structural<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        operation: F,
    ) -> Result<(), EditError>
    where
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.mutate_with_scene_heading_deletions(requested_scope, BTreeSet::new(), true, operation)
    }

    pub(super) fn mutate_structural_with_scene_heading_deletions<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        generated_scene_heading_deletions: BTreeSet<StableId>,
        operation: F,
    ) -> Result<(), EditError>
    where
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.mutate_with_scene_heading_deletions(
            requested_scope,
            generated_scene_heading_deletions,
            true,
            operation,
        )
    }

    pub(super) fn mutate_with_scene_heading_deletions<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        generated_scene_heading_deletions: BTreeSet<StableId>,
        structural: bool,
        mut operation: F,
    ) -> Result<(), EditError>
    where
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        let capture_diagnostics = self.capture_mutation_diagnostics;
        let total_started = capture_diagnostics.then(Instant::now);
        let mut diagnostics = EditMutationDiagnostics::default();
        let result = (|| {
            let setup_started = capture_diagnostics.then(Instant::now);
            let (scope_start, scope_end) = self.selection.ordered();
            let requested_element = (requested_scope == MutationValidationScope::SingleParagraph
                && scope_start.element == scope_end.element)
                .then(|| {
                    self.project
                        .elements
                        .get(scope_start.element)
                        .map(|element| element.id.clone())
                })
                .flatten();
            let baseline = &self.project;
            let baseline_generated = self.generated_parentheticals.clone();
            let mut first_project = baseline.clone();
            let mut first_generated = baseline_generated.clone();
            add_diagnostic_duration(&mut diagnostics.candidate_setup_nanos, setup_started);

            let edit_started = capture_diagnostics.then(Instant::now);
            let first_result = operation(
                &mut first_project,
                &self.selection,
                &mut first_generated,
                &self.policy,
            );
            add_diagnostic_duration(&mut diagnostics.edit_operation_nanos, edit_started);
            let (mut project, mut generated, selection) = match first_result {
                Ok(selection) => {
                    dual_dialogue::prune_removed_group_paragraphs(&baseline, &mut first_project);
                    let plan = if structural {
                        dual_dialogue::classify_structural_result(
                            &baseline,
                            self.selection,
                            &first_project,
                        )
                    } else {
                        Vec::new()
                    };
                    let unwrap_ids = plan
                        .into_iter()
                        .filter_map(|classification| {
                            matches!(
                                classification.outcome,
                                dual_dialogue::StructuralGroupOutcome::Removed
                                    | dual_dialogue::StructuralGroupOutcome::InvalidRequiresUnwrap
                            )
                            .then_some(classification.group_id)
                        })
                        .collect::<Vec<_>>();
                    if unwrap_ids.is_empty() {
                        (first_project, first_generated, selection)
                    } else {
                        diagnostics.structural_retries =
                            diagnostics.structural_retries.saturating_add(1);
                        let retry_started = capture_diagnostics.then(Instant::now);
                        let retry_result =
                            (|| -> Result<(Project, BTreeSet<String>, Selection), EditError> {
                                let setup_started = capture_diagnostics.then(Instant::now);
                                let mut retry_project = baseline.clone();
                                unwrap_dual_dialogue_groups(&mut retry_project, &unwrap_ids)?;
                                let retry_selection = remap_selection_by_identity(
                                    &baseline,
                                    &retry_project,
                                    self.selection,
                                )?;
                                let mut retry_generated = baseline_generated.clone();
                                add_diagnostic_duration(
                                    &mut diagnostics.candidate_setup_nanos,
                                    setup_started,
                                );
                                let edit_started = capture_diagnostics.then(Instant::now);
                                let selection_result = operation(
                                    &mut retry_project,
                                    &retry_selection,
                                    &mut retry_generated,
                                    &self.policy,
                                );
                                add_diagnostic_duration(
                                    &mut diagnostics.edit_operation_nanos,
                                    edit_started,
                                );
                                let selection = selection_result?;
                                Ok((retry_project, retry_generated, selection))
                            })();
                        add_optional_diagnostic_duration(
                            &mut diagnostics.structural_retry_nanos,
                            retry_started,
                        );
                        let (retry_project, retry_generated, selection) = retry_result?;
                        (retry_project, retry_generated, selection)
                    }
                }
                Err(error @ EditError::InlineAltsRoleChange) => return Err(error),
                Err(first_error) if structural => {
                    let unwrap_ids = groups_touched_by_selection(&baseline, self.selection);
                    if unwrap_ids.is_empty() {
                        return Err(first_error);
                    }
                    diagnostics.structural_retries =
                        diagnostics.structural_retries.saturating_add(1);
                    let retry_started = capture_diagnostics.then(Instant::now);
                    let retry_result =
                        (|| -> Result<(Project, BTreeSet<String>, Selection), EditError> {
                            let setup_started = capture_diagnostics.then(Instant::now);
                            let mut retry_project = baseline.clone();
                            unwrap_dual_dialogue_groups(&mut retry_project, &unwrap_ids)?;
                            let retry_selection = remap_selection_by_identity(
                                &baseline,
                                &retry_project,
                                self.selection,
                            )?;
                            let mut retry_generated = baseline_generated.clone();
                            add_diagnostic_duration(
                                &mut diagnostics.candidate_setup_nanos,
                                setup_started,
                            );
                            let edit_started = capture_diagnostics.then(Instant::now);
                            let selection_result = operation(
                                &mut retry_project,
                                &retry_selection,
                                &mut retry_generated,
                                &self.policy,
                            );
                            add_diagnostic_duration(
                                &mut diagnostics.edit_operation_nanos,
                                edit_started,
                            );
                            let selection = selection_result?;
                            Ok((retry_project, retry_generated, selection))
                        })();
                    add_optional_diagnostic_duration(
                        &mut diagnostics.structural_retry_nanos,
                        retry_started,
                    );
                    let (retry_project, retry_generated, selection) = retry_result?;
                    (retry_project, retry_generated, selection)
                }
                Err(error) => return Err(error),
            };
            let prior_generated = baseline_generated;
            dual_dialogue::prune_removed_group_paragraphs(&self.project, &mut project);
            let preservation_started = capture_diagnostics.then(Instant::now);
            if let Some(preservation) = project.preservation.as_mut() {
                preservation.ensure_managed_paragraph_index();
            }
            add_diagnostic_duration(&mut diagnostics.preservation_nanos, preservation_started);
            invalidate_changed_generated_parentheticals(
                &self.project,
                &project,
                &prior_generated,
                &mut generated,
            );
            generated.retain(|id| {
                project.elements.iter().any(|element| {
                    element.id.as_str() == id
                        && project
                            .element_semantics(&element.element_type())
                            .is_some_and(|semantics| semantics.capabilities().parenthetical)
                })
            });
            let scoped_preconditions = !structural
                && self.validated_baseline
                && requested_element.as_ref().is_some_and(|id| {
                    project.elements.len() == self.project.elements.len()
                        && project
                            .elements
                            .get(scope_start.element)
                            .is_some_and(|element| &element.id == id)
                        && project.direct_paragraph_overrides
                            == self.project.direct_paragraph_overrides
                        && selection.anchor.element == scope_start.element
                        && selection.focus.element == scope_start.element
                        // Scoped validation currently assumes that a single
                        // paragraph retains its exact type. Type changes must
                        // validate complete group invariants so an invalidated
                        // dual container cannot slip through while WO-003.7 is
                        // still pending.
                        && self
                            .project
                            .elements
                            .get(scope_start.element)
                            .zip(project.elements.get(scope_start.element))
                            .is_some_and(|(before, after)| {
                                before.element_type() == after.element_type()
                            })
                        && project
                            .preservation
                            .as_ref()
                            .is_none_or(|state| state.has_managed_paragraph_index())
                });
            let scoped_valid = if scoped_preconditions {
                diagnostics.scoped_candidate_attempts =
                    diagnostics.scoped_candidate_attempts.saturating_add(1);
                let scoped_started = capture_diagnostics.then(Instant::now);
                let scoped_result = (|| match project
                    .validate_scoped_single_element_edit(&self.project, scope_start.element)
                {
                    Ok(true) if validate_selection(&project, selection).is_ok() => {
                        diagnostics.scoped_candidate_accepts =
                            diagnostics.scoped_candidate_accepts.saturating_add(1);
                        Ok(true)
                    }
                    Ok(_) => Ok(false),
                    Err(error) => Err(EditError::Model(error)),
                })();
                add_optional_diagnostic_duration(
                    &mut diagnostics.scoped_validation_nanos,
                    scoped_started,
                );
                scoped_result?
            } else {
                false
            };
            if !scoped_valid {
                if requested_scope == MutationValidationScope::SingleParagraph {
                    diagnostics.full_validation_fallbacks =
                        diagnostics.full_validation_fallbacks.saturating_add(1);
                }
                let preservation_started = capture_diagnostics.then(Instant::now);
                let preservation_result = (|| -> Result<(), EditError> {
                    if let Some(preservation) = project.preservation.as_mut() {
                        preservation
                            .reconcile_body_structure(
                                &self.project,
                                &self.project.elements,
                                &mut project.elements,
                                &generated_scene_heading_deletions,
                                &project.dual_dialogue_groups,
                            )
                            .map_err(EditError::Preservation)?;
                    }
                    normalize_reconciled_body_run_candidates(&self.project, &mut project)?;
                    Ok(())
                })();
                add_diagnostic_duration(&mut diagnostics.preservation_nanos, preservation_started);
                preservation_result?;

                diagnostics.full_candidate_validations =
                    diagnostics.full_candidate_validations.saturating_add(1);
                let full_started = capture_diagnostics.then(Instant::now);
                let full_result = validate_full_candidate(&project);
                add_optional_diagnostic_duration(
                    &mut diagnostics.full_validation_nanos,
                    full_started,
                );
                full_result?;

                let preservation_started = capture_diagnostics.then(Instant::now);
                if let Some(preservation) = project.preservation.as_mut() {
                    preservation.ensure_source_identity_use_index();
                }
                add_diagnostic_duration(&mut diagnostics.preservation_nanos, preservation_started);
                let selection_started = capture_diagnostics.then(Instant::now);
                let selection_result = validate_selection(&project, selection);
                add_optional_diagnostic_duration(
                    &mut diagnostics.full_validation_nanos,
                    selection_started,
                );
                selection_result?;
            }
            let no_op_started = capture_diagnostics.then(Instant::now);
            let no_op = project == self.project && selection == self.selection;
            add_optional_diagnostic_duration(&mut diagnostics.no_op_equality_nanos, no_op_started);
            if no_op {
                diagnostics.no_op = true;
                return Ok(());
            }
            let derived_started = capture_diagnostics.then(Instant::now);
            let derived_invalidation = if can_skip_derived_invalidation(
                &self.project,
                &project,
                scope_start.element,
                scoped_valid,
            ) {
                PendingDerivedInvalidation::None
            } else {
                classify_derived_invalidation(&self.project, &project)
            };
            add_optional_diagnostic_duration(
                &mut diagnostics.derived_classification_nanos,
                derived_started,
            );
            match &derived_invalidation {
                PendingDerivedInvalidation::Scenes(scenes) => {
                    self.deferred_derived_scenes
                        .retain(|scene| !scenes.contains(scene));
                    self.failed_derived_scenes
                        .retain(|scene| !scenes.contains(scene));
                    // A new relevant scoped invalidation permits the previously
                    // failed complete candidate to be attempted again.
                    self.failed_derived_complete = false;
                }
                PendingDerivedInvalidation::Complete => {
                    self.deferred_derived_scenes.clear();
                    self.failed_derived_complete = false;
                    self.failed_derived_scenes.clear();
                }
                PendingDerivedInvalidation::None => {}
            }
            let publication_started = capture_diagnostics.then(Instant::now);
            let replaced_project = std::mem::replace(&mut self.project, project);
            self.push_history(Snapshot {
                project: replaced_project,
                selection: self.selection,
                title_selection: self.title_selection,
                beat_selection: self.beat_selection.clone(),
                beat_typing_style: self.beat_typing_style,
                beat_typing_formatting: self.beat_typing_formatting,
                script_note_typing_style: self.script_note_typing_style,
                script_note_typing_formatting: self.script_note_typing_formatting,
                title_typing_style: self.title_typing_style,
                body_typing_style: self.body_typing_style,
                body_typing_formatting: self.body_typing_formatting,
                generated_parentheticals: self.generated_parentheticals.clone(),
            });
            self.selection = selection;
            self.generated_parentheticals = generated;
            self.redo.clear();
            self.revision = self.revision.saturating_add(1);
            add_optional_diagnostic_duration(
                &mut diagnostics.history_publication_nanos,
                publication_started,
            );
            let dirty_started = capture_diagnostics.then(Instant::now);
            self.dirty = self.project != self.clean_project;
            add_optional_diagnostic_duration(
                &mut diagnostics.dirty_comparison_nanos,
                dirty_started,
            );
            let publication_started = capture_diagnostics.then(Instant::now);
            self.validated_baseline = true;
            self.pending_derived_invalidation
                .merge(derived_invalidation);
            self.mark_mutated();
            add_optional_diagnostic_duration(
                &mut diagnostics.history_publication_nanos,
                publication_started,
            );
            Ok(())
        })();
        if capture_diagnostics {
            add_diagnostic_duration(&mut diagnostics.total_nanos, total_started);
            self.last_mutation_diagnostics = Some(diagnostics);
        }
        result
    }
}

fn normalize_reconciled_body_run_candidates(
    baseline: &Project,
    candidate: &mut Project,
) -> Result<(), EditError> {
    for index in 0..candidate.elements.len() {
        let id = candidate.elements[index].id.clone();
        let text = candidate.elements[index].text.clone();
        let runs = candidate.elements[index].runs.clone();
        let baseline_element = baseline
            .elements
            .iter()
            .find(|element| element.id == id);
        let baseline_alt_state =
            baseline_element.and_then(|element| element.dialogue_alternatives.clone());
        let candidate_alt_state = candidate.elements[index].dialogue_alternatives.clone();
        let alt_state_changed = baseline_element.is_some_and(|element| {
            element.dialogue_alternatives != candidate_alt_state
        });
        let unchanged = baseline_element
            .is_some_and(|element| element.text == text && element.runs == runs)
            && !alt_state_changed;
        if unchanged {
            continue;
        }
        let normalized = scriptcraft_model::formatted_text::normalize_runs(&text, &runs)
            .map_err(EditError::FormattedText)?;
        let has_provenance = candidate.elements[index].source_provenance.is_some();
        BodyOwner::update(&mut candidate.elements[index], |element| {
            element.runs = normalized.clone();
        });
        if let Some(preservation) = candidate.preservation.as_mut() {
            if preservation.has_managed_element(&id) {
                if alt_state_changed {
                    preservation
                        .replace_managed_projection(&candidate.elements[index])
                        .map_err(EditError::Preservation)?;
                    preservation
                        .reconcile_managed_dialogue_alts(
                            &id,
                            baseline_alt_state.as_ref(),
                            candidate_alt_state.as_ref(),
                        )
                        .map_err(EditError::Preservation)?;
                } else {
                    preservation
                        .sync_managed_text_runs(&id, &text, &normalized)
                        .map_err(EditError::Preservation)?;
                }
            } else if has_provenance {
                return Err(EditError::Preservation(
                    scriptcraft_model::preservation::PreservationError::ManagedAnchorMissing,
                ));
            }
        }
    }
    Ok(())
}
