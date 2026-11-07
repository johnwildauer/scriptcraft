//! Headless, transactional editing over the semantic screenplay model.

use crate::{
    cast_list::{self, CastListError},
    validate_layout_inputs, LayoutInputs, ModelError, Project, Role, ScreenplayElement, StableId,
    TextRun, MAX_ELEMENTS, MAX_TEXT_BYTES,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

/// Session edit-validation work counts. These are structural counters, not
/// timings, and do not expose document values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EditValidationDiagnostics {
    pub scoped_candidate_attempts: u64,
    pub scoped_candidate_accepts: u64,
    pub full_candidate_validations: u64,
    pub full_validation_fallbacks: u64,
}

static SCOPED_CANDIDATE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static SCOPED_CANDIDATE_ACCEPTS: AtomicU64 = AtomicU64::new(0);
static FULL_CANDIDATE_VALIDATIONS: AtomicU64 = AtomicU64::new(0);
static FULL_VALIDATION_FALLBACKS: AtomicU64 = AtomicU64::new(0);

pub fn edit_validation_diagnostics() -> EditValidationDiagnostics {
    EditValidationDiagnostics {
        scoped_candidate_attempts: SCOPED_CANDIDATE_ATTEMPTS.load(Ordering::Relaxed),
        scoped_candidate_accepts: SCOPED_CANDIDATE_ACCEPTS.load(Ordering::Relaxed),
        full_candidate_validations: FULL_CANDIDATE_VALIDATIONS.load(Ordering::Relaxed),
        full_validation_fallbacks: FULL_VALIDATION_FALLBACKS.load(Ordering::Relaxed),
    }
}

pub fn reset_edit_validation_diagnostics() {
    SCOPED_CANDIDATE_ATTEMPTS.store(0, Ordering::Relaxed);
    SCOPED_CANDIDATE_ACCEPTS.store(0, Ordering::Relaxed);
    FULL_CANDIDATE_VALIDATIONS.store(0, Ordering::Relaxed);
    FULL_VALIDATION_FALLBACKS.store(0, Ordering::Relaxed);
}

/// Value-free view of session-owned derived invalidation for scheduler policy.
/// Scene identities remain internal to the core handoff.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DerivedWorkSummary {
    /// Whether any derived invalidation remains, including safely deferred scenes.
    pub pending: bool,
    pub complete: bool,
    pub affected_scene_count: usize,
    /// Whether the scheduler can make progress now. Deferred-only work waits for
    /// a later relevant edit instead of being retried on every timer tick.
    pub retryable: bool,
    /// Number of affected scenes currently held by safe-stop deferral.
    pub deferred_scene_count: usize,
    /// Whether a complete refresh failed and is suspended pending invalidation.
    pub failed_complete: bool,
    /// Number of affected scenes whose correction failed and is suspended.
    pub failed_scene_count: usize,
}

/// Pending derived work. Scene-heading identities are deliberately not exposed
/// through the public summary or diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingDerivedInvalidation {
    None,
    Scenes(BTreeSet<String>),
    Complete,
}

impl PendingDerivedInvalidation {
    fn summary(&self) -> DerivedWorkSummary {
        match self {
            Self::None => DerivedWorkSummary::default(),
            Self::Scenes(scenes) => DerivedWorkSummary {
                pending: !scenes.is_empty(),
                complete: false,
                affected_scene_count: scenes.len(),
                retryable: !scenes.is_empty(),
                deferred_scene_count: 0,
                failed_complete: false,
                failed_scene_count: 0,
            },
            Self::Complete => DerivedWorkSummary {
                pending: true,
                complete: true,
                affected_scene_count: 0,
                retryable: true,
                deferred_scene_count: 0,
                failed_complete: false,
                failed_scene_count: 0,
            },
        }
    }

    fn merge(&mut self, newer: Self) {
        match (self, newer) {
            (current @ Self::Complete, _) => *current = Self::Complete,
            (current, Self::Complete) => *current = Self::Complete,
            (Self::None, Self::None) => {}
            (current @ Self::None, Self::Scenes(scenes)) => {
                *current = Self::Scenes(scenes);
            }
            (Self::Scenes(_), Self::None) => {}
            (Self::Scenes(current), Self::Scenes(scenes)) => current.extend(scenes),
        }
    }
}

fn validate_full_candidate(project: &Project) -> Result<(), ModelError> {
    let _ = FULL_CANDIDATE_VALIDATIONS.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_add(1))
    });
    project.validate()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub element: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

impl Selection {
    pub const fn caret(position: Position) -> Self {
        Self {
            anchor: position,
            focus: position,
        }
    }

    fn ordered(self) -> (Position, Position) {
        if (self.anchor.element, self.anchor.offset) <= (self.focus.element, self.focus.offset) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }
}

/// Ephemeral editing behavior. This is session/application input, not document
/// state and is intentionally absent from snapshots and native serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditPolicy {
    pub auto_parenthetical: bool,
    pub strict_transition_boundaries: bool,
}

impl Default for EditPolicy {
    fn default() -> Self {
        Self {
            auto_parenthetical: true,
            strict_transition_boundaries: true,
        }
    }
}

/// Read-only outcome for a paragraph-key projection. A `Target` is the role
/// resolved by editing policy, not a promise that later safety checks will
/// allow the candidate to publish. `NoOp` describes a consumed key with no
/// semantic target; `Unavailable` describes a key that cannot resolve for the
/// current selection (or whose editing resolver rejects the request).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphKeyTarget {
    Target(Role),
    NoOp,
    Unavailable,
}

/// Actual Enter and Tab target decisions for the session's current selection
/// and editing policy. This value is a nonmutating hint, not a preview edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParagraphKeyTargets {
    pub enter: ParagraphKeyTarget,
    pub tab: ParagraphKeyTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidElement,
    InvalidUtf8Boundary,
    InvalidSelection,
    EmptyHistory,
    TextTooLong,
    TooManyElements,
    UnsupportedTransition { from: Role },
    Preservation(crate::preservation::PreservationError),
    CastList(CastListError),
    Model(ModelError),
}

impl EditError {
    /// Returns only the bounded call-site label for a managed-text mismatch.
    pub fn managed_text_mismatch_diagnostic(&self) -> Option<&'static str> {
        use crate::preservation::{ManagedTextMismatchSite, PreservationError};

        let site = match self {
            Self::Preservation(PreservationError::ManagedTextMismatchAt(site)) => Some(*site),
            Self::Model(ModelError::InvalidPreservationState(
                PreservationError::ManagedTextMismatchAt(site),
            )) => Some(*site),
            Self::CastList(CastListError::Preservation(
                PreservationError::ManagedTextMismatchAt(site),
            )) => Some(*site),
            Self::CastList(CastListError::Model(ModelError::InvalidPreservationState(
                PreservationError::ManagedTextMismatchAt(site),
            ))) => Some(*site),
            Self::CastList(CastListError::SceneMetadata(
                crate::scene_metadata::SceneMetadataError::Preservation(
                    PreservationError::ManagedTextMismatchAt(site),
                ),
            )) => Some(*site),
            _ => None,
        };
        site.map(ManagedTextMismatchSite::diagnostic_name)
    }

    /// Converts scheduled derived-refresh failures to a bounded category.
    pub fn scheduled_refresh_failure_category(&self) -> &'static str {
        match self {
            Self::CastList(CastListError::SceneMetadata(_)) => "scene reconciliation",
            Self::CastList(CastListError::Preservation(_)) => "Cast List source mapping",
            Self::CastList(CastListError::Model(_)) => "model validation",
            Self::CastList(CastListError::UnsupportedFormattedCastList) => {
                "formatted Cast List mapping"
            }
            _ => "other safe refresh failure",
        }
    }
}

impl From<ModelError> for EditError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    // Project includes semantic runs and the complete preservation graph, so
    // undo/redo restores both authorities rather than only the text projection.
    project: Project,
    selection: Selection,
    generated_parentheticals: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct EditingSession {
    project: Project,
    /// Wall-clock time captured when this editing session begins. It is
    /// ephemeral lineage state, independent of DocumentRef and undo history.
    session_open_timestamp: String,
    policy: EditPolicy,
    layout_inputs: LayoutInputs,
    selection: Selection,
    dirty: bool,
    revision: u64,
    clean_project: Project,
    history: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    generated_parentheticals: BTreeSet<String>,
    history_limit: usize,
    validated_baseline: bool,
    last_validation_was_scoped: bool,
    pending_derived_invalidation: PendingDerivedInvalidation,
    deferred_derived_scenes: BTreeSet<String>,
    failed_derived_complete: bool,
    failed_derived_scenes: BTreeSet<String>,
}

impl PartialEq for EditingSession {
    fn eq(&self, other: &Self) -> bool {
        self.project == other.project
            && self.session_open_timestamp == other.session_open_timestamp
            && self.policy == other.policy
            && self.layout_inputs == other.layout_inputs
            && self.selection == other.selection
            && self.dirty == other.dirty
            && self.revision == other.revision
            && self.clean_project == other.clean_project
            && self.history == other.history
            && self.redo == other.redo
            && self.generated_parentheticals == other.generated_parentheticals
            && self.history_limit == other.history_limit
            && self.validated_baseline == other.validated_baseline
            && self.pending_derived_invalidation == other.pending_derived_invalidation
            && self.deferred_derived_scenes == other.deferred_derived_scenes
            && self.failed_derived_complete == other.failed_derived_complete
            && self.failed_derived_scenes == other.failed_derived_scenes
    }
}

impl Eq for EditingSession {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MutationValidationScope {
    Complete,
    SingleParagraph,
}

impl EditingSession {
    pub fn new(project: Project) -> Result<Self, EditError> {
        Self::with_policy(project, EditPolicy::default())
    }

    pub fn with_policy(project: Project, policy: EditPolicy) -> Result<Self, EditError> {
        let layout_inputs = project.session_layout_inputs();
        Self::with_policy_and_layout_inputs(project, policy, layout_inputs)
    }

    pub fn with_policy_and_layout_inputs(
        project: Project,
        policy: EditPolicy,
        layout_inputs: LayoutInputs,
    ) -> Result<Self, EditError> {
        Self::with_policy_layout_and_session_timestamp(
            project,
            policy,
            layout_inputs,
            crate::fdx::current_timestamp(),
        )
    }

    fn with_policy_layout_and_session_timestamp(
        mut project: Project,
        policy: EditPolicy,
        layout_inputs: LayoutInputs,
        session_open_timestamp: String,
    ) -> Result<Self, EditError> {
        validate_full_candidate(&project)?;
        if let Some(preservation) = project.preservation.as_mut() {
            preservation.rebuild_managed_paragraph_index();
        }
        validate_layout_inputs(&layout_inputs)?;
        let selection = Selection::caret(Position {
            element: 0,
            offset: 0,
        });
        validate_selection(&project, selection)?;
        Ok(Self {
            clean_project: project.clone(),
            project,
            session_open_timestamp,
            policy,
            layout_inputs,
            selection,
            dirty: false,
            revision: 0,
            history: Vec::new(),
            redo: Vec::new(),
            generated_parentheticals: BTreeSet::new(),
            history_limit: 100,
            validated_baseline: true,
            last_validation_was_scoped: false,
            pending_derived_invalidation: PendingDerivedInvalidation::Complete,
            deferred_derived_scenes: BTreeSet::new(),
            failed_derived_complete: false,
            failed_derived_scenes: BTreeSet::new(),
        })
    }

    pub fn with_history_limit(project: Project, history_limit: usize) -> Result<Self, EditError> {
        let mut session = Self::new(project)?;
        session.history_limit = history_limit;
        Ok(session)
    }

    pub fn project(&self) -> &Project {
        &self.project
    }
    pub(crate) fn session_open_timestamp(&self) -> &str {
        &self.session_open_timestamp
    }
    /// Current ephemeral editing policy. Undo/redo and document replacement do
    /// not restore or alter it; a newly constructed session uses defaults.
    pub const fn policy(&self) -> EditPolicy {
        self.policy
    }
    /// Effective geometry inputs are session-scoped and intentionally excluded
    /// from project snapshots, dirty tracking and native serialization.
    pub fn layout_inputs(&self) -> &LayoutInputs {
        &self.layout_inputs
    }
    /// Changes effective layout inputs without changing document data or dirty
    /// state. Callers must supply a complete validated input set.
    pub fn set_layout_inputs(&mut self, inputs: LayoutInputs) -> Result<(), EditError> {
        validate_layout_inputs(&inputs)?;
        self.layout_inputs = inputs;
        Ok(())
    }
    /// Changes runtime policy without altering document contents, revision,
    /// history, or clean/dirty state.
    pub fn set_policy(&mut self, policy: EditPolicy) {
        self.policy = policy;
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    /// Projects the exact role decisions used by [`Self::enter`] and
    /// [`Self::tab`] without executing edits, touching history, or changing
    /// dirty/revision state. The result intentionally does not guarantee that
    /// preservation or capacity checks will accept a later edit.
    pub fn paragraph_key_targets(&self) -> ParagraphKeyTargets {
        let enter = resolve_enter(
            &self.project,
            self.selection,
            &self.generated_parentheticals,
            &self.policy,
        )
        .map_or(ParagraphKeyTarget::Unavailable, |decision| decision.target);
        let tab = resolve_tab(
            &self.project,
            self.selection,
            &self.generated_parentheticals,
            &self.policy,
        )
        .map_or(ParagraphKeyTarget::Unavailable, |decision| decision.target);
        ParagraphKeyTargets { enter, tab }
    }
    pub const fn dirty(&self) -> bool {
        self.dirty
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// Returns value-free derived-work state without disclosing document or
    /// scene identities. A complete refresh reports zero individually affected
    /// scenes because its scope is the whole document. Deferred-only work stays
    /// pending but is not retryable until a later relevant invalidation.
    pub fn pending_derived_work_summary(&self) -> DerivedWorkSummary {
        let mut summary = self.pending_derived_invalidation.summary();
        match &self.pending_derived_invalidation {
            PendingDerivedInvalidation::Complete => {
                summary.failed_complete = self.failed_derived_complete;
                summary.retryable = !self.failed_derived_complete;
            }
            PendingDerivedInvalidation::Scenes(scenes) => {
                summary.deferred_scene_count =
                    scenes.intersection(&self.deferred_derived_scenes).count();
                summary.failed_scene_count =
                    scenes.intersection(&self.failed_derived_scenes).count();
                summary.retryable = scenes
                    .difference(&self.deferred_derived_scenes)
                    .find(|scene| !self.failed_derived_scenes.contains(*scene))
                    .is_some();
            }
            PendingDerivedInvalidation::None => {}
        }
        summary
    }

    /// Completes only work actually reconciled by a successful correction.
    /// Scoped completion cannot consume a pending complete refresh.
    pub(crate) fn complete_derived_invalidation(
        &mut self,
        completed_scenes: &BTreeSet<String>,
        deferred_scenes: &BTreeSet<String>,
        complete_refresh: bool,
    ) {
        if complete_refresh {
            self.pending_derived_invalidation = if deferred_scenes.is_empty() {
                PendingDerivedInvalidation::None
            } else {
                PendingDerivedInvalidation::Scenes(deferred_scenes.clone())
            };
            self.deferred_derived_scenes = deferred_scenes.clone();
            self.failed_derived_complete = false;
            self.failed_derived_scenes.clear();
            return;
        }
        if let PendingDerivedInvalidation::Scenes(pending) = &mut self.pending_derived_invalidation
        {
            pending.retain(|scene| !completed_scenes.contains(scene));
            if pending.is_empty() {
                self.pending_derived_invalidation = PendingDerivedInvalidation::None;
            }
        }
        self.failed_derived_scenes
            .retain(|scene| !completed_scenes.contains(scene));
        if let PendingDerivedInvalidation::Scenes(pending) = &mut self.pending_derived_invalidation
        {
            pending.extend(deferred_scenes.iter().cloned());
        }
        self.deferred_derived_scenes
            .retain(|scene| !completed_scenes.contains(scene));
        self.deferred_derived_scenes
            .extend(deferred_scenes.iter().cloned());
    }
    /// Reports whether the latest successful session mutation used the
    /// preconditioned single-paragraph validator. This is diagnostic-only.
    pub const fn last_edit_used_scoped_validation(&self) -> bool {
        self.last_validation_was_scoped
    }

    /// Revalidates the live semantic project and selection without mutating it.
    pub fn validate(&self) -> Result<(), EditError> {
        validate_full_candidate(&self.project)?;
        validate_selection(&self.project, self.selection)
    }

    pub fn mark_saved(&mut self) {
        self.clean_project = self.project.clone();
        self.dirty = false;
    }

    /// Refreshes derived cast lists without adding a separate undo entry.
    /// Selection is unchanged; dirty state and revision follow the corrected
    /// project so cached layout and the clean baseline remain accurate.
    pub fn refresh_cast_lists(&mut self) -> Result<bool, EditError> {
        self.refresh_cast_lists_with_outcome()
            .map(|outcome| outcome.changed)
    }

    /// Refreshes derived data and reports safe-stop deferrals without exposing
    /// document-owned identities or content to the caller.
    pub fn refresh_cast_lists_with_outcome(
        &mut self,
    ) -> Result<crate::cast_list::CastListCorrection, EditError> {
        let (scope, complete_refresh) = match &self.pending_derived_invalidation {
            PendingDerivedInvalidation::None => {
                return Ok(crate::cast_list::CastListCorrection::default());
            }
            PendingDerivedInvalidation::Complete => {
                if self.failed_derived_complete {
                    return Ok(crate::cast_list::CastListCorrection::default());
                }
                (None, true)
            }
            PendingDerivedInvalidation::Scenes(scenes) => {
                let retryable = scenes
                    .difference(&self.deferred_derived_scenes)
                    .filter(|scene| !self.failed_derived_scenes.contains(*scene))
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if retryable.is_empty() {
                    return Ok(crate::cast_list::CastListCorrection::default());
                }
                (Some(retryable), false)
            }
        };
        let (mut project, outcome, deferred_scenes, completed_scenes) =
            match cast_list::corrected_cast_list_candidate_with_scope(&self.project, scope.as_ref())
            {
                Ok(candidate) => candidate,
                Err(error) => {
                    if complete_refresh {
                        self.failed_derived_complete = true;
                    } else if let Some(attempted_scenes) = scope {
                        self.failed_derived_scenes.extend(attempted_scenes);
                    }
                    return Err(EditError::CastList(error));
                }
            };
        if outcome.changed {
            if let Some(preservation) = project.preservation.as_mut() {
                if !preservation.has_source_identity_use_index() {
                    preservation.rebuild_managed_paragraph_index();
                }
            }
            self.project = project;
            self.revision = self.revision.saturating_add(1);
            self.dirty = self.project != self.clean_project;
            self.last_validation_was_scoped = false;
        }
        self.complete_derived_invalidation(&completed_scenes, &deferred_scenes, complete_refresh);
        Ok(outcome)
    }

    /// Builds corrected detached data for Save or Save As preflight. This does
    /// not mutate the live session; the storage owner controls publication.
    pub fn prepare_cast_list_save_candidate(&self) -> Result<Project, EditError> {
        cast_list::prepare_save_candidate(&self.project).map_err(EditError::CastList)
    }

    /// Builds a detached session representing a successfully serialized and
    /// re-read project. This is prepared before storage publication; callers
    /// must install it only after the destination has been replaced.
    pub(crate) fn prepare_saved_rebase(&self, mut project: Project) -> Result<Self, EditError> {
        validate_full_candidate(&project)?;
        if let Some(preservation) = project.preservation.as_mut() {
            preservation.rebuild_managed_paragraph_index();
        }
        if project.elements.len() != self.project.elements.len() {
            return Err(EditError::InvalidSelection);
        }

        let mut rebased = self.clone();
        let selection = map_selection_to_project(self.selection, &project)?;
        let generated_parentheticals = self
            .project
            .elements
            .iter()
            .zip(&project.elements)
            .filter(|(old, new)| {
                old.role == Role::Parenthetical
                    && new.role == Role::Parenthetical
                    && self.generated_parentheticals.contains(old.id.as_str())
            })
            .map(|(_, new)| new.id.as_str().to_owned())
            .collect();

        rebased.project = project.clone();
        rebased.clean_project = project;
        rebased.selection = selection;
        rebased.generated_parentheticals = generated_parentheticals;
        rebased.dirty = false;
        rebased.last_validation_was_scoped = false;
        // The detached save candidate has already passed complete derived
        // preparation; do not schedule a duplicate timer pass after publish.
        rebased.pending_derived_invalidation = PendingDerivedInvalidation::None;
        rebased.deferred_derived_scenes.clear();
        rebased.failed_derived_complete = false;
        rebased.failed_derived_scenes.clear();
        // Saving changes the preservation baseline and therefore invalidates
        // render caches even when the visible semantic text did not change.
        rebased.revision = rebased.revision.saturating_add(1);
        rebased.validate()?;
        Ok(rebased)
    }

    pub fn replace_with_saved(&mut self, project: Project) -> Result<(), EditError> {
        let mut replacement = Self::with_policy(project, self.policy)?;
        replacement.session_open_timestamp = self.session_open_timestamp.clone();
        *self = replacement;
        Ok(())
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn set_selection(&mut self, selection: Selection) -> Result<(), EditError> {
        validate_selection(&self.project, selection)?;
        self.selection = selection;
        Ok(())
    }

    /// Inserts or replaces text while retaining the existing element identity.
    /// Mapped source runs and text leaves are transformed in the same candidate;
    /// unsafe inline intersections reject the whole operation.
    pub fn insert(&mut self, text: &str) -> Result<(), EditError> {
        self.mutate(
            MutationValidationScope::SingleParagraph,
            |project, selection, _generated, _policy| {
                let (start, end) = selection.ordered();
                replace_range(project, start, end, text)?;
                Ok(Selection::caret(Position {
                    element: start.element,
                    offset: start.offset + text.len(),
                }))
            },
        )
    }

    /// Inserts clipboard text as one transactional semantic edit.
    ///
    /// Newline characters remain in the active element. Layout is responsible
    /// for visual line breaks; paste does not silently invent screenplay roles
    /// or multiple undo entries.
    pub fn insert_paste(&mut self, text: &str) -> Result<(), EditError> {
        self.insert(text)
    }

    /// Deletes text or, when the selection spans paragraph boundaries, removes
    /// the selected source-anchored paragraphs transactionally.
    pub fn delete(&mut self) -> Result<(), EditError> {
        let (selection_start, selection_end) = self.selection.ordered();
        let generated_scene_heading_deletions = if selection_start.element < selection_end.element {
            self.project
                .elements
                .iter()
                .enumerate()
                .filter(|(index, element)| {
                    *index > selection_start.element
                        && *index <= selection_end.element
                        && element.role == Role::SceneHeading
                        && (*index < selection_end.element
                            || selection_end.offset == element.text.len())
                })
                .map(|(_, element)| element.id.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        self.mutate_with_scene_heading_deletions(
            MutationValidationScope::SingleParagraph,
            generated_scene_heading_deletions,
            |project, selection, generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end {
                    replace_range(project, start, end, "")?;
                    return Ok(Selection::caret(start));
                }
                let item = project
                    .elements
                    .get(start.element)
                    .ok_or(EditError::InvalidElement)?;
                if item.role == Role::Parenthetical
                    && is_delimiter_only_parenthetical(item.text.as_str())
                {
                    let item_text_len = item.text.len();
                    let id = item.id.as_str().to_owned();
                    generated.remove(&id);
                    if start.element == 0 || project.elements.len() == 1 {
                        replace_range(
                            project,
                            Position {
                                element: 0,
                                offset: 0,
                            },
                            Position {
                                element: 0,
                                offset: item_text_len,
                            },
                            "",
                        )?;
                        update_preserved_role(project, 0, Role::Parenthetical, Role::Dialogue)?;
                        project.elements[0].role = Role::Dialogue;
                        return Ok(Selection::caret(Position {
                            element: 0,
                            offset: 0,
                        }));
                    }
                    let previous = start.element.saturating_sub(1);
                    let offset = project.elements[previous].text.len();
                    project.elements.remove(start.element);
                    return Ok(Selection::caret(Position {
                        element: previous,
                        offset,
                    }));
                }
                if start.offset < item.text.len() {
                    let next = next_boundary(&item.text, start.offset);
                    replace_range(
                        project,
                        start,
                        Position {
                            offset: next,
                            ..start
                        },
                        "",
                    )?;
                }
                Ok(Selection::caret(start))
            },
        )
    }

    pub fn backspace(&mut self) -> Result<(), EditError> {
        self.mutate(
            MutationValidationScope::SingleParagraph,
            |project, selection, _generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end {
                    replace_range(project, start, end, "")?;
                    return Ok(Selection::caret(start));
                }
                if start.offset > 0 {
                    let item = project
                        .elements
                        .get(start.element)
                        .ok_or(EditError::InvalidElement)?;
                    let previous = previous_boundary(&item.text, start.offset);
                    let from = Position {
                        offset: previous,
                        ..start
                    };
                    replace_range(project, from, start, "")?;
                    return Ok(Selection::caret(from));
                }
                if start.element > 0 {
                    let previous = project
                        .elements
                        .get(start.element - 1)
                        .ok_or(EditError::InvalidElement)?;
                    let offset = previous.text.len();
                    join_at(project, start.element - 1)?;
                    return Ok(Selection::caret(Position {
                        element: start.element - 1,
                        offset,
                    }));
                }
                Ok(Selection::caret(start))
            },
        )
    }

    /// Splits a semantic paragraph while retaining the original foreign ID on
    /// the left side and allocating a fresh source identity for the right.
    pub fn split(&mut self) -> Result<(), EditError> {
        self.mutate(
            MutationValidationScope::Complete,
            |project, selection, _generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end {
                    return Err(EditError::InvalidSelection);
                }
                split_at(project, start, project.elements[start.element].role)
            },
        )
    }

    pub fn enter(&mut self) -> Result<(), EditError> {
        self.mutate(
            MutationValidationScope::Complete,
            |project, selection, generated, policy| {
                let decision = resolve_enter(project, *selection, generated, policy)?;
                match decision.action {
                    EnterAction::NoOp => Ok(*selection),
                    EnterAction::InsertParentheticalBefore { element, automatic } => {
                        let id = split_id(project)?;
                        let new = ScreenplayElement::new(
                            id.clone(),
                            Role::Parenthetical,
                            if automatic { "()" } else { "" },
                        )?;
                        project.elements.insert(element, new);
                        if automatic {
                            generated.insert(id);
                            Ok(Selection::caret(Position { element, offset: 1 }))
                        } else {
                            Ok(Selection::caret(Position { element, offset: 0 }))
                        }
                    }
                    EnterAction::ClearGeneratedAndSplit { position, role } => {
                        project.elements[position.element].text.clear();
                        generated.remove(project.elements[position.element].id.as_str());
                        split_at(project, position, role)
                    }
                    EnterAction::RepairAndInsertDialogue { element } => {
                        let text = project.elements[element].text.clone();
                        project.elements[element].text = repair_parenthetical(text.as_str());
                        insert_dialogue_below(project, element)
                    }
                    EnterAction::Split { position, role } => split_at(project, position, role),
                }
            },
        )
    }

    /// Applies the explicit Tab policy. Unlisted roles are intentionally consumed
    /// without changing the project, selection, or history.
    pub fn tab(&mut self) -> Result<(), EditError> {
        self.mutate(
            MutationValidationScope::Complete,
            |project, selection, generated, policy| {
                let decision = resolve_tab(project, *selection, generated, policy)?;
                match decision.action {
                    TabAction::NoOp => Ok(*selection),
                    TabAction::DelimiterParentheticalToDialogue { element } => {
                        let text_len = project.elements[element].text.len();
                        replace_range(
                            project,
                            Position { element, offset: 0 },
                            Position { element, offset: text_len },
                            "",
                        )?;
                        update_preserved_role(
                            project,
                            element,
                            Role::Parenthetical,
                            Role::Dialogue,
                        )?;
                        project.elements[element].role = Role::Dialogue;
                        generated.remove(project.elements[element].id.as_str());
                        Ok(Selection::caret(Position { element, offset: 0 }))
                    }
                    TabAction::RepairAndInsertDialogue { element } => {
                        let text = project.elements[element].text.clone();
                        project.elements[element].text = repair_parenthetical(text.as_str());
                        insert_dialogue_below(project, element)
                    }
                    TabAction::SetRole {
                        position,
                        role,
                        clear_generated_parenthetical,
                    } => {
                        if clear_generated_parenthetical {
                            project.elements[position.element].text.clear();
                            generated.remove(project.elements[position.element].id.as_str());
                        }
                        let old_role = project.elements[position.element].role;
                        if old_role != role {
                            update_preserved_role(project, position.element, old_role, role)?;
                            project.elements[position.element].role = role;
                        }
                        if policy.auto_parenthetical
                            && role == Role::Parenthetical
                            && project.elements[position.element].text.is_empty()
                        {
                            replace_range(
                                project,
                                Position { element: position.element, offset: 0 },
                                Position { element: position.element, offset: 0 },
                                "()",
                            )?;
                            generated.insert(
                                project.elements[position.element].id.as_str().to_owned(),
                            );
                            return Ok(Selection::caret(Position {
                                element: position.element,
                                offset: 1,
                            }));
                        }
                        Ok(Selection::caret(position))
                    }
                }
            },
        )
    }

    /// Joins adjacent paragraphs while retaining the left source identity. Any
    /// opaque attachment or unsafe reference prevents the candidate from being
    /// published.
    pub fn join_with_next(&mut self) -> Result<(), EditError> {
        self.mutate(
            MutationValidationScope::Complete,
            |project, selection, _generated, _policy| {
                let (start, end) = selection.ordered();
                if start != end || start.offset != project.elements[start.element].text.len() {
                    return Err(EditError::InvalidSelection);
                }
                let offset = project.elements[start.element].text.len();
                join_at(project, start.element)?;
                Ok(Selection::caret(Position {
                    element: start.element,
                    offset,
                }))
            },
        )
    }

    /// Changes a known role and its exact mapped source Type attribute together.
    /// A role change that would invalidate attached scene data is rejected.
    pub fn change_role(&mut self, role: Role) -> Result<(), EditError> {
        self.mutate(
            MutationValidationScope::SingleParagraph,
            |project, selection, generated, policy| {
                let (start, end) = selection.ordered();
                if start.element != end.element {
                    return Err(EditError::InvalidSelection);
                }
                let old_role = project.elements[start.element].role;
                if old_role != role {
                    update_preserved_role(project, start.element, old_role, role)?;
                }
                project.elements[start.element].role = role;
                if role != Role::Parenthetical {
                    generated.remove(project.elements[start.element].id.as_str());
                } else if policy.auto_parenthetical
                    && project.elements[start.element].text.is_empty()
                {
                    replace_range(
                        project,
                        Position {
                            element: start.element,
                            offset: 0,
                        },
                        Position {
                            element: start.element,
                            offset: 0,
                        },
                        "()",
                    )?;
                    generated.insert(project.elements[start.element].id.as_str().to_owned());
                    return Ok(Selection::caret(Position {
                        element: start.element,
                        offset: 1,
                    }));
                }
                Ok(*selection)
            },
        )
    }

    pub fn undo(&mut self) -> Result<(), EditError> {
        let mut snapshot = self
            .history
            .last()
            .cloned()
            .ok_or(EditError::EmptyHistory)?;
        // Undo must remain available even when a derived list's source mapping
        // cannot be changed safely; retain that snapshot unchanged and let the
        // scheduled refresh/save preflight report the mapping limitation.
        let _ = cast_list::correct_cast_lists(&mut snapshot.project);
        validate_full_candidate(&snapshot.project)?;
        if let Some(preservation) = snapshot.project.preservation.as_mut() {
            preservation.rebuild_managed_paragraph_index();
        }
        validate_selection(&snapshot.project, snapshot.selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.history.pop();
        self.redo.push(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.generated_parentheticals = snapshot.generated_parentheticals;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.last_validation_was_scoped = false;
        self.pending_derived_invalidation = PendingDerivedInvalidation::Complete;
        self.deferred_derived_scenes.clear();
        self.failed_derived_complete = false;
        self.failed_derived_scenes.clear();
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), EditError> {
        let mut snapshot = self.redo.last().cloned().ok_or(EditError::EmptyHistory)?;
        let _ = cast_list::correct_cast_lists(&mut snapshot.project);
        validate_full_candidate(&snapshot.project)?;
        if let Some(preservation) = snapshot.project.preservation.as_mut() {
            preservation.rebuild_managed_paragraph_index();
        }
        validate_selection(&snapshot.project, snapshot.selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.redo.pop();
        self.push_history(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.generated_parentheticals = snapshot.generated_parentheticals;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.last_validation_was_scoped = false;
        self.pending_derived_invalidation = PendingDerivedInvalidation::Complete;
        self.deferred_derived_scenes.clear();
        self.failed_derived_complete = false;
        self.failed_derived_scenes.clear();
        Ok(())
    }

    fn mutate<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        operation: F,
    ) -> Result<(), EditError>
    where
        F: FnOnce(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.mutate_with_scene_heading_deletions(requested_scope, BTreeSet::new(), operation)
    }

    fn mutate_with_scene_heading_deletions<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        generated_scene_heading_deletions: BTreeSet<StableId>,
        operation: F,
    ) -> Result<(), EditError>
    where
        F: FnOnce(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.last_validation_was_scoped = false;
        let (scope_start, scope_end) = self.selection.ordered();
        let requested_element = (requested_scope == MutationValidationScope::SingleParagraph
            && scope_start.element == scope_end.element)
            .then(|| {
                self.project
                    .elements
                    .get(scope_start.element)
                    .map(|element| element.id.clone())
            })
            .flatten();
        let mut project = self.project.clone();
        let mut generated = self.generated_parentheticals.clone();
        let prior_generated = generated.clone();
        let selection = operation(&mut project, &self.selection, &mut generated, &self.policy)?;
        if let Some(preservation) = project.preservation.as_mut() {
            if !preservation.has_source_identity_use_index() {
                preservation.rebuild_managed_paragraph_index();
            }
        }
        invalidate_changed_generated_parentheticals(
            &self.project,
            &project,
            &prior_generated,
            &mut generated,
        );
        generated.retain(|id| {
            project
                .elements
                .iter()
                .any(|element| element.id.as_str() == id && element.role == Role::Parenthetical)
        });
        let scoped_preconditions = self.validated_baseline
            && requested_element.as_ref().is_some_and(|id| {
                project.elements.len() == self.project.elements.len()
                    && project
                        .elements
                        .get(scope_start.element)
                        .is_some_and(|element| &element.id == id)
                    && selection.anchor.element == scope_start.element
                    && selection.focus.element == scope_start.element
                    && project
                        .preservation
                        .as_ref()
                        .is_none_or(|state| state.has_managed_paragraph_index())
            });
        let scoped_valid = if scoped_preconditions {
            let _ = SCOPED_CANDIDATE_ATTEMPTS.try_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |value| Some(value.saturating_add(1)),
            );
            match project.validate_scoped_single_element_edit(&self.project, scope_start.element) {
                Ok(true) if validate_selection(&project, selection).is_ok() => {
                    let _ = SCOPED_CANDIDATE_ACCEPTS.try_update(
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        |value| Some(value.saturating_add(1)),
                    );
                    true
                }
                _ => false,
            }
        } else {
            false
        };
        if !scoped_valid {
            if requested_scope == MutationValidationScope::SingleParagraph {
                let _ = FULL_VALIDATION_FALLBACKS.try_update(
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                    |value| Some(value.saturating_add(1)),
                );
            }
            if let Some(preservation) = project.preservation.as_mut() {
                preservation
                    .reconcile_body_structure(
                        &self.project.elements,
                        &mut project.elements,
                        &generated_scene_heading_deletions,
                    )
                    .map_err(EditError::Preservation)?;
            }
            validate_full_candidate(&project)?;
            if let Some(preservation) = project.preservation.as_mut() {
                if !preservation.has_source_identity_use_index() {
                    preservation.rebuild_managed_paragraph_index();
                }
            }
            validate_selection(&project, selection)?;
        }
        if project == self.project && selection == self.selection {
            self.last_validation_was_scoped = scoped_valid;
            return Ok(());
        }
        let derived_invalidation = classify_derived_invalidation(&self.project, &project);
        match &derived_invalidation {
            PendingDerivedInvalidation::Scenes(scenes) => {
                self.deferred_derived_scenes
                    .retain(|scene| !scenes.contains(scene));
                self.failed_derived_scenes
                    .retain(|scene| !scenes.contains(scene));
                // A new relevant scoped invalidation permits the previously
                // failed complete candidate to be attempted again.
                self.failed_derived_complete = false;
            }
            PendingDerivedInvalidation::Complete => {
                self.deferred_derived_scenes.clear();
                self.failed_derived_complete = false;
                self.failed_derived_scenes.clear();
            }
            PendingDerivedInvalidation::None => {}
        }
        self.push_history(Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            generated_parentheticals: self.generated_parentheticals.clone(),
        });
        self.project = project;
        self.selection = selection;
        self.generated_parentheticals = generated;
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.last_validation_was_scoped = scoped_valid;
        self.pending_derived_invalidation
            .merge(derived_invalidation);
        Ok(())
    }

    fn push_history(&mut self, snapshot: Snapshot) {
        if self.history_limit == 0 {
            return;
        }
        self.history.push(snapshot);
        if self.history.len() > self.history_limit {
            self.history.remove(0);
        }
    }
}

fn classify_derived_invalidation(before: &Project, after: &Project) -> PendingDerivedInvalidation {
    // Structural edits are matched by stable identity, not by position. A
    // duplicate identity makes the before/after correspondence ambiguous, so
    // retain the safe complete fallback rather than guessing at ownership.
    let Some(before_by_id) = unique_elements_by_id(&before.elements) else {
        return PendingDerivedInvalidation::Complete;
    };
    let Some(after_by_id) = unique_elements_by_id(&after.elements) else {
        return PendingDerivedInvalidation::Complete;
    };

    let before_heading_order = before
        .elements
        .iter()
        .filter(|element| element.role == Role::SceneHeading)
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let after_heading_order = after
        .elements
        .iter()
        .filter(|element| element.role == Role::SceneHeading)
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let before_heading_set = before_heading_order
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let after_heading_set = after_heading_order.iter().copied().collect::<BTreeSet<_>>();

    // Existing scene boundaries changing relative order cannot be assigned a
    // reliable local split/join mapping. Insertions and removals remain
    // classifiable through each Character cue's before/after owner.
    let before_common_heading_order = before_heading_order
        .iter()
        .copied()
        .filter(|id| after_heading_set.contains(id))
        .collect::<Vec<_>>();
    let after_common_heading_order = after_heading_order
        .iter()
        .copied()
        .filter(|id| before_heading_set.contains(id))
        .collect::<Vec<_>>();
    if before_common_heading_order != after_common_heading_order {
        return PendingDerivedInvalidation::Complete;
    }

    // The ordered Character identity/text projection is exactly the input
    // whose changes can affect a scene's generated cast and arc data. Building
    // it on both sides also catches cue insertion/deletion, role transitions,
    // cue reorder, and ownership transfer caused by heading topology edits.
    let Some(before_character_inputs) = character_projection_inputs(&before.elements) else {
        return PendingDerivedInvalidation::Complete;
    };
    let Some(after_character_inputs) = character_projection_inputs(&after.elements) else {
        return PendingDerivedInvalidation::Complete;
    };

    let mut affected = after_heading_set
        .difference(&before_heading_set)
        .map(|heading| (*heading).to_owned())
        .collect::<BTreeSet<_>>();
    let owners = before_character_inputs
        .keys()
        .chain(after_character_inputs.keys())
        .collect::<BTreeSet<_>>();
    for owner in owners {
        if before_character_inputs.get(owner) != after_character_inputs.get(owner)
            && after_heading_set.contains(owner.as_str())
        {
            affected.insert(owner.clone());
        }
    }

    // Preserve the established single-element behavior for heading text
    // changes. Newly-created/role-transitioned headings are already included
    // above; removed headings cannot be refreshed because they are no longer
    // live scene owners.
    for (id, old) in &before_by_id {
        if let Some(new) = after_by_id.get(id) {
            if old.role == Role::SceneHeading
                && new.role == Role::SceneHeading
                && old.text != new.text
            {
                affected.insert(id.clone());
            }
        }
    }

    if affected.is_empty() {
        PendingDerivedInvalidation::None
    } else {
        PendingDerivedInvalidation::Scenes(affected)
    }
}

fn unique_elements_by_id(
    elements: &[ScreenplayElement],
) -> Option<BTreeMap<String, &ScreenplayElement>> {
    let mut by_id = BTreeMap::new();
    for element in elements {
        if by_id
            .insert(element.id.as_str().to_owned(), element)
            .is_some()
        {
            return None;
        }
    }
    Some(by_id)
}

fn character_projection_inputs(
    elements: &[ScreenplayElement],
) -> Option<BTreeMap<String, Vec<(String, String)>>> {
    let mut by_scene = BTreeMap::<String, Vec<(String, String)>>::new();
    for (index, element) in elements.iter().enumerate() {
        if element.role != Role::Character {
            continue;
        }
        let owner = owning_scene_id(elements, index)?;
        by_scene
            .entry(owner)
            .or_default()
            .push((element.id.as_str().to_owned(), element.text.clone()));
    }
    Some(by_scene)
}

fn owning_scene_id(elements: &[ScreenplayElement], element_index: usize) -> Option<String> {
    elements
        .get(..=element_index)?
        .iter()
        .rev()
        .find(|element| element.role == Role::SceneHeading)
        .map(|heading| heading.id.as_str().to_owned())
}

fn map_selection_to_project(
    selection: Selection,
    project: &Project,
) -> Result<Selection, EditError> {
    fn map_position(mut position: Position, project: &Project) -> Result<Position, EditError> {
        let element = project
            .elements
            .get(position.element)
            .ok_or(EditError::InvalidSelection)?;
        position.offset = position.offset.min(element.text.len());
        while !element.text.is_char_boundary(position.offset) {
            position.offset = position.offset.saturating_sub(1);
        }
        Ok(position)
    }

    Ok(Selection {
        anchor: map_position(selection.anchor, project)?,
        focus: map_position(selection.focus, project)?,
    })
}

fn invalidate_changed_generated_parentheticals(
    before: &Project,
    after: &Project,
    prior_generated: &BTreeSet<String>,
    generated: &mut BTreeSet<String>,
) {
    for id in prior_generated {
        if !generated.contains(id) {
            continue;
        }
        let Some(before_element) = before
            .elements
            .iter()
            .find(|element| element.id.as_str() == id)
        else {
            continue;
        };
        let Some(after_element) = after
            .elements
            .iter()
            .find(|element| element.id.as_str() == id)
        else {
            continue;
        };
        if before_element != after_element {
            generated.remove(id);
        }
    }
}

fn validate_position(project: &Project, position: Position) -> Result<(), EditError> {
    let element = project
        .elements
        .get(position.element)
        .ok_or(EditError::InvalidElement)?;
    if element.is_boundary(position.offset) {
        Ok(())
    } else {
        Err(EditError::InvalidUtf8Boundary)
    }
}

fn validate_selection(project: &Project, selection: Selection) -> Result<(), EditError> {
    validate_position(project, selection.anchor)?;
    validate_position(project, selection.focus)
}

fn replace_range(
    project: &mut Project,
    start: Position,
    end: Position,
    inserted: &str,
) -> Result<(), EditError> {
    validate_position(project, start)?;
    validate_position(project, end)?;
    if (start.element, start.offset) > (end.element, end.offset) {
        return Err(EditError::InvalidSelection);
    }
    if inserted.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    if start.element == end.element {
        replace_element_text(project, start.element, start.offset, end.offset, inserted)?;
        return Ok(());
    }
    // The transaction-level preservation reconciliation retains the first ID,
    // removes the consumed source paragraphs and carries source runs forward.
    let prefix = project.elements[start.element].text[..start.offset].to_owned();
    let suffix = project.elements[end.element].text[end.offset..].to_owned();
    let combined = format!("{prefix}{inserted}{suffix}");
    if combined.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let runs = replace_cross_element_runs(project, start, end, inserted, &combined)?;
    project.elements[start.element].text = combined;
    project.elements[start.element].runs = runs;
    project.elements.drain(start.element + 1..=end.element);
    Ok(())
}

fn replace_element_text(
    project: &mut Project,
    element_index: usize,
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<(), EditError> {
    let item = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?;
    let old_text = item.text.clone();
    let old_runs = item.runs.clone();
    let segments = if old_runs.is_empty() {
        vec![old_text.clone()]
    } else {
        old_runs.iter().map(|run| run.text.clone()).collect()
    };
    let updated_segments = replace_text_segments(&segments, start, end, inserted)?;
    let new_text = updated_segments.concat();
    if new_text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let id = item.id.clone();
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        let expected_run_text = old_runs
            .iter()
            .map(|run| run.text.clone())
            .collect::<Vec<_>>();
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                crate::preservation::PreservationError::ManagedAnchorMissing,
            ))?
            .replace_managed_text(
                &id,
                &old_text,
                start,
                end,
                inserted,
                (!old_runs.is_empty()).then_some(expected_run_text.as_slice()),
            )
            .map_err(EditError::Preservation)?;
    } else if item.source_provenance.is_some() {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::ManagedAnchorMissing,
        ));
    }
    let item = &mut project.elements[element_index];
    item.text = new_text;
    if item.text.is_empty() {
        item.runs.clear();
    } else if !old_runs.is_empty() {
        for (run, text) in item.runs.iter_mut().zip(updated_segments) {
            run.text = text;
        }
    }
    Ok(())
}

fn replace_text_segments(
    segments: &[String],
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<Vec<String>, EditError> {
    let total = segments.iter().map(String::len).sum::<usize>();
    if start > end || end > total {
        return Err(EditError::InvalidSelection);
    }
    let mut insertion_segment = None;
    let mut base = 0usize;
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        if start < limit || (start == total && limit == total) {
            insertion_segment = Some(index);
            break;
        }
        base = limit;
    }
    let insertion_segment = insertion_segment
        .or_else(|| segments.len().checked_sub(1))
        .ok_or(EditError::InvalidSelection)?;
    let mut base = 0usize;
    let mut updated = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        let before_len = start.saturating_sub(base).min(segment.len());
        let after_start = end.saturating_sub(base).min(segment.len());
        let mut value = segment[..before_len].to_owned();
        if index == insertion_segment {
            value.push_str(inserted);
        }
        if end < limit || end <= base {
            value.push_str(&segment[after_start..]);
        }
        updated.push(value);
        base = limit;
    }
    Ok(updated)
}

fn replace_cross_element_runs(
    project: &Project,
    start: Position,
    end: Position,
    inserted: &str,
    combined: &str,
) -> Result<Vec<TextRun>, EditError> {
    let left = &project.elements[start.element];
    let right = &project.elements[end.element];
    if left.runs.is_empty() && right.runs.is_empty() {
        return Ok(Vec::new());
    }
    let left_runs = effective_runs(left);
    let right_runs = effective_runs(right);
    let mut result = slice_runs(&left_runs, 0, start.offset);
    let insertion_attributes = run_attributes_at(&left_runs, start.offset);
    push_run(&mut result, inserted, insertion_attributes);
    let suffix_runs = if start.element == end.element {
        slice_runs(&left_runs, end.offset, left.text.len())
    } else {
        slice_runs(&right_runs, end.offset, right.text.len())
    };
    result.extend(suffix_runs);
    coalesce_runs(&mut result);
    let actual = result
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    if actual != combined {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::UnsupportedTextMapping,
        ));
    }
    Ok(result)
}

fn effective_runs(element: &ScreenplayElement) -> Vec<TextRun> {
    if element.runs.is_empty() {
        vec![TextRun {
            text: element.text.clone(),
            attributes: Vec::new(),
        }]
    } else {
        element.runs.clone()
    }
}

fn slice_runs(runs: &[TextRun], start: usize, end: usize) -> Vec<TextRun> {
    let mut result = Vec::new();
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        let from = start.saturating_sub(base).min(run.text.len());
        let to = end.saturating_sub(base).min(run.text.len());
        if from < to {
            result.push(TextRun {
                text: run.text[from..to].to_owned(),
                attributes: run.attributes.clone(),
            });
        }
        base = limit;
        if base >= end {
            break;
        }
    }
    result
}

fn run_attributes_at(runs: &[TextRun], offset: usize) -> Vec<crate::RunAttribute> {
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        if offset < limit || (offset == limit && limit == runs.iter().map(|r| r.text.len()).sum()) {
            return run.attributes.clone();
        }
        base = limit;
    }
    Vec::new()
}

fn push_run(runs: &mut Vec<TextRun>, text: &str, attributes: Vec<crate::RunAttribute>) {
    if text.is_empty() {
        return;
    }
    runs.push(TextRun {
        text: text.to_owned(),
        attributes,
    });
}

fn coalesce_runs(runs: &mut Vec<TextRun>) {
    let mut coalesced: Vec<TextRun> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let Some(previous) = coalesced.last_mut() {
            if previous.attributes == run.attributes {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        coalesced.push(run);
    }
    *runs = coalesced;
}

fn update_preserved_role(
    project: &mut Project,
    element_index: usize,
    old_role: Role,
    new_role: Role,
) -> Result<(), EditError> {
    let element = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?;
    let id = element.id.clone();
    let old_fdx_role = fdx_role_name(old_role);
    if element
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_role.as_deref())
        .is_some_and(|source_role| source_role != old_fdx_role)
    {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::UnsupportedRoleMapping,
        ));
    }
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                crate::preservation::PreservationError::ManagedAnchorMissing,
            ))?
            .change_managed_role(&id, old_fdx_role, fdx_role_name(new_role))
            .map_err(EditError::Preservation)?;
    } else if element.source_provenance.is_some() {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::ManagedAnchorMissing,
        ));
    }
    if let Some(provenance) = &mut project.elements[element_index].source_provenance {
        provenance.foreign_role = Some(fdx_role_name(new_role).to_owned());
    }
    Ok(())
}

fn fdx_role_name(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "Scene Heading",
        Role::Action => "Action",
        Role::Character => "Character",
        Role::Dialogue => "Dialogue",
        Role::Parenthetical => "Parenthetical",
        Role::Transition => "Transition",
        Role::Note => "Note",
        Role::NewAct => "New Act",
        Role::EndOfAct => "End of Act",
        Role::CastList => "Cast List",
        Role::General => "General",
        Role::Shot => "Shot",
        Role::Sequence => "Sequence",
        Role::Outline1 => "Outline 1",
        Role::Outline2 => "Outline 2",
        Role::Outline3 => "Outline 3",
        Role::Summary => "Summary",
    }
}

fn strict_boundary_rejects(
    project: &Project,
    policy: &EditPolicy,
    element: usize,
    role: Role,
    text_len: usize,
    offset: usize,
) -> bool {
    policy.strict_transition_boundaries
        && matches!(
            role,
            Role::SceneHeading | Role::Character | Role::Transition
        )
        && project.elements.get(element).is_some()
        && offset != text_len
}

#[derive(Debug, Clone, Copy)]
struct EnterDecision {
    target: ParagraphKeyTarget,
    action: EnterAction,
}

#[derive(Debug, Clone, Copy)]
enum EnterAction {
    NoOp,
    InsertParentheticalBefore { element: usize, automatic: bool },
    ClearGeneratedAndSplit { position: Position, role: Role },
    RepairAndInsertDialogue { element: usize },
    Split { position: Position, role: Role },
}

#[derive(Debug, Clone, Copy)]
struct TabDecision {
    target: ParagraphKeyTarget,
    action: TabAction,
}

#[derive(Debug, Clone, Copy)]
enum TabAction {
    NoOp,
    DelimiterParentheticalToDialogue { element: usize },
    RepairAndInsertDialogue { element: usize },
    SetRole {
        position: Position,
        role: Role,
        clear_generated_parenthetical: bool,
    },
}

/// The shared Enter decision is intentionally value-only. Editing executes its
/// action after this point; hint projection reads only `target`.
fn resolve_enter(
    project: &Project,
    selection: Selection,
    generated: &BTreeSet<String>,
    policy: &EditPolicy,
) -> Result<EnterDecision, EditError> {
    let (start, end) = selection.ordered();
    if start != end {
        return Err(EditError::InvalidSelection);
    }
    let item = project
        .elements
        .get(start.element)
        .ok_or(EditError::InvalidElement)?;
    if strict_boundary_rejects(
        project,
        policy,
        start.element,
        item.role,
        item.text.len(),
        start.offset,
    ) {
        return Ok(EnterDecision {
            target: ParagraphKeyTarget::NoOp,
            action: EnterAction::NoOp,
        });
    }

    let mut position = start;
    if item.role == Role::Parenthetical {
        if position.offset == 0 && item.text.starts_with('(') {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(Role::Parenthetical),
                action: EnterAction::InsertParentheticalBefore {
                    element: position.element,
                    automatic: policy.auto_parenthetical,
                },
            });
        }
        if empty_parenthetical_at(item.text.as_str(), position.offset)
            && generated.contains(item.id.as_str())
        {
            position.offset = 0;
            let role = enter_transition(item.role)
                .ok_or(EditError::UnsupportedTransition { from: item.role })?;
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(role),
                action: EnterAction::ClearGeneratedAndSplit { position, role },
            });
        }
        if parenthetical_repair_boundary(item.text.as_str(), position.offset).is_some() {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(Role::Dialogue),
                action: EnterAction::RepairAndInsertDialogue {
                    element: position.element,
                },
            });
        }
        if !item.text.is_empty() {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::NoOp,
                action: EnterAction::NoOp,
            });
        }
    }

    let role = if item.role == Role::Dialogue && position.offset < item.text.len() {
        Role::Dialogue
    } else {
        enter_transition(item.role)
            .ok_or(EditError::UnsupportedTransition { from: item.role })?
    };
    Ok(EnterDecision {
        target: ParagraphKeyTarget::Target(role),
        action: EnterAction::Split { position, role },
    })
}

/// The shared Tab decision preserves consumed no-ops and same-role displayable
/// targets separately. Its action is also the sole source for real Tab edits.
fn resolve_tab(
    project: &Project,
    selection: Selection,
    generated: &BTreeSet<String>,
    policy: &EditPolicy,
) -> Result<TabDecision, EditError> {
    let (start, end) = selection.ordered();
    if start != end {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::Unavailable,
            action: TabAction::NoOp,
        });
    }
    let item = project
        .elements
        .get(start.element)
        .ok_or(EditError::InvalidElement)?;
    if strict_boundary_rejects(
        project,
        policy,
        start.element,
        item.role,
        item.text.len(),
        start.offset,
    ) {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        });
    }

    let mut position = start;
    let mut clear_generated_parenthetical = false;
    if policy.auto_parenthetical && item.role == Role::Parenthetical {
        if is_delimiter_only_parenthetical(item.text.as_str()) {
            return Ok(TabDecision {
                target: ParagraphKeyTarget::Target(Role::Dialogue),
                action: TabAction::DelimiterParentheticalToDialogue {
                    element: position.element,
                },
            });
        }
        if empty_parenthetical_at(item.text.as_str(), position.offset)
            && generated.contains(item.id.as_str())
        {
            position.offset = 0;
            clear_generated_parenthetical = true;
        } else if parenthetical_repair_boundary(item.text.as_str(), position.offset).is_some() {
            return Ok(TabDecision {
                target: ParagraphKeyTarget::Target(Role::Dialogue),
                action: TabAction::RepairAndInsertDialogue {
                    element: position.element,
                },
            });
        } else if !item.text.is_empty() {
            return Ok(TabDecision {
                target: ParagraphKeyTarget::NoOp,
                action: TabAction::NoOp,
            });
        }
    }

    let role = tab_transition(item.role, item.text.is_empty());
    Ok(match role {
        Some(role) => TabDecision {
            target: ParagraphKeyTarget::Target(role),
            action: TabAction::SetRole {
                position,
                role,
                clear_generated_parenthetical,
            },
        },
        None => TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        },
    })
}

fn empty_parenthetical_at(text: &str, offset: usize) -> bool {
    text == "()" && offset <= 2
}

fn is_delimiter_only_parenthetical(text: &str) -> bool {
    matches!(text, "(" | "()" | ")")
}

fn parenthetical_repair_boundary(text: &str, offset: usize) -> Option<usize> {
    let closed_pair_inner_end = text.starts_with('(')
        && text.ends_with(')')
        && text.len() > 2
        && offset == text.len().saturating_sub(1);
    let accepted = closed_pair_inner_end
        || (text.starts_with('(') && !text.ends_with(')') && text.len() > 1)
        || (!text.starts_with('(') && text.ends_with(')') && text.len() > 1)
        || (!text.starts_with('(') && !text.ends_with(')') && !text.is_empty());
    let at_supported_boundary = closed_pair_inner_end || offset == text.len();
    if !accepted || !at_supported_boundary {
        return None;
    }
    Some(match (text.starts_with('('), text.ends_with(')')) {
        (true, false) => text.len() + 1,
        (false, true) => text.len(),
        (false, false) => text.len() + 1,
        (true, true) => text.len(),
    })
}

fn repair_parenthetical(text: &str) -> String {
    match (text.starts_with('('), text.ends_with(')')) {
        (true, false) => format!("{text})"),
        (false, true) => format!("({text}"),
        (false, false) => format!("({text})"),
        (true, true) => text.to_owned(),
    }
}

fn insert_dialogue_below(project: &mut Project, element: usize) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let new = ScreenplayElement::new(split_id(project)?, Role::Dialogue, "")?;
    project.elements.insert(element + 1, new);
    Ok(Selection::caret(Position {
        element: element + 1,
        offset: 0,
    }))
}

fn split_at(project: &mut Project, position: Position, role: Role) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let item = project
        .elements
        .get(position.element)
        .ok_or(EditError::InvalidElement)?
        .clone();
    let left = item.text[..position.offset].to_owned();
    let right = item.text[position.offset..].to_owned();
    let old_runs = effective_runs(&item);
    let left_runs = slice_runs(&old_runs, 0, position.offset);
    let right_runs = slice_runs(&old_runs, position.offset, item.text.len());
    let id = split_id(project)?;
    let new_item = ScreenplayElement::new(id, role, right)?;
    project.elements[position.element].text = left;
    if !item.runs.is_empty() {
        project.elements[position.element].runs = left_runs;
    }
    let mut new_item = new_item;
    if !item.runs.is_empty() {
        new_item.runs = right_runs;
    }
    project.elements.insert(position.element + 1, new_item);
    Ok(Selection::caret(Position {
        element: position.element + 1,
        offset: 0,
    }))
}

fn split_id(project: &Project) -> Result<String, ModelError> {
    Ok(project.allocate_element_id()?.as_str().to_owned())
}

fn join_at(project: &mut Project, index: usize) -> Result<(), EditError> {
    if index + 1 >= project.elements.len() {
        return Err(EditError::InvalidElement);
    }
    let removed_id = &project.elements[index + 1].id;
    if project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_direct_paragraph_formatting(removed_id))
    {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::StructuralAttachmentConflict,
        ));
    }
    let right = project.elements[index + 1].text.clone();
    let joined_runs =
        if project.elements[index].runs.is_empty() && project.elements[index + 1].runs.is_empty() {
            Vec::new()
        } else {
            let mut runs = effective_runs(&project.elements[index]);
            runs.extend(effective_runs(&project.elements[index + 1]));
            coalesce_runs(&mut runs);
            runs
        };
    let item = project
        .elements
        .get_mut(index)
        .ok_or(EditError::InvalidElement)?;
    if item.text.len().saturating_add(right.len()) > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    item.text.push_str(&right);
    item.runs = joined_runs;
    project.elements.remove(index + 1);
    Ok(())
}

fn previous_boundary(text: &str, offset: usize) -> usize {
    text[..offset]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .chars()
        .next()
        .map_or(offset, |character| offset + character.len_utf8())
}

/// Compatibility alias for the shared native ReturnKey application default.
pub fn transition(role: Role) -> Option<Role> {
    enter_transition(role)
}

pub fn enter_transition(role: Role) -> Option<Role> {
    role_for_native_behavior_target(
        crate::formatting::formatting_for_role(role)
            .behavior
            .return_key,
    )
}

/// Returns the shared native TabKey application default, with the established
/// empty-Character condition retained as an application-specific boundary.
pub fn tab_transition(role: Role, current_text_is_empty: bool) -> Option<Role> {
    if role == Role::Character && !current_text_is_empty {
        return None;
    }
    role_for_native_behavior_target(
        crate::formatting::formatting_for_role(role)
            .behavior
            .tab_key,
    )
}

fn role_for_native_behavior_target(target: &str) -> Option<Role> {
    crate::formatting::formatting_in_native_order()
        .iter()
        .find(|formatting| formatting.fdx_label == target)
        .map(|formatting| formatting.role)
}
