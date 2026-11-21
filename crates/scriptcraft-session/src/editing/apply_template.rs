//! Settings-only Apply Template as one ordinary session candidate.

use super::template_plan::plan_element_template_application;
use super::EditingSession;
use scriptcraft_fdx::{
    extract_element_template_settings, instantiate_builtin_template, BuiltinTemplateId,
    ElementTemplateSettings,
};
use std::fmt;

/// The publication result of one Apply Template attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyTemplateOutcome {
    /// The generic definition candidate changed and was published once.
    Changed,
    /// The complete candidate was equal to the current session state.
    NoOp,
}

/// Bounded, privacy-safe failure categories for built-in and lower-level Apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyTemplateError {
    /// The selected built-in could not be loaded through the common reader.
    TargetLoadFailed,
    /// The loaded built-in could not be reduced to bounded template settings.
    TargetExtractionFailed,
    /// The current and target definition/type projections could not be planned.
    PlanningFailed,
    /// The detached generic candidate failed source projection or validation.
    CandidateRejected,
}

impl fmt::Display for ApplyTemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TargetLoadFailed => "built-in template loading failed",
            Self::TargetExtractionFailed => "built-in template extraction failed",
            Self::PlanningFailed => "template application planning failed",
            Self::CandidateRejected => "template application candidate was rejected",
        })
    }
}

impl std::error::Error for ApplyTemplateError {}

impl EditingSession {
    /// Loads and applies one built-in template through the common FDX loader
    /// and settings-only extraction path. The full target document is dropped
    /// before the current session is planned or mutated.
    pub fn apply_builtin_template(
        &mut self,
        id: BuiltinTemplateId,
    ) -> Result<ApplyTemplateOutcome, ApplyTemplateError> {
        let template = {
            let document = instantiate_builtin_template(id)
                .map_err(|_| ApplyTemplateError::TargetLoadFailed)?;
            extract_element_template_settings(&document)
                .map_err(|_| ApplyTemplateError::TargetExtractionFailed)?
        };

        self.apply_element_template_settings(&template)
    }

    /// Applies detached template settings through one ordinary undoable session
    /// candidate. The current clean baseline, selections, storage target, and
    /// filesystem are not replaced or otherwise changed by this operation.
    pub fn apply_element_template_settings(
        &mut self,
        template: &ElementTemplateSettings,
    ) -> Result<ApplyTemplateOutcome, ApplyTemplateError> {
        let mut candidate =
            plan_element_template_application(&self.project, template.definitions())
                .map_err(|_| ApplyTemplateError::PlanningFailed)?;

        if self.project.preservation.is_some() {
            candidate.retained_settings_replacement = Some(template.retained_settings().to_vec());
        }

        let previous_mutation_token = self.mutation_token;
        self.replace_element_definitions_and_remap_types(candidate)
            .map_err(|_| ApplyTemplateError::CandidateRejected)?;

        if self.mutation_token == previous_mutation_token {
            Ok(ApplyTemplateOutcome::NoOp)
        } else {
            Ok(ApplyTemplateOutcome::Changed)
        }
    }
}
