//! Ordered, exact-key registry of complete document-owned element definitions.
//!
//! Definition order is semantic source order. Construction and insertion
//! validate every owned value and reject duplicate keys without normalizing
//! labels; the backing vector is private so callers cannot bypass those rules.

use std::collections::BTreeSet;

use crate::{ElementDefinition, ElementTypeKey, ModelError};

/// Maximum number of definitions in one document registry.
pub const MAX_DOCUMENT_ELEMENT_DEFINITIONS: usize = 1_024;

/// Source-ordered, unique registry of document element definitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentElementDefinitions {
    definitions: Vec<ElementDefinition>,
}

impl DocumentElementDefinitions {
    /// Builds a registry in iterator order, validating each definition.
    pub fn new(
        definitions: impl IntoIterator<Item = ElementDefinition>,
    ) -> Result<Self, ModelError> {
        let mut registry = Self::default();
        for definition in definitions {
            registry.insert(definition)?;
        }
        Ok(registry)
    }

    /// Adds one definition at the end of source order.
    ///
    /// The registry remains unchanged if the definition is invalid, duplicate,
    /// or would exceed the document bound.
    pub fn insert(&mut self, definition: ElementDefinition) -> Result<(), ModelError> {
        if self.definitions.len() >= MAX_DOCUMENT_ELEMENT_DEFINITIONS {
            return Err(ModelError::TooManyElementDefinitions);
        }
        definition.validate()?;
        if self.get(&definition.key).is_some() {
            return Err(ModelError::DuplicateElementDefinitionKey);
        }
        self.definitions.push(definition);
        Ok(())
    }

    /// Returns a definition for the exact key, if present.
    pub fn get(&self, key: &ElementTypeKey) -> Option<&ElementDefinition> {
        self.get_by_label(key.as_str())
    }

    /// Returns a definition for the exact label, if present.
    pub fn get_by_label(&self, label: &str) -> Option<&ElementDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.key.as_str() == label)
    }

    /// Returns the definitions in source order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ElementDefinition> + DoubleEndedIterator {
        self.definitions.iter()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Rechecks bounds, key uniqueness, and all nested owned values.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.definitions.len() > MAX_DOCUMENT_ELEMENT_DEFINITIONS {
            return Err(ModelError::TooManyElementDefinitions);
        }
        let mut keys = BTreeSet::new();
        for definition in &self.definitions {
            definition.validate()?;
            if !keys.insert(&definition.key) {
                return Err(ModelError::DuplicateElementDefinitionKey);
            }
        }
        Ok(())
    }
}

