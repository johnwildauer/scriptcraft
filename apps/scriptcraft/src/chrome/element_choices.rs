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

#[cfg(test)]
mod tests {
    use super::*;
    const DYNAMIC_CONTROLS_FDX: &[u8] = include_bytes!("../../tests/fixtures/dynamic_controls.fdx");
    use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
    use scriptcraft_model::{
        ElementBehavior, ElementDefinition, ElementFontSpec, ElementOutline, ElementParagraphSpec,
    };

    fn definition(label: &str) -> Result<ElementDefinition, scriptcraft_model::ModelError> {
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
                left_indent: FixedPoint::from_points(0),
                right_indent: FixedPoint::from_points(0),
                leading: "Regular".to_owned(),
                space_before: FixedPoint::from_points(0),
                spacing: 1,
                starts_new_page: false,
            },
            behavior: ElementBehavior {
                paginate_as: None,
                return_key: None,
                shortcut: None,
                tab_key: None,
                win_shortcut: None,
            },
            outline: ElementOutline {
                can_hide: false,
                level: 1,
            },
        })
    }

    fn registry(
        labels: &[&str],
    ) -> Result<DocumentElementDefinitions, scriptcraft_model::ModelError> {
        let definitions = labels
            .iter()
            .map(|label| definition(label))
            .collect::<Result<Vec<_>, _>>()?;
        DocumentElementDefinitions::new(definitions)
    }

    #[test]
    fn choices_keep_arbitrary_registry_order_and_exact_case_distinct_labels() {
        let Ok(definitions) = registry(&["Lyrics", "action", "Notations", "Action"]) else {
            assert!(false, "test registry should be valid");
            return;
        };

        let choices = ordered_element_choices(&definitions);

        assert_eq!(
            choices.iter().map(ElementChoice::label).collect::<Vec<_>>(),
            vec!["Lyrics", "action", "Notations", "Action"]
        );
        assert_eq!(choices[1].key.as_str(), "action");
    }

    #[test]
    fn recomputing_choices_tracks_a_replaced_ordered_registry() {
        let Ok(initial) = registry(&["Scene Heading", "Lyrics"]) else {
            assert!(false, "initial test registry should be valid");
            return;
        };
        let Ok(replacement) = registry(&["Notations", "Action", "lyrics"]) else {
            assert!(false, "replacement test registry should be valid");
            return;
        };

        let previous_snapshot = ordered_element_choices(&initial);
        let current_snapshot = ordered_element_choices(&replacement);

        assert_eq!(
            previous_snapshot
                .iter()
                .map(ElementChoice::label)
                .collect::<Vec<_>>(),
            vec!["Scene Heading", "Lyrics"]
        );
        assert_eq!(
            current_snapshot
                .iter()
                .map(ElementChoice::label)
                .collect::<Vec<_>>(),
            vec!["Notations", "Action", "lyrics"]
        );
    }

    #[test]
    fn sanitized_dynamic_fixture_projects_every_definition_in_source_order() {
        use scriptcraft_model::dual_dialogue::active_dual_dialogue_projection;

        let project = scriptcraft_fdx::read_v6(DYNAMIC_CONTROLS_FDX)
            .unwrap_or_else(|_| unreachable!("checked-in dynamic controls fixture is valid"))
            .project;
        let choices = ordered_element_choices(project.element_definitions());
        let labels = choices.iter().map(ElementChoice::label).collect::<Vec<_>>();

        assert_eq!(
            labels[0..5],
            ["Notations", "Scene Heading", "Action", "Lyrics", "lyrics"]
        );
        assert!(labels.contains(&"Punctuation"));
        assert!(labels.contains(&"Conflict Three"));
        for required_base in [
            "Scene Heading",
            "Action",
            "Character",
            "Dialogue",
            "Parenthetical",
            "Transition",
            "General",
            "Shot",
            "Cast List",
        ] {
            assert!(
                labels.contains(&required_base),
                "missing base {required_base}"
            );
        }
        assert_eq!(project.element_definitions().len(), labels.len());
        assert_eq!(project.elements[0].element_type().as_str(), "Notations");
        assert_eq!(project.elements[0].text, "Multi run — café");
        assert_eq!(project.elements[0].runs.len(), 2);
        assert_eq!(
            project.elements[0]
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref()),
            Some("controls-body-foreign")
        );
        assert!(project
            .direct_paragraph_overrides
            .contains_key(&project.elements[0].id));
        assert_eq!(project.dual_dialogue_groups.len(), 2);
        assert!(project
            .dual_dialogue_groups
            .iter()
            .any(|group| { active_dual_dialogue_projection(&project, group).is_some() }));
        assert!(project
            .dual_dialogue_groups
            .iter()
            .any(|group| { active_dual_dialogue_projection(&project, group).is_none() }));

        let written = scriptcraft_fdx::write_v6(&project)
            .unwrap_or_else(|_| unreachable!("fixture projection can be written as native FDX"));
        let written = String::from_utf8(written)
            .unwrap_or_else(|_| unreachable!("native FDX output is UTF-8"));
        for preserved_source in [
            "BodyAttachment",
            "BodyOpaque",
            "TitleAttachment",
            "TitleOpaque",
            "DefinitionPayload",
            "controls-body-foreign",
        ] {
            assert!(written.contains(preserved_source));
        }
    }
}
