use std::collections::BTreeSet;
use std::sync::Arc;

use crate::{
    dual_dialogue::{self, DualDialogueError},
    element::{
        ScreenplayElement, MAX_PROVENANCE_STRING_BYTES, MAX_RUNS_PER_ELEMENT, MAX_RUN_ATTRIBUTES,
        MAX_RUN_ATTRIBUTE_BYTES,
    },
    identity::StableId,
    project::Project,
    ModelError, MAX_TEXT_BYTES,
};

pub const MAX_ELEMENTS: usize = 10_000;

/// Validates canonical screenplay element text, runs, identity, and provenance.
pub(crate) fn validate_element(element: &ScreenplayElement) -> Result<(), ModelError> {
    StableId::new(element.id.as_str())?;
    if element.text.len() > MAX_TEXT_BYTES {
        return Err(ModelError::TextTooLong);
    }
    if element.runs.len() > MAX_RUNS_PER_ELEMENT {
        return Err(ModelError::TooManyRuns);
    }
    let mut run_text = String::new();
    let mut attribute_count = 0usize;
    for run in &element.runs {
        if run_text.len().saturating_add(run.text.len()) > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        run_text.push_str(&run.text);
        attribute_count = attribute_count.saturating_add(run.attributes.len());
        if run.attributes.iter().any(|attribute| {
            attribute.name.is_empty()
                || attribute.name.len() > MAX_RUN_ATTRIBUTE_BYTES
                || attribute.value.len() > MAX_RUN_ATTRIBUTE_BYTES
        }) {
            return Err(ModelError::StringTooLong);
        }
    }
    if attribute_count > MAX_RUN_ATTRIBUTES {
        return Err(ModelError::TooManyRunAttributes);
    }
    if !element.runs.is_empty() && run_text != element.text {
        return Err(ModelError::InvalidRunProjection);
    }
    if element.source_provenance.as_ref().is_some_and(|source| {
        source
            .foreign_id
            .as_ref()
            .is_some_and(|value| value.len() > MAX_PROVENANCE_STRING_BYTES)
            || source
                .foreign_role
                .as_ref()
                .is_some_and(|value| value.len() > MAX_PROVENANCE_STRING_BYTES)
    }) {
        return Err(ModelError::StringTooLong);
    }
    Ok(())
}

pub fn validate_layout_inputs(
    inputs: &crate::layout_inputs::LayoutInputs,
) -> Result<(), ModelError> {
    if inputs.name.len() > crate::layout_inputs::MAX_LAYOUT_NAME_BYTES
        || inputs.format_version == 0
        || inputs.paper_width == 0
        || inputs.paper_height == 0
        || inputs.chars_per_line == 0
        || inputs.lines_per_page == 0
        || inputs.line_advance == 0
        || inputs.line_advance > 1_000
        || inputs.boundary_gap > 1_000
        || !matches!(
            inputs.font_family.as_str(),
            "Courier" | "Courier New" | "Courier Prime"
        )
        || inputs.font_size == 0
        || inputs.font_size > 72
        || inputs.char_width == 0
        || inputs.char_width > 100
    {
        return Err(ModelError::InvalidNumber);
    }
    Ok(())
}

impl Project {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.elements.len() > MAX_ELEMENTS || self.title_page.len() > MAX_ELEMENTS {
            return Err(ModelError::TooManyElements);
        }
        self.element_definitions().validate()?;
        let mut ids = BTreeSet::new();
        let mut element_ids = BTreeSet::new();
        for item in &self.title_page {
            item.validate()?;
            if let Some(key) = item.element_type() {
                if self.element_definitions().get(&key).is_none() {
                    return Err(ModelError::MissingElementDefinition(key));
                }
            }
            if !ids.insert(&item.id) {
                return Err(ModelError::DuplicateId);
            }
            element_ids.insert(item.id.clone());
        }
        for item in &self.elements {
            validate_element(item)?;
            let key = item.element_type();
            if self.element_definitions().get(&key).is_none() {
                return Err(ModelError::MissingElementDefinition(key));
            }
            if !ids.insert(&item.id) {
                return Err(ModelError::DuplicateId);
            }
            element_ids.insert(item.id.clone());
        }
        dual_dialogue::validate_groups(self).map_err(ModelError::InvalidDualDialogue)?;
        if self
            .direct_paragraph_overrides
            .keys()
            .any(|id| !self.elements.iter().any(|element| &element.id == id))
        {
            return Err(ModelError::InvalidParagraphOverride);
        }
        let group_ids = self
            .dual_dialogue_groups
            .iter()
            .map(|group| group.id.clone())
            .collect::<BTreeSet<_>>();
        dual_dialogue::validate_wrapper_overrides(self).map_err(ModelError::InvalidDualDialogue)?;
        if let Some(preservation) = &self.preservation {
            preservation
                .validate_project_elements(&element_ids, &group_ids)
                .map_err(ModelError::InvalidPreservationState)?;
            let definition_keys = self
                .element_definitions()
                .iter()
                .map(|definition| definition.key.clone())
                .collect::<BTreeSet<_>>();
            preservation
                .validate_element_definition_anchors(&definition_keys)
                .map_err(ModelError::InvalidPreservationState)?;
            for group in &self.dual_dialogue_groups {
                let wrapper_anchor = preservation.dual_dialogue_wrapper_anchor(&group.id);
                if wrapper_anchor.as_ref() != group.wrapper.preservation_anchor.as_ref() {
                    return Err(ModelError::InvalidDualDialogue(
                        DualDialogueError::InvalidWrapperOwnership,
                    ));
                }
            }
        } else if self
            .dual_dialogue_groups
            .iter()
            .any(|group| group.wrapper.preservation_anchor.is_some())
        {
            return Err(ModelError::InvalidDualDialogue(
                DualDialogueError::InvalidWrapperOwnership,
            ));
        }
        Ok(())
    }

    /// Validates one ordinary-edit candidate against a complete valid baseline.
    /// `Ok(false)` means scope preconditions were not established and the caller
    /// must perform complete validation.
    pub fn validate_scoped_single_element_edit(
        &mut self,
        baseline: &Project,
        element_index: usize,
    ) -> Result<bool, ModelError> {
        if self.elements.len() != baseline.elements.len()
            || self.title_page.len() != baseline.title_page.len()
            || self.dual_dialogue_groups != baseline.dual_dialogue_groups
            || self.dual_dialogue_paragraph_overrides != baseline.dual_dialogue_paragraph_overrides
            || self.direct_paragraph_overrides != baseline.direct_paragraph_overrides
            || self.element_definitions() != baseline.element_definitions()
            || !Arc::ptr_eq(&self.layout_identity, &baseline.layout_identity)
        {
            return Ok(false);
        }
        let Some(before) = baseline.elements.get(element_index) else {
            return Ok(false);
        };
        let Some(after) = self.elements.get(element_index) else {
            return Ok(false);
        };
        if before.id != after.id {
            return Ok(false);
        }
        if self.title_page != baseline.title_page
            || self
                .elements
                .iter()
                .zip(&baseline.elements)
                .enumerate()
                .any(|(index, (candidate, original))| {
                    index != element_index
                        && !Arc::ptr_eq(candidate, original)
                        && candidate != original
                })
        {
            return Ok(false);
        }
        validate_element(after)?;
        let updated_source_text_bytes = match (&self.preservation, &baseline.preservation) {
            (Some(candidate), Some(valid_baseline)) => {
                if !candidate.has_managed_paragraph_index()
                    || !valid_baseline.has_managed_paragraph_index()
                    || !candidate.scoped_global_invariants_match(valid_baseline)
                {
                    return Ok(false);
                }
                Some(
                    candidate
                        .validate_scoped_managed_edit(valid_baseline, after)
                        .map_err(ModelError::InvalidPreservationState)?,
                )
            }
            (None, None) => None,
            _ => return Ok(false),
        };
        if let Some(byte_count) = updated_source_text_bytes {
            if let Some(preservation) = self.preservation.as_mut() {
                preservation.set_validated_source_text_bytes(byte_count);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registry_change_falls_back_from_scoped_validation() {
        let Ok(baseline) = crate::project::test_project() else {
            assert!(false, "test fixture should provide valid definitions");
            return;
        };
        let mut candidate = baseline.clone();
        let reordered = candidate
            .element_definitions()
            .iter()
            .cloned()
            .rev()
            .collect::<Vec<_>>();
        assert!(candidate.replace_element_definitions(reordered).is_ok());
        assert_eq!(
            candidate.validate_scoped_single_element_edit(&baseline, 0),
            Ok(false)
        );
    }
}
