mod prepared;
mod wrap;

use crate::{
    geometry::{LayoutError, LayoutErrorKind, ScreenplayLayout},
    inputs::{
        BodyEditorPresentation, EffectiveGeometryMode, EffectiveParagraph, LayoutComposition,
        LayoutPurpose,
    },
};
use scriptcraft_model::formatting::{EffectiveTextStyle, FixedPoint, SourceLayoutSettings};
use scriptcraft_model::{LayoutInputs, Project, Role, StableId, TextRun};
use scriptcraft_session::editing::EditingSession;
use std::{
    hash::{Hash, Hasher},
    ops::Range,
    sync::Arc,
    time::Instant,
};

pub(crate) use prepared::{
    prepare_body_for_layout, PreparedBodyCache, PreparedBodyEnvironment, PreparedBodyParagraph,
};

/// Measurements from one layout build. These values intentionally contain no
/// document-owned content and are returned separately from the layout so that
/// diagnostics cannot affect layout equality or pagination output.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LayoutBuildDiagnostics {
    /// Complete Project validations performed by this layout request.
    pub complete_project_validations: u64,
    pub layout_builds: u64,
    /// Document-sized semantic signatures constructed by the generic cache path.
    /// Validated-session lookups leave this at zero on both hits and misses.
    pub semantic_signature_constructions: u64,
    pub element_wraps_built: u64,
    pub element_wrap_misses: u64,
    pub element_wraps_reused: u64,
    pub element_wrap_nanos: u128,
    /// Complete cache-request work through a successful actual build, including
    /// preflight, lookup, placement, indexes, eviction, and publication.
    pub whole_build_nanos: u128,
    /// Effective prepared-environment resolution and cache lookup, excluding
    /// all miss preparation. Hits and misses have separate paragraph counts.
    pub prepared_lookup_nanos: u128,
    pub prepared_hits: u64,
    pub prepared_misses: u64,
    /// Owner-local misses that reused a proven unchanged prepared-line prefix.
    pub prepared_incremental_hits: u64,
    /// Owner-local misses that used complete preparation instead.
    pub prepared_incremental_fallbacks: u64,
    /// Aggregate preparation after prepared-cache misses only.
    pub prepared_miss_nanos: u128,
    /// Ordinary, Title, dual-dialogue, and oversized placement/reflow work.
    /// Prepared line/span rebase is nested and is not additive with this total.
    pub placement_nanos: u128,
    pub rebase_nanos: u128,
    /// Complete interaction-index construction. Body Y-order/sort is nested.
    pub interaction_index_nanos: u128,
    pub body_y_sort_nanos: u128,
}

/// Narrow root-level bridge to the private element-wrap cache implementation.
#[derive(Default)]
pub(super) struct ElementWrapCache {
    inner: wrap::ElementWrapCache,
}

impl std::fmt::Debug for ElementWrapCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, formatter)
    }
}

impl ElementWrapCache {
    pub(super) fn start_generation(&mut self) {
        self.inner.start_generation();
    }

    pub(super) fn clear(&mut self) {
        self.inner.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn wrap(
        &mut self,
        element_id: &StableId,
        text: &str,
        element: usize,
        role: Role,
        paragraph: EffectiveParagraph,
        text_style: EffectiveTextStyle,
        char_width: FixedPoint,
        runs: &[TextRun],
        atomic_ranges: &[Range<usize>],
        atomic_boundaries: &[Range<usize>],
        purpose: LayoutPurpose,
        title_page: bool,
        geometry_mode: EffectiveGeometryMode,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Vec<crate::wrap::WrappedLine> {
        self.inner.wrap(
            element_id,
            text,
            element,
            role,
            paragraph,
            text_style,
            char_width,
            runs,
            atomic_ranges,
            atomic_boundaries,
            purpose,
            title_page,
            geometry_mode,
            diagnostics,
        )
    }

    pub(super) fn evict_if_needed(&mut self) {
        self.inner.evict_if_needed();
    }
}

impl LayoutBuildDiagnostics {
    pub(super) fn record_prepared_lookup(&mut self, started: Option<Instant>, cache_hit: bool) {
        if cache_hit {
            self.prepared_hits = self.prepared_hits.saturating_add(1);
        } else {
            self.prepared_misses = self.prepared_misses.saturating_add(1);
        }
        add_elapsed_nanos(&mut self.prepared_lookup_nanos, started);
    }

    pub(super) fn record_element_wrap(&mut self, started: Instant) {
        self.element_wraps_built = self.element_wraps_built.saturating_add(1);
        self.element_wrap_misses = self.element_wrap_misses.saturating_add(1);
        self.element_wrap_nanos = self
            .element_wrap_nanos
            .saturating_add(started.elapsed().as_nanos());
    }
}

pub(super) fn add_elapsed_nanos(total: &mut u128, started: Option<Instant>) {
    if let Some(started) = started {
        *total = total.saturating_add(started.elapsed().as_nanos());
    }
}

/// Session-owned immutable layout cache shared by desktop adapters and
/// headless measurements. The revision, effective geometry and source-derived
/// layout settings are the cache key; viewport presentation state is excluded.
#[derive(Debug, Default)]
pub struct LayoutCache {
    revision: Option<u64>,
    inputs: Option<LayoutInputs>,
    source_settings: Option<SourceLayoutSettings>,
    purpose: Option<LayoutPurpose>,
    composition: Option<LayoutComposition>,
    body_presentation: Option<BodyEditorPresentation>,
    project_layout_signature: Option<u64>,
    layout: Option<Arc<ScreenplayLayout>>,
    project_identity: Option<Arc<()>>,
    element_wraps: ElementWrapCache,
    prepared_body: PreparedBodyCache,
}

impl LayoutCache {
    pub fn get_or_build(
        &mut self,
        revision: u64,
        project: &Project,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        let inputs = project.session_layout_inputs();
        self.get_or_build_with_inputs(revision, project, &inputs)
    }

    pub fn get_or_build_with_inputs(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_inputs_for_purpose(revision, project, inputs, LayoutPurpose::Editor)
    }

    pub fn get_or_build_with_inputs_for_purpose(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            purpose,
            match purpose {
                LayoutPurpose::Print => LayoutComposition::Print {
                    include_title_page: true,
                },
                LayoutPurpose::Editor => LayoutComposition::BodyEditor,
            },
            None,
            false,
            BodyEditorPresentation::default(),
        )
    }

    pub fn get_or_build_with_composition(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        composition: LayoutComposition,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            composition.purpose(),
            composition,
            None,
            false,
            BodyEditorPresentation::default(),
        )
    }

    /// Builds or reuses a Body-editor layout for a transient active-group
    /// presentation. There is intentionally no corresponding Print overload.
    pub fn get_or_build_body_editor(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        presentation: &BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            None,
            false,
            presentation.clone(),
        )
    }

    pub fn get_or_build_body_editor_with_diagnostics(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        presentation: &BodyEditorPresentation,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            Some(diagnostics),
            false,
            presentation.clone(),
        )
    }

    /// Looks up a Body-editor layout using the validated revision published by
    /// the editing session. The transient presentation remains part of the key.
    pub fn get_or_build_body_editor_for_session(
        &mut self,
        session: &EditingSession,
        presentation: &BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            None,
            true,
            presentation.clone(),
        )
    }

    /// Diagnostic counterpart to [`Self::get_or_build_body_editor_for_session`].
    pub fn get_or_build_body_editor_for_session_with_diagnostics(
        &mut self,
        session: &EditingSession,
        presentation: &BodyEditorPresentation,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            Some(diagnostics),
            true,
            presentation.clone(),
        )
    }

    /// Looks up the Title-page-editor layout using the session's validated
    /// revision and the title-editor composition.
    pub fn get_or_build_title_page_editor_for_session(
        &mut self,
        session: &EditingSession,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::TitlePageEditor,
            None,
            true,
            BodyEditorPresentation::default(),
        )
    }

    /// Diagnostic counterpart to [`Self::get_or_build_title_page_editor_for_session`].
    pub fn get_or_build_title_page_editor_for_session_with_diagnostics(
        &mut self,
        session: &EditingSession,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::TitlePageEditor,
            Some(diagnostics),
            true,
            BodyEditorPresentation::default(),
        )
    }

    /// Diagnostic counterpart to the ordinary cache lookup. A cache hit
    /// returns zero build measurements; a miss reports wrapping work separately
    /// from validation, pagination, and cache lookup.
    pub fn get_or_build_with_inputs_for_purpose_and_diagnostics(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            revision,
            project,
            inputs,
            purpose,
            match purpose {
                LayoutPurpose::Print => LayoutComposition::Print {
                    include_title_page: true,
                },
                LayoutPurpose::Editor => LayoutComposition::BodyEditor,
            },
            Some(diagnostics),
            false,
            BodyEditorPresentation::default(),
        )
    }

    /// Builds from a live editing session whose complete baseline and every
    /// published edit candidate are validated by the session boundary.
    pub fn get_or_build_for_session_with_diagnostics(
        &mut self,
        revision: u64,
        session: &EditingSession,
        purpose: LayoutPurpose,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_optional_diagnostics(
            revision,
            session.project(),
            session.layout_inputs(),
            purpose,
            match purpose {
                LayoutPurpose::Print => LayoutComposition::Print {
                    include_title_page: true,
                },
                LayoutPurpose::Editor => LayoutComposition::BodyEditor,
            },
            Some(diagnostics),
            true,
            BodyEditorPresentation::default(),
        )
    }

    fn get_or_build_with_optional_diagnostics(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
        composition: LayoutComposition,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
        validated_session: bool,
        body_presentation: BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        let whole_build_started = diagnostics.as_ref().map(|_| Instant::now());
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.complete_project_validations = if validated_session { 0 } else { 1 };
        }
        if !validated_session {
            project.validate()?;
        }
        let mut source_settings = project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source_layout_settings().clone())
            .unwrap_or_default();
        source_settings
            .direct_paragraph_overrides
            .extend(project.direct_paragraph_overrides.clone());
        let same_document = self
            .project_identity
            .as_ref()
            .is_some_and(|identity| Arc::ptr_eq(identity, project.layout_identity()));
        if self.project_identity.is_some() && !same_document {
            self.revision = None;
            self.inputs = None;
            self.source_settings = None;
            self.purpose = None;
            self.composition = None;
            self.body_presentation = None;
            self.project_layout_signature = None;
            self.layout = None;
            self.element_wraps.clear();
            self.prepared_body.clear();
        }
        let source_settings_match = self
            .source_settings
            .as_ref()
            .is_some_and(|cached| cached == &source_settings);
        let cache_keys_match = self.revision == Some(revision)
            && self.inputs.as_ref() == Some(inputs)
            && source_settings_match
            && same_document
            && self.purpose == Some(purpose)
            && self.composition == Some(composition)
            && self.body_presentation.as_ref() == Some(&body_presentation);
        let mut project_layout_signature = None;
        if cache_keys_match {
            if validated_session {
                return self
                    .layout
                    .clone()
                    .ok_or(LayoutError {
                        kind: LayoutErrorKind::NoGeometry,
                    })
                    .map(|layout| (layout, true));
            }
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.semantic_signature_constructions = diagnostics
                    .semantic_signature_constructions
                    .saturating_add(1);
            }
            let signature = semantic_project_layout_signature(project);
            if self.project_layout_signature == Some(signature) {
                return self
                    .layout
                    .clone()
                    .ok_or(LayoutError {
                        kind: LayoutErrorKind::NoGeometry,
                    })
                    .map(|layout| (layout, true));
            }
            project_layout_signature = Some(signature);
        }
        if !validated_session && project_layout_signature.is_none() {
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.semantic_signature_constructions = diagnostics
                    .semantic_signature_constructions
                    .saturating_add(1);
            }
            project_layout_signature = Some(semantic_project_layout_signature(project));
        }
        self.element_wraps.start_generation();
        self.prepared_body.start_generation();
        let layout = Arc::new(
            ScreenplayLayout::build_with_effective_inputs_for_purpose_internal(
                project,
                inputs,
                &source_settings,
                purpose,
                composition,
                diagnostics.as_deref_mut(),
                Some(&mut self.element_wraps),
                Some(&mut self.prepared_body),
                true,
                &body_presentation,
            )?,
        );
        self.element_wraps.evict_if_needed();
        self.prepared_body.evict_if_needed();
        self.revision = Some(revision);
        self.inputs = Some(inputs.clone());
        self.source_settings = Some(source_settings);
        self.purpose = Some(purpose);
        self.composition = Some(composition);
        self.body_presentation = Some(body_presentation);
        self.project_layout_signature = project_layout_signature;
        self.layout = Some(layout.clone());
        self.project_identity = Some(project.layout_identity().clone());
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            add_elapsed_nanos(&mut diagnostics.whole_build_nanos, whole_build_started);
        }
        Ok((layout, false))
    }
}

fn semantic_project_layout_signature(project: &Project) -> u64 {
    let mut signature_hasher = std::collections::hash_map::DefaultHasher::new();
    format!(
        "{:?}{:?}{:?}{:?}{:?}",
        project.elements,
        project.title_page,
        project.direct_paragraph_overrides,
        project.dual_dialogue_groups,
        project.dual_dialogue_paragraph_overrides,
    )
    .hash(&mut signature_hasher);
    signature_hasher.finish()
}
