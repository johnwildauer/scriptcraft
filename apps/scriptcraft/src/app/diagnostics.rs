//! App-owned performance/native diagnostics and status/error bookkeeping.

use super::{responsiveness, ScriptCraftApp};
use crate::views::script::RevealIntent;
use eframe::egui;
use scriptcraft_commands::{Command, EditorSurface};
use scriptcraft_fdx as fdx;
use scriptcraft_model::cast_list::CAST_LIST_REFRESH_INTERVAL;
use scriptcraft_session::editing::EditError;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

pub(crate) fn safe_layout_error_category(error: &scriptcraft_layout::LayoutError) -> &'static str {
    use scriptcraft_layout::LayoutErrorKind;
    match &error.kind {
        LayoutErrorKind::Model(_) => "model validation",
        LayoutErrorKind::InvalidProfile => "invalid layout profile",
        LayoutErrorKind::InvalidPosition => "invalid layout position",
        LayoutErrorKind::NoGeometry => "layout geometry unavailable",
    }
}

#[derive(Debug, Default)]
pub(crate) struct NativeInputDiagnostics {
    pub(crate) pointer_activations: u32,
    pub(crate) received_events: u32,
    pub(crate) dispatched_frames: u32,
    pub(crate) last_event_category: &'static str,
    pub(crate) focused: bool,
    pub(crate) focus_lock_installed: bool,
    pub(crate) dispatch_enabled: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PerformanceMeasurements {
    pub(crate) frames: u64,
    pub(crate) events: u64,
    pub(crate) layout_builds: u64,
    pub(crate) layout_cache_hits: u64,
    pub(crate) painted_pages: u64,
    pub(crate) repaint_requests: u64,
    pub(crate) pages_considered: u64,
    pub(crate) pages_culled: u64,
    pub(crate) lines_submitted: u64,
    pub(crate) selection_spans_submitted: u64,
    /// ScriptCraft's fresh fixed-cell/natural galley transformations. This does
    /// not report hits in egui's internal atlas-aware layout cache.
    pub(crate) galleys_built: u64,
    /// Reuses of ScriptCraft-transformed UV-bearing galleys within a pass.
    /// Currently zero by policy: without an atlas identity we rebuild variants.
    pub(crate) galleys_reused: u64,
    pub(crate) element_wraps_built: u64,
    pub(crate) element_wraps_reused: u64,
    pub(crate) layout_identity_changes: u64,
    pub(crate) caret_lookups: u64,
    pub(crate) indexed_interaction_queries: u64,
    pub(crate) linear_interaction_queries: u64,
    pub(crate) interaction_index_entries_examined: u64,
    pub(crate) derived_refresh_attempts: u64,
    pub(crate) derived_refresh_completed: u64,
    pub(crate) derived_refresh_skipped: u64,
    pub(crate) derived_refresh_changed: u64,
    pub(crate) derived_refresh_unchanged: u64,
    pub(crate) derived_refresh_deferred: u64,
    pub(crate) derived_refresh_failed: u64,
    pub(crate) derived_refresh_irrelevant_skips: u64,
    pub(crate) derived_scenes_requested: u64,
    pub(crate) derived_scenes_attempted: u64,
    pub(crate) derived_scenes_changed: u64,
    pub(crate) derived_scenes_unchanged: u64,
    pub(crate) derived_scenes_deferred: u64,
    pub(crate) derived_project_candidate_clones: u64,
    pub(crate) derived_scene_candidate_clones: u64,
    pub(crate) derived_redundant_preservation_clones: u64,
    pub(crate) zoom_anchor_capture_successes: u64,
    pub(crate) zoom_anchor_capture_misses: u64,
    pub(crate) zoom_manual_scroll_cancellations: u64,
    pub(crate) zoom_compensations_queued: u64,
    pub(crate) zoom_geometry_published: u64,
    pub(crate) zoom_anchor_consumed: u64,
    pub(crate) frame_nanos: u128,
    pub(crate) event_nanos: u128,
    pub(crate) layout_nanos: u128,
    pub(crate) layout_lookup_nanos: u128,
    pub(crate) element_wrap_nanos: u128,
    pub(crate) paint_nanos: u128,
    pub(crate) caret_lookup_nanos: u128,
    pub(crate) derived_refresh_nanos: u128,
}

impl PerformanceMeasurements {
    fn delta_since(self, baseline: Self) -> Self {
        Self {
            frames: self.frames.saturating_sub(baseline.frames),
            events: self.events.saturating_sub(baseline.events),
            layout_builds: self.layout_builds.saturating_sub(baseline.layout_builds),
            layout_cache_hits: self
                .layout_cache_hits
                .saturating_sub(baseline.layout_cache_hits),
            painted_pages: self.painted_pages.saturating_sub(baseline.painted_pages),
            repaint_requests: self
                .repaint_requests
                .saturating_sub(baseline.repaint_requests),
            pages_considered: self
                .pages_considered
                .saturating_sub(baseline.pages_considered),
            pages_culled: self.pages_culled.saturating_sub(baseline.pages_culled),
            lines_submitted: self
                .lines_submitted
                .saturating_sub(baseline.lines_submitted),
            selection_spans_submitted: self
                .selection_spans_submitted
                .saturating_sub(baseline.selection_spans_submitted),
            galleys_built: self.galleys_built.saturating_sub(baseline.galleys_built),
            galleys_reused: self.galleys_reused.saturating_sub(baseline.galleys_reused),
            element_wraps_built: self
                .element_wraps_built
                .saturating_sub(baseline.element_wraps_built),
            element_wraps_reused: self
                .element_wraps_reused
                .saturating_sub(baseline.element_wraps_reused),
            layout_identity_changes: self
                .layout_identity_changes
                .saturating_sub(baseline.layout_identity_changes),
            caret_lookups: self.caret_lookups.saturating_sub(baseline.caret_lookups),
            indexed_interaction_queries: self
                .indexed_interaction_queries
                .saturating_sub(baseline.indexed_interaction_queries),
            linear_interaction_queries: self
                .linear_interaction_queries
                .saturating_sub(baseline.linear_interaction_queries),
            interaction_index_entries_examined: self
                .interaction_index_entries_examined
                .saturating_sub(baseline.interaction_index_entries_examined),
            derived_refresh_attempts: self
                .derived_refresh_attempts
                .saturating_sub(baseline.derived_refresh_attempts),
            derived_refresh_completed: self
                .derived_refresh_completed
                .saturating_sub(baseline.derived_refresh_completed),
            derived_refresh_skipped: self
                .derived_refresh_skipped
                .saturating_sub(baseline.derived_refresh_skipped),
            derived_refresh_changed: self
                .derived_refresh_changed
                .saturating_sub(baseline.derived_refresh_changed),
            derived_refresh_unchanged: self
                .derived_refresh_unchanged
                .saturating_sub(baseline.derived_refresh_unchanged),
            derived_refresh_deferred: self
                .derived_refresh_deferred
                .saturating_sub(baseline.derived_refresh_deferred),
            derived_refresh_failed: self
                .derived_refresh_failed
                .saturating_sub(baseline.derived_refresh_failed),
            derived_refresh_irrelevant_skips: self
                .derived_refresh_irrelevant_skips
                .saturating_sub(baseline.derived_refresh_irrelevant_skips),
            derived_scenes_requested: self
                .derived_scenes_requested
                .saturating_sub(baseline.derived_scenes_requested),
            derived_scenes_attempted: self
                .derived_scenes_attempted
                .saturating_sub(baseline.derived_scenes_attempted),
            derived_scenes_changed: self
                .derived_scenes_changed
                .saturating_sub(baseline.derived_scenes_changed),
            derived_scenes_unchanged: self
                .derived_scenes_unchanged
                .saturating_sub(baseline.derived_scenes_unchanged),
            derived_scenes_deferred: self
                .derived_scenes_deferred
                .saturating_sub(baseline.derived_scenes_deferred),
            derived_project_candidate_clones: self
                .derived_project_candidate_clones
                .saturating_sub(baseline.derived_project_candidate_clones),
            derived_scene_candidate_clones: self
                .derived_scene_candidate_clones
                .saturating_sub(baseline.derived_scene_candidate_clones),
            derived_redundant_preservation_clones: self
                .derived_redundant_preservation_clones
                .saturating_sub(baseline.derived_redundant_preservation_clones),
            zoom_anchor_capture_successes: self
                .zoom_anchor_capture_successes
                .saturating_sub(baseline.zoom_anchor_capture_successes),
            zoom_anchor_capture_misses: self
                .zoom_anchor_capture_misses
                .saturating_sub(baseline.zoom_anchor_capture_misses),
            zoom_manual_scroll_cancellations: self
                .zoom_manual_scroll_cancellations
                .saturating_sub(baseline.zoom_manual_scroll_cancellations),
            zoom_compensations_queued: self
                .zoom_compensations_queued
                .saturating_sub(baseline.zoom_compensations_queued),
            zoom_geometry_published: self
                .zoom_geometry_published
                .saturating_sub(baseline.zoom_geometry_published),
            zoom_anchor_consumed: self
                .zoom_anchor_consumed
                .saturating_sub(baseline.zoom_anchor_consumed),
            frame_nanos: self.frame_nanos.saturating_sub(baseline.frame_nanos),
            event_nanos: self.event_nanos.saturating_sub(baseline.event_nanos),
            layout_nanos: self.layout_nanos.saturating_sub(baseline.layout_nanos),
            layout_lookup_nanos: self
                .layout_lookup_nanos
                .saturating_sub(baseline.layout_lookup_nanos),
            element_wrap_nanos: self
                .element_wrap_nanos
                .saturating_sub(baseline.element_wrap_nanos),
            paint_nanos: self.paint_nanos.saturating_sub(baseline.paint_nanos),
            caret_lookup_nanos: self
                .caret_lookup_nanos
                .saturating_sub(baseline.caret_lookup_nanos),
            derived_refresh_nanos: self
                .derived_refresh_nanos
                .saturating_sub(baseline.derived_refresh_nanos),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchmarkOperation {
    General,
}

impl BenchmarkOperation {
    fn label(self) -> &'static str {
        match self {
            Self::General => "general",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PerformanceDiagnostics {
    totals: PerformanceMeasurements,
    benchmark_start: PerformanceMeasurements,
    benchmark_operation: BenchmarkOperation,
}

pub(crate) fn add_performance_counter(counter: &mut u64, amount: u64) {
    *counter = counter.saturating_add(amount);
}

pub(crate) fn add_performance_nanos(total: &mut u128, amount: u128) {
    *total = total.saturating_add(amount);
}

impl Default for PerformanceDiagnostics {
    fn default() -> Self {
        Self {
            totals: PerformanceMeasurements::default(),
            benchmark_start: PerformanceMeasurements::default(),
            benchmark_operation: BenchmarkOperation::General,
        }
    }
}

impl std::ops::Deref for PerformanceDiagnostics {
    type Target = PerformanceMeasurements;

    fn deref(&self) -> &Self::Target {
        &self.totals
    }
}

impl std::ops::DerefMut for PerformanceDiagnostics {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.totals
    }
}

impl PerformanceDiagnostics {
    fn benchmark_delta(&self) -> PerformanceMeasurements {
        self.totals.delta_since(self.benchmark_start)
    }

    #[allow(dead_code)] // Retained instrumentation is intentionally not displayed.
    fn safe_summary(&self) -> String {
        let build_profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let totals = self.totals;
        let benchmark = self.benchmark_delta();
        format!(
            "profile={build_profile} operation={} frames={} events={} layouts={} cache-hits={} layout-identity-changes={} pages-considered={} pages-culled={} pages-painted={} lines={} selection-spans={} galleys-built={} galleys-reused={} wraps-built={} wraps-reused={} caret-lookups={} interaction-indexed={} interaction-linear={} interaction-index-entries={} derived-attempted={} derived-completed={} derived-skipped={} derived-irrelevant-skips={} derived-changed={} derived-unchanged={} derived-deferred={} derived-failed={} derived-scenes-requested={} derived-scenes-attempted={} derived-scenes-changed={} derived-scenes-unchanged={} derived-scenes-deferred={} derived-project-candidates={} derived-scene-candidates={} derived-redundant-preservation-clones={} zoom-anchor-capture-success={} zoom-anchor-capture-miss={} zoom-scroll-canceled={} zoom-compensation-queued={} zoom-geometry-published={} zoom-anchor-consumed={} frame-ns={} event-ns={} layout-lookup-ns={} element-wrap-ns={} page-paint-ns={} caret-lookup-ns={} derived-refresh-ns={} benchmark-frames={} benchmark-layouts={} benchmark-pages-painted={} benchmark-wraps-built={} benchmark-linear-queries={} benchmark-indexed-queries={} benchmark-index-entries={}",
            self.benchmark_operation.label(),
            totals.frames,
            totals.events,
            totals.layout_builds,
            totals.layout_cache_hits,
            totals.layout_identity_changes,
            totals.pages_considered,
            totals.pages_culled,
            totals.painted_pages,
            totals.lines_submitted,
            totals.selection_spans_submitted,
            totals.galleys_built,
            totals.galleys_reused,
            totals.element_wraps_built,
            totals.element_wraps_reused,
            totals.caret_lookups,
            totals.indexed_interaction_queries,
            totals.linear_interaction_queries,
            totals.interaction_index_entries_examined,
            totals.derived_refresh_attempts,
            totals.derived_refresh_completed,
            totals.derived_refresh_skipped,
            totals.derived_refresh_irrelevant_skips,
            totals.derived_refresh_changed,
            totals.derived_refresh_unchanged,
            totals.derived_refresh_deferred,
            totals.derived_refresh_failed,
            totals.derived_scenes_requested,
            totals.derived_scenes_attempted,
            totals.derived_scenes_changed,
            totals.derived_scenes_unchanged,
            totals.derived_scenes_deferred,
            totals.derived_project_candidate_clones,
            totals.derived_scene_candidate_clones,
            totals.derived_redundant_preservation_clones,
            totals.zoom_anchor_capture_successes,
            totals.zoom_anchor_capture_misses,
            totals.zoom_manual_scroll_cancellations,
            totals.zoom_compensations_queued,
            totals.zoom_geometry_published,
            totals.zoom_anchor_consumed,
            totals.frame_nanos,
            totals.event_nanos,
            totals.layout_lookup_nanos,
            totals.element_wrap_nanos,
            totals.paint_nanos,
            totals.caret_lookup_nanos,
            totals.derived_refresh_nanos,
            benchmark.frames,
            benchmark.layout_builds,
            benchmark.painted_pages,
            benchmark.element_wraps_built,
            benchmark.linear_interaction_queries,
            benchmark.indexed_interaction_queries,
            benchmark.interaction_index_entries_examined
        )
    }
}

fn scheduled_refresh_failure_status(category: &'static str) -> String {
    format!(
        "CAST_LIST_REFRESH_FAILED: retry suspended until relevant document changes; category={category}; screenplay unchanged"
    )
}

impl ScriptCraftApp {
    pub(crate) fn request_repaint(&mut self, ctx: &egui::Context) {
        self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
        ctx.request_repaint();
    }

    pub(crate) fn schedule_sprint_timer_repaint(&mut self, ctx: &egui::Context, now: Duration) {
        if let Some(delay) = self.sprint_timer.next_repaint_delay(now) {
            self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
            ctx.request_repaint_after(delay);
        }
    }

    pub(crate) fn schedule_focus_mode_repaint(&mut self, ctx: &egui::Context, now: Duration) {
        if let Some(delay) = self.focus_mode.next_repaint_delay(now) {
            self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
            ctx.request_repaint_after(delay);
        }
    }

    pub(crate) fn refresh_cast_lists_on_schedule(&mut self, ctx: &egui::Context, now: Instant) {
        let started = Instant::now();
        if now < self.next_cast_list_refresh {
            self.performance.derived_refresh_skipped =
                self.performance.derived_refresh_skipped.saturating_add(1);
            self.performance.derived_refresh_nanos = self
                .performance
                .derived_refresh_nanos
                .saturating_add(started.elapsed().as_nanos());
            ctx.request_repaint_after(self.next_cast_list_refresh.saturating_duration_since(now));
            return;
        }

        let pending = self.session.pending_derived_work_summary();
        if !pending.retryable {
            self.next_cast_list_refresh = now + CAST_LIST_REFRESH_INTERVAL;
            self.performance.derived_refresh_skipped =
                self.performance.derived_refresh_skipped.saturating_add(1);
            if !pending.pending {
                self.performance.derived_refresh_irrelevant_skips = self
                    .performance
                    .derived_refresh_irrelevant_skips
                    .saturating_add(1);
            }
            self.performance.derived_refresh_nanos = self
                .performance
                .derived_refresh_nanos
                .saturating_add(started.elapsed().as_nanos());
            self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
            ctx.request_repaint_after(CAST_LIST_REFRESH_INTERVAL);
            return;
        }

        self.performance.derived_refresh_attempts =
            self.performance.derived_refresh_attempts.saturating_add(1);
        self.next_cast_list_refresh = now + CAST_LIST_REFRESH_INTERVAL;
        match self.session.refresh_cast_lists_with_outcome() {
            Ok(outcome) => {
                self.performance.derived_refresh_completed =
                    self.performance.derived_refresh_completed.saturating_add(1);
                if outcome.changed {
                    self.performance.derived_refresh_changed =
                        self.performance.derived_refresh_changed.saturating_add(1);
                    self.request_repaint(ctx);
                } else {
                    self.performance.derived_refresh_unchanged =
                        self.performance.derived_refresh_unchanged.saturating_add(1);
                }
                if outcome.deferred {
                    self.performance.derived_refresh_deferred =
                        self.performance.derived_refresh_deferred.saturating_add(1);
                }
                add_performance_counter(
                    &mut self.performance.derived_scenes_requested,
                    outcome.requested_scenes,
                );
                add_performance_counter(
                    &mut self.performance.derived_scenes_attempted,
                    outcome.attempted_scenes,
                );
                add_performance_counter(
                    &mut self.performance.derived_scenes_changed,
                    outcome.changed_scenes,
                );
                add_performance_counter(
                    &mut self.performance.derived_scenes_unchanged,
                    outcome.unchanged_scenes,
                );
                add_performance_counter(
                    &mut self.performance.derived_scenes_deferred,
                    outcome.deferred_scenes,
                );
                add_performance_counter(
                    &mut self.performance.derived_project_candidate_clones,
                    outcome.project_candidate_clones,
                );
                add_performance_counter(
                    &mut self.performance.derived_scene_candidate_clones,
                    outcome.scene_candidate_clones,
                );
                add_performance_counter(
                    &mut self.performance.derived_redundant_preservation_clones,
                    outcome.redundant_preservation_clones,
                );
            }
            Err(error) => {
                self.performance.derived_refresh_failed =
                    self.performance.derived_refresh_failed.saturating_add(1);
                self.show_error(scheduled_refresh_failure_status(
                    error.scheduled_refresh_failure_category(),
                ));
            }
        }
        self.performance.derived_refresh_nanos = self
            .performance
            .derived_refresh_nanos
            .saturating_add(started.elapsed().as_nanos());
        self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
        ctx.request_repaint_after(CAST_LIST_REFRESH_INTERVAL);
    }

    pub(crate) fn record_native_event(&mut self, category: &'static str) {
        self.native_input_diagnostics.received_events = self
            .native_input_diagnostics
            .received_events
            .saturating_add(1);
        self.native_input_diagnostics.last_event_category = category;
    }

    pub(crate) fn show_error(&mut self, detail: impl AsRef<str>) {
        let health = self.session_health();
        self.post_diagnostic(&format!("{}; {health}", detail.as_ref()));
    }

    pub(crate) fn show_edit_error(&mut self, error: EditError) {
        if let Some(category) = error.managed_text_mismatch_diagnostic() {
            self.show_error(format!(
                "EDIT_MANAGED_TEXT_MISMATCH: {category}; document unchanged"
            ));
        } else {
            let code = match error {
                EditError::InvalidElement => "EDIT_INVALID_ELEMENT",
                EditError::InvalidUtf8Boundary => "EDIT_INVALID_TEXT_BOUNDARY",
                EditError::InvalidSelection => "EDIT_INVALID_SELECTION",
                EditError::EmptyHistory => "EDIT_EMPTY_HISTORY",
                EditError::TextTooLong => "EDIT_TEXT_LIMIT",
                EditError::TooManyElements => "EDIT_ELEMENT_LIMIT",
                EditError::InlineAltsRoleChange => "EDIT_INLINE_ALTS_ROLE_CHANGE",
                EditError::Preservation(_) => "EDIT_PRESERVATION_CONSTRAINT",
                EditError::InvalidDefinitionCandidate => "EDIT_INVALID_DEFINITION_CANDIDATE",
                EditError::CastList(_) => "EDIT_CAST_LIST_CONSTRAINT",
                EditError::FormattedText(_) => "EDIT_FORMATTING_CONSTRAINT",
                EditError::Model(_) => "EDIT_MODEL_VALIDATION",
            };
            self.show_error(format!("{code}; document unchanged"));
        }
    }

    fn session_health(&self) -> String {
        let validation = match self.session.validate() {
            Ok(()) => "session valid",
            Err(_) => "session validation failed",
        };
        let serialization = if fdx::write_v6(self.session.project()).is_ok() {
            "FDX projection valid"
        } else {
            "FDX projection unavailable"
        };
        format!("{validation}; {serialization}")
    }

    pub(crate) fn clear_error(&mut self) {
        self.pending_diagnostic_commands
            .push_back(Command::UiStatusSet { message: None });
    }

    pub(crate) fn mutate(&mut self, result: Result<(), EditError>, reveal_intent: RevealIntent) {
        match result {
            Ok(()) => {
                let surface = self.active_editor_surface();
                if surface == EditorSurface::TitlePage && !self.title_view_state_swapped {
                    self.title_view.preferred_vertical_x = None;
                    self.title_view.reveal_intent = reveal_intent;
                } else {
                    self.preferred_vertical_x = None;
                    self.reveal_intent = reveal_intent;
                }
                self.clear_error();
                self.invalidate_surface(surface);
                self.reconcile_body_active_group();
            }
            Err(error) => self.show_edit_error(error),
        }
    }
}

pub(crate) fn measure_root_other_bucket<T>(
    app: &mut ScriptCraftApp,
    bucket: responsiveness::RootOtherBucket,
    work: impl FnOnce(&mut ScriptCraftApp) -> T,
) -> T {
    let started = app.development_mode.then(|| {
        app.responsiveness.begin_other_bucket(bucket);
        Instant::now()
    });
    let result = work(app);
    if let Some(started) = started {
        app.responsiveness
            .end_other_bucket(bucket, started.elapsed());
    }
    result
}
