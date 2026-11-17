use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    dual_dialogue::DualDialogueGroup,
    element::{body_owners_equal, BodyOwner},
    formatting::SourceParagraphOverrides,
    identity::{StableId, StableIdAllocator},
    preservation::PreservationState,
    title_page::{reference_boilerplate, TitlePageParagraph},
    ModelError, ScreenplayElement,
};

/// Legacy fields kept only while an unchanged JSON-origin document uses the
/// transitional JSON adapter. These are not part of the FDX semantic model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LegacyJsonMetadata {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) contact: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyJsonState {
    pub(crate) project_id: StableId,
    pub(crate) project_schema_version: u32,
    pub(crate) metadata: LegacyJsonMetadata,
    pub(crate) layout_inputs: crate::layout_inputs::LayoutInputs,
}

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
    /// Source-owned state is separate from the semantic projection and is not
    /// representable in transitional JSON.
    pub preservation: Option<PreservationState>,
    /// Transitional JSON-only source state; never consumed by the FDX codec.
    pub(crate) legacy_json_state: Option<LegacyJsonState>,
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
            && self.legacy_json_state == other.legacy_json_state
    }
}

impl Eq for Project {}

impl Project {
    pub fn new() -> Result<Self, ModelError> {
        let mut ids = StableIdAllocator::new();
        let title_page = reference_boilerplate(&mut ids)?;
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
            legacy_json_state: None,
            layout_identity: Arc::new(()),
        })
    }

    /// Creates the empty semantic projection required while importing FDX.
    /// It carries no title, body, group, override, preservation, or legacy state.
    pub fn empty_projection() -> Self {
        Self {
            title_page: Vec::new(),
            elements: Vec::new(),
            dual_dialogue_groups: Vec::new(),
            direct_paragraph_overrides: BTreeMap::new(),
            dual_dialogue_paragraph_overrides: BTreeMap::new(),
            preservation: None,
            legacy_json_state: None,
            layout_identity: Arc::new(()),
        }
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

    /// Runtime layout inputs use preserved legacy geometry when available and
    /// deterministic application defaults otherwise.
    pub fn session_layout_inputs(&self) -> crate::layout_inputs::LayoutInputs {
        self.legacy_json_state
            .as_ref()
            .map(|legacy| legacy.layout_inputs.clone())
            .unwrap_or_default()
    }

    /// Reports whether legacy JSON geometry fallback rules apply to this
    /// project, without exposing the adapter's private metadata.
    pub fn uses_legacy_layout_defaults(&self) -> bool {
        self.legacy_json_state.is_some()
    }

    /// Clone-stable identity for cache invalidation across document replacement.
    pub fn layout_identity(&self) -> &Arc<()> {
        &self.layout_identity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProjectEnvelope {
    pub schema_version: u32,
    pub project: Project,
}

impl NativeProjectEnvelope {
    pub fn new(project: Project) -> Result<Self, ModelError> {
        project.validate()?;
        Ok(Self {
            schema_version: crate::legacy_json::CURRENT_SCHEMA_VERSION,
            project,
        })
    }

    pub fn to_json(&self) -> Result<String, ModelError> {
        if self.schema_version != crate::legacy_json::CURRENT_SCHEMA_VERSION {
            return Err(ModelError::InvalidSchemaVersion);
        }
        self.project.to_json()
    }

    pub fn from_json(input: &str) -> Result<Self, ModelError> {
        let project = Project::from_json(input)?;
        Ok(Self {
            schema_version: crate::legacy_json::CURRENT_SCHEMA_VERSION,
            project,
        })
    }
}
