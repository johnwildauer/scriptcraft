//! Open, exact element-type labels used as document-local identities.
//!
//! These keys are canonical owned values. They do not normalize spelling or
//! require a corresponding [`Role`](crate::element::Role); the role conversion
//! methods are adapters for the existing fixed-role boundary only.

use crate::{element::Role, ModelError};
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

    /// Converts a known static role to its exact native FDX label.
    pub fn from_role(role: Role) -> Self {
        Self(
            match role {
                Role::SceneHeading => "Scene Heading",
                Role::Action => "Action",
                Role::Character => "Character",
                Role::Dialogue => "Dialogue",
                Role::Parenthetical => "Parenthetical",
                Role::Transition => "Transition",
                Role::Note => "Note",
                Role::NewAct => "New Act",
                Role::EndOfAct => "End of Act",
                Role::CastList => "Cast List",
                Role::General => "General",
                Role::Shot => "Shot",
                Role::Sequence => "Sequence",
                Role::Outline1 => "Outline 1",
                Role::Outline2 => "Outline 2",
                Role::Outline3 => "Outline 3",
                Role::Summary => "Summary",
            }
            .to_owned(),
        )
    }

    /// Returns the matching static role only for an exact known FDX label.
    pub fn to_role(&self) -> Option<Role> {
        Some(match self.0.as_str() {
            "Scene Heading" => Role::SceneHeading,
            "Action" => Role::Action,
            "Character" => Role::Character,
            "Dialogue" => Role::Dialogue,
            "Parenthetical" => Role::Parenthetical,
            "Transition" => Role::Transition,
            "Note" => Role::Note,
            "New Act" => Role::NewAct,
            "End of Act" => Role::EndOfAct,
            "Cast List" => Role::CastList,
            "General" => Role::General,
            "Shot" => Role::Shot,
            "Sequence" => Role::Sequence,
            "Outline 1" => Role::Outline1,
            "Outline 2" => Role::Outline2,
            "Outline 3" => Role::Outline3,
            "Summary" => Role::Summary,
            _ => return None,
        })
    }
}

impl From<Role> for ElementTypeKey {
    fn from(role: Role) -> Self {
        Self::from_role(role)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_preserve_arbitrary_and_case_distinct_labels() {
        assert_eq!(
            ElementTypeKey::new("Lyrics").map(ElementTypeKey::into_string),
            Ok("Lyrics".to_owned())
        );
        assert_ne!(ElementTypeKey::new("Lyrics"), ElementTypeKey::new("lyrics"));
        assert_eq!(
            ElementTypeKey::new("lyrics").map(|key| key.as_str().to_owned()),
            Ok("lyrics".to_owned())
        );
    }

    #[test]
    fn role_conversion_is_exact_and_covers_every_role() {
        for role in Role::ALL {
            assert_eq!(ElementTypeKey::from_role(role).to_role(), Some(role));
        }
        assert_eq!(
            ElementTypeKey::new("action").map(|key| key.to_role()),
            Ok(None)
        );
        assert!(ElementTypeKey::new("Action")
            .map(|key| key.to_role() == Some(Role::Action))
            .unwrap_or(false));
    }

    #[test]
    fn key_rejects_empty_and_over_limit_labels() {
        assert_eq!(
            ElementTypeKey::new(""),
            Err(ModelError::EmptyElementTypeKey)
        );
        assert_eq!(
            ElementTypeKey::new("x".repeat(MAX_ELEMENT_TYPE_KEY_BYTES + 1)),
            Err(ModelError::ElementTypeKeyTooLong)
        );
        assert!(ElementTypeKey::new("x".repeat(MAX_ELEMENT_TYPE_KEY_BYTES)).is_ok());
    }
}
