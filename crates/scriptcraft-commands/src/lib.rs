//! UI-independent command vocabulary, versioned wire envelope, and headless editor executor.
//!
//! The crate contains no desktop state. Its semantic editor executor operates
//! on the real session EditingSession; other commands remain descriptions for host
//! adapters. Serde's data model is the serialization boundary so hosts may
//! select a concrete transport format without coupling this crate to a codec.

#![forbid(unsafe_code)]

mod values;
pub use values::{
    Appearance, EditorSurface, HorizontalDirection, ImportResolution, LineEdge,
    ParagraphAlignmentValue, PdfExportOptions, PointerPhase, RoleValue, ScriptViewMode,
    SprintTimerMode, TitleTextStyle, VerticalDirection,
};

mod conversions;
pub use conversions::{
    pointer_selection, DiagnosticMessage, DocumentPath, InvalidDiagnosticMessage,
    InvalidDocumentPath, InvalidZoomFactor, InvalidZoomPercent, Position, PositionError, Selection,
    SelectionError, ZoomFactor, ZoomPercent, MAX_DIAGNOSTIC_MESSAGE_BYTES,
};

mod command;
pub use command::{Command, CommandId, HostEffectDescription, UnknownCommandId};

mod wire;
pub use wire::{CommandEnvelope, CommandOrigin, EnvelopeError, COMMAND_WIRE_VERSION};

mod catalog;
pub use catalog::{
    descriptor_for, validate_descriptor_catalog, AvailabilityPolicy, CatalogError,
    CommandDescriptor, COMMAND_DESCRIPTORS,
};

pub mod executor;
pub use executor::{
    execute_editor_command, execute_editor_command_on_surface, CommandOutcome, CommandRejection,
    UnavailableReason,
};
