//! Value-free development diagnostics for Body candidate mutation.

use std::time::Instant;

/// One development-only Body candidate-mutation observation. Durations are
/// value-free nanoseconds; optional stages are absent when their work did not
/// run. Retry time is a raw nested subset of setup plus edit-operation time.
/// Explicit preservation time covers post-operation index/reconciliation work;
/// the edit operation can also perform its own nested source reconciliation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EditMutationDiagnostics {
    pub total_nanos: u64,
    pub candidate_setup_nanos: u64,
    pub edit_operation_nanos: u64,
    pub structural_retry_nanos: Option<u64>,
    pub preservation_nanos: u64,
    pub scoped_validation_nanos: Option<u64>,
    pub full_validation_nanos: Option<u64>,
    pub no_op_equality_nanos: Option<u64>,
    pub derived_classification_nanos: Option<u64>,
    pub dirty_comparison_nanos: Option<u64>,
    pub history_publication_nanos: Option<u64>,
    pub structural_retries: u64,
    pub scoped_candidate_attempts: u64,
    pub scoped_candidate_accepts: u64,
    pub full_candidate_validations: u64,
    pub full_validation_fallbacks: u64,
    pub no_op: bool,
}

fn diagnostic_duration_nanos(started: Instant) -> u64 {
    started.elapsed().as_nanos().min(u64::MAX as u128) as u64
}

pub(super) fn add_diagnostic_duration(target: &mut u64, started: Option<Instant>) {
    if let Some(started) = started {
        *target = target.saturating_add(diagnostic_duration_nanos(started));
    }
}

pub(super) fn add_optional_diagnostic_duration(target: &mut Option<u64>, started: Option<Instant>) {
    if let Some(started) = started {
        let elapsed = diagnostic_duration_nanos(started);
        *target = Some(target.unwrap_or(0).saturating_add(elapsed));
    }
}

