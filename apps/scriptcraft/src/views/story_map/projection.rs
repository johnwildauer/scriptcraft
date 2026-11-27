//! Document-scoped delayed projection state for canonical Body scenes/caret.

use scriptcraft_layout::StoryMapProjection;
use scriptcraft_model::{
    dual_dialogue::DualDialogueGroup,
    formatting::{SourceLayoutSettings, SourceParagraphOverrides},
    BodyOwner, DocumentElementDefinitions, DocumentPageLayout, LayoutInputs, Position, Project,
    StableId,
};
use std::{sync::Arc, time::{Duration, Instant}};

#[derive(Debug, Clone, PartialEq, Eq)]
struct BodyOwnerStamp {
    stable_id: StableId,
    address: usize,
    revision: u64,
}

#[derive(Debug, Clone)]
pub(super) struct ProjectionInputs {
    document_identity: Arc<()>,
    body_owners: Vec<BodyOwnerStamp>,
    body_focus: Position,
    layout_inputs: LayoutInputs,
    page_layout: DocumentPageLayout,
    source_layout_settings: Option<SourceLayoutSettings>,
    definitions: DocumentElementDefinitions,
    direct_paragraph_overrides: std::collections::BTreeMap<StableId, SourceParagraphOverrides>,
    dual_dialogue_paragraph_overrides:
        std::collections::BTreeMap<StableId, SourceParagraphOverrides>,
    dual_dialogue_groups: Vec<DualDialogueGroup>,
    active_body_group: Option<StableId>,
}

impl ProjectionInputs {
    pub(super) fn capture(
        project: &Project,
        layout_inputs: &LayoutInputs,
        body_focus: Position,
        active_body_group: Option<StableId>,
    ) -> Self {
        Self {
            document_identity: project.layout_identity().clone(),
            body_owners: body_owner_stamps(&project.elements),
            body_focus,
            layout_inputs: layout_inputs.clone(),
            page_layout: project.page_layout().clone(),
            source_layout_settings: project
                .preservation
                .as_ref()
                .map(|preservation| preservation.source_layout_settings().clone()),
            definitions: project.element_definitions().clone(),
            direct_paragraph_overrides: project.direct_paragraph_overrides.clone(),
            dual_dialogue_paragraph_overrides:
                project.dual_dialogue_paragraph_overrides.clone(),
            dual_dialogue_groups: project.dual_dialogue_groups.clone(),
            active_body_group,
        }
    }

    fn same_projection_inputs(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document_identity, &other.document_identity)
            && self.body_owners == other.body_owners
            && self.body_focus == other.body_focus
            && self.layout_inputs == other.layout_inputs
            && self.page_layout == other.page_layout
            && self.source_layout_settings == other.source_layout_settings
            && self.definitions == other.definitions
            && self.direct_paragraph_overrides == other.direct_paragraph_overrides
            && self.dual_dialogue_paragraph_overrides
                == other.dual_dialogue_paragraph_overrides
            && self.dual_dialogue_groups == other.dual_dialogue_groups
            && self.active_body_group == other.active_body_group
    }

}

/// Quiet scheduler plus one atomically replaced canonical projection snapshot.
#[derive(Debug, Default)]
pub(crate) struct StoryMapProjectionState {
    document_identity: Option<Arc<()>>,
    observed_inputs: Option<Arc<ProjectionInputs>>,
    snapshot_inputs: Option<Arc<ProjectionInputs>>,
    snapshot: Option<Arc<StoryMapProjection>>,
    observed_revision: Option<u64>,
    observed_mutation_token: Option<u64>,
    observed_activity_generation: u64,
    activity_generation: u64,
    generation: u64,
    attempted_generation: Option<u64>,
    failed_generation: Option<u64>,
    dirty: bool,
    deadline: Option<Instant>,
    was_visible: bool,
    body_pointer_selecting: bool,
}

impl StoryMapProjectionState {
    pub(crate) fn reset_document(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn snapshot(&self) -> Option<&StoryMapProjection> {
        let snapshot = self.snapshot.as_deref()?;
        let identity = self.document_identity.as_ref()?;
        let inputs = self.snapshot_inputs.as_ref()?;
        (Arc::ptr_eq(&snapshot.document_identity, identity)
            && Arc::ptr_eq(&inputs.document_identity, identity))
            .then_some(snapshot)
    }

    pub(crate) fn displayed_domain_pages(&self, target_pages: u32) -> f64 {
        self.snapshot()
            .map_or(f64::from(target_pages), |snapshot| {
                snapshot.displayed_domain_pages(target_pages)
            })
    }

    pub(crate) fn note_body_activity(&mut self) {
        self.activity_generation = self.activity_generation.wrapping_add(1);
    }

    pub(crate) fn note_body_pointer_button(&mut self, pressed: bool, inside_body: bool) {
        if pressed && inside_body {
            self.body_pointer_selecting = true;
            self.note_body_activity();
        } else if !pressed && self.body_pointer_selecting {
            self.body_pointer_selecting = false;
            self.note_body_activity();
        }
    }

    pub(crate) fn note_body_pointer_move(&mut self) {
        if self.body_pointer_selecting {
            self.note_body_activity();
        }
    }

    pub(crate) fn clear_body_pointer_capture(&mut self) {
        self.body_pointer_selecting = false;
    }

    /// Observes cheap identity/revision hints every composition, then retains and
    /// compares relevant inputs only after one of those hints changes.
    pub(crate) fn observe(
        &mut self,
        project: &Project,
        layout_inputs: &LayoutInputs,
        body_focus: Position,
        active_body_group: Option<StableId>,
        revision: u64,
        mutation_token: u64,
        visible: bool,
        now: Instant,
        quiet_delay: Duration,
    ) {
        let identity = project.layout_identity();
        let identity_changed = self
            .document_identity
            .as_ref()
            .is_none_or(|current| !Arc::ptr_eq(current, identity));
        if identity_changed {
            let activity_generation = self.activity_generation;
            *self = Self {
                document_identity: Some(identity.clone()),
                observed_activity_generation: activity_generation,
                ..Self::default()
            };
        }

        let focus_or_group_changed = self.observed_inputs.as_ref().is_none_or(|previous| {
            previous.body_focus != body_focus
                || previous.active_body_group.as_ref() != active_body_group.as_ref()
        });
        let cheap_body_changed = self.observed_inputs.as_ref().is_none_or(|previous| {
            !body_owners_match(&previous.body_owners, &project.elements)
        });
        let hint_changed = identity_changed
            || focus_or_group_changed
            || cheap_body_changed
            || self.observed_revision != Some(revision)
            || self.observed_mutation_token != Some(mutation_token);
        if hint_changed {
            let candidate = Arc::new(ProjectionInputs::capture(
                project,
                layout_inputs,
                body_focus,
                active_body_group,
            ));
            let relevant_change = self
                .observed_inputs
                .as_ref()
                .is_none_or(|previous| !previous.same_projection_inputs(&candidate));
            if relevant_change && !identity_changed {
                self.generation = self.generation.wrapping_add(1);
                self.dirty = true;
                self.deadline = Some(now + quiet_delay);
                self.failed_generation = None;
            }
            self.observed_inputs = Some(candidate);
            self.observed_revision = Some(revision);
            self.observed_mutation_token = Some(mutation_token);
        }

        if self.activity_generation != self.observed_activity_generation {
            self.observed_activity_generation = self.activity_generation;
            if !identity_changed {
                self.generation = self.generation.wrapping_add(1);
                self.dirty = true;
                self.deadline = Some(now + quiet_delay);
                self.failed_generation = None;
            }
        }

        if visible && !self.was_visible && self.failed_generation == Some(self.generation) {
            self.generation = self.generation.wrapping_add(1);
            self.dirty = true;
            self.deadline = Some(now);
            self.failed_generation = None;
        }
        self.was_visible = visible;
    }

    pub(crate) fn begin_due_attempt(&mut self, now: Instant) -> bool {
        if !self.was_visible || self.attempted_generation == Some(self.generation) {
            return false;
        }
        let initial = self.snapshot().is_none() && self.attempted_generation.is_none();
        let due = initial || (self.dirty && self.deadline.is_none_or(|deadline| deadline <= now));
        if due {
            self.attempted_generation = Some(self.generation);
        }
        due
    }

    pub(crate) fn remaining_delay(&self, now: Instant) -> Option<Duration> {
        if !self.was_visible || !self.dirty {
            return None;
        }
        let delay = self.deadline?.saturating_duration_since(now);
        (delay > Duration::ZERO).then_some(delay)
    }

    pub(super) fn complete_success(
        &mut self,
        snapshot: StoryMapProjection,
        inputs: ProjectionInputs,
        revision: u64,
        mutation_token: u64,
    ) -> bool {
        let Some(identity) = self.document_identity.as_ref() else {
            return false;
        };
        if !Arc::ptr_eq(identity, &snapshot.document_identity)
            || !Arc::ptr_eq(identity, &inputs.document_identity)
        {
            return false;
        }
        let inputs = Arc::new(inputs);
        // These assignments are the single publication point for a coherent
        // scene/caret/domain snapshot and the inputs that produced it.
        self.snapshot = Some(Arc::new(snapshot));
        self.snapshot_inputs = Some(inputs.clone());
        self.observed_inputs = Some(inputs);
        self.observed_revision = Some(revision);
        self.observed_mutation_token = Some(mutation_token);
        self.dirty = false;
        self.deadline = None;
        self.failed_generation = None;
        true
    }

    pub(crate) fn complete_failure(&mut self) {
        self.failed_generation = Some(self.generation);
        self.dirty = false;
        self.deadline = None;
    }
}

fn body_owner_stamps(owners: &[Arc<BodyOwner>]) -> Vec<BodyOwnerStamp> {
    owners
        .iter()
        .map(|owner| BodyOwnerStamp {
            stable_id: owner.id.clone(),
            address: Arc::as_ptr(owner) as usize,
            revision: owner.revision(),
        })
        .collect()
}

fn body_owners_match(stamps: &[BodyOwnerStamp], owners: &[Arc<BodyOwner>]) -> bool {
    stamps.len() == owners.len()
        && stamps.iter().zip(owners).all(|(stamp, owner)| {
            stamp.stable_id == owner.id
                && stamp.address == Arc::as_ptr(owner) as usize
                && stamp.revision == owner.revision()
        })
}
