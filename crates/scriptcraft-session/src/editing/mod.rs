//! Headless, transactional editing over the semantic screenplay model.

use scriptcraft_model::Project;
use std::collections::BTreeSet;
use std::sync::Arc;

use scriptcraft_model::{
    cast_list::CastListError,
    error::ModelError,
    formatted_text::FormattedTextError,
    preservation::{ManagedTextMismatchSite, PreservationError},
};

pub use scriptcraft_model::position::{Position, Selection};
mod state;
pub use state::EditingSession;
mod selection;
pub(crate) use selection::{map_selection_to_project, map_title_selection_to_project};
use selection::{validate_position, validate_selection};
mod candidate;
mod diagnostics;
pub use diagnostics::EditMutationDiagnostics;
mod derived;
pub use derived::DerivedWorkSummary;
mod definitions;
pub use definitions::{ElementDefinitionCandidate, ElementTypeRemapping};
mod body;
mod dual_dialogue;
mod formatting;
mod history;
mod policy;
mod structure;
mod text;
mod title;
pub use policy::{
    enter_transition, tab_transition, transition, ParagraphKeyTarget, ParagraphKeyTargets,
};

/// Ephemeral editing behavior. This is session/application input, not document
/// state and is intentionally absent from snapshots and native serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditPolicy {
    pub auto_parenthetical: bool,
    pub strict_transition_boundaries: bool,
}

impl Default for EditPolicy {
    fn default() -> Self {
        Self {
            auto_parenthetical: true,
            strict_transition_boundaries: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidElement,
    InvalidUtf8Boundary,
    InvalidSelection,
    EmptyHistory,
    TextTooLong,
    TooManyElements,
    UnsupportedTransition {
        from: scriptcraft_model::Role,
    },
    /// Inline Alts is valid only while its exact source owner remains Dialogue.
    /// This failure is intentionally not eligible for automatic dual unwrap.
    InlineAltsRoleChange,
    Preservation(PreservationError),
    /// A definition/remapping request or retained settings projection was invalid.
    InvalidDefinitionCandidate,
    CastList(CastListError),
    FormattedText(FormattedTextError),
    Model(ModelError),
}

impl EditError {
    /// Returns only the bounded call-site label for a managed-text mismatch.
    pub fn managed_text_mismatch_diagnostic(&self) -> Option<&'static str> {
        let site = match self {
            Self::Preservation(PreservationError::ManagedTextMismatchAt(site)) => Some(*site),
            Self::Model(ModelError::InvalidPreservationState(
                PreservationError::ManagedTextMismatchAt(site),
            )) => Some(*site),
            Self::CastList(CastListError::Preservation(
                PreservationError::ManagedTextMismatchAt(site),
            )) => Some(*site),
            Self::CastList(CastListError::Model(ModelError::InvalidPreservationState(
                PreservationError::ManagedTextMismatchAt(site),
            ))) => Some(*site),
            Self::CastList(CastListError::SceneMetadata(
                scriptcraft_model::scene_metadata::SceneMetadataError::Preservation(
                    PreservationError::ManagedTextMismatchAt(site),
                ),
            )) => Some(*site),
            _ => None,
        };
        site.map(ManagedTextMismatchSite::diagnostic_name)
    }

    /// Converts scheduled derived-refresh failures to a bounded category.
    pub fn scheduled_refresh_failure_category(&self) -> &'static str {
        match self {
            Self::CastList(CastListError::SceneMetadata(_)) => "scene reconciliation",
            Self::CastList(CastListError::Preservation(_)) => "Cast List source mapping",
            Self::CastList(CastListError::Model(_)) => "model validation",
            Self::CastList(CastListError::UnsupportedFormattedCastList) => {
                "formatted Cast List mapping"
            }
            _ => "other safe refresh failure",
        }
    }
}

impl From<ModelError> for EditError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

impl From<FormattedTextError> for EditError {
    fn from(error: FormattedTextError) -> Self {
        Self::FormattedText(error)
    }
}
fn validate_full_candidate(project: &Project) -> Result<(), ModelError> {
    project.validate()
}

fn invalidate_changed_generated_parentheticals(
    before: &Project,
    after: &Project,
    prior_generated: &BTreeSet<String>,
    generated: &mut BTreeSet<String>,
) {
    for id in prior_generated {
        if !generated.contains(id) {
            continue;
        }
        let Some(before_element) = before
            .elements
            .iter()
            .find(|element| element.id.as_str() == id)
        else {
            continue;
        };
        let Some(after_element) = after
            .elements
            .iter()
            .find(|element| element.id.as_str() == id)
        else {
            continue;
        };
        if !Arc::ptr_eq(before_element, after_element) && before_element != after_element {
            generated.remove(id);
        }
    }
}
