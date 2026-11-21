//! Read-only semantic and Behavior projections over current document definitions.

use crate::{ElementDefinition, ElementTypeKey, SemanticCapabilities};

/// Selects one of the document-owned paragraph Behavior destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphBehaviorKey {
    Return,
    Tab,
}

/// Exact resolution state for a present or absent Behavior destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorTargetResolution {
    /// The destination exists in the current registry. Same-key targets remain resolved.
    Resolved(ElementTypeKey),
    /// The source definition has no destination for the selected Behavior key.
    Absent,
    /// The source definition names a destination that is not in the current registry.
    Unresolved(ElementTypeKey),
}

/// Immutable view of one current definition and its exact-key capabilities.
///
/// The view borrows the complete definition from the Project registry; it is
/// neither stored on Project nor serialized as independent semantic state.
#[derive(Debug, Clone, Copy)]
pub struct ElementSemantics<'a> {
    definition: &'a ElementDefinition,
}

impl<'a> ElementSemantics<'a> {
    pub(crate) fn new(definition: &'a ElementDefinition) -> Self {
        Self { definition }
    }

    /// Returns the exact key that selected this definition.
    pub fn key(&self) -> &'a ElementTypeKey {
        &self.definition.key
    }

    /// Returns the complete current document-owned definition, including raw shortcuts.
    pub fn definition(&self) -> &'a ElementDefinition {
        self.definition
    }

    /// Returns capabilities derived solely from this definition's exact key.
    pub fn capabilities(&self) -> SemanticCapabilities {
        self.definition.capabilities()
    }
}

