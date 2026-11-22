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
    cast_list, BodyOwner, ElementTypeKey, Project, StableId, TitlePageParagraph,
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
    fn same_owner_overrides(expected: &Project, actual: &Project) -> bool {
        let expected_owned_overrides = expected
            .elements
            .iter()
            .filter(|owner| expected.direct_paragraph_overrides.contains_key(&owner.id))
            .count();
        let actual_owned_overrides = actual
            .elements
            .iter()
            .filter(|owner| actual.direct_paragraph_overrides.contains_key(&owner.id))
            .count();
        expected.elements.len() == actual.elements.len()
            && expected_owned_overrides == expected.direct_paragraph_overrides.len()
            && actual_owned_overrides == actual.direct_paragraph_overrides.len()
            && expected
                .elements
                .iter()
                .zip(&actual.elements)
                .all(|(expected_owner, actual_owner)| {
                    expected
                        .direct_paragraph_overrides
                        .get(&expected_owner.id)
                        == actual
                            .direct_paragraph_overrides
                            .get(&actual_owner.id)
                })
    }

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
        && expected.page_layout() == actual.page_layout()
        && same_owner_overrides(expected, actual)
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
            return None;
        }
    }

    // A no-foreign-ID fallback is deliberately content-unique rather than
    // positional or Type-dependent. Ambiguous repeated text is not identity.
    let mut old_matches = old_project
        .elements
        .iter()
        .filter(|owner| owner.text == old.text);
    let old_match = old_matches.next()?;
    if old_matches.next().is_some() || old_match.id != old.id {
        return None;
    }
    let mut new_matches = new_project
        .elements
        .iter()
        .filter(|owner| owner.text == old.text);
    let owner = new_matches.next()?;
    if new_matches.next().is_some() {
        return None;
    }
    Some(owner.as_ref())
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
        let generated_parentheticals = self
            .generated_parentheticals
            .iter()
            .filter_map(|old_id| {
                let old = self
                    .project()
                    .elements
                    .iter()
                    .find(|owner| owner.id.as_str() == old_id)?;
                let old_is_parenthetical = self
                    .project()
                    .element_semantics(&old.element_type())
                    .is_some_and(|semantics| semantics.capabilities().parenthetical);
                if !old_is_parenthetical || old.text != "()" {
                    return None;
                }
                let new = map_body_owner_identity(self.project(), &old.id, &project)?;
                let new_is_parenthetical = project
                    .element_semantics(&new.element_type())
                    .is_some_and(|semantics| semantics.capabilities().parenthetical);
                (new_is_parenthetical && new.text == "()").then(|| new.id.as_str().to_owned())
            })
            .collect();

        rebased.project = project.clone();
        rebased.refresh_layout_inputs_from_project();
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
