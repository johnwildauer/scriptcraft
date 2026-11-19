//! Read-only semantic and Behavior projections over current document definitions.

use crate::{ElementDefinition, ElementTypeKey, SemanticCapabilities};

/// Selects one of the document-owned paragraph Behavior destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphBehaviorKey {
    Return,
    Tab,
}

/// Exact resolution state for a present or absent Behavior destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorTargetResolution {
    /// The destination exists in the current registry. Same-key targets remain resolved.
    Resolved(ElementTypeKey),
    /// The source definition has no destination for the selected Behavior key.
    Absent,
    /// The source definition names a destination that is not in the current registry.
    Unresolved(ElementTypeKey),
}

/// Immutable view of one current definition and its exact-key capabilities.
///
/// The view borrows the complete definition from the Project registry; it is
/// neither stored on Project nor serialized as independent semantic state.
#[derive(Debug, Clone, Copy)]
pub struct ElementSemantics<'a> {
    definition: &'a ElementDefinition,
}

impl<'a> ElementSemantics<'a> {
    pub(crate) fn new(definition: &'a ElementDefinition) -> Self {
        Self { definition }
    }

    /// Returns the exact key that selected this definition.
    pub fn key(&self) -> &'a ElementTypeKey {
        &self.definition.key
    }

    /// Returns the complete current document-owned definition, including raw shortcuts.
    pub fn definition(&self) -> &'a ElementDefinition {
        self.definition
    }

    /// Returns capabilities derived solely from this definition's exact key.
    pub fn capabilities(&self) -> SemanticCapabilities {
        self.definition.capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting::{FixedPoint, ParagraphAlignment};
    use crate::{
        ElementBehavior, ElementFontSpec, ElementOutline, ElementParagraphSpec, ModelError, Project,
    };

    fn definition(
        label: &str,
        return_key: Option<&str>,
        tab_key: Option<&str>,
    ) -> Result<ElementDefinition, ModelError> {
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
                return_key: return_key.map(ElementTypeKey::new).transpose()?,
                shortcut: Some("raw shortcut chord".to_owned()),
                tab_key: tab_key.map(ElementTypeKey::new).transpose()?,
                win_shortcut: Some(String::new()),
            },
            outline: ElementOutline {
                can_hide: false,
                level: 1,
            },
        })
    }

    fn project(definitions: Vec<ElementDefinition>) -> Result<Project, ModelError> {
        let mut project = Project::empty_projection()?;
        project.replace_element_definitions(definitions)?;
        Ok(project)
    }

    fn key(label: &str) -> Result<ElementTypeKey, ModelError> {
        ElementTypeKey::new(label)
    }

    #[test]
    fn semantic_view_uses_exact_key_and_exposes_complete_definition() {
        let definitions = [
            definition("Scene Heading", None, None),
            definition("scene heading", None, None),
            definition("Lyrics", None, None),
            definition("Shot", None, None),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>();
        let Ok(definitions) = definitions else {
            assert!(false, "test definitions should be valid");
            return;
        };
        let Ok(mut project) = project(definitions) else {
            assert!(false, "test project should be valid");
            return;
        };

        let Ok(scene_key) = key("Scene Heading") else {
            assert!(false, "base key should be valid");
            return;
        };
        let Some(scene) = project.element_semantics(&scene_key) else {
            assert!(false, "exact registered key should project");
            return;
        };
        assert_eq!(scene.key(), &scene_key);
        assert_eq!(
            scene.capabilities(),
            SemanticCapabilities {
                scene_boundary: true,
                ..SemanticCapabilities::default()
            }
        );
        assert_eq!(
            scene.definition().behavior.shortcut.as_deref(),
            Some("raw shortcut chord")
        );
        assert_eq!(
            scene.definition().behavior.win_shortcut.as_deref(),
            Some("")
        );

        for label in ["scene heading", "Lyrics", "Shot"] {
            let Ok(label_key) = key(label) else {
                assert!(false, "test key should be valid");
                return;
            };
            let Some(view) = project.element_semantics(&label_key) else {
                assert!(
                    false,
                    "registered arbitrary and case-distinct keys should project"
                );
                return;
            };
            assert_eq!(view.capabilities(), SemanticCapabilities::default());
        }

        let Ok(missing) = key("Missing") else {
            assert!(false, "test key should be valid");
            return;
        };
        assert!(project.element_semantics(&missing).is_none());
        assert!(project
            .resolve_paragraph_behavior_target(&missing, ParagraphBehaviorKey::Return)
            .is_none());
    }

    #[test]
    fn semantic_view_projects_each_exact_structural_base_capability() {
        let cases = [
            (
                "Scene Heading",
                SemanticCapabilities {
                    scene_boundary: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Character",
                SemanticCapabilities {
                    character_cue: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Dialogue",
                SemanticCapabilities {
                    dialogue: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Parenthetical",
                SemanticCapabilities {
                    parenthetical: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Transition",
                SemanticCapabilities {
                    transition: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Cast List",
                SemanticCapabilities {
                    cast_list: true,
                    ..SemanticCapabilities::default()
                },
            ),
        ];

        for (label, expected) in cases {
            let Ok(definition) = definition(label, None, None) else {
                assert!(false, "base definition should be valid");
                return;
            };
            let Ok(project) = project(vec![definition]) else {
                assert!(false, "base project should be valid");
                return;
            };
            let Ok(key) = key(label) else {
                assert!(false, "base key should be valid");
                return;
            };
            let Some(view) = project.element_semantics(&key) else {
                assert!(false, "exact base key should project");
                return;
            };
            assert_eq!(view.capabilities(), expected);
        }
    }

    #[test]
    fn behavior_resolution_distinguishes_absent_unresolved_and_exact_targets() {
        let definitions = [
            definition("Source", Some("Source"), Some("Paginated")),
            definition("Unresolved Source", Some("Later Type"), None),
            definition("Existing", None, None),
            definition("Paginated", None, None),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>();
        let Ok(mut definitions) = definitions else {
            assert!(false, "test definitions should be valid");
            return;
        };
        if let Some(paginated) = definitions
            .iter_mut()
            .find(|item| item.key.as_str() == "Paginated")
        {
            paginated.behavior.paginate_as = Some("Character".to_owned());
        }
        let Ok(project) = project(definitions) else {
            assert!(false, "test project should be valid");
            return;
        };

        let Ok(source) = key("Source") else {
            assert!(false, "test key should be valid");
            return;
        };
        assert_eq!(
            project.resolve_paragraph_behavior_target(&source, ParagraphBehaviorKey::Return),
            Some(BehaviorTargetResolution::Resolved(source.clone()))
        );
        assert_eq!(
            project.resolve_paragraph_behavior_target(&source, ParagraphBehaviorKey::Tab),
            key("Paginated")
                .ok()
                .map(BehaviorTargetResolution::Resolved)
        );

        let Ok(unresolved_source) = key("Unresolved Source") else {
            assert!(false, "test key should be valid");
            return;
        };
        assert_eq!(
            project.resolve_paragraph_behavior_target(
                &unresolved_source,
                ParagraphBehaviorKey::Return
            ),
            key("Later Type")
                .ok()
                .map(BehaviorTargetResolution::Unresolved)
        );

        let Ok(existing) = key("Existing") else {
            assert!(false, "test key should be valid");
            return;
        };
        assert_eq!(
            project.resolve_paragraph_behavior_target(&existing, ParagraphBehaviorKey::Return),
            Some(BehaviorTargetResolution::Absent)
        );
        assert_eq!(
            project.resolve_paragraph_behavior_target(&existing, ParagraphBehaviorKey::Tab),
            Some(BehaviorTargetResolution::Absent)
        );

        let Ok(paginated) = key("Paginated") else {
            assert!(false, "test key should be valid");
            return;
        };
        let Some(view) = project.element_semantics(&paginated) else {
            assert!(false, "registered Behavior target should project");
            return;
        };
        assert_eq!(view.capabilities(), SemanticCapabilities::default());
        assert_eq!(
            view.definition().behavior.paginate_as.as_deref(),
            Some("Character")
        );
    }

    #[test]
    fn exact_behavior_resolution_does_not_depend_on_registry_order() {
        let definitions = [
            definition("Source", Some("Target"), Some("target")),
            definition("Target", None, None),
            definition("target", None, None),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>();
        let Ok(definitions) = definitions else {
            assert!(false, "test definitions should be valid");
            return;
        };
        let Ok(mut original) = project(definitions) else {
            assert!(false, "test project should be valid");
            return;
        };
        let Ok(source) = key("Source") else {
            assert!(false, "test key should be valid");
            return;
        };

        let before_return =
            original.resolve_paragraph_behavior_target(&source, ParagraphBehaviorKey::Return);
        let before_tab =
            original.resolve_paragraph_behavior_target(&source, ParagraphBehaviorKey::Tab);
        let reversed = original
            .element_definitions()
            .iter()
            .cloned()
            .rev()
            .collect::<Vec<_>>();
        assert!(original.replace_element_definitions(reversed).is_ok());
        assert_eq!(
            original.resolve_paragraph_behavior_target(&source, ParagraphBehaviorKey::Return),
            before_return
        );
        assert_eq!(
            original.resolve_paragraph_behavior_target(&source, ParagraphBehaviorKey::Tab),
            before_tab
        );
        assert_eq!(
            before_return,
            key("Target").ok().map(BehaviorTargetResolution::Resolved)
        );
        assert_eq!(
            before_tab,
            key("target").ok().map(BehaviorTargetResolution::Resolved)
        );
    }
}
