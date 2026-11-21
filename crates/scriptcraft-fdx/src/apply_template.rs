//! Detached, settings-only extraction for built-in template application.

use crate::diagnostics::FdxError;
use crate::document::FdxDocument;
use crate::element_settings::ordered_detached_element_settings_winners;
use scriptcraft_model::element_definitions::{
    DocumentElementDefinitions, MAX_DOCUMENT_ELEMENT_DEFINITIONS,
};
use scriptcraft_model::preservation::PreservedElement;

/// A bounded template projection containing only normalized definitions and
/// their matching retained direct-root settings winners.
///
/// It deliberately has no document, content, preservation graph, source bytes,
/// path, view state, or storage/session state. Private fields keep the ordered
/// registry and retained-node vector synchronized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementTemplateSettings {
    definitions: DocumentElementDefinitions,
    retained_settings: Vec<PreservedElement>,
}

impl ElementTemplateSettings {
    /// Returns the complete normalized ordered target registry.
    pub fn definitions(&self) -> &DocumentElementDefinitions {
        &self.definitions
    }

    /// Returns one detached retained winning settings node per definition, in
    /// the same exact-key order as [`Self::definitions`].
    pub fn retained_settings(&self) -> &[PreservedElement] {
        &self.retained_settings
    }
}

/// Reduces a fresh source-backed common-reader result to settings needed by
/// Apply Template. The loaded project is cloned and reconciled locally; the
/// supplied document and its source snapshot/indexes are never mutated.
pub fn extract_element_template_settings(
    document: &FdxDocument,
) -> Result<ElementTemplateSettings, FdxError> {
    let mut detached = document.project.clone();
    crate::element_settings::reconcile_project_element_settings(&mut detached)?;
    let preservation = detached
        .preservation
        .as_ref()
        .ok_or(FdxError::InvalidValue)?;

    let definitions = detached.element_definitions().clone();
    definitions
        .validate()
        .map_err(crate::diagnostics::map_model_error)?;
    if definitions.len() > MAX_DOCUMENT_ELEMENT_DEFINITIONS {
        return Err(FdxError::ResourceLimit);
    }
    if definitions.is_empty() {
        return Err(FdxError::InvalidValue);
    }
    if definitions
        .iter()
        .filter(|definition| definition.key.as_str() == "Action")
        .count()
        != 1
        || definitions.get_by_label("Action").is_none()
    {
        return Err(FdxError::InvalidValue);
    }

    let retained_settings = ordered_detached_element_settings_winners(preservation, &definitions)?;
    if retained_settings.len() != definitions.len()
        || retained_settings
            .iter()
            .zip(definitions.iter())
            .any(|(node, definition)| node.source_type.as_deref() != Some(definition.key.as_str()))
    {
        return Err(FdxError::InvalidValue);
    }

    Ok(ElementTemplateSettings {
        definitions,
        retained_settings,
    })
}

