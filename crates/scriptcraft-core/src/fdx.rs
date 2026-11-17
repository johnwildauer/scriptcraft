//! Transitional FDX compatibility facade; the final codec API is owned by
//! `scriptcraft-fdx` and this adapter is scheduled for later removal.

pub use scriptcraft_fdx::{
    current_timestamp, flat_offset_to_position, import_fdx, parse_flat_selection,
    preflight_version, read_v6, same_dual_dialogue_projection, save_diagnostic_at,
    selection_to_flat_offsets, write_v6, write_v6_for_save, FdxDocument, FdxError,
    FdxImportCandidate, FdxImportOutcome, FdxMigrationConsent, FdxMigrationKind,
    FdxVersionDirection, FdxVersionStatus, ModelErrorKind, PreservationErrorKind, SaveContext,
    SaveDiagnosticCode, SaveOperation, TextState, WindowState, MAX_ATTRIBUTES_PER_ELEMENT,
    MAX_IMAGE_BYTES, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH, MAX_SOURCE_EVENTS,
    MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES, MAX_TOTAL_IMAGE_BYTES,
    MAX_TEXT_SCALING, MIN_TEXT_SCALING,
};
