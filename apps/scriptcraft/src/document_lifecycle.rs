//! Transient desktop decisions for root-document lifecycle operations.
//!
//! [`UnsavedChangesChoice::DontSave`] is memory-only: it never reverts a
//! session, clears semantic dirty state, or changes a file. A confirmed close
//! is authorized immediately but only for the next root update's close
//! request; it is consumed once or expires if that update has no request.

/// The user's response to the native unsaved-changes prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnsavedChangesChoice {
    Save,
    DontSave,
    Cancel,
}

/// The typed result of attempting to save a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    Saved,
    Cancelled,
    Failed,
}

/// A lifecycle continuation requires both a successful save and a clean live
/// session; a successful result alone is not sufficient authorization.
pub(crate) fn clean_save_outcome(outcome: SaveOutcome, is_dirty: bool) -> SaveOutcome {
    match (outcome, is_dirty) {
        (SaveOutcome::Saved, false) => SaveOutcome::Saved,
        (SaveOutcome::Cancelled, _) => SaveOutcome::Cancelled,
        (SaveOutcome::Failed, _) | (SaveOutcome::Saved, true) => SaveOutcome::Failed,
    }
}

/// A root-document close observation, with dirty state supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootCloseRequest {
    pub(crate) is_dirty: bool,
}

/// The coordinator's pure decision for an observed root-document close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootCloseDecision {
    /// Permit an ordinary clean close.
    Allow,
    /// The caller must cancel the native close and request a user decision.
    CancelAndPrompt,
    /// Permit the one close authorized by the immediately preceding choice.
    AllowAuthorized,
}

/// One-shot, transient authorization for a confirmed root-document close.
#[derive(Debug, Default)]
pub(crate) struct RootCloseLifecycle {
    confirmed_close_armed: bool,
}

impl RootCloseLifecycle {
    /// Arms an in-memory authorization for the next root-update close request.
    /// This does not access or mutate the document session or its dirty state.
    pub(crate) fn arm_confirmed_close(&mut self) {
        self.confirmed_close_armed = true;
    }

    /// Resolves the close observation at the start of a root update.
    ///
    /// When authorization is armed, that authorization is consumed whether or
    /// not this update reports a close request. Thus it cannot survive beyond
    /// the next update, and a missing request expires it. Dirty state is always
    /// supplied by the caller (the live `EditingSession::dirty()` authority).
    pub(crate) fn begin_root_update(
        &mut self,
        request: Option<RootCloseRequest>,
    ) -> Option<RootCloseDecision> {
        if self.confirmed_close_armed {
            self.confirmed_close_armed = false;
            return request.map(|_| RootCloseDecision::AllowAuthorized);
        }

        request.map(|request| {
            if request.is_dirty {
                RootCloseDecision::CancelAndPrompt
            } else {
                RootCloseDecision::Allow
            }
        })
    }
}

/// Whether a prompt response authorizes reissuing root Close. Save must have
/// completed successfully and left the live session clean; Don't Save is purely
/// an in-memory confirmation and does not change the session.
pub(crate) fn confirmed_close_for_choice(
    choice: UnsavedChangesChoice,
    save_outcome: Option<SaveOutcome>,
) -> bool {
    match choice {
        UnsavedChangesChoice::Cancel => false,
        UnsavedChangesChoice::DontSave => true,
        UnsavedChangesChoice::Save => save_outcome == Some(SaveOutcome::Saved),
    }
}
