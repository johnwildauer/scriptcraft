//! FDX-aware construction of editing sessions.

use crate::editing::{EditError, EditPolicy, EditingSession};
use scriptcraft_fdx::current_timestamp;
use scriptcraft_model::Project;

impl EditingSession {
    pub fn new(project: Project) -> Result<Self, EditError> {
        Self::with_policy(project, EditPolicy::default())
    }

    pub fn with_policy(project: Project, policy: EditPolicy) -> Result<Self, EditError> {
        EditingSession::with_policy_layout_and_session_timestamp(
            project,
            policy,
            current_timestamp(),
            100,
        )
    }

    pub fn with_history_limit(project: Project, history_limit: usize) -> Result<Self, EditError> {
        EditingSession::with_policy_layout_and_session_timestamp(
            project,
            EditPolicy::default(),
            current_timestamp(),
            history_limit,
        )
    }
}
