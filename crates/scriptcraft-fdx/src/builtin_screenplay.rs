//! Screenplay-derived required-base definitions used to complete imported registries.

use crate::constants::FDX_ROOT;
use crate::diagnostics::FdxError;
use crate::element_settings::{read_element_settings, ElementSettingsDraft};
use crate::reader::{attribute, parse_document, RawElement, RawNode};
use scriptcraft_model::element_definition::ElementDefinition;
use scriptcraft_model::element_type::ElementTypeKey;
use std::sync::OnceLock;

/// The only required-base labels and their canonical append order.
pub(crate) const REQUIRED_BASE_LABELS: [&str; 9] = [
    "General",
    "Scene Heading",
    "Action",
    "Character",
    "Parenthetical",
    "Dialogue",
    "Transition",
    "Shot",
    "Cast List",
];

/// Immutable, source-derived values for exactly the required Screenplay bases.
#[derive(Debug, Clone)]
pub(crate) struct ScreenplayDefaults {
    definitions: Vec<ElementDefinition>,
}

impl ScreenplayDefaults {
    /// Returns one independent owned definition for an exact required label.
    pub(crate) fn definition(&self, label: &str) -> Option<ElementDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.key.as_str() == label)
            .cloned()
    }

    /// Returns an owned General baseline for incomplete arbitrary definitions.
    pub(crate) fn general(&self) -> ElementDefinition {
        // The provider constructor enforces the fixed label order and cardinality.
        self.definitions[0].clone()
    }

    /// Returns independent owned values in required-base order.
    pub(crate) fn definitions(&self) -> Vec<ElementDefinition> {
        self.definitions.clone()
    }
}

/// Loads the embedded resource once and returns an owned clone per caller.
pub(crate) fn screenplay_defaults() -> Result<ScreenplayDefaults, FdxError> {
    static DEFAULTS: OnceLock<Result<ScreenplayDefaults, FdxError>> = OnceLock::new();
    DEFAULTS.get_or_init(parse_screenplay_defaults).clone()
}

fn parse_screenplay_defaults() -> Result<ScreenplayDefaults, FdxError> {
    let screenplay_bytes = crate::builtin_templates::screenplay_bytes();
    if screenplay_bytes.len() > crate::MAX_SOURCE_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    let document = parse_document(screenplay_bytes)?;
    let roots = document
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element) => Some(element),
            _ => None,
        })
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err(FdxError::InvalidRoot);
    }
    let root = roots[0];
    if root.name.local_name != FDX_ROOT || attribute(root, "DocumentType") != Some("Script") {
        return Err(FdxError::InvalidRoot);
    }

    let root_namespace = root.name.namespace_uri.as_deref();
    for label in REQUIRED_BASE_LABELS {
        if direct_settings_count(root, root_namespace, label) != 1 {
            return Err(FdxError::InvalidValue);
        }
    }

    let imported = read_element_settings(root, root_namespace)?;
    let mut definitions = Vec::with_capacity(REQUIRED_BASE_LABELS.len());
    for label in REQUIRED_BASE_LABELS {
        let draft = imported
            .winners
            .iter()
            .find(|draft| draft.key.as_str() == label)
            .ok_or(FdxError::InvalidValue)?;
        definitions.push(complete_packaged_draft(draft)?);
    }

    Ok(ScreenplayDefaults { definitions })
}

fn complete_packaged_draft(draft: &ElementSettingsDraft) -> Result<ElementDefinition, FdxError> {
    draft.complete_packaged_definition()
}

fn direct_settings_count(root: &RawElement, root_namespace: Option<&str>, label: &str) -> usize {
    root.children
        .iter()
        .filter(|node| match node {
            RawNode::Element(element) => {
                element.name.local_name == "ElementSettings"
                    && element.name.namespace_uri.as_deref() == root_namespace
                    && attribute(element, "Type") == Some(label)
            }
            _ => false,
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded_defaults() -> ScreenplayDefaults {
        match screenplay_defaults() {
            Ok(defaults) => defaults,
            Err(_) => {
                assert!(false, "the embedded Screenplay resource must be valid");
                ScreenplayDefaults {
                    definitions: Vec::new(),
                }
            }
        }
    }

    #[test]
    fn exposes_only_complete_required_bases_in_canonical_order() {
        let defaults = loaded_defaults();
        let definitions = defaults.definitions();

        assert_eq!(
            definitions
                .iter()
                .map(|definition| definition.key.as_str())
                .collect::<Vec<_>>(),
            REQUIRED_BASE_LABELS.to_vec()
        );
        assert_eq!(definitions.len(), REQUIRED_BASE_LABELS.len());
        assert!(definitions
            .iter()
            .all(|definition| definition.validate().is_ok()));
        assert!(defaults.definition("Summary").is_none());
    }

    #[test]
    fn required_values_come_from_packaged_behavior_and_outline_fields() {
        let defaults = loaded_defaults();
        let character = defaults.definition("Character");
        assert_eq!(
            character
                .as_ref()
                .and_then(|definition| definition.behavior.return_key.as_ref())
                .map(ElementTypeKey::as_str),
            Some("Dialogue")
        );
        assert_eq!(
            character
                .as_ref()
                .and_then(|definition| definition.behavior.tab_key.as_ref())
                .map(ElementTypeKey::as_str),
            Some("Transition")
        );
        assert_eq!(
            character.as_ref().map(|definition| definition.outline),
            Some(scriptcraft_model::element_definition::ElementOutline {
                can_hide: false,
                level: 1,
            })
        );

        let general = defaults.general();
        assert_eq!(general.key.as_str(), "General");
        assert_eq!(general.behavior.paginate_as.as_deref(), Some("General"));
        assert_eq!(
            general
                .behavior
                .return_key
                .as_ref()
                .map(ElementTypeKey::as_str),
            Some("General")
        );
    }

    #[test]
    fn callers_receive_independent_owned_clones() {
        let defaults = loaded_defaults();
        let mut first = defaults.definition("General");
        if let Some(definition) = &mut first {
            definition.font.family.clear();
        }

        let second = defaults.definition("General");
        assert_eq!(
            second
                .as_ref()
                .map(|definition| definition.font.family.as_str()),
            Some("Courier Final Draft")
        );
        assert_eq!(
            first.map(|definition| definition.font.family),
            Some(String::new())
        );
    }
}
