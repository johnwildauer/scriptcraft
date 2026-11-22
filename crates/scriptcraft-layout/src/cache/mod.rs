mod prepared;
mod wrap;

use crate::{
    geometry::{LayoutError, LayoutErrorKind, ScreenplayLayout},
    inputs::{
        BodyEditorPresentation, EffectiveGeometryMode, EffectiveParagraph, LayoutComposition,
        LayoutEnvironment, LayoutPurpose,
    },
};
use scriptcraft_model::formatting::{FixedPoint, SourceLayoutSettings};
use scriptcraft_model::typography::EffectiveTextStyle as ResolvedTextStyle;
use scriptcraft_model::{
    dual_dialogue::DualDialogueGroup, element::body_owners_equal,
    formatting::SourceParagraphOverrides, ElementDefinition, ElementTypeKey, LayoutInputs,
    PageLayoutInputs, Project, StableId, TextRun, TitlePageParagraph,
};
use scriptcraft_session::editing::EditingSession;
use std::{
    hash::{Hash, Hasher},
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
    /// Reserved incremental-preparation counter; resolved preparation does not
    /// currently take an incremental path.
    pub prepared_incremental_hits: u64,
    /// Reserved fallback counter paired with `prepared_incremental_hits`.
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
    pub(super) fn set_layout_environment(&mut self, environment: Option<Arc<LayoutEnvironment>>) {
        self.inner.set_layout_environment(environment);
    }

    pub(super) fn start_generation(&mut self) {
        self.inner.start_generation();
    }

    pub(super) fn clear(&mut self) {
        self.inner.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn wrap_resolved(
        &mut self,
        element_id: &StableId,
        element_key: Option<&scriptcraft_model::ElementTypeKey>,
        text: &str,
        element: usize,
        element_type: Option<ElementTypeKey>,
        paragraph: EffectiveParagraph,
        resolved_text_style: ResolvedTextStyle,
        runs: &[TextRun],
        purpose: LayoutPurpose,
        title_page: bool,
        geometry_mode: EffectiveGeometryMode,
        layout_environment: &LayoutEnvironment,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Result<Vec<crate::wrap::WrappedLine>, LayoutError> {
        self.inner.wrap_resolved(
            element_id,
            element_key,
            text,
            element,
            element_type,
            paragraph,
            resolved_text_style,
            runs,
            purpose,
            title_page,
            geometry_mode,
            layout_environment,
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
/// headless measurements. Revision, semantic project state, effective page
/// inputs, resolved typography, and composition are identity inputs. The
/// desktop zoom baseline is refreshed on a hit but is not canonical identity.
/// Viewport and paint/presentation state are excluded.
#[derive(Debug, Default)]
pub struct LayoutCache {
    revision: Option<u64>,
    inputs: Option<PageLayoutInputs>,
    source_settings: Option<SourceLayoutSettings>,
    layout_environment: Option<Arc<LayoutEnvironment>>,
    purpose: Option<LayoutPurpose>,
    composition: Option<LayoutComposition>,
    body_presentation: Option<BodyEditorPresentation>,
    project_layout_signature: Option<u64>,
    semantic_project: Option<SemanticProjectLayoutSnapshot>,
    definition_registry: Option<Vec<ElementDefinition>>,
    layout: Option<Arc<ScreenplayLayout>>,
    project_identity: Option<Arc<()>>,
    element_wraps: ElementWrapCache,
    prepared_body: PreparedBodyCache,
}

#[derive(Debug, Clone)]
struct SemanticProjectLayoutSnapshot {
    elements: Vec<Arc<scriptcraft_model::BodyOwner>>,
    title_page: Vec<TitlePageParagraph>,
    direct_paragraph_overrides: std::collections::BTreeMap<StableId, SourceParagraphOverrides>,
    dual_dialogue_groups: Vec<DualDialogueGroup>,
    dual_dialogue_paragraph_overrides:
        std::collections::BTreeMap<StableId, SourceParagraphOverrides>,
}

impl SemanticProjectLayoutSnapshot {
    fn from_project(project: &Project) -> Self {
        Self {
            elements: project.elements.clone(),
            title_page: project.title_page.clone(),
            direct_paragraph_overrides: project.direct_paragraph_overrides.clone(),
            dual_dialogue_groups: project.dual_dialogue_groups.clone(),
            dual_dialogue_paragraph_overrides: project.dual_dialogue_paragraph_overrides.clone(),
        }
    }

    fn matches(&self, project: &Project) -> bool {
        body_owners_equal(&self.elements, &project.elements)
            && self.title_page == project.title_page
            && self.direct_paragraph_overrides == project.direct_paragraph_overrides
            && self.dual_dialogue_groups == project.dual_dialogue_groups
            && self.dual_dialogue_paragraph_overrides == project.dual_dialogue_paragraph_overrides
    }
}

impl LayoutCache {
    /// Builds any composition with the complete resolved-font environment in
    /// the cache identity. Output adapters can select Print through this path.
    pub fn get_or_build_with_composition_and_environment(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        composition: LayoutComposition,
        environment: &LayoutEnvironment,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_environment(
            revision,
            project,
            inputs,
            composition.purpose(),
            composition,
            None,
            false,
            BodyEditorPresentation::default(),
            environment,
        )
    }

    pub fn get_or_build_with_composition_and_environment_and_diagnostics(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        composition: LayoutComposition,
        environment: &LayoutEnvironment,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_environment(
            revision,
            project,
            inputs,
            composition.purpose(),
            composition,
            Some(diagnostics),
            false,
            BodyEditorPresentation::default(),
            environment,
        )
    }

    /// Body lookup uses the complete resolved environment as a cache identity input.
    pub fn get_or_build_body_editor_for_session_with_environment(
        &mut self,
        session: &EditingSession,
        presentation: &BodyEditorPresentation,
        environment: &LayoutEnvironment,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_environment(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            None,
            true,
            presentation.clone(),
            environment,
        )
    }

    pub fn get_or_build_body_editor_for_session_with_environment_and_diagnostics(
        &mut self,
        session: &EditingSession,
        presentation: &BodyEditorPresentation,
        environment: &LayoutEnvironment,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_environment(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::BodyEditor,
            Some(diagnostics),
            true,
            presentation.clone(),
            environment,
        )
    }

    pub fn get_or_build_title_page_editor_for_session_with_environment(
        &mut self,
        session: &EditingSession,
        environment: &LayoutEnvironment,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_environment(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::TitlePageEditor,
            None,
            true,
            BodyEditorPresentation::default(),
            environment,
        )
    }

    pub fn get_or_build_title_page_editor_for_session_with_environment_and_diagnostics(
        &mut self,
        session: &EditingSession,
        environment: &LayoutEnvironment,
        diagnostics: &mut LayoutBuildDiagnostics,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        *diagnostics = LayoutBuildDiagnostics::default();
        self.get_or_build_with_environment(
            session.revision(),
            session.project(),
            session.layout_inputs(),
            LayoutPurpose::Editor,
            LayoutComposition::TitlePageEditor,
            Some(diagnostics),
            true,
            BodyEditorPresentation::default(),
            environment,
        )
    }

    fn get_or_build_with_environment(
        &mut self,
        revision: u64,
        project: &Project,
        inputs: &LayoutInputs,
        purpose: LayoutPurpose,
        composition: LayoutComposition,
        mut diagnostics: Option<&mut LayoutBuildDiagnostics>,
        validated_session: bool,
        body_presentation: BodyEditorPresentation,
        layout_environment: &LayoutEnvironment,
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
            self.layout_environment = None;
            self.purpose = None;
            self.composition = None;
            self.body_presentation = None;
            self.project_layout_signature = None;
            self.semantic_project = None;
            self.definition_registry = None;
            self.layout = None;
            self.element_wraps.clear();
            self.prepared_body.clear();
        }
        let current_definitions = project
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let definition_registry_matches =
            self.definition_registry.as_ref() == Some(&current_definitions);
        if self
            .definition_registry
            .as_ref()
            .is_some_and(|cached| cached != &current_definitions)
        {
            self.element_wraps.clear();
            self.prepared_body.clear();
        }
        let environment_matches = self.layout_environment.as_deref() == Some(layout_environment);
        let layout_environment = if environment_matches {
            self.layout_environment
                .clone()
                .expect("matching layout environment is cached")
        } else {
            Arc::new(layout_environment.clone())
        };
        self.prepared_body
            .set_layout_environment(Some(layout_environment.clone()));
        self.element_wraps
            .set_layout_environment(Some(layout_environment.clone()));
        let source_settings_match = self
            .source_settings
            .as_ref()
            .is_some_and(|cached| cached == &source_settings);
        let page_inputs = inputs.page_only();
        let cache_keys_match = self.revision == Some(revision)
            && self.inputs.as_ref() == Some(&page_inputs)
            && source_settings_match
            && environment_matches
            && definition_registry_matches
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
                    .map(|layout| (layout_with_current_font_size(layout, inputs), true));
            }
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.semantic_signature_constructions = diagnostics
                    .semantic_signature_constructions
                    .saturating_add(1);
            }
            let signature = semantic_project_layout_signature(project);
            if self.project_layout_signature == Some(signature)
                && self
                    .semantic_project
                    .as_ref()
                    .is_some_and(|cached| cached.matches(project))
            {
                return self
                    .layout
                    .clone()
                    .ok_or(LayoutError {
                        kind: LayoutErrorKind::NoGeometry,
                    })
                    .map(|layout| (layout_with_current_font_size(layout, inputs), true));
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
        let layout = match ScreenplayLayout::build_with_effective_inputs_for_purpose_internal(
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
                layout_environment.clone(),
            ) {
            Ok(layout) => Arc::new(layout),
            Err(error) => {
                // Do not retain partial lower-level products from a failed
                // complete-layout construction.
                self.element_wraps.clear();
                self.prepared_body.clear();
                return Err(error);
            }
        };
        self.element_wraps.evict_if_needed();
        self.prepared_body.evict_if_needed();
        self.revision = Some(revision);
        self.inputs = Some(page_inputs);
        self.source_settings = Some(source_settings);
        self.layout_environment = Some(layout_environment);
        self.purpose = Some(purpose);
        self.composition = Some(composition);
        self.body_presentation = Some(body_presentation);
        self.project_layout_signature = project_layout_signature;
        self.semantic_project = Some(SemanticProjectLayoutSnapshot::from_project(project));
        self.definition_registry = Some(current_definitions);
        self.layout = Some(layout.clone());
        self.project_identity = Some(project.layout_identity().clone());
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            add_elapsed_nanos(&mut diagnostics.whole_build_nanos, whole_build_started);
        }
        Ok((layout, false))
    }
}

fn layout_with_current_font_size(
    layout: Arc<ScreenplayLayout>,
    inputs: &LayoutInputs,
) -> Arc<ScreenplayLayout> {
    if layout.inputs.font_size == inputs.font_size {
        return layout;
    }
    // The desktop zoom baseline is presented through LayoutInputs but is not a
    // canonical geometry determinant. Refresh only that projection on a hit.
    let mut projected = layout.as_ref().clone();
    projected.inputs.font_size = inputs.font_size;
    Arc::new(projected)
}

fn semantic_project_layout_signature(project: &Project) -> u64 {
    let mut signature_hasher = std::collections::hash_map::DefaultHasher::new();
    format!(
        "{:?}{:?}{:?}{:?}{:?}{:?}",
        project.elements,
        project.title_page,
        project.direct_paragraph_overrides,
        project.dual_dialogue_groups,
        project.dual_dialogue_paragraph_overrides,
        project.element_definitions(),
    )
    .hash(&mut signature_hasher);
    signature_hasher.finish()
}
