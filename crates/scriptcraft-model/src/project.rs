//! Semantic project state shared by editing, native FDX preservation, and layout.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    beat_board::BeatBoard,
    dual_dialogue::DualDialogueGroup,
    element::{body_owners_equal, BodyOwner},
    element_definitions::DocumentElementDefinitions,
    element_semantics::{BehaviorTargetResolution, ElementSemantics, ParagraphBehaviorKey},
    formatting::SourceParagraphOverrides,
    identity::{StableId, StableIdAllocator},
    layout_inputs::DocumentPageLayout,
    preservation::PreservationState,
    title_page::TitlePageParagraph,
    validation::validate_document_page_layout,
    ElementTypeKey, ModelError,
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
    /// Semantic Beat content; presentation and viewport state are separate.
    beat_board: BeatBoard,
    /// Ordered document-owned element settings. The backing registry is private
    /// and changes only through validated whole-registry replacement.
    element_definitions: DocumentElementDefinitions,
    /// Effective page layout is document data; retained FDX remains its source
    /// projection rather than a second runtime authority.
    page_layout: DocumentPageLayout,
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
            && self.beat_board == other.beat_board
            && self.element_definitions == other.element_definitions
            && self.page_layout == other.page_layout
    }
}

impl Eq for Project {}

impl Project {
    /// Creates an empty semantic projection with the standard effective page
    /// layout. Import installs its complete ordered registry before publishing;
    /// callers must populate and validate their own content explicitly.
    pub fn empty_projection() -> Result<Self, ModelError> {
        Self::empty_projection_with_page_layout(DocumentPageLayout::default())
    }

    /// Creates a fresh semantic projection with a validated effective page
    /// layout. This is a construction boundary, not an existing-document setter.
    pub fn empty_projection_with_page_layout(
        page_layout: DocumentPageLayout,
    ) -> Result<Self, ModelError> {
        validate_document_page_layout(&page_layout)?;
        Ok(Self {
            title_page: Vec::new(),
            elements: Vec::new(),
            dual_dialogue_groups: Vec::new(),
            direct_paragraph_overrides: BTreeMap::new(),
            dual_dialogue_paragraph_overrides: BTreeMap::new(),
            preservation: None,
            beat_board: BeatBoard::default(),
            element_definitions: DocumentElementDefinitions::default(),
            page_layout,
            layout_identity: Arc::new(()),
        })
    }

    /// Returns the immutable effective document page layout.
    pub fn page_layout(&self) -> &DocumentPageLayout {
        &self.page_layout
    }

    /// Returns the semantic board without exposing unchecked live mutation.
    pub fn beat_board(&self) -> &BeatBoard {
        &self.beat_board
    }

    /// Replaces the board only after validating a detached complete candidate.
    pub fn replace_beat_board(&mut self, board: BeatBoard) -> Result<(), ModelError> {
        let mut candidate = self.clone();
        candidate.beat_board = board;
        candidate.validate()?;
        self.beat_board = candidate.beat_board;
        Ok(())
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

    /// Allocates an application ID distinct from all semantic owner IDs.
    /// Foreign source IDs intentionally remain a separate namespace.
    pub fn allocate_element_id(&self) -> Result<StableId, ModelError> {
        self.allocate_stable_id("element")
    }

    fn allocate_stable_id(&self, prefix: &str) -> Result<StableId, ModelError> {
        self.stable_id_allocator()?.allocate(prefix)
    }

    /// Reserves every live semantic identity for detached candidate allocation.
    pub fn stable_id_allocator(&self) -> Result<StableIdAllocator, ModelError> {
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
        for card in &self.beat_board.cards {
            allocator.reserve(card.id.clone())?;
            for paragraph in &card.paragraphs {
                allocator.reserve(paragraph.id.clone())?;
            }
        }
        Ok(allocator)
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

    /// Runtime layout inputs are derived from this Project's effective page data.
    pub fn session_layout_inputs(&self) -> crate::layout_inputs::LayoutInputs {
        self.page_layout.session_layout_inputs()
    }

    /// Clone-stable identity for cache invalidation across document replacement.
    pub fn layout_identity(&self) -> &Arc<()> {
        &self.layout_identity
    }
}
