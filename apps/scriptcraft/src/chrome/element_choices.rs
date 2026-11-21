//! Read-only projections of the current document's ordered element registry.

use scriptcraft_model::{DocumentElementDefinitions, ElementTypeKey};

/// One toolbar choice whose label is always the exact open key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElementChoice {
    pub(crate) key: ElementTypeKey,
}

impl ElementChoice {
    pub(crate) fn label(&self) -> &str {
        self.key.as_str()
    }
}

/// Projects every current definition in registry order without filtering or
/// normalizing labels. Callers keep this snapshot for the complete toolbar pass.
pub(crate) fn ordered_element_choices(
    definitions: &DocumentElementDefinitions,
) -> Vec<ElementChoice> {
    definitions
        .iter()
        .map(|definition| ElementChoice {
            key: definition.key.clone(),
        })
        .collect()
}

