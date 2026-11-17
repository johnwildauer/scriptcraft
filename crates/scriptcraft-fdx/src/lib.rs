#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! FDX version/settings mapping, outcomes, save context, diagnostics, and root
//! view-state mapping.

mod constants;
mod content;
mod identity;
mod import;
mod lineage;
mod new_document;
mod reader;
mod reconcile;
mod settings;
mod title_page;
mod tree;
mod version;
mod whitespace;
mod window_state;
mod writer;
mod verification;
mod context;
mod diagnostics;
mod document;
mod text_state;

pub use context::{current_timestamp, SaveContext, SaveOperation};
pub use diagnostics::{FdxError, ModelErrorKind, PreservationErrorKind, SaveDiagnosticCode, save_diagnostic_at};
pub use document::{
    FdxDocument, FdxImportCandidate, FdxImportOutcome, FdxMigrationConsent, FdxMigrationKind,
    FdxVersionDirection, FdxVersionStatus, WindowState,
};
pub use scriptcraft_model::preservation::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_IMAGE_BYTES, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH,
    MAX_SOURCE_EVENTS, MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES,
    MAX_TOTAL_IMAGE_BYTES,
};
pub use import::import_fdx;
pub use text_state::{
    flat_offset_to_position, parse_flat_selection, selection_to_flat_offsets, TextState,
    MAX_TEXT_SCALING, MIN_TEXT_SCALING,
};
pub use version::preflight_version;
pub use reader::read_v6;
pub use verification::same_dual_dialogue_projection;
pub use writer::{write_v6, write_v6_for_save};
