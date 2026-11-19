use scriptcraft_model::{
    error::ModelError, paragraph_id::ParagraphIdAllocationError, preservation::PreservationError,
};
use std::fmt;

/// Bounded, privacy-safe reader failures. Variants intentionally carry no source
/// text, identifiers, file paths, or image payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdxError {
    SourceTooLarge,
    InvalidEncoding,
    MalformedXml,
    ForbiddenDoctype,
    UnsupportedEntity,
    UnsupportedVersion,
    MissingVersion,
    InvalidVersion,
    DuplicateVersion,
    InvalidRoot,
    MissingContent,
    DuplicateSection,
    ResourceLimit,
    InvalidValue,
    AmbiguousIdentity,
    DanglingReference,
    UnsupportedConstruct,
    /// Save-path UnsupportedConstruct failure with a stable call-site code.
    SaveDiagnosticUnsupportedConstruct(u16),
    /// Code 30 with bounded, value-free structural detail for a normalized tree mismatch.
    SaveDiagnosticSourceTreeMismatch(String),
    ParagraphIdentityEntropyUnavailable,
    ParagraphIdentityCollisionLimit,
    Model(ModelErrorKind),
    Preservation(PreservationErrorKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelErrorKind {
    InvalidModel,
    InvalidIdentifier,
    InvalidText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreservationErrorKind {
    InvalidState,
    InvalidIdentity,
    InvalidAnchor,
    InvalidReference,
    ResourceLimit,
}

impl fmt::Display for FdxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SourceTooLarge => "FDX source exceeds the configured size limit",
            Self::InvalidEncoding => "FDX source is not valid UTF-8",
            Self::MalformedXml => "FDX XML is malformed or unsupported",
            Self::ForbiddenDoctype => "FDX document types are not accepted",
            Self::UnsupportedEntity => "FDX contains an unsupported entity reference",
            Self::UnsupportedVersion => "FDX is not declared as Version 6",
            Self::MissingVersion => "FDX version declaration is missing",
            Self::InvalidVersion => "FDX version declaration is not a decimal integer",
            Self::DuplicateVersion => "FDX contains duplicate version declarations",
            Self::InvalidRoot => "FDX root element is not a Script document",
            Self::MissingContent => "FDX Content section is missing",
            Self::DuplicateSection => "FDX contains ambiguous duplicate managed sections",
            Self::ResourceLimit => "FDX exceeds a configured parser resource limit",
            Self::InvalidValue => "FDX contains an invalid value",
            Self::AmbiguousIdentity => "FDX contains an invalid or ambiguous identity",
            Self::DanglingReference => "FDX contains an unresolved internal reference",
            Self::UnsupportedConstruct => {
                "FDX contains a construct that cannot be projected safely"
            }
            Self::SaveDiagnosticUnsupportedConstruct(code) => {
                return write!(
                    formatter,
                    "{code}: FDX contains a construct that cannot be projected safely"
                );
            }
            Self::SaveDiagnosticSourceTreeMismatch(detail) => {
                return write!(
                    formatter,
                    "30: FDX contains a construct that cannot be projected safely; {detail}"
                );
            }
            Self::ParagraphIdentityEntropyUnavailable => {
                "secure randomness for a new FDX paragraph identity is unavailable"
            }
            Self::ParagraphIdentityCollisionLimit => {
                "a unique FDX paragraph identity could not be allocated within the retry limit"
            }
            Self::Model(_) => "FDX semantic projection failed model validation",
            Self::Preservation(_) => "FDX preservation state failed validation",
        })
    }
}

impl std::error::Error for FdxError {}

/// Stable Save/Save As diagnostics. Values are persistent diagnostic identifiers,
/// not source-line numbers. Keep this map centralized and append new values rather
/// than reassigning existing ones. The corresponding human-readable map is also
/// recorded in `.forge/blueprints/technical-context.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum SaveDiagnosticCode {
    WriteSourceFormat = 1,
    WriteOriginalRereadPropagation = 2,
    WriteOriginalPreservationMissing = 3,
    WriteOriginalIdentityGraphMismatch = 4,
    ReconcileProjectManagedOrderMismatch = 5,
    ReconcileNodeDuplicateManagedIdentity = 6,
    ReconcileNodeManagedOutsideSection = 7,
    ReconcileNodeSemanticElementMissing = 8,
    ReconcileProjectNodePropagation = 9,
    ReconcileParagraphRoleMismatch = 10,
    ReconcileParagraphTypeAttributeMismatch = 11,
    ReconcileParagraphEmptyRunsMismatch = 12,
    ReconcileParagraphCdataInEmptyText = 13,
    ReconcileParagraphUnsupportedEmptyChild = 14,
    ReconcileParagraphUnexpectedEmptyText = 15,
    ReconcileParagraphRunCountMismatch = 16,
    ReconcileParagraphRunProjectionMismatch = 17,
    ReconcileParagraphUnexpectedFirstChild = 18,
    ReconcileParagraphUnexpectedLaterChild = 19,
    NewDocumentHeadingNodeMismatch = 20,
    NewDocumentScenePropertiesUnsupported = 21,
    NewParagraphEmptyRunsMismatch = 22,
    NewParagraphNamespacedRunAttribute = 23,
    WriteElementNamespacePrefixMissing = 24,
    WriteOutputRereadPropagation = 25,
    WriteSemanticRereadMismatch = 26,
    WriteOutputPreservationMissing = 27,
    // Reserved diagnostic slot; candidate-preservation failure currently maps to
    // the output-preservation check, but this identifier must not be recycled.
    #[allow(dead_code)]
    WriteCandidatePreservationMissing = 28,
    WriteIdentitySetMismatch = 29,
    WriteNormalizedSourceTreeMismatch = 30,
    StorageRereadPropagation = 31,
    StorageSemanticProjectionMismatch = 32,
    ReconcileProjectChildPropagation = 33,
    ReconcileManagedParagraphPropagation = 34,
    LineageAmbiguousDocumentRef = 35,
    LineageAmbiguousDocumentRefDateTime = 36,
    LineageIdentityEntropyUnavailable = 37,
    LineageIdentityCollisionLimit = 38,
}

impl SaveDiagnosticCode {
    const fn number(self) -> u16 {
        self as u16
    }

    pub const fn error(self) -> FdxError {
        FdxError::SaveDiagnosticUnsupportedConstruct(self.number())
    }
}

/// Retained as the storage-facing diagnostic propagation API.
pub fn save_diagnostic_at(error: FdxError, code: SaveDiagnosticCode) -> FdxError {
    if error == FdxError::UnsupportedConstruct {
        code.error()
    } else {
        error
    }
}

pub(crate) fn map_paragraph_id_error(error: ParagraphIdAllocationError) -> FdxError {
    match error {
        ParagraphIdAllocationError::EntropyUnavailable => {
            FdxError::ParagraphIdentityEntropyUnavailable
        }
        ParagraphIdAllocationError::CollisionLimit => FdxError::ParagraphIdentityCollisionLimit,
    }
}

pub(crate) fn map_model_error(error: ModelError) -> FdxError {
    match error {
        ModelError::EmptyId | ModelError::InvalidId | ModelError::DuplicateId => {
            FdxError::Model(ModelErrorKind::InvalidIdentifier)
        }
        ModelError::TextTooLong | ModelError::StringTooLong => {
            FdxError::Model(ModelErrorKind::InvalidText)
        }
        _ => FdxError::Model(ModelErrorKind::InvalidModel),
    }
}

pub(crate) fn map_preservation_error(error: PreservationError) -> FdxError {
    use PreservationError as Error;
    match error {
        Error::SourceTooLarge
        | Error::TooManyEvents
        | Error::ExcessiveDepth
        | Error::TooManyAttributes
        | Error::FieldTooLarge
        | Error::TextTooLarge
        | Error::ImageTooLarge
        | Error::TotalImageBytesTooLarge => {
            FdxError::Preservation(PreservationErrorKind::ResourceLimit)
        }
        Error::DuplicateIdentity => FdxError::AmbiguousIdentity,
        Error::ParagraphIdentityEntropyUnavailable => FdxError::ParagraphIdentityEntropyUnavailable,
        Error::ParagraphIdentityCollisionLimit => FdxError::ParagraphIdentityCollisionLimit,
        Error::DanglingReference | Error::MissingReferenceSource | Error::ImageLengthMismatch => {
            FdxError::DanglingReference
        }
        Error::InvalidAnchor
        | Error::DuplicateAnchor
        | Error::UnboundAnchor
        | Error::AnchorTargetMissing
        | Error::ManagedAnchorMissing
        | Error::ManagedAnchorAmbiguous => {
            FdxError::Preservation(PreservationErrorKind::InvalidAnchor)
        }
        _ => FdxError::Preservation(PreservationErrorKind::InvalidState),
    }
}
