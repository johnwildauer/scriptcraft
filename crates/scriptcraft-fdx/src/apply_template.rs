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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instantiate_builtin_template, BuiltinTemplateId};
    use scriptcraft_model::preservation::{PreservedNode, SourceOwnership};

    const SYNTHETIC_TARGET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<FinalDraft DocumentType="Script" Version="6">
  <ElementSettings Type="ExactCase" vendor-attribute="winner" xmlns:v="urn:template-test">
    <FontSpec Font="Courier Final Draft" Size="12" Style="Regular" />
    <ParagraphSpec Alignment="Left" FirstIndent="0" Leading="Regular" LeftIndent="0" RightIndent="0" SpaceBefore="0" Spacing="0" StartsNewPage="No" Type="ExactCase" />
    <Behavior PaginateAs="Action" ReturnKey="Action" TabKey="Action" Shortcut="" WinShortcut="" />
    <Outline CanHide="No" Level="1" />
    <!-- retained winner comment -->
    <?vendor keep-winner-pi?>
    <v:Opaque marker="winner-payload">winner opaque text</v:Opaque>
  </ElementSettings>
  <ElementSettings Type="exactcase"><FontSpec Font="Times New Roman" Size="12" Style="Regular" /></ElementSettings>
  <ElementSettings Type="ExactCase" xmlns:v="urn:template-test"><v:Opaque marker="inert-duplicate">duplicate payload</v:Opaque></ElementSettings>
  <PageLayout marker="target-only-page-layout" />
  <HeaderAndFooter marker="target-only-header-footer" />
  <ContinueNumbering marker="target-only-continuation" />
  <TitlePage><Content><Paragraph Type="TitleOnly" id="target-title"><Text>target-only-title</Text></Paragraph></Content></TitlePage>
  <Content><Paragraph Type="FirstUse" id="target-body"><Text>target-only-body</Text></Paragraph></Content>
</FinalDraft>"#;

    fn collect_elements(node: &PreservedNode, output: &mut Vec<PreservedElement>) {
        if let PreservedNode::Element(element) = node {
            output.push(element.clone());
            for child in &element.children {
                collect_elements(child, output);
            }
        }
    }

    fn all_elements(settings: &[PreservedElement]) -> Vec<PreservedElement> {
        let mut output = Vec::new();
        for setting in settings {
            output.push(setting.clone());
            for child in &setting.children {
                collect_elements(child, &mut output);
            }
        }
        output
    }

    fn processing_instructions(settings: &[PreservedElement]) -> Vec<(String, String)> {
        settings
            .iter()
            .flat_map(|setting| setting.children.iter())
            .filter_map(|node| match node {
                PreservedNode::ProcessingInstruction { target, data } => {
                    Some((target.to_string(), data.to_string()))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn builtins_extract_only_normalized_ordered_definitions_and_settings_winners() {
        for id in [
            BuiltinTemplateId::Screenplay,
            BuiltinTemplateId::Stageplay,
            BuiltinTemplateId::DramatistsGuildMusical,
        ] {
            let Ok(document) = instantiate_builtin_template(id) else {
                assert!(false, "built-in common load failed");
                return;
            };
            let original = document.clone();
            let Ok(extraction) = extract_element_template_settings(&document) else {
                assert!(false, "built-in extraction failed");
                return;
            };
            let keys = extraction
                .definitions()
                .iter()
                .map(|definition| definition.key.as_str())
                .collect::<Vec<_>>();
            assert_eq!(keys.len(), extraction.retained_settings().len());
            assert_eq!(
                extraction
                    .retained_settings()
                    .iter()
                    .map(|node| node.source_type.as_deref())
                    .collect::<Vec<_>>(),
                keys.iter().copied().map(Some).collect::<Vec<_>>()
            );
            assert!(keys.contains(&"Action"));
            assert!(extraction
                .retained_settings()
                .iter()
                .all(|node| node.anchor.is_none() && node.ownership == SourceOwnership::Opaque));
            assert_eq!(document, original);
        }
    }

    #[test]
    fn processing_instruction_separator_is_canonical_and_roundtrips_stably() {
        let input = SYNTHETIC_TARGET.replace(
            "    <?vendor keep-winner-pi?>",
            "    <?ordinary data?>\n    <?repeated   data \t?>\n    <?empty?>",
        );
        let Ok(document) = crate::read_v6(input.as_bytes()) else {
            assert!(false, "synthetic common load failed");
            return;
        };
        let Ok(extraction) = extract_element_template_settings(&document) else {
            assert!(false, "synthetic extraction failed");
            return;
        };
        let expected = vec![
            ("ordinary".to_owned(), "data".to_owned()),
            ("repeated".to_owned(), "  data \t".to_owned()),
            ("empty".to_owned(), String::new()),
        ];
        assert_eq!(
            processing_instructions(extraction.retained_settings()),
            expected
        );

        let Ok(first_output) = crate::writer::write_v6(&document.project) else {
            assert!(false, "PI document serialization failed");
            return;
        };
        let Ok(first_reread) = crate::read_v6(&first_output) else {
            assert!(false, "serialized PI document reread failed");
            return;
        };
        let Ok(first_extraction) = extract_element_template_settings(&first_reread) else {
            assert!(false, "serialized PI extraction failed");
            return;
        };
        assert_eq!(
            processing_instructions(first_extraction.retained_settings()),
            expected
        );

        let Ok(second_output) = crate::writer::write_v6(&first_reread.project) else {
            assert!(false, "second PI document serialization failed");
            return;
        };
        let Ok(second_reread) = crate::read_v6(&second_output) else {
            assert!(false, "second serialized PI document reread failed");
            return;
        };
        let Ok(second_extraction) = extract_element_template_settings(&second_reread) else {
            assert!(false, "second serialized PI extraction failed");
            return;
        };
        assert_eq!(
            processing_instructions(second_extraction.retained_settings()),
            expected
        );
    }

    #[test]
    fn extraction_materializes_first_use_definitions_and_retains_only_ordered_winners() {
        let Ok(document) = crate::read_v6(SYNTHETIC_TARGET.as_bytes()) else {
            assert!(false, "synthetic common load failed");
            return;
        };
        let before = document.clone();
        let Ok(extraction) = extract_element_template_settings(&document) else {
            assert!(false, "synthetic extraction failed");
            return;
        };

        let keys = extraction
            .definitions()
            .iter()
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(keys.first(), Some(&"ExactCase"));
        assert_eq!(keys.get(1), Some(&"exactcase"));
        assert!(keys.contains(&"FirstUse"));
        assert!(keys.contains(&"Action"));
        assert_eq!(keys.len(), extraction.retained_settings().len());
        assert_eq!(
            extraction
                .retained_settings()
                .iter()
                .map(|node| node.source_type.as_deref())
                .collect::<Vec<_>>(),
            keys.iter().copied().map(Some).collect::<Vec<_>>()
        );

        let winner = extraction
            .retained_settings()
            .iter()
            .find(|node| node.source_type.as_deref() == Some("ExactCase"));
        let Some(winner) = winner else {
            assert!(false, "the exact first winner must be retained");
            return;
        };
        assert!(winner.anchor.is_none());
        assert!(winner.attributes.iter().any(|attribute| {
            attribute.name.local_name == "vendor-attribute" && attribute.value == "winner"
        }));
        assert!(winner.namespace_bindings.iter().any(|binding| {
            binding.prefix == "v" && binding.namespace_uri == "urn:template-test"
        }));
        assert!(winner.children.iter().any(|node| matches!(
            node,
            PreservedNode::Comment(value) if value.as_str() == " retained winner comment "
        )));
        assert!(winner.children.iter().any(|node| matches!(
            node,
            PreservedNode::ProcessingInstruction { target, data }
                if target.as_str() == "vendor" && data.as_str() == "keep-winner-pi"
        )));
        assert!(winner.children.iter().any(|node| matches!(
            node,
            PreservedNode::Element(element)
                if element.name.local_name == "Opaque"
                    && element.attributes.iter().any(|attribute| {
                        attribute.name.local_name == "marker" && attribute.value == "winner-payload"
                    })
        )));

        let flattened = all_elements(extraction.retained_settings());
        assert!(flattened.iter().all(|element| element.anchor.is_none()));
        assert!(flattened
            .iter()
            .all(|element| element.ownership == SourceOwnership::Opaque));
        let retained_debug = format!(
            "{:?}{:?}",
            extraction.definitions(),
            extraction.retained_settings()
        );
        for excluded in [
            "target-only-page-layout",
            "target-only-header-footer",
            "target-only-continuation",
            "target-title",
            "target-only-title",
            "target-body",
            "target-only-body",
            "inert-duplicate",
            "duplicate payload",
        ] {
            assert!(!retained_debug.contains(excluded));
        }

        let Some(original_preservation) = before.project.preservation.as_ref() else {
            assert!(false, "common-loaded target must retain its source");
            return;
        };
        assert!(original_preservation.anchors.iter().any(|anchor| {
            matches!(
                &anchor.target,
                scriptcraft_model::preservation::AnchorTarget::ElementDefinition(key)
                    if key.as_str() == "ExactCase"
            )
        }));
        assert_eq!(document, before);
    }

    #[test]
    fn extraction_rejects_a_source_less_document_without_partial_output() {
        let Ok(mut document) = instantiate_builtin_template(BuiltinTemplateId::Screenplay) else {
            assert!(false, "built-in common load failed");
            return;
        };
        document.project.preservation = None;
        assert_eq!(
            extract_element_template_settings(&document),
            Err(FdxError::InvalidValue)
        );
    }
}
