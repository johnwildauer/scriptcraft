#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreservationError {
    SourceTooLarge,
    InvalidEncoding,
    TooManyEvents,
    ExcessiveDepth,
    TooManyAttributes,
    FieldTooLarge,
    TextTooLarge,
    ImageTooLarge,
    TotalImageBytesTooLarge,
    DuplicateSourceOrder,
    InvalidSourceName,
    InvalidAnchor,
    DuplicateAnchor,
    UnboundAnchor,
    AnchorTargetMissing,
    DuplicateIdentity,
    MissingReferenceSource,
    DanglingReference,
    ImageLengthMismatch,
    ManagedAnchorMissing,
    ManagedAnchorAmbiguous,
    ManagedTextMismatch,
    /// Value-free call-site tag for managed projection mismatches.
    ManagedTextMismatchAt(ManagedTextMismatchSite),
    UnsupportedTextMapping,
    UnsupportedRunTopology,
    AmbiguousDirectAttribute,
    UnsupportedRoleMapping,
    StructuralAttachmentConflict,
    PotentialReferenceConflict,
    ParagraphIdentityEntropyUnavailable,
    ParagraphIdentityCollisionLimit,
}

/// Stable, bounded call-site categories for managed-text synchronization failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedTextMismatchSite {
    ScopedEditValidation,
    TextReplacement,
    TextElementsReplacement,
    ParagraphRemoval,
}

impl ManagedTextMismatchSite {
    pub const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::ScopedEditValidation => "scoped-edit-validation",
            Self::TextReplacement => "text-replacement",
            Self::TextElementsReplacement => "text-elements-replacement",
            Self::ParagraphRemoval => "paragraph-removal",
        }
    }
}
