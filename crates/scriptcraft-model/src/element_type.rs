//! Open, exact element-type labels used as document-local identities.
//!
//! These keys are canonical owned values. They do not normalize spelling or
//! require membership in a closed vocabulary.

use crate::ModelError;
use std::fmt;

/// Maximum UTF-8 byte length for an element type key.
pub const MAX_ELEMENT_TYPE_KEY_BYTES: usize = 4_096;

/// An owned, exact, case-sensitive element Type label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementTypeKey(String);

impl ElementTypeKey {
    /// Creates a key without trimming, case folding, or other normalization.
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ModelError::EmptyElementTypeKey);
        }
        if value.len() > MAX_ELEMENT_TYPE_KEY_BYTES {
            return Err(ModelError::ElementTypeKeyTooLong);
        }
        Ok(Self(value))
    }

    /// Returns the exact borrowed label.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extracts the owned label without changing it.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for ElementTypeKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ElementTypeKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

