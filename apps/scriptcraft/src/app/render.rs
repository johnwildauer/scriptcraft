//! Session-aware layout lookup for desktop rendering.

use super::{diagnostics::add_performance_nanos, responsiveness, ScriptCraftApp};
use scriptcraft_commands::EditorSurface;
use scriptcraft_layout::{
    BodyEditorPresentation, LayoutBuildDiagnostics, LayoutEnvironment, LayoutErrorKind,
    ScreenplayLayout,
};
use std::time::Instant;

impl ScriptCraftApp {
    pub(crate) fn layout_for_render(
        &mut self,
        surface: EditorSurface,
    ) -> Result<std::sync::Arc<ScreenplayLayout>, scriptcraft_layout::LayoutError> {
        let started = Instant::now();
        if surface == EditorSurface::Body {
            self.reconcile_body_active_group();
        }
        let prepared_fonts = self
            .document_font_store
            .prepare_document(self.session.project())
            .map_err(|_| scriptcraft_layout::LayoutError {
                kind: LayoutErrorKind::NoGeometry,
            })?;
        let layout_environment = LayoutEnvironment::new(
            self.session.layout_inputs().page_only(),
            prepared_fonts.environment.clone(),
        );
        self.prepared_document_fonts = Some(prepared_fonts);
        let cache = match surface {
            EditorSurface::Body => &mut self.layout_cache,
            EditorSurface::TitlePage => &mut self.title_layout_cache,
        };
        let mut build_diagnostics = self.development_mode.then(LayoutBuildDiagnostics::default);
        let (layout, hit) = if surface == EditorSurface::Body {
            let presentation = BodyEditorPresentation::for_project(
                self.session.project(),
                self.active_body_dual_dialogue_group.clone(),
            );
            if let Some(diagnostics) = build_diagnostics.as_mut() {
                cache.get_or_build_body_editor_for_session_with_environment_and_diagnostics(
                    &self.session,
                    &presentation,
                    &layout_environment,
                    diagnostics,
                )?
            } else {
                cache.get_or_build_body_editor_for_session_with_environment(
                    &self.session,
                    &presentation,
                    &layout_environment,
                )?
            }
        } else if let Some(diagnostics) = build_diagnostics.as_mut() {
            cache.get_or_build_title_page_editor_for_session_with_environment_and_diagnostics(
                &self.session,
                &layout_environment,
                diagnostics,
            )?
        } else {
            cache.get_or_build_title_page_editor_for_session_with_environment(
                &self.session,
                &layout_environment,
            )?
        };
        let elapsed = started.elapsed();
        add_performance_nanos(
            &mut self.performance.layout_lookup_nanos,
            elapsed.as_nanos(),
        );
        if self.development_mode {
            self.responsiveness.record_layout(elapsed, hit);
            if !hit {
                if let Some(observation) = build_diagnostics.as_ref() {
                    self.responsiveness.record_layout_build(observation);
                }
            }
        }
        if hit {
            self.performance.layout_cache_hits =
                self.performance.layout_cache_hits.saturating_add(1);
        } else {
            self.performance.layout_builds = self.performance.layout_builds.saturating_add(1);
            self.performance.layout_nanos = self
                .performance
                .layout_nanos
                .saturating_add(elapsed.as_nanos());
        }
        let identity_changed = self.last_layout_identity.as_ref().is_some_and(|previous| {
            previous
                .upgrade()
                .is_none_or(|previous| !std::sync::Arc::ptr_eq(&previous, &layout))
        });
        if identity_changed {
            self.performance.layout_identity_changes =
                self.performance.layout_identity_changes.saturating_add(1);
        }
        self.last_layout_identity = Some(std::sync::Arc::downgrade(&layout));
        Ok(layout)
    }
}
