#![deny(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use eframe::egui::{
    self, Color32, FontFamily, FontId, Key, RichText, Sense, Stroke, TextStyle, TextWrapMode, Vec2,
    WidgetText,
};
use egui::epaint::text::{Galley, LayoutJob};
use scriptcraft_core::cast_list::CAST_LIST_REFRESH_INTERVAL;
use scriptcraft_core::editing::{EditError, EditPolicy, EditingSession, Position, Selection};
use scriptcraft_core::exports;
use scriptcraft_core::fdx::{self, FdxImportOutcome, FdxMigrationConsent, FdxVersionStatus};
use scriptcraft_core::fdx_storage::{
    self, FdxBaseline, FdxSaveMode, FdxSaveTarget, OpenedFdxSource, StdFdxStorageFileSystem,
};
use scriptcraft_core::layout::{
    Caret, FixedRect, InteractionQueryDiagnostics, LayoutBuildDiagnostics, LayoutCache,
    LayoutPurpose, ScreenplayLayout,
};
use scriptcraft_core::persistence::{FileSystem, StdFileSystem};
use scriptcraft_core::{FixedPoint, Project, Role};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

mod document_font;
mod focus_mode;
mod sprint_timer;
mod status_bar;
mod toolbar;
use sprint_timer::{NotificationResult, NotificationSink};
use status_bar::{DiagnosticProjection, ViewportCenterAnchor, ViewportPageTracker};
use toolbar::{toolbar_chrome_color, toolbar_content_height};

const EDITOR_SURFACE_ID: &str = "page-editor-surface";
const CARET_WIDTH: f32 = 1.5;
const PAGE_HORIZONTAL_PADDING: f32 = 16.0;
const PAGE_VERTICAL_GAP: f32 = 12.0;
const PAGE_PAINT_OVERSCAN: usize = 1;
const FIXED_CELL_GALLEY_CACHE_ID: &str = "scriptcraft-fixed-cell-galley-cache-v1";
const FIXED_CELL_GALLEY_CACHE_LIMIT: usize = 16_384;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

fn viewport_paint_page_range(
    page_rects: &[egui::Rect],
    editor_clip: egui::Rect,
) -> Option<(usize, usize)> {
    let mut first_visible = None;
    let mut last_visible = None;
    for (page_index, page_rect) in page_rects.iter().enumerate() {
        if page_rect.intersects(editor_clip) {
            first_visible.get_or_insert(page_index);
            last_visible = Some(page_index);
        }
    }
    let first_visible = first_visible?;
    let last_visible = last_visible?;
    Some((
        first_visible.saturating_sub(PAGE_PAINT_OVERSCAN),
        last_visible
            .saturating_add(PAGE_PAINT_OVERSCAN)
            .min(page_rects.len().saturating_sub(1)),
    ))
}

fn has_extension(path: &std::path::Path, expected: &str) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn looks_like_legacy_json(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    bytes
        .iter()
        .copied()
        .find(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        == Some(b'{')
}

struct PendingFdxOpen {
    path: PathBuf,
    source: OpenedFdxSource,
    declared_version: String,
}

#[derive(Debug, Default)]
struct NativeInputDiagnostics {
    pointer_activations: u32,
    received_events: u32,
    dispatched_frames: u32,
    last_event_category: &'static str,
    focused: bool,
    focus_lock_installed: bool,
    dispatch_enabled: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct PerformanceMeasurements {
    frames: u64,
    events: u64,
    layout_builds: u64,
    layout_cache_hits: u64,
    painted_pages: u64,
    repaint_requests: u64,
    pages_considered: u64,
    pages_culled: u64,
    lines_submitted: u64,
    selection_spans_submitted: u64,
    galleys_built: u64,
    galleys_reused: u64,
    element_wraps_built: u64,
    element_wraps_reused: u64,
    layout_identity_changes: u64,
    caret_lookups: u64,
    indexed_interaction_queries: u64,
    linear_interaction_queries: u64,
    interaction_index_entries_examined: u64,
    derived_refresh_attempts: u64,
    derived_refresh_completed: u64,
    derived_refresh_skipped: u64,
    derived_refresh_changed: u64,
    derived_refresh_unchanged: u64,
    derived_refresh_deferred: u64,
    derived_refresh_failed: u64,
    derived_refresh_irrelevant_skips: u64,
    derived_scenes_requested: u64,
    derived_scenes_attempted: u64,
    derived_scenes_changed: u64,
    derived_scenes_unchanged: u64,
    derived_scenes_deferred: u64,
    derived_project_candidate_clones: u64,
    derived_scene_candidate_clones: u64,
    derived_redundant_preservation_clones: u64,
    zoom_anchor_capture_successes: u64,
    zoom_anchor_capture_misses: u64,
    zoom_manual_scroll_cancellations: u64,
    zoom_compensations_queued: u64,
    zoom_geometry_published: u64,
    zoom_anchor_consumed: u64,
    frame_nanos: u128,
    event_nanos: u128,
    layout_nanos: u128,
    layout_lookup_nanos: u128,
    element_wrap_nanos: u128,
    paint_nanos: u128,
    caret_lookup_nanos: u128,
    derived_refresh_nanos: u128,
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
struct PerformanceDiagnostics {
    totals: PerformanceMeasurements,
    benchmark_start: PerformanceMeasurements,
    benchmark_operation: BenchmarkOperation,
}

fn add_performance_counter(counter: &mut u64, amount: u64) {
    *counter = counter.saturating_add(amount);
}

fn add_performance_nanos(total: &mut u128, amount: u128) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevealIntent {
    None,
    Conditional,
}

struct ScriptCraftApp {
    session: EditingSession,
    focus_mode: focus_mode::FocusModeController,
    sprint_timer: sprint_timer::SprintTimer,
    timer_clock_origin: Instant,
    next_cast_list_refresh: Instant,
    notification_sink: sprint_timer::NativeNotificationSink,
    notification_results: Receiver<(u64, NotificationResult)>,
    surface_focused: bool,
    drag_anchor: Option<Position>,
    ime_preedit: String,
    ime_active: bool,
    rendered_revision: Option<u64>,
    status: String,
    error: Option<String>,
    native_path: Option<PathBuf>,
    fdx_baseline: Option<FdxBaseline>,
    pending_window_restore: Option<fdx::WindowState>,
    latest_window_state: Option<fdx::WindowState>,
    pending_fdx_open: Option<PendingFdxOpen>,
    native_input_diagnostics: NativeInputDiagnostics,
    editor_surface_id: Option<egui::Id>,
    editor_surface_rect: Option<egui::Rect>,
    reveal_intent: RevealIntent,
    preferred_vertical_x: Option<i64>,
    page_geometry: ViewportPageTracker,
    pending_zoom_anchor: Option<ViewportCenterAnchor>,
    page_scale: Option<f32>,
    effective_pixels_per_point: Option<f32>,
    zoom_percent: u16,
    pinch_zoom_percent: f64,
    appearance_mode: status_bar::AppearanceMode,
    applied_appearance: Option<(status_bar::AppearanceMode, egui::Theme)>,
    page_palette: status_bar::PagePalette,
    caret_blink_focus_generation: u64,
    caret_blink_reset_time: f64,
    layout_cache: LayoutCache,
    performance: PerformanceDiagnostics,
    last_layout_identity: Option<std::sync::Weak<ScreenplayLayout>>,
}

fn scheduled_refresh_failure_status(category: &'static str) -> String {
    format!(
        "Cast-list refresh failed and is suspended until relevant document changes (failure category: {category})."
    )
}

impl ScriptCraftApp {
    fn new() -> Result<Self, String> {
        let project = Project::new().map_err(|error| format!("new project failed: {error:?}"))?;
        let session = EditingSession::with_policy(project, EditPolicy::default())
            .map_err(|error| format!("session failed: {error:?}"))?;
        let (notification_sink, notification_results) = sprint_timer::NativeNotificationSink::new();
        Ok(Self {
            session,
            focus_mode: focus_mode::FocusModeController::default(),
            sprint_timer: sprint_timer::SprintTimer::default(),
            timer_clock_origin: Instant::now(),
            next_cast_list_refresh: Instant::now() + CAST_LIST_REFRESH_INTERVAL,
            notification_sink,
            notification_results,
            surface_focused: false,
            drag_anchor: None,
            ime_preedit: String::new(),
            ime_active: false,
            rendered_revision: None,
            status: String::new(),
            error: None,
            native_path: None,
            fdx_baseline: None,
            pending_window_restore: None,
            latest_window_state: None,
            pending_fdx_open: None,
            native_input_diagnostics: NativeInputDiagnostics::default(),
            editor_surface_id: None,
            editor_surface_rect: None,
            reveal_intent: RevealIntent::None,
            preferred_vertical_x: None,
            page_geometry: ViewportPageTracker::default(),
            pending_zoom_anchor: None,
            page_scale: None,
            effective_pixels_per_point: None,
            zoom_percent: 100,
            pinch_zoom_percent: 100.0,
            appearance_mode: status_bar::AppearanceMode::System,
            applied_appearance: None,
            page_palette: status_bar::PagePalette::for_theme(
                status_bar::AppearanceMode::System,
                egui::Theme::Light,
            ),
            caret_blink_focus_generation: 0,
            caret_blink_reset_time: 0.0,
            layout_cache: LayoutCache::default(),
            performance: PerformanceDiagnostics::default(),
            last_layout_identity: None,
        })
    }

    fn request_repaint(&mut self, ctx: &egui::Context) {
        self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
        ctx.request_repaint();
    }

    fn timer_now(&self) -> Duration {
        self.timer_clock_origin.elapsed()
    }

    fn reconcile_sprint_timer(&mut self, now: Duration) {
        if let Some(notice) = self.sprint_timer.reconcile(now) {
            let result = self.notification_sink.request(notice);
            self.sprint_timer
                .apply_notification_result(notice.generation, result);
        }
        while let Ok((generation, result)) = self.notification_results.try_recv() {
            self.sprint_timer
                .apply_notification_result(generation, result);
        }
    }

    fn schedule_sprint_timer_repaint(&mut self, ctx: &egui::Context, now: Duration) {
        if let Some(delay) = self.sprint_timer.next_repaint_delay(now) {
            self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
            ctx.request_repaint_after(delay);
        }
    }

    fn schedule_focus_mode_repaint(&mut self, ctx: &egui::Context, now: Duration) {
        if let Some(delay) = self.focus_mode.next_repaint_delay(now) {
            self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
            ctx.request_repaint_after(delay);
        }
    }

    fn refresh_cast_lists_on_schedule(&mut self, ctx: &egui::Context, now: Instant) {
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
                self.status =
                    scheduled_refresh_failure_status(error.scheduled_refresh_failure_category());
            }
        }
        self.performance.derived_refresh_nanos = self
            .performance
            .derived_refresh_nanos
            .saturating_add(started.elapsed().as_nanos());
        self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
        ctx.request_repaint_after(CAST_LIST_REFRESH_INTERVAL);
    }

    fn sync_pinch_zoom_from_controls(&mut self) {
        self.pinch_zoom_percent = f64::from(self.zoom_percent);
    }

    fn capture_pending_zoom_anchor(&mut self, ctx: &egui::Context) {
        if self.pending_zoom_anchor.is_none() {
            let anchor = self.page_geometry.capture_center_anchor(
                self.performance.frames,
                self.session.revision(),
                ctx.content_rect(),
                self.page_scale.unwrap_or(f32::NAN),
            );
            if anchor.is_some() {
                add_performance_counter(&mut self.performance.zoom_anchor_capture_successes, 1);
            } else {
                add_performance_counter(&mut self.performance.zoom_anchor_capture_misses, 1);
            }
            self.pending_zoom_anchor = anchor;
        }
    }

    fn layout_for_render(
        &mut self,
    ) -> Result<std::sync::Arc<ScreenplayLayout>, scriptcraft_core::layout::LayoutError> {
        let revision = self.session.revision();
        let started = Instant::now();
        let mut build_diagnostics = LayoutBuildDiagnostics::default();
        let (layout, hit) = self
            .layout_cache
            .get_or_build_for_session_with_diagnostics(
                revision,
                &self.session,
                LayoutPurpose::Editor,
                &mut build_diagnostics,
            )?;
        add_performance_nanos(
            &mut self.performance.layout_lookup_nanos,
            started.elapsed().as_nanos(),
        );
        if hit {
            self.performance.layout_cache_hits =
                self.performance.layout_cache_hits.saturating_add(1);
        } else {
            self.performance.layout_builds = self.performance.layout_builds.saturating_add(1);
            self.performance.layout_nanos = self
                .performance
                .layout_nanos
                .saturating_add(started.elapsed().as_nanos());
            self.performance.element_wraps_built = self
                .performance
                .element_wraps_built
                .saturating_add(build_diagnostics.element_wraps_built);
            self.performance.element_wraps_reused = self
                .performance
                .element_wraps_reused
                .saturating_add(build_diagnostics.element_wraps_reused);
            self.performance.element_wrap_nanos = self
                .performance
                .element_wrap_nanos
                .saturating_add(build_diagnostics.element_wrap_nanos);
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

    fn save_native(&mut self) {
        let Some(path) = self.native_path.clone() else {
            self.save_as_native();
            return;
        };
        self.save_native_at(&path);
    }

    fn save_native_at(&mut self, path: &std::path::Path) {
        if has_extension(path, "json") {
            self.show_error(
                "Legacy JSON saving is retired; the destination and current document are unchanged",
            );
            return;
        }

        let mode = match self.fdx_baseline.as_ref() {
            Some(baseline) if baseline.path() == path => FdxSaveMode::Save,
            _ => FdxSaveMode::SaveAs(path.to_owned()),
        };
        self.save_fdx_mode_at(path, mode);
    }

    fn save_fdx_as_at(&mut self, path: &std::path::Path) {
        if !has_extension(path, "fdx") {
            self.show_error("Native Save As requires an .fdx destination");
            return;
        }
        self.save_fdx_mode_at(path, FdxSaveMode::SaveAs(path.to_owned()));
    }

    fn save_fdx_mode_at(&mut self, path: &std::path::Path, mode: FdxSaveMode) {
        if matches!(&mode, FdxSaveMode::SaveAs(_)) && !has_extension(path, "fdx") {
            self.show_error("Native Save As requires an .fdx destination");
            return;
        }
        if self.fdx_baseline.is_none()
            && self
                .native_path
                .as_deref()
                .is_some_and(|source_path| has_extension(source_path, "json"))
        {
            self.show_error(
                "FDX save refused: legacy JSON-origin state has no FDX mapping; source and live session are unchanged",
            );
            return;
        }
        let mut target = self
            .fdx_baseline
            .clone()
            .map(FdxSaveTarget::from_baseline)
            .unwrap_or_else(FdxSaveTarget::new_unsaved);
        let text_state = fdx::TextState {
            scaling: self.zoom_percent.clamp(
                scriptcraft_core::text_state::MIN_TEXT_SCALING,
                scriptcraft_core::text_state::MAX_TEXT_SCALING,
            ),
            selection: self.session.selection(),
        };
        match fdx_storage::save_session_with_view_state(
            &StdFdxStorageFileSystem,
            &mut self.session,
            &mut target,
            mode,
            self.latest_window_state
                .unwrap_or(fdx::WindowState::DEFAULT),
            text_state,
        ) {
            Ok(outcome) => {
                self.native_path = target.path().map(PathBuf::from);
                self.fdx_baseline = target.baseline().cloned();
                // Save preflight always prepares and publishes complete derived
                // data. The session's pending-work state, not a revision marker,
                // keeps the timer from repeating that work.
                self.next_cast_list_refresh = Instant::now() + CAST_LIST_REFRESH_INTERVAL;
                self.layout_cache = LayoutCache::default();
                self.page_geometry.invalidate();
                self.rendered_revision = None;
                self.preferred_vertical_x = None;
                self.status = if outcome.temporary_cleanup_warning {
                    "FDX saved; temporary-file cleanup warning".into()
                } else {
                    "Saved FDX document".into()
                };
                self.clear_error();
            }
            Err(error) => self.show_fdx_save_error(error),
        }
    }

    fn show_fdx_save_error(&mut self, error: fdx_storage::TransactionalSaveError) {
        let detail = error.to_string();
        let health = self.session_health();
        self.error = Some(detail.clone());
        self.status = format!("FDX save failed; document unchanged; {health}");
    }

    fn open_native(&mut self) {
        let dialog = rfd::FileDialog::new().add_filter("Final Draft document", &["fdx"]);
        if let Some(path) = dialog.pick_file() {
            self.open_native_at(&path);
        }
    }

    fn open_native_at(&mut self, path: &std::path::Path) {
        self.open_staged_native_at(path);
    }

    /// Format selection is based on bounded file content; the extension only
    /// controls the file-picker filter, never the decoder.
    fn open_staged_native_at(&mut self, path: &std::path::Path) {
        if self.pending_fdx_open.is_some() {
            self.status = "Resolve the pending FDX import before opening another document".into();
            return;
        }
        if self.session.dirty() {
            self.error = None;
            self.status = "Open cancelled: save the unsaved project first".into();
            return;
        }
        let source = match fdx_storage::open_source(&StdFdxStorageFileSystem, path) {
            Ok(source) => source,
            Err(error) => {
                self.show_error(format!("open failed: {error:?}"));
                return;
            }
        };
        if looks_like_legacy_json(source.bytes()) {
            self.show_error(
                "Legacy ScriptCraft JSON is no longer supported; the file and active document are unchanged",
            );
            return;
        }

        match fdx::preflight_version(source.bytes()) {
            Ok(FdxVersionStatus::Version6) => {
                self.finish_fdx_import(path.to_owned(), source, FdxMigrationConsent::Continue);
            }
            Ok(FdxVersionStatus::ExplicitNon6 { declared, .. }) => {
                self.pending_fdx_open = Some(PendingFdxOpen {
                    path: path.to_owned(),
                    source,
                    declared_version: declared,
                });
                self.clear_error();
                self.status = "FDX version update requires confirmation".into();
            }
            Ok(FdxVersionStatus::Missing) => {
                self.show_error("incompatible FDX: version is missing")
            }
            Ok(FdxVersionStatus::Malformed) => {
                self.show_error("incompatible FDX: version declaration is invalid")
            }
            Ok(FdxVersionStatus::Duplicate) => {
                self.show_error("incompatible FDX: duplicate version declarations")
            }
            Err(error) => self.show_error(format!("incompatible FDX: {error:?}")),
        }
    }

    fn finish_fdx_import(
        &mut self,
        path: PathBuf,
        source: OpenedFdxSource,
        consent: FdxMigrationConsent,
    ) {
        if consent == FdxMigrationConsent::Cancel {
            self.status = "FDX import cancelled; active document unchanged".into();
            self.clear_error();
            return;
        }
        let outcome = fdx::import_fdx(source.bytes(), consent);
        match outcome {
            Ok(FdxImportOutcome::Candidate(candidate)) => {
                let migration_status = match candidate.migration {
                    None => None,
                    Some(fdx::FdxMigrationKind::OlderLossy { .. }) => {
                        Some("Opened previous-version FDX; some data may be lost")
                    }
                    Some(fdx::FdxMigrationKind::NewerBestEffort { .. }) => {
                        Some("Opened non-current FDX with best-effort Version 6 import")
                    }
                };
                self.activate_opened_project(
                    candidate.document.project,
                    &path,
                    Some(source.baseline),
                    migration_status,
                    candidate.document.window_state,
                    candidate.document.text_state,
                );
            }
            Ok(FdxImportOutcome::Cancelled { .. }) => {
                self.status = "FDX import cancelled; active document unchanged".into();
                self.clear_error();
            }
            Err(error) => self.show_error(format!("incompatible FDX document: {error:?}")),
        }
    }

    fn activate_opened_project(
        &mut self,
        project: Project,
        path: &std::path::Path,
        baseline: Option<FdxBaseline>,
        migration_status: Option<&str>,
        window_state: Option<fdx::WindowState>,
        text_state: fdx::TextState,
    ) {
        let mut replacement = match EditingSession::with_policy(project, self.session.policy()) {
            Ok(replacement) => replacement,
            Err(error) => {
                self.show_error(format!("incompatible document: {error:?}"));
                return;
            }
        };
        if replacement.set_selection(text_state.selection).is_err() {
            let _ = replacement.set_selection(Selection::caret(Position {
                element: 0,
                offset: 0,
            }));
        }
        self.session = replacement;
        self.zoom_percent = text_state.scaling.clamp(
            scriptcraft_core::text_state::MIN_TEXT_SCALING,
            scriptcraft_core::text_state::MAX_TEXT_SCALING,
        );
        self.sync_pinch_zoom_from_controls();
        self.pending_zoom_anchor = None;
        self.native_path = Some(path.to_owned());
        self.fdx_baseline = baseline;
        self.pending_window_restore = window_state;
        self.pending_fdx_open = None;
        self.next_cast_list_refresh = Instant::now() + CAST_LIST_REFRESH_INTERVAL;
        self.page_geometry.reset_for_document_replacement();
        self.layout_cache = LayoutCache::default();
        self.rendered_revision = None;
        self.preferred_vertical_x = None;
        self.drag_anchor = None;
        self.ime_preedit.clear();
        self.ime_active = false;
        self.status = migration_status
            .map(str::to_owned)
            .unwrap_or_else(|| "Opened FDX document".into());
        self.clear_error();
    }

    fn resolve_pending_fdx_import(&mut self, consent: FdxMigrationConsent) {
        let Some(pending) = self.pending_fdx_open.take() else {
            return;
        };
        if consent == FdxMigrationConsent::Cancel {
            self.status = "FDX import cancelled; active document unchanged".into();
            self.clear_error();
            return;
        }
        if self.session.dirty() {
            self.status = "Open cancelled: save the unsaved project first".into();
            self.error = None;
            return;
        }
        self.finish_fdx_import(pending.path, pending.source, consent);
    }

    fn save_as_native(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Final Draft document", &["fdx"])
            .set_file_name("Untitled Screenplay.fdx")
            .save_file()
        {
            if !has_extension(&path, "fdx") {
                self.show_error("Native Save As requires an .fdx destination");
                return;
            }
            self.save_fdx_as_at(&path);
        }
    }

    fn export_pdf(&mut self) {
        match exports::export_pdf_with_layout_inputs(
            self.session.project(),
            self.session.layout_inputs(),
        ) {
            Ok(bytes) => match StdFileSystem.write(&PathBuf::from("scriptcraft.pdf"), &bytes) {
                Ok(()) => {
                    self.status = "Exported fixed-layout PDF to scriptcraft.pdf".into();
                    self.clear_error();
                }
                Err(error) => self.show_error(error),
            },
            Err(error) => self.show_error(error),
        }
    }

    fn show_error<E: std::fmt::Debug>(&mut self, error: E) {
        let detail = format!("{error:?}");
        let health = self.session_health();
        self.error = Some(detail.clone());
        self.status = format!("Operation failed; document unchanged ({detail}); {health}");
    }

    fn show_edit_error(&mut self, error: EditError) {
        let detail = match error.managed_text_mismatch_diagnostic() {
            Some(category) => format!("ManagedTextMismatch ({category})"),
            None => format!("{error:?}"),
        };
        self.show_error(detail);
    }

    fn session_health(&self) -> String {
        let validation = match self.session.validate() {
            Ok(()) => "session valid".to_owned(),
            Err(error) => format!("session invalid: {error:?}"),
        };
        let serialization = if fdx::write_v6(self.session.project()).is_ok() {
            "FDX projection valid"
        } else {
            "no complete FDX serialization available"
        };
        format!("{validation}; {serialization}")
    }

    fn clear_error(&mut self) {
        self.error = None;
    }

    fn record_native_event(&mut self, category: &'static str) {
        self.native_input_diagnostics.received_events = self
            .native_input_diagnostics
            .received_events
            .saturating_add(1);
        self.native_input_diagnostics.last_event_category = category;
    }

    fn mutate(
        &mut self,
        result: Result<(), EditError>,
        success: &str,
        reveal_intent: RevealIntent,
    ) {
        match result {
            Ok(()) => {
                self.preferred_vertical_x = None;
                self.status = success.into();
                self.clear_error();
                self.rendered_revision = None;
                self.reveal_intent = reveal_intent;
            }
            Err(error) => self.show_edit_error(error),
        }
    }

    fn caret_from_pointer(
        &mut self,
        layout: &ScreenplayLayout,
        page: usize,
        page_rect: egui::Rect,
        scale: f32,
        pointer: egui::Pos2,
    ) -> Option<Position> {
        let local_x =
            FixedPoint::from_points_f64(f64::from((pointer.x - page_rect.left()) / scale))?;
        let local_y =
            FixedPoint::from_points_f64(f64::from((pointer.y - page_rect.top()) / scale))?;
        let started = Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let result = layout
            .nearest_line_hit_test_fixed_with_diagnostics(
                page,
                local_x,
                local_y,
                &mut query_diagnostics,
            )
            .map(|caret| Position {
                element: caret.element,
                offset: caret.offset,
            });
        self.performance.indexed_interaction_queries = self
            .performance
            .indexed_interaction_queries
            .saturating_add(1);
        self.performance.interaction_index_entries_examined = self
            .performance
            .interaction_index_entries_examined
            .saturating_add(query_diagnostics.index_entries_examined);
        self.performance.caret_lookup_nanos = self
            .performance
            .caret_lookup_nanos
            .saturating_add(started.elapsed().as_nanos());
        result
    }

    fn pointer_body_page(
        layout: &ScreenplayLayout,
        page_rects: &[egui::Rect],
        pointer: egui::Pos2,
    ) -> Option<usize> {
        if page_rects
            .iter()
            .enumerate()
            .any(|(index, rect)| rect.contains(pointer) && layout.pages[index].title_page)
        {
            // Title-page ownership is explicit: a title click never becomes a
            // body selection, even when a body line is globally nearest.
            return None;
        }
        page_rects
            .iter()
            .enumerate()
            .filter(|(index, _)| !layout.pages[*index].title_page)
            .min_by_key(|(index, rect)| {
                let distance = if pointer.y < rect.top() {
                    rect.top() - pointer.y
                } else if pointer.y > rect.bottom() {
                    pointer.y - rect.bottom()
                } else {
                    0.0
                };
                (distance.to_bits(), *index)
            })
            .map(|(index, _)| index)
    }

    fn apply_pointer_selection(
        &mut self,
        position: Position,
        drag_started: bool,
        dragged: bool,
    ) -> Result<(), EditError> {
        let selection = if drag_started {
            self.drag_anchor = Some(position);
            Selection::caret(position)
        } else if dragged {
            Selection {
                anchor: self.drag_anchor.unwrap_or(self.session.selection().anchor),
                focus: position,
            }
        } else {
            self.drag_anchor = None;
            Selection::caret(position)
        };
        let result = self.session.set_selection(selection);
        if result.is_ok() {
            self.preferred_vertical_x = None;
            self.rendered_revision = None;
            // Pointer selection is authoritative: it must not inherit a reveal
            // request queued by an earlier edit or navigation command.
            self.reveal_intent = RevealIntent::None;
        }
        result
    }

    fn selection_range(
        &self,
        line_start: usize,
        line_end: usize,
        element: usize,
    ) -> Option<(usize, usize)> {
        let selection = self.session.selection();
        let (start, end) = ordered_selection(selection);
        if start.element > element || end.element < element {
            return None;
        }
        let from = if start.element == element {
            start.offset
        } else {
            line_start
        };
        let to = if end.element == element {
            end.offset
        } else {
            line_end
        };
        (from < line_end && to > line_start).then_some((from, to))
    }

    // Rendering needs these geometry and interaction inputs together; wrapping
    // them in a temporary parameter object would add noise to the paint path.
    #[allow(clippy::too_many_arguments)]
    fn paint_page(
        &mut self,
        ui: &mut egui::Ui,
        layout: &ScreenplayLayout,
        page_index: usize,
        page_rect: egui::Rect,
        metrics: PresentationMetrics,
        editor_id: egui::Id,
        caret_page_index: Option<usize>,
        caret_geometry: Option<FixedRect>,
    ) {
        let scale = metrics.scale;
        let paint_started = Instant::now();
        self.performance.painted_pages = self.performance.painted_pages.saturating_add(1);
        add_performance_counter(
            &mut self.performance.lines_submitted,
            layout.pages[page_index].lines.len() as u64,
        );
        let painter = ui.painter_at(page_rect);
        let palette = self.page_palette;
        painter.rect_filled(page_rect, 2.0, palette.paper);
        painter.rect_stroke(
            page_rect,
            2.0,
            Stroke::new(1.0_f32, palette.border),
            egui::StrokeKind::Middle,
        );

        for line in &layout.pages[page_index].lines {
            let x = page_rect.left() + line.geometry.x.to_points_f32() * scale;
            let y = page_rect.top() + line.geometry.y.to_points_f32() * scale;
            let char_width = line.char_width.to_points_f32() * scale;
            let selection = self.selection_range(line.source.start, line.source.end, line.element);
            if let Some((from, to)) = selection {
                add_performance_counter(&mut self.performance.selection_spans_submitted, 1);
                let display_from = line.display_char_for_source(from, false);
                let display_to = line.display_char_for_source(to, true);
                let left = x + display_from as f32 * char_width;
                let width = display_to.saturating_sub(display_from) as f32 * char_width;
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(left, y),
                        Vec2::new(width.max(1.0), line.geometry.height.to_points_f32() * scale),
                    ),
                    0.0,
                    palette.selection,
                );
            }
            paint_fixed_metric_text_with_selection(
                &painter,
                ui.ctx(),
                egui::pos2(x, y),
                &line.display_text,
                renderer_font_for_style(
                    line.text_style.bold,
                    line.text_style.italic,
                    line.text_style.size as f32 * scale,
                ),
                char_width,
                line.geometry.height.to_points_f32() * scale,
                FixedMetricTextStyle {
                    color: if line.text_style.hidden_text {
                        palette.subdued_text
                    } else {
                        palette.text
                    },
                    selected_color: palette.selection_text,
                    selection: selection.map(|(start, end)| {
                        (
                            line.display_char_for_source(start, false),
                            line.display_char_for_source(end, true),
                        )
                    }),
                },
                &mut self.performance,
            );
            if line.text_style.underline {
                let underline_y = y + line.geometry.height.to_points_f32() * scale * 0.88;
                painter.line_segment(
                    [
                        egui::pos2(x, underline_y),
                        egui::pos2(x + line.geometry.width.to_points_f32() * scale, underline_y),
                    ],
                    Stroke::new(
                        1.0_f32,
                        if line.text_style.hidden_text {
                            palette.subdued_text
                        } else {
                            palette.text
                        },
                    ),
                );
            }
        }

        let selection = self.session.selection();
        if caret_page_index == Some(page_index)
            && selection.anchor == selection.focus
            && !layout.pages[page_index].title_page
        {
            if let Some(rect) = caret_geometry {
                let caret = Caret {
                    element: selection.focus.element,
                    offset: selection.focus.offset,
                };
                let blink_id = editor_id.with((
                    "caret-blink",
                    self.session.revision(),
                    caret.element,
                    caret.offset,
                    self.caret_blink_focus_generation,
                ));
                let blink_active = self.surface_focused;
                if caret_blink_visible(
                    ui.ctx(),
                    blink_id,
                    blink_active,
                    true,
                    self.caret_blink_reset_time,
                ) {
                    painter.rect_filled(caret_bar_rect(page_rect, rect, scale), 0.0, palette.caret);
                }
                if self.surface_focused {
                    self.performance.repaint_requests =
                        self.performance.repaint_requests.saturating_add(1);
                }
                if !self.ime_preedit.is_empty() {
                    let caret_line = layout.pages[page_index].lines.iter().find(|line| {
                        line.element == caret.element
                            && caret.offset >= line.source.start
                            && caret.offset <= line.source.end
                    });
                    let line_char_width =
                        caret_line.map_or(7.0, |line| line.char_width.to_points_f32()) * scale;
                    let ime_font = caret_line.map_or_else(
                        || renderer_font(&layout.inputs.font_family, metrics.font_size),
                        |line| {
                            renderer_font_for_style(
                                line.text_style.bold,
                                line.text_style.italic,
                                line.text_style.size as f32 * scale,
                            )
                        },
                    );
                    let x = page_rect.left() + rect.x.to_points_f32() * scale + line_char_width;
                    let y = page_rect.top() + rect.y.to_points_f32() * scale;
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        &self.ime_preedit,
                        ime_font,
                        palette.ime_text,
                    );
                }
            }
        }

        self.performance.paint_nanos = self
            .performance
            .paint_nanos
            .saturating_add(paint_started.elapsed().as_nanos());
    }

    fn live_document_surface(&mut self, ui: &mut egui::Ui) {
        let layout = match self.layout_for_render() {
            Ok(layout) => layout,
            Err(error) => {
                self.page_geometry.invalidate();
                self.page_scale = None;
                self.show_error(error.clone());
                ui.colored_label(Color32::RED, format!("Layout error: {error:?}"));
                return;
            }
        };
        self.rendered_revision = Some(self.session.revision());
        let editor_id = ui.id().with(EDITOR_SURFACE_ID);
        let scroll_outer_rect = ui.available_rect_before_wrap();
        let mut page_rects = Vec::with_capacity(layout.pages.len());
        let mut rendered_scale = 1.0;
        let reveal_intent = self.reveal_intent;
        let pending_zoom_anchor = self.pending_zoom_anchor;
        let zoom_compensation_offset = pending_zoom_anchor.and_then(|anchor| {
            prepaint_zoom_scroll_offset(
                anchor,
                layout.pages.len(),
                layout.inputs.paper_width as f32,
                layout.inputs.paper_height as f32,
                layout.inputs.font_size as f32,
                self.zoom_percent,
                scroll_outer_rect,
                ui.spacing().item_spacing.y,
            )
        });
        let zoom_compensation_queued = zoom_compensation_offset.is_some();
        if zoom_compensation_queued {
            add_performance_counter(&mut self.performance.zoom_compensations_queued, 1);
        }
        let selection = self.session.selection();
        let mut scroll_area = egui::ScrollArea::both().id_salt("page-editor-scroll");
        if let Some(offset) = zoom_compensation_offset {
            scroll_area = scroll_area.scroll_offset(offset);
        }
        let scroll_output = scroll_area.show(ui, |ui| {
            // ScrollArea's child max rect is the actual inner viewport after
            // reserved scrollbar width and parent clipping have been applied.
            let viewport_width = ui.available_width().max(0.0);
            let metrics = presentation_metrics_at_zoom(
                viewport_width,
                layout.inputs.paper_width as f32,
                layout.inputs.font_size as f32,
                self.zoom_percent,
            );
            let geometry = page_scroll_geometry(viewport_width, metrics.page_width);
            debug_assert_eq!(
                geometry.horizontal_overflow,
                geometry.content_width > geometry.viewport_width
            );
            let scale = metrics.scale;
            rendered_scale = scale;
            self.page_scale = Some(scale);
            ui.set_min_width(geometry.content_width);
            // Allocate every page row to preserve full scroll extent and source geometry;
            // page painting is a separate viewport-bounded pass below.
            let editor_clip = ui.clip_rect();
            for _ in 0..layout.pages.len() {
                add_performance_counter(&mut self.performance.pages_considered, 1);
                ui.horizontal(|ui| {
                    // Explicit geometry owns all horizontal spacing; default
                    // row gaps would inflate measured scroll content.
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(geometry.leading_padding);
                    let page_height = layout.inputs.paper_height as f32 * scale;
                    let page_rect = ui
                        .allocate_space(Vec2::new(metrics.page_width, page_height))
                        .1;
                    let trailing_padding =
                        (geometry.content_width - geometry.leading_padding - metrics.page_width)
                            .max(0.0);
                    ui.add_space(trailing_padding);
                    page_rects.push(page_rect);
                });
                ui.add_space(PAGE_VERTICAL_GAP);
            }
            let paint_range = viewport_paint_page_range(&page_rects, editor_clip);
            let painted_in_range = paint_range.map_or(0, |(first, last)| {
                last.saturating_sub(first).saturating_add(1)
            });
            add_performance_counter(
                &mut self.performance.pages_culled,
                page_rects.len().saturating_sub(painted_in_range) as u64,
            );
            let caret_page_index = if reveal_intent == RevealIntent::Conditional {
                let mut query_diagnostics = InteractionQueryDiagnostics::default();
                let page = layout.page_for_position_in_range_with_diagnostics(
                    selection.focus.element,
                    selection.focus.offset,
                    None,
                    &mut query_diagnostics,
                );
                self.performance.indexed_interaction_queries = self
                    .performance
                    .indexed_interaction_queries
                    .saturating_add(1);
                self.performance.interaction_index_entries_examined = self
                    .performance
                    .interaction_index_entries_examined
                    .saturating_add(query_diagnostics.index_entries_examined);
                page
            } else if selection.anchor == selection.focus {
                paint_range.and_then(|(first, last)| {
                    let mut query_diagnostics = InteractionQueryDiagnostics::default();
                    let page = layout.page_for_position_in_range_with_diagnostics(
                        selection.focus.element,
                        selection.focus.offset,
                        Some((first, last)),
                        &mut query_diagnostics,
                    );
                    self.performance.indexed_interaction_queries = self
                        .performance
                        .indexed_interaction_queries
                        .saturating_add(1);
                    self.performance.interaction_index_entries_examined = self
                        .performance
                        .interaction_index_entries_examined
                        .saturating_add(query_diagnostics.index_entries_examined);
                    page
                })
            } else {
                None
            };
            let resolve_caret = caret_page_index.is_some_and(|page_index| {
                reveal_intent == RevealIntent::Conditional
                    || (selection.anchor == selection.focus && !layout.pages[page_index].title_page)
            });
            let caret_geometry = if resolve_caret {
                let caret_started = Instant::now();
                self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
                let mut query_diagnostics = InteractionQueryDiagnostics::default();
                let result = layout
                    .caret_geometry_with_diagnostics(
                        Caret {
                            element: selection.focus.element,
                            offset: selection.focus.offset,
                        },
                        &mut query_diagnostics,
                    )
                    .ok();
                self.performance.indexed_interaction_queries = self
                    .performance
                    .indexed_interaction_queries
                    .saturating_add(1);
                self.performance.interaction_index_entries_examined = self
                    .performance
                    .interaction_index_entries_examined
                    .saturating_add(query_diagnostics.index_entries_examined);
                self.performance.caret_lookup_nanos = self
                    .performance
                    .caret_lookup_nanos
                    .saturating_add(caret_started.elapsed().as_nanos());
                result
            } else {
                None
            };
            let caret_paint_page_index = (selection.anchor == selection.focus)
                .then_some(caret_page_index)
                .flatten()
                .filter(|page_index| !layout.pages[*page_index].title_page);
            if reveal_intent == RevealIntent::Conditional && !zoom_compensation_queued {
                if let (Some(page_index), Some(caret)) = (caret_page_index, caret_geometry) {
                    let page_rect = page_rects[page_index];
                    let caret_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            page_rect.left() + caret.x.to_points_f32() * scale,
                            page_rect.top() + caret.y.to_points_f32() * scale,
                        ),
                        Vec2::new(CARET_WIDTH, caret.height.to_points_f32() * scale),
                    );
                    if caret_needs_reveal(caret_rect, editor_clip) {
                        // `None` asks egui for nearest/minimal scrolling;
                        // centering would override the writer's viewport.
                        ui.scroll_to_rect(caret_rect, None);
                    }
                }
            }
            if let Some((first, last)) = paint_range {
                for (page_index, page_rect) in page_rects
                    .iter()
                    .enumerate()
                    .take(last.saturating_add(1))
                    .skip(first)
                {
                    self.paint_page(
                        ui,
                        &layout,
                        page_index,
                        *page_rect,
                        metrics,
                        editor_id,
                        caret_paint_page_index,
                        caret_geometry,
                    );
                }
            }
        });
        let scale = rendered_scale;
        let published_scroll_offset = scroll_output.state.offset;
        let published_page_rects = page_rects.clone();
        if !zoom_compensation_queued {
            self.reveal_intent = RevealIntent::None;
        }

        let actual_clip = scroll_output.inner_rect.intersect(ui.clip_rect());
        let geometry_count_matches = page_rects.len() == layout.pages.len();
        let zoom_anchor_was_pending = pending_zoom_anchor.is_some();
        let needs_repaint = if geometry_count_matches {
            self.page_geometry.publish(
                self.performance.frames,
                self.session.revision(),
                ui.ctx().content_rect(),
                scale,
                published_page_rects,
                actual_clip,
                published_scroll_offset,
            )
        } else {
            self.page_geometry.invalidate();
            true
        };
        if geometry_count_matches {
            // The request is consumed only after the scaled page geometry has
            // been published. Invalid/missing anchors simply preserve old behavior.
            if zoom_anchor_was_pending {
                add_performance_counter(&mut self.performance.zoom_geometry_published, 1);
                add_performance_counter(&mut self.performance.zoom_anchor_consumed, 1);
            }
            self.pending_zoom_anchor = None;
        }
        if needs_repaint {
            self.request_repaint(ui.ctx());
        }

        // This is the only interactive response in the painted editor. Pages remain
        // source-mapped geometry, not independent focusable widgets.
        self.editor_surface_id = Some(editor_id);
        let response = ui
            .interact(scroll_output.inner_rect, editor_id, Sense::click_and_drag())
            .on_hover_cursor(document_cursor_icon());
        self.editor_surface_rect = Some(response.rect);
        if let Some(pointer) = response.interact_pointer_pos() {
            let pointer_active = response.clicked()
                || response.drag_started()
                || response.dragged()
                || (response.hovered() && ui.input(|input| input.pointer.primary_down()));
            if pointer_active {
                response.request_focus();
                self.native_input_diagnostics.pointer_activations = self
                    .native_input_diagnostics
                    .pointer_activations
                    .saturating_add(1);
                if let Some(page_index) = Self::pointer_body_page(&layout, &page_rects, pointer) {
                    if let Some(position) = self.caret_from_pointer(
                        &layout,
                        page_index,
                        page_rects[page_index],
                        scale,
                        pointer,
                    ) {
                        if let Err(error) = self.apply_pointer_selection(
                            position,
                            response.drag_started(),
                            response.dragged(),
                        ) {
                            self.show_error(error);
                        } else {
                            self.clear_error();
                        }
                    }
                }
            }
        }
        let surface_focused = response.has_focus();
        if surface_focused != self.surface_focused {
            self.caret_blink_focus_generation = self.caret_blink_focus_generation.wrapping_add(1);
            if surface_focused {
                self.caret_blink_reset_time = ui.ctx().input(|input| input.time.max(0.0));
            }
            // The next frame must use the new focus-generation identity. This is
            // a transition-only repaint, not an unfocused blink loop.
            self.request_repaint(ui.ctx());
        }
        self.surface_focused = surface_focused;
        self.native_input_diagnostics.focused = self.surface_focused;
        if self.surface_focused {
            ui.ctx().memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    editor_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
            self.native_input_diagnostics.focus_lock_installed = true;
        } else {
            self.native_input_diagnostics.focus_lock_installed = false;
        }
    }

    fn dispatch_surface_events(&mut self, events: &[egui::Event], modifiers: egui::Modifiers) {
        let started = Instant::now();
        self.performance.events = self.performance.events.saturating_add(events.len() as u64);
        for (index, event) in events.iter().enumerate() {
            self.record_native_event(match event {
                egui::Event::Text(_) => "text",
                egui::Event::Paste(_) => "paste",
                egui::Event::Ime(egui::ImeEvent::Preedit(_)) => "ime-preedit",
                egui::Event::Ime(egui::ImeEvent::Commit(_)) => "ime-commit",
                egui::Event::Ime(egui::ImeEvent::Enabled) => "ime-enabled",
                egui::Event::Ime(egui::ImeEvent::Disabled) => "ime-disabled",
                egui::Event::Zoom(_) => "zoom",
                egui::Event::Key { pressed: true, .. } => "key-pressed",
                egui::Event::Key { pressed: false, .. } => "key-released",
                _ => "other",
            });
            let result = match event {
                egui::Event::Text(text)
                    if !text.is_empty()
                        && !self.ime_active
                        && !events[index + 1..].iter().any(|next| {
                            matches!(next, egui::Event::Ime(egui::ImeEvent::Commit(commit)) if commit == text)
                        })
                        && !modifiers.command
                        && !(modifiers.ctrl && !cfg!(target_os = "macos"))
                        && text.chars().all(|character| !character.is_control()) =>
                {
                    Some(self.session.insert(text))
                }
                egui::Event::Paste(text) if !text.is_empty() => {
                    Some(self.session.insert_paste(text))
                }
                egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                    self.ime_active = true;
                    self.ime_preedit.clone_from(text);
                    self.rendered_revision = None;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    (!text.is_empty()).then(|| self.session.insert(text))
                }
                egui::Event::Ime(egui::ImeEvent::Enabled) => {
                    self.ime_active = true;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Disabled) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    None
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => self.key_event(*key, *modifiers),
                _ => continue,
            };
            if let Some(result) = result {
                self.mutate(
                    result,
                    "Edited through EditingSession",
                    RevealIntent::Conditional,
                );
            }
        }
        self.performance.event_nanos = self
            .performance
            .event_nanos
            .saturating_add(started.elapsed().as_nanos());
    }

    fn key_event(&mut self, key: Key, modifiers: egui::Modifiers) -> Option<Result<(), EditError>> {
        if modifiers.command || (modifiers.ctrl && !cfg!(target_os = "macos")) {
            return match key {
                Key::Z if modifiers.shift => Some(self.session.redo()),
                Key::Z => Some(self.session.undo()),
                Key::Y => Some(self.session.redo()),
                Key::A => Some(self.session.set_selection(Selection {
                    anchor: Position {
                        element: 0,
                        offset: 0,
                    },
                    focus: self.document_end(),
                })),
                _ => None,
            };
        }
        match key {
            Key::Backspace => Some(self.session.backspace()),
            Key::Delete => Some(self.session.delete()),
            Key::Enter => Some(self.session.enter()),
            Key::Tab => Some(self.session.tab()),
            Key::ArrowLeft => Some(self.move_horizontal(false, modifiers.shift)),
            Key::ArrowRight => Some(self.move_horizontal(true, modifiers.shift)),
            Key::ArrowUp => Some(self.move_vertical(false, modifiers.shift)),
            Key::ArrowDown => Some(self.move_vertical(true, modifiers.shift)),
            Key::Home => Some(self.move_to_edge(false, modifiers.shift)),
            Key::End => Some(self.move_to_edge(true, modifiers.shift)),
            _ => None,
        }
    }

    fn handle_surface_input(&mut self, ctx: &egui::Context) {
        self.native_input_diagnostics.dispatch_enabled = self.surface_focused;
        if ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            // Explicit user scrolling wins over a not-yet-applied zoom request.
            // It must not be pulled back to the old viewport center afterward.
            if self.pending_zoom_anchor.is_some() {
                add_performance_counter(&mut self.performance.zoom_manual_scroll_cancellations, 1);
            }
            self.pending_zoom_anchor = None;
            self.reveal_intent = RevealIntent::None;
        }
        if !self.surface_focused {
            return;
        }
        self.native_input_diagnostics.dispatched_frames = self
            .native_input_diagnostics
            .dispatched_frames
            .saturating_add(1);
        let events = ctx.input(|input| input.events.clone());
        let modifiers = ctx.input(|input| input.modifiers);
        self.dispatch_surface_events(&events, modifiers);
        if !events.is_empty() || ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            self.request_repaint(ctx);
        }
    }

    fn document_end(&self) -> Position {
        let element = self.session.project().elements.len().saturating_sub(1);
        Position {
            element,
            offset: self.session.project().elements[element].text.len(),
        }
    }

    fn move_horizontal(&mut self, right: bool, extend: bool) -> Result<(), EditError> {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        if selection.anchor != selection.focus && !extend {
            let position = if right {
                ordered_selection(selection).1
            } else {
                ordered_selection(selection).0
            };
            return self.session.set_selection(Selection::caret(position));
        }
        let position = selection.focus;
        let item = &self.session.project().elements[position.element];
        let next = if right {
            if position.offset < item.text.len() {
                Position {
                    offset: item.text[position.offset..]
                        .chars()
                        .next()
                        .map_or(position.offset, |c| position.offset + c.len_utf8()),
                    ..position
                }
            } else if position.element + 1 < self.session.project().elements.len() {
                Position {
                    element: position.element + 1,
                    offset: 0,
                }
            } else {
                position
            }
        } else if position.offset > 0 {
            Position {
                offset: item.text[..position.offset]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(at, _)| at),
                ..position
            }
        } else if position.element > 0 {
            let previous = &self.session.project().elements[position.element - 1];
            Position {
                element: position.element - 1,
                offset: previous.text.len(),
            }
        } else {
            position
        };
        self.session.set_selection(if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    fn move_vertical(&mut self, down: bool, extend: bool) -> Result<(), EditError> {
        let selection = self.session.selection();
        let current = selection.focus;
        let layout = self
            .layout_for_render()
            .map_err(|_| EditError::InvalidElement)?;
        let interaction_started = Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let current_index = layout.body_line_index_for_position_with_diagnostics(
            current.element,
            current.offset,
            &mut query_diagnostics,
        );
        self.performance.indexed_interaction_queries = self
            .performance
            .indexed_interaction_queries
            .saturating_add(1);
        self.performance.interaction_index_entries_examined = self
            .performance
            .interaction_index_entries_examined
            .saturating_add(query_diagnostics.index_entries_examined);
        add_performance_nanos(
            &mut self.performance.caret_lookup_nanos,
            interaction_started.elapsed().as_nanos(),
        );
        let Some(current_index) = current_index else {
            self.preferred_vertical_x = None;
            return self.session.set_selection(selection);
        };
        let target_index = if down {
            current_index
                .checked_add(1)
                .filter(|index| *index < layout.body_line_count())
        } else {
            current_index.checked_sub(1)
        };
        let Some(current_line) = layout.body_line(current_index) else {
            self.preferred_vertical_x = None;
            return self.session.set_selection(selection);
        };
        let Some(target_index) = target_index else {
            self.preferred_vertical_x = None;
            let next = Position {
                element: current_line.element,
                offset: if down {
                    current_line.source.end
                } else {
                    current_line.source.start
                },
            };
            return self.session.set_selection(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        };
        let goal = self.preferred_vertical_x.unwrap_or_else(|| {
            let columns = current_line.display_char_for_source(current.offset, false) as i64;
            current_line.geometry.x.raw() + columns.saturating_mul(current_line.char_width.raw())
        });
        self.preferred_vertical_x = Some(goal);
        let Some(target_line) = layout.body_line(target_index) else {
            self.preferred_vertical_x = None;
            return self.session.set_selection(selection);
        };
        let cell = target_line.char_width.raw().max(1);
        let columns = (goal
            .saturating_sub(target_line.geometry.x.raw())
            .saturating_add(cell / 2)
            / cell) as u32;
        let next = Position {
            element: target_line.element,
            offset: target_line.source_for_display_char(columns as usize),
        };
        self.session.set_selection(if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    fn move_to_edge(&mut self, end: bool, extend: bool) -> Result<(), EditError> {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        let position = if end {
            Position {
                element: selection.focus.element,
                offset: self.session.project().elements[selection.focus.element]
                    .text
                    .len(),
            }
        } else {
            Position {
                element: selection.focus.element,
                offset: 0,
            }
        };
        self.session.set_selection(if extend {
            Selection {
                anchor: selection.anchor,
                focus: position,
            }
        } else {
            Selection::caret(position)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PresentationMetrics {
    page_width: f32,
    scale: f32,
    font_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresentationPlatform {
    MacOS,
    #[cfg(target_os = "windows")]
    Windows,
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Other,
}

fn logical_points_per_document_point(platform: PresentationPlatform) -> f32 {
    match platform {
        PresentationPlatform::MacOS => 1.0,
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        PresentationPlatform::Other => 1.0,
        #[cfg(target_os = "windows")]
        PresentationPlatform::Windows => 96.0 / 72.0,
    }
}

fn current_presentation_platform() -> PresentationPlatform {
    #[cfg(target_os = "macos")]
    {
        PresentationPlatform::MacOS
    }
    #[cfg(target_os = "windows")]
    {
        PresentationPlatform::Windows
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PresentationPlatform::Other
    }
}

fn presentation_metrics_at_zoom(
    available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
) -> PresentationMetrics {
    presentation_metrics_at_zoom_for_platform(
        available_width,
        paper_width,
        base_font_size,
        zoom_percent,
        current_presentation_platform(),
    )
}

fn presentation_metrics_at_zoom_for_platform(
    _available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
    platform: PresentationPlatform,
) -> PresentationMetrics {
    let normalized = status_bar::normalized_zoom_percent(zoom_percent as f64);
    let scale = logical_points_per_document_point(platform) * normalized as f32 / 100.0;
    let page_width = paper_width * scale;
    PresentationMetrics {
        page_width,
        scale,
        font_size: (base_font_size * scale).max(5.0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PageScrollGeometry {
    viewport_width: f32,
    content_width: f32,
    leading_padding: f32,
    horizontal_overflow: bool,
}

fn page_scroll_geometry(viewport_width: f32, page_width: f32) -> PageScrollGeometry {
    let viewport_width = if viewport_width.is_finite() {
        viewport_width.max(0.0)
    } else {
        0.0
    };
    let page_width = if page_width.is_finite() {
        page_width.max(0.0)
    } else {
        0.0
    };
    let buffered_page_width = page_width + PAGE_HORIZONTAL_PADDING * 2.0;
    let content_width = viewport_width.max(buffered_page_width);
    PageScrollGeometry {
        viewport_width,
        content_width,
        leading_padding: ((content_width - page_width) / 2.0).max(0.0),
        horizontal_overflow: buffered_page_width > viewport_width,
    }
}

// Pure geometry inputs stay explicit so this helper remains directly testable.
#[allow(clippy::too_many_arguments)]
fn prepaint_zoom_scroll_offset(
    anchor: ViewportCenterAnchor,
    page_count: usize,
    paper_width: f32,
    paper_height: f32,
    base_font_size: f32,
    zoom_percent: u16,
    outer_rect: egui::Rect,
    vertical_item_spacing: f32,
) -> Option<Vec2> {
    if anchor.page_index >= page_count || page_count == 0 {
        return None;
    }
    // egui's overlay scrollbars do not consume the ScrollArea inner viewport.
    // Keep planning dimensions identical to the rect that will paint this frame.
    let inner_size = outer_rect.size().max(Vec2::ZERO);
    let metrics =
        presentation_metrics_at_zoom(inner_size.x, paper_width, base_font_size, zoom_percent);
    let geometry = page_scroll_geometry(inner_size.x, metrics.page_width);
    let page_height = paper_height * metrics.scale;
    // `live_document_surface` allocates each page row and then a 12-point
    // spacer in egui's top-down layout. The layout also applies its vertical
    // item spacing between rows, which is part of both the page stride and
    // the committed scroll extent (the first page itself starts at outer.min).
    let page_stride = page_height + PAGE_VERTICAL_GAP + vertical_item_spacing;
    let content_size = Vec2::new(geometry.content_width, page_count as f32 * page_stride);
    let page_top = anchor.page_index as f32 * page_stride;
    let content_target = outer_rect.min
        + Vec2::new(geometry.leading_padding, page_top)
        + anchor.page_local_document_point * metrics.scale
        + anchor.outside_page_offset_points;
    let viewport_center = outer_rect.min + inner_size * 0.5;
    let requested_offset = content_target - viewport_center;
    let max_offset = (content_size - inner_size).max(Vec2::ZERO);
    (requested_offset.x.is_finite() && requested_offset.y.is_finite()).then(|| {
        Vec2::new(
            requested_offset.x.clamp(0.0, max_offset.x),
            requested_offset.y.clamp(0.0, max_offset.y),
        )
    })
}

struct FixedMetricTextStyle {
    color: Color32,
    selected_color: Color32,
    selection: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FixedCellGalleyKey {
    text: String,
    font_family: FontFamily,
    font_size_bits: u32,
    char_width_bits: u32,
    line_height_bits: u32,
    color: Color32,
    pixels_per_point_bits: u32,
}

impl FixedCellGalleyKey {
    fn new(
        text: &str,
        font: &FontId,
        char_width: f32,
        line_height: f32,
        color: Color32,
        pixels_per_point: f32,
    ) -> Self {
        Self {
            text: text.to_owned(),
            font_family: font.family.clone(),
            font_size_bits: font.size.to_bits(),
            char_width_bits: char_width.to_bits(),
            line_height_bits: line_height.to_bits(),
            color,
            pixels_per_point_bits: pixels_per_point.to_bits(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct FixedCellGalleyCache {
    galleys: HashMap<FixedCellGalleyKey, std::sync::Arc<Galley>>,
}

impl FixedCellGalleyCache {
    fn insert(&mut self, key: FixedCellGalleyKey, galley: std::sync::Arc<Galley>) {
        if self.galleys.len() >= FIXED_CELL_GALLEY_CACHE_LIMIT {
            // Keep retention bounded under repeated text edits and document replacement.
            // Clearing is diagnostic-neutral: the next use is simply a fresh build.
            self.galleys.clear();
        }
        self.galleys.insert(key, galley);
    }
}

// Text, selection, and paint instrumentation are independent view inputs.
#[allow(clippy::too_many_arguments)]
fn paint_fixed_metric_text_with_selection(
    painter: &egui::Painter,
    ctx: &egui::Context,
    origin: egui::Pos2,
    text: &str,
    font: FontId,
    char_width: f32,
    line_height: f32,
    style: FixedMetricTextStyle,
    performance: &mut PerformanceDiagnostics,
) {
    if text.is_empty() {
        return;
    }

    let char_count = text.chars().count();
    let selection = style
        .selection
        .map(|(start, end)| (start.min(char_count), end.min(char_count)));
    let mut spans = Vec::with_capacity(3);
    let mut span_start_byte = 0;
    let mut span_start_column = 0;
    let mut span_selected = selection.is_some_and(|(start, end)| start == 0 && end > 0);
    for (column, (byte, _)) in text.char_indices().enumerate().skip(1) {
        let selected = selection.is_some_and(|(start, end)| column >= start && column < end);
        if selected != span_selected {
            spans.push((span_start_byte..byte, span_start_column, span_selected));
            span_start_byte = byte;
            span_start_column = column;
            span_selected = selected;
        }
    }
    spans.push((
        span_start_byte..text.len(),
        span_start_column,
        span_selected,
    ));

    for (byte_range, start_column, selected) in spans {
        let span_text = &text[byte_range];
        if span_text.is_empty() {
            continue;
        }
        let color = if selected {
            style.selected_color
        } else {
            style.color
        };
        let span_origin = egui::pos2(fixed_metric_x(origin.x, start_column, char_width), origin.y);
        let (galley, reused, fixed_geometry) = fixed_cell_span_galley(
            painter,
            ctx,
            span_text,
            &font,
            char_width,
            line_height,
            color,
        );
        if reused {
            performance.galleys_reused = performance.galleys_reused.saturating_add(1);
        } else {
            performance.galleys_built = performance.galleys_built.saturating_add(1);
        }
        if fixed_geometry {
            painter.galley(span_origin, galley, color);
        } else {
            // Keep output geometry exact if a later epaint version changes its
            // single-row glyph/mesh contract; this compatibility path is not used
            // by the pinned 0.29 desktop implementation.
            for (column, character) in span_text.chars().enumerate() {
                painter.text(
                    egui::pos2(
                        fixed_metric_x(span_origin.x, column, char_width),
                        span_origin.y,
                    ),
                    egui::Align2::LEFT_TOP,
                    character.to_string(),
                    font.clone(),
                    color,
                );
            }
        }
    }
}

fn fixed_cell_span_galley(
    painter: &egui::Painter,
    ctx: &egui::Context,
    text: &str,
    font: &FontId,
    char_width: f32,
    line_height: f32,
    color: Color32,
) -> (std::sync::Arc<Galley>, bool, bool) {
    let pixels_per_point = ctx.pixels_per_point();
    let key = FixedCellGalleyKey::new(text, font, char_width, line_height, color, pixels_per_point);
    let cached = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<FixedCellGalleyCache>(egui::Id::new(
            FIXED_CELL_GALLEY_CACHE_ID,
        ))
        .galleys
        .get(&key)
        .cloned()
    });
    if let Some(galley) = cached {
        return (galley, true, true);
    }

    let job = LayoutJob::simple_singleline(key.text.clone(), font.clone(), color);
    let mut galley = painter.layout_job(job).as_ref().clone();
    if !position_galley_in_fixed_cells(&mut galley, char_width) {
        // The epaint 0.29 single-row galley contract is one glyph record per
        // Unicode scalar and four mesh vertices per textured glyph. Preserve
        // correctness if a future backend violates that contract by leaving its
        // host layout untouched; the caller records the requested build.
        return (std::sync::Arc::new(galley), false, false);
    }
    let galley = std::sync::Arc::new(galley);
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<FixedCellGalleyCache>(egui::Id::new(
            FIXED_CELL_GALLEY_CACHE_ID,
        ))
        .insert(key, galley.clone());
    });
    (galley, false, true)
}

fn position_galley_in_fixed_cells(galley: &mut Galley, char_width: f32) -> bool {
    if galley.rows.len() != 1 {
        return false;
    }
    let char_count = galley.job.text.chars().count();
    let placed_row = &mut galley.rows[0];
    let row = std::sync::Arc::make_mut(&mut placed_row.row);
    if row.glyphs.len() != char_count {
        return false;
    }
    let expected_vertex_count = row
        .glyphs
        .iter()
        .filter(|glyph| !glyph.uv_rect.is_nothing())
        .count()
        .saturating_mul(4);
    if row.visuals.glyph_vertex_range.len() != expected_vertex_count {
        return false;
    }

    let pixels_per_point = galley.pixels_per_point;
    let mut vertex_index = row.visuals.glyph_vertex_range.start;
    let vertex_end = row.visuals.glyph_vertex_range.end;
    for (column, glyph) in row.glyphs.iter_mut().enumerate() {
        let fixed_x = column as f32 * char_width;
        if !glyph.uv_rect.is_nothing() {
            let Some(vertices) = row
                .visuals
                .mesh
                .vertices
                .get_mut(vertex_index..vertex_index.saturating_add(4))
            else {
                return false;
            };
            let round_to_pixel = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
            let left = fixed_x + round_to_pixel(glyph.uv_rect.offset.x);
            let right = left + glyph.uv_rect.size.x;
            vertices[0].pos.x = left;
            vertices[1].pos.x = right;
            vertices[2].pos.x = left;
            vertices[3].pos.x = right;
            vertex_index = vertex_index.saturating_add(4);
        }
        glyph.pos.x = fixed_x;
        glyph.advance_width = char_width;
    }
    if vertex_index != vertex_end {
        return false;
    }

    let fixed_width = char_count as f32 * char_width;
    placed_row.pos.x = 0.0;
    row.size.x = fixed_width;
    row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    galley.rect.min.x = 0.0;
    galley.rect.max.x = fixed_width;
    galley.mesh_bounds = galley.rows.iter().fold(egui::Rect::NOTHING, |bounds, row| {
        bounds.union(row.visuals.mesh_bounds)
    });
    true
}

fn renderer_font(font_family: &str, font_size: f32) -> FontId {
    FontId::new(
        font_size,
        if font_family == document_font::COURIER_PRIME_FAMILY {
            FontFamily::Name(document_font::COURIER_PRIME_FAMILY.into())
        } else {
            // Preserve the renderer's established appearance for explicit legacy
            // Courier/Courier New profiles; only the new Prime default selects
            // the embedded face. Viewport scale remains presentation-only.
            FontFamily::Monospace
        },
    )
}

fn renderer_font_for_style(bold: bool, italic: bool, font_size: f32) -> FontId {
    let family = match (bold, italic) {
        (true, true) => "Courier Prime Bold Italic",
        (true, false) => "Courier Prime Bold",
        (false, true) => "Courier Prime Italic",
        (false, false) => document_font::COURIER_PRIME_FAMILY,
    };
    FontId::new(font_size.max(1.0), FontFamily::Name(family.into()))
}

fn fixed_metric_x(origin: f32, column: usize, char_width: f32) -> f32 {
    origin + column as f32 * char_width
}

fn caret_blink_visible(
    ctx: &egui::Context,
    blink_id: egui::Id,
    focused: bool,
    collapsed: bool,
    reset_time: f64,
) -> bool {
    if !focused || !collapsed {
        return false;
    }

    // egui owns both the clock and animation entry. A zero-duration animation
    // makes each 500 ms phase switch discrete rather than introducing a fade.
    // The caller's identity includes revision and caret position, so changes
    // begin a distinct transient entry without model or thread state.
    let time = ctx.input(|input| input.time.max(0.0));
    let elapsed = (time - reset_time).max(0.0);
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (elapsed / interval).floor() as u64;
    let target_visible = phase.is_multiple_of(2);
    let phase_value = ctx.animate_bool_with_time(blink_id, target_visible, 0.0);
    ctx.request_repaint_after(caret_blink_repaint_delay(elapsed));
    phase_value > 0.5
}

fn caret_blink_repaint_delay(time: f64) -> Duration {
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (time.max(0.0) / interval).floor() as u64;
    Duration::from_secs_f64(((phase + 1) as f64 * interval - time.max(0.0)).max(0.001))
}

fn caret_bar_rect(
    page_rect: egui::Rect,
    caret: scriptcraft_core::layout::FixedRect,
    scale: f32,
) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            page_rect.left() + caret.x.to_points_f32() * scale,
            page_rect.top() + caret.y.to_points_f32() * scale,
        ),
        Vec2::new(CARET_WIDTH, caret.height.to_points_f32() * scale),
    )
}

fn caret_needs_reveal(caret_rect: egui::Rect, visible_clip: egui::Rect) -> bool {
    !visible_clip.contains_rect(caret_rect)
}

fn ordered_selection(selection: Selection) -> (Position, Position) {
    if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}

fn chrome_clear_color(visuals: &egui::Visuals) -> [f32; 4] {
    visuals.widgets.open.weak_bg_fill.to_normalized_gamma_f32()
}

fn document_cursor_icon() -> egui::CursorIcon {
    egui::CursorIcon::Text
}

fn chrome_frame(ctx: &egui::Context) -> egui::Frame {
    // The top toolbar panel uses the shared app fill and a zero-inset frame;
    // row padding remains explicit and its separator stays disabled.
    egui::Frame::NONE
        .fill(toolbar_chrome_color(ctx))
        .stroke(egui::Stroke::NONE)
}

fn central_panel_frame(ctx: &egui::Context) -> egui::Frame {
    // The central panel is the immediate owner below the toolbar. Keep its
    // normal 8-point document inset, but use the same explicit surface fill
    // as the top chrome so the boundary cannot expose panel_fill as a dark
    // (black-looking) strip between the two app-owned surfaces.
    egui::Frame::central_panel(ctx.global_style().as_ref()).fill(toolbar_chrome_color(ctx))
}

fn show_application_panels(app: &mut ScriptCraftApp, ui: &mut egui::Ui) -> egui::Rect {
    let ctx = ui.ctx().clone();
    let now = app.timer_now();
    app.reconcile_sprint_timer(now);
    let fullscreen = ctx.input(|input| input.viewport().fullscreen);
    app.focus_mode.observe(fullscreen, now);
    let (resolved_theme, applied_appearance) = status_bar::configure_appearance(
        &ctx,
        app.appearance_mode,
        app.applied_appearance,
    );
    app.applied_appearance = applied_appearance;
    app.page_palette = status_bar::PagePalette::for_theme(app.appearance_mode, resolved_theme);

    egui::Panel::top("titlebar-toolbar")
        .frame(chrome_frame(&ctx))
        .show_separator_line(false)
        .exact_size(toolbar_content_height(&ctx.global_style()))
        .show_inside(ui, |ui| app.toolbar(ui));
    let viewport_rect = ctx.content_rect();
    let page_number = app.page_geometry.visible_page(
        app.performance.frames,
        app.session.revision(),
        viewport_rect,
        app.page_scale.unwrap_or(f32::NAN),
        app.reveal_intent == RevealIntent::Conditional
            || page_geometry_input_invalidated(&ctx, app.page_geometry.clip()),
    );
    let center = status_bar::project_center(&app.status, app.error.as_deref());
    let performance_summary = if status_bar::diagnostics_enabled() {
        let derived_work = app.session.pending_derived_work_summary();
        Some(format!(
            "{} derived-pending={} derived-pending-complete={} derived-pending-scenes={} derived-retryable={} derived-pending-deferred-scenes={} derived-failed-complete={} derived-failed-scenes={}",
            app.performance.safe_summary(),
            derived_work.pending,
            derived_work.complete,
            derived_work.affected_scene_count,
            derived_work.retryable,
            derived_work.deferred_scene_count,
            derived_work.failed_complete,
            derived_work.failed_scene_count,
        ))
    } else {
        None
    };
    let diagnostics = performance_summary.as_deref().map(|performance_summary| {
        let performance = app.performance;
        let input_diagnostics = &app.native_input_diagnostics;
        DiagnosticProjection {
            focused: input_diagnostics.focused,
            focus_lock: input_diagnostics.focus_lock_installed,
            dispatch_enabled: input_diagnostics.dispatch_enabled,
            received_events: input_diagnostics.received_events,
            dispatched_frames: input_diagnostics.dispatched_frames,
            pointer_activations: input_diagnostics.pointer_activations,
            last_event_category: input_diagnostics.last_event_category,
            performance_events: performance.events,
            layout_builds: performance.layout_builds,
            layout_cache_hits: performance.layout_cache_hits,
            painted_pages: performance.painted_pages,
            repaint_requests: performance.repaint_requests,
            performance_summary,
        }
    });
    egui::Panel::bottom("status")
        .frame(status_bar::status_panel_frame(&ctx))
        .show_inside(ui, |ui| {
        let was_timer_open = app.sprint_timer.popup_open;
        let focus_view = app.focus_mode.view();
        let mut focus_activated = false;
        let (zoom_changed, appearance_changed, timer_button_rect, _focus_button_rect) =
            status_bar::show_with_focus_mode(
                ui,
                page_number,
                &center,
                diagnostics,
                &mut app.zoom_percent,
                &mut app.appearance_mode,
                &mut app.sprint_timer.popup_open,
                &mut app.sprint_timer.just_opened,
                &focus_view,
                &mut focus_activated,
            );
        if !was_timer_open && app.sprint_timer.popup_open {
            app.sprint_timer.popup_anchor = timer_button_rect.unwrap_or(egui::Rect::NOTHING);
        }
        if was_timer_open && !app.sprint_timer.popup_open {
            app.sprint_timer.close_popup();
        }
        if appearance_changed {
            ctx.request_repaint();
        }
        if zoom_changed {
            // Pointer/keyboard slider and exact preset edits become the new
            // continuous pinch baseline, keeping both control paths in sync
            // without changing document/session state.
            app.capture_pending_zoom_anchor(&ctx);
            app.sync_pinch_zoom_from_controls();
            app.page_geometry.invalidate();
            app.page_scale = None;
        }
        if focus_activated {
            if let Some(request) = app.focus_mode.request(now) {
                // The request/pending generation is committed before the
                // viewport effect so same-frame or reentrant activation cannot
                // issue an implicit inverse.
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(request.target));
            }
        }
        });
    let central = egui::CentralPanel::default()
        .frame(central_panel_frame(&ctx))
        .show_inside(ui, |ui| app.live_document_surface(ui));
    sprint_timer::show_popup(&ctx, &mut app.sprint_timer, now, &mut app.notification_sink);
    central.response.rect
}

fn show_fdx_migration_dialog(app: &mut ScriptCraftApp, ctx: &egui::Context) {
    let Some(declared_version) = app
        .pending_fdx_open
        .as_ref()
        .map(|pending| pending.declared_version.clone())
    else {
        return;
    };
    let mut choice = None;
    egui::Window::new("Update FDX document")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(format!(
                "This document declares FDX version {declared_version}. It will be opened with the current Version 6 importer. Updating may contain errors, some data may be lost, and compatibility is not guaranteed."
            ));
            ui.horizontal(|ui| {
                if ui.button("Continue").clicked() {
                    choice = Some(FdxMigrationConsent::Continue);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(FdxMigrationConsent::Cancel);
                }
            });
        });
    if let Some(consent) = choice {
        app.resolve_pending_fdx_import(consent);
    }
}

impl eframe::App for ScriptCraftApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        chrome_clear_color(visuals)
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // egui computes total pixels-per-point as zoom_factor × native scale.
        // Resolve the factor against this frame's native scale before begin_pass
        // so moving between monitors never briefly multiplies a stale UI factor
        // by the newly reported OS scale.
        let viewport = raw_input.viewports.get_mut(&raw_input.viewport_id);
        let native_scale = viewport.and_then(|viewport| {
            if viewport
                .native_pixels_per_point
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                viewport.native_pixels_per_point = None;
            }
            viewport.native_pixels_per_point
        });
        let effective_pixels_per_point = status_bar::resolved_ui_pixels_per_point(native_scale);
        ctx.set_zoom_factor(status_bar::ui_zoom_factor(native_scale));
        let scale_changed = self.effective_pixels_per_point != Some(effective_pixels_per_point);
        self.effective_pixels_per_point = Some(effective_pixels_per_point);

        let original_zoom = self.zoom_percent;
        for event in &raw_input.events {
            if let egui::Event::Zoom(factor) = event {
                self.record_native_event("zoom");
                if let Some(next) =
                    status_bar::zoom_after_native_factor(self.pinch_zoom_percent, *factor)
                {
                    let next_zoom = status_bar::normalized_zoom_percent(next);
                    if next_zoom != self.zoom_percent {
                        self.capture_pending_zoom_anchor(ctx);
                    }
                    self.pinch_zoom_percent = next;
                    self.zoom_percent = next_zoom;
                }
            }
        }
        // The app has consumed the event for page zoom. Keep it out of egui's
        // later input-state zoom delta so no other surface can apply it again.
        raw_input
            .events
            .retain(|event| !matches!(event, egui::Event::Zoom(_)));
        if self.zoom_percent != original_zoom || scale_changed {
            self.page_geometry.invalidate();
            self.page_scale = None;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ScriptCraftApp::update(self, ui);
    }
}

impl ScriptCraftApp {
    fn update(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let frame_started = Instant::now();
        self.performance.frames = self.performance.frames.saturating_add(1);
        self.update_native_window_state(&ctx);
        self.notification_sink.set_repaint_context(&ctx);
        let migration_pending = self.pending_fdx_open.is_some();
        if !migration_pending {
            // Route events using the editor-focus state established by the prior
            // surface frame, then let this frame's layout and paint observe edits.
            self.handle_surface_input(&ctx);
            // Scheduled derived work sees every accepted same-frame edit and
            // never delays already-queued editor input on its deadline frame.
            self.refresh_cast_lists_on_schedule(&ctx, Instant::now());
        }
        let timer_now = self.timer_now();
        self.reconcile_sprint_timer(timer_now);
        if migration_pending {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let _ = ui.allocate_space(ui.available_size());
            });
            show_fdx_migration_dialog(self, &ctx);
        } else {
            show_application_panels(self, ui);
        }
        self.schedule_sprint_timer_repaint(&ctx, self.timer_now());
        self.schedule_focus_mode_repaint(&ctx, self.timer_now());
        self.performance.frame_nanos = self
            .performance
            .frame_nanos
            .saturating_add(frame_started.elapsed().as_nanos());
    }
}

impl ScriptCraftApp {
    fn update_native_window_state(&mut self, ctx: &egui::Context) {
        let reported = ctx.input(|input| {
            let viewport = input.viewport();
            window_state_from_rects(viewport.inner_rect, viewport.outer_rect)
        });
        if let Some(reported) = reported {
            self.latest_window_state = Some(reported);
        }
        if let Some(commands) = take_window_restore_commands(&mut self.pending_window_restore) {
            for command in commands {
                ctx.send_viewport_cmd(command);
            }
        }
    }
}

fn window_state_from_rects(
    inner: Option<egui::Rect>,
    outer: Option<egui::Rect>,
) -> Option<fdx::WindowState> {
    fn bounded_integer(value: f32) -> Option<i32> {
        let value = f64::from(value);
        (value.is_finite() && value >= f64::from(i32::MIN) && value <= f64::from(i32::MAX))
            .then(|| value.round() as i32)
    }

    let inner = inner?;
    let outer = outer?;
    fdx::WindowState::new(
        bounded_integer(outer.min.x)?,
        bounded_integer(outer.min.y)?,
        bounded_integer(inner.width())?,
        bounded_integer(inner.height())?,
    )
}

fn window_restore_commands(state: fdx::WindowState) -> [egui::ViewportCommand; 2] {
    [
        egui::ViewportCommand::OuterPosition(egui::pos2(state.left as f32, state.top as f32)),
        egui::ViewportCommand::InnerSize(egui::vec2(state.width as f32, state.height as f32)),
    ]
}

fn take_window_restore_commands(
    pending: &mut Option<fdx::WindowState>,
) -> Option<[egui::ViewportCommand; 2]> {
    pending.take().map(window_restore_commands)
}

fn page_geometry_input_invalidated(ctx: &egui::Context, previous_clip: Option<egui::Rect>) -> bool {
    ctx.input(|input| {
        let scroll_changed = has_nonzero_mouse_wheel(&input.raw.events)
            || input.smooth_scroll_delta != Vec2::ZERO;
        let pointer_scrolling = input.pointer.any_down()
            && input
                .pointer
                .interact_pos()
                .zip(previous_clip)
                .is_some_and(|(position, clip)| clip.contains(position));
        scroll_changed || pointer_scrolling
    })
}

fn has_nonzero_mouse_wheel(events: &[egui::Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            egui::Event::MouseWheel { delta, .. } if *delta != Vec2::ZERO
        )
    })
}

fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(true)
            .with_transparent(false)
            .with_title_shown(true)
            .with_titlebar_shown(true)
            .with_fullsize_content_view(false)
            .with_titlebar_buttons_shown(true),
        ..Default::default()
    }
}

struct StartupPresentation {
    options: eframe::NativeOptions,
}

fn startup_presentation() -> StartupPresentation {
    StartupPresentation {
        options: native_options(),
    }
}

fn main() -> eframe::Result {
    let StartupPresentation { options } = startup_presentation();
    eframe::run_native(
        "ScriptCraft",
        options,
        Box::new(|cc| {
            document_font::install(&cc.egui_ctx);
            egui_material_icons::initialize(&cc.egui_ctx);
            ScriptCraftApp::new()
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|error| error.into())
        }),
    )
}
