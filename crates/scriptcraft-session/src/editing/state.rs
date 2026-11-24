//! Editing-session value state, construction, and basic accessors.

use super::derived::PendingDerivedInvalidation;
use super::selection::{validate_selection, validate_title_selection};
use super::{
    validate_full_candidate, EditError, EditMutationDiagnostics, EditPolicy, Position, Selection,
};
use scriptcraft_model::formatted_text::{
    project_style_at_caret, InlineRunFormattingPatch, RunBoundaryAffinity, RunStyleFlags,
};
use scriptcraft_model::{validate_layout_inputs, LayoutInputs, Project};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct EditingSession {
    pub(crate) project: Project,
    /// Ephemeral operation cursor used by callers to distinguish successful
    /// mutations from no-ops without cloning or comparing document state.
    pub(super) mutation_token: u64,
    /// Wall-clock time captured when this editing session begins. It is
    /// ephemeral lineage state, independent of DocumentRef and undo history.
    pub(super) session_open_timestamp: String,
    pub(super) policy: EditPolicy,
    pub(super) layout_inputs: LayoutInputs,
    pub(crate) selection: Selection,
    pub(crate) title_selection: Selection,
    pub(crate) beat_selection: super::BeatSelection,
    pub(super) beat_typing_style: RunStyleFlags,
    pub(super) beat_typing_formatting: InlineRunFormattingPatch,
    pub(super) title_typing_style: RunStyleFlags,
    pub(super) body_typing_style: RunStyleFlags,
    pub(super) body_typing_formatting: InlineRunFormattingPatch,
    pub(crate) dirty: bool,
    pub(crate) revision: u64,
    pub(crate) clean_project: Project,
    pub(super) history: Vec<super::history::Snapshot>,
    pub(super) redo: Vec<super::history::Snapshot>,
    pub(crate) generated_parentheticals: BTreeSet<String>,
    pub(super) history_limit: usize,
    pub(super) validated_baseline: bool,
    /// Transient desktop-controlled capture; never serialized or compared as
    /// semantic session state and disabled unless the development host opts in.
    pub(super) capture_mutation_diagnostics: bool,
    pub(super) last_mutation_diagnostics: Option<EditMutationDiagnostics>,
    pub(super) pending_derived_invalidation: PendingDerivedInvalidation,
    pub(super) deferred_derived_scenes: BTreeSet<String>,
    pub(super) failed_derived_complete: bool,
    pub(super) failed_derived_scenes: BTreeSet<String>,
}

impl PartialEq for EditingSession {
    fn eq(&self, other: &Self) -> bool {
        // The operation cursor observes mutations; it is not semantic session
        // state and deliberately does not participate in value equality.
        self.project == other.project
            && self.session_open_timestamp == other.session_open_timestamp
            && self.policy == other.policy
            && self.layout_inputs == other.layout_inputs
            && self.selection == other.selection
            && self.title_selection == other.title_selection
            && self.beat_selection == other.beat_selection
            && self.beat_typing_style == other.beat_typing_style
            && self.beat_typing_formatting == other.beat_typing_formatting
            && self.title_typing_style == other.title_typing_style
            && self.body_typing_style == other.body_typing_style
            && self.body_typing_formatting == other.body_typing_formatting
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

impl EditingSession {
    pub(crate) fn with_policy_layout_and_session_timestamp(
        mut project: Project,
        policy: EditPolicy,
        session_open_timestamp: String,
        history_limit: usize,
    ) -> Result<Self, EditError> {
        validate_full_candidate(&project)?;
        if let Some(preservation) = project.preservation.as_mut() {
            preservation.rebuild_indexes();
        }
        let layout_inputs = project.session_layout_inputs();
        validate_layout_inputs(&layout_inputs)?;
        let selection = Selection::caret(Position {
            element: 0,
            offset: 0,
        });
        validate_selection(&project, selection)?;
        let body_typing_style =
            project
                .elements
                .first()
                .map_or(Ok(RunStyleFlags::default()), |element| {
                    project_style_at_caret(
                        &element.text,
                        &element.runs,
                        0,
                        RunBoundaryAffinity::Right,
                        RunStyleFlags::default(),
                    )
                    .map_err(EditError::FormattedText)
                })?;
        let title_typing_style =
            project
                .title_page
                .first()
                .map_or(Ok(RunStyleFlags::default()), |paragraph| {
                    project_style_at_caret(
                        &paragraph.text,
                        &paragraph.runs,
                        0,
                        RunBoundaryAffinity::Right,
                        RunStyleFlags::default(),
                    )
                    .map_err(EditError::FormattedText)
                })?;
        Ok(Self {
            clean_project: project.clone(),
            project,
            mutation_token: 0,
            session_open_timestamp,
            policy,
            layout_inputs,
            selection,
            title_selection: Selection::caret(Position {
                element: 0,
                offset: 0,
            }),
            beat_selection: super::BeatSelection::None,
            beat_typing_style: RunStyleFlags::default(),
            beat_typing_formatting: InlineRunFormattingPatch::default(),
            title_typing_style,
            body_typing_style,
            body_typing_formatting: InlineRunFormattingPatch::default(),
            dirty: false,
            revision: 0,
            history: Vec::new(),
            redo: Vec::new(),
            generated_parentheticals: BTreeSet::new(),
            history_limit,
            validated_baseline: true,
            capture_mutation_diagnostics: false,
            last_mutation_diagnostics: None,
            pending_derived_invalidation: PendingDerivedInvalidation::Complete,
            deferred_derived_scenes: BTreeSet::new(),
            failed_derived_complete: false,
            failed_derived_scenes: BTreeSet::new(),
        })
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

    /// Read-only Project-derived geometry snapshot for existing layout/output
    /// consumers. The Project remains the sole document page-layout authority.
    pub fn layout_inputs(&self) -> &LayoutInputs {
        &self.layout_inputs
    }

    pub(crate) fn refresh_layout_inputs_from_project(&mut self) {
        self.layout_inputs = self.project.session_layout_inputs();
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

    /// Transient Body Font/Size/Color overrides for insertion at the current
    /// collapsed caret. This state is session-only and never dirties the Project.
    pub fn body_typing_formatting(&self) -> InlineRunFormattingPatch {
        self.body_typing_formatting
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

    /// Transient Beat body style at the active collapsed text caret.
    pub fn beat_typing_style(&self) -> RunStyleFlags { self.beat_typing_style }

    /// Transient Beat body inline-formatting patch; never persisted or dirtied.
    pub fn beat_typing_formatting(&self) -> InlineRunFormattingPatch { self.beat_typing_formatting }

    pub const fn dirty(&self) -> bool {
        self.dirty
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Cheap ephemeral cursor for editor-command outcome detection. Reachable
    /// session operations advance it for semantic, selection, or typing-style
    /// mutations. Unlike `revision`, it includes transient changes and cannot
    /// saturate on a content edit.
    pub const fn mutation_token(&self) -> u64 {
        self.mutation_token
    }

    pub(crate) fn mark_mutated(&mut self) {
        self.mutation_token = self.mutation_token.wrapping_add(1);
    }

    /// Arms one development-only observation for the next actual Body candidate
    /// mutation. Selection-only commands and Title mutations produce no sample.
    pub fn begin_mutation_diagnostics(&mut self) {
        self.capture_mutation_diagnostics = true;
        self.last_mutation_diagnostics = None;
    }

    /// Ends the opt-in capture and transfers only the completed value-free
    /// observation. No timer or document content crosses the core/UI boundary.
    pub fn take_mutation_diagnostics(&mut self) -> Option<EditMutationDiagnostics> {
        self.capture_mutation_diagnostics = false;
        self.last_mutation_diagnostics.take()
    }

    /// Revalidates the live semantic project and selection without mutating it.
    pub fn validate(&self) -> Result<(), EditError> {
        validate_full_candidate(&self.project)?;
        validate_selection(&self.project, self.selection)?;
        super::beat_board::validate_beat_selection(&self.project, &self.beat_selection)?;
        validate_title_selection(&self.project, self.title_selection)
    }

    pub fn mark_saved(&mut self) {
        self.clean_project = self.project.clone();
        self.dirty = false;
    }

    pub fn replace_with_saved(&mut self, project: Project) -> Result<(), EditError> {
        let mut replacement = Self::with_policy_layout_and_session_timestamp(
            project,
            self.policy,
            self.session_open_timestamp.clone(),
            100,
        )?;
        replacement.session_open_timestamp = self.session_open_timestamp.clone();
        *self = replacement;
        Ok(())
    }
}
