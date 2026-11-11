//! Headless, transactional editing over the semantic screenplay model.

use crate::{
    cast_list::{self, CastListError},
    clear_paragraph_run_styles, project_style_at_caret, set_paragraph_alignments,
    toggle_paragraph_run_styles,
    validate_layout_inputs, LayoutInputs, ModelError, ParagraphAlignment, ParagraphTarget, Project,
    Role, RunBoundaryAffinity, RunStyle, RunStyleFlags, ScreenplayElement, StableId, TextRun,
    TitlePageParagraph, MAX_ELEMENTS, MAX_TEXT_BYTES,
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
    /// Inline Alts is valid only while its exact source owner remains Dialogue.
    /// This failure is intentionally not eligible for automatic dual unwrap.
    InlineAltsRoleChange,
    Preservation(crate::preservation::PreservationError),
    CastList(CastListError),
    FormattedText(crate::FormattedTextError),
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

impl From<crate::FormattedTextError> for EditError {
    fn from(error: crate::FormattedTextError) -> Self {
        Self::FormattedText(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    // Project includes semantic runs and the complete preservation graph, so
    // undo/redo restores both authorities rather than only the text projection.
    project: Project,
    selection: Selection,
    title_selection: Selection,
    title_typing_style: RunStyleFlags,
    body_typing_style: RunStyleFlags,
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
    title_selection: Selection,
    title_typing_style: RunStyleFlags,
    body_typing_style: RunStyleFlags,
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
            && self.title_selection == other.title_selection
            && self.title_typing_style == other.title_typing_style
            && self.body_typing_style == other.body_typing_style
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
        let body_typing_style = project.elements.first().map_or(Ok(RunStyleFlags::default()), |element| {
            project_style_at_caret(
                &element.text,
                &element.runs,
                0,
                RunBoundaryAffinity::Right,
                RunStyleFlags::default(),
            ).map_err(EditError::FormattedText)
        })?;
        let title_typing_style = project.title_page.first().map_or(Ok(RunStyleFlags::default()), |paragraph| {
            project_style_at_caret(
                &paragraph.text,
                &paragraph.runs,
                0,
                RunBoundaryAffinity::Right,
                RunStyleFlags::default(),
            ).map_err(EditError::FormattedText)
        })?;
        Ok(Self {
            clean_project: project.clone(),
            project,
            session_open_timestamp,
            policy,
            layout_inputs,
            selection,
            title_selection: Selection::caret(Position { element: 0, offset: 0 }),
            title_typing_style,
            body_typing_style,
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
    /// Transient typing styles for a collapsed body caret. This state is
    /// session-only and does not participate in document dirty tracking.
    pub fn body_typing_style(&self) -> RunStyleFlags {
        self.body_typing_style
    }
    /// Current title-page-local selection. Its paragraph indexes are never
    /// interpreted against the screenplay body vector.
    pub fn title_selection(&self) -> Selection {
        self.title_selection
    }
    /// Transient typing styles for a collapsed title-page caret. This state is
    /// session-only and does not participate in document dirty tracking.
    pub fn title_typing_style(&self) -> RunStyleFlags {
        self.title_typing_style
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
        validate_selection(&self.project, self.selection)?;
        validate_title_selection(&self.project, self.title_selection)
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
        let mut rebased = self.clone();
        let selection = map_selection_to_project(self.selection, &self.project, &project)
            .ok_or(EditError::InvalidSelection)?;
        let title_selection = map_title_selection_to_project(
            self.title_selection,
            &self.project,
            &project,
        );
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
        rebased.title_selection = title_selection;
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
        if selection.anchor == selection.focus {
            let element = &self.project.elements[selection.focus.element];
            self.body_typing_style = project_style_at_caret(
                &element.text,
                &element.runs,
                selection.focus.offset,
                RunBoundaryAffinity::Right,
                self.body_typing_style,
            ).map_err(EditError::FormattedText)?;
        }
        Ok(())
    }

    /// Sets a title-page-local selection without affecting the body caret,
    /// history, revision, or dirty state.
    pub fn set_title_selection(&mut self, selection: Selection) -> Result<(), EditError> {
        validate_title_selection(&self.project, selection)?;
        self.title_selection = selection;
        if selection.anchor == selection.focus {
            if let Some(paragraph) = self.project.title_page.get(selection.focus.element) {
                self.title_typing_style = project_style_at_caret(
                    &paragraph.text,
                    &paragraph.runs,
                    selection.focus.offset,
                    RunBoundaryAffinity::Right,
                    self.title_typing_style,
                ).map_err(EditError::FormattedText)?;
            }
        }
        Ok(())
    }

    /// Inserts or replaces canonical title text in one paragraph. Paste is an
    /// alias so one clipboard operation always creates one history entry.
    pub fn insert_title(&mut self, text: &str) -> Result<(), EditError> {
        self.mutate_title(|project, selection, typing| {
            let (start, end) = selection.ordered();
            let insertion_style = if start == end {
                *typing
            } else {
                let paragraph = project.title_page.get(start.element).ok_or(EditError::InvalidElement)?;
                project_style_at_caret(
                    &paragraph.text,
                    &paragraph.runs,
                    start.offset,
                    RunBoundaryAffinity::Right,
                    *typing,
                ).map_err(EditError::FormattedText)?
            };
            replace_title_range(project, start, end, text, insertion_style)?;
            Ok(Selection::caret(Position {
                element: start.element,
                offset: start.offset.saturating_add(text.len()),
            }))
        })
    }

    pub fn paste_title(&mut self, text: &str) -> Result<(), EditError> {
        self.insert_title(text)
    }

    /// Deletes a title selection, or the next Unicode scalar at a collapsed
    /// caret. Structural source reconciliation is staged with the semantic edit.
    pub fn delete_title(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, typing| {
            let (start, end) = selection.ordered();
            if start != end {
                replace_title_range(project, start, end, "", *typing)?;
                return Ok(Selection::caret(start));
            }
            let paragraph_text = project
                .title_page
                .get(start.element)
                .ok_or(EditError::InvalidElement)?
                .text
                .clone();
                if start.offset == paragraph_text.len() {
                    if start.element + 1 < project.title_page.len() {
                    join_title_at(project, start.element)?;
                }
                return Ok(*selection);
            }
            let next = next_boundary(&paragraph_text, start.offset);
            let finish = Position { offset: next, ..start };
            replace_title_range(project, start, finish, "", *typing)?;
            Ok(Selection::caret(start))
        })
    }

    pub fn backspace_title(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, typing| {
            let (start, end) = selection.ordered();
            if start != end {
                replace_title_range(project, start, end, "", *typing)?;
                return Ok(Selection::caret(start));
            }
            let paragraph_text = project
                .title_page
                .get(start.element)
                .ok_or(EditError::InvalidElement)?
                .text
                .clone();
            if start.offset == 0 {
                if start.element > 0 {
                    let left = start.element - 1;
                    let offset = project.title_page[left].text.len();
                    join_title_at(project, left)?;
                    return Ok(Selection::caret(Position { element: left, offset }));
                }
                return Ok(*selection);
            }
            let previous = previous_boundary(&paragraph_text, start.offset);
            let from = Position { offset: previous, ..start };
            replace_title_range(project, from, start, "", *typing)?;
            Ok(Selection::caret(from))
        })
    }

    pub fn delete_forward_title(&mut self) -> Result<(), EditError> {
        self.delete_title()
    }

    /// Read-only resolution for the current selection on the requested surface.
    /// TitlePage is always ineligible for the Body-only dual-dialogue toggle.
    pub fn dual_dialogue_toggle_target(
        &self,
        surface: crate::dual_dialogue::SelectionSurface,
    ) -> Result<Option<crate::dual_dialogue::ToggleTarget>, EditError> {
        let selection = match surface {
            crate::dual_dialogue::SelectionSurface::Body => self.selection,
            crate::dual_dialogue::SelectionSurface::TitlePage => self.title_selection,
        };
        crate::dual_dialogue::resolve_toggle_target(&self.project, surface, selection)
            .map_err(EditError::Model)
    }

    /// Atomically enables a valid ungrouped dual sequence or disables the group
    /// containing the collapsed Body caret. An ineligible selection is a no-op.
    pub fn toggle_dual_dialogue(&mut self) -> Result<(), EditError> {
        let Some(target) = self.dual_dialogue_toggle_target(
            crate::dual_dialogue::SelectionSurface::Body,
        )? else {
            return Ok(());
        };
        let mut project = self.project.clone();
        match target {
            crate::dual_dialogue::ToggleTarget::Enable(pattern) => {
                let candidate_start = pattern.flat_range.start;
                let group_id = project.allocate_element_id()?;
                let member_ids = pattern.left.member_ids()
                    .chain(pattern.right.member_ids())
                    .cloned()
                    .collect::<Vec<_>>();
                let mut wrapper = crate::dual_dialogue::DualDialogueWrapperProvenance::default();
                if let Some(preservation) = project.preservation.as_mut() {
                    let (foreign_id, anchor, ordinal) = preservation
                        .wrap_dual_dialogue_members(&group_id, &member_ids, None)
                        .map_err(EditError::Preservation)?;
                    wrapper.outer_foreign_id = Some(foreign_id.clone());
                    wrapper.source = Some(crate::SourceProvenance {
                        format: crate::SourceFormat::Fdx,
                        section: crate::SourceSection::Content,
                        ordinal,
                        foreign_id: Some(foreign_id),
                        foreign_role: Some("General".to_owned()),
                    });
                    wrapper.preservation_anchor = Some(anchor);
                }
                let group = crate::dual_dialogue::DualDialogueGroup {
                    id: group_id,
                    left: pattern.left,
                    right: pattern.right,
                    wrapper,
                };
                let insertion = project.dual_dialogue_groups.iter().position(|existing| {
                    crate::dual_dialogue::first_member(&project, existing)
                        .is_some_and(|index| index > candidate_start)
                }).unwrap_or(project.dual_dialogue_groups.len());
                project.dual_dialogue_groups.insert(insertion, group);
            }
            crate::dual_dialogue::ToggleTarget::Disable(group_id) => {
                let group_index = project.dual_dialogue_groups.iter()
                    .position(|group| group.id == group_id)
                    .ok_or(EditError::InvalidSelection)?;
                let group = project.dual_dialogue_groups[group_index].clone();
                if let Some(preservation) = project.preservation.as_mut() {
                    let anchor = group.wrapper.preservation_anchor.as_ref()
                        .ok_or(EditError::Preservation(
                            crate::preservation::PreservationError::ManagedAnchorMissing,
                        ))?;
                    let members = group.member_ids().cloned().collect::<Vec<_>>();
                    preservation
                        .unwrap_dual_dialogue_members(
                            &group.id,
                            &members,
                            anchor,
                            group.wrapper.outer_foreign_id.as_deref(),
                        )
                        .map_err(EditError::Preservation)?;
                }
                project.dual_dialogue_groups.remove(group_index);
                project.dual_dialogue_paragraph_overrides.remove(&group_id);
            }
        }

        validate_full_candidate(&project)?;
        if let Some(preservation) = project.preservation.as_mut() {
            preservation.rebuild_managed_paragraph_index();
        }
        validate_selection(&project, self.selection)?;
        let snapshot = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.push_history(snapshot);
        self.project = project;
        self.redo.clear();
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

    /// Title Enter inserts a new role-less paragraph. Source-backed structure
    /// is reconciled transactionally against its preserved TitlePage Content.
    pub fn split_title(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, _typing| {
            let (start, end) = selection.ordered();
            if start != end {
                return Err(EditError::InvalidSelection);
            }
            let original = project
                .title_page
                .get(start.element)
                .cloned()
                .ok_or(EditError::InvalidElement)?;
            let id = project.allocate_element_id()?;
            let source_runs = effective_title_runs(&original);
            let original_len = original.text.len();
            let mut left = original.clone();
            let mut right = original;
            left.text = left.text[..start.offset].to_owned();
            right.id = id;
            right.text = right.text[start.offset..].to_owned();
            left.runs = slice_runs(&source_runs, 0, start.offset);
            right.runs = slice_runs(&source_runs, start.offset, original_len);
            right.source_provenance = None;
            project.title_page[start.element] = left;
            project.title_page.insert(start.element + 1, right);
            Ok(Selection::caret(Position { element: start.element + 1, offset: 0 }))
        })
    }

    pub fn enter_title(&mut self) -> Result<(), EditError> {
        self.split_title()
    }

    /// Title Tab is consumed without inserting a literal tab or changing
    /// screenplay roles.
    pub fn tab_title(&self) -> Result<(), EditError> {
        validate_title_selection(&self.project, self.title_selection)
    }

    /// Joins the title paragraph before the caret into the paragraph at the
    /// caret, retaining the preceding stable identity. Source-backed structure
    /// either reconciles wholly or leaves the live session unchanged.
    pub fn join_title_with_previous(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, _typing| {
            let (start, end) = selection.ordered();
            if start != end || start.offset != 0 || start.element == 0 {
                return Err(EditError::InvalidSelection);
            }
            let left_index = start.element - 1;
            let offset = project.title_page[left_index].text.len();
            join_title_at(project, left_index)?;
            Ok(Selection::caret(Position { element: left_index, offset }))
        })
    }

    /// Applies a run-style toggle to the body selection through the same
    /// canonical range engine used by title text.
    pub fn toggle_body_run_style(&mut self, style: RunStyle) -> Result<(), EditError> {
        self.toggle_surface_run_style(false, style)
    }

    pub fn toggle_title_run_style(&mut self, style: RunStyle) -> Result<(), EditError> {
        self.toggle_surface_run_style(true, style)
    }

    /// Clears all supported run formatting from the current Body selection in
    /// one transactional edit. Alignment and unknown source style tokens remain.
    pub fn clear_body_run_styles(&mut self) -> Result<(), EditError> {
        self.clear_surface_run_styles(false)
    }

    /// Clears all supported run formatting from the current Title Page selection
    /// in one transactional edit. Alignment and unknown source style tokens remain.
    pub fn clear_title_run_styles(&mut self) -> Result<(), EditError> {
        self.clear_surface_run_styles(true)
    }

    pub fn set_body_alignment(&mut self, alignment: ParagraphAlignment) -> Result<(), EditError> {
        self.set_surface_alignment(false, alignment)
    }

    pub fn set_title_alignment(&mut self, alignment: ParagraphAlignment) -> Result<(), EditError> {
        self.set_surface_alignment(true, alignment)
    }

    /// Inserts or replaces text while retaining the existing element identity.
    /// Mapped source runs and text leaves are transformed in the same candidate;
    /// unsafe inline intersections reject the whole operation.
    pub fn insert(&mut self, text: &str) -> Result<(), EditError> {
        let typing = self.body_typing_style;
        let (start, end) = self.selection.ordered();
        let operation = |project: &mut Project,
                         selection: &Selection,
                         _generated: &mut BTreeSet<String>,
                         _policy: &EditPolicy| {
                let (start, end) = selection.ordered();
                let insertion_style = if start == end {
                    typing
                } else {
                    let element = project.elements.get(start.element).ok_or(EditError::InvalidElement)?;
                    project_style_at_caret(
                        &element.text,
                        &element.runs,
                        start.offset,
                        RunBoundaryAffinity::Right,
                        typing,
                    ).map_err(EditError::FormattedText)?
                };
                replace_range(project, start, end, text)?;
                if !text.is_empty() {
                    synchronize_body_typing_style(project, start, text, insertion_style)?;
                }
                Ok(Selection::caret(Position {
                    element: start.element,
                    offset: start.offset + text.len(),
                }))
            };
        if start.element != end.element {
            self.mutate_structural(MutationValidationScope::SingleParagraph, operation)
        } else {
            self.mutate(MutationValidationScope::SingleParagraph, operation)
        }
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
        self.mutate_structural_with_scene_heading_deletions(
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
                } else if start.element + 1 < project.elements.len() {
                    let offset = item.text.len();
                    join_at(project, start.element)?;
                    return Ok(Selection::caret(Position {
                        element: start.element,
                        offset,
                    }));
                }
                Ok(Selection::caret(start))
            },
        )
    }

    pub fn backspace(&mut self) -> Result<(), EditError> {
        self.mutate_structural(
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
        self.mutate_structural(
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
        self.mutate_structural(
            MutationValidationScope::Complete,
            |project, selection, generated, policy| {
                let (start, end) = selection.ordered();
                if start == end
                    && project.elements.get(start.element).is_some_and(|item| {
                        item.role == Role::Dialogue && start.offset == item.text.len()
                    })
                {
                    let is_final_right_dialogue = crate::dual_dialogue::group_at_element_index(
                        project,
                        start.element,
                    ).is_some_and(|group| {
                        crate::dual_dialogue::last_member(project, group) == Some(start.element)
                            && group.right.paragraphs.last().is_some_and(|id| {
                                project.elements.get(start.element).is_some_and(|item| &item.id == id)
                            })
                    });
                    if is_final_right_dialogue {
                        let selection = split_at(project, start, Role::Action)?;
                        let action_id = project.elements.get(start.element + 1)
                            .ok_or(EditError::InvalidElement)?.id.clone();
                        crate::dual_dialogue::remove_staged_group_member(project, &action_id);
                        return Ok(selection);
                    }
                }
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
                        if let Some(anchor) = project.elements.get(element).map(|item| item.id.clone()) {
                            crate::dual_dialogue::insert_group_member_at_flat_index(
                                project,
                                element,
                                &anchor,
                                new.id.clone(),
                            )?;
                        }
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
        self.mutate_structural(
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
        self.mutate_structural(
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
        let (start, end) = self.selection.ordered();
        if start.element == end.element
            && self.project.elements.get(start.element).is_some_and(|element| {
                element.role == Role::Dialogue
                    && role != Role::Dialogue
                    && self.project.preservation.as_ref()
                        .is_some_and(|state| state.has_inline_alts(&element.id))
            })
        {
            // Reject before entering the structural planner: role changes are
            // not repaired by unwrapping an otherwise-valid dual group.
            return Err(EditError::InlineAltsRoleChange);
        }
        self.mutate_structural(
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
        validate_title_selection(&snapshot.project, snapshot.title_selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.history.pop();
        self.redo.push(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.title_selection = snapshot.title_selection;
        self.title_typing_style = snapshot.title_typing_style;
        self.body_typing_style = snapshot.body_typing_style;
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
        validate_title_selection(&snapshot.project, snapshot.title_selection)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.redo.pop();
        self.push_history(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.title_selection = snapshot.title_selection;
        self.title_typing_style = snapshot.title_typing_style;
        self.body_typing_style = snapshot.body_typing_style;
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
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.mutate_with_scene_heading_deletions(
            requested_scope,
            BTreeSet::new(),
            false,
            operation,
        )
    }

    fn mutate_structural<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        operation: F,
    ) -> Result<(), EditError>
    where
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.mutate_with_scene_heading_deletions(
            requested_scope,
            BTreeSet::new(),
            true,
            operation,
        )
    }

    fn mutate_structural_with_scene_heading_deletions<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        generated_scene_heading_deletions: BTreeSet<StableId>,
        operation: F,
    ) -> Result<(), EditError>
    where
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        self.mutate_with_scene_heading_deletions(
            requested_scope,
            generated_scene_heading_deletions,
            true,
            operation,
        )
    }

    fn mutate_with_scene_heading_deletions<F>(
        &mut self,
        requested_scope: MutationValidationScope,
        generated_scene_heading_deletions: BTreeSet<StableId>,
        structural: bool,
        mut operation: F,
    ) -> Result<(), EditError>
    where
        F: FnMut(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
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
        let baseline = self.project.clone();
        let baseline_generated = self.generated_parentheticals.clone();
        let mut first_project = baseline.clone();
        let mut first_generated = baseline_generated.clone();
        let first_result = operation(
            &mut first_project,
            &self.selection,
            &mut first_generated,
            &self.policy,
        );
        let (mut project, mut generated, selection) = match first_result {
            Ok(selection) => {
                crate::dual_dialogue::prune_removed_group_paragraphs(&baseline, &mut first_project);
                let plan = if structural {
                    crate::dual_dialogue::classify_structural_result(
                        &baseline,
                        self.selection,
                        &first_project,
                    )
                } else {
                    Vec::new()
                };
                let unwrap_ids = plan.into_iter().filter_map(|classification| {
                    matches!(
                        classification.outcome,
                        crate::dual_dialogue::StructuralGroupOutcome::Removed
                            | crate::dual_dialogue::StructuralGroupOutcome::InvalidRequiresUnwrap
                    ).then_some(classification.group_id)
                }).collect::<Vec<_>>();
                if unwrap_ids.is_empty() {
                    (first_project, first_generated, selection)
                } else {
                    let mut retry_project = baseline.clone();
                    unwrap_dual_dialogue_groups(&mut retry_project, &unwrap_ids)?;
                    let retry_selection = remap_selection_by_identity(
                        &baseline,
                        &retry_project,
                        self.selection,
                    )?;
                    let mut retry_generated = baseline_generated.clone();
                    let selection = operation(
                        &mut retry_project,
                        &retry_selection,
                        &mut retry_generated,
                        &self.policy,
                    )?;
                    (retry_project, retry_generated, selection)
                }
            }
            Err(error @ EditError::InlineAltsRoleChange) => return Err(error),
            Err(first_error) if structural => {
                let unwrap_ids = groups_touched_by_selection(&baseline, self.selection);
                if unwrap_ids.is_empty() {
                    return Err(first_error);
                }
                let mut retry_project = baseline.clone();
                unwrap_dual_dialogue_groups(&mut retry_project, &unwrap_ids)?;
                let retry_selection = remap_selection_by_identity(
                    &baseline,
                    &retry_project,
                    self.selection,
                )?;
                let mut retry_generated = baseline_generated.clone();
                let selection = operation(
                    &mut retry_project,
                    &retry_selection,
                    &mut retry_generated,
                    &self.policy,
                )?;
                (retry_project, retry_generated, selection)
            }
            Err(error) => return Err(error),
        };
        let prior_generated = baseline_generated;
        crate::dual_dialogue::prune_removed_group_paragraphs(&self.project, &mut project);
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
        let scoped_preconditions = !structural
            && self.validated_baseline
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
                        &project.dual_dialogue_groups,
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
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
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

    fn mutate_title<F>(&mut self, operation: F) -> Result<(), EditError>
    where
        F: FnOnce(
            &mut Project,
            &Selection,
            &mut RunStyleFlags,
        ) -> Result<Selection, EditError>,
    {
        let mut project = self.project.clone();
        let before_title = self.project.title_page.clone();
        let mut typing = self.title_typing_style;
        let selection = operation(&mut project, &self.title_selection, &mut typing)?;
        let before_ids = before_title.iter().map(|paragraph| &paragraph.id).collect::<Vec<_>>();
        let after_ids = project.title_page.iter().map(|paragraph| &paragraph.id).collect::<Vec<_>>();
        if before_ids != after_ids {
            if let Some(preservation) = project.preservation.as_mut() {
                preservation
                    .reconcile_title_structure(&before_title, &mut project.title_page)
                    .map_err(EditError::Preservation)?;
            } else if before_title.iter().any(|paragraph| paragraph.source_provenance.is_some())
                || project.title_page.iter().any(|paragraph| paragraph.source_provenance.is_some())
            {
                return Err(EditError::Preservation(
                    crate::preservation::PreservationError::ManagedAnchorMissing,
                ));
            }
        }
        if let Some(preservation) = project.preservation.as_mut() {
            if !preservation.has_source_identity_use_index() {
                preservation.rebuild_managed_paragraph_index();
            }
        }
        validate_full_candidate(&project)?;
        validate_title_selection(&project, selection)?;
        if project == self.project {
            self.title_selection = selection;
            self.title_typing_style = typing;
            self.last_validation_was_scoped = false;
            return Ok(());
        }
        self.push_history(Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            generated_parentheticals: self.generated_parentheticals.clone(),
        });
        self.project = project;
        self.title_selection = selection;
        self.title_typing_style = typing;
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        self.last_validation_was_scoped = false;
        // Title-only edits cannot affect screenplay scene-derived attachments.
        Ok(())
    }

    fn toggle_surface_run_style(
        &mut self,
        title_surface: bool,
        style: RunStyle,
    ) -> Result<(), EditError> {
        let selection = if title_surface { self.title_selection } else { self.selection };
        if selection.anchor == selection.focus {
            let position = selection.focus;
            let (text, runs, active) = if title_surface {
                let paragraph = self
                    .project
                    .title_page
                    .get(position.element)
                    .ok_or(EditError::InvalidElement)?;
                (paragraph.text.as_str(), paragraph.runs.as_slice(), self.title_typing_style)
            } else {
                let paragraph = self
                    .project
                    .elements
                    .get(position.element)
                    .ok_or(EditError::InvalidElement)?;
                (paragraph.text.as_str(), paragraph.runs.as_slice(), self.body_typing_style)
            };
            let current = project_style_at_caret(
                text,
                runs,
                position.offset,
                RunBoundaryAffinity::Right,
                active,
            )
            .map_err(EditError::FormattedText)?;
            let next = current.toggled(style);
            if title_surface {
                self.title_typing_style = next;
            } else {
                self.body_typing_style = next;
            }
            return Ok(());
        }

        let active = if title_surface { self.title_typing_style } else { self.body_typing_style };
        let ranges = selection_run_ranges(&self.project, selection, title_surface)?;
        if title_surface {
            self.mutate_title(|project, _, _| {
                toggle_paragraph_run_styles(project, &ranges, active, style)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        } else {
            self.mutate(MutationValidationScope::Complete, |project, _, _, _| {
                toggle_paragraph_run_styles(project, &ranges, active, style)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        }
    }

    fn clear_surface_run_styles(&mut self, title_surface: bool) -> Result<(), EditError> {
        let selection = if title_surface { self.title_selection } else { self.selection };
        if selection.anchor == selection.focus {
            if title_surface {
                self.project
                    .title_page
                    .get(selection.focus.element)
                    .ok_or(EditError::InvalidElement)?;
                self.title_typing_style = RunStyleFlags::default();
            } else {
                self.project
                    .elements
                    .get(selection.focus.element)
                    .ok_or(EditError::InvalidElement)?;
                self.body_typing_style = RunStyleFlags::default();
            }
            return Ok(());
        }

        let ranges = selection_run_ranges(&self.project, selection, title_surface)?;
        if title_surface {
            self.mutate_title(|project, _, typing| {
                clear_paragraph_run_styles(project, &ranges)
                    .map_err(EditError::FormattedText)?;
                *typing = RunStyleFlags::default();
                Ok(selection)
            })
        } else {
            self.mutate(MutationValidationScope::Complete, |project, _, _, _| {
                clear_paragraph_run_styles(project, &ranges)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })?;
            self.body_typing_style = RunStyleFlags::default();
            Ok(())
        }
    }

    fn set_surface_alignment(
        &mut self,
        title_surface: bool,
        alignment: ParagraphAlignment,
    ) -> Result<(), EditError> {
        let selection = if title_surface { self.title_selection } else { self.selection };
        let targets = selection_paragraph_targets(&self.project, selection, title_surface)?;
        if title_surface {
            self.mutate_title(|project, _, _| {
                set_paragraph_alignments(project, &targets, alignment)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        } else {
            self.mutate(MutationValidationScope::Complete, |project, _, _, _| {
                set_paragraph_alignments(project, &targets, alignment)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        }
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
    old_project: &Project,
    new_project: &Project,
) -> Option<Selection> {
    fn map_position(position: Position, old: &Project, new: &Project) -> Option<Position> {
        if new.elements.is_empty() {
            return None;
        }
        let old_element = old.elements.get(position.element);
        let old_source = old_element.and_then(|element| element.source_provenance.as_ref());
        let source_id = old_source.and_then(|provenance| provenance.foreign_id.as_deref());
        let index = if let Some(source_id) = source_id {
            if old.elements.iter().filter(|element| {
                element.source_provenance.as_ref()
                    .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(source_id)
            }).count() != 1 {
                return None;
            }
            let matches = new.elements.iter().enumerate().filter(|(_, element)| {
                element.source_provenance.as_ref().is_some_and(|provenance| {
                    provenance.foreign_id.as_deref() == Some(source_id)
                        && old_source.is_some_and(|old| {
                            provenance.format == old.format
                                && provenance.section == old.section
                                && provenance.foreign_role == old.foreign_role
                        })
                })
            }).map(|(index, _)| index).collect::<Vec<_>>();
            let [index] = matches.as_slice() else { return None };
            *index
        } else {
            let old_element = old_element?;
            let stable_matches = if old_source.is_none() {
                new.elements.iter().enumerate().filter(|(_, element)| {
                    element.id == old_element.id
                }).map(|(index, _)| index).collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if let [index] = stable_matches.as_slice() {
                *index
            } else {
                // Source-less projects acquire FDX identities during Save As.
                // Map them by a unique ordered semantic occurrence, never by a
                // bare paragraph index that could now refer to unrelated text.
                let old_occurrences = old.elements.iter().enumerate().filter(|(_, element)| {
                    element.role == old_element.role && element.text == old_element.text
                }).map(|(index, _)| index).collect::<Vec<_>>();
                let occurrence = old_occurrences.iter().position(|index| *index == position.element)?;
                let new_occurrences = new.elements.iter().enumerate().filter(|(_, element)| {
                    element.role == old_element.role && element.text == old_element.text
                }).map(|(index, _)| index).collect::<Vec<_>>();
                if old_occurrences.len() != new_occurrences.len() {
                    return None;
                }
                *new_occurrences.get(occurrence)?
            }
        };
        let element = &new.elements[index];
        let mut offset = position.offset.min(element.text.len());
        while !element.text.is_char_boundary(offset) {
            offset = offset.saturating_sub(1);
        }
        Some(Position { element: index, offset })
    }
    Some(Selection {
        anchor: map_position(selection.anchor, old_project, new_project)?,
        focus: map_position(selection.focus, old_project, new_project)?,
    })
}

fn map_title_selection_to_project(
    selection: Selection,
    old_project: &Project,
    new_project: &Project,
) -> Selection {
    fn map_position(position: Position, old: &Project, new: &Project) -> Position {
        if new.title_page.is_empty() {
            return Position { element: 0, offset: 0 };
        }
        let old_paragraph = old.title_page.get(position.element);
        let source_id = old_paragraph
            .and_then(|paragraph| paragraph.source_provenance.as_ref())
            .and_then(|provenance| provenance.foreign_id.as_deref());
        let unique_source_id = source_id.filter(|source_id| {
            old.title_page.iter().filter(|paragraph| {
                paragraph.source_provenance.as_ref()
                    .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(*source_id)
            }).count() == 1
                && new.title_page.iter().filter(|paragraph| {
                    paragraph.source_provenance.as_ref()
                        .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(*source_id)
                }).count() == 1
        });
        let index = unique_source_id
            .and_then(|source_id| new.title_page.iter().position(|paragraph| {
                paragraph.source_provenance.as_ref()
                    .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(source_id)
            }))
            .or_else(|| {
                let old_paragraph = old_paragraph?;
                if old_paragraph.source_provenance.is_some() {
                    return None;
                }
                new.title_page.iter().position(|paragraph| {
                    paragraph.id == old_paragraph.id
                        && paragraph.source_provenance.as_ref()
                            .is_none_or(|provenance| provenance.foreign_id.is_none())
                })
            })
            .unwrap_or_else(|| position.element.min(new.title_page.len() - 1));
        let paragraph = &new.title_page[index];
        let mut offset = position.offset.min(paragraph.text.len());
        while !paragraph.text.is_char_boundary(offset) {
            offset = offset.saturating_sub(1);
        }
        Position { element: index, offset }
    }
    Selection {
        anchor: map_position(selection.anchor, old_project, new_project),
        focus: map_position(selection.focus, old_project, new_project),
    }
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

fn validate_title_position(project: &Project, position: Position) -> Result<(), EditError> {
    if project.title_page.is_empty() && position == (Position { element: 0, offset: 0 }) {
        return Ok(());
    }
    let paragraph = project
        .title_page
        .get(position.element)
        .ok_or(EditError::InvalidElement)?;
    if paragraph.is_boundary(position.offset) {
        Ok(())
    } else {
        Err(EditError::InvalidUtf8Boundary)
    }
}

fn validate_title_selection(project: &Project, selection: Selection) -> Result<(), EditError> {
    validate_title_position(project, selection.anchor)?;
    validate_title_position(project, selection.focus)
}

fn selection_run_ranges(
    project: &Project,
    selection: Selection,
    title_surface: bool,
) -> Result<Vec<(ParagraphTarget, std::ops::Range<usize>)>, EditError> {
    if title_surface {
        validate_title_selection(project, selection)?;
    } else {
        validate_selection(project, selection)?;
    }
    let (start, end) = selection.ordered();
    let mut ranges = Vec::new();
    for index in start.element..=end.element {
        let (text_len, target) = if title_surface {
            let paragraph = project.title_page.get(index).ok_or(EditError::InvalidElement)?;
            (paragraph.text.len(), ParagraphTarget::TitlePage(index))
        } else {
            let paragraph = project.elements.get(index).ok_or(EditError::InvalidElement)?;
            (paragraph.text.len(), ParagraphTarget::Body(index))
        };
        let from = if index == start.element { start.offset } else { 0 };
        let to = if index == end.element { end.offset } else { text_len };
        ranges.push((target, from..to));
    }
    Ok(ranges)
}

fn selection_paragraph_targets(
    project: &Project,
    selection: Selection,
    title_surface: bool,
) -> Result<Vec<ParagraphTarget>, EditError> {
    if title_surface {
        validate_title_selection(project, selection)?;
    } else {
        validate_selection(project, selection)?;
    }
    let (start, end) = selection.ordered();
    let last = if start.element < end.element && end.offset == 0 {
        end.element - 1
    } else {
        end.element
    };
    (start.element..=last)
        .map(|index| {
            if title_surface {
                project.title_page.get(index).ok_or(EditError::InvalidElement)?;
                Ok(ParagraphTarget::TitlePage(index))
            } else {
                project.elements.get(index).ok_or(EditError::InvalidElement)?;
                Ok(ParagraphTarget::Body(index))
            }
        })
        .collect()
}

fn unsupported_title_structure() -> EditError {
    EditError::Preservation(crate::preservation::PreservationError::UnsupportedTextMapping)
}

fn effective_title_runs(paragraph: &TitlePageParagraph) -> Vec<TextRun> {
    if paragraph.runs.is_empty() {
        if paragraph.text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun { text: paragraph.text.clone(), attributes: Vec::new() }]
        }
    } else {
        paragraph.runs.clone()
    }
}

fn join_title_at(project: &mut Project, left_index: usize) -> Result<(), EditError> {
    let right_index = left_index.checked_add(1).ok_or(EditError::InvalidElement)?;
    let right = project.title_page.get(right_index).cloned().ok_or(EditError::InvalidElement)?;
    let mut left = project.title_page.get(left_index).cloned().ok_or(EditError::InvalidElement)?;
    if left.text.len().saturating_add(right.text.len()) > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let mut runs = effective_title_runs(&left);
    runs.extend(effective_title_runs(&right));
    left.text.push_str(&right.text);
    coalesce_runs(&mut runs);
    left.runs = if left.text.is_empty() { Vec::new() } else { runs };
    project.title_page[left_index] = left;
    project.title_page.remove(right_index);
    Ok(())
}

fn title_run_attributes_at(runs: &[TextRun], offset: usize, total: usize) -> Vec<crate::RunAttribute> {
    let mut cursor = 0usize;
    for run in runs {
        let limit = cursor.saturating_add(run.text.len());
        if offset < limit || (offset == total && limit == total) {
            return run.attributes.clone();
        }
        cursor = limit;
    }
    Vec::new()
}

fn apply_typing_styles(
    mut attributes: Vec<crate::RunAttribute>,
    typing: RunStyleFlags,
) -> Vec<crate::RunAttribute> {
    let mut tokens = attributes
        .iter()
        .find(|attribute| attribute.name == "Style")
        .map(|attribute| attribute.value.split('+').filter(|token| {
            !matches!(*token, "Bold" | "Italic" | "Underline" | "AllCaps" | "Strikethrough" | "StrikeThrough")
        }).map(str::to_owned).collect::<Vec<_>>())
        .unwrap_or_default();
    for (enabled, token) in [
        (typing.bold, "Bold"),
        (typing.italics, "Italic"),
        (typing.underline, "Underline"),
        (typing.all_caps, "AllCaps"),
        (typing.strikethrough, "Strikethrough"),
    ] {
        if enabled {
            tokens.push(token.to_owned());
        }
    }
    if let Some(attribute) = attributes.iter_mut().find(|attribute| attribute.name == "Style") {
        attribute.value = tokens.join("+");
    } else if !tokens.is_empty() {
        attributes.push(crate::RunAttribute { name: "Style".into(), value: tokens.join("+") });
    }
    attributes
}

fn synchronize_body_typing_style(
    project: &mut Project,
    start: Position,
    inserted: &str,
    typing: RunStyleFlags,
) -> Result<(), EditError> {
    let element = project.elements.get(start.element).ok_or(EditError::InvalidElement)?;
    let current = project_style_at_caret(
        &element.text,
        &element.runs,
        start.offset,
        RunBoundaryAffinity::Right,
        RunStyleFlags::default(),
    ).map_err(EditError::FormattedText)?;
    let target = ParagraphTarget::Body(start.element);
    let range = start.offset..start.offset.saturating_add(inserted.len());
    for style in [
        RunStyle::Bold,
        RunStyle::Italics,
        RunStyle::Underline,
        RunStyle::AllCaps,
        RunStyle::Strikethrough,
    ] {
        if current.contains(style) != typing.contains(style) {
            toggle_paragraph_run_styles(
                project,
                &[(target, range.clone())],
                RunStyleFlags::default(),
                style,
            ).map_err(EditError::FormattedText)?;
        }
    }
    Ok(())
}

fn replace_title_range(
    project: &mut Project,
    start: Position,
    end: Position,
    inserted: &str,
    typing: RunStyleFlags,
) -> Result<(), EditError> {
    validate_title_position(project, start)?;
    validate_title_position(project, end)?;
    if (start.element, start.offset) > (end.element, end.offset) {
        return Err(EditError::InvalidSelection);
    }
    if inserted.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    if project.title_page.is_empty() {
        if start != end || project.preservation.is_some() {
            return Err(unsupported_title_structure());
        }
        let id = project.allocate_element_id()?;
        let mut paragraph = TitlePageParagraph::new(
            id.as_str(),
            inserted,
            crate::TitlePageParagraphFormatting::default(),
        )?;
        if !inserted.is_empty() && typing != RunStyleFlags::default() {
            paragraph.runs.push(TextRun {
                text: inserted.to_owned(),
                attributes: apply_typing_styles(Vec::new(), typing),
            });
        }
        project.title_page.push(paragraph);
        return Ok(());
    }
    if start.element == end.element {
        let old = project.title_page[start.element].clone();
        let combined_len = old.text.len().saturating_sub(end.offset - start.offset)
            .saturating_add(inserted.len());
        if combined_len > MAX_TEXT_BYTES {
            return Err(EditError::TextTooLong);
        }
        let has_runs = !old.runs.is_empty();
        let mut has_source_mapping = false;
        if let Some(preservation) = project.preservation.as_mut() {
            if preservation.has_managed_element(&old.id) {
                has_source_mapping = true;
                if has_runs {
                    preservation.ensure_run_range_is_contiguous(
                        &old.id,
                        start.offset..end.offset,
                    )
                        .map_err(EditError::Preservation)?;
                }
                let expected = old.runs.iter().map(|run| run.text.clone()).collect::<Vec<_>>();
                preservation.replace_managed_text(
                    &old.id,
                    &old.text,
                    start.offset,
                    end.offset,
                    inserted,
                    has_runs.then_some(expected.as_slice()),
                ).map_err(EditError::Preservation)?;
            } else if old.source_provenance.is_some() {
                return Err(EditError::Preservation(crate::preservation::PreservationError::ManagedAnchorMissing));
            }
        } else if old.source_provenance.is_some() {
            return Err(EditError::Preservation(crate::preservation::PreservationError::ManagedAnchorMissing));
        }
        let text = format!("{}{}{}", &old.text[..start.offset], inserted, &old.text[end.offset..]);
        let mut updated = old;
        if has_runs {
            let effective = effective_title_runs(&updated);
            let mut runs = slice_runs(&effective, 0, start.offset);
            let inherited = title_run_attributes_at(&effective, start.offset, updated.text.len());
            push_run(&mut runs, inserted, apply_typing_styles(inherited, typing));
            runs.extend(slice_runs(&effective, end.offset, updated.text.len()));
            coalesce_runs(&mut runs);
            updated.runs = if text.is_empty() { Vec::new() } else { runs };
        } else if !inserted.is_empty() && typing != RunStyleFlags::default() {
            let effective = effective_title_runs(&updated);
            let mut runs = slice_runs(&effective, 0, start.offset);
            push_run(&mut runs, inserted, apply_typing_styles(
                title_run_attributes_at(&effective, start.offset, updated.text.len()),
                typing,
            ));
            runs.extend(slice_runs(&effective, end.offset, updated.text.len()));
            coalesce_runs(&mut runs);
            updated.runs = runs;
        }
        updated.text = text;
        if has_source_mapping {
            let sync_text = updated.text.clone();
            let sync_runs = updated.runs.clone();
            project.preservation.as_mut()
                .ok_or(EditError::Preservation(crate::preservation::PreservationError::ManagedAnchorMissing))?
                .sync_managed_text_runs(&updated.id, &sync_text, &sync_runs)
                .map_err(EditError::Preservation)?;
        }
        project.title_page[start.element] = updated;
        return Ok(());
    }

    // Structural replacement is available for detached/new documents only.
    let left = project.title_page[start.element].clone();
    let right = project.title_page[end.element].clone();
    let text = format!("{}{}{}", &left.text[..start.offset], inserted, &right.text[end.offset..]);
    if text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let mut runs = slice_runs(&effective_title_runs(&left), 0, start.offset);
    let insertion_attributes = title_run_attributes_at(&effective_title_runs(&left), start.offset, left.text.len());
    push_run(&mut runs, inserted, apply_typing_styles(insertion_attributes, typing));
    runs.extend(slice_runs(&effective_title_runs(&right), end.offset, right.text.len()));
    coalesce_runs(&mut runs);
    let mut replacement = left;
    replacement.text = text;
    replacement.runs = runs;
    project.title_page[start.element] = replacement;
    project.title_page.drain(start.element + 1..=end.element);
    Ok(())
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
    {
        let item = &mut project.elements[element_index];
        item.text = new_text;
        if item.text.is_empty() {
            item.runs.clear();
        } else if !old_runs.is_empty() {
            for (run, text) in item.runs.iter_mut().zip(updated_segments) {
                run.text = text;
            }
        }
    }
    if has_source_mapping {
        project.preservation.as_mut()
            .ok_or(EditError::Preservation(crate::preservation::PreservationError::ManagedAnchorMissing))?
            .sync_managed_text_runs(&id, &project.elements[element_index].text, &project.elements[element_index].runs)
            .map_err(EditError::Preservation)?;
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
    if new_role != Role::Dialogue
        && project.preservation.as_ref().is_some_and(|state| state.has_inline_alts(&id))
    {
        return Err(EditError::InlineAltsRoleChange);
    }
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
    let anchor = project.elements[element].id.clone();
    crate::dual_dialogue::insert_group_member_at_flat_index(
        project,
        element + 1,
        &anchor,
        new.id.clone(),
    )?;
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
    crate::dual_dialogue::insert_group_member_at_flat_index(
        project,
        position.element + 1,
        &item.id,
        new_item.id.clone(),
    )?;
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

fn unwrap_dual_dialogue_groups(
    project: &mut Project,
    group_ids: &[StableId],
) -> Result<(), EditError> {
    for group_id in group_ids {
        let index = project.dual_dialogue_groups.iter()
            .position(|group| &group.id == group_id)
            .ok_or(EditError::InvalidSelection)?;
        let group = project.dual_dialogue_groups[index].clone();
        if let Some(preservation) = project.preservation.as_mut() {
            let anchor = group.wrapper.preservation_anchor.as_ref().ok_or(
                EditError::Preservation(crate::preservation::PreservationError::ManagedAnchorMissing),
            )?;
            let members = group.member_ids().cloned().collect::<Vec<_>>();
            preservation.unwrap_dual_dialogue_members(
                &group.id,
                &members,
                anchor,
                group.wrapper.outer_foreign_id.as_deref(),
            ).map_err(EditError::Preservation)?;
        }
        project.dual_dialogue_groups.remove(index);
        project.dual_dialogue_paragraph_overrides.remove(group_id);
    }
    Ok(())
}

fn remap_selection_by_identity(
    baseline: &Project,
    candidate: &Project,
    selection: Selection,
) -> Result<Selection, EditError> {
    let remap = |position: Position| -> Result<Position, EditError> {
        let id = &baseline.elements.get(position.element).ok_or(EditError::InvalidElement)?.id;
        let element = candidate.elements.iter().position(|item| &item.id == id)
            .ok_or(EditError::InvalidElement)?;
        Ok(Position { element, offset: position.offset })
    };
    Ok(Selection { anchor: remap(selection.anchor)?, focus: remap(selection.focus)? })
}

fn groups_touched_by_selection(project: &Project, selection: Selection) -> Vec<StableId> {
    let (start, end) = selection.ordered();
    project.dual_dialogue_groups.iter().filter_map(|group| {
        let first = crate::dual_dialogue::first_member(project, group)?;
        let last = crate::dual_dialogue::last_member(project, group)?;
        let touched = if start == end {
            (first..=last).contains(&start.element)
                || start.element.checked_add(1).is_some_and(|next| next == first)
                || start.element.checked_sub(1).is_some_and(|previous| previous == last)
        } else {
            start.element <= last && end.element >= first
        };
        touched.then(|| group.id.clone())
    }).collect()
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
