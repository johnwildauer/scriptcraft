//! Application and transient title-surface state plus construction.

use super::diagnostics::{NativeInputDiagnostics, PerformanceDiagnostics};
use super::responsiveness;
use super::startup::StartupDocumentInput;
use crate::chrome::status_bar::{ViewportCenterAnchor, ViewportPageTracker};
use crate::documents::save::UNTITLED_DOCUMENT_TITLE;
#[cfg(target_os = "macos")]
use crate::platform::macos::{
    macos_lifecycle, macos_menu, macos_open_documents, macos_print_dialog,
};
use crate::views::script::RevealIntent;
use crate::{
    chrome::{focus_mode, menu, theme},
    commands::CachedAppCommandProjection,
    documents::{
        lifecycle::RootCloseLifecycle, open::PendingFdxOpen, replacement, replacement_protocol,
        replacement_transport,
    },
    presentation::{document_font, wheel_zoom},
    widgets::sprint_timer,
};
use eframe::egui;
use scriptcraft_commands::{Command, DiagnosticMessage, EditorSurface, ScriptViewMode};
use scriptcraft_layout::{LayoutCache, ScreenplayLayout};
use scriptcraft_model::{Project, StableId};
use scriptcraft_session::editing::{EditPolicy, EditingSession, Position};
use scriptcraft_session::storage::FdxBaseline;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// Transient title-window presentation state. Semantic selection and all edits
/// remain in the shared EditingSession; only viewport-local interaction state is
/// separate from the body window.
pub(crate) struct TitlePageViewState {
    pub(crate) surface_focused: bool,
    pub(crate) drag_anchor: Option<Position>,
    pub(crate) ime_preedit: String,
    pub(crate) ime_active: bool,
    pub(crate) rendered_revision: Option<u64>,
    pub(crate) reveal_intent: RevealIntent,
    pub(crate) preferred_vertical_x: Option<i64>,
    pub(crate) page_geometry: ViewportPageTracker,
    pub(crate) pending_zoom_anchor: Option<ViewportCenterAnchor>,
    pub(crate) page_scale: Option<f32>,
    pub(crate) effective_pixels_per_point: Option<f32>,
    pub(crate) zoom_percent: u16,
    pub(crate) pinch_zoom_percent: f64,
    pub(crate) caret_blink_focus_generation: u64,
    pub(crate) caret_blink_reset_time: f64,
    pub(crate) wheel_zoom: wheel_zoom::WheelZoomAdapter,
}

impl Default for TitlePageViewState {
    fn default() -> Self {
        Self {
            surface_focused: false,
            drag_anchor: None,
            ime_preedit: String::new(),
            ime_active: false,
            rendered_revision: None,
            reveal_intent: RevealIntent::None,
            preferred_vertical_x: None,
            page_geometry: ViewportPageTracker::default(),
            pending_zoom_anchor: None,
            page_scale: None,
            effective_pixels_per_point: None,
            zoom_percent: 100,
            pinch_zoom_percent: 100.0,
            caret_blink_focus_generation: 0,
            caret_blink_reset_time: 0.0,
            wheel_zoom: wheel_zoom::WheelZoomAdapter::default(),
        }
    }
}

pub(crate) struct ScriptCraftApp {
    pub(crate) session: EditingSession,
    pub(crate) root_close_lifecycle: RootCloseLifecycle,
    pub(crate) document_font_store: document_font::DocumentFontStore,
    pub(crate) prepared_document_fonts: Option<document_font::PreparedDocumentFonts>,
    pub(crate) pending_commands: menu::PendingDesktopCommands,
    pub(crate) command_projection_cache: Option<CachedAppCommandProjection>,
    pub(crate) menu_policy: menu::RuntimeMenuPolicy,
    pub(crate) menu_platform_choice: menu::MenuPlatformChoice,
    pub(crate) development_mode: bool,
    #[cfg(target_os = "macos")]
    pub(crate) native_menu: Option<macos_menu::NativeMenuController>,
    #[cfg(target_os = "macos")]
    pub(crate) native_lifecycle: Option<macos_lifecycle::NativeLifecycleController>,
    #[cfg(target_os = "macos")]
    pub(crate) native_open_documents: Option<macos_open_documents::NativeOpenDocumentsController>,
    pub(crate) focus_mode: focus_mode::FocusModeController,
    pub(crate) sprint_timer: sprint_timer::SprintTimer,
    pub(crate) timer_popup_surface: EditorSurface,
    pub(crate) timer_body_host_geometry: Option<sprint_timer::geometry::SprintTimerHostGeometry>,
    pub(crate) timer_title_host_geometry: Option<sprint_timer::geometry::SprintTimerHostGeometry>,
    pub(crate) timer_clock_origin: Instant,
    pub(crate) next_cast_list_refresh: Instant,
    pub(crate) notification_sink: sprint_timer::notifications::NativeNotificationSink,
    pub(crate) notification_results:
        Receiver<(u64, sprint_timer::notifications::NotificationResult)>,
    // The bounded event channel is paired with the optional single pending
    // record; a monotonic ID sequence scopes each acknowledged child.
    pub(crate) replacement_open_event_sender: replacement_transport::ReplacementOpenEventSender,
    pub(crate) replacement_open_events: replacement_transport::ReplacementOpenEventReceiver,
    pub(crate) replacement_handoff_ids: replacement_protocol::HandoffIdSequence,
    pub(crate) pending_replacement: Option<replacement::PendingReplacement>,
    pub(crate) replacement_resume_surface: Option<EditorSurface>,
    pub(crate) staged_replacement_open: Option<PathBuf>,
    pub(crate) surface_focused: bool,
    pub(crate) active_editor_surface: EditorSurface,
    pub(crate) active_body_dual_dialogue_group: Option<StableId>,
    pub(crate) title_page_window_open: bool,
    pub(crate) title_page_focus_requested: bool,
    pub(crate) title_viewport_unsupported: bool,
    pub(crate) title_view_reset_pending: bool,
    pub(crate) title_view_state_swapped: bool,
    pub(crate) title_view: TitlePageViewState,
    pub(crate) wheel_zoom: wheel_zoom::WheelZoomAdapter,
    pub(crate) last_status_modifiers: egui::Modifiers,
    pub(crate) drag_anchor: Option<Position>,
    pub(crate) ime_preedit: String,
    pub(crate) ime_active: bool,
    pub(crate) rendered_revision: Option<u64>,
    pub(crate) diagnostic_message: Option<DiagnosticMessage>,
    pub(crate) pending_diagnostic_commands: VecDeque<Command>,
    pub(crate) flushing_diagnostic_commands: bool,
    pub(crate) native_path: Option<PathBuf>,
    pub(crate) native_title: String,
    #[cfg(target_os = "macos")]
    pub(crate) accepted_print_settings: Option<macos_print_dialog::AcceptedPrintSettings>,
    #[cfg(target_os = "windows")]
    pub(crate) windows_print_session: Option<crate::platform::windows::windows_print::PrintSession>,
    #[cfg(target_os = "windows")]
    pub(crate) windows_print_preparation:
        Option<crate::platform::windows::windows_print::PrintPreparation>,
    pub(crate) native_output_modal_active: bool,
    pub(crate) fdx_baseline: Option<FdxBaseline>,
    pub(crate) pending_window_restore: Option<scriptcraft_fdx::WindowState>,
    pub(crate) latest_window_state: Option<scriptcraft_fdx::WindowState>,
    pub(crate) pending_fdx_open: Option<PendingFdxOpen>,
    pub(crate) startup_document: Option<StartupDocumentInput>,
    pub(crate) initial_document_slot_finalized: bool,
    pub(crate) startup_document_consumed: bool,
    pub(crate) native_input_diagnostics: NativeInputDiagnostics,
    pub(crate) editor_surface_id: Option<egui::Id>,
    pub(crate) title_editor_surface_id: Option<egui::Id>,
    pub(crate) editor_surface_rect: Option<egui::Rect>,
    pub(crate) reveal_intent: RevealIntent,
    pub(crate) preferred_vertical_x: Option<i64>,
    pub(crate) page_geometry: ViewportPageTracker,
    pub(crate) pending_zoom_anchor: Option<ViewportCenterAnchor>,
    pub(crate) page_scale: Option<f32>,
    pub(crate) effective_pixels_per_point: Option<f32>,
    pub(crate) zoom_percent: u16,
    pub(crate) pinch_zoom_percent: f64,
    pub(crate) script_view_mode: ScriptViewMode,
    pub(crate) appearance_mode: theme::AppearanceMode,
    pub(crate) appearance_applications: theme::AppearanceApplications,
    pub(crate) theme_palette: theme::ApplicationPalette,
    pub(crate) caret_blink_focus_generation: u64,
    pub(crate) caret_blink_reset_time: f64,
    pub(crate) layout_cache: LayoutCache,
    pub(crate) title_layout_cache: LayoutCache,
    pub(crate) performance: PerformanceDiagnostics,
    pub(crate) responsiveness: responsiveness::ResponsivenessDiagnostics,
    pub(crate) last_layout_identity: Option<std::sync::Weak<ScreenplayLayout>>,
}

impl ScriptCraftApp {
    pub(super) fn new() -> Result<Self, String> {
        Self::new_with_startup(
            menu::RuntimeMenuPolicy::for_current_target(),
            menu::MenuPlatformChoice::Auto,
            None,
            false,
        )
    }

    pub(super) fn new_with_startup(
        menu_policy: menu::RuntimeMenuPolicy,
        menu_platform_choice: menu::MenuPlatformChoice,
        startup_document: Option<StartupDocumentInput>,
        development_mode: bool,
    ) -> Result<Self, String> {
        let project = Project::new().map_err(|error| format!("new project failed: {error:?}"))?;
        let session = EditingSession::with_policy(project, EditPolicy::default())
            .map_err(|error| format!("session failed: {error:?}"))?;
        let (notification_sink, notification_results) =
            sprint_timer::notifications::NativeNotificationSink::new();
        // The UI retains the receiver; replacement reader threads get sender
        // clones and request egui repaint only after a successful try_send.
        let (replacement_open_event_sender, replacement_open_events) =
            replacement_transport::event_channel();
        Ok(Self {
            session,
            root_close_lifecycle: RootCloseLifecycle::default(),
            document_font_store: document_font::DocumentFontStore::default(),
            prepared_document_fonts: None,
            pending_commands: menu::PendingDesktopCommands::default(),
            command_projection_cache: None,
            menu_policy,
            menu_platform_choice,
            development_mode,
            #[cfg(target_os = "macos")]
            native_menu: None,
            #[cfg(target_os = "macos")]
            native_lifecycle: None,
            #[cfg(target_os = "macos")]
            native_open_documents: None,
            focus_mode: focus_mode::FocusModeController::default(),
            sprint_timer: sprint_timer::SprintTimer::default(),
            timer_popup_surface: EditorSurface::Body,
            timer_body_host_geometry: None,
            timer_title_host_geometry: None,
            timer_clock_origin: Instant::now(),
            next_cast_list_refresh: Instant::now()
                + scriptcraft_model::cast_list::CAST_LIST_REFRESH_INTERVAL,
            notification_sink,
            notification_results,
            replacement_open_event_sender,
            replacement_open_events,
            replacement_handoff_ids: replacement_protocol::HandoffIdSequence::default(),
            pending_replacement: None,
            replacement_resume_surface: None,
            staged_replacement_open: None,
            surface_focused: false,
            active_editor_surface: EditorSurface::Body,
            active_body_dual_dialogue_group: None,
            title_page_window_open: false,
            title_page_focus_requested: false,
            title_viewport_unsupported: false,
            title_view_reset_pending: false,
            title_view_state_swapped: false,
            title_view: TitlePageViewState::default(),
            wheel_zoom: wheel_zoom::WheelZoomAdapter::default(),
            last_status_modifiers: egui::Modifiers::NONE,
            drag_anchor: None,
            ime_preedit: String::new(),
            ime_active: false,
            rendered_revision: None,
            diagnostic_message: None,
            pending_diagnostic_commands: VecDeque::new(),
            flushing_diagnostic_commands: false,
            native_path: None,
            native_title: UNTITLED_DOCUMENT_TITLE.to_owned(),
            #[cfg(target_os = "macos")]
            accepted_print_settings: None,
            #[cfg(target_os = "windows")]
            windows_print_session: None,
            #[cfg(target_os = "windows")]
            windows_print_preparation: None,
            native_output_modal_active: false,
            fdx_baseline: None,
            pending_window_restore: None,
            latest_window_state: None,
            pending_fdx_open: None,
            startup_document,
            initial_document_slot_finalized: false,
            startup_document_consumed: false,
            native_input_diagnostics: NativeInputDiagnostics::default(),
            editor_surface_id: None,
            title_editor_surface_id: None,
            editor_surface_rect: None,
            reveal_intent: RevealIntent::None,
            preferred_vertical_x: None,
            page_geometry: ViewportPageTracker::default(),
            pending_zoom_anchor: None,
            page_scale: None,
            effective_pixels_per_point: None,
            zoom_percent: 100,
            pinch_zoom_percent: 100.0,
            script_view_mode: ScriptViewMode::Page,
            appearance_mode: theme::AppearanceMode::System,
            appearance_applications: theme::AppearanceApplications::default(),
            theme_palette: theme::ApplicationPalette::from_visuals(
                theme::ResolvedAppearance::Day,
                &egui::Visuals::light(),
            ),
            caret_blink_focus_generation: 0,
            caret_blink_reset_time: 0.0,
            layout_cache: LayoutCache::default(),
            title_layout_cache: LayoutCache::default(),
            performance: PerformanceDiagnostics::default(),
            responsiveness: responsiveness::ResponsivenessDiagnostics::default(),
            last_layout_identity: None,
        })
    }
}

impl ScriptCraftApp {
    pub(crate) fn timer_now(&self) -> Duration {
        self.timer_clock_origin.elapsed()
    }
}
