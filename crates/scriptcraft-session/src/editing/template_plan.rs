//! Pure exact-key and Action-fallback planning for Apply Template.

use super::definitions::{ElementDefinitionCandidate, ElementTypeRemapping};
use super::EditError;
use scriptcraft_model::{DocumentElementDefinitions, ElementTypeKey, Project};
use std::collections::BTreeSet;

/// Builds the generic Phase 3 definition-remap request for a normalized target
/// registry. The planner reads only the current semantic keys/registries and
/// never loads a template, traverses retained source, or publishes a candidate.
pub(crate) fn plan_element_template_application(
    current: &Project,
    target: &DocumentElementDefinitions,
) -> Result<ElementDefinitionCandidate, EditError> {
    current
        .element_definitions()
        .validate()
        .map_err(|_| EditError::InvalidDefinitionCandidate)?;
    target
        .validate()
        .map_err(|_| EditError::InvalidDefinitionCandidate)?;

    let action_keys = target
        .iter()
        .filter(|definition| definition.key.as_str() == "Action")
        .map(|definition| definition.key.clone())
        .collect::<Vec<_>>();
    let [action_key] = action_keys.as_slice() else {
        return Err(EditError::InvalidDefinitionCandidate);
    };

    let body_remappings = plan_surface_remappings(
        current.elements.iter().map(|owner| owner.element_type()),
        current.element_definitions(),
        target,
        action_key,
    )?;
    let title_remappings = plan_surface_remappings(
        current
            .title_page
            .iter()
            .filter_map(|paragraph| paragraph.element_type()),
        current.element_definitions(),
        target,
        action_key,
    )?;

    Ok(ElementDefinitionCandidate {
        definitions: target.iter().cloned().collect(),
        body_remappings,
        title_remappings,
        retained_settings_replacement: None,
    })
}

fn plan_surface_remappings(
    current_keys: impl IntoIterator<Item = ElementTypeKey>,
    current_definitions: &DocumentElementDefinitions,
    target_definitions: &DocumentElementDefinitions,
    action_key: &ElementTypeKey,
) -> Result<Vec<ElementTypeRemapping>, EditError> {
    let mut seen_unmatched = BTreeSet::new();
    let mut remappings = Vec::new();

    for current_key in current_keys {
        if current_definitions.get(&current_key).is_none() {
            return Err(EditError::InvalidDefinitionCandidate);
        }
        if target_definitions.get(&current_key).is_some() {
            continue;
        }

        // Phase 3 represents remapping by exact source key, so one explicit
        // mapping covers every current owner sharing that key on this surface.
        if seen_unmatched.insert(current_key.clone()) {
            if target_definitions.get(action_key).is_none() {
                return Err(EditError::InvalidDefinitionCandidate);
            }
            remappings.push(ElementTypeRemapping::new(current_key, action_key.clone()));
        }
    }

    Ok(remappings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptcraft_model::{
        dual_dialogue::{DualDialogueGroup, DualDialogueWrapperProvenance},
        element::BodyOwner,
        identity::StableId,
        title_page::{TitlePageParagraph, TitlePageParagraphFormatting},
        DocumentElementDefinitions, ScreenplayElement,
    };

    fn definitions_from_builtin(
        id: scriptcraft_fdx::BuiltinTemplateId,
    ) -> Option<DocumentElementDefinitions> {
        let document = scriptcraft_fdx::instantiate_builtin_template(id).ok()?;
        Some(document.project.element_definitions().clone())
    }

    fn planning_project() -> Option<(Project, DocumentElementDefinitions)> {
        let mut definitions =
            definitions_from_builtin(scriptcraft_fdx::BuiltinTemplateId::DramatistsGuildMusical)?
                .iter()
                .cloned()
                .collect::<Vec<_>>();

        for (source, label) in [
            ("Action", "action"),
            ("Action", "lyrics"),
            ("Action", "TitleOnly"),
        ] {
            let mut definition = definitions
                .iter()
                .find(|definition| definition.key.as_str() == source)?
                .clone();
            definition.key = ElementTypeKey::new(label).ok()?;
            definitions.push(definition);
        }

        let mut project = Project::empty_projection().ok()?;
        project.replace_element_definitions(definitions).ok()?;

        for (id, label, text) in [
            ("body-lyrics-a", "Lyrics", "lyrics body A"),
            ("body-lower-lyrics", "lyrics", "case-distinct body lyrics"),
            ("body-action", "Action", "exact action"),
            ("body-lower-action", "action", "case-distinct action"),
            ("body-heading", "Scene Heading", "INT. SOMEWHERE"),
            ("body-notation", "Notations", "notation member"),
            ("body-lyrics-b", "Lyrics", "lyrics body B"),
        ] {
            let key = ElementTypeKey::new(label).ok()?;
            let element = ScreenplayElement::new(id, key, text).ok()?;
            project.elements.push(BodyOwner::shared(element));
        }

        let first_member = project.elements.first()?.id.clone();
        let second_member = project.elements.get(1)?.id.clone();
        project.dual_dialogue_groups.push(DualDialogueGroup {
            id: StableId::new("planner-dual-group").ok()?,
            members: vec![first_member, second_member],
            wrapper: DualDialogueWrapperProvenance::default(),
        });

        for (id, label, text) in [
            ("title-lyrics", Some("Lyrics"), "typed lyrics title"),
            ("title-action", Some("Action"), "exact action title"),
            ("title-only", Some("TitleOnly"), "typed title-only key"),
            ("title-typeless", None, "type-less title"),
        ] {
            let mut paragraph =
                TitlePageParagraph::new(id, text, TitlePageParagraphFormatting::default()).ok()?;
            paragraph.element_type = match label {
                Some(label) => Some(ElementTypeKey::new(label).ok()?),
                None => None,
            };
            project.title_page.push(paragraph);
        }

        let target = definitions_from_builtin(scriptcraft_fdx::BuiltinTemplateId::Screenplay)?;
        Some((project, target))
    }

    fn remap_pairs(remappings: &[ElementTypeRemapping]) -> Vec<(&str, &str)> {
        remappings
            .iter()
            .map(|mapping| (mapping.from.as_str(), mapping.to.as_str()))
            .collect()
    }

    #[test]
    fn plans_exact_retention_and_surface_specific_action_fallback_without_mutation() {
        let Some((project, target)) = planning_project() else {
            assert!(false, "built-in planning fixture should load");
            return;
        };
        let before = project.clone();

        let planned = plan_element_template_application(&project, &target);
        let Ok(candidate) = planned else {
            assert!(false, "valid current and target registries should plan");
            return;
        };

        assert_eq!(
            candidate
                .definitions
                .iter()
                .map(|definition| definition.key.as_str())
                .collect::<Vec<_>>(),
            target
                .iter()
                .map(|definition| definition.key.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            remap_pairs(&candidate.body_remappings),
            vec![
                ("Lyrics", "Action"),
                ("lyrics", "Action"),
                ("action", "Action"),
                ("Notations", "Action"),
            ]
        );
        assert_eq!(
            remap_pairs(&candidate.title_remappings),
            vec![("Lyrics", "Action"), ("TitleOnly", "Action")]
        );
        assert_eq!(
            candidate
                .body_remappings
                .iter()
                .filter(|mapping| mapping.from.as_str() == "Lyrics")
                .count(),
            1,
            "the key-based Phase 3 request maps all owners with one exact old key"
        );
        assert!(candidate.body_remappings.iter().all(|mapping| {
            target.get(&mapping.to).is_some() && mapping.to.as_str() == "Action"
        }));
        assert!(candidate.title_remappings.iter().all(|mapping| {
            target.get(&mapping.to).is_some() && mapping.to.as_str() == "Action"
        }));
        assert!(candidate.retained_settings_replacement.is_none());
        assert_eq!(
            project, before,
            "planning must not mutate the current project"
        );
    }

    #[test]
    fn empty_body_and_title_projections_produce_no_remappings() {
        let Some((mut project, target)) = planning_project() else {
            assert!(false, "built-in planning fixture should load");
            return;
        };
        project.elements.clear();
        project.title_page.clear();
        project.dual_dialogue_groups.clear();

        let planned = plan_element_template_application(&project, &target);
        let Ok(candidate) = planned else {
            assert!(false, "empty semantic projections should plan");
            return;
        };
        assert!(candidate.body_remappings.is_empty());
        assert!(candidate.title_remappings.is_empty());
    }

    #[test]
    fn rejects_missing_target_action_and_unregistered_current_managed_keys() {
        let Some((mut project, target)) = planning_project() else {
            assert!(false, "built-in planning fixture should load");
            return;
        };

        let without_action = DocumentElementDefinitions::new(
            target
                .iter()
                .filter(|definition| definition.key.as_str() != "Action")
                .cloned(),
        );
        let Ok(without_action) = without_action else {
            assert!(
                false,
                "the target registry without Action remains otherwise valid"
            );
            return;
        };
        assert_eq!(
            plan_element_template_application(&project, &without_action),
            Err(EditError::InvalidDefinitionCandidate)
        );

        let missing_key = ElementTypeKey::new("CurrentOnlyUnregistered");
        let Ok(missing_key) = missing_key else {
            assert!(false, "synthetic key should be valid");
            return;
        };
        let element = ScreenplayElement::new("body-unregistered", missing_key, "opaque text");
        let Ok(element) = element else {
            assert!(false, "synthetic body owner should be valid");
            return;
        };
        project.elements.push(BodyOwner::shared(element));
        assert_eq!(
            plan_element_template_application(&project, &target),
            Err(EditError::InvalidDefinitionCandidate)
        );
    }
}
