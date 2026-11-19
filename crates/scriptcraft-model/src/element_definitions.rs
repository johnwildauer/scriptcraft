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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting::{FixedPoint, ParagraphAlignment};
    use crate::{ElementBehavior, ElementFontSpec, ElementOutline, ElementParagraphSpec};

    fn definition(label: &str) -> Result<ElementDefinition, ModelError> {
        Ok(ElementDefinition {
            key: ElementTypeKey::new(label)?,
            font: ElementFontSpec {
                family: "Courier Final Draft".to_owned(),
                size: 12,
                bold: false,
                italic: false,
                underline: false,
                all_caps: false,
                hidden_text: false,
                color: None,
                adornment_style: 0,
                revision_id: 0,
            },
            paragraph: ElementParagraphSpec {
                alignment: ParagraphAlignment::Left,
                first_indent: FixedPoint::from_points(0),
                left_indent: FixedPoint::from_points(1),
                right_indent: FixedPoint::from_points(7),
                leading: "Regular".to_owned(),
                space_before: FixedPoint::from_points(0),
                spacing: 1,
                starts_new_page: false,
            },
            behavior: ElementBehavior {
                paginate_as: None,
                return_key: None,
                shortcut: Some(String::new()),
                tab_key: None,
                win_shortcut: Some(String::new()),
            },
            outline: ElementOutline {
                can_hide: false,
                level: 1,
            },
        })
    }

    #[test]
    fn lookup_is_exact_and_case_distinct_labels_coexist() {
        let definitions = [
            definition("Action"),
            definition("action"),
            definition("Lyrics"),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>();
        let Ok(definitions) = definitions else {
            assert!(false, "test definitions should be valid");
            return;
        };
        let registry = DocumentElementDefinitions::new(definitions);
        let Ok(registry) = registry else {
            assert!(false, "distinct exact labels should be accepted");
            return;
        };

        assert_eq!(registry.len(), 3);
        assert_eq!(
            registry
                .get_by_label("Action")
                .map(|item| item.key.as_str()),
            Some("Action")
        );
        assert_eq!(
            registry
                .get_by_label("action")
                .map(|item| item.key.as_str()),
            Some("action")
        );
        assert!(registry.get_by_label("ACTION").is_none());
        assert_eq!(
            registry
                .iter()
                .map(|item| item.key.as_str())
                .collect::<Vec<_>>(),
            vec!["Action", "action", "Lyrics"]
        );
    }

    #[test]
    fn duplicate_keys_fail_construction_and_insertion() {
        let definitions = [definition("Action"), definition("Action")]
            .into_iter()
            .collect::<Result<Vec<_>, _>>();
        let Ok(definitions) = definitions else {
            assert!(false, "test definitions should be valid");
            return;
        };
        let Err(error) = DocumentElementDefinitions::new(definitions) else {
            assert!(false, "duplicate exact keys should be rejected");
            return;
        };
        assert_eq!(error, ModelError::DuplicateElementDefinitionKey);

        let Ok(initial) = definition("Action") else {
            assert!(false, "test definition should be valid");
            return;
        };
        let Ok(mut registry) = DocumentElementDefinitions::new([initial]) else {
            assert!(false, "unique test definition should construct a registry");
            return;
        };
        assert_eq!(
            definition("Action").and_then(|item| registry.insert(item)),
            Err(ModelError::DuplicateElementDefinitionKey)
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn insertion_enforces_the_registry_bound_without_mutation() {
        let mut registry = DocumentElementDefinitions::default();
        for index in 0..MAX_DOCUMENT_ELEMENT_DEFINITIONS {
            let item = definition(&format!("Type {index}"));
            let Ok(item) = item else {
                assert!(false, "generated test key should be valid");
                return;
            };
            assert!(registry.insert(item).is_ok());
        }
        assert_eq!(
            definition("Beyond limit").and_then(|item| registry.insert(item)),
            Err(ModelError::TooManyElementDefinitions)
        );
        assert_eq!(registry.len(), MAX_DOCUMENT_ELEMENT_DEFINITIONS);
    }
}
