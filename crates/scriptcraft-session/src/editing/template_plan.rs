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
