//! Read-only projections of the current document's ordered element registry.

use scriptcraft_model::{DocumentElementDefinitions, ElementTypeKey, Project};
use std::sync::Arc;

/// One toolbar choice whose label is always the exact open key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElementChoice {
    pub(crate) key: ElementTypeKey,
}

/// Identity of the exact current ordered-choice snapshot. The model identity
/// prevents same-revision document replacement from reusing a stale registry;
/// the ordered values detect definition changes within the same document.
#[derive(Debug, Clone)]
pub(crate) struct ElementChoicesSnapshot {
    document_identity: Arc<()>,
    choices: Arc<[ElementChoice]>,
}

impl ElementChoicesSnapshot {
    pub(crate) fn new(document_identity: Arc<()>, choices: Arc<[ElementChoice]>) -> Self {
        Self {
            document_identity,
            choices,
        }
    }

    pub(crate) fn choices(&self) -> &[ElementChoice] {
        &self.choices
    }
}

impl PartialEq for ElementChoicesSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document_identity, &other.document_identity)
            && self.choices == other.choices
    }
}

impl Eq for ElementChoicesSnapshot {}

/// Shared menu-facing projection; rows always come from the exact controls
/// snapshot and the checked key is supplied only for an eligible Body target.
#[derive(Debug, Clone)]
pub(crate) struct DynamicElementMenuProjection {
    pub(crate) snapshot: ElementChoicesSnapshot,
    pub(crate) focused_type_key: Option<ElementTypeKey>,
}

impl DynamicElementMenuProjection {
    pub(crate) fn choices(&self) -> &[ElementChoice] {
        self.snapshot.choices()
    }
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

/// Shared focused Body type projection for the toolbar and generated menus.
pub(crate) fn focused_body_element_type_key(
    project: &Project,
    focus_element: usize,
) -> Option<ElementTypeKey> {
    let key = project.elements.get(focus_element)?.element_type();
    project
        .element_definitions()
        .get(&key)
        .map(|definition| definition.key.clone())
}
