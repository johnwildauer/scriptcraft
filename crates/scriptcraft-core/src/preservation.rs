//! Transitional re-export adapter for model-owned preservation state.

pub use scriptcraft_model::preservation::{
    AnchorRelation, AnchorTarget, BoardEntity, BoardEntityKind, BoardGraph,
    DependencyConfidence, ForeignIdDomain, ForeignIdentity, ForeignReference,
    ManagedTextMismatchSite, NamespaceBinding, PreservationError, PreservationState,
    PreservedAttribute, PreservedElement, PreservedNode, PreservedSection, ReferencePolicy, SectionKind,
    SharedBytes, SharedText, SharedVec, SourceAnchor, SourceAnchorId, SourceEncoding,
    SourceName, SourceOwnership, SourceSnapshot, MAX_ATTRIBUTES_PER_ELEMENT,
    MAX_IMAGE_BYTES, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH, MAX_SOURCE_EVENTS,
    MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES, MAX_TOTAL_IMAGE_BYTES,
};
pub(crate) use scriptcraft_model::preservation::{
    is_user_paragraph_data_lookalike, is_user_paragraph_data_name,
};
