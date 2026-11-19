//! Transactional native save preparation, verification, publication, and rebase.

use super::candidate::PreparedFdx;
use super::error::{FdxStorageErrorKind, TransactionalSaveError, TransactionalSaveOutcome};
use super::filesystem::FdxStorageFileSystem;
use super::publication::{replace, save_as, save_as_overwrite};
use super::target::{FdxSaveMode, FdxSaveTarget};
use crate::editing::{
    map_selection_to_project, map_title_selection_to_project, EditError, EditingSession,
};
use scriptcraft_fdx::{self as fdx, same_dual_dialogue_projection, write_v6_for_save, FdxError};
use scriptcraft_model::{
    cast_list, BodyOwner, ElementTypeKey, Project, Role, StableId, TitlePageParagraph,
};

/// Save against the session's opened/saved target, or create a new target using
/// the mode's Save As policy. Session and target state change only after publish.
pub fn save_session<F: FdxStorageFileSystem>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let text_state = fdx::TextState {
        scaling: 100,
        selection: session.selection(),
    };
    save_session_with_view_state(
        fs,
        session,
        target,
        mode,
        fdx::WindowState::DEFAULT,
        text_state,
    )
}

/// Save using the latest desktop-reported logical native geometry. Geometry is
/// save-scoped disposable FDX view state and never enters the editing session.
pub fn save_session_with_window_state<F: FdxStorageFileSystem>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    window_state: fdx::WindowState,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let text_state = fdx::TextState {
        scaling: 100,
        selection: session.selection(),
    };
    save_session_with_view_state(fs, session, target, mode, window_state, text_state)
}

/// Save with the editor selection and persisted whole-percent page zoom. Both
/// values are captured in the detached save candidate only.
pub fn save_session_with_view_state<F: FdxStorageFileSystem>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    window_state: fdx::WindowState,
    text_state: fdx::TextState,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    save_session_with_codec_and_view_state(
        fs,
        session,
        target,
        mode,
        &NativeSaveCandidateCodec,
        window_state,
        text_state,
    )
}

trait SaveCandidateCodec {
    fn serialize(
        &self,
        project: &Project,
        context: &mut fdx::SaveContext,
    ) -> Result<PreparedFdx, TransactionalSaveError>;
    fn reread(&self, bytes: &[u8]) -> Result<Project, FdxError>;
}

struct NativeSaveCandidateCodec;

impl SaveCandidateCodec for NativeSaveCandidateCodec {
    fn serialize(
        &self,
        project: &Project,
        context: &mut fdx::SaveContext,
    ) -> Result<PreparedFdx, TransactionalSaveError> {
        let bytes = write_v6_for_save(project, context).map_err(TransactionalSaveError::Codec)?;
        PreparedFdx::from_validated_bytes(bytes).map_err(|error| match &error.kind {
            FdxStorageErrorKind::InvalidFdx(codec_error) => {
                TransactionalSaveError::Codec(codec_error.clone())
            }
            _ => TransactionalSaveError::Storage(error),
        })
    }

    fn reread(&self, bytes: &[u8]) -> Result<Project, FdxError> {
        fdx::read_v6(bytes)
            .map(|document| document.project)
            .map_err(|error| {
                fdx::save_diagnostic_at(error, fdx::SaveDiagnosticCode::StorageRereadPropagation)
            })
    }
}

fn save_session_with_codec_and_view_state<F: FdxStorageFileSystem, C: SaveCandidateCodec>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    codec: &C,
    window_state: fdx::WindowState,
    text_state: fdx::TextState,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let operation = match &mode {
        FdxSaveMode::Save => fdx::SaveOperation::Save,
        FdxSaveMode::SaveAs(_) | FdxSaveMode::SaveAsOverwrite(_) => fdx::SaveOperation::SaveAs,
    };
    let context = fdx::SaveContext::new(operation)
        .with_session_open_timestamp(session.session_open_timestamp().to_owned())
        .with_window_state(window_state)
        .with_text_state(text_state);
    save_session_with_context(fs, session, target, mode, context, codec)
}

fn save_session_with_context<F: FdxStorageFileSystem, C: SaveCandidateCodec>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    mut context: fdx::SaveContext,
    codec: &C,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let (destination, existing_baseline, overwrite_consent) = match mode {
        FdxSaveMode::Save => {
            let path = target
                .path
                .clone()
                .ok_or(TransactionalSaveError::MissingBaseline)?;
            let baseline = target
                .baseline
                .as_ref()
                .ok_or(TransactionalSaveError::MissingBaseline)?
                .clone();
            (path, Some(baseline), false)
        }
        FdxSaveMode::SaveAs(destination) => (destination, None, false),
        FdxSaveMode::SaveAsOverwrite(destination) => (destination, None, true),
    };

    let corrected = session
        .prepare_cast_list_save_candidate()
        .map_err(TransactionalSaveError::Session)?;
    let prepared = codec.serialize(&corrected, &mut context)?;
    let rebased_project = codec
        .reread(&prepared.bytes)
        .map_err(TransactionalSaveError::Codec)?;
    if !same_save_projection(&corrected, &rebased_project) {
        return Err(TransactionalSaveError::Codec(
            fdx::SaveDiagnosticCode::StorageSemanticProjectionMismatch.error(),
        ));
    }
    let publishable_session = session
        .prepare_saved_rebase(rebased_project)
        .map_err(TransactionalSaveError::Session)?;

    // The destination and all session work are now prepared. No fallible
    // application transition follows successful filesystem publication.
    let receipt = match existing_baseline.as_ref() {
        Some(baseline) => replace(fs, &destination, &prepared, baseline),
        None if overwrite_consent => save_as_overwrite(fs, &destination, &prepared),
        None => save_as(fs, &destination, &prepared),
    }
    .map_err(TransactionalSaveError::Storage)?;

    let outcome = TransactionalSaveOutcome {
        temporary_cleanup_warning: receipt.temporary_cleanup_failed,
    };
    target.path = Some(destination);
    target.baseline = Some(receipt.baseline);
    *session = publishable_session;
    Ok(outcome)
}

fn same_save_projection(expected: &Project, actual: &Project) -> bool {
    fn same_elements(
        expected: &[std::sync::Arc<BodyOwner>],
        actual: &[std::sync::Arc<BodyOwner>],
    ) -> bool {
        expected.len() == actual.len()
            && expected.iter().zip(actual).all(|(expected, actual)| {
                expected.element_type() == actual.element_type()
                    && expected.text == actual.text
                    && if expected.runs.is_empty() {
                        actual.runs.iter().all(|run| run.attributes.is_empty())
                    } else {
                        expected.runs == actual.runs
                    }
            })
    }

    expected.element_definitions() == actual.element_definitions()
        && expected.direct_paragraph_overrides == actual.direct_paragraph_overrides
        && same_title_paragraphs(&expected.title_page, &actual.title_page)
        && same_elements(&expected.elements, &actual.elements)
        && same_dual_dialogue_projection(expected, actual)
}

fn same_title_paragraphs(expected: &[TitlePageParagraph], actual: &[TitlePageParagraph]) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            expected.element_type() == actual.element_type()
                && expected.text == actual.text
                && expected.formatting == actual.formatting
                && if expected.runs.is_empty() {
                    actual.runs.iter().all(|run| run.attributes.is_empty())
                } else {
                    expected.runs == actual.runs
                }
        })
}

fn map_body_owner_identity<'a>(
    old_project: &Project,
    old_id: &StableId,
    new_project: &'a Project,
) -> Option<&'a BodyOwner> {
    let old = old_project
        .elements
        .iter()
        .find(|owner| &owner.id == old_id)?;
    if old_project
        .elements
        .iter()
        .filter(|owner| owner.id == *old_id)
        .count()
        != 1
    {
        return None;
    }

    let stable_matches = new_project
        .elements
        .iter()
        .filter(|owner| {
            owner.id == *old_id
                && match (
                    old.source_provenance.as_ref(),
                    owner.source_provenance.as_ref(),
                ) {
                    (Some(old), Some(new)) => {
                        old.format == new.format
                            && old.section == new.section
                            && old.foreign_id == new.foreign_id
                    }
                    (Some(_), None) => false,
                    _ => true,
                }
        })
        .collect::<Vec<_>>();
    if let [owner] = stable_matches.as_slice() {
        return Some(owner);
    }

    if let Some(old_source) = old.source_provenance.as_ref() {
        if let Some(foreign_id) = old_source.foreign_id.as_deref() {
            let old_matches = old_project
                .elements
                .iter()
                .filter(|owner| {
                    owner.source_provenance.as_ref().is_some_and(|source| {
                        source.foreign_id.as_deref() == Some(foreign_id)
                            && source.format == old_source.format
                            && source.section == old_source.section
                    })
                })
                .count();
            let new_matches = new_project
                .elements
                .iter()
                .filter(|owner| {
                    owner.source_provenance.as_ref().is_some_and(|source| {
                        source.foreign_id.as_deref() == Some(foreign_id)
                            && source.format == old_source.format
                            && source.section == old_source.section
                    })
                })
                .collect::<Vec<_>>();
            if old_matches == 1 {
                if let [owner] = new_matches.as_slice() {
                    return Some(owner);
                }
            }
        }
        return None;
    }

    let old_matches = old_project
        .elements
        .iter()
        .filter(|owner| owner.element_type() == old.element_type() && owner.text == old.text)
        .collect::<Vec<_>>();
    let occurrence = old_matches.iter().position(|owner| owner.id == old.id)?;
    let new_matches = new_project
        .elements
        .iter()
        .enumerate()
        .filter(|owner| {
            let owner = owner.1;
            owner.element_type() == old.element_type() && owner.text == old.text
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if old_matches.len() != new_matches.len() {
        return None;
    }
    new_matches
        .get(occurrence)
        .and_then(|index| new_project.elements.get(*index))
        .map(|owner| owner.as_ref())
}

impl EditingSession {
    pub(crate) fn prepare_cast_list_save_candidate(&self) -> Result<Project, EditError> {
        cast_list::prepare_save_candidate(self.project()).map_err(EditError::CastList)
    }

    pub(crate) fn prepare_saved_rebase(&self, mut project: Project) -> Result<Self, EditError> {
        project.validate()?;
        project.share_unchanged_body_owners_from(self.project());
        if let Some(preservation) = project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        let mut rebased = self.clone();
        let selection = map_selection_to_project(self.selection, self.project(), &project)
            .ok_or(EditError::InvalidSelection)?;
        let title_selection =
            map_title_selection_to_project(self.title_selection, self.project(), &project);
        let parenthetical_key = ElementTypeKey::from_role(Role::Parenthetical);
        let generated_parentheticals = self
            .generated_parentheticals
            .iter()
            .filter_map(|old_id| {
                let old = self
                    .project()
                    .elements
                    .iter()
                    .find(|owner| owner.id.as_str() == old_id)?;
                if old.element_type() != parenthetical_key || old.text != "()" {
                    return None;
                }
                let new = map_body_owner_identity(self.project(), &old.id, &project)?;
                (new.element_type() == parenthetical_key && new.text == "()")
                    .then(|| new.id.as_str().to_owned())
            })
            .collect();

        rebased.project = project.clone();
        rebased.clean_project = project;
        rebased.selection = selection;
        rebased.title_selection = title_selection;
        rebased.generated_parentheticals = generated_parentheticals;
        rebased.dirty = false;
        rebased.reset_derived_after_saved_rebase();
        // Saving changes the preservation baseline and therefore invalidates
        // render caches even when the visible semantic text did not change.
        rebased.revision = rebased.revision.saturating_add(1);
        rebased.validate()?;
        Ok(rebased)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptcraft_model::{ElementDefinition, ElementTypeKey};

    fn project_with_changed_action(change: impl FnOnce(&mut ElementDefinition)) -> Project {
        let mut project = Project::new().expect("the default project should be valid");
        let mut definitions = project
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let action = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == "Action")
            .expect("the default project should define Action");
        change(action);
        project
            .replace_element_definitions(definitions)
            .expect("the changed definitions should remain valid");
        project
    }

    #[test]
    fn save_projection_compares_ordered_complete_definitions() {
        let original = Project::new().expect("the default project should be valid");
        assert!(same_save_projection(&original, &original.clone()));

        let mut reordered = original.clone();
        reordered
            .replace_element_definitions(
                reordered
                    .element_definitions()
                    .iter()
                    .cloned()
                    .rev()
                    .collect(),
            )
            .expect("reordered unique definitions should remain valid");
        assert_eq!(original.elements, reordered.elements);
        assert_eq!(original.title_page, reordered.title_page);
        assert!(!same_save_projection(&original, &reordered));

        let changed_behavior = project_with_changed_action(|definition| {
            definition.behavior.paginate_as = Some("Dialogue".to_owned());
        });
        let changed_outline = project_with_changed_action(|definition| {
            definition.outline.can_hide = !definition.outline.can_hide;
        });
        let changed_color = project_with_changed_action(|definition| {
            definition.font.color = Some("#87878F8FE0E0".to_owned());
        });
        for changed in [changed_behavior, changed_outline, changed_color] {
            assert_eq!(original.elements, changed.elements);
            assert_eq!(original.title_page, changed.title_page);
            assert!(!same_save_projection(&original, &changed));
        }

        let mut changed_direct_override = original.clone();
        changed_direct_override.direct_paragraph_overrides.insert(
            changed_direct_override.elements[0].id.clone(),
            scriptcraft_model::formatting::SourceParagraphOverrides {
                starts_new_page: Some(true),
                ..Default::default()
            },
        );
        assert!(!same_save_projection(&original, &changed_direct_override));

        let changed_case = project_with_changed_action(|definition| {
            definition.key =
                ElementTypeKey::new("action").expect("a case-distinct key should be valid");
        });
        assert_eq!(original.elements, changed_case.elements);
        assert_eq!(original.title_page, changed_case.title_page);
        assert!(!same_save_projection(&original, &changed_case));
    }
}
