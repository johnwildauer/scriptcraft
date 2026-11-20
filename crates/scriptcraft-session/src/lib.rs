#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Transactional editing sessions and native document composition.

pub mod editing;
pub mod persistence;
pub mod storage;

pub use editing::{
    ApplyTemplateError, ApplyTemplateOutcome, DerivedWorkSummary, EditError,
    EditMutationDiagnostics, EditPolicy, ParagraphKeyTarget, ParagraphKeyTargets, Position,
    Selection,
};
