pub mod error;
pub mod identity;
pub mod shared;
pub mod tree;

mod dual_dialogue;
mod indexes;
mod navigation;
mod reconciliation;
mod state;
mod text;
mod title;
mod validation;

pub use error::{ManagedTextMismatchSite, PreservationError};
pub use identity::{
    AnchorRelation, AnchorTarget, BoardEntity, BoardEntityKind, BoardGraph, ForeignIdDomain,
    ForeignIdentity, ForeignReference, ReferencePolicy, SourceAnchor, SourceAnchorId,
};
pub use shared::{SharedBytes, SharedText, SharedVec};
pub use tree::{
    is_user_paragraph_data_lookalike, is_user_paragraph_data_name, DependencyConfidence,
    NamespaceBinding, PreservedAttribute, PreservedElement, PreservedNode, PreservedSection,
    SectionKind, SourceEncoding, SourceName, SourceOwnership, SourceSnapshot,
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_IMAGE_BYTES, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH,
    MAX_SOURCE_EVENTS, MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES,
    MAX_TOTAL_IMAGE_BYTES,
};

pub use state::PreservationState;
use indexes::SourceIdentityUseIndex;
use state::ManagedParagraphLocation;

/// Attach a stable call-site category to managed-text mismatch errors.
pub(super) fn categorize_managed_text_mismatch(
    error: PreservationError,
    site: ManagedTextMismatchSite,
) -> PreservationError {
    match error {
        PreservationError::ManagedTextMismatch => PreservationError::ManagedTextMismatchAt(site),
        other => other,
    }
}
