//! Semantic project state shared by editing, native FDX preservation, and layout.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    dual_dialogue::DualDialogueGroup,
    element::{body_owners_equal, BodyOwner},
    element_definitions::DocumentElementDefinitions,
    element_semantics::{BehaviorTargetResolution, ElementSemantics, ParagraphBehaviorKey},
    formatting::{transitional_static_role_definitions, SourceParagraphOverrides},
    identity::{StableId, StableIdAllocator},
    preservation::PreservationState,
    title_page::{reference_boilerplate, TitlePageParagraph},
    ModelError, ScreenplayElement,
};

#[derive(Debug, Clone)]
pub struct Project {
    pub title_page: Vec<TitlePageParagraph>,
    pub elements: Vec<Arc<BodyOwner>>,
    /// Ordered semantic groups; members remain in `elements` in visible order.
    pub dual_dialogue_groups: Vec<DualDialogueGroup>,
    /// Direct body paragraph overrides not yet backed by imported FDX source.
    /// Source-backed projects mirror these values in preservation settings.
    pub direct_paragraph_overrides: BTreeMap<StableId, SourceParagraphOverrides>,
    /// Outer-wrapper direct fields, keyed only by semantic wrapper identities.
    pub dual_dialogue_paragraph_overrides: BTreeMap<StableId, SourceParagraphOverrides>,
    /// Source-owned FDX state is separate from the semantic projection.
    pub preservation: Option<PreservationState>,
    /// Ordered document-owned element settings. The backing registry is private
    /// and changes only through validated whole-registry replacement.
    element_definitions: DocumentElementDefinitions,
    /// Clone-stable identity for cache invalidation across document replacement.
    /// It is intentionally excluded from semantic equality and serialization.
    pub(crate) layout_identity: Arc<()>,
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.title_page == other.title_page
            && body_owners_equal(&self.elements, &other.elements)
            && self.dual_dialogue_groups == other.dual_dialogue_groups
            && self.direct_paragraph_overrides == other.direct_paragraph_overrides
            && self.dual_dialogue_paragraph_overrides == other.dual_dialogue_paragraph_overrides
            && self.preservation == other.preservation
            && self.element_definitions == other.element_definitions
    }
}

impl Eq for Project {}

impl Project {
    pub fn new() -> Result<Self, ModelError> {
        let mut ids = StableIdAllocator::new();
        let title_page = reference_boilerplate(&mut ids)?;
        // Transitional static-role bootstrap. Phase 7 replaces this with the
        // packaged Screenplay template; FDX import has its own Phase 2 handoff.
        let element_definitions = transitional_static_role_definitions()?;
        let start = ScreenplayElement::new(
            ids.allocate("element")?.as_str().to_owned(),
            crate::Role::SceneHeading,
            "",
        )?;
        Ok(Self {
            title_page,
            elements: vec![BodyOwner::shared(start)],
            dual_dialogue_groups: Vec::new(),
            direct_paragraph_overrides: BTreeMap::new(),
            dual_dialogue_paragraph_overrides: BTreeMap::new(),
            preservation: None,
            element_definitions,
            layout_identity: Arc::new(()),
        })
    }

    /// Creates the empty semantic projection required while importing FDX.
    /// It carries no title, body, group, override, or preservation state, but
    /// uses the transitional static bootstrap until the common FDX reader
    /// replaces it with WO-002.3's completed document-owned definitions.
    pub fn empty_projection() -> Result<Self, ModelError> {
        Ok(Self {
            title_page: Vec::new(),
            elements: Vec::new(),
            dual_dialogue_groups: Vec::new(),
            direct_paragraph_overrides: BTreeMap::new(),
            dual_dialogue_paragraph_overrides: BTreeMap::new(),
            preservation: None,
            element_definitions: transitional_static_role_definitions()?,
            layout_identity: Arc::new(()),
        })
    }

    /// Returns the project's ordered definition registry.
    pub fn element_definitions(&self) -> &DocumentElementDefinitions {
        &self.element_definitions
    }

    /// Returns the borrowed semantic view for an exact key in the current registry.
    pub fn element_semantics(&self, key: &crate::ElementTypeKey) -> Option<ElementSemantics<'_>> {
        self.element_definitions.get(key).map(ElementSemantics::new)
    }

    /// Resolves one owned Behavior target against this Project's current registry.
    ///
    /// An unknown source key returns `None`. For a known source, absent and
    /// unresolved target values remain distinct; no fallback is manufactured.
    pub fn resolve_paragraph_behavior_target(
        &self,
        source_key: &crate::ElementTypeKey,
        behavior_key: ParagraphBehaviorKey,
    ) -> Option<BehaviorTargetResolution> {
        let definition = self.element_definitions.get(source_key)?;
        let target = match behavior_key {
            ParagraphBehaviorKey::Return => definition.behavior.return_key.as_ref(),
            ParagraphBehaviorKey::Tab => definition.behavior.tab_key.as_ref(),
        };
        Some(match target {
            None => BehaviorTargetResolution::Absent,
            Some(target) if self.element_definitions.get(target).is_some() => {
                BehaviorTargetResolution::Resolved(target.clone())
            }
            Some(target) => BehaviorTargetResolution::Unresolved(target.clone()),
        })
    }

    /// Validates and atomically replaces the complete ordered registry.
    pub fn replace_element_definitions(
        &mut self,
        definitions: Vec<crate::ElementDefinition>,
    ) -> Result<(), ModelError> {
        let replacement = DocumentElementDefinitions::new(definitions)?;
        self.element_definitions = replacement;
        Ok(())
    }

    /// Allocates an application element ID distinct from title-page and body IDs.
    /// Foreign source IDs intentionally remain a separate namespace.
    pub fn allocate_element_id(&self) -> Result<StableId, ModelError> {
        let mut allocator = StableIdAllocator::new();
        for element in &self.title_page {
            allocator.reserve(element.id.clone())?;
        }
        for element in &self.elements {
            allocator.reserve(element.id.clone())?;
        }
        for group in &self.dual_dialogue_groups {
            allocator.reserve(group.id.clone())?;
        }
        allocator.allocate("element")
    }

    /// Reuses exact semantic owners after a validated persistence round trip.
    /// Changed identities/content retain their freshly decoded allocations.
    pub fn share_unchanged_body_owners_from(&mut self, previous: &Project) {
        let previous_by_id = previous
            .elements
            .iter()
            .map(|owner| (owner.id.clone(), owner))
            .collect::<BTreeMap<_, _>>();
        for owner in &mut self.elements {
            if let Some(previous) = previous_by_id.get(&owner.id) {
                if previous.as_element() == owner.as_element() {
                    *owner = (*previous).clone();
                }
            }
        }
    }

    /// Runtime layout inputs use deterministic application defaults.
    pub fn session_layout_inputs(&self) -> crate::layout_inputs::LayoutInputs {
        crate::layout_inputs::LayoutInputs::default()
    }

    /// Clone-stable identity for cache invalidation across document replacement.
    pub fn layout_identity(&self) -> &Arc<()> {
        &self.layout_identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModelError;

    #[test]
    fn definition_order_participates_in_project_equality() {
        let Ok(project) = Project::new() else {
            assert!(false, "static bootstrap should produce valid definitions");
            return;
        };
        let mut reordered = project.clone();
        let definitions = reordered
            .element_definitions()
            .iter()
            .cloned()
            .rev()
            .collect::<Vec<_>>();
        assert!(reordered.replace_element_definitions(definitions).is_ok());
        assert_ne!(project, reordered);
    }

    #[test]
    fn failed_definition_replacement_is_atomic() {
        let Ok(mut project) = Project::new() else {
            assert!(false, "static bootstrap should produce valid definitions");
            return;
        };
        let original = project.clone();
        let duplicate = project.element_definitions().iter().next().cloned();
        let Some(duplicate) = duplicate else {
            assert!(false, "bootstrap registry should be nonempty");
            return;
        };

        assert_eq!(
            project.replace_element_definitions(vec![duplicate.clone(), duplicate]),
            Err(ModelError::DuplicateElementDefinitionKey)
        );
        assert_eq!(project, original);
    }
}
