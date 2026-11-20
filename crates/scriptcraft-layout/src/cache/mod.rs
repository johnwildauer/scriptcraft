mod prepared;
mod wrap;

use crate::{
    geometry::{LayoutError, LayoutErrorKind, ScreenplayLayout},
    inputs::{
        BodyEditorPresentation, EffectiveGeometryMode, EffectiveParagraph, LayoutComposition,
        LayoutEnvironment, LayoutPurpose,
    },
};
use scriptcraft_model::formatting::{EffectiveTextStyle, FixedPoint, SourceLayoutSettings};
use scriptcraft_model::typography::EffectiveTextStyle as ResolvedTextStyle;
use scriptcraft_model::{
    dual_dialogue::DualDialogueGroup, element::body_owners_equal,
    formatting::SourceParagraphOverrides, ElementDefinition, ElementTypeKey, LayoutInputs, Project,
    StableId, TextRun, TitlePageParagraph,
};
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
    pub(super) fn wrap(
        &mut self,
        element_id: &StableId,
        element_key: Option<&scriptcraft_model::ElementTypeKey>,
        text: &str,
        element: usize,
        element_type: Option<ElementTypeKey>,
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
            element_key,
            text,
            element,
            element_type,
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn wrap_resolved(
        &mut self,
        element_id: &StableId,
        element_key: Option<&scriptcraft_model::ElementTypeKey>,
        text: &str,
        element: usize,
        element_type: Option<ElementTypeKey>,
        paragraph: EffectiveParagraph,
        text_style: EffectiveTextStyle,
        resolved_text_style: ResolvedTextStyle,
        runs: &[TextRun],
        purpose: LayoutPurpose,
        title_page: bool,
        geometry_mode: EffectiveGeometryMode,
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
    ) -> Result<Vec<crate::wrap::WrappedLine>, LayoutError> {
        self.inner.wrap_resolved(
            element_id,
            element_key,
            text,
            element,
            element_type,
            paragraph,
            text_style,
            resolved_text_style,
            runs,
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
/// headless measurements. Revision, semantic project state, effective layout
/// settings, resolved typography, and composition are identity inputs;
/// viewport presentation state is excluded.
#[derive(Debug, Default)]
pub struct LayoutCache {
    revision: Option<u64>,
    inputs: Option<LayoutInputs>,
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

    /// Builds any composition with a complete resolved-font environment in the
    /// cache identity. Output adapters can select Print without a separate
    /// typography-blind cache path.
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
            Some(environment),
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
            Some(environment),
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

    /// Typography-aware Body lookup. The complete transient environment is a
    /// cache identity input even while legacy layout composition is migrating.
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
            Some(environment),
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
            Some(environment),
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
            Some(environment),
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
            Some(environment),
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
        diagnostics: Option<&mut LayoutBuildDiagnostics>,
        validated_session: bool,
        body_presentation: BodyEditorPresentation,
    ) -> Result<(Arc<ScreenplayLayout>, bool), LayoutError> {
        self.get_or_build_with_environment(
            revision,
            project,
            inputs,
            purpose,
            composition,
            diagnostics,
            validated_session,
            body_presentation,
            None,
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
        layout_environment: Option<&LayoutEnvironment>,
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
        let environment_matches = self.layout_environment.as_deref() == layout_environment;
        if !environment_matches {
            self.prepared_body.clear();
        }
        let layout_environment = if environment_matches {
            self.layout_environment.clone()
        } else {
            layout_environment.cloned().map(Arc::new)
        };
        self.element_wraps
            .set_layout_environment(layout_environment.clone());
        let source_settings_match = self
            .source_settings
            .as_ref()
            .is_some_and(|cached| cached == &source_settings);
        let cache_keys_match = self.revision == Some(revision)
            && self.inputs.as_ref() == Some(inputs)
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
                    .map(|layout| (layout, true));
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
                layout_environment.clone(),
            )?,
        );
        self.element_wraps.evict_if_needed();
        self.prepared_body.evict_if_needed();
        self.revision = Some(revision);
        self.inputs = Some(inputs.clone());
        self.source_settings = Some(source_settings);
        self.layout_environment = layout_environment;
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

#[cfg(test)]
mod typography_cache_identity_tests {
    use super::*;
    use scriptcraft_model::{
        formatting::{ParagraphAlignment, SourceParagraphOverrides},
        layout_inputs::PageLayoutInputs,
        typography::{
            DocumentFontEnvironment, FaceStyle, FontResolutionKind, RequestedFontResolution,
            ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
            ScalarAdvance,
        },
        ModelError, TitlePageAlignment,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn synthetic_environment(
        ascender: i32,
        fingerprint_seed: u64,
    ) -> Result<LayoutEnvironment, ModelError> {
        let project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let mut families = project
            .element_definitions()
            .iter()
            .map(|definition| definition.font.family.clone())
            .collect::<BTreeSet<_>>();
        for owner in &project.elements {
            families.extend(owner.as_element().runs.iter().flat_map(|run| {
                run.attributes.iter().filter_map(|attribute| {
                    (attribute.name == "Font").then(|| attribute.value.clone())
                })
            }));
        }
        for title in &project.title_page {
            if let Some(key) = title.element_type.as_ref() {
                if let Some(definition) = project.element_definitions().get(key) {
                    families.insert(definition.font.family.clone());
                }
            }
        }
        families.insert("Courier Prime".to_owned());
        let mut scalars = project
            .elements
            .iter()
            .flat_map(|owner| owner.as_element().text.chars())
            .collect::<BTreeSet<_>>();
        let uppercase = scalars
            .iter()
            .flat_map(|scalar| scalar.to_uppercase())
            .collect::<Vec<_>>();
        scalars.extend(uppercase);
        scalars.insert('x');
        let mut faces = BTreeMap::new();
        let mut requests = BTreeMap::new();
        for (family_index, family) in families.into_iter().enumerate() {
            let mut ids = Vec::new();
            for (index, style) in FaceStyle::ALL.into_iter().enumerate() {
                let id = ResolvedFontId::new(format!("synthetic-{family_index}-{index}"))?;
                let advances = scalars
                    .iter()
                    .enumerate()
                    .map(|(scalar_index, scalar)| {
                        (
                            *scalar,
                            ScalarAdvance {
                                advance_units: 500 + scalar_index as u32,
                                missing_glyph: false,
                            },
                        )
                    })
                    .collect();
                let metrics = ResolvedFaceMetrics::new(1_000, ascender, -200, 0, advances)?;
                let face = ResolvedFontFace::new(
                    id.clone(),
                    family.clone(),
                    format!("{family} {style:?}"),
                    style,
                    ResolvedFontSourceKind::Synthetic,
                    128,
                    fingerprint_seed + family_index as u64 + index as u64,
                    metrics,
                )?;
                ids.push(id.clone());
                faces.insert(id, face);
            }
            let kind = if family == "Courier Prime" {
                FontResolutionKind::CourierAlias
            } else {
                FontResolutionKind::UnknownFallback
            };
            requests.insert(
                family,
                RequestedFontResolution {
                    kind,
                    regular: ids[0].clone(),
                    bold: ids[1].clone(),
                    italic: ids[2].clone(),
                    bold_italic: ids[3].clone(),
                },
            );
        }
        let fonts = DocumentFontEnvironment::new(requests, faces)?;
        Ok(LayoutEnvironment::new(PageLayoutInputs::default(), fonts))
    }

    #[test]
    fn complete_environment_equality_rejects_same_ids_with_new_bytes_or_metrics(
    ) -> Result<(), ModelError> {
        let original = synthetic_environment(800, 10)?;
        let same = synthetic_environment(800, 10)?;
        let different_bytes = synthetic_environment(800, 11)?;
        let different_metrics = synthetic_environment(801, 10)?;
        let mut different_page = original.clone();
        different_page.page.paper_width = different_page.page.paper_width.saturating_add(1);
        let different_neutral_title =
            original
                .clone()
                .with_neutral_title_style(crate::inputs::NeutralTitleStyle {
                    size: 14,
                    ..crate::inputs::NeutralTitleStyle::default()
                });
        assert_eq!(original, same);
        assert_ne!(original, different_bytes);
        assert_ne!(original, different_metrics);
        assert_ne!(original, different_page);
        assert_ne!(original, different_neutral_title);
        Ok(())
    }

    #[test]
    fn generic_semantic_signature_covers_ordered_definitions_capabilities_and_direct_values(
    ) -> Result<(), ModelError> {
        let project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let signature = semantic_project_layout_signature(&project);

        let mut changed_definition = project.clone();
        let mut definitions = changed_definition
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if let Some(action) = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == "Action")
        {
            action.font.size = action.font.size.saturating_add(1);
            action.behavior.paginate_as = Some("Scene Heading".to_owned());
        }
        changed_definition.replace_element_definitions(definitions)?;
        assert_ne!(
            signature,
            semantic_project_layout_signature(&changed_definition)
        );

        let mut paginate_as_only = project.clone();
        let mut definitions = paginate_as_only
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if let Some(action) = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == "Action")
        {
            action.behavior.paginate_as = Some("Character".to_owned());
        }
        paginate_as_only.replace_element_definitions(definitions)?;
        assert_ne!(
            signature,
            semantic_project_layout_signature(&paginate_as_only)
        );

        let mut capability_key_change = project.clone();
        let mut definitions = capability_key_change
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if let Some(action) = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == "Action")
        {
            action.key = scriptcraft_model::ElementTypeKey::new("Custom Action")?;
        }
        capability_key_change.replace_element_definitions(definitions)?;
        assert_ne!(
            signature,
            semantic_project_layout_signature(&capability_key_change)
        );

        let mut reordered = project.clone();
        let reversed = reordered
            .element_definitions()
            .iter()
            .cloned()
            .rev()
            .collect();
        reordered.replace_element_definitions(reversed)?;
        assert_ne!(signature, semantic_project_layout_signature(&reordered));

        let mut direct_override = project;
        let owner_id = direct_override.elements[0].id.clone();
        let mut direct = SourceParagraphOverrides::default();
        direct.paragraph.alignment = Some(ParagraphAlignment::Center);
        direct_override
            .direct_paragraph_overrides
            .insert(owner_id, direct);
        assert_ne!(
            signature,
            semantic_project_layout_signature(&direct_override)
        );

        let mut title_override = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let title = title_override
            .title_page
            .first_mut()
            .ok_or(ModelError::InvalidNumber)?;
        title.formatting.alignment = Some(TitlePageAlignment::Center);
        assert_ne!(
            signature,
            semantic_project_layout_signature(&title_override)
        );

        let mut wrapper_override = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        wrapper_override.dual_dialogue_paragraph_overrides.insert(
            StableId::new("wrapper-cache-test")?,
            SourceParagraphOverrides::default(),
        );
        assert_ne!(
            signature,
            semantic_project_layout_signature(&wrapper_override)
        );

        let mut dual_source = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let member = dual_source
            .elements
            .first()
            .map(|owner| owner.id.clone())
            .ok_or(ModelError::InvalidNumber)?;
        dual_source.dual_dialogue_groups.push(DualDialogueGroup {
            id: StableId::new("dual-source-cache-test")?,
            members: vec![member],
            wrapper: Default::default(),
        });
        assert_ne!(signature, semantic_project_layout_signature(&dual_source));
        Ok(())
    }

    #[test]
    fn same_revision_definition_mutation_and_document_replacement_miss() -> Result<(), ModelError> {
        let project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let inputs = project.session_layout_inputs();
        let mut cache = LayoutCache::default();
        let _ = cache
            .get_or_build_with_inputs_for_purpose(71, &project, &inputs, LayoutPurpose::Editor)
            .map_err(|_| ModelError::InvalidNumber)?;

        let mut changed = project.clone();
        let mut definitions = changed
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if let Some(action) = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == "Action")
        {
            action.font.size = action.font.size.saturating_add(1);
        }
        changed.replace_element_definitions(definitions)?;
        let (changed_layout, definition_hit) = cache
            .get_or_build_with_inputs_for_purpose(71, &changed, &inputs, LayoutPurpose::Editor)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert!(!definition_hit);
        let fresh = ScreenplayLayout::build_with_inputs_for_purpose(
            &changed,
            &inputs,
            LayoutPurpose::Editor,
        )
        .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(changed_layout.pages, fresh.pages);

        let replacement = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let (_, replacement_hit) = cache
            .get_or_build_with_inputs_for_purpose(71, &replacement, &inputs, LayoutPurpose::Editor)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert!(!replacement_hit);
        Ok(())
    }

    #[test]
    fn environment_change_forces_rebuild_and_matches_fresh_geometry() -> Result<(), ModelError> {
        let project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let inputs = project.session_layout_inputs();
        let mut cache = LayoutCache::default();
        let original = synthetic_environment(800, 10)?;
        let metric_change = synthetic_environment(801, 10)?;

        let (first, first_hit) = cache
            .get_or_build_with_environment(
                0,
                &project,
                &inputs,
                LayoutPurpose::Editor,
                LayoutComposition::BodyEditor,
                None,
                false,
                BodyEditorPresentation::default(),
                Some(&original),
            )
            .map_err(|_| ModelError::InvalidNumber)?;
        assert!(!first_hit);

        let (_, same_environment_hit) = cache
            .get_or_build_with_environment(
                0,
                &project,
                &inputs,
                LayoutPurpose::Editor,
                LayoutComposition::BodyEditor,
                None,
                false,
                BodyEditorPresentation::default(),
                Some(&original),
            )
            .map_err(|_| ModelError::InvalidNumber)?;
        assert!(same_environment_hit);

        let (changed, changed_environment_hit) = cache
            .get_or_build_with_environment(
                0,
                &project,
                &inputs,
                LayoutPurpose::Editor,
                LayoutComposition::BodyEditor,
                None,
                false,
                BodyEditorPresentation::default(),
                Some(&metric_change),
            )
            .map_err(|_| ModelError::InvalidNumber)?;
        assert!(!changed_environment_hit);
        let fresh = ScreenplayLayout::build_with_composition_and_environment(
            &project,
            &inputs,
            &project
                .preservation
                .as_ref()
                .map(|preservation| preservation.source_layout_settings().clone())
                .unwrap_or_default(),
            LayoutComposition::BodyEditor,
            &metric_change,
        )
        .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(changed.pages, fresh.pages);
        assert_ne!(first.pages, changed.pages);
        Ok(())
    }
}
