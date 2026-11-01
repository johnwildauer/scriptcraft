#![deny(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use eframe::egui::{
    self, Color32, FontFamily, FontId, Key, RichText, Sense, Stroke, TextStyle, TextWrapMode, Vec2,
    WidgetText,
};
use scriptcraft_core::cast_list::CAST_LIST_REFRESH_INTERVAL;
use scriptcraft_core::editing::{EditError, EditPolicy, EditingSession, Position, Selection};
use scriptcraft_core::exports;
use scriptcraft_core::fdx::{self, FdxImportOutcome, FdxMigrationConsent, FdxVersionStatus};
use scriptcraft_core::fdx_storage::{
    self, FdxBaseline, FdxSaveMode, FdxSaveTarget, OpenedFdxSource, StdFdxStorageFileSystem,
};
use scriptcraft_core::layout::{Caret, LayoutCache, LayoutPurpose, ScreenplayLayout};
use scriptcraft_core::persistence::{FileSystem, StdFileSystem};
use scriptcraft_core::{FixedPoint, Project, Role};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

mod document_font;
mod focus_mode;
mod sprint_timer;
mod status_bar;
mod toolbar;
use sprint_timer::{NotificationResult, NotificationSink};
use status_bar::{DiagnosticProjection, ViewportPageTracker};
use toolbar::{toolbar_chrome_color, toolbar_content_height};

const EDITOR_SURFACE_ID: &str = "page-editor-surface";
const CARET_WIDTH: f32 = 1.5;
const CURRENT_DEFAULT_PAGE_WIDTH: f32 = 360.0;
const PAGE_HORIZONTAL_PADDING: f32 = 16.0;
#[cfg(test)]
const PRESENTATION_METRIC_TOLERANCE: f32 = 0.01;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

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
struct PerformanceDiagnostics {
    frames: u64,
    events: u64,
    layout_builds: u64,
    layout_cache_hits: u64,
    painted_pages: u64,
    repaint_requests: u64,
    frame_nanos: u128,
    event_nanos: u128,
    layout_nanos: u128,
    paint_nanos: u128,
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
    pending_fdx_open: Option<PendingFdxOpen>,
    native_input_diagnostics: NativeInputDiagnostics,
    editor_surface_id: Option<egui::Id>,
    editor_surface_rect: Option<egui::Rect>,
    reveal_intent: RevealIntent,
    preferred_vertical_x: Option<i64>,
    page_geometry: ViewportPageTracker,
    page_scale: Option<f32>,
    effective_pixels_per_point: Option<f32>,
    zoom_percent: u16,
    pinch_zoom_percent: f64,
    appearance_mode: status_bar::AppearanceMode,
    applied_appearance: Option<(status_bar::AppearanceMode, egui::Theme)>,
    page_palette: status_bar::PagePalette,
    #[cfg(test)]
    last_scroll_offset: Vec2,
    #[cfg(test)]
    last_scroll_outer_rect: egui::Rect,
    #[cfg(test)]
    last_scroll_inner_rect: egui::Rect,
    #[cfg(test)]
    last_scroll_content_size: Vec2,
    #[cfg(test)]
    last_scroll_horizontal_item_spacing: f32,
    #[cfg(test)]
    last_scroll_page_rects: Vec<egui::Rect>,
    #[cfg(test)]
    focus_mode_status_item_rect: Option<egui::Rect>,
    #[cfg(test)]
    status_panel_rect: Option<egui::Rect>,
    caret_blink_focus_generation: u64,
    caret_blink_reset_time: f64,
    layout_cache: LayoutCache,
    performance: PerformanceDiagnostics,
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
            pending_fdx_open: None,
            native_input_diagnostics: NativeInputDiagnostics::default(),
            editor_surface_id: None,
            editor_surface_rect: None,
            reveal_intent: RevealIntent::None,
            preferred_vertical_x: None,
            page_geometry: ViewportPageTracker::default(),
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
            #[cfg(test)]
            last_scroll_offset: Vec2::ZERO,
            #[cfg(test)]
            last_scroll_outer_rect: egui::Rect::NOTHING,
            #[cfg(test)]
            last_scroll_inner_rect: egui::Rect::NOTHING,
            #[cfg(test)]
            last_scroll_content_size: Vec2::ZERO,
            #[cfg(test)]
            last_scroll_horizontal_item_spacing: 0.0,
            #[cfg(test)]
            last_scroll_page_rects: Vec::new(),
            #[cfg(test)]
            focus_mode_status_item_rect: None,
            #[cfg(test)]
            status_panel_rect: None,
            caret_blink_focus_generation: 0,
            caret_blink_reset_time: 0.0,
            layout_cache: LayoutCache::default(),
            performance: PerformanceDiagnostics::default(),
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
        if now < self.next_cast_list_refresh {
            ctx.request_repaint_after(self.next_cast_list_refresh.saturating_duration_since(now));
            return;
        }

        self.next_cast_list_refresh = now + CAST_LIST_REFRESH_INTERVAL;
        match self.session.refresh_cast_lists() {
            Ok(true) => self.request_repaint(ctx),
            Ok(false) => {}
            Err(_) => {
                self.status =
                    "Cast-list refresh deferred because source data could not be updated safely."
                        .into();
            }
        }
        self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
        ctx.request_repaint_after(CAST_LIST_REFRESH_INTERVAL);
    }

    fn sync_pinch_zoom_from_controls(&mut self) {
        self.pinch_zoom_percent = f64::from(self.zoom_percent);
    }

    fn layout_for_render(
        &mut self,
    ) -> Result<std::sync::Arc<ScreenplayLayout>, scriptcraft_core::layout::LayoutError> {
        let revision = self.session.revision();
        let started = Instant::now();
        let (layout, hit) = self.layout_cache.get_or_build_with_inputs_for_purpose(
            revision,
            self.session.project(),
            self.session.layout_inputs(),
            LayoutPurpose::Editor,
        )?;
        if hit {
            self.performance.layout_cache_hits =
                self.performance.layout_cache_hits.saturating_add(1);
        } else {
            self.performance.layout_builds = self.performance.layout_builds.saturating_add(1);
            self.performance.layout_nanos = self
                .performance
                .layout_nanos
                .saturating_add(started.elapsed().as_nanos());
        }
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
        match fdx_storage::save_session(
            &StdFdxStorageFileSystem,
            &mut self.session,
            &mut target,
            mode,
        ) {
            Ok(outcome) => {
                self.native_path = target.path().map(PathBuf::from);
                self.fdx_baseline = target.baseline().cloned();
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
        self.status = format!("FDX save failed; document unchanged ({detail}); {health}");
    }

    fn open_native(&mut self) {
        let dialog = rfd::FileDialog::new()
            .add_filter("Final Draft document", &["fdx"]);
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
    ) {
        let replacement = match EditingSession::with_policy(project, self.session.policy()) {
            Ok(replacement) => replacement,
            Err(error) => {
                self.show_error(format!("incompatible document: {error:?}"));
                return;
            }
        };
        self.session = replacement;
        self.native_path = Some(path.to_owned());
        self.fdx_baseline = baseline;
        self.pending_fdx_open = None;
        self.next_cast_list_refresh = Instant::now() + CAST_LIST_REFRESH_INTERVAL;
        self.page_geometry.invalidate();
        self.layout_cache = LayoutCache::default();
        self.rendered_revision = None;
        self.preferred_vertical_x = None;
        self.drag_anchor = None;
        self.ime_preedit.clear();
        self.ime_active = false;
        self.status = migration_status
            .map(str::to_owned)
            .unwrap_or_else(|| {
                "Opened FDX document".into()
            });
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
            Err(error) => self.show_error(error),
        }
    }

    fn caret_from_pointer(
        &self,
        layout: &ScreenplayLayout,
        page: usize,
        page_rect: egui::Rect,
        scale: f32,
        pointer: egui::Pos2,
    ) -> Option<Position> {
        let local_x = FixedPoint::from_points_f64(
            f64::from((pointer.x - page_rect.left()) / scale),
        )?;
        let local_y = FixedPoint::from_points_f64(
            f64::from((pointer.y - page_rect.top()) / scale),
        )?;
        layout
            .nearest_line_hit_test_fixed(page, local_x, local_y)
            .map(|caret| Position {
                element: caret.element,
                offset: caret.offset,
            })
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

    fn paint_page(
        &mut self,
        ui: &mut egui::Ui,
        layout: &ScreenplayLayout,
        page_index: usize,
        metrics: PresentationMetrics,
        editor_id: egui::Id,
    ) -> egui::Rect {
        let page_width = metrics.page_width;
        let scale = metrics.scale;
        let paint_started = Instant::now();
        self.performance.painted_pages = self.performance.painted_pages.saturating_add(1);
        let page_height = layout.inputs.paper_height as f32 * scale;
        let page_rect = ui.allocate_space(Vec2::new(page_width, page_height)).1;
        let painter = ui.painter_at(page_rect);
        let palette = self.page_palette;
        painter.rect_filled(page_rect, 2.0, palette.paper);
        painter.rect_stroke(page_rect, 2.0, Stroke::new(1.0_f32, palette.border));

        for line in &layout.pages[page_index].lines {
            let x = page_rect.left() + line.geometry.x.to_points_f32() * scale;
            let y = page_rect.top() + line.geometry.y.to_points_f32() * scale;
            let char_width = line.char_width.to_points_f32() * scale;
            let selection = self.selection_range(line.source.start, line.source.end, line.element);
            if let Some((from, to)) = selection {
                let display_from = line.display_char_for_source(from, false);
                let display_to = line.display_char_for_source(to, true);
                let left =
                    x + display_from as f32 * char_width;
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
                egui::pos2(x, y),
                &line.display_text,
                renderer_font_for_style(
                    line.text_style.bold,
                    line.text_style.italic,
                    line.text_style.size as f32 * scale,
                ),
                char_width,
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
            );
            if line.text_style.underline {
                let underline_y = y + line.geometry.height.to_points_f32() * scale * 0.88;
                painter.line_segment(
                    [
                        egui::pos2(x, underline_y),
                        egui::pos2(x + line.geometry.width.to_points_f32() * scale, underline_y),
                    ],
                    Stroke::new(1.0, if line.text_style.hidden_text {
                        palette.subdued_text
                    } else {
                        palette.text
                    }),
                );
            }
        }

        let selection = self.session.selection();
        if selection.anchor == selection.focus && !layout.pages[page_index].title_page {
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
            if let Ok(rect) = layout.caret_geometry(caret.clone()) {
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
                    let caret_line = layout.pages[page_index]
                        .lines
                        .iter()
                        .find(|line| {
                            line.element == caret.element
                                && caret.offset >= line.source.start
                                && caret.offset <= line.source.end
                        });
                    let line_char_width = caret_line
                        .map_or(7.0, |line| line.char_width.to_points_f32())
                        * scale;
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
        page_rect
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
        let mut page_rects = Vec::with_capacity(layout.pages.len());
        #[cfg(test)]
        let mut horizontal_item_spacing = 0.0;
        #[cfg(test)]
        let scroll_outer_rect = ui.available_rect_before_wrap();
        let mut rendered_scale = 1.0;
        let reveal_intent = self.reveal_intent;
        let scroll_output = egui::ScrollArea::both()
            .id_salt("page-editor-scroll")
            .show(ui, |ui| {
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
                for page_index in 0..layout.pages.len() {
                    let mut page_rect = None;
                    ui.horizontal(|ui| {
                        // Explicit geometry owns all horizontal spacing; default
                        // row gaps would inflate measured scroll content.
                        ui.spacing_mut().item_spacing.x = 0.0;
                        #[cfg(test)]
                        {
                            horizontal_item_spacing = ui.spacing().item_spacing.x;
                        }
                        ui.add_space(geometry.leading_padding);
                        page_rect =
                            Some(self.paint_page(ui, &layout, page_index, metrics, editor_id));
                        let trailing_padding = (geometry.content_width
                            - geometry.leading_padding
                            - metrics.page_width)
                            .max(0.0);
                        ui.add_space(trailing_padding);
                    });
                    page_rects.push(page_rect.unwrap_or_else(|| {
                        egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::ZERO)
                    }));
                    ui.add_space(12.0);
                }
                if reveal_intent == RevealIntent::Conditional {
                    let focus = self.session.selection().focus;
                    if let Some(page_index) = layout.pages.iter().position(|page| {
                        page.lines.iter().any(|line| {
                            line.element == focus.element
                                && focus.offset >= line.source.start
                                && focus.offset <= line.source.end
                        })
                    }) {
                        if let Ok(caret) = layout.caret_geometry(Caret {
                            element: focus.element,
                            offset: focus.offset,
                        }) {
                            let page_rect = page_rects[page_index];
                            let caret_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    page_rect.left() + caret.x.to_points_f32() * scale,
                                    page_rect.top() + caret.y.to_points_f32() * scale,
                                ),
                                Vec2::new(CARET_WIDTH, caret.height.to_points_f32() * scale),
                            );
                            let visible_clip = ui.clip_rect();
                            if caret_needs_reveal(caret_rect, visible_clip) {
                                // `None` asks egui for nearest/minimal scrolling;
                                // centering would override the writer's viewport.
                                ui.scroll_to_rect(caret_rect, None);
                            }
                        }
                    }
                }
            });
        let scale = rendered_scale;
        #[cfg(test)]
        {
            self.last_scroll_offset = scroll_output.state.offset;
            self.last_scroll_outer_rect = scroll_outer_rect;
            self.last_scroll_inner_rect = scroll_output.inner_rect;
            self.last_scroll_content_size = scroll_output.content_size;
            self.last_scroll_horizontal_item_spacing = horizontal_item_spacing;
            self.last_scroll_page_rects.clone_from(&page_rects);
        }
        self.reveal_intent = RevealIntent::None;

        let actual_clip = scroll_output.inner_rect.intersect(ui.clip_rect());
        let geometry_count_matches = page_rects.len() == layout.pages.len();
        let needs_repaint = if geometry_count_matches {
            self.page_geometry.publish(
                self.performance.frames,
                self.session.revision(),
                ui.ctx().screen_rect(),
                scale,
                page_rects.clone(),
                actual_clip,
            )
        } else {
            self.page_geometry.invalidate();
            true
        };
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
        if !self.surface_focused {
            return;
        }
        if ctx.input(|input| input.raw_scroll_delta != Vec2::ZERO) {
            self.reveal_intent = RevealIntent::None;
        }
        self.native_input_diagnostics.dispatched_frames = self
            .native_input_diagnostics
            .dispatched_frames
            .saturating_add(1);
        let events = ctx.input(|input| input.events.clone());
        let modifiers = ctx.input(|input| input.modifiers);
        self.dispatch_surface_events(&events, modifiers);
        if !events.is_empty() || ctx.input(|input| input.raw_scroll_delta != Vec2::ZERO) {
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
        let lines: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .collect();
        let current_index = lines.iter().position(|line| {
            line.element == current.element
                && current.offset >= line.source.start
                && current.offset <= line.source.end
        });
        let Some(current_index) = current_index else {
            self.preferred_vertical_x = None;
            return self.session.set_selection(selection);
        };
        let target_index = if down {
            current_index
                .checked_add(1)
                .filter(|index| *index < lines.len())
        } else {
            current_index.checked_sub(1)
        };
        let current_line = lines[current_index];
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
            current_line.geometry.x.raw()
                + columns.saturating_mul(current_line.char_width.raw())
        });
        self.preferred_vertical_x = Some(goal);
        let target_line = lines[target_index];
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

fn presentation_metrics_at_zoom(
    _available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
) -> PresentationMetrics {
    let normalized = status_bar::normalized_zoom_percent(zoom_percent as f64);
    let zoom_factor = normalized as f32 / 100.0;
    let page_width = CURRENT_DEFAULT_PAGE_WIDTH * zoom_factor;
    let scale = page_width / paper_width;
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

#[cfg(test)]
fn presentation_geometry(available_width: f32, paper_width: f32) -> (f32, f32, f32) {
    let metrics = presentation_metrics_at_zoom(available_width, paper_width, 12.0, 100);
    let content_width = available_width.max(metrics.page_width + PAGE_HORIZONTAL_PADDING * 2.0);
    let leading_padding = ((content_width - metrics.page_width) / 2.0).max(0.0);
    (metrics.page_width, leading_padding, metrics.scale)
}

#[cfg(test)]
fn presentation_metrics(
    available_width: f32,
    paper_width: f32,
    base_font_size: f32,
) -> PresentationMetrics {
    presentation_metrics_at_zoom(available_width, paper_width, base_font_size, 100)
}

struct FixedMetricTextStyle {
    color: Color32,
    selected_color: Color32,
    selection: Option<(usize, usize)>,
}

fn paint_fixed_metric_text_with_selection(
    painter: &egui::Painter,
    origin: egui::Pos2,
    text: &str,
    font: FontId,
    char_width: f32,
    style: FixedMetricTextStyle,
) {
    for (column, character) in text.chars().enumerate() {
        let glyph_color = if style
            .selection
            .is_some_and(|(start, end)| column >= start && column < end)
        {
            style.selected_color
        } else {
            style.color
        };
        painter.text(
            egui::pos2(fixed_metric_x(origin.x, column, char_width), origin.y),
            egui::Align2::LEFT_TOP,
            character.to_string(),
            font.clone(),
            glyph_color,
        );
    }
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

#[cfg(test)]
fn toolbar_icon_hover_owner(icon_hover_rect: egui::Rect, pointer: egui::Pos2) -> bool {
    icon_hover_rect.contains(pointer)
}

#[cfg(test)]
fn surface_cursor_owner(surface_rect: egui::Rect, pointer: egui::Pos2) -> bool {
    surface_rect.contains(pointer)
}

fn chrome_frame(ctx: &egui::Context) -> egui::Frame {
    // The top toolbar panel uses the shared app fill and a zero-inset frame;
    // row padding remains explicit and its separator stays disabled.
    egui::Frame::none()
        .fill(toolbar_chrome_color(ctx))
        .stroke(egui::Stroke::NONE)
}

fn central_panel_frame(ctx: &egui::Context) -> egui::Frame {
    // The central panel is the immediate owner below the toolbar. Keep its
    // normal 8-point document inset, but use the same explicit surface fill
    // as the top chrome so the boundary cannot expose panel_fill as a dark
    // (black-looking) strip between the two app-owned surfaces.
    egui::Frame::central_panel(ctx.style().as_ref()).fill(toolbar_chrome_color(ctx))
}

fn show_application_panels(app: &mut ScriptCraftApp, ctx: &egui::Context) -> egui::Rect {
    let now = app.timer_now();
    app.reconcile_sprint_timer(now);
    let fullscreen = ctx.input(|input| input.viewport().fullscreen);
    app.focus_mode.observe(fullscreen, now);
    let (resolved_theme, applied_appearance) =
        status_bar::configure_appearance(ctx, app.appearance_mode, app.applied_appearance);
    app.applied_appearance = applied_appearance;
    app.page_palette = status_bar::PagePalette::for_theme(app.appearance_mode, resolved_theme);

    egui::TopBottomPanel::top("titlebar-toolbar")
        .frame(chrome_frame(ctx))
        .show_separator_line(false)
        .exact_height(toolbar_content_height(&ctx.style()))
        .show(ctx, |ui| app.toolbar(ui));
    let viewport_rect = ctx.screen_rect();
    let page_number = app.page_geometry.visible_page(
        app.performance.frames,
        app.session.revision(),
        viewport_rect,
        app.page_scale.unwrap_or(f32::NAN),
        app.reveal_intent == RevealIntent::Conditional
            || page_geometry_input_invalidated(ctx, app.page_geometry.clip()),
    );
    let center = status_bar::project_center(app.session.dirty(), &app.status, app.error.as_deref());
    let performance = app.performance;
    let input_diagnostics = &app.native_input_diagnostics;
    let diagnostics = DiagnosticProjection {
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
    };
    let status_panel = egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
        let was_timer_open = app.sprint_timer.popup_open;
        let focus_view = app.focus_mode.view();
        let mut focus_activated = false;
        let (zoom_changed, appearance_changed, timer_button_rect, focus_button_rect) =
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
        #[cfg(test)]
        {
            app.sprint_timer.status_item_rect = timer_button_rect;
            app.focus_mode_status_item_rect = focus_button_rect;
        }
        #[cfg(not(test))]
        let _ = focus_button_rect;
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
    #[cfg(test)]
    {
        app.status_panel_rect = Some(status_panel.response.rect);
    }
    #[cfg(not(test))]
    let _ = status_panel;
    let central = egui::CentralPanel::default()
        .frame(central_panel_frame(ctx))
        .show(ctx, |ui| app.live_document_surface(ui));
    sprint_timer::show_popup(ctx, &mut app.sprint_timer, now, &mut app.notification_sink);
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
        // so moving between monitors never briefly multiplies fallback 2.3 by
        // the newly reported OS scale.
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
                    self.pinch_zoom_percent = next;
                    self.zoom_percent = status_bar::normalized_zoom_percent(next);
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

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_started = Instant::now();
        self.performance.frames = self.performance.frames.saturating_add(1);
        self.notification_sink.set_repaint_context(ctx);
        let migration_pending = self.pending_fdx_open.is_some();
        if !migration_pending {
            self.refresh_cast_lists_on_schedule(ctx, Instant::now());
        }
        let timer_now = self.timer_now();
        self.reconcile_sprint_timer(timer_now);
        if migration_pending {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.allocate_space(ui.available_size());
            });
            show_fdx_migration_dialog(self, ctx);
        } else {
            show_application_panels(self, ctx);
        }
        self.schedule_sprint_timer_repaint(ctx, self.timer_now());
        self.schedule_focus_mode_repaint(ctx, self.timer_now());
        if !migration_pending {
            self.handle_surface_input(ctx);
        }
        self.performance.frame_nanos = self
            .performance
            .frame_nanos
            .saturating_add(frame_started.elapsed().as_nanos());
    }
}

fn page_geometry_input_invalidated(ctx: &egui::Context, previous_clip: Option<egui::Rect>) -> bool {
    ctx.input(|input| {
        let scroll_changed =
            input.raw_scroll_delta != Vec2::ZERO || input.smooth_scroll_delta != Vec2::ZERO;
        let pointer_scrolling = input.pointer.any_down()
            && input
                .pointer
                .interact_pos()
                .zip(previous_clip)
                .is_some_and(|(position, clip)| clip.contains(position));
        scroll_changed || pointer_scrolling
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
            ScriptCraftApp::new()
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|error| error.into())
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolbar::*;

    fn test_context() -> egui::Context {
        let context = egui::Context::default();
        document_font::install(&context);
        context
    }

    fn legacy_json_project() -> Project {
        Project::from_json(include_str!(
            "../../../crates/scriptcraft-core/tests/fixtures/legacy_project.json"
        ))
        .unwrap_or_else(|_| unreachable!())
    }

    fn activate_legacy_json_project(app: &mut ScriptCraftApp) {
        app.session
            .replace_with_saved(legacy_json_project())
            .unwrap_or_else(|_| unreachable!());
    }

    fn render_app_frame(
        context: &egui::Context,
        app: &mut ScriptCraftApp,
        screen: egui::Rect,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
    ) -> egui::FullOutput {
        render_app_frame_with_fullscreen(context, app, screen, events, modifiers, None)
    }

    #[test]
    fn scheduled_cast_list_refresh_runs_after_one_second_without_undo_noise() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = vec![
            scriptcraft_core::ScreenplayElement::new("heading", Role::SceneHeading, "INT. ROOM")
                .unwrap_or_else(|_| unreachable!()),
            scriptcraft_core::ScreenplayElement::new("cue", Role::Character, "ALEX")
                .unwrap_or_else(|_| unreachable!()),
            scriptcraft_core::ScreenplayElement::new("cast", Role::CastList, "(stale)")
                .unwrap_or_else(|_| unreachable!()),
        ];
        app.session
            .replace_with_saved(project)
            .unwrap_or_else(|_| unreachable!());
        let context = test_context();
        let scheduled = app.next_cast_list_refresh;

        app.refresh_cast_lists_on_schedule(&context, scheduled - Duration::from_millis(1));
        assert_eq!(app.session.project().elements[2].text, "(stale)");
        app.refresh_cast_lists_on_schedule(&context, scheduled);
        assert_eq!(app.session.project().elements[2].text, "(ALEX)");
        assert!(!app.session.can_undo());
        assert!(app.session.dirty());
    }

    fn render_app_frame_with_fullscreen(
        context: &egui::Context,
        app: &mut ScriptCraftApp,
        screen: egui::Rect,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
        fullscreen: Option<bool>,
    ) -> egui::FullOutput {
        app.performance.frames = app.performance.frames.saturating_add(1);
        let mut input = egui::RawInput {
            screen_rect: Some(screen),
            focused: true,
            events,
            modifiers,
            ..Default::default()
        };
        if let Some(viewport) = input.viewports.get_mut(&input.viewport_id) {
            viewport.fullscreen = fullscreen;
        }
        context.run(input, |ctx| {
            show_application_panels(app, ctx);
            app.handle_surface_input(ctx);
        })
    }

    fn click_events(rect: egui::Rect) -> [Vec<egui::Event>; 2] {
        let point = rect.center();
        let pointer = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        [
            vec![egui::Event::PointerMoved(point), pointer(true)],
            vec![egui::Event::PointerMoved(point), pointer(false)],
        ]
    }

    fn rendered_text(output: &egui::FullOutput) -> String {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn rendered_text_center(output: &egui::FullOutput, needle: &str) -> Option<egui::Pos2> {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text) if text.galley.text().contains(needle) => {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
    }

    fn fullscreen_commands(output: &egui::FullOutput) -> Vec<bool> {
        output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .into_iter()
            .flat_map(|viewport| viewport.commands.iter())
            .filter_map(|command| match command {
                egui::ViewportCommand::Fullscreen(fullscreen) => Some(*fullscreen),
                _ => None,
            })
            .collect()
    }

    fn contains_only_fullscreen_commands(output: &egui::FullOutput) -> bool {
        output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .is_none_or(|viewport| {
                viewport
                    .commands
                    .iter()
                    .all(|command| matches!(command, egui::ViewportCommand::Fullscreen(_)))
            })
    }

    #[test]
    fn wo004_rendered_focus_action_uses_reported_state_and_preserves_window_content_state() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_200.0, 820.0));
        app.zoom_percent = 150;
        app.sprint_timer
            .select_mode(sprint_timer::TimerMode::Stopwatch);
        app.sprint_timer.start_stopwatch(Duration::ZERO);
        app.appearance_mode = status_bar::AppearanceMode::Night;
        let revision = app.session.revision();

        let first = render_app_frame_with_fullscreen(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
            Some(false),
        );
        assert!(rendered_text(&first).contains("⛶ Focus Mode"));
        let focus_rect = app
            .focus_mode_status_item_rect
            .unwrap_or_else(|| unreachable!("Focus Mode should be inline at wide width"));
        assert!(focus_rect.width() >= 112.0, "focus target={focus_rect:?}");
        let timer_rect = app
            .sprint_timer
            .status_item_rect
            .unwrap_or_else(|| unreachable!("Sprint Timer should remain inline at wide width"));
        let appearance_center = rendered_text_center(&first, "Appearance")
            .unwrap_or_else(|| unreachable!("Appearance should remain visible at wide width"));
        assert!(timer_rect.right() < focus_rect.left());
        assert!(focus_rect.right() < appearance_center.x);
        assert_eq!(
            app.focus_mode.view().accessible_name(),
            "Enter Focus Mode; the window reports windowed"
        );

        let mut command_output = egui::FullOutput::default();
        for events in click_events(focus_rect) {
            command_output = render_app_frame_with_fullscreen(
                &context,
                &mut app,
                screen,
                events,
                egui::Modifiers::default(),
                Some(false),
            );
        }
        assert_eq!(fullscreen_commands(&command_output), [true]);
        assert!(contains_only_fullscreen_commands(&command_output));
        assert!(app
            .focus_mode
            .view()
            .accessible_name()
            .contains("transition pending: entering fullscreen"));
        assert!(!app.focus_mode.view().enabled());
        assert_eq!(app.zoom_percent, 150);
        assert_eq!(app.appearance_mode, status_bar::AppearanceMode::Night);
        assert!(app.sprint_timer.is_running_or_paused());
        assert_eq!(app.session.revision(), revision);

        let unknown = render_app_frame_with_fullscreen(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
            None,
        );
        assert!(fullscreen_commands(&unknown).is_empty());
        assert!(app
            .focus_mode
            .view()
            .accessible_name()
            .contains("fullscreen state is not reported"));

        let exit_context = test_context();
        let mut exit_app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let exit_first = render_app_frame_with_fullscreen(
            &exit_context,
            &mut exit_app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
            Some(true),
        );
        assert!(rendered_text(&exit_first).contains("⛶ Focus Mode"));
        let exit_rect = exit_app
            .focus_mode_status_item_rect
            .unwrap_or_else(|| unreachable!("fullscreen action should be inline at wide width"));
        let mut exit_output = egui::FullOutput::default();
        for events in click_events(exit_rect) {
            exit_output = render_app_frame_with_fullscreen(
                &exit_context,
                &mut exit_app,
                screen,
                events,
                egui::Modifiers::default(),
                Some(true),
            );
        }
        assert_eq!(fullscreen_commands(&exit_output), [false]);
        assert!(contains_only_fullscreen_commands(&exit_output));
    }

    #[test]
    fn wo004_rendered_overflow_focus_and_external_os_changes_reconcile_without_commands() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let narrow = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(480.0, 820.0));
        let first = render_app_frame_with_fullscreen(
            &context,
            &mut app,
            narrow,
            Vec::new(),
            egui::Modifiers::default(),
            Some(false),
        );
        assert!(app.focus_mode_status_item_rect.is_none());
        let more_center = rendered_text_center(&first, "More").unwrap_or_else(|| {
            unreachable!("narrow layout must expose the status overflow trigger")
        });
        let more_rect = egui::Rect::from_center_size(more_center, Vec2::splat(2.0));
        for events in click_events(more_rect) {
            render_app_frame_with_fullscreen(
                &context,
                &mut app,
                narrow,
                events,
                egui::Modifiers::default(),
                Some(false),
            );
        }
        let menu = render_app_frame_with_fullscreen(
            &context,
            &mut app,
            narrow,
            Vec::new(),
            egui::Modifiers::default(),
            Some(false),
        );
        assert!(rendered_text(&menu).contains("⛶ Focus Mode"));
        let focus_rect = app
            .focus_mode_status_item_rect
            .unwrap_or_else(|| unreachable!("Focus Mode should be available in overflow"));
        let mut command_output = egui::FullOutput::default();
        for events in click_events(focus_rect) {
            command_output = render_app_frame_with_fullscreen(
                &context,
                &mut app,
                narrow,
                events,
                egui::Modifiers::default(),
                Some(false),
            );
        }
        assert_eq!(fullscreen_commands(&command_output), [true]);

        // OS-originated fullscreen changes are observed; they do not cause an
        // application command or replay the preceding request.
        let entered = render_app_frame_with_fullscreen(
            &context,
            &mut app,
            narrow,
            Vec::new(),
            egui::Modifiers::default(),
            Some(true),
        );
        assert!(fullscreen_commands(&entered).is_empty());
        assert!(app
            .focus_mode
            .view()
            .accessible_name()
            .contains("transition pending: entering fullscreen"));

        let mut external_app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let external_initial = render_app_frame_with_fullscreen(
            &context,
            &mut external_app,
            narrow,
            Vec::new(),
            egui::Modifiers::default(),
            Some(false),
        );
        assert!(fullscreen_commands(&external_initial).is_empty());
        let entered = render_app_frame_with_fullscreen(
            &context,
            &mut external_app,
            narrow,
            Vec::new(),
            egui::Modifiers::default(),
            Some(true),
        );
        assert!(fullscreen_commands(&entered).is_empty());
        assert!(external_app
            .focus_mode
            .view()
            .accessible_name()
            .contains("window reports fullscreen"));
        let exited = render_app_frame_with_fullscreen(
            &context,
            &mut external_app,
            narrow,
            Vec::new(),
            egui::Modifiers::default(),
            Some(false),
        );
        assert!(fullscreen_commands(&exited).is_empty());
        assert!(external_app
            .focus_mode
            .view()
            .accessible_name()
            .contains("window reports windowed"));
    }

    #[test]
    fn wo004_pending_and_unavailable_focus_states_keep_the_footer_height_bounded() {
        for width in [1_200.0, 480.0] {
            for pending in [true, false] {
                let context = test_context();
                let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(width, 820.0));
                if pending {
                    render_app_frame_with_fullscreen(
                        &context,
                        &mut app,
                        screen,
                        Vec::new(),
                        egui::Modifiers::default(),
                        Some(false),
                    );
                    assert!(app.focus_mode.request(app.timer_now()).is_some());
                }

                let mut heights = Vec::with_capacity(60);
                let fullscreen = if pending { Some(false) } else { None };
                for _ in 0..60 {
                    render_app_frame_with_fullscreen(
                        &context,
                        &mut app,
                        screen,
                        Vec::new(),
                        egui::Modifiers::default(),
                        fullscreen,
                    );
                    let rect = app
                        .status_panel_rect
                        .unwrap_or_else(|| unreachable!("status panel should be measured"));
                    heights.push(rect.height());
                }
                assert!(
                    heights.iter().all(|height| *height < 120.0),
                    "width={width}, pending={pending}: status panel consumed the viewport: {heights:?}"
                );
                let stable = &heights[1..];
                let minimum = stable.iter().copied().fold(f32::INFINITY, f32::min);
                let maximum = stable.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                assert!(
                    maximum - minimum <= 1.0,
                    "width={width}, pending={pending}: status height range={minimum}..{maximum}"
                );
                assert!(
                    !stable.windows(2).all(|pair| pair[1] > pair[0]),
                    "width={width}, pending={pending}: status panel grew monotonically: {stable:?}"
                );
                let accessible = app.focus_mode.view().accessible_name();
                if pending {
                    assert!(accessible.contains("transition pending"));
                } else {
                    assert!(accessible.contains("state is not reported"));
                }
            }
        }
    }

    #[test]
    fn wo003_rendered_status_icon_opens_timer_and_enter_uses_filtered_minutes() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_200.0, 820.0));
        let first = render_app_frame(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
        );
        assert_eq!(app.sprint_timer.minutes_input, "25");
        assert!(rendered_text(&first).contains('⏱'));
        let status_rect = app
            .sprint_timer
            .status_item_rect
            .unwrap_or_else(|| unreachable!("the timer action should be inline at 1200 points"));
        assert!(
            status_rect.width() >= 36.0,
            "timer target is too small: {status_rect:?}"
        );
        for events in click_events(status_rect) {
            render_app_frame(
                &context,
                &mut app,
                screen,
                events,
                egui::Modifiers::default(),
            );
        }
        assert!(app.sprint_timer.popup_open);
        let popup_output = render_app_frame(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
        );
        let popup_text = rendered_text(&popup_output);
        assert!(
            popup_text.contains("Countdown"),
            "popup text={popup_text:?}; window={:?}; status={status_rect:?}",
            app.sprint_timer.popup_rect
        );
        assert!(popup_text.contains("Stopwatch"));
        assert!(popup_text.contains("25:00"));
        assert!(popup_text.contains("Start Sprint"));
        assert!(popup_text.contains("End Sprint"));
        assert!(!app.sprint_timer.end_enabled());
        let minutes_rect = app
            .sprint_timer
            .minutes_input_rect
            .unwrap_or_else(|| unreachable!("the popup should render its minutes editor"));
        for events in click_events(minutes_rect) {
            render_app_frame(
                &context,
                &mut app,
                screen,
                events,
                egui::Modifiers::default(),
            );
        }
        let select_all = egui::Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                command: true,
                ..Default::default()
            },
        };
        render_app_frame(
            &context,
            &mut app,
            screen,
            vec![select_all, egui::Event::Paste("1.5".to_owned())],
            egui::Modifiers {
                command: true,
                ..Default::default()
            },
        );
        assert_eq!(app.sprint_timer.minutes_input, "15");
        render_app_frame(
            &context,
            &mut app,
            screen,
            vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            egui::Modifiers::default(),
        );
        assert_eq!(
            app.sprint_timer.primary_label(),
            "Pause",
            "popup={:?}; primary={:?}; status={status_rect:?}",
            app.sprint_timer.popup_rect,
            app.sprint_timer.primary_button_rect
        );
        assert_eq!(app.sprint_timer.minutes_input, "15");
        assert_eq!(
            app.session.revision(),
            0,
            "timer Enter must not edit screenplay content"
        );
    }

    #[test]
    fn wo003_rendered_start_and_outside_close_preserve_running_timer() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.sprint_timer.minutes_input = "1".to_owned();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_200.0, 820.0));
        render_app_frame(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
        );
        let status_rect = app
            .sprint_timer
            .status_item_rect
            .unwrap_or_else(|| unreachable!("timer status icon"));
        for events in click_events(status_rect) {
            render_app_frame(
                &context,
                &mut app,
                screen,
                events,
                egui::Modifiers::default(),
            );
        }
        render_app_frame(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
        );
        let start_rect = app
            .sprint_timer
            .primary_button_rect
            .unwrap_or_else(|| unreachable!("Start Sprint action"));
        for events in click_events(start_rect) {
            render_app_frame(
                &context,
                &mut app,
                screen,
                events,
                egui::Modifiers::default(),
            );
        }
        assert_eq!(app.sprint_timer.primary_label(), "Pause");
        let outside = egui::pos2(16.0, 800.0);
        render_app_frame(
            &context,
            &mut app,
            screen,
            vec![
                egui::Event::PointerMoved(outside),
                egui::Event::PointerButton {
                    pos: outside,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            egui::Modifiers::default(),
        );
        render_app_frame(
            &context,
            &mut app,
            screen,
            vec![egui::Event::PointerButton {
                pos: outside,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            }],
            egui::Modifiers::default(),
        );
        assert!(!app.sprint_timer.popup_open);
        assert_eq!(app.sprint_timer.primary_label(), "Pause");
        assert_eq!(app.session.revision(), 0);

        for events in click_events(status_rect) {
            render_app_frame(
                &context,
                &mut app,
                screen,
                events,
                egui::Modifiers::default(),
            );
        }
        render_app_frame(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
        );
        render_app_frame(
            &context,
            &mut app,
            screen,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            egui::Modifiers::default(),
        );
        assert!(!app.sprint_timer.popup_open);
        assert_eq!(app.sprint_timer.primary_label(), "Pause");
        assert_eq!(app.session.revision(), 0);
    }

    #[test]
    fn wo003_closed_popup_schedules_bounded_running_timer_repaints_only() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.sprint_timer
            .select_mode(sprint_timer::TimerMode::Stopwatch);
        app.sprint_timer.start_stopwatch(Duration::ZERO);
        assert!(!app.sprint_timer.popup_open);
        let before = app.performance.repaint_requests;
        let _ = context.run(egui::RawInput::default(), |ctx| {
            app.schedule_sprint_timer_repaint(ctx, Duration::from_millis(250));
        });
        assert_eq!(app.performance.repaint_requests, before + 1);
        app.sprint_timer.pause(Duration::from_secs(1));
        let paused_requests = app.performance.repaint_requests;
        let _ = context.run(egui::RawInput::default(), |ctx| {
            app.schedule_sprint_timer_repaint(ctx, Duration::from_secs(2));
        });
        assert_eq!(app.performance.repaint_requests, paused_requests);
        assert!(!app.sprint_timer.popup_open);
    }

    #[test]
    fn wo003_rendered_completion_keeps_reset_enabled_and_end_disabled() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.sprint_timer.set_minutes_input("1".to_owned());
        app.sprint_timer.start_countdown(Duration::ZERO);
        app.sprint_timer.reconcile(Duration::from_secs(60));
        app.sprint_timer.popup_open = true;
        app.sprint_timer.just_opened = true;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_200.0, 820.0));
        render_app_frame(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
        );
        let output = render_app_frame(
            &context,
            &mut app,
            screen,
            Vec::new(),
            egui::Modifiers::default(),
        );
        let text = rendered_text(&output);
        assert!(text.contains("Great Work! You completed a 1 Minute Sprint"));
        assert!(text.contains("Reset"));
        assert_eq!(app.sprint_timer.primary_label(), "Reset");
        assert!(!app.sprint_timer.end_enabled());
        let reset_rect = app
            .sprint_timer
            .primary_button_rect
            .unwrap_or_else(|| unreachable!("completed timer Reset button"));
        for events in click_events(reset_rect) {
            render_app_frame(
                &context,
                &mut app,
                screen,
                events,
                egui::Modifiers::default(),
            );
        }
        assert!(app.sprint_timer.is_idle());
        assert_eq!(app.sprint_timer.minutes_input, "25");
    }

    fn test_toolbar_label_width(label: &str) -> f32 {
        let ctx = test_context();
        let mut width = 0.0;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                width = toolbar_label_width(ui, label);
            });
        });
        width
    }

    fn render_editor_frame(
        context: &egui::Context,
        app: &mut ScriptCraftApp,
        input: egui::RawInput,
    ) -> egui::FullOutput {
        context.run(input, |context| {
            egui::CentralPanel::default().show(context, |ui| app.live_document_surface(ui));
        })
    }

    #[test]
    fn new_document_has_no_legacy_json_metadata_and_diagnostics_exclude_script_text() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        assert!(app.session.project().to_json().is_err());
        app.session
            .insert("SENSITIVE-DOCUMENT-CONTENT")
            .unwrap_or_else(|_| unreachable!());

        let diagnostics = app.session_health();
        assert!(diagnostics.contains("FDX projection valid"));
        assert!(!diagnostics.contains("SENSITIVE-DOCUMENT-CONTENT"));
        assert!(!diagnostics.contains("SENSITIVE DOCUMENT CONTENT"));
    }

    #[test]
    fn rendered_application_omits_project_title_strip_and_keeps_title_page() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let mut project = legacy_json_project();
        project.title_page.push(
            scriptcraft_core::ScreenplayElement::new(
                "wo008-title-page-content",
                Role::Action,
                "VISIBLE TITLE PAGE CONTENT",
            )
            .unwrap_or_else(|_| unreachable!()),
        );
        app.session
            .replace_with_saved(project)
            .unwrap_or_else(|_| unreachable!());
        app.page_geometry.invalidate();

        assert_eq!(
            app.session.project().title_page[0].text,
            "VISIBLE TITLE PAGE CONTENT"
        );
        app.performance.frames = 1;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_024.0, 768.0));
        let mut central_rect = egui::Rect::NOTHING;
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| central_rect = show_application_panels(&mut app, ctx),
        );

        let rendered_text = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let rendered_document = rendered_text.concat();
        assert!(rendered_text.iter().any(|text| text == "New"));
        assert!(
            rendered_text.iter().any(|text| text == "100%")
                || rendered_text
                    .iter()
                    .any(|text| text == "More" || text == "! More"),
            "zoom remains inline or reachable from status overflow"
        );
        assert!(!rendered_text
            .iter()
            .any(|text| text == "Untitled Screenplay"));
        assert!(rendered_document.contains("VISIBLE TITLE PAGE CONTENT"));
        let editor_rect = app
            .editor_surface_rect
            .unwrap_or_else(|| unreachable!("the central panel must render the editor"));
        assert!(editor_rect.height() > 0.0);
        assert!(
            editor_rect.top() - central_rect.top() < 20.0,
            "editor should start at the central panel's normal inset without a reserved title row: editor={editor_rect:?}, central={central_rect:?}"
        );
        assert_eq!(
            app.session.project().title_page[0].text,
            "VISIBLE TITLE PAGE CONTENT"
        );
    }

    #[test]
    fn wo002_rendered_pages_follow_theme_palette_without_mutating_document_state() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.title_page.push(
            scriptcraft_core::ScreenplayElement::new(
                "wo002-theme-title",
                Role::Action,
                "TITLE PAGE THEME",
            )
            .unwrap_or_else(|_| unreachable!()),
        );
        app.session
            .replace_with_saved(project)
            .unwrap_or_else(|_| unreachable!());
        app.session
            .insert("BODY THEME")
            .unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection {
                anchor: Position {
                    element: 0,
                    offset: 0,
                },
                focus: Position {
                    element: 0,
                    offset: 4,
                },
            })
            .unwrap_or_else(|_| unreachable!());
        app.surface_focused = false;
        let project_before = app.session.project().clone();
        let revision_before = app.session.revision();
        let selection_before = app.session.selection();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_200.0, 900.0));

        for (mode, system_theme, expected_theme) in [
            (
                status_bar::AppearanceMode::System,
                Some(egui::Theme::Light),
                egui::Theme::Light,
            ),
            (
                status_bar::AppearanceMode::System,
                Some(egui::Theme::Dark),
                egui::Theme::Dark,
            ),
            (
                status_bar::AppearanceMode::Day,
                Some(egui::Theme::Dark),
                egui::Theme::Light,
            ),
            (
                status_bar::AppearanceMode::Night,
                Some(egui::Theme::Light),
                egui::Theme::Dark,
            ),
            (
                status_bar::AppearanceMode::Midnight,
                Some(egui::Theme::Light),
                egui::Theme::Dark,
            ),
        ] {
            app.appearance_mode = mode;
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    system_theme,
                    ..Default::default()
                },
                |ctx| {
                    show_application_panels(&mut app, ctx);
                },
            );
            let palette = status_bar::PagePalette::for_theme(mode, expected_theme);
            assert_eq!(app.page_palette, palette);
            let paper_pages = output
                .shapes
                .iter()
                .filter(|clipped| {
                    matches!(
                        &clipped.shape,
                        egui::epaint::Shape::Rect(rect) if rect.fill == palette.paper
                    )
                })
                .count();
            assert!(
                paper_pages >= 2,
                "title and body pages use {mode:?} palette"
            );
            assert!(output.shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::epaint::Shape::Rect(rect) if rect.fill == palette.selection
            )));
            assert!(output.shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::epaint::Shape::Text(text)
                    if text.fallback_color == palette.selection_text
            )));
            assert!(output.shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::epaint::Shape::Text(text)
                    if text.fallback_color == palette.text
            )));
            let rendered_text = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect::<String>();
            assert!(rendered_text.contains("TITLE PAGE THEME"));
            assert_eq!(app.session.revision(), revision_before);
            assert_eq!(app.session.selection(), selection_before);
            assert_eq!(app.session.project(), &project_before);
        }

        app.ime_preedit = "IME PREEDIT".to_owned();
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 0,
            }))
            .unwrap_or_else(|_| unreachable!());
        let surface = app.editor_surface_rect.unwrap_or_else(|| unreachable!());
        let focus_point = surface.center();
        let pointer_button = |pressed| egui::Event::PointerButton {
            pos: focus_point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        for pressed in [true, false] {
            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    focused: true,
                    events: vec![
                        egui::Event::PointerMoved(focus_point),
                        pointer_button(pressed),
                    ],
                    ..Default::default()
                },
                |ctx| {
                    show_application_panels(&mut app, ctx);
                },
            );
        }
        assert!(app.surface_focused);
        let editor_id = app.editor_surface_id.unwrap_or_else(|| unreachable!());
        assert!(context.memory(|memory| memory.has_focus(editor_id)));
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 0,
            }))
            .unwrap_or_else(|_| unreachable!());
        let mut render_time = context.input(|input| input.time) + 0.01;
        for mode in [
            status_bar::AppearanceMode::Day,
            status_bar::AppearanceMode::Night,
            status_bar::AppearanceMode::Midnight,
        ] {
            app.appearance_mode = mode;
            app.caret_blink_reset_time = render_time;
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    system_theme: Some(egui::Theme::Light),
                    time: Some(render_time),
                    ..Default::default()
                },
                |ctx| {
                    show_application_panels(&mut app, ctx);
                },
            );
            render_time += 0.01;
            let expected_theme = if mode == status_bar::AppearanceMode::Day {
                egui::Theme::Light
            } else {
                egui::Theme::Dark
            };
            let palette = status_bar::PagePalette::for_theme(mode, expected_theme);
            let caret_rendered = output.shapes.iter().any(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Rect(rect) if rect.fill == palette.caret
                )
            });
            assert!(
                caret_rendered,
                "mode={mode:?}; focused={}; selection={:?}; blink_reset={}; input_time={}; rectangle_fills={:?}",
                app.surface_focused,
                app.session.selection(),
                app.caret_blink_reset_time,
                context.input(|input| input.time),
                output.shapes.iter().filter_map(|clipped| match &clipped.shape {
                    egui::epaint::Shape::Rect(rect) => Some(rect.fill),
                    _ => None,
                }).collect::<Vec<_>>(),
            );
            assert!(output.shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::epaint::Shape::Text(text)
                    if text.fallback_color == palette.ime_text
                        && text.galley.text().contains("IME PREEDIT")
            )));
        }
    }

    fn seed_page_snapshot(app: &mut ScriptCraftApp, pages: Vec<egui::Rect>) -> egui::Rect {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(800.0, 580.0));
        app.performance.frames = 1;
        app.page_scale = Some(1.0);
        assert!(app
            .page_geometry
            .publish(1, app.session.revision(), viewport, 1.0, pages, clip,));
        viewport
    }

    fn project_page_status(
        app: &mut ScriptCraftApp,
        frame: u64,
        viewport: egui::Rect,
    ) -> status_bar::VisiblePage {
        app.page_geometry.visible_page(
            frame,
            app.session.revision(),
            viewport,
            app.page_scale.unwrap_or(f32::NAN),
            false,
        )
    }

    fn render_fresh_editor_geometry(app: &mut ScriptCraftApp) -> status_bar::VisiblePage {
        let context = test_context();
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        app.performance.frames = 2;
        let input = egui::RawInput {
            screen_rect: Some(viewport),
            ..Default::default()
        };
        let _ = render_editor_frame(&context, app, input);
        project_page_status(app, 3, context.screen_rect())
    }

    #[test]
    fn visible_page_tracks_actual_scrolled_geometry_without_following_offscreen_caret() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert(&"line\n".repeat(180))
            .unwrap_or_else(|_| unreachable!());
        let layout = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        assert!(layout.pages.len() >= 2);

        let context = test_context();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        app.performance.frames = 1;
        let _ = render_editor_frame(&context, &mut app, input);
        let viewport = context.screen_rect();
        let first = app.page_geometry.visible_page(
            2,
            app.session.revision(),
            viewport,
            app.page_scale.unwrap_or(1.0),
            false,
        );
        assert_eq!(
            first,
            status_bar::VisiblePage::Available {
                one_based: 1,
                total: layout.pages.len()
            }
        );
        let original_caret = app.session.selection().focus;

        app.performance.frames = 2;
        let wheel = egui::RawInput {
            screen_rect: Some(screen),
            events: vec![
                egui::Event::PointerMoved(egui::pos2(400.0, 300.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -2_000.0),
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        let mut during_scroll = status_bar::VisiblePage::Unavailable;
        let _ = context.run(wheel, |context| {
            during_scroll = app.page_geometry.visible_page(
                app.performance.frames,
                app.session.revision(),
                context.screen_rect(),
                app.page_scale.unwrap_or(1.0),
                page_geometry_input_invalidated(context, app.page_geometry.clip()),
            );
            egui::CentralPanel::default().show(context, |ui| app.live_document_surface(ui));
        });
        assert_eq!(during_scroll, status_bar::VisiblePage::Unavailable);

        let mut after_scroll = status_bar::VisiblePage::Unavailable;
        for frame in 3..=60 {
            app.performance.frames = frame;
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let _ = context.run(input, |context| {
                after_scroll = app.page_geometry.visible_page(
                    app.performance.frames,
                    app.session.revision(),
                    context.screen_rect(),
                    app.page_scale.unwrap_or(1.0),
                    page_geometry_input_invalidated(context, app.page_geometry.clip()),
                );
                egui::CentralPanel::default().show(context, |ui| app.live_document_surface(ui));
            });
            if matches!(
                after_scroll,
                status_bar::VisiblePage::Available { one_based, .. } if one_based > 1
            ) {
                break;
            }
        }
        assert!(
            matches!(
                after_scroll,
                status_bar::VisiblePage::Available { one_based, .. } if one_based > 1
            ),
            "expected settled scrolled geometry to select a later sheet, got {after_scroll:?}"
        );
        assert_eq!(app.session.selection().focus, original_caret);
    }

    #[test]
    fn visible_page_new_replacement_invalidates_revision_alias_before_projection() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let viewport = seed_page_snapshot(
            &mut app,
            vec![
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(700.0, 100.0)),
                egui::Rect::from_min_max(egui::pos2(0.0, 110.0), egui::pos2(700.0, 210.0)),
                egui::Rect::from_min_max(egui::pos2(0.0, 220.0), egui::pos2(700.0, 320.0)),
            ],
        );
        assert_eq!(app.session.revision(), 0);
        assert_eq!(
            project_page_status(&mut app, 2, viewport),
            status_bar::VisiblePage::Available {
                one_based: 1,
                total: 3
            }
        );

        app.dispatch_toolbar_action(ToolbarAction::New);

        assert_eq!(
            app.session.revision(),
            0,
            "replacement aliases the old revision"
        );
        assert_eq!(
            project_page_status(&mut app, 2, viewport),
            status_bar::VisiblePage::Unavailable,
            "successful New must invalidate before the footer's first page projection"
        );
        let selection_after_new = app.session.selection();
        let repaint_requests_before_publish = app.performance.repaint_requests;
        assert_eq!(
            render_fresh_editor_geometry(&mut app),
            status_bar::VisiblePage::Available {
                one_based: 1,
                total: 1
            }
        );
        assert!(app.performance.repaint_requests > repaint_requests_before_publish);
        assert_eq!(app.session.selection(), selection_after_new);
        assert!(!app.surface_focused);
    }

    #[test]
    fn visible_page_open_replacement_invalidates_revision_alias_and_failed_open_does_not() {
        let root =
            std::env::temp_dir().join(format!("scriptcraft-wo002a-open-{}", std::process::id()));
        let saved_path = root.join("replacement.fdx");
        let _ = std::fs::create_dir_all(&root);

        let mut source = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        source.save_fdx_as_at(&saved_path);
        assert!(source.error.is_none());
        source
            .session
            .insert(&"action line\n".repeat(260))
            .unwrap_or_else(|_| unreachable!());
        source.save_native();
        assert!(source.error.is_none());
        let expected_total = ScreenplayLayout::build(source.session.project())
            .unwrap_or_else(|_| unreachable!())
            .pages
            .len();
        assert!(expected_total > 1);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let viewport = seed_page_snapshot(
            &mut app,
            vec![egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(700.0, 500.0),
            )],
        );
        assert_eq!(app.session.revision(), 0);
        assert_eq!(
            project_page_status(&mut app, 2, viewport),
            status_bar::VisiblePage::Available {
                one_based: 1,
                total: 1
            }
        );

        app.open_native_at(&saved_path);

        assert_eq!(
            app.session.revision(),
            0,
            "successful Open aliases revision zero"
        );
        assert_eq!(app.native_path.as_deref(), Some(saved_path.as_path()));
        assert!(app.error.is_none());
        assert_eq!(
            project_page_status(&mut app, 2, viewport),
            status_bar::VisiblePage::Unavailable,
            "successful Open must invalidate before the footer's first page projection"
        );
        let selection_after_open = app.session.selection();
        let repaint_requests_before_publish = app.performance.repaint_requests;
        assert_eq!(
            render_fresh_editor_geometry(&mut app),
            status_bar::VisiblePage::Available {
                one_based: 1,
                total: expected_total
            }
        );
        assert!(app.performance.repaint_requests > repaint_requests_before_publish);
        assert_eq!(app.session.selection(), selection_after_open);
        assert!(!app.surface_focused);

        let mut failed_app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let failed_viewport = seed_page_snapshot(
            &mut failed_app,
            vec![egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(700.0, 500.0),
            )],
        );
        failed_app.native_path = Some(PathBuf::from("kept-project.fdx"));
        failed_app.open_native_at(&root.join("missing.fdx"));
        assert!(failed_app.error.is_some());
        assert_eq!(
            failed_app.native_path.as_deref(),
            Some(std::path::Path::new("kept-project.fdx"))
        );
        assert_eq!(
            project_page_status(&mut failed_app, 2, failed_viewport),
            status_bar::VisiblePage::Available {
                one_based: 1,
                total: 1
            },
            "failed Open must preserve the prior published page snapshot"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn status_bar_marks_scroll_and_caret_reveal_as_stale_before_fresh_publish() {
        let context = test_context();
        let clip = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(300.0, 400.0));
        let input = egui::RawInput {
            events: vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 80.0),
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        let mut invalidated = false;
        let _ = context.run(input, |context| {
            invalidated = page_geometry_input_invalidated(context, Some(clip));
        });
        assert!(invalidated);

        let mut tracker = ViewportPageTracker::default();
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(500.0, 600.0));
        let pages = vec![egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(300.0, 500.0),
        )];
        assert!(tracker.publish(1, 4, viewport, 1.0, pages, clip));
        assert_eq!(
            tracker.visible_page(2, 4, viewport, 1.0, true),
            status_bar::VisiblePage::Unavailable
        );
    }
    use scriptcraft_core::layout::{CHAR_WIDTH, LINE_HEIGHT};
    use scriptcraft_core::Role;

    #[test]
    fn native_startup_selects_the_single_locked_viewport_tuple() {
        let StartupPresentation { options } = startup_presentation();
        let expected = native_options();

        assert_eq!(options.viewport.title, None);
        assert_eq!(options.viewport.title_shown, Some(true));
        assert_eq!(options.viewport.titlebar_shown, Some(true));
        assert_eq!(options.viewport.titlebar_buttons_shown, Some(true));
        assert_eq!(options.viewport.fullsize_content_view, Some(false));
        assert_eq!(options.viewport.decorations, Some(true));
        assert_eq!(options.viewport.transparent, Some(false));
        assert_eq!(options.viewport.title, expected.viewport.title);
        assert_eq!(options.viewport.title_shown, expected.viewport.title_shown);
        assert_eq!(
            options.viewport.titlebar_shown,
            expected.viewport.titlebar_shown
        );
        assert_eq!(
            options.viewport.titlebar_buttons_shown,
            expected.viewport.titlebar_buttons_shown
        );
        assert_eq!(
            options.viewport.fullsize_content_view,
            expected.viewport.fullsize_content_view
        );
        assert_eq!(options.viewport.decorations, expected.viewport.decorations);
        assert_eq!(options.viewport.transparent, expected.viewport.transparent);
    }

    #[test]
    fn toolbar_starts_at_content_boundary_and_preserves_full_width_and_height() {
        let panel = egui::Rect::from_min_size(egui::pos2(12.0, 8.0), Vec2::new(800.0, 120.0));
        let toolbar_height = 48.0;
        let toolbar = toolbar_layout_rect(panel, toolbar_height);

        assert_eq!(toolbar.left(), panel.left());
        assert_eq!(toolbar.right(), panel.right());
        assert_eq!(toolbar.width(), panel.width());
        assert_eq!(toolbar.top(), panel.top());
        assert_eq!(toolbar.height(), toolbar_height);
        assert!(panel.contains_rect(toolbar));
    }

    #[test]
    fn toolbar_chrome_uses_open_widget_fill_without_mutating_style() {
        let ctx = test_context();
        let mut style = (*ctx.style()).clone();
        style.visuals.panel_fill = Color32::from_rgb(17, 34, 51);
        style.visuals.widgets.open.weak_bg_fill = Color32::from_rgb(68, 85, 102);
        ctx.set_style(style.clone());

        assert_eq!(
            toolbar_chrome_color(&ctx),
            style.visuals.widgets.open.weak_bg_fill
        );
        assert_eq!(
            chrome_frame(&ctx).fill,
            style.visuals.widgets.open.weak_bg_fill
        );
        assert_eq!(
            central_panel_frame(&ctx).fill,
            style.visuals.widgets.open.weak_bg_fill
        );
        assert_eq!(ctx.style().visuals.panel_fill, style.visuals.panel_fill);
    }

    #[test]
    fn toolbar_group_hover_response_is_deterministic_and_preserves_label_color() {
        let ctx = test_context();
        let style = ctx.style();
        assert_eq!(toolbar_group_fill(&style, false), Color32::TRANSPARENT);
        assert_eq!(
            toolbar_group_fill(&style, true),
            style.visuals.widgets.hovered.bg_fill
        );
        assert_eq!(style.visuals.text_color(), ctx.style().visuals.text_color());
    }

    #[test]
    fn toolbar_icon_hover_region_is_centered_and_excludes_the_label_area() {
        let group = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), Vec2::new(72.0, 48.0));
        let control = toolbar_icon_control_rect(group, Vec2::splat(24.0));
        let icon = toolbar_icon_hover_rect(control, TOOLBAR_ICON_HOVER_PADDING);

        assert_eq!(icon.min.x, 32.0);
        assert_eq!(icon.max.x, 60.0);
        assert_eq!(icon.min.y, 18.0);
        assert_eq!(icon.max.y, 46.0);
        assert!(icon.center().x == group.center().x);
        assert!(icon.max.y < group.max.y);
        assert!(!icon.contains(egui::pos2(group.center().x, 60.0)));
    }

    #[test]
    fn toolbar_icon_hover_owner_has_deterministic_outside_edge_and_center_positions() {
        let group = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), Vec2::new(72.0, 48.0));
        let control = toolbar_icon_control_rect(group, Vec2::splat(24.0));
        let hover = toolbar_icon_hover_rect(control, 2.0);

        assert!(!toolbar_icon_hover_owner(
            hover,
            egui::pos2(hover.min.x - 0.1, hover.center().y)
        ));
        assert!(toolbar_icon_hover_owner(hover, hover.min));
        assert!(toolbar_icon_hover_owner(hover, control.center()));
        assert_eq!(hover.min.x, control.min.x - 2.0);
        assert_eq!(hover.max.x, control.max.x + 2.0);
        assert_eq!(hover.min.y, control.min.y - 2.0);
        assert_eq!(hover.max.y, control.max.y + 2.0);
    }

    #[test]
    fn cursor_intents_are_arrow_for_controls_and_text_for_document_surface() {
        assert_eq!(toolbar_cursor_icon(), egui::CursorIcon::Default);
        assert_eq!(document_cursor_icon(), egui::CursorIcon::Text);
    }

    #[test]
    fn elements_cursor_ledger_keeps_closed_regions_on_arrow_and_document_on_text() {
        let ledger = elements_cursor_ledger();

        assert_eq!(ledger.outer_group.width(), TOOLBAR_ELEMENTS_WIDTH);
        assert!(toolbar_rect_contains(
            ledger.outer_group,
            ledger.stock_combo
        ));
        assert!(toolbar_rect_contains(
            ledger.stock_combo,
            ledger.closed_selected_galley
        ));
        assert!(toolbar_rect_contains(
            ledger.outer_group,
            ledger.decorative_label
        ));
        assert_eq!(ledger.closed_cursor, egui::CursorIcon::Default);
        assert_eq!(ledger.popup_cursor, egui::CursorIcon::Default);
        assert_eq!(ledger.document_cursor, egui::CursorIcon::Text);

        for point in [
            ledger.stock_combo.center(),
            ledger.closed_selected_galley.center(),
            ledger.decorative_label.center(),
            ledger.group_gap.center(),
            ledger.popup_row.center(),
        ] {
            assert_ne!(
                point,
                ledger.document.center(),
                "Elements ledger region must not be document-owned"
            );
        }
        assert!(ledger.document.contains(ledger.document.center()));
    }

    #[test]
    fn decorative_elements_label_has_no_interaction_response() {
        let ledger = toolbar_combo_boundary_ledger(TOOLBAR_ELEMENTS_WIDTH);

        assert!(ledger.decorative_label_response.width() > 0.0);
        assert!(ledger.decorative_label_response.height() > 0.0);
        assert!(toolbar_rect_contains(
            ledger.group_response,
            ledger.decorative_label_response
        ));
    }

    #[test]
    fn cursor_owner_geometry_covers_elements_label_and_document_surface_only() {
        let elements_group =
            egui::Rect::from_min_size(egui::pos2(20.0, 10.0), Vec2::new(92.0, 48.0));
        let document_surface =
            egui::Rect::from_min_size(egui::pos2(0.0, 70.0), Vec2::new(400.0, 300.0));

        assert!(surface_cursor_owner(elements_group, egui::pos2(25.0, 45.0)));
        assert!(!surface_cursor_owner(
            elements_group,
            egui::pos2(15.0, 45.0)
        ));
        assert!(surface_cursor_owner(
            document_surface,
            egui::pos2(200.0, 200.0)
        ));
        assert!(!surface_cursor_owner(
            document_surface,
            egui::pos2(200.0, 60.0)
        ));
    }

    #[test]
    fn toolbar_icon_hover_fill_uses_the_existing_hover_color_source_only() {
        let ctx = test_context();
        let style = ctx.style();
        assert_eq!(
            toolbar_group_fill(&style, true),
            style.visuals.widgets.hovered.bg_fill
        );
        assert_eq!(toolbar_group_fill(&style, false), Color32::TRANSPARENT);
        assert_eq!(TOOLBAR_ICON_HOVER_PADDING, 2.0);
    }

    #[test]
    fn toolbar_chrome_frame_preserves_top_panel_geometry_defaults() {
        let ctx = test_context();
        let actual = chrome_frame(&ctx);

        assert_eq!(actual.inner_margin, egui::Margin::ZERO);
        assert_eq!(actual.outer_margin, egui::Margin::ZERO);
        assert_eq!(actual.stroke, egui::Stroke::NONE);
        assert_eq!(actual.fill, toolbar_chrome_color(&ctx));
    }

    #[test]
    fn restored_toolbar_padding_stays_inside_the_exact_panel_boundary() {
        let ctx = test_context();
        let row_height = ctx.style().spacing.interact_size.y * 2.0;

        assert_eq!(TOOLBAR_BOTTOM_PADDING, 2.0);
        assert_eq!(toolbar_content_height(&ctx.style()), row_height + 2.0);

        let panel = egui::Rect::from_min_size(
            egui::pos2(12.25, 4.5),
            Vec2::new(800.0, toolbar_content_height(&ctx.style())),
        );
        let row = egui::Rect::from_min_size(panel.left_top(), Vec2::new(panel.width(), row_height));
        let bottom_padding =
            egui::Rect::from_min_max(egui::pos2(panel.left(), row.bottom()), panel.right_bottom());

        assert_eq!(row.top(), panel.top());
        assert_eq!(row.bottom(), bottom_padding.top());
        assert_eq!(bottom_padding.bottom(), panel.bottom());
        assert_eq!(bottom_padding.height(), TOOLBAR_BOTTOM_PADDING);
        assert!(panel.contains_rect(row));
        assert!(panel.contains_rect(bottom_padding));
    }

    #[test]
    fn viewport_clear_color_matches_the_chrome_surface_owner() {
        let ctx = test_context();
        let visuals = ctx.style().visuals.clone();

        assert_eq!(
            chrome_clear_color(&visuals),
            toolbar_chrome_color(&ctx).to_normalized_gamma_f32()
        );
    }

    #[test]
    fn presentation_metrics_are_stable_for_normal_and_maximized_widths() {
        let normal = presentation_metrics(800.0, 612.0, 12.0);
        let maximized = presentation_metrics(1_400.0, 612.0, 12.0);

        assert!((normal.scale - maximized.scale).abs() <= PRESENTATION_METRIC_TOLERANCE);
        assert!((normal.page_width - maximized.page_width).abs() <= PRESENTATION_METRIC_TOLERANCE);
        assert!((normal.font_size - maximized.font_size).abs() <= PRESENTATION_METRIC_TOLERANCE);
    }

    #[test]
    fn presentation_metrics_do_not_shrink_to_narrow_viewports() {
        let narrow = presentation_metrics(400.0, 612.0, 12.0);
        let stable = presentation_metrics(800.0, 612.0, 12.0);

        assert_eq!(narrow.page_width, CURRENT_DEFAULT_PAGE_WIDTH);
        assert_eq!(narrow, stable);
        assert_eq!(TOOLBAR_ELEMENTS_WIDTH, 128.0);
    }

    #[test]
    fn wo001_zoom_geometry_uses_fixed_baseline_and_scales_every_integer_percent() {
        for available_width in [320.0, 400.0, 800.0, 1_400.0] {
            let baseline = presentation_metrics(available_width, 612.0, 12.0);
            let at_100 = presentation_metrics_at_zoom(available_width, 612.0, 12.0, 100);
            assert_eq!(at_100, baseline);
            for percent in status_bar::MIN_ZOOM_PERCENT..=status_bar::MAX_ZOOM_PERCENT {
                let zoomed = presentation_metrics_at_zoom(available_width, 612.0, 12.0, percent);
                let factor = percent as f32 / 100.0;
                assert!((zoomed.scale - baseline.scale * factor).abs() < 0.001);
                assert!((zoomed.page_width - baseline.page_width * factor).abs() < 0.001);
                assert!((zoomed.font_size - baseline.font_size * factor).abs() < 0.001);
            }
        }
        let narrow_maximum = presentation_metrics_at_zoom(320.0, 612.0, 12.0, 300);
        assert!(narrow_maximum.page_width > 320.0);
    }

    #[test]
    fn zoom_state_is_transient_and_preserves_layout_export_dirty_and_undo_contracts() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let initial_project = app.session.project().clone();
        let initial_pdf =
            exports::export_pdf(app.session.project()).unwrap_or_else(|_| unreachable!());
        let initial_revision = app.session.revision();
        let initial_dirty = app.session.dirty();
        let initial_layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());

        for percent in status_bar::ZOOM_PRESETS {
            app.zoom_percent = percent;
            assert_eq!(app.session.revision(), initial_revision);
            assert_eq!(app.session.dirty(), initial_dirty);
            assert_eq!(app.session.project(), &initial_project);
            assert_eq!(
                exports::export_pdf(app.session.project()).unwrap_or_else(|_| unreachable!()),
                initial_pdf
            );
            let layout =
                ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
            assert_eq!(layout.pages.len(), initial_layout.pages.len());
            let source_ranges = |layout: &ScreenplayLayout| {
                layout
                    .pages
                    .iter()
                    .flat_map(|page| page.lines.iter())
                    .map(|line| (line.element, line.source.start, line.source.end))
                    .collect::<Vec<_>>()
            };
            assert_eq!(source_ranges(&layout), source_ranges(&initial_layout));
        }

        assert!(app.session.insert("draft").is_ok());
        let dirty_project = app.session.project().clone();
        let dirty_revision = app.session.revision();
        for percent in status_bar::MIN_ZOOM_PERCENT..=status_bar::MAX_ZOOM_PERCENT {
            app.zoom_percent = percent;
            assert!(app.session.dirty());
            assert_eq!(app.session.revision(), dirty_revision);
            assert_eq!(app.session.project(), &dirty_project);
        }
        assert!(app.session.undo().is_ok());
        assert!(!app.session.dirty());
        assert!(app.session.redo().is_ok());
        assert!(app.session.dirty());
    }

    #[test]
    fn native_zoom_events_change_transient_page_zoom_and_leave_editor_input_untouched() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let context = test_context();
        let initial_revision = app.session.revision();
        let initial_selection = app.session.selection();
        let initial_project = app.session.project().clone();
        let mut raw_input = egui::RawInput {
            events: vec![
                egui::Event::Zoom(1.02),
                egui::Event::Zoom(1.02),
                egui::Event::Zoom(1.02),
            ],
            ..Default::default()
        };

        eframe::App::raw_input_hook(&mut app, &context, &mut raw_input);
        assert_eq!(app.zoom_percent, 106);
        assert!((app.pinch_zoom_percent - 106.1208).abs() < 0.001);
        assert_eq!(app.page_scale, None);
        assert_eq!(app.session.revision(), initial_revision);
        assert_eq!(app.session.selection(), initial_selection);

        assert!(raw_input.events.is_empty(), "native zoom was consumed once");
        let mut egui_zoom_delta = 0.0;
        let _ = context.run(raw_input, |context| {
            egui_zoom_delta = context.input(|input| input.zoom_delta());
        });
        assert_eq!(egui_zoom_delta, 1.0);
        app.dispatch_surface_events(&[], egui::Modifiers::default());
        assert_eq!(app.session.revision(), initial_revision);
        assert_eq!(app.session.selection(), initial_selection);
        assert_eq!(app.session.project(), &initial_project);
        assert_eq!(app.native_input_diagnostics.received_events, 3);
        assert_eq!(app.native_input_diagnostics.last_event_category, "zoom");

        let context = test_context();
        context.enable_accesskit();
        let mut zoom = app.zoom_percent;
        let output = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                assert!(!status_bar::show(
                    ui,
                    status_bar::VisiblePage::Available {
                        one_based: 1,
                        total: 1,
                    },
                    &status_bar::project_center(false, "", None),
                    status_bar::DiagnosticProjection {
                        focused: false,
                        focus_lock: false,
                        dispatch_enabled: false,
                        received_events: 0,
                        dispatched_frames: 0,
                        pointer_activations: 0,
                        last_event_category: "",
                        performance_events: 0,
                        layout_builds: 0,
                        layout_cache_hits: 0,
                        painted_pages: 0,
                        repaint_requests: 0,
                    },
                    &mut zoom,
                ));
            });
        });
        assert_eq!(zoom, 106);
        assert!(output.shapes.iter().any(|clipped| matches!(
            &clipped.shape,
            egui::epaint::Shape::Text(text) if text.galley.text() == "106%"
        )));
        assert!(output
            .platform_output
            .accesskit_update
            .as_ref()
            .is_some_and(|update| update.nodes.iter().any(|(_, node)| {
                node.role() == egui::accesskit::Role::Slider
                    && node.name() == Some("Page zoom")
                    && node.numeric_value() == Some(106.0)
            })));
    }

    #[test]
    fn native_zoom_rejects_invalid_events_and_clamps_both_bounds() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let context = test_context();

        for factor in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut input = egui::RawInput {
                events: vec![egui::Event::Zoom(factor)],
                ..Default::default()
            };
            eframe::App::raw_input_hook(&mut app, &context, &mut input);
            assert_eq!(app.zoom_percent, 100, "invalid factor {factor:?}");
            assert_eq!(app.pinch_zoom_percent, 100.0);
        }

        let mut outward = egui::RawInput {
            events: vec![egui::Event::Zoom(100.0)],
            ..Default::default()
        };
        eframe::App::raw_input_hook(&mut app, &context, &mut outward);
        assert_eq!(app.zoom_percent, status_bar::MAX_ZOOM_PERCENT);
        assert_eq!(
            app.pinch_zoom_percent,
            f64::from(status_bar::MAX_ZOOM_PERCENT)
        );

        let mut inward = egui::RawInput {
            events: vec![egui::Event::Zoom(0.001)],
            ..Default::default()
        };
        eframe::App::raw_input_hook(&mut app, &context, &mut inward);
        assert_eq!(app.zoom_percent, status_bar::MIN_ZOOM_PERCENT);
        assert_eq!(
            app.pinch_zoom_percent,
            f64::from(status_bar::MIN_ZOOM_PERCENT)
        );
    }

    #[test]
    fn wo001_ui_scale_hook_uses_native_pixels_once_and_replaces_fallback_immediately() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let context = test_context();
        for (native, expected_factor, expected_pixels_per_point) in [
            (Some(1.0), 1.0, 1.0),
            (Some(1.5), 1.0, 1.5),
            (Some(2.0), 1.0, 2.0),
            (None, 2.3, 2.3),
            (Some(1.5), 1.0, 1.5),
            (Some(f32::NAN), 2.3, 2.3),
            (Some(0.0), 2.3, 2.3),
            (Some(-1.0), 2.3, 2.3),
            (Some(f32::INFINITY), 2.3, 2.3),
            (Some(f32::NEG_INFINITY), 2.3, 2.3),
            (None, 2.3, 2.3),
        ] {
            let mut input = egui::RawInput::default();
            if let Some(viewport) = input.viewports.get_mut(&input.viewport_id) {
                viewport.native_pixels_per_point = native;
            }
            eframe::App::raw_input_hook(&mut app, &context, &mut input);
            let _ = context.run(input, |_| {});
            assert_eq!(context.zoom_factor(), expected_factor);
            assert_eq!(context.pixels_per_point(), expected_pixels_per_point);
        }
        assert_eq!(status_bar::resolved_ui_pixels_per_point(Some(1.5)), 1.5);
        assert_eq!(status_bar::resolved_ui_pixels_per_point(None), 2.3);
    }

    #[test]
    fn status_controls_override_pinch_fraction_and_continue_from_selected_value() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.pinch_zoom_percent = 143.4;
        app.zoom_percent = status_bar::normalized_zoom_percent(app.pinch_zoom_percent);
        assert_eq!(app.zoom_percent, 143);
        // A control edit is handled by update after status_bar::show; this is
        // the same synchronization performed in that production path.
        app.zoom_percent = 200;
        app.sync_pinch_zoom_from_controls();
        assert_eq!(app.pinch_zoom_percent, 200.0);
        assert_eq!(
            status_bar::normalized_zoom_percent(
                status_bar::zoom_after_native_factor(app.pinch_zoom_percent, 1.01)
                    .unwrap_or(app.pinch_zoom_percent)
            ),
            202
        );
    }

    #[test]
    fn zoomed_editor_presents_wide_pages_with_horizontal_and_vertical_scroll() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.zoom_percent = status_bar::MAX_ZOOM_PERCENT;
        app.session
            .insert(&"wide line\n".repeat(240))
            .unwrap_or_else(|_| unreachable!());
        let context = test_context();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));

        let page_rects = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::epaint::Shape::Rect(rect) if rect.fill == Color32::from_gray(246) => {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let initial = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
        );
        let initial_pages = page_rects(&initial);
        let initial_page = initial_pages
            .first()
            .copied()
            .unwrap_or(egui::Rect::NOTHING);
        assert!(initial_page.width() > screen.width());
        assert!(initial_page.height() > screen.height());
        assert!(app.page_scale.is_some_and(|scale| scale > 1.0));

        let horizontally_scrolled = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(screen),
                events: vec![
                    egui::Event::PointerMoved(egui::pos2(400.0, 300.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(-900.0, 0.0),
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
        );
        let horizontal_pages = page_rects(&horizontally_scrolled);
        let horizontal_page = horizontal_pages
            .first()
            .copied()
            .unwrap_or(egui::Rect::NOTHING);
        assert!(app.last_scroll_offset.x > 0.0);
        assert_eq!(horizontal_page.width(), initial_page.width());

        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(screen),
                events: vec![
                    egui::Event::PointerMoved(egui::pos2(400.0, 300.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -900.0),
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
        );
        assert!(app.last_scroll_offset.y > 0.0);
        assert!(app.page_scale.is_some_and(|scale| scale > 1.0));

        let retained_x = app.last_scroll_offset.x;
        let narrower_screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(700.0, 600.0));
        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(narrower_screen),
                ..Default::default()
            },
        );
        assert!(app.last_scroll_offset.x + 0.01 >= retained_x);
        assert_eq!(
            app.last_scroll_page_rects.first().map(egui::Rect::width),
            Some(initial_page.width()),
            "resizing must not refit the selected page zoom"
        );

        let fitting_screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_600.0, 900.0));
        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(fitting_screen),
                ..Default::default()
            },
        );
        assert!(app.last_scroll_offset.x.abs() < 0.01);
    }

    #[test]
    fn wo001_scroll_offsets_reach_edges_and_clamp_or_persist_across_resize_and_zoom() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.zoom_percent = 300;
        app.session
            .insert(&"wide paragraph\n".repeat(240))
            .unwrap_or_else(|_| unreachable!());
        let narrow = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(narrow),
                ..Default::default()
            },
        );
        let width_before = app
            .last_scroll_page_rects
            .first()
            .map(egui::Rect::width)
            .unwrap_or(0.0);

        let wheel = |width: f32, x: f32, y: f32| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(width, 600.0),
            )),
            events: vec![
                egui::Event::PointerMoved(egui::pos2(400.0, 300.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(x, y),
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        let settle = |app: &mut ScriptCraftApp, width: f32, height: f32| {
            for _ in 0..6 {
                let time = context.input(|input| input.time) + 0.2;
                let _ = render_editor_frame(
                    &context,
                    app,
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width, height),
                        )),
                        time: Some(time),
                        predicted_dt: 0.1,
                        ..Default::default()
                    },
                );
            }
        };
        let _ = render_editor_frame(&context, &mut app, wheel(800.0, -900.0, 0.0));
        settle(&mut app, 800.0, 600.0);
        let max_x = (app.last_scroll_content_size.x - app.last_scroll_inner_rect.width()).max(0.0);
        assert!(max_x > 0.0);
        assert!(app.last_scroll_offset.x > 0.0);
        assert!(app.last_scroll_offset.x <= max_x + 0.01);

        let _ = render_editor_frame(&context, &mut app, wheel(800.0, 900.0, 0.0));
        settle(&mut app, 800.0, 600.0);
        assert!(
            app.last_scroll_offset.x.abs() < 0.01,
            "reverse horizontal scroll should reach the leading edge, got {}",
            app.last_scroll_offset.x
        );
        let _ = render_editor_frame(&context, &mut app, wheel(800.0, -120.0, 0.0));
        settle(&mut app, 800.0, 600.0);
        let offset_before_resize = app.last_scroll_offset.x;
        assert!(offset_before_resize > 0.0 && offset_before_resize < max_x - 1.0);
        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(700.0, 600.0),
                )),
                ..Default::default()
            },
        );
        settle(&mut app, 700.0, 600.0);
        assert!(
            (app.last_scroll_offset.x - offset_before_resize).abs() < 0.01,
            "valid horizontal offset should survive width resize: before={offset_before_resize}, after={}, max={}",
            app.last_scroll_offset.x,
            (app.last_scroll_content_size.x - app.last_scroll_inner_rect.width()).max(0.0)
        );
        assert_eq!(
            app.last_scroll_page_rects.first().map(egui::Rect::width),
            Some(width_before)
        );

        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1_600.0, 900.0),
                )),
                ..Default::default()
            },
        );
        assert!(app.last_scroll_offset.x.abs() < 0.01);

        let _ = render_editor_frame(&context, &mut app, wheel(800.0, -120.0, 0.0));
        settle(&mut app, 800.0, 600.0);
        app.zoom_percent = 100;
        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(narrow),
                ..Default::default()
            },
        );
        assert!(app.last_scroll_offset.x.abs() < 0.01);
        assert_eq!(
            app.last_scroll_page_rects.first().map(egui::Rect::width),
            Some(CURRENT_DEFAULT_PAGE_WIDTH)
        );
    }

    #[test]
    fn zoom_transforms_keep_pointer_hit_testing_on_the_same_source_position() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let line = &layout.pages[0].lines[0];
        let page_origin = egui::pos2(25.0, 40.0);

        for zoom_percent in [75, 100, 137, 200, 300] {
            let metrics = presentation_metrics_at_zoom(
                800.0,
                layout.inputs.paper_width as f32,
                layout.inputs.font_size as f32,
                zoom_percent,
            );
            let page_rect = egui::Rect::from_min_size(
                page_origin,
                Vec2::new(
                    metrics.page_width,
                    layout.inputs.paper_height as f32 * metrics.scale,
                ),
            );
            let pointer = egui::pos2(
                page_rect.left() + line.geometry.x.to_points_f32() * metrics.scale,
                page_rect.top() + line.geometry.y.to_points_f32() * metrics.scale,
            );
            assert_eq!(
                app.caret_from_pointer(&layout, 0, page_rect, metrics.scale, pointer),
                Some(Position {
                    element: line.element,
                    offset: line.source.start,
                }),
                "zoom {zoom_percent}% must preserve semantic pointer mapping"
            );
        }
    }

    #[test]
    fn visible_page_returns_after_fresh_zoom_geometry_is_published() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert(&"page\n".repeat(180))
            .unwrap_or_else(|_| unreachable!());
        let context = test_context();
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        app.performance.frames = 1;
        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
        );
        assert!(matches!(
            project_page_status(&mut app, 2, context.screen_rect()),
            status_bar::VisiblePage::Available { .. }
        ));
        let fit_scale = app.page_scale.unwrap_or(0.0);

        app.zoom_percent = 200;
        app.page_geometry.invalidate();
        app.page_scale = None;
        assert_eq!(
            project_page_status(&mut app, 2, context.screen_rect()),
            status_bar::VisiblePage::Unavailable
        );
        app.performance.frames = 2;
        let _ = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
        );
        assert_eq!(app.zoom_percent, 200);
        assert!((app.page_scale.unwrap_or(0.0) - fit_scale * 2.0).abs() < 0.001);
        assert!(matches!(
            project_page_status(&mut app, 3, context.screen_rect()),
            status_bar::VisiblePage::Available { .. }
        ));
    }

    #[test]
    fn default_toolbar_has_approved_order_and_labels() {
        let items = default_toolbar_items();
        let labels: Vec<_> = items.iter().map(|item| item.label).collect();
        assert_eq!(
            labels,
            vec![
                "New",
                "Open",
                "Save",
                "Save As",
                "Flexible Space",
                "Elements",
                "Flexible Space",
                "Export PDF",
            ]
        );
        assert!(matches!(
            items[0].kind,
            ToolbarItemKind::Action(ToolbarAction::New)
        ));
        assert!(matches!(items[4].kind, ToolbarItemKind::FlexibleSpace));
        assert!(matches!(items[5].kind, ToolbarItemKind::Elements));
        assert!(items.iter().all(|item| !item.label.is_empty()));
        assert!(items
            .iter()
            .filter(|item| matches!(item.kind, ToolbarItemKind::Action(_)))
            .all(|item| !item.icon.is_empty()));
    }

    #[test]
    fn toolbar_model_keeps_normal_and_flexible_spacing_typed() {
        assert!(matches!(
            ToolbarItem::space("space").kind,
            ToolbarItemKind::Space
        ));
        assert!(matches!(
            ToolbarItem::flexible_space("flex").kind,
            ToolbarItemKind::FlexibleSpace
        ));
    }

    #[test]
    fn toolbar_action_metadata_keeps_icon_and_label_separate() {
        for item in default_toolbar_items()
            .iter()
            .copied()
            .filter(|item| matches!(item.kind, ToolbarItemKind::Action(_)))
        {
            assert!(!item.icon.is_empty());
            assert!(!item.label.is_empty());
            assert_ne!(item.icon, item.label);
        }
    }

    #[test]
    fn elements_item_keeps_selected_role_and_presentation_label() {
        let item = ToolbarItem::elements();
        assert_eq!(item.label, "Elements");
        assert_eq!(screenplay_role_label(Role::Dialogue), "Dialogue");
        assert!(matches!(item.kind, ToolbarItemKind::Elements));
    }

    #[test]
    fn elements_closed_text_is_centered_in_the_arrow_excluded_content_area() {
        let control = egui::Rect::from_min_size(egui::pos2(10.0, 4.0), Vec2::new(128.0, 24.0));
        let text_rect = elements_closed_text_rect(control, Vec2::splat(4.0), 4.0, 12.0);

        assert_eq!(text_rect.left(), 14.0);
        assert_eq!(text_rect.right(), 118.0);
        assert_eq!(text_rect.width(), 104.0);
        assert_eq!(
            elements_closed_text_position(text_rect, Vec2::new(40.0, 14.0)).x,
            46.0
        );

        let arrow_rect = egui::Align2::RIGHT_CENTER
            .align_size_within_rect(Vec2::new(12.0, 12.0), control.shrink2(Vec2::splat(4.0)));
        assert_eq!(arrow_rect.center().x, 128.0);
        assert_eq!(text_rect.right() + 4.0, arrow_rect.left());
    }

    #[test]
    fn elements_closed_text_layout_truncates_long_labels_without_leaving_the_control() {
        let context = test_context();
        let control = egui::Rect::from_min_size(egui::pos2(10.0, 4.0), Vec2::new(128.0, 24.0));
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(400.0, 200.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let text_rect =
                        elements_closed_text_rect(control, ui.spacing().button_padding, 4.0, 12.0);
                    let galley = WidgetText::from(
                        "A deliberately long screenplay role label that must truncate",
                    )
                    .into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        text_rect.width(),
                        TextStyle::Button,
                    );
                    let position = elements_closed_text_position(text_rect, galley.size());

                    assert!(galley.size().x <= text_rect.width());
                    assert!(position.x >= text_rect.left());
                    assert!(position.x + galley.size().x <= text_rect.right());
                });
            },
        );
    }

    #[test]
    fn elements_popup_choices_keep_left_aligned_stock_path_for_all_roles() {
        // The custom presentation is intentionally gated on the ComboBox being
        // closed. Popup entries continue to use the stock selectable_value path,
        // whose menu UI owns left-aligned labels and keyboard navigation.
        assert_eq!(SUPPORTED_SCREENPLAY_ROLES.len(), Role::ALL.len());
        assert_eq!(SUPPORTED_SCREENPLAY_ROLES.map(|(role, _)| role), Role::ALL);
        assert!(SUPPORTED_SCREENPLAY_ROLES
            .iter()
            .all(|(_, label)| !label.is_empty()));
    }

    #[test]
    fn toolbar_allocation_wide_and_exact_fit_are_deterministic() {
        let items = default_toolbar_items();
        let wide = allocate_toolbar(&items, 800.0, test_toolbar_label_width);
        assert!(wide.overflow.is_empty());
        assert_eq!(wide.visible.len(), items.len());
        let fixed_width = items
            .iter()
            .copied()
            .map(|item| toolbar_item_width(item, test_toolbar_label_width))
            .sum::<f32>();
        let reserved =
            fixed_width + TOOLBAR_ROW_PADDING * 2.0 + (items.len() - 1) as f32 * TOOLBAR_ITEM_GAP;
        let expected_flexible_width = (800.0 - reserved) / 2.0;
        assert_eq!(wide.visible[4].1, expected_flexible_width);
        assert_eq!(wide.visible[6].1, expected_flexible_width);

        let exact_width = toolbar_content_width(
            &items
                .iter()
                .copied()
                .map(|item| (item, toolbar_item_width(item, test_toolbar_label_width)))
                .collect::<Vec<_>>(),
            0.0,
        );
        let exact = allocate_toolbar(&items, exact_width, test_toolbar_label_width);
        assert!(exact.overflow.is_empty());
        assert_eq!(exact.visible.len(), items.len());
        assert_eq!(toolbar_content_width(&exact.visible, 0.0), exact_width);
    }

    #[test]
    fn toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu() {
        let allocation =
            allocate_toolbar(&default_toolbar_items(), 300.0, test_toolbar_label_width);
        let visible: Vec<_> = allocation
            .visible
            .iter()
            .map(|(item, _)| item.label)
            .collect();
        let overflow: Vec<_> = allocation.overflow.iter().map(|item| item.label).collect();
        assert_eq!(visible, vec!["New", "Open", "Save", "Save As"]);
        assert_eq!(overflow, vec!["Elements", "Export PDF"]);
        assert_eq!(allocation.overflow_width, TOOLBAR_OVERFLOW_WIDTH);
    }

    #[test]
    fn toolbar_manual_geometry_controls_are_narrow_and_explicit() {
        assert_eq!(TOOLBAR_NORMAL_ITEM_WIDTH, 46.0);
        assert_eq!(TOOLBAR_ROW_PADDING, 4.0);
        assert_eq!(TOOLBAR_ITEM_GAP, 4.0);
        assert_eq!(TOOLBAR_TOP_PADDING, 0.0);
        assert_eq!(TOOLBAR_BOTTOM_PADDING, 2.0);
    }

    #[test]
    fn toolbar_allocation_normal_items_use_the_narrow_manual_width() {
        let allocation = allocate_toolbar(
            &[
                ToolbarItem::action("a", "A", "", ToolbarAction::New),
                ToolbarItem::elements(),
            ],
            200.0,
            test_toolbar_label_width,
        );

        assert_eq!(allocation.visible[0].1, TOOLBAR_NORMAL_ITEM_WIDTH);
        assert_eq!(allocation.visible[1].1, TOOLBAR_ELEMENTS_WIDTH);
    }

    #[test]
    fn toolbar_allocation_total_width_stays_within_available_bounds() {
        for available_width in [300.0, 600.0, 800.0] {
            let allocation = allocate_toolbar(
                &default_toolbar_items(),
                available_width,
                test_toolbar_label_width,
            );
            assert!(
                toolbar_content_width(&allocation.visible, allocation.overflow_width)
                    <= available_width
            );
            assert_eq!(
                allocation.row_width,
                toolbar_content_width(&allocation.visible, allocation.overflow_width)
            );
        }
    }

    #[test]
    fn toolbar_shared_row_edge_contains_final_group_at_actual_content_edge() {
        let content = egui::Rect::from_min_max(egui::pos2(12.0, 0.0), egui::pos2(812.0, 48.0));
        let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(804.0, 48.0));
        let row_left = content.left();
        let available = toolbar_actual_available_width(content, clip, row_left);
        let allocation = allocate_toolbar(
            &default_toolbar_items(),
            available,
            test_toolbar_label_width,
        );

        assert_eq!(available, 792.0);
        assert!(toolbar_row_right_edge(row_left, &allocation) <= clip.right());
        assert_eq!(
            allocation.visible.last().map(|(item, _)| item.label),
            Some("Export PDF")
        );
    }

    #[test]
    fn toolbar_shared_row_edge_contains_wide_and_exact_fit_allocations() {
        let content = egui::Rect::from_min_size(egui::pos2(20.0, 0.0), Vec2::new(1000.0, 48.0));
        let wide = allocate_toolbar(
            &default_toolbar_items(),
            toolbar_actual_available_width(content, content, content.left()),
            test_toolbar_label_width,
        );
        assert!(toolbar_row_right_edge(content.left(), &wide) <= content.right());

        let exact_width = toolbar_content_width(
            &default_toolbar_items()
                .iter()
                .copied()
                .map(|item| (item, toolbar_item_width(item, test_toolbar_label_width)))
                .collect::<Vec<_>>(),
            0.0,
        );
        let exact_content =
            egui::Rect::from_min_size(egui::pos2(20.0, 0.0), Vec2::new(exact_width, 48.0));
        let exact = allocate_toolbar(
            &default_toolbar_items(),
            exact_width,
            test_toolbar_label_width,
        );
        assert!(toolbar_row_right_edge(exact_content.left(), &exact) <= exact_content.right());
        assert!(exact.overflow.is_empty());
    }

    #[test]
    fn toolbar_shared_row_edge_preserves_narrow_overflow_reservation() {
        let content = egui::Rect::from_min_size(egui::pos2(8.0, 0.0), Vec2::new(300.0, 48.0));
        let allocation = allocate_toolbar(
            &default_toolbar_items(),
            toolbar_actual_available_width(content, content, content.left()),
            test_toolbar_label_width,
        );

        assert_eq!(allocation.overflow_width, TOOLBAR_OVERFLOW_WIDTH);
        assert_eq!(
            allocation
                .overflow
                .iter()
                .map(|item| item.label)
                .collect::<Vec<_>>(),
            vec!["Elements", "Export PDF"]
        );
        assert!(toolbar_row_right_edge(content.left(), &allocation) <= content.right());
    }

    #[test]
    fn toolbar_rectangle_ledger_contains_content_clip_row_child_and_final_label() {
        let item_widths = [
            toolbar_content_width(
                &default_toolbar_items()
                    .iter()
                    .copied()
                    .map(|item| (item, toolbar_item_width(item, test_toolbar_label_width)))
                    .collect::<Vec<_>>(),
                0.0,
            ),
            300.0,
            1_000.0,
        ];

        for width in item_widths {
            let content = egui::Rect::from_min_size(egui::pos2(20.0, 0.0), Vec2::new(width, 48.0));
            let clip = egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(content.right().min(980.0), content.bottom()),
            );
            let allocation = allocate_toolbar(
                &default_toolbar_items(),
                toolbar_actual_available_width(content, clip, content.left()),
                test_toolbar_label_width,
            );
            let (content, clip, row, child, final_label) =
                toolbar_rect_ledger(content, clip, content.left(), &allocation);

            assert!(toolbar_rect_contains(content, row));
            assert!(toolbar_rect_contains(clip, row));
            assert!(toolbar_rect_contains(row, child));
            assert!(toolbar_rect_contains(row, final_label));
            assert!(toolbar_rect_contains(clip, final_label));
            assert!(toolbar_row_right_edge(content.left(), &allocation) <= clip.right());
        }
    }

    #[test]
    fn toolbar_combo_boundary_ledger_contains_response_child_group_and_final_edge_cases() {
        let items = default_toolbar_items();
        let exact_width = toolbar_content_width(
            &items
                .iter()
                .copied()
                .map(|item| (item, toolbar_item_width(item, test_toolbar_label_width)))
                .collect::<Vec<_>>(),
            0.0,
        );
        let cases = [
            ("wide", 1_000.0),
            ("exact-fit", exact_width),
            ("narrow", TOOLBAR_NORMAL_ITEM_WIDTH),
        ];

        for (name, width) in cases {
            let ledger = toolbar_combo_boundary_ledger(width);
            assert!(
                ledger.combo_response.width() <= width + f32::EPSILON,
                "{name}: ComboBox response expanded beyond child allocation: {:?} > {:?}",
                ledger.combo_response,
                width
            );
            assert!(
                ledger.decorative_label_response.width() <= width + f32::EPSILON,
                "{name}: child min rect exceeded Elements budget"
            );
            assert!(
                ledger.group_response.width() <= width + f32::EPSILON,
                "{name}: Elements group exceeded allocated width"
            );
        }
    }

    #[test]
    fn toolbar_vertical_padding_is_explicit_and_reduced() {
        assert_eq!(TOOLBAR_TOP_PADDING, 0.0);
        assert_eq!(TOOLBAR_BOTTOM_PADDING, 2.0);
        assert_eq!(TOOLBAR_TOP_PADDING + TOOLBAR_BOTTOM_PADDING, 2.0);
    }

    #[test]
    fn toolbar_allocation_two_flexible_spaces_split_remaining_width_equally() {
        let items = [
            ToolbarItem::action("a", "A", "", ToolbarAction::New),
            ToolbarItem::flexible_space("flex-a"),
            ToolbarItem::action("b", "B", "", ToolbarAction::Save),
            ToolbarItem::flexible_space("flex-b"),
            ToolbarItem::action("c", "C", "", ToolbarAction::ExportPdf),
        ];
        let allocation = allocate_toolbar(&items, 500.0, test_toolbar_label_width);
        let fixed_width = items
            .iter()
            .copied()
            .map(|item| toolbar_item_width(item, test_toolbar_label_width))
            .sum::<f32>();
        let reserved =
            fixed_width + TOOLBAR_ROW_PADDING * 2.0 + (items.len() - 1) as f32 * TOOLBAR_ITEM_GAP;
        let expected_flexible_width = (500.0 - reserved) / 2.0;
        assert_eq!(allocation.visible[1].1, expected_flexible_width);
        assert_eq!(allocation.visible[3].1, expected_flexible_width);
        assert!(allocation.overflow.is_empty());
    }

    #[test]
    fn toolbar_geometry_uses_half_default_gap_and_stays_within_bounds() {
        assert_eq!(TOOLBAR_ITEM_GAP, 8.0 / 2.0);
        let items = default_toolbar_items();
        let allocation = allocate_toolbar(&items, 800.0, test_toolbar_label_width);
        assert!(toolbar_content_width(&allocation.visible, allocation.overflow_width) <= 800.0);
        assert_eq!(
            allocation.visible.last().map(|(item, _)| item.label),
            Some("Export PDF")
        );
    }

    #[test]
    fn toolbar_allocation_wide_maximized_equivalent_keeps_export_pdf_fully_allocated() {
        let items = default_toolbar_items();
        let allocation = allocate_toolbar(&items, 600.0, test_toolbar_label_width);
        assert!(allocation.overflow.is_empty());
        assert_eq!(
            allocation.visible.last().map(|(item, _)| item.label),
            Some("Export PDF")
        );
        assert_eq!(
            allocation.visible.last().map(|(_, width)| *width),
            Some(toolbar_item_width(
                *allocation
                    .visible
                    .last()
                    .map(|(item, _)| item)
                    .unwrap_or(&items[0]),
                test_toolbar_label_width,
            ))
        );
        assert!(toolbar_content_width(&allocation.visible, 0.0) <= 600.0);
    }

    #[test]
    fn toolbar_item_width_is_text_aware_and_includes_group_extent() {
        let export = ToolbarItem::action("export-pdf", "Export PDF", "⇱", ToolbarAction::ExportPdf);
        let measured_label_width = test_toolbar_label_width(export.label);
        assert_eq!(
            toolbar_item_width(export, test_toolbar_label_width),
            TOOLBAR_NORMAL_ITEM_WIDTH.max(measured_label_width + TOOLBAR_GROUP_HORIZONTAL_PADDING)
        );
        assert!(toolbar_item_width(export, test_toolbar_label_width) >= TOOLBAR_NORMAL_ITEM_WIDTH);
        assert_ne!(
            measured_label_width,
            export.label.chars().count() as f32 * 8.0
        );
        assert_eq!(
            toolbar_item_width(ToolbarItem::space("space"), test_toolbar_label_width),
            TOOLBAR_SPACE_WIDTH
        );
    }

    #[test]
    fn toolbar_width_overrides_are_typed_and_resolved_once_for_elements_and_space() {
        let elements = ToolbarItem::elements();
        let space = ToolbarItem::space("space");

        assert_eq!(
            elements.width_override,
            ToolbarWidthOverride::Fixed(TOOLBAR_ELEMENTS_WIDTH)
        );
        assert_eq!(
            toolbar_item_width(elements, |_| 1_000.0),
            TOOLBAR_ELEMENTS_WIDTH
        );
        assert_eq!(
            space.width_override,
            ToolbarWidthOverride::Fixed(TOOLBAR_SPACE_WIDTH)
        );
        assert_eq!(toolbar_item_width(space, |_| 1_000.0), TOOLBAR_SPACE_WIDTH);
    }

    #[test]
    fn elements_width_target_covers_active_role_label_metrics() {
        let widest_role = SUPPORTED_SCREENPLAY_ROLES
            .iter()
            .map(|(_, label)| test_toolbar_label_width(label))
            .fold(0.0, f32::max);

        assert!(
            widest_role + TOOLBAR_GROUP_HORIZONTAL_PADDING <= TOOLBAR_ELEMENTS_WIDTH,
            "active egui role label metrics require more than the Elements target: {widest_role}"
        );
    }

    #[test]
    fn overflow_entries_route_actions_and_elements_submenu_choices() {
        let allocation =
            allocate_toolbar(&default_toolbar_items(), 300.0, test_toolbar_label_width);
        assert!(allocation
            .overflow
            .iter()
            .any(|item| matches!(item.kind, ToolbarItemKind::Elements)));

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements.len(), 1);
        assert_eq!(app.focused_role(), Some(Role::SceneHeading));
        assert!(app.session.project().elements[0].text.is_empty());
        assert_eq!(
            app.session.selection().focus,
            Position {
                element: 0,
                offset: 0
            }
        );
        app.dispatch_toolbar_action(ToolbarAction::New);
        assert_eq!(app.status, "New in-memory project");
        assert_eq!(app.session.project().elements.len(), 1);
        assert_eq!(app.focused_role(), Some(Role::SceneHeading));
        assert!(app.session.project().elements[0].text.is_empty());
        assert_eq!(
            app.session.selection().focus,
            Position {
                element: 0,
                offset: 0
            }
        );
        app.change_role_from_toolbar(Role::Dialogue);
        assert_eq!(app.focused_role(), Some(Role::Dialogue));
        assert!(app.error.is_none());
    }

    #[test]
    fn toolbar_new_dispatch_replaces_session_without_persisting_path() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.native_path = Some(PathBuf::from("existing.json"));
        app.session
            .insert("draft")
            .unwrap_or_else(|_| unreachable!());

        app.dispatch_toolbar_action(ToolbarAction::New);

        assert_eq!(app.session.project().elements.len(), 1);
        assert_eq!(app.session.project().elements[0].role, Role::SceneHeading);
        assert_eq!(app.session.project().elements[0].text, "");
        assert_eq!(
            app.session.selection().focus,
            Position {
                element: 0,
                offset: 0
            }
        );
        assert_eq!(app.native_path, None);
        assert_eq!(app.status, "New in-memory project");
        assert!(app.error.is_none());
    }

    #[test]
    fn elements_role_labels_cover_all_supported_roles() {
        let expected = [
            (Role::SceneHeading, "Scene Heading"),
            (Role::Action, "Action"),
            (Role::Character, "Character"),
            (Role::Dialogue, "Dialogue"),
            (Role::Parenthetical, "Parenthetical"),
            (Role::Transition, "Transition"),
            (Role::Note, "Note"),
            (Role::NewAct, "New Act"),
            (Role::EndOfAct, "End of Act"),
            (Role::CastList, "Cast List"),
            (Role::General, "General"),
            (Role::Shot, "Shot"),
            (Role::Sequence, "Sequence"),
            (Role::Outline1, "Outline 1"),
            (Role::Outline2, "Outline 2"),
            (Role::Outline3, "Outline 3"),
            (Role::Summary, "Summary"),
        ];
        assert_eq!(SUPPORTED_SCREENPLAY_ROLES, expected);
        for (role, label) in expected {
            assert_eq!(screenplay_role_label(role), label);
        }
    }

    #[test]
    fn elements_role_choices_change_the_current_caret_element() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("first")
            .unwrap_or_else(|_| unreachable!());
        app.session.enter().unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 1,
                offset: 0,
            }))
            .unwrap_or_else(|_| unreachable!());

        for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
            app.change_role_from_toolbar(role);
            assert_eq!(app.focused_role(), Some(role), "{label}");
            assert_eq!(screenplay_role_label(role), label);
            assert_eq!(app.session.selection().focus.element, 1);
            assert_eq!(app.session.project().elements[0].role, Role::SceneHeading);
            assert!(app.error.is_none());
        }
    }

    #[test]
    fn rejected_elements_role_change_preserves_session_and_reports_error() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("unchanged")
            .unwrap_or_else(|_| unreachable!());
        app.session.enter().unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection {
                anchor: Position {
                    element: 0,
                    offset: 0,
                },
                focus: Position {
                    element: 1,
                    offset: 0,
                },
            })
            .unwrap_or_else(|_| unreachable!());
        let before = app.session.clone();

        app.change_role_from_toolbar(Role::Dialogue);

        assert_eq!(app.session, before);
        assert!(app.error.is_some());
        assert!(app.status.contains("document unchanged"));
    }

    #[test]
    fn app_has_one_live_surface_and_no_ui_text_buffer() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements.len(), 1);
        assert!(!app.surface_focused);
    }

    #[test]
    fn session_health_reports_valid_live_model_and_fdx_projection() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        activate_legacy_json_project(&mut app);
        let health = app.session_health();
        assert!(health.contains("session valid"));
        assert!(health.contains("FDX projection valid"));
        assert!(!health.contains("JSON"));
    }

    #[test]
    fn layout_cache_hits_unchanged_reads_and_misses_revision_or_profile_changes() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let first = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        let second = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        assert_eq!(first.as_ref(), second.as_ref());
        assert_eq!(app.performance.layout_builds, 1);
        assert_eq!(app.performance.layout_cache_hits, 1);

        let mut changed_inputs = app.session.layout_inputs().clone();
        changed_inputs.line_advance += 1;
        app.session
            .set_layout_inputs(changed_inputs)
            .unwrap_or_else(|_| unreachable!());
        let _ = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.performance.layout_builds, 2);

        app.session
            .insert("revision change")
            .unwrap_or_else(|_| unreachable!());
        let _ = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.performance.layout_builds, 3);
    }

    #[test]
    fn wo001_presentation_geometry_keeps_fixed_page_and_centers_it() {
        let (page_width, padding, scale) = presentation_geometry(800.0, 612.0);
        assert!((page_width - 360.0).abs() < f32::EPSILON);
        assert!((padding - 220.0).abs() < f32::EPSILON);
        assert!((scale - (360.0 / 612.0)).abs() < f32::EPSILON);

        let (narrow_width, narrow_padding, narrow_scale) = presentation_geometry(400.0, 612.0);
        assert!((narrow_width - 360.0).abs() < f32::EPSILON);
        assert!((narrow_padding - 20.0).abs() < f32::EPSILON);
        assert_eq!(narrow_scale, scale);
    }

    #[test]
    fn wo001_scroll_geometry_has_exact_buffered_overflow_threshold() {
        let wide = page_scroll_geometry(800.0, 360.0);
        assert_eq!(wide.content_width, 800.0);
        assert_eq!(wide.leading_padding, 220.0);
        assert!(!wide.horizontal_overflow);

        let exact = page_scroll_geometry(392.0, 360.0);
        assert_eq!(exact.content_width, 392.0);
        assert_eq!(exact.leading_padding, 16.0);
        assert!(!exact.horizontal_overflow);

        let overflow = page_scroll_geometry(391.0, 360.0);
        assert_eq!(overflow.content_width, 392.0);
        assert_eq!(overflow.leading_padding, 16.0);
        assert!(overflow.horizontal_overflow);

        let tiny = page_scroll_geometry(0.0, 360.0);
        assert_eq!(tiny.content_width, 392.0);
        assert_eq!(tiny.leading_padding, 16.0);
        assert!(tiny.horizontal_overflow);

        let invalid = page_scroll_geometry(f32::NAN, 360.0);
        assert_eq!(invalid.viewport_width, 0.0);
        assert_eq!(invalid.content_width, 392.0);
        assert!(invalid.horizontal_overflow);
    }

    #[test]
    fn wo001_rendered_sheet_uses_fixed_logical_width_and_single_system_scaling() {
        for (native_scale, effective_scale) in [
            (Some(1.0), 1.0),
            (Some(1.5), 1.5),
            (Some(2.0), 2.0),
            (None, 2.3),
        ] {
            for zoom_percent in [100, 150] {
                let context = test_context();
                let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
                app.zoom_percent = zoom_percent;
                let screen =
                    egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1_600.0, 1_000.0));
                let mut input = egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                };
                if let Some(viewport) = input.viewports.get_mut(&input.viewport_id) {
                    viewport.native_pixels_per_point = native_scale;
                }
                eframe::App::raw_input_hook(&mut app, &context, &mut input);
                let output = context.run(input, |context| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none())
                        .show(context, |ui| app.live_document_surface(ui));
                });
                let page = output
                    .shapes
                    .iter()
                    .find_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Rect(rect) if rect.fill == Color32::from_gray(246) => {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .unwrap_or(egui::Rect::NOTHING);
                let expected_logical = 360.0 * f32::from(zoom_percent) / 100.0;
                assert!((page.width() - expected_logical).abs() < 0.01);
                assert!(
                    (page.width() * context.pixels_per_point()
                        - expected_logical * effective_scale)
                        .abs()
                        < 0.1
                );
                assert!((page.height() / page.width() - 792.0 / 612.0).abs() < 0.001);
                assert!((context.pixels_per_point() - effective_scale).abs() < 0.001);
                assert_eq!(
                    context.zoom_factor(),
                    if native_scale.is_some() { 1.0 } else { 2.3 },
                    "page zoom must not alter global UI zoom"
                );
                assert!((app.page_scale.unwrap_or(0.0) - expected_logical / 612.0).abs() < 0.001);
                assert_eq!(app.session.revision(), 0);
                assert!(!app.session.dirty());
            }
        }
    }

    #[test]
    fn wo001_rendered_inner_clip_controls_centering_and_horizontal_overflow() {
        for (viewport_width, expected_padding, should_overflow) in [
            (800.0, 220.0, false),
            (392.0, 16.0, false),
            (390.0, 16.0, true),
        ] {
            let context = test_context();
            let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
            let screen =
                egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(viewport_width, 900.0));
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none())
                        .show(context, |ui| app.live_document_surface(ui));
                },
            );
            let page = app
                .last_scroll_page_rects
                .first()
                .copied()
                .unwrap_or(egui::Rect::NOTHING);
            let inner = app.last_scroll_inner_rect;
            assert!((inner.width() - viewport_width).abs() < 0.01);
            assert!((page.left() - inner.left() - expected_padding).abs() < 0.01);
            assert!((page.width() - CURRENT_DEFAULT_PAGE_WIDTH).abs() < 0.01);
            assert!(
                (inner.left() + app.last_scroll_content_size.x - page.right() - expected_padding)
                    .abs()
                    < 0.01
            );
            assert!((app.last_scroll_content_size.x - viewport_width.max(392.0)).abs() < 0.01);
            assert_eq!(app.last_scroll_horizontal_item_spacing, 0.0);
            assert_eq!(
                app.last_scroll_content_size.x > inner.width() + 0.01,
                should_overflow
            );
            assert!(
                app.last_scroll_offset.x
                    <= (app.last_scroll_content_size.x - inner.width()).max(0.0) + 0.01
            );
            assert!(app.last_scroll_outer_rect.contains_rect(inner));
            assert!(output.shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::epaint::Shape::Rect(rect) if rect.fill == Color32::from_gray(246)
            )));
        }
    }

    #[test]
    fn wo001_native_scale_changes_invalidate_status_geometry_until_republished() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        app.performance.frames = 1;
        let mut initial_input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        if let Some(viewport) = initial_input.viewports.get_mut(&initial_input.viewport_id) {
            viewport.native_pixels_per_point = Some(1.5);
        }
        eframe::App::raw_input_hook(&mut app, &context, &mut initial_input);
        let _ = render_editor_frame(&context, &mut app, initial_input);
        assert!(matches!(
            project_page_status(&mut app, 2, context.screen_rect()),
            status_bar::VisiblePage::Available { .. }
        ));

        let mut migrated_input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        if let Some(viewport) = migrated_input
            .viewports
            .get_mut(&migrated_input.viewport_id)
        {
            viewport.native_pixels_per_point = Some(2.0);
        }
        eframe::App::raw_input_hook(&mut app, &context, &mut migrated_input);
        assert_eq!(app.page_scale, None);
        assert_eq!(
            project_page_status(&mut app, 2, context.screen_rect()),
            status_bar::VisiblePage::Unavailable
        );

        app.performance.frames = 2;
        let _ = render_editor_frame(&context, &mut app, migrated_input);
        assert!(matches!(
            project_page_status(&mut app, 3, context.screen_rect()),
            status_bar::VisiblePage::Available { .. }
        ));
    }

    #[test]
    fn conditional_reveal_only_requests_outside_or_partial_carets() {
        let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        assert!(!caret_needs_reveal(
            egui::Rect::from_min_max(egui::pos2(20.0, 20.0), egui::pos2(21.0, 40.0)),
            clip
        ));
        assert!(caret_needs_reveal(
            egui::Rect::from_min_max(egui::pos2(20.0, -2.0), egui::pos2(21.0, 10.0)),
            clip
        ));
        assert!(caret_needs_reveal(
            egui::Rect::from_min_max(egui::pos2(20.0, 95.0), egui::pos2(21.0, 105.0)),
            clip
        ));
    }

    #[test]
    fn pointer_selection_clears_pending_reveal_intent() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.reveal_intent = RevealIntent::Conditional;
        app.apply_pointer_selection(
            Position {
                element: 0,
                offset: 0,
            },
            true,
            false,
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(app.reveal_intent, RevealIntent::None);
    }

    #[test]
    fn pointer_drag_preserves_initial_hit_as_selection_anchor() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("abcdef")
            .unwrap_or_else(|_| unreachable!());
        let start = Position {
            element: 0,
            offset: 1,
        };
        let end = Position {
            element: 0,
            offset: 5,
        };

        app.apply_pointer_selection(start, true, false)
            .unwrap_or_else(|_| unreachable!());
        app.apply_pointer_selection(end, false, true)
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(
            app.session.selection(),
            Selection {
                anchor: start,
                focus: end,
            }
        );
    }

    #[test]
    fn direct_adapter_operations_use_core_selection_and_role_transition() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.surface_focused = true;
        app.session
            .insert("INT. OFFICE")
            .unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::SceneHeading)
            .unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: "INT. OFFICE".len(),
            }))
            .unwrap_or_else(|_| unreachable!());
        app.session.enter().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[1].role, Role::Action);
        assert_eq!(app.session.selection().focus.element, 1);
    }

    #[test]
    fn enter_key_dispatch_splits_interior_dialogue_without_role_drift() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::Dialogue)
            .unwrap_or_else(|_| unreachable!());
        app.session
            .insert("hello🙂world")
            .unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: "hello".len(),
            }))
            .unwrap_or_else(|_| unreachable!());
        let result = app
            .key_event(Key::Enter, egui::Modifiers::default())
            .unwrap_or_else(|| unreachable!());
        app.mutate(result, "Enter", RevealIntent::Conditional);
        assert_eq!(app.session.project().elements[0].text, "hello");
        assert_eq!(app.session.project().elements[1].text, "🙂world");
        assert_eq!(app.session.project().elements[1].role, Role::Dialogue);
        assert_eq!(
            app.session.selection(),
            Selection::caret(Position {
                element: 1,
                offset: 0
            })
        );
    }

    #[test]
    fn vertical_navigation_uses_wrapped_lines_and_preserves_shift_anchor() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::Dialogue)
            .unwrap_or_else(|_| unreachable!());
        app.session
            .insert(&"abcdefghij ".repeat(5))
            .unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 12,
            }))
            .unwrap_or_else(|_| unreachable!());
        app.move_vertical(true, false)
            .unwrap_or_else(|_| unreachable!());
        assert!(app.session.selection().focus.offset > 12);
        app.move_vertical(false, true)
            .unwrap_or_else(|_| unreachable!());
        assert_ne!(
            app.session.selection().anchor.offset,
            app.session.selection().focus.offset
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vertical_navigation_clamps_single_line_document_edges() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session.insert("first and last").unwrap();

        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 7,
            }))
            .unwrap();
        app.move_vertical(false, false).unwrap();
        assert_eq!(
            app.session.selection(),
            Selection::caret(Position {
                element: 0,
                offset: 0,
            })
        );

        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 7,
            }))
            .unwrap();
        app.move_vertical(true, false).unwrap();
        assert_eq!(
            app.session.selection(),
            Selection::caret(Position {
                element: 0,
                offset: "first and last".len(),
            })
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vertical_navigation_clamps_wrapped_first_and_last_lines() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let text = "🙂abcdefghij".repeat(12);
        app.session.insert(&text).unwrap();
        let layout = ScreenplayLayout::build(app.session.project()).unwrap();
        let lines: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .collect();
        assert!(lines.len() > 1);

        let first = lines[0];
        let first_offset = first.source.start + first.text.chars().next().unwrap().len_utf8();
        app.session
            .set_selection(Selection::caret(Position {
                element: first.element,
                offset: first_offset,
            }))
            .unwrap();
        app.move_vertical(false, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, first.source.start);

        let last = lines.last().unwrap();
        let last_offset = last.source.start + last.text.chars().next().unwrap().len_utf8();
        app.session
            .set_selection(Selection::caret(Position {
                element: last.element,
                offset: last_offset,
            }))
            .unwrap();
        app.move_vertical(true, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, last.source.end);
        assert!(text.is_char_boundary(app.session.selection().focus.offset));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vertical_edge_clamps_reset_goal_repeat_safely_and_preserve_shift_anchor() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session.insert(&"a🙂b".repeat(24)).unwrap();
        let layout = ScreenplayLayout::build(app.session.project()).unwrap();
        let lines: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .collect();
        assert!(lines.len() > 1);

        let first = lines[0];
        let first_focus = Position {
            element: first.element,
            offset: first.source.start + first.text.chars().next().unwrap().len_utf8(),
        };
        app.session
            .set_selection(Selection::caret(first_focus))
            .unwrap();
        app.move_vertical(false, true).unwrap();
        let anchor = app.session.selection().anchor;
        assert_eq!(app.session.selection().focus.offset, first.source.start);
        app.move_vertical(false, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, first.source.start);
        assert_eq!(app.preferred_vertical_x, None);
        assert_eq!(anchor, first_focus);

        let last = lines.last().unwrap();
        app.session
            .set_selection(Selection::caret(Position {
                element: last.element,
                offset: last.source.start + last.text.chars().next().unwrap().len_utf8(),
            }))
            .unwrap();
        app.move_vertical(true, true).unwrap();
        assert_eq!(app.session.selection().focus.offset, last.source.end);
        app.move_vertical(true, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, last.source.end);
        assert_eq!(app.preferred_vertical_x, None);
    }

    #[test]
    fn tab_key_is_semantic_and_does_not_insert_literal_tab() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::Note)
            .unwrap_or_else(|_| unreachable!());
        let before = app.session.clone();
        let result = app.key_event(Key::Tab, egui::Modifiers::default());
        assert!(result.is_some());
        result
            .unwrap_or_else(|| unreachable!())
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "");
        assert_eq!(app.session.project().elements[0].role, Role::Note);
        assert_eq!(app.session.can_undo(), before.can_undo());
    }

    #[test]
    fn rendered_parenthetical_inner_end_routes_pointer_and_both_keys() {
        for key in [Key::Tab, Key::Enter] {
            let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
            app.session
                .insert("(words)")
                .unwrap_or_else(|_| unreachable!());
            app.session
                .change_role(Role::Parenthetical)
                .unwrap_or_else(|_| unreachable!());

            let context = test_context();
            let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
            let layout =
                ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
            let line = &layout.pages[0].lines[0];
            let inner_end_pointer = |surface_rect: egui::Rect| {
                let (_, horizontal_padding, scale) =
                    presentation_geometry(surface_rect.width(), layout.inputs.paper_width as f32);
                egui::pos2(
                    surface_rect.left()
                        + horizontal_padding
                        + (line.geometry.x.to_points_f32()
                            + ("(words".chars().count() as f32 * layout.char_width as f32))
                            * scale,
                    surface_rect.top() + (line.geometry.y.to_points_f32() + 1.0) * scale,
                )
            };
            let pointer_click = |pos| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            };

            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(screen_rect),
                    focused: true,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.live_document_surface(ui);
                    });
                },
            );
            let surface_rect = app.editor_surface_rect.unwrap_or_else(|| unreachable!());
            let pointer = inner_end_pointer(surface_rect);

            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(screen_rect),
                    focused: true,
                    events: vec![egui::Event::PointerMoved(pointer), pointer_click(pointer)],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.live_document_surface(ui);
                    });
                    app.handle_surface_input(ctx);
                },
            );

            assert_eq!(
                app.session.selection(),
                Selection::caret(Position {
                    element: 0,
                    offset: "(words".len(),
                })
            );
            let key_event = egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            };
            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(screen_rect),
                    focused: true,
                    events: vec![key_event],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.live_document_surface(ui);
                    });
                    app.handle_surface_input(ctx);
                },
            );

            assert_eq!(app.session.project().elements[0].text, "(WORDS)");
            assert_eq!(app.session.project().elements[1].role, Role::Dialogue);
            assert_eq!(
                app.session.selection().focus,
                Position {
                    element: 1,
                    offset: 0
                }
            );
        }
    }

    #[test]
    fn direct_surface_editing_supports_selection_replacement_deletion_and_backspace() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session.insert("abc").unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection {
                anchor: Position {
                    element: 0,
                    offset: 1,
                },
                focus: Position {
                    element: 0,
                    offset: 2,
                },
            })
            .unwrap_or_else(|_| unreachable!());
        app.session.insert("🙂").unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "A🙂C");
        app.session.delete().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "A🙂");
        app.session.backspace().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "A");
    }

    #[test]
    fn focused_event_dispatch_inserts_unicode_and_invalidates_render_state() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.surface_focused = true;
        app.rendered_revision = Some(app.session.revision());
        app.dispatch_surface_events(
            &[egui::Event::Text("café🙂".into())],
            egui::Modifiers::default(),
        );
        assert_eq!(app.session.project().elements[0].text, "CAFÉ🙂");
        assert_eq!(app.session.selection().focus.offset, "CAFÉ🙂".len());
        assert_eq!(app.rendered_revision, None);
        assert_eq!(app.native_input_diagnostics.received_events, 1);
        assert_eq!(app.native_input_diagnostics.last_event_category, "text");
        assert!(app.performance.event_nanos > 0);
    }

    #[test]
    fn unfocused_surface_does_not_consume_keyboard_events() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let context = test_context();
        let _ = context.run(
            egui::RawInput {
                focused: true,
                events: vec![egui::Event::Text("blocked".into())],
                ..Default::default()
            },
            |ctx| app.handle_surface_input(ctx),
        );
        assert_eq!(app.session.project().elements[0].text, "");
        assert_eq!(app.native_input_diagnostics.received_events, 0);
        assert!(!app.native_input_diagnostics.dispatch_enabled);
    }

    #[test]
    fn rendered_page_editor_retains_focus_for_later_frame_input() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("seed")
            .unwrap_or_else(|_| unreachable!());
        let context = test_context();
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let (_, horizontal_padding, scale) =
            presentation_geometry(800.0, layout.inputs.paper_width as f32);
        let line = &layout.pages[0].lines[0];
        let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let pointer_click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: Vec::new(),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                app.handle_surface_input(ctx);
            },
        );
        let editor_id = app.editor_surface_id.unwrap_or_else(|| unreachable!());
        assert!(!app.surface_focused);
        assert!(!context.memory(|memory| memory.has_focus(editor_id)));

        let surface_rect = app.editor_surface_rect.unwrap_or_else(|| unreachable!());
        let pointer = egui::pos2(
            surface_rect.left()
                + horizontal_padding
                + (line.geometry.x.to_points_f32() + 1.0) * scale,
            surface_rect.top() + (line.geometry.y.to_points_f32() + 1.0) * scale,
        );
        let blank_pointer = egui::pos2(surface_rect.right() - 4.0, surface_rect.top() + 4.0);
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::PointerMoved(blank_pointer),
                    pointer_click(blank_pointer, true),
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
            },
        );
        assert!(app.surface_focused);
        assert!(context.memory(|memory| memory.has_focus(editor_id)));
        assert_eq!(app.native_input_diagnostics.pointer_activations, 1);
        assert_eq!(app.session.selection().focus.offset, "seed".len());

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::PointerMoved(pointer),
                    pointer_click(pointer, true),
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                app.handle_surface_input(ctx);
            },
        );
        assert!(app.surface_focused);
        assert!(context.memory(|memory| memory.has_focus(editor_id)));
        assert!(app.native_input_diagnostics.focus_lock_installed);
        assert_eq!(app.native_input_diagnostics.pointer_activations, 2);

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::PointerMoved(pointer),
                    pointer_click(pointer, false),
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                assert!(app.surface_focused);
                assert!(context.memory(|memory| memory.has_focus(editor_id)));
                app.handle_surface_input(ctx);
            },
        );

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::Text("later".into()),
                    egui::Event::Key {
                        key: Key::Backspace,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                assert!(app.surface_focused);
                app.handle_surface_input(ctx);
            },
        );

        assert_eq!(app.session.project().elements[0].text.chars().count(), 8);
        assert_ne!(app.session.project().elements[0].text, "seed");
    }

    #[test]
    fn ime_preedit_is_transient_and_commit_is_inserted_once() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.dispatch_surface_events(
            &[
                egui::Event::Ime(egui::ImeEvent::Preedit("に".into())),
                egui::Event::Text("に".into()),
                egui::Event::Ime(egui::ImeEvent::Commit("日本".into())),
            ],
            egui::Modifiers::default(),
        );
        assert_eq!(app.session.project().elements[0].text, "日本");
        assert!(app.ime_preedit.is_empty());
        assert!(!app.ime_active);
    }

    #[test]
    fn editing_key_dispatch_covers_enter_delete_backspace_undo_redo_and_shift_selection() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.dispatch_surface_events(
            &[egui::Event::Text("abc".into())],
            egui::Modifiers::default(),
        );
        let plain = egui::Modifiers::default();
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    shift: true,
                    ..plain
                },
            }],
            plain,
        );
        assert_ne!(
            app.session.selection().anchor,
            app.session.selection().focus
        );
        app.dispatch_surface_events(
            &[
                egui::Event::Key {
                    key: Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: plain,
                },
                egui::Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: plain,
                },
            ],
            plain,
        );
        assert_eq!(app.session.project().elements.len(), 2);
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    command: true,
                    ..plain
                },
            }],
            plain,
        );
        assert_eq!(app.session.project().elements.len(), 1);
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::Y,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    command: true,
                    ..plain
                },
            }],
            plain,
        );
        assert_eq!(app.session.project().elements.len(), 2);
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: plain,
            }],
            plain,
        );
    }

    #[test]
    fn caret_selection_mapping_is_source_mapped() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let line = &layout.pages[0].lines[0];
        assert_eq!(
            layout.hit_test(0, line.rect.x, line.rect.y),
            Some(Caret {
                element: 0,
                offset: 0
            })
        );
        assert!(layout
            .caret_rect(Caret {
                element: 0,
                offset: 0
            })
            .is_ok());
    }

    #[test]
    fn fixed_metric_rendering_and_caret_bar_do_not_accumulate_drift() {
        let columns = 120;
        let scale = 360.0 / 612.0;
        let origin = 90.0 * scale;
        let expected = origin + columns as f32 * CHAR_WIDTH as f32 * scale;
        assert_eq!(
            fixed_metric_x(origin, columns, CHAR_WIDTH as f32 * scale),
            expected
        );

        let bar = caret_bar_rect(
            egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(360.0, 460.0)),
            scriptcraft_core::layout::FixedRect {
                x: FixedPoint::from_points(90 + (columns as i64) * i64::from(CHAR_WIDTH)),
                y: FixedPoint::from_points(72),
                width: FixedPoint::from_points(1),
                height: FixedPoint::from_points(i64::from(LINE_HEIGHT)),
            },
            scale,
        );
        assert_eq!(bar.width(), CARET_WIDTH);
        assert!((bar.left() - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn viewport_page_scale_only_transforms_fixed_print_geometry() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let line = &layout.pages[0].lines[0];
        let (_, _, small_scale) = presentation_geometry(400.0, 612.0);
        let (_, _, large_scale) = presentation_geometry(1_200.0, 612.0);

        assert_eq!(layout.pages.len(), 1);
        assert_eq!(line.rect.y, 72);
        assert_eq!(line.rect.height, LINE_HEIGHT);
        assert_eq!(line.rect.y as f32 * small_scale, 72.0 * small_scale);
        assert_eq!(line.rect.y as f32 * large_scale, 72.0 * large_scale);
        assert_eq!(small_scale, large_scale);
        assert_eq!(layout.pages[0].content_bounds.x, 108);
        assert_eq!(layout.pages[0].content_bounds.y, 72);
    }

    #[test]
    fn renderer_uses_embedded_profile_face_and_preserves_legacy_fallback() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let font = renderer_font(&layout.inputs.font_family, 6.0);
        assert_eq!(layout.inputs.font_family, "Courier Prime");
        assert_eq!(layout.inputs.font_size, 12);
        assert_eq!(font.size, 6.0);
        assert_eq!(
            font.family,
            FontFamily::Name(document_font::COURIER_PRIME_FAMILY.into())
        );
        assert_eq!(renderer_font("Courier", 6.0).family, FontFamily::Monospace);
        assert_eq!(
            renderer_font("Courier New", 6.0).family,
            FontFamily::Monospace
        );
        assert_eq!(
            renderer_font_for_style(true, false, 6.0).family,
            FontFamily::Name("Courier Prime Bold".into())
        );
        assert_eq!(
            renderer_font_for_style(false, true, 6.0).family,
            FontFamily::Name("Courier Prime Italic".into())
        );
        assert_eq!(
            renderer_font_for_style(true, true, 6.0).family,
            FontFamily::Name("Courier Prime Bold Italic".into())
        );
    }

    #[test]
    fn rendered_document_text_uses_bundled_face_without_changing_fixed_cells() {
        let context = test_context();
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("INT. ROOM - DAY")
            .unwrap_or_else(|_| unreachable!());
        let layout = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        let line = layout.pages[0].lines[0].clone();
        let output = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800.0, 600.0),
                )),
                ..Default::default()
            },
        );
        let scale = app.page_scale.unwrap_or(1.0);
        let mut glyphs = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.job.sections[0].format.font_id.family
                        == FontFamily::Name(document_font::COURIER_PRIME_FAMILY.into()) =>
                {
                    Some((text.pos.x, text.galley.text().to_owned()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        glyphs.sort_by(|left, right| left.0.total_cmp(&right.0));
        assert_eq!(glyphs.len(), line.display_text.chars().count());
        assert!(glyphs
            .iter()
            .all(|(_, character)| character.chars().count() == 1));
        for pair in glyphs.windows(2) {
            assert!(
                ((pair[1].0 - pair[0].0) - line.char_width.to_points_f32() * scale).abs() < 0.01
            );
        }
        assert_eq!(line.source.end - line.source.start, line.text.len());
    }

    #[test]
    fn rendered_role_typography_allcaps_expansion_and_hidden_editor_treatment() {
        let context = test_context();
        let imported = fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Scene Heading'><FontSpec Size='16' Style='Bold+Italic+AllCaps'/></ElementSettings><ElementSettings Type='Note'><FontSpec Style='Italic+HiddenText'/></ElementSettings><Content><Paragraph Type='Scene Heading' id='heading'><Text>Stra\xc3\x9fe</Text></Paragraph><Paragraph Type='Note' id='note'><Text>private</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .replace_with_saved(imported.project)
            .unwrap_or_else(|_| unreachable!());
        app.appearance_mode = status_bar::AppearanceMode::Day;
        app.page_palette = status_bar::PagePalette::for_theme(
            app.appearance_mode,
            egui::Theme::Light,
        );
        let output = render_editor_frame(
            &context,
            &mut app,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800.0, 600.0),
                )),
                ..Default::default()
            },
        );
        let glyph_shapes = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(shape) => {
                    let section = shape.galley.job.sections.first()?;
                    Some((
                        shape.pos.y,
                        shape.pos.x,
                        shape.galley.text().to_owned(),
                        section.format.font_id.clone(),
                        section.format.color,
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut heading_glyphs = glyph_shapes
            .iter()
            .filter(|(_, _, _, font, _)| {
                font.family == FontFamily::Name("Courier Prime Bold Italic".into())
            })
            .map(|(y, x, text, font, _)| (*y, *x, text.as_str(), font.size))
            .collect::<Vec<_>>();
        heading_glyphs.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
        });
        assert_eq!(
            heading_glyphs
                .iter()
                .map(|(_, _, text, _)| *text)
                .collect::<Vec<_>>()
                .join(""),
            "STRASSE"
        );
        assert!(heading_glyphs
            .iter()
            .all(|(_, _, _, size)| (*size - 16.0 * app.page_scale.unwrap_or(1.0)).abs() < 0.01));
        assert!(glyph_shapes.iter().any(|(_, _, text, font, color)| {
            font.family == FontFamily::Name("Courier Prime Italic".into())
                && *color == app.page_palette.subdued_text
                && text == "p"
        }));
    }

    #[test]
    fn rendered_caret_blink_alternates_on_controlled_egui_time() {
        let context = test_context();
        let id = egui::Id::new("wo002-caret");
        let frame = |time: f64| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, true, true, 0.0),
            );
            (visible, output)
        };

        assert!(frame(0.0).0);
        assert!(frame(0.499).0);
        assert!(!frame(0.5).0);
        assert!(!frame(0.999).0);
        assert!(frame(1.0).0);
    }

    #[test]
    fn rendered_caret_blink_schedules_only_focused_collapsed_phases() {
        let context = test_context();
        let id = egui::Id::new("wo002-repaint");
        let run = |time: f64, focused: bool, collapsed: bool| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, focused, collapsed, 0.0),
            );
            (visible, output)
        };

        let _ = run(2.0, true, true);
        let focused = run(2.1, true, true);
        assert!(focused.0);
        assert!(focused
            .1
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|output| output.repaint_delay)
            .is_some_and(|delay| delay <= CARET_BLINK_INTERVAL));
        assert_eq!(caret_blink_repaint_delay(2.1), Duration::from_millis(400));
        let unfocused = run(2.6, false, true);
        assert!(!unfocused.0);
        assert_eq!(caret_blink_repaint_delay(2.6), Duration::from_millis(400));
        assert!(!run(2.6, true, false).0);
    }

    #[test]
    fn rendered_caret_blink_identity_change_uses_a_distinct_entry() {
        let context = test_context();
        let first = egui::Id::new("wo002-first-caret");
        let second = egui::Id::new("wo002-second-caret");
        let run = |id: egui::Id, time: f64| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, true, true, 0.0),
            );
            (visible, output)
        };

        assert!(run(first, 0.0).0);
        assert!(!run(first, 0.5).0);
        assert!(!run(second, 0.5).0);
        assert!(run(second, 1.0).0);
    }

    #[test]
    fn rendered_caret_blink_focus_regain_starts_at_initial_visible_phase() {
        let context = test_context();
        let focused_id = egui::Id::new("wo002-focus-generation-0");
        let regained_id = egui::Id::new("wo002-focus-generation-1");
        let run = |id: egui::Id, time: f64, focused: bool, reset_time: f64| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, focused, true, reset_time),
            );
            (visible, output)
        };

        assert!(run(focused_id, 0.0, true, 0.0).0);
        assert!(!run(focused_id, 0.75, true, 0.0).0);

        let unfocused = run(focused_id, 0.75, false, 0.0);
        assert!(!unfocused.0);
        assert_eq!(
            unfocused
                .1
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map(|output| output.repaint_delay),
            Some(Duration::MAX)
        );

        // Focus loss changes the animation identity; regain establishes a new
        // origin so a stale hidden phase cannot leak across the boundary.
        assert!(run(regained_id, 0.75, true, 0.75).0);
        assert!(run(regained_id, 1.249, true, 0.75).0);
        assert!(!run(regained_id, 1.25, true, 0.75).0);
    }

    #[test]
    fn named_save_and_open_update_path_and_clean_state() {
        let root = std::env::temp_dir().join(format!("scriptcraft-wo010-{}", std::process::id()));
        let first = root.join("first.fdx");
        let second = root.join("second.fdx");
        let _ = std::fs::create_dir_all(&root);

        let mut source = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        source.save_fdx_as_at(&first);
        assert!(source.error.is_none());
        source
            .session
            .insert("named project")
            .unwrap_or_else(|_| unreachable!());
        source.save_native();
        assert_eq!(source.native_path, Some(first.clone()));
        assert!(!source.session.dirty());

        let mut replacement = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        replacement.save_fdx_as_at(&second);
        assert!(replacement.error.is_none());
        replacement
            .session
            .insert("second project")
            .unwrap_or_else(|_| unreachable!());
        replacement.save_native();

        source.open_native_at(&second);
        assert_eq!(source.native_path, Some(second));
        assert!(!source.session.dirty());
        assert_eq!(source.session.project().elements[0].text, "SECOND PROJECT");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_named_save_preserves_path_file_and_dirty_state() {
        let root =
            std::env::temp_dir().join(format!("scriptcraft-wo010-failure-{}", std::process::id()));
        let destination = root.join("project.fdx");
        let _ = std::fs::create_dir_all(&root);
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.save_fdx_as_at(&destination);
        assert!(app.error.is_none());
        app.session
            .insert("unsaved change")
            .unwrap_or_else(|_| unreachable!());
        let before = std::fs::read(&destination).unwrap_or_default();
        let failed_path = root.join("missing").join("project.fdx");

        app.save_native_at(&failed_path);

        assert_eq!(app.native_path, Some(destination));
        assert!(app.session.dirty());
        assert_eq!(
            std::fs::read(root.join("project.fdx")).unwrap_or_default(),
            before
        );
        assert!(app.error.is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_open_does_not_replace_named_project() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let original_path = PathBuf::from("known-project.fdx");
        let original_text = app.session.project().elements[0].text.clone();
        app.native_path = Some(original_path.clone());

        app.open_native_at(std::path::Path::new("does-not-exist.fdx"));

        assert_eq!(app.native_path, Some(original_path));
        assert_eq!(app.session.project().elements[0].text, original_text);
        assert!(!app.session.dirty());
        assert!(app.error.is_some());
    }

    #[test]
    fn legacy_json_open_is_rejected_without_mutating_live_session_or_source() {
        let root = std::env::temp_dir().join(format!(
            "scriptcraft-wo012b-policy-rejection-{}",
            std::process::id()
        ));
        let path = root.join("legacy.json");
        let project = legacy_json_project();
        let incompatible = project
            .to_json()
            .unwrap_or_else(|_| unreachable!())
            .replace(
                "\"auto_parenthetical\":true",
                "\"auto_parenthetical\":false",
            );
        let mut source = vec![0xEF, 0xBB, 0xBF, b' ', b'\n'];
        source.extend(incompatible.into_bytes());
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&path, &source);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session.set_policy(EditPolicy {
            uppercase_cues: false,
            ..app.session.policy()
        });
        let session_before = app.session.clone();
        let original_path = PathBuf::from("current-project.fdx");
        app.native_path = Some(original_path.clone());

        app.open_native_at(&path);

        assert_eq!(app.session, session_before);
        assert_eq!(app.native_path, Some(original_path));
        assert_eq!(std::fs::read(&path).unwrap_or_default(), source);
        assert!(app
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Legacy ScriptCraft JSON is no longer supported")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_json_save_is_retired_without_changing_file_or_live_session() {
        let root = std::env::temp_dir()
            .join(format!("scriptcraft-wo013-json-save-{}", std::process::id()));
        let path = root.join("existing.json");
        let original_bytes = b"existing user JSON";
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&path, original_bytes);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("pending edit")
            .unwrap_or_else(|_| unreachable!());
        let project_before = app.session.project().clone();
        let path_before = app.native_path.clone();

        app.save_native_at(&path);

        assert!(app.error.as_deref().is_some_and(|error| {
            error.contains("Legacy JSON saving is retired")
        }));
        assert!(app.session.dirty());
        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.native_path, path_before);
        assert_eq!(std::fs::read(&path).unwrap_or_default(), original_bytes);
        let _ = std::fs::remove_dir_all(root);
    }

    fn staged_fdx_fixture(version: &str) -> Vec<u8> {
        format!(
            "<FinalDraft DocumentType='Script' Version='{version}'><Content><Paragraph Type='Character' id='character-1'><Text>Casey</Text></Paragraph><Paragraph Type='Cast List' id='cast-1'><Text>(CUSTOM)</Text></Paragraph></Content></FinalDraft>"
        )
        .into_bytes()
    }

    #[test]
    fn staged_fdx_open_activates_version_six_only_after_complete_import_without_writing_source() {
        let root =
            std::env::temp_dir().join(format!("scriptcraft-wo011-v6-{}", std::process::id()));
        let path = root.join("fixture.fdx");
        let source_bytes = staged_fdx_fixture("6");
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&path, &source_bytes);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.open_staged_native_at(&path);

        assert!(app.error.is_none());
        assert_eq!(app.native_path.as_deref(), Some(path.as_path()));
        assert!(app.fdx_baseline.is_some());
        assert!(!app.session.dirty());
        assert_eq!(app.session.project().elements[0].text, "Casey");
        assert_eq!(app.session.project().elements[1].text, "(CUSTOM)");
        assert_eq!(std::fs::read(&path).unwrap_or_default(), source_bytes);
        app.save_native_at(&path);
        assert!(app.error.is_none());
        assert_eq!(app.native_path.as_deref(), Some(path.as_path()));
        assert!(app.fdx_baseline.is_some());
        assert!(!app.session.dirty());
        let saved_bytes = std::fs::read(&path).unwrap_or_default();
        assert_ne!(saved_bytes, source_bytes);
        assert!(matches!(
            fdx::preflight_version(&saved_bytes),
            Ok(FdxVersionStatus::Version6)
        ));
        let saved_project = fdx::read_v6(&saved_bytes)
            .unwrap_or_else(|_| unreachable!())
            .project;
        assert_eq!(
            saved_project.elements[1].text,
            app.session.project().elements[1].text
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn native_fdx_save_as_preserves_source_and_publishes_the_saved_candidate() {
        let root =
            std::env::temp_dir().join(format!("scriptcraft-wo012-save-as-{}", std::process::id()));
        let source_path = root.join("source.fdx");
        let destination = root.join("copy.fdx");
        let source_bytes = staged_fdx_fixture("6");
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&source_path, &source_bytes);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.open_native_at(&source_path);
        let opened_session = app.session.clone();
        app.save_fdx_as_at(&source_path);
        assert!(app.error.is_some());
        assert_eq!(app.session, opened_session);
        assert_eq!(std::fs::read(&source_path).unwrap_or_default(), source_bytes);

        app.session
            .insert("pending edit")
            .unwrap_or_else(|_| unreachable!());
        let selection_before_save = app.session.selection();
        let history_before_save = app.session.can_undo();
        app.save_native_at(&destination);

        assert!(app.error.is_none());
        assert_eq!(app.native_path.as_deref(), Some(destination.as_path()));
        assert_eq!(app.fdx_baseline.as_ref().map(FdxBaseline::path), Some(destination.as_path()));
        assert!(!app.session.dirty());
        assert_eq!(app.session.can_undo(), history_before_save);
        assert_eq!(app.session.selection(), selection_before_save);
        assert_eq!(std::fs::read(&source_path).unwrap_or_default(), source_bytes);
        let saved_bytes = std::fs::read(&destination).unwrap_or_default();
        let reopened = fdx::read_v6(&saved_bytes).unwrap_or_else(|_| unreachable!());
        assert_eq!(reopened.project.elements[0].text, app.session.project().elements[0].text);
        assert_eq!(reopened.project.elements[1].text, app.session.project().elements[1].text);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn native_fdx_save_conflict_keeps_external_destination_and_live_pending_edits() {
        let root = std::env::temp_dir()
            .join(format!("scriptcraft-wo012-conflict-{}", std::process::id()));
        let path = root.join("conflict.fdx");
        let original_bytes = staged_fdx_fixture("6");
        let external_bytes = staged_fdx_fixture("5");
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&path, &original_bytes);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.open_native_at(&path);
        app.session
            .insert("pending edit")
            .unwrap_or_else(|_| unreachable!());
        let project_before = app.session.project().clone();
        let selection_before = app.session.selection();
        let baseline_before = app.fdx_baseline.clone();
        let _ = std::fs::write(&path, &external_bytes);

        app.save_native();

        assert!(app.error.is_some());
        assert!(app.session.dirty());
        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.selection(), selection_before);
        assert_eq!(app.fdx_baseline, baseline_before);
        assert_eq!(app.native_path.as_deref(), Some(path.as_path()));
        assert_eq!(std::fs::read(&path).unwrap_or_default(), external_bytes);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn fdx_save_as_refuses_to_drop_legacy_json_origin_state() {
        let root = std::env::temp_dir()
            .join(format!("scriptcraft-wo012-json-origin-{}", std::process::id()));
        let source_path = root.join("legacy.json");
        let destination = root.join("migrated.fdx");
        let project = legacy_json_project();
        let source_bytes = project
            .to_json()
            .unwrap_or_default()
            .into_bytes();
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&source_path, &source_bytes);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        // Emulate a pre-sunset session still held by a running process.
        activate_legacy_json_project(&mut app);
        app.native_path = Some(source_path.clone());
        app.open_native_at(&source_path);
        let project_before = app.session.project().clone();
        let path_before = app.native_path.clone();
        app.save_native_at(&destination);

        assert!(app.error.is_some());
        assert!(!app.session.dirty());
        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.native_path, path_before);
        assert!(app.fdx_baseline.is_none());
        assert!(!destination.exists());
        assert_eq!(std::fs::read(&source_path).unwrap_or_default(), source_bytes);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn new_document_saves_as_a_native_version_six_fdx() {
        let root = std::env::temp_dir()
            .join(format!("scriptcraft-wo012-new-fdx-{}", std::process::id()));
        let path = root.join("new-document.fdx");
        let _ = std::fs::create_dir_all(&root);

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.save_fdx_as_at(&path);

        assert!(app.error.is_none());
        assert_eq!(app.native_path.as_deref(), Some(path.as_path()));
        assert_eq!(app.fdx_baseline.as_ref().map(FdxBaseline::path), Some(path.as_path()));
        assert!(!app.session.dirty());
        let bytes = std::fs::read(&path).unwrap_or_default();
        assert!(matches!(
            fdx::preflight_version(&bytes),
            Ok(FdxVersionStatus::Version6)
        ));
        assert!(fdx::read_v6(&bytes).is_ok());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn staged_non_six_open_requires_consent_and_cancel_or_continue_never_writes_input() {
        let root = std::env::temp_dir().join(format!(
            "scriptcraft-wo011-migration-{}",
            std::process::id()
        ));
        let path = root.join("older.fdx");
        let source_bytes = staged_fdx_fixture("5");
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&path, &source_bytes);

        let mut cancelled = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let original_project = cancelled.session.project().clone();
        cancelled.open_staged_native_at(&path);
        assert!(cancelled.pending_fdx_open.is_some());
        assert_eq!(cancelled.session.project(), &original_project);
        assert!(cancelled.native_path.is_none());
        cancelled.resolve_pending_fdx_import(FdxMigrationConsent::Cancel);
        assert!(cancelled.pending_fdx_open.is_none());
        assert!(cancelled.fdx_baseline.is_none());
        assert_eq!(cancelled.session.project(), &original_project);
        assert!(cancelled.native_path.is_none());
        assert_eq!(std::fs::read(&path).unwrap_or_default(), source_bytes);

        let mut continued = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        continued.open_staged_native_at(&path);
        continued.resolve_pending_fdx_import(FdxMigrationConsent::Continue);
        assert!(continued.error.is_none());
        assert!(continued.pending_fdx_open.is_none());
        assert_eq!(continued.native_path.as_deref(), Some(path.as_path()));
        assert!(continued.fdx_baseline.is_some());
        assert!(!continued.session.dirty());
        assert_eq!(continued.session.project().elements[0].text, "Casey");
        assert_eq!(continued.session.project().elements[1].text, "(CUSTOM)");
        assert!(continued.status.contains("previous-version"));
        assert_eq!(std::fs::read(&path).unwrap_or_default(), source_bytes);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn staged_newer_fdx_requires_consent_and_parse_or_dirty_failures_preserve_active_state() {
        let root =
            std::env::temp_dir().join(format!("scriptcraft-wo011-safety-{}", std::process::id()));
        let newer_path = root.join("newer.fdx");
        let malformed_path = root.join("malformed.fdx");
        let failed_import_path = root.join("failed-import.fdx");
        let dirty_path = root.join("dirty.fdx");
        let newer_bytes = staged_fdx_fixture("7");
        let malformed_bytes = b"<FinalDraft DocumentType='Script' Version='6'><Content>";
        let failed_import_bytes =
            b"<FinalDraft DocumentType='Script' Version='5'><TitlePage/></FinalDraft>";
        let dirty_bytes = staged_fdx_fixture("6");
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(&newer_path, &newer_bytes);
        let _ = std::fs::write(&malformed_path, malformed_bytes);
        let _ = std::fs::write(&failed_import_path, failed_import_bytes);
        let _ = std::fs::write(&dirty_path, &dirty_bytes);

        let mut newer = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let original_project = newer.session.project().clone();
        newer.open_staged_native_at(&newer_path);
        assert!(newer.pending_fdx_open.is_some());
        assert_eq!(newer.session.project(), &original_project);
        newer.resolve_pending_fdx_import(FdxMigrationConsent::Continue);
        assert!(newer.error.is_none());
        assert!(newer.status.contains("best-effort"));
        assert_eq!(newer.native_path.as_deref(), Some(newer_path.as_path()));
        assert!(newer.fdx_baseline.is_some());
        assert_eq!(std::fs::read(&newer_path).unwrap_or_default(), newer_bytes);

        let mut malformed = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let malformed_original = malformed.session.project().clone();
        let kept_path = PathBuf::from("kept-project.fdx");
        malformed.native_path = Some(kept_path.clone());
        malformed.open_staged_native_at(&malformed_path);
        assert!(malformed.error.is_some());
        assert_eq!(malformed.session.project(), &malformed_original);
        assert_eq!(malformed.native_path, Some(kept_path.clone()));
        assert!(malformed.fdx_baseline.is_none());
        assert_eq!(
            std::fs::read(&malformed_path).unwrap_or_default(),
            malformed_bytes
        );

        let mut failed_import = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let failed_import_original = failed_import.session.project().clone();
        failed_import.native_path = Some(kept_path.clone());
        failed_import.open_staged_native_at(&failed_import_path);
        assert!(failed_import.pending_fdx_open.is_some());
        failed_import.resolve_pending_fdx_import(FdxMigrationConsent::Continue);
        assert!(failed_import.error.is_some());
        assert!(failed_import.pending_fdx_open.is_none());
        assert_eq!(failed_import.session.project(), &failed_import_original);
        assert!(!failed_import.session.dirty());
        assert_eq!(failed_import.native_path, Some(kept_path));
        assert!(failed_import.fdx_baseline.is_none());
        assert_eq!(
            std::fs::read(&failed_import_path).unwrap_or_default(),
            failed_import_bytes
        );

        let mut dirty = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        dirty
            .session
            .insert("unsaved")
            .unwrap_or_else(|_| unreachable!());
        let dirty_original = dirty.session.project().clone();
        dirty.open_staged_native_at(&dirty_path);
        assert!(dirty.pending_fdx_open.is_none());
        assert_eq!(dirty.session.project(), &dirty_original);
        assert!(dirty.session.dirty());
        assert!(dirty.native_path.is_none());
        assert_eq!(std::fs::read(&dirty_path).unwrap_or_default(), dirty_bytes);
        let _ = std::fs::remove_dir_all(root);
    }
}
