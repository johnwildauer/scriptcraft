#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! FDX version/settings mapping, outcomes, save context, diagnostics, and root
//! view-state mapping.

mod apply_template;
mod builtin_screenplay;
mod builtin_template_preview;
mod builtin_templates;
mod constants;
mod content;
mod context;
mod diagnostics;
mod document;
mod element_settings;
mod element_settings_import;
mod element_settings_serialization;
mod identity;
mod import;
mod lineage;
mod new_document;
mod reader;
mod reconcile;
mod settings;
mod split_state;
mod text_state;
mod title_page;
mod tree;
mod verification;
mod version;
mod whitespace;
mod window_state;
mod writer;

pub use apply_template::{extract_element_template_settings, ElementTemplateSettings};
pub use builtin_template_preview::builtin_template_preview_description;
pub use builtin_templates::{
    builtin_template_metadata, builtin_templates, instantiate_builtin_template, BuiltinTemplateId,
    BuiltinTemplateMetadata, DEFAULT_BUILTIN_TEMPLATE_ID,
};
pub use context::{current_timestamp, SaveContext, SaveOperation};
pub use diagnostics::{
    save_diagnostic_at, FdxError, ModelErrorKind, PreservationErrorKind, SaveDiagnosticCode,
};
pub use document::{
    FdxDocument, FdxImportCandidate, FdxImportDiagnostic, FdxImportDiagnostics, FdxImportOutcome,
    FdxMigrationConsent, FdxMigrationKind, FdxVersionDirection, FdxVersionStatus, InvisiblesColor,
    InvisiblesState, ScriptPanelDisplayMode, WindowState, MAX_FDX_IMPORT_DIAGNOSTICS,
};
pub use element_settings::{reconcile_project_element_settings, replace_project_element_settings};
pub use import::import_fdx;
pub use reader::read_v6;
pub use scriptcraft_model::preservation::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_IMAGE_BYTES, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH,
    MAX_SOURCE_EVENTS, MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES,
    MAX_TOTAL_IMAGE_BYTES,
};
pub use text_state::{
    flat_offset_to_position, parse_flat_selection, selection_to_flat_offsets, TextState,
    MAX_TEXT_SCALING, MIN_TEXT_SCALING,
};
pub use verification::same_dual_dialogue_projection;
pub use version::preflight_version;
pub use writer::{write_v6, write_v6_for_save};
