//! Transactional replacement of document definitions and explicit type keys.

use super::derived::PendingDerivedInvalidation;
use super::history::Snapshot;
use super::selection::{validate_selection, validate_title_selection};
use super::{validate_full_candidate, EditError, EditingSession};
use scriptcraft_fdx::{reconcile_project_element_settings, replace_project_element_settings};
use scriptcraft_model::preservation::PreservedElement;
use scriptcraft_model::{ElementDefinition, ElementTypeKey, Project};
use std::collections::{BTreeMap, BTreeSet};

/// One explicit exact-key type mapping in a definition replacement request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementTypeRemapping {
    pub from: ElementTypeKey,
    pub to: ElementTypeKey,
}

impl ElementTypeRemapping {
    pub fn new(from: ElementTypeKey, to: ElementTypeKey) -> Self {
        Self { from, to }
    }
}

/// Complete generic request for replacing the ordered definition registry and
/// explicitly remapping managed body and TitlePage Content types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementDefinitionCandidate {
    pub definitions: Vec<ElementDefinition>,
    pub body_remappings: Vec<ElementTypeRemapping>,
    pub title_remappings: Vec<ElementTypeRemapping>,
    /// Optional ordered direct-root ElementSettings nodes, reserved for a
    /// caller that intentionally supplies a retained source projection.
    pub retained_settings_replacement: Option<Vec<PreservedElement>>,
}

impl ElementDefinitionCandidate {
    pub fn new(definitions: Vec<ElementDefinition>) -> Self {
        Self {
            definitions,
            body_remappings: Vec::new(),
            title_remappings: Vec::new(),
            retained_settings_replacement: None,
        }
    }
}

impl EditingSession {
    /// Replaces the complete ordered definition registry and applies all
    /// explicit managed body/title type mappings as one undoable transaction.
    /// Only semantic body owners and TitlePage Content paragraphs participate;
    /// out-of-Content title source remains opaque and untouched.
    pub fn replace_element_definitions_and_remap_types(
        &mut self,
        candidate: ElementDefinitionCandidate,
    ) -> Result<(), EditError> {
        let mut definition_project = self.project.clone();
        definition_project.replace_element_definitions(candidate.definitions.clone())?;
        let target_keys = definition_project
            .element_definitions()
            .iter()
            .map(|definition| definition.key.clone())
            .collect::<BTreeSet<_>>();

        let body_keys = self
            .project
            .elements
            .iter()
            .map(|owner| owner.element_type())
            .collect::<BTreeSet<_>>();
        let title_keys = self
            .project
            .title_page
            .iter()
            .filter_map(|paragraph| paragraph.element_type())
            .collect::<BTreeSet<_>>();
        let body_remappings = validate_remappings(
            &candidate.body_remappings,
            &body_keys,
            &target_keys,
            &self.project,
        )?;
        let title_remappings = validate_remappings(
            &candidate.title_remappings,
            &title_keys,
            &target_keys,
            &self.project,
        )?;
        validate_used_keys(&body_keys, &body_remappings, &target_keys)?;
        validate_used_keys(&title_keys, &title_remappings, &target_keys)?;

        if candidate.retained_settings_replacement.is_some() && self.project.preservation.is_none()
        {
            return Err(EditError::InvalidDefinitionCandidate);
        }

        let mut project = self.project.clone();
        for index in 0..project.elements.len() {
            let old_type = project.elements[index].element_type();
            let Some(new_type) = body_remappings.get(&old_type) else {
                continue;
            };
            if &old_type == new_type {
                continue;
            }
            super::structure::update_preserved_type(&mut project, index, new_type)?;
            scriptcraft_model::BodyOwner::update(&mut project.elements[index], |element| {
                element.set_element_type(new_type.clone());
            });
        }

        for index in 0..project.title_page.len() {
            let Some(old_type) = project.title_page[index].element_type() else {
                continue;
            };
            let Some(new_type) = title_remappings.get(&old_type) else {
                continue;
            };
            if &old_type == new_type {
                continue;
            }
            let semantic = project.title_page[index].clone();
            if semantic.source_provenance.is_some() {
                project
                    .preservation
                    .as_mut()
                    .ok_or(EditError::InvalidDefinitionCandidate)?
                    .change_managed_title_element_type(&semantic, Some(&old_type), Some(new_type))
                    .map_err(EditError::Preservation)?;
            }
            let paragraph = &mut project.title_page[index];
            paragraph.set_element_type(Some(new_type.clone()));
            if let Some(provenance) = &mut paragraph.source_provenance {
                provenance.foreign_role = Some(new_type.as_str().to_owned());
            }
        }

        project.replace_element_definitions(candidate.definitions)?;
        match candidate.retained_settings_replacement {
            Some(nodes) => {
                let definitions = project.element_definitions().clone();
                let preservation = project
                    .preservation
                    .as_mut()
                    .ok_or(EditError::InvalidDefinitionCandidate)?;
                replace_project_element_settings(preservation, &definitions, nodes)
                    .map_err(|_| EditError::InvalidDefinitionCandidate)?;
            }
            None if project.preservation.is_some() => {
                reconcile_project_element_settings(&mut project)
                    .map_err(|_| EditError::InvalidDefinitionCandidate)?;
            }
            None => {}
        }

        if let Some(preservation) = project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        validate_full_candidate(&project)?;
        validate_selection(&project, self.selection)?;
        validate_title_selection(&project, self.title_selection)?;

        let mut generated = self.generated_parentheticals.clone();
        generated.retain(|id| {
            project.elements.iter().any(|owner| {
                owner.id.as_str() == id && owner.element_type().as_str() == "Parenthetical"
            })
        });
        if project == self.project && generated == self.generated_parentheticals {
            return Ok(());
        }

        self.push_history(Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            beat_selection: self.beat_selection.clone(),
            beat_typing_style: self.beat_typing_style,
            beat_typing_formatting: self.beat_typing_formatting,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            body_typing_formatting: self.body_typing_formatting,
            generated_parentheticals: self.generated_parentheticals.clone(),
        });
        self.project = project;
        self.generated_parentheticals = generated;
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.pending_derived_invalidation = PendingDerivedInvalidation::Complete;
        self.deferred_derived_scenes.clear();
        self.failed_derived_complete = false;
        self.failed_derived_scenes.clear();
        self.mark_mutated();
        Ok(())
    }
}

fn validate_remappings(
    remappings: &[ElementTypeRemapping],
    used_keys: &BTreeSet<ElementTypeKey>,
    target_keys: &BTreeSet<ElementTypeKey>,
    current: &Project,
) -> Result<BTreeMap<ElementTypeKey, ElementTypeKey>, EditError> {
    let mut validated = BTreeMap::new();
    for mapping in remappings {
        if !used_keys.contains(&mapping.from)
            || current.element_definitions().get(&mapping.from).is_none()
            || !target_keys.contains(&mapping.to)
            || validated
                .insert(mapping.from.clone(), mapping.to.clone())
                .is_some()
        {
            return Err(EditError::InvalidDefinitionCandidate);
        }
    }
    Ok(validated)
}

fn validate_used_keys(
    used_keys: &BTreeSet<ElementTypeKey>,
    remappings: &BTreeMap<ElementTypeKey, ElementTypeKey>,
    target_keys: &BTreeSet<ElementTypeKey>,
) -> Result<(), EditError> {
    for key in used_keys {
        if !remappings.contains_key(key) && !target_keys.contains(key) {
            return Err(EditError::InvalidDefinitionCandidate);
        }
    }
    Ok(())
}
