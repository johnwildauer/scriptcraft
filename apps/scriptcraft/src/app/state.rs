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
    chrome::{
        dynamic_controls::{DynamicElementControls, DynamicIssueSummary},
        focus_mode, menu, theme,
    },
    commands::CachedAppCommandProjection,
    documents::{
        lifecycle::RootCloseLifecycle, open::PendingFdxOpen, replacement, replacement_protocol,
        replacement_transport,
    },
    presentation::{document_font, wheel_zoom},
    widgets::{elements::state::ElementWindowState, sprint_timer},
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
    pub(crate) dynamic_controls: Option<DynamicElementControls>,
    pub(crate) reported_dynamic_issue_summary: Option<DynamicIssueSummary>,
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
    pub(crate) element_windows: ElementWindowState,
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
    /// Present only together with the matching FDX storage baseline.
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
    /// Paired with `native_path`; absent for fresh template-backed sessions.
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
            None,
            false,
        )
    }

    pub(super) fn new_with_startup(
        menu_policy: menu::RuntimeMenuPolicy,
        menu_platform_choice: menu::MenuPlatformChoice,
        startup_document: Option<StartupDocumentInput>,
        startup_template: Option<scriptcraft_fdx::BuiltinTemplateId>,
        development_mode: bool,
    ) -> Result<Self, String> {
        let project = scriptcraft_fdx::instantiate_builtin_template(
            startup_template.unwrap_or(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID),
        )
        .map_err(|error| format!("startup built-in template failed: {error:?}"))?
        .project;
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
            dynamic_controls: None,
            reported_dynamic_issue_summary: None,
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
            element_windows: ElementWindowState::default(),
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
            // New startup is semantically clean but has no saved-target pair;
            // ordinary Save therefore routes through the FDX Save As picker.
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
            // A selected built-in already owns this child process's initial
            // document slot; later native Open events must not replace it.
            initial_document_slot_finalized: startup_template.is_some(),
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

#[cfg(test)]
impl ScriptCraftApp {
    /// Shared cold-start constructor for sibling-module staged-open regressions.
    pub(crate) fn new_for_template_lifecycle_tests() -> Result<Self, String> {
        Self::new()
    }
}

#[cfg(test)]
mod dynamic_controls_refresh_tests {
    use super::*;
    use crate::chrome::shortcuts::ShortcutPlatform;
    use scriptcraft_session::editing::{ElementDefinitionCandidate, ElementTypeRemapping};
    use std::sync::Arc;

    const DYNAMIC_CONTROLS_FDX: &[u8] = include_bytes!("../../tests/fixtures/dynamic_controls.fdx");

    fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|error| panic!("prepared regression input failed: {error:?}"))
    }

    fn imported_fixture() -> Project {
        valid(scriptcraft_fdx::read_v6(DYNAMIC_CONTROLS_FDX)).project
    }

    fn assert_choices_refresh(
        app: &mut ScriptCraftApp,
        previous: &Arc<[crate::chrome::element_choices::ElementChoice]>,
    ) -> Arc<[crate::chrome::element_choices::ElementChoice]> {
        let current = Arc::clone(&app.refresh_dynamic_controls().choices);
        assert!(!Arc::ptr_eq(previous, &current));
        current
    }

    fn labels(choices: &[crate::chrome::element_choices::ElementChoice]) -> Vec<&str> {
        choices
            .iter()
            .map(crate::chrome::element_choices::ElementChoice::label)
            .collect()
    }

    #[test]
    fn controls_refresh_after_same_revision_open_type_history_and_generic_remap() {
        let mut app = valid(ScriptCraftApp::new());
        app.session = valid(EditingSession::new(imported_fixture()));
        let first_choices = Arc::clone(&app.refresh_dynamic_controls().choices);
        assert_eq!(labels(&first_choices)[0], "Notations");
        if ShortcutPlatform::current() == ShortcutPlatform::Other {
            assert!(app.reported_dynamic_issue_summary.is_none());
        } else {
            assert!(app.reported_dynamic_issue_summary.is_some());
        }

        // Opening a distinct document at the same revision and mutation token
        // must not reuse the first document's controls or shortcut diagnostics.
        let predecessor_revision = app.session.revision();
        let predecessor_token = app.session.mutation_token();
        let mut replacement_xml = String::from_utf8_lossy(DYNAMIC_CONTROLS_FDX).into_owned();
        replacement_xml = replacement_xml.replace("Notations", "Reopened");
        for source in [
            "Shortcut=\"2\" WinShortcut=\"2\"",
            "Shortcut=\"Ctrl+Alt+Shift+3\" WinShortcut=\"Shift+3\"",
            "Shortcut=\"Ctrl+Alt+;\" WinShortcut=\"Alt+;\"",
            "Shortcut=\":\" WinShortcut=\":\"",
            "Shortcut=\"Shift+;\" WinShortcut=\"Shift+;\"",
            "Shortcut=\"Alt+K\" WinShortcut=\"Alt+K\"",
            "Shortcut=\"Alt+k\" WinShortcut=\"Alt+k\"",
            "Shortcut=\"Q\" WinShortcut=\"\"",
            "Shortcut=\"\" WinShortcut=\"Alt+D\"",
        ] {
            replacement_xml = replacement_xml.replace(source, "Shortcut=\"\" WinShortcut=\"\"");
        }
        let replacement_project =
            valid(scriptcraft_fdx::read_v6(replacement_xml.as_bytes())).project;
        let replacement_session = valid(EditingSession::new(replacement_project));
        assert_eq!(replacement_session.revision(), predecessor_revision);
        assert_eq!(replacement_session.mutation_token(), predecessor_token);
        app.session = replacement_session;
        let reopened_choices = assert_choices_refresh(&mut app, &first_choices);
        assert_eq!(labels(&reopened_choices)[0], "Reopened");
        assert!(!labels(&reopened_choices).contains(&"Notations"));
        let reopened_controls = app.refresh_dynamic_controls();
        assert!(reopened_controls.shortcuts.bindings.is_empty());
        assert!(reopened_controls.shortcuts.issues.is_empty());
        assert!(app.reported_dynamic_issue_summary.is_none());
        assert!(!app.pending_diagnostic_commands.iter().any(|command| {
            matches!(command, Command::UiStatusSet { message: Some(message) }
                if message.as_str().starts_with("ELEMENT_SHORTCUTS:"))
        }));

        let before_type_change = Arc::clone(&app.dynamic_controls.as_ref().unwrap().choices);
        valid(
            app.session
                .change_element_type(valid(scriptcraft_model::ElementTypeKey::new("Lyrics"))),
        );
        let after_type_change = assert_choices_refresh(&mut app, &before_type_change);
        assert_eq!(
            app.session.project().elements[0].element_type().as_str(),
            "Lyrics"
        );

        valid(app.session.undo());
        let after_undo = assert_choices_refresh(&mut app, &after_type_change);
        assert_eq!(
            app.session.project().elements[0].element_type().as_str(),
            "Reopened"
        );
        valid(app.session.redo());
        let after_redo = assert_choices_refresh(&mut app, &after_undo);
        assert_eq!(
            app.session.project().elements[0].element_type().as_str(),
            "Lyrics"
        );

        let mut definitions = app
            .session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let lyrics = valid(scriptcraft_model::ElementTypeKey::new("Lyrics"));
        let songs = valid(scriptcraft_model::ElementTypeKey::new("Songs"));
        let replacement_definition = definitions
            .iter_mut()
            .find(|definition| definition.key == lyrics)
            .unwrap_or_else(|| unreachable!("fixture contains the Lyrics definition"));
        replacement_definition.key = songs.clone();
        let mut candidate = ElementDefinitionCandidate::new(definitions);
        candidate
            .body_remappings
            .push(ElementTypeRemapping::new(lyrics, songs));
        valid(
            app.session
                .replace_element_definitions_and_remap_types(candidate),
        );
        let after_definition_replacement = assert_choices_refresh(&mut app, &after_redo);
        assert!(labels(&after_definition_replacement).contains(&"Songs"));
        assert!(!labels(&after_definition_replacement).contains(&"Lyrics"));
        valid(app.session.undo());
        let after_definition_undo = assert_choices_refresh(&mut app, &after_definition_replacement);
        assert!(labels(&after_definition_undo).contains(&"Lyrics"));
        valid(app.session.redo());
        let after_definition_redo = assert_choices_refresh(&mut app, &after_definition_undo);
        assert!(labels(&after_definition_redo).contains(&"Songs"));
    }

    #[test]
    fn accepted_fixture_text_binding_changes_type_once_without_inserting_its_text() {
        let platform = ShortcutPlatform::current();
        if platform == ShortcutPlatform::Other {
            return;
        }
        let mut app = valid(ScriptCraftApp::new());
        app.session = valid(EditingSession::new(imported_fixture()));
        app.surface_focused = true;
        app.active_editor_surface = EditorSurface::Body;
        let original_text = app.session.project().elements[0].text.clone();
        let original_revision = app.session.revision();
        let modifiers = eframe::egui::Modifiers {
            command: true,
            ctrl: platform == ShortcutPlatform::MacOS,
            alt: true,
            ..eframe::egui::Modifiers::default()
        };
        app.dispatch_surface_events_for_test(
            &[eframe::egui::Event::Text(";".to_owned())],
            modifiers,
            &eframe::egui::Context::default(),
            EditorSurface::Body,
        );

        assert_eq!(
            app.session.project().elements[0].element_type().as_str(),
            "Punctuation"
        );
        assert_eq!(app.session.project().elements[0].text, original_text);
        assert_eq!(app.session.revision(), original_revision + 1);
    }

    #[test]
    fn same_frame_ime_event_suppresses_fixture_text_binding() {
        if ShortcutPlatform::current() == ShortcutPlatform::Other {
            return;
        }
        let mut app = valid(ScriptCraftApp::new());
        app.session = valid(EditingSession::new(imported_fixture()));
        app.surface_focused = true;
        app.active_editor_surface = EditorSurface::Body;
        let original_text = app.session.project().elements[0].text.clone();
        let modifiers = eframe::egui::Modifiers {
            command: true,
            ..eframe::egui::Modifiers::default()
        };
        app.dispatch_surface_events_for_test(
            &[
                eframe::egui::Event::Text(";".to_owned()),
                eframe::egui::Event::Ime(eframe::egui::ImeEvent::Preedit("compose".to_owned())),
            ],
            modifiers,
            &eframe::egui::Context::default(),
            EditorSurface::Body,
        );

        assert_eq!(
            app.session.project().elements[0].element_type().as_str(),
            "Notations"
        );
        assert_eq!(app.session.project().elements[0].text, original_text);
        assert!(app.ime_active);
        assert_eq!(app.ime_preedit, "compose");
    }
}

#[cfg(test)]
mod default_template_startup_tests {
    use super::*;
    use std::sync::Arc;

    fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|error| panic!("prepared regression input failed: {error:?}"))
    }

    #[test]
    fn startup_uses_complete_default_screenplay_as_a_fresh_unsaved_session() {
        let app = valid(ScriptCraftApp::new());
        let expected = valid(scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        ));
        let project = app.session.project();

        assert_eq!(project.elements, expected.project.elements);
        assert_eq!(project.title_page, expected.project.title_page);
        assert_eq!(
            project.element_definitions(),
            expected.project.element_definitions()
        );
        assert!(!project.elements.is_empty());
        assert_eq!(project.title_page.len(), 54);
        assert!(!Arc::ptr_eq(
            project.layout_identity(),
            expected.project.layout_identity()
        ));
        assert_eq!(app.session.revision(), 0);
        assert!(!app.session.dirty());
        assert!(app.native_path.is_none());
        assert!(app.fdx_baseline.is_none());
        assert_eq!(app.native_title, UNTITLED_DOCUMENT_TITLE);
    }

    #[test]
    fn selected_builtin_startup_uses_complete_content_and_fresh_unsaved_state() {
        let app = valid(ScriptCraftApp::new_with_startup(
            menu::RuntimeMenuPolicy::for_current_target(),
            menu::MenuPlatformChoice::Auto,
            None,
            Some(scriptcraft_fdx::BuiltinTemplateId::Stageplay),
            false,
        ));
        let expected = valid(scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::BuiltinTemplateId::Stageplay,
        ));

        assert_eq!(app.session.project().elements, expected.project.elements);
        assert_eq!(
            app.session.project().title_page,
            expected.project.title_page
        );
        assert_eq!(
            app.session.project().element_definitions(),
            expected.project.element_definitions()
        );
        assert!(app.session.project().preservation.is_some());
        assert!(app.native_path.is_none());
        assert!(app.fdx_baseline.is_none());
        assert_eq!(app.native_title, UNTITLED_DOCUMENT_TITLE);
        assert!(app.initial_document_slot_finalized);
        assert!(app.startup_document.is_none());
        assert_eq!(app.session.revision(), 0);
        assert!(!app.session.dirty());
        assert!(app.element_windows == ElementWindowState::default());
        assert!(!Arc::ptr_eq(
            app.session.project().layout_identity(),
            expected.project.layout_identity()
        ));
    }

    #[test]
    fn fresh_unsaved_activation_clears_an_existing_path_and_baseline() {
        let mut app = valid(ScriptCraftApp::new());
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("dynamic_controls.fdx");
        let opened = valid(scriptcraft_session::storage::open_source(
            &scriptcraft_session::storage::StdFdxStorageFileSystem,
            &fixture,
        ));
        let previous_layout_identity = Arc::clone(app.session.project().layout_identity());
        app.native_path = Some(opened.baseline.path().to_owned());
        app.native_title = "dynamic_controls.fdx".to_owned();
        app.fdx_baseline = Some(opened.baseline);
        app.element_windows.open_elements(EditorSurface::TitlePage);
        app.element_windows
            .open_selector(crate::widgets::elements::state::TemplateSelectorMode::ApplyToCurrent);
        app.element_windows
            .select_template(scriptcraft_fdx::BuiltinTemplateId::Stageplay);

        let template = valid(scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        ));
        valid(app.activate_opened_project(
            template.project,
            crate::documents::open::ActivationPersistence::FreshUnsaved,
            None,
            None,
            scriptcraft_fdx::TextState::DEFAULT,
        ));
        app.sync_native_title(&eframe::egui::Context::default());

        assert!(app.native_path.is_none());
        assert!(app.fdx_baseline.is_none());
        assert_eq!(app.element_windows, ElementWindowState::default());
        assert_eq!(app.native_title, UNTITLED_DOCUMENT_TITLE);
        assert!(!Arc::ptr_eq(
            &previous_layout_identity,
            app.session.project().layout_identity()
        ));
        assert_eq!(app.session.revision(), 0);
        assert!(!app.session.dirty());
    }
}

impl ScriptCraftApp {
    pub(crate) fn timer_now(&self) -> Duration {
        self.timer_clock_origin.elapsed()
    }
}
