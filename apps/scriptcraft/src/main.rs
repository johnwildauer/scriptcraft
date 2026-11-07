#![deny(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use eframe::egui::{self, Color32, FontId, RichText, Sense, TextStyle, TextWrapMode, Vec2, WidgetText};
use scriptcraft_core::cast_list::CAST_LIST_REFRESH_INTERVAL;
use scriptcraft_core::editing::{EditError, EditPolicy, EditingSession, Position, Selection};
use scriptcraft_core::exports;
use scriptcraft_core::fdx::{self, FdxImportOutcome, FdxMigrationConsent, FdxVersionStatus};
use scriptcraft_core::fdx_storage::{
    self, FdxBaseline, FdxSaveMode, FdxSaveTarget, OpenedFdxSource, StdFdxStorageFileSystem,
};
use scriptcraft_core::layout::{
    LayoutBuildDiagnostics, LayoutCache, LayoutPurpose, ScreenplayLayout,
};
use scriptcraft_core::persistence::{FileSystem, StdFileSystem};
use scriptcraft_core::{Project, Role};
use scriptcraft_commands::DiagnosticMessage;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

mod app_commands;
mod document_font;
mod focus_mode;
mod material_font;
#[cfg(target_os = "macos")]
mod macos_menu;
#[cfg(target_os = "macos")]
mod macos_sf_symbols;
mod menu;
mod menu_items;
mod runtime_icon;
mod script_view;
mod shortcuts;
mod sprint_timer;
mod status_bar;
mod theme;
mod toolbar;
mod toolbar_items;
use sprint_timer::NotificationResult;
use status_bar::{DiagnosticProjection, ViewportCenterAnchor, ViewportPageTracker};
use toolbar::toolbar_content_height;
use app_commands::CommandInvocation;
use scriptcraft_commands::{Command, CommandOrigin, DocumentPath, ImportResolution, ZoomFactor};

const UNTITLED_DOCUMENT_TITLE: &str = "Untitled Screenplay";
const UNTITLED_PDF_FILENAME: &str = "Untitled Screenplay.pdf";
const STARTUP_USAGE: &str = "usage: scriptcraft [--menu-platform=auto|macos|windows] [--document-path=<UTF-8 FDX path>]";

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartupConfig {
    menu_platform: menu::MenuPlatformChoice,
    document_path: Option<PathBuf>,
}

fn parse_startup_args(
    args: impl IntoIterator<Item = OsString>,
) -> Result<StartupConfig, String> {
    let mut seen_menu_platform = false;
    let mut seen_document_path = false;
    let mut menu_platform = menu::MenuPlatformChoice::Auto;
    let mut document_path = None;

    for argument in args {
        let Some(argument) = argument.to_str() else {
            return Err(format!("arguments must be valid UTF-8\n{STARTUP_USAGE}"));
        };
        if argument == "--menu-platform" {
            if seen_menu_platform {
                return Err(format!("duplicate --menu-platform option\n{STARTUP_USAGE}"));
            }
            seen_menu_platform = true;
            return Err(format!("--menu-platform requires an equals-form value\n{STARTUP_USAGE}"));
        } else if let Some(value) = argument.strip_prefix("--menu-platform=") {
            if seen_menu_platform {
                return Err(format!("duplicate --menu-platform option\n{STARTUP_USAGE}"));
            }
            seen_menu_platform = true;
            menu_platform = match value {
                "auto" => menu::MenuPlatformChoice::Auto,
                "macos" => menu::MenuPlatformChoice::MacOS,
                "windows" => menu::MenuPlatformChoice::Windows,
                _ => {
                    return Err(format!(
                        "invalid --menu-platform value; expected auto, macos, or windows\n{STARTUP_USAGE}"
                    ));
                }
            };
        } else if argument == "--document-path" {
            if seen_document_path {
                return Err(format!("duplicate --document-path option\n{STARTUP_USAGE}"));
            }
            seen_document_path = true;
            return Err(format!("--document-path requires an equals-form value\n{STARTUP_USAGE}"));
        } else if let Some(value) = argument.strip_prefix("--document-path=") {
            if seen_document_path {
                return Err(format!("duplicate --document-path option\n{STARTUP_USAGE}"));
            }
            seen_document_path = true;
            if value.is_empty() {
                return Err(format!("--document-path value must not be empty\n{STARTUP_USAGE}"));
            }
            document_path = Some(PathBuf::from(value));
        } else {
            let error = if argument.starts_with('-') {
                "unknown option"
            } else {
                "unexpected positional argument"
            };
            return Err(format!("{error}\n{STARTUP_USAGE}"));
        };
    }

    Ok(StartupConfig {
        menu_platform,
        document_path,
    })
}

fn menu_platform_argument(choice: menu::MenuPlatformChoice) -> OsString {
    let value = match choice {
        menu::MenuPlatformChoice::Auto => "auto",
        menu::MenuPlatformChoice::MacOS => "macos",
        menu::MenuPlatformChoice::Windows => "windows",
    };
    OsString::from(format!("--menu-platform={value}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildLaunchError {
    InvalidDocumentPath,
    ExecutableUnavailable,
    SpawnFailed,
}

fn child_arguments(
    menu_platform: menu::MenuPlatformChoice,
    document_path: Option<&Path>,
) -> Result<Vec<OsString>, ChildLaunchError> {
    let mut arguments = vec![menu_platform_argument(menu_platform)];
    if let Some(path) = document_path {
        let Some(path) = path.to_str().filter(|path| !path.is_empty()) else {
            return Err(ChildLaunchError::InvalidDocumentPath);
        };
        arguments.push(OsString::from(format!("--document-path={path}")));
    }
    Ok(arguments)
}

fn launch_child_with(
    executable: &Path,
    arguments: &[OsString],
    spawn: impl FnOnce(&Path, &[OsString]) -> std::io::Result<()>,
) -> Result<(), ChildLaunchError> {
    spawn(executable, arguments).map_err(|_| ChildLaunchError::SpawnFailed)
}

fn launch_child_for_app(
    app: &ScriptCraftApp,
    executable: &Path,
    document_path: Option<&Path>,
    spawn: impl FnOnce(&Path, &[OsString]) -> std::io::Result<()>,
) -> Result<(), ChildLaunchError> {
    let arguments = child_arguments(app.menu_platform_choice, document_path)?;
    launch_child_with(executable, &arguments, spawn)
}

fn dispatch_open_selection(path: Option<PathBuf>, dispatch: impl FnOnce(&Path)) {
    if let Some(path) = path {
        dispatch(&path);
    }
}

fn spawn_current_child(
    app: &ScriptCraftApp,
    document_path: Option<&Path>,
) -> Result<(), ChildLaunchError> {
    let executable =
        std::env::current_exe().map_err(|_| ChildLaunchError::ExecutableUnavailable)?;
    launch_child_for_app(app, &executable, document_path, |executable, arguments| {
        std::process::Command::new(executable)
            .args(arguments)
            .spawn()
            .map(|_child| ())
    })
}

fn show_native_lifecycle_error(title: &str, message: &str) {
    let _ = rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

fn show_child_startup_open_failure() {
    show_native_lifecycle_error(
        "Unable to Open Document",
        "The selected document could not be opened in this new instance. This instance remains untitled.",
    );
}

fn status_modifiers_changed(previous: egui::Modifiers, current: egui::Modifiers) -> bool {
    previous != current
}

#[cfg(test)]
mod menu_platform_cli_tests {
    use super::{
        parse_startup_args, resolve_startup_menu_policy, StartupConfig, STARTUP_USAGE,
    };
    use crate::menu::{
        MenuAcceleratorOwner, MenuHostPlatform, MenuPlatformChoice, RuntimeMenuPolicy,
    };
    use std::ffi::OsString;

    fn parse(arguments: &[&str]) -> Result<StartupConfig, String> {
        parse_startup_args(arguments.iter().map(|argument| OsString::from(*argument)))
    }

    #[test]
    fn menu_platform_option_defaults_to_auto_and_accepts_each_value() {
        assert_eq!(parse(&[]), Ok(StartupConfig { menu_platform: MenuPlatformChoice::Auto, document_path: None }));
        assert_eq!(
            parse(&["--menu-platform=auto"]),
            Ok(StartupConfig { menu_platform: MenuPlatformChoice::Auto, document_path: None })
        );
        assert_eq!(
            parse(&["--menu-platform=macos"]),
            Ok(StartupConfig { menu_platform: MenuPlatformChoice::MacOS, document_path: None })
        );
        assert_eq!(
            parse(&["--menu-platform=windows"]),
            Ok(StartupConfig { menu_platform: MenuPlatformChoice::Windows, document_path: None })
        );
    }

    #[test]
    fn document_path_is_optional_utf8_and_preserves_spaces_and_menu_selection() {
        assert_eq!(
            parse(&["--document-path=/drafts/Été draft.fdx", "--menu-platform=windows"]),
            Ok(StartupConfig {
                menu_platform: MenuPlatformChoice::Windows,
                document_path: Some(std::path::PathBuf::from("/drafts/Été draft.fdx")),
            })
        );
    }

    #[test]
    fn malformed_duplicate_unknown_and_positional_arguments_fail_with_usage() {
        for (arguments, expected) in [
            (
                &["--menu-platform"][..],
                "--menu-platform requires an equals-form value",
            ),
            (
                &["--menu-platform="][..],
                "invalid --menu-platform value",
            ),
            (
                &["--menu-platform=Windows"][..],
                "invalid --menu-platform value",
            ),
            (
                &["--menu-platform=sideways"][..],
                "invalid --menu-platform value",
            ),
            (
                &["--menu-platform=windows", "--menu-platform=macos"][..],
                "duplicate --menu-platform option",
            ),
            (
                &["--menu-platform=windows", "--menu-platform"][..],
                "duplicate --menu-platform option",
            ),
            (&["--document-path="][..], "--document-path value must not be empty"),
            (
                &["--document-path=first.fdx", "--document-path=second.fdx"][..],
                "duplicate --document-path option",
            ),
            (
                &["--document-path=one.fdx", "--document-path"][..],
                "duplicate --document-path option",
            ),
            (
                &["--document-path"][..],
                "--document-path requires an equals-form value",
            ),
            (&["--unknown"][..], "unknown option"),
            (&["draft.fdx"][..], "unexpected positional argument"),
        ] {
            let error = parse(arguments);
            assert!(error.is_err());
            if let Err(error) = error {
                assert!(error.starts_with(expected));
                assert!(error.ends_with(STARTUP_USAGE));
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_arguments_are_rejected_without_lossy_parsing() {
        use std::os::unix::ffi::OsStringExt;

        let error = parse_startup_args([OsString::from_vec(vec![0xff])]);
        assert!(error.is_err());
        if let Err(error) = error {
            assert!(error.starts_with("arguments must be valid UTF-8"));
            assert!(error.ends_with(STARTUP_USAGE));
        }
    }

    #[test]
    fn startup_choice_resolves_before_launch_and_preserves_single_backend_owner() {
        for (host, requested, expected_owner) in [
            (
                MenuHostPlatform::MacOS,
                MenuPlatformChoice::Auto,
                MenuAcceleratorOwner::NativeMacOS,
            ),
            (
                MenuHostPlatform::MacOS,
                MenuPlatformChoice::Windows,
                MenuAcceleratorOwner::InWindow,
            ),
            (
                MenuHostPlatform::Windows,
                MenuPlatformChoice::Auto,
                MenuAcceleratorOwner::InWindow,
            ),
            (
                MenuHostPlatform::Other,
                MenuPlatformChoice::Auto,
                MenuAcceleratorOwner::None,
            ),
            (
                MenuHostPlatform::Other,
                MenuPlatformChoice::Windows,
                MenuAcceleratorOwner::InWindow,
            ),
        ] {
            let policy = resolve_startup_menu_policy(host, requested);
            assert!(policy.is_ok());
            if let Ok(policy) = policy {
                assert_eq!(policy.accelerator_owner(), expected_owner);
                assert!(!(policy.installs_native_macos() && policy.renders_in_window()));
            }
        }

        for host in [MenuHostPlatform::Windows, MenuHostPlatform::Other] {
            let error = resolve_startup_menu_policy(host, MenuPlatformChoice::MacOS);
            assert!(error.is_err());
            if let Err(error) = error {
                assert!(error.contains("--menu-platform=macos is unsupported"));
                assert!(error.ends_with(STARTUP_USAGE));
            }
        }

        assert_eq!(
            RuntimeMenuPolicy::resolve(MenuHostPlatform::MacOS, MenuPlatformChoice::Windows)
                .map(RuntimeMenuPolicy::accelerator_owner),
            Ok(MenuAcceleratorOwner::InWindow)
        );
    }
}

#[cfg(test)]
mod process_lifecycle_tests {
    use super::{
        child_arguments, dispatch_open_selection, launch_child_for_app, launch_child_with,
    };
    use crate::menu::MenuPlatformChoice;
    use crate::{fdx, ScriptCraftApp, UNTITLED_DOCUMENT_TITLE};
    use scriptcraft_commands::{execute_editor_command, Command, CommandOutcome};
    use std::cell::Cell;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    #[test]
    fn child_arguments_forward_menu_selection_and_one_utf8_path_argument() {
        let arguments = child_arguments(
            MenuPlatformChoice::Windows,
            Some(Path::new("/drafts/Été draft.fdx")),
        );
        assert_eq!(
            arguments,
            Ok(vec![
                OsString::from("--menu-platform=windows"),
                OsString::from("--document-path=/drafts/Été draft.fdx"),
            ])
        );
        assert_eq!(
            child_arguments(MenuPlatformChoice::Auto, None),
            Ok(vec![OsString::from("--menu-platform=auto")])
        );
    }

    #[cfg(unix)]
    #[test]
    fn child_arguments_reject_non_utf8_document_paths() {
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(std::ffi::OsStr::from_bytes(b"/drafts/\xff.fdx"));
        assert!(child_arguments(MenuPlatformChoice::Auto, Some(path)).is_err());
    }

    #[test]
    fn process_adapter_forwards_arguments_without_shell_reinterpretation_and_reports_failure() {
        let arguments = vec![
            OsString::from("--menu-platform=windows"),
            OsString::from("--document-path=/drafts/one two.fdx"),
        ];
        let executable = Path::new("/current/scriptcraft");
        let mut forwarded = None;
        let result = launch_child_with(executable, &arguments, |received_executable, received| {
            forwarded = Some((received_executable.to_owned(), received.to_vec()));
            Ok(())
        });
        assert_eq!(result, Ok(()));
        assert_eq!(forwarded, Some((executable.to_owned(), arguments.clone())));

        let failed = launch_child_with(executable, &arguments, |_executable, _arguments| {
            Err(std::io::Error::other("test-only spawn failure"))
        });
        assert!(failed.is_err());
    }

    #[test]
    fn cancelled_open_picker_dispatches_nothing() {
        let dispatch_count = Cell::new(0);
        dispatch_open_selection(None, |_path| dispatch_count.set(dispatch_count.get() + 1));
        assert_eq!(dispatch_count.get(), 0);

        dispatch_open_selection(Some(PathBuf::from("selected.fdx")), |_path| {
            dispatch_count.set(dispatch_count.get() + 1)
        });
        assert_eq!(dispatch_count.get(), 1);
    }

    #[test]
    fn child_startup_uses_staged_open_once_without_recursive_launch() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else {
            return;
        };
        app.startup_document_path = Some(PathBuf::from("missing-child-startup.fdx"));

        assert_eq!(app.consume_startup_document_handoff(), Some(false));
        assert_eq!(app.consume_startup_document_handoff(), None);
        assert!(app.native_path.is_none());
        assert_eq!(app.native_title, UNTITLED_DOCUMENT_TITLE);
    }

    #[test]
    fn separate_instance_launch_preserves_clean_and_dirty_origin_state() {
        for make_dirty in [false, true] {
            let app_result = ScriptCraftApp::new();
            assert!(app_result.is_ok());
            let Ok(mut app) = app_result else {
                continue;
            };
            app.native_path = Some(PathBuf::from("origin.fdx"));
            app.native_title = "origin.fdx".to_owned();
            app.zoom_percent = 137;
            if make_dirty {
                let edit = execute_editor_command(
                    &mut app.session,
                    &Command::EditorTextInsert {
                        text: "origin edit".to_owned(),
                    },
                );
                assert_eq!(edit, CommandOutcome::Changed);
            }

            let content_before = fdx::write_v6(app.session.project()).ok();
            assert!(content_before.is_some());
            let dirty_before = app.session.dirty();
            let selection_before = app.session.selection();
            let history_before = (app.session.can_undo(), app.session.can_redo());
            let path_before = app.native_path.clone();
            let baseline_before = app.fdx_baseline.clone();
            let title_before = app.native_title.clone();
            let zoom_before = app.zoom_percent;
            let migration_before = app.pending_fdx_open.is_some();
            let mut forwarded = None;
            let launch = launch_child_for_app(
                &app,
                Path::new("/current/scriptcraft"),
                Some(Path::new("/picked/Selected.fdx")),
                |executable, arguments| {
                    forwarded = Some((executable.to_owned(), arguments.to_vec()));
                    Ok(())
                },
            );
            assert_eq!(launch, Ok(()));
            assert_eq!(
                forwarded,
                Some((
                    PathBuf::from("/current/scriptcraft"),
                    vec![
                        OsString::from("--menu-platform=auto"),
                        OsString::from("--document-path=/picked/Selected.fdx"),
                    ],
                ))
            );
            let mut blank_forwarded = None;
            let blank_launch = launch_child_for_app(
                &app,
                Path::new("/current/scriptcraft"),
                None,
                |executable, arguments| {
                    blank_forwarded = Some((executable.to_owned(), arguments.to_vec()));
                    Ok(())
                },
            );
            assert_eq!(blank_launch, Ok(()));
            assert_eq!(
                blank_forwarded,
                Some((
                    PathBuf::from("/current/scriptcraft"),
                    vec![OsString::from("--menu-platform=auto")],
                ))
            );
            let failed_launch = launch_child_for_app(
                &app,
                Path::new("/current/scriptcraft"),
                None,
                |_executable, _arguments| {
                    Err(std::io::Error::other("test-only spawn failure"))
                },
            );
            assert!(failed_launch.is_err());
            assert_eq!(fdx::write_v6(app.session.project()).ok(), content_before);
            assert_eq!(app.session.dirty(), dirty_before);
            assert_eq!(app.session.selection(), selection_before);
            assert_eq!((app.session.can_undo(), app.session.can_redo()), history_before);
            assert_eq!(app.native_path, path_before);
            assert_eq!(app.fdx_baseline, baseline_before);
            assert_eq!(app.native_title, title_before);
            assert_eq!(app.zoom_percent, zoom_before);
            assert_eq!(app.pending_fdx_open.is_some(), migration_before);
            assert_eq!(dirty_before, make_dirty);
        }
    }
}

fn native_document_title(path: Option<&Path>) -> String {
    path.and_then(Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .filter(|file_name| !file_name.is_empty())
        .unwrap_or(UNTITLED_DOCUMENT_TITLE)
        .to_owned()
}

fn changed_native_document_title(current: &str, path: Option<&Path>) -> Option<String> {
    let next = native_document_title(path);
    (next != current).then_some(next)
}

fn has_extension(path: &std::path::Path, expected: &str) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn pdf_export_filename(active_path: Option<&Path>) -> String {
    let Some(file_name) = active_path
        .and_then(Path::file_name)
        .filter(|file_name| !file_name.is_empty())
        .and_then(std::ffi::OsStr::to_str)
    else {
        return UNTITLED_PDF_FILENAME.to_owned();
    };

    let mut projected = PathBuf::from(file_name);
    if !projected.set_extension("pdf") {
        return UNTITLED_PDF_FILENAME.to_owned();
    }

    projected
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|file_name| !file_name.is_empty())
        .unwrap_or(UNTITLED_PDF_FILENAME)
        .to_owned()
}

#[derive(Debug)]
enum PdfExportAdapterError {
    Export(exports::ExportError),
    Write(scriptcraft_core::persistence::PersistenceError),
}

fn export_pdf_to_selected_path<F: FileSystem>(
    selected_path: Option<&Path>,
    filesystem: &F,
    build_pdf: impl FnOnce() -> Result<Vec<u8>, exports::ExportError>,
) -> Result<bool, PdfExportAdapterError> {
    let Some(path) = selected_path else {
        return Ok(false);
    };
    let bytes = build_pdf().map_err(PdfExportAdapterError::Export)?;
    filesystem
        .write(path, &bytes)
        .map_err(PdfExportAdapterError::Write)?;
    Ok(true)
}

pub(crate) fn safe_layout_error_category(
    error: &scriptcraft_core::layout::LayoutError,
) -> &'static str {
    use scriptcraft_core::layout::LayoutErrorKind;
    match &error.kind {
        LayoutErrorKind::Model(_) => "model validation",
        LayoutErrorKind::InvalidProfile => "invalid layout profile",
        LayoutErrorKind::InvalidPosition => "invalid layout position",
        LayoutErrorKind::NoGeometry => "layout geometry unavailable",
    }
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
    pending_commands: menu::PendingDesktopCommands,
    menu_policy: menu::RuntimeMenuPolicy,
    menu_platform_choice: menu::MenuPlatformChoice,
    #[cfg(target_os = "macos")]
    native_menu: Option<macos_menu::NativeMenuController>,
    focus_mode: focus_mode::FocusModeController,
    sprint_timer: sprint_timer::SprintTimer,
    timer_clock_origin: Instant,
    next_cast_list_refresh: Instant,
    notification_sink: sprint_timer::NativeNotificationSink,
    notification_results: Receiver<(u64, NotificationResult)>,
    surface_focused: bool,
    last_status_modifiers: egui::Modifiers,
    drag_anchor: Option<Position>,
    ime_preedit: String,
    ime_active: bool,
    rendered_revision: Option<u64>,
    diagnostic_message: Option<DiagnosticMessage>,
    pending_diagnostic_commands: VecDeque<Command>,
    flushing_diagnostic_commands: bool,
    native_path: Option<PathBuf>,
    native_title: String,
    fdx_baseline: Option<FdxBaseline>,
    pending_window_restore: Option<fdx::WindowState>,
    latest_window_state: Option<fdx::WindowState>,
    pending_fdx_open: Option<PendingFdxOpen>,
    startup_document_path: Option<PathBuf>,
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
    appearance_mode: theme::AppearanceMode,
    applied_appearance: theme::AppliedAppearance,
    theme_palette: theme::ApplicationPalette,
    caret_blink_focus_generation: u64,
    caret_blink_reset_time: f64,
    layout_cache: LayoutCache,
    performance: PerformanceDiagnostics,
    last_layout_identity: Option<std::sync::Weak<ScreenplayLayout>>,
}

fn scheduled_refresh_failure_status(category: &'static str) -> String {
    format!(
        "CAST_LIST_REFRESH_FAILED: retry suspended until relevant document changes; category={category}; screenplay unchanged"
    )
}

impl ScriptCraftApp {
    fn new() -> Result<Self, String> {
        Self::new_with_startup(
            menu::RuntimeMenuPolicy::for_current_target(),
            menu::MenuPlatformChoice::Auto,
            None,
        )
    }

    fn new_with_startup(
        menu_policy: menu::RuntimeMenuPolicy,
        menu_platform_choice: menu::MenuPlatformChoice,
        startup_document_path: Option<PathBuf>,
    ) -> Result<Self, String> {
        let project = Project::new().map_err(|error| format!("new project failed: {error:?}"))?;
        let session = EditingSession::with_policy(project, EditPolicy::default())
            .map_err(|error| format!("session failed: {error:?}"))?;
        let (notification_sink, notification_results) = sprint_timer::NativeNotificationSink::new();
        Ok(Self {
            session,
            pending_commands: menu::PendingDesktopCommands::default(),
            menu_policy,
            menu_platform_choice,
            #[cfg(target_os = "macos")]
            native_menu: None,
            focus_mode: focus_mode::FocusModeController::default(),
            sprint_timer: sprint_timer::SprintTimer::default(),
            timer_clock_origin: Instant::now(),
            next_cast_list_refresh: Instant::now() + CAST_LIST_REFRESH_INTERVAL,
            notification_sink,
            notification_results,
            surface_focused: false,
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
            fdx_baseline: None,
            pending_window_restore: None,
            latest_window_state: None,
            pending_fdx_open: None,
            startup_document_path,
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
            appearance_mode: theme::AppearanceMode::System,
            applied_appearance: None,
            theme_palette: theme::ApplicationPalette::from_visuals(
                theme::ResolvedAppearance::Day,
                &egui::Visuals::light(),
            ),
            caret_blink_focus_generation: 0,
            caret_blink_reset_time: 0.0,
            layout_cache: LayoutCache::default(),
            performance: PerformanceDiagnostics::default(),
            last_layout_identity: None,
        })
    }

    fn launch_child_instance(&self, document_path: Option<&Path>) {
        if spawn_current_child(self, document_path).is_err() {
            show_native_lifecycle_error(
                if document_path.is_some() {
                    "Unable to Open Document"
                } else {
                    "Unable to Start ScriptCraft"
                },
                if document_path.is_some() {
                    "A separate instance for the selected document could not be started. The current document is unchanged."
                } else {
                    "A separate blank ScriptCraft instance could not be started. The current document is unchanged."
                },
            );
        }
    }

    fn launch_blank_instance(&self) {
        self.launch_child_instance(None);
    }

    fn launch_document_instance(&self, path: &Path) {
        self.launch_child_instance(Some(path));
    }

    fn consume_startup_document_handoff(&mut self) -> Option<bool> {
        let path = self.startup_document_path.take()?;
        Some(self.open_staged_native_at(&path))
    }

    fn request_repaint(&mut self, ctx: &egui::Context) {
        self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
        ctx.request_repaint();
    }

    fn timer_now(&self) -> Duration {
        self.timer_clock_origin.elapsed()
    }

    fn reconcile_sprint_timer(&mut self, now: Duration) {
        let notification = self.sprint_timer.reconcile(now);
        self.request_timer_notification(notification);
        while let Ok((generation, result)) = self.notification_results.try_recv() {
            self.apply_timer_notification_result(generation, result);
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

    fn save_native(&mut self, ctx: &egui::Context) {
        let Some(path) = self.native_path.clone() else {
            self.dispatch_command(
                CommandInvocation::new(
                    Command::FileSaveAsRequest,
                    CommandOrigin::InternalFollowUp,
                ),
                ctx,
            );
            return;
        };
        self.save_native_at(&path);
    }

    fn save_native_at(&mut self, path: &std::path::Path) {
        if has_extension(path, "json") {
            self.show_error(
                "LEGACY_JSON_SAVE_REJECTED: destination and current document unchanged",
            );
            return;
        }

        let mode = match self.fdx_baseline.as_ref() {
            Some(baseline) if baseline.path() == path => FdxSaveMode::Save,
            _ => FdxSaveMode::SaveAs(path.to_owned()),
        };
        self.save_fdx_mode_at(path, mode);
    }

    fn save_fdx_as_at(&mut self, path: &std::path::Path, overwrite: bool) {
        if !has_extension(path, "fdx") {
            self.show_error("SAVE_AS_EXTENSION_REJECTED: .fdx required; destination and document unchanged");
            return;
        }
        let mode = if overwrite {
            FdxSaveMode::SaveAsOverwrite(path.to_owned())
        } else {
            FdxSaveMode::SaveAs(path.to_owned())
        };
        self.save_fdx_mode_at(path, mode);
    }

    fn save_fdx_mode_at(&mut self, path: &std::path::Path, mode: FdxSaveMode) {
        if matches!(
            &mode,
            FdxSaveMode::SaveAs(_) | FdxSaveMode::SaveAsOverwrite(_)
        ) && !has_extension(path, "fdx")
        {
            self.show_error("SAVE_AS_EXTENSION_REJECTED: .fdx required; destination and document unchanged");
            return;
        }
        if self.fdx_baseline.is_none()
            && self
                .native_path
                .as_deref()
                .is_some_and(|source_path| has_extension(source_path, "json"))
        {
            self.show_error(
                "FDX_SAVE_REFUSED_LEGACY_SOURCE: no FDX mapping; source, destination, and live document unchanged",
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
                if outcome.temporary_cleanup_warning {
                    self.show_error(
                        "FDX_SAVE_CLEANUP_WARNING: save completed, but temporary-file cleanup failed; the document is saved",
                    );
                } else {
                    self.clear_error();
                }
            }
            Err(error) => self.show_fdx_save_error(error),
        }
    }

    fn show_fdx_save_error(&mut self, error: fdx_storage::TransactionalSaveError) {
        self.show_error(format!(
            "FDX_SAVE_FAILED: {}; destination and live document unchanged",
            error
        ));
    }

    fn open_native(&mut self, ctx: &egui::Context) {
        let dialog = rfd::FileDialog::new().add_filter("Final Draft document", &["fdx"]);
        dispatch_open_selection(dialog.pick_file(), |path| {
            self.dispatch_document_path_command(path, false, false, ctx);
        });
    }

    /// Format selection is based on bounded file content; the extension only
    /// controls the file-picker filter, never the decoder.
    fn open_staged_native_at(&mut self, path: &std::path::Path) -> bool {
        if self.pending_fdx_open.is_some() {
            self.show_error("OPEN_BLOCKED_BY_MIGRATION: resolve pending FDX import; active document unchanged");
            return false;
        }
        if self.session.dirty() {
            self.show_error("OPEN_CANCELLED_DIRTY: unsaved project and active document unchanged");
            return false;
        }
        let source = match fdx_storage::open_source(&StdFdxStorageFileSystem, path) {
            Ok(source) => source,
            Err(_) => {
                self.show_error("FDX_OPEN_FAILED: unable to read selected document; active document unchanged");
                return false;
            }
        };
        if looks_like_legacy_json(source.bytes()) {
            self.show_error(
                "LEGACY_JSON_OPEN_REJECTED: format unsupported; active document unchanged",
            );
            return false;
        }

        match fdx::preflight_version(source.bytes()) {
            Ok(FdxVersionStatus::Version6) => {
                self.finish_fdx_import(path.to_owned(), source, FdxMigrationConsent::Continue)
            }
            Ok(FdxVersionStatus::ExplicitNon6 { declared, .. }) => {
                self.pending_fdx_open = Some(PendingFdxOpen {
                    path: path.to_owned(),
                    source,
                    declared_version: declared,
                });
                self.clear_error();
                true
            }
            Ok(FdxVersionStatus::Missing) => {
                self.show_error("FDX_VERSION_MISSING: active document unchanged");
                false
            }
            Ok(FdxVersionStatus::Malformed) => {
                self.show_error("FDX_VERSION_INVALID: active document unchanged");
                false
            }
            Ok(FdxVersionStatus::Duplicate) => {
                self.show_error("FDX_VERSION_DUPLICATE: active document unchanged");
                false
            }
            Err(_) => {
                self.show_error("FDX_VERSION_CHECK_FAILED: selected document could not be checked; active document unchanged");
                false
            }
        }
    }

    fn finish_fdx_import(
        &mut self,
        path: PathBuf,
        source: OpenedFdxSource,
        consent: FdxMigrationConsent,
    ) -> bool {
        if consent == FdxMigrationConsent::Cancel {
            self.clear_error();
            return true;
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
                )
            }
            Ok(FdxImportOutcome::Cancelled { .. }) => {
                self.clear_error();
                false
            }
            Err(_) => {
                self.show_error("FDX_IMPORT_FAILED: selected document could not be imported; active document unchanged");
                false
            }
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
    ) -> bool {
        let mut replacement = match EditingSession::with_policy(project, self.session.policy()) {
            Ok(replacement) => replacement,
            Err(_) => {
                self.show_error("FDX_SESSION_ACTIVATION_FAILED: active document unchanged");
                return false;
            }
        };
        let restored_selection = scriptcraft_commands::Selection::from_core(text_state.selection)
            .ok()
            .map(|selection| Command::EditorSelectionSet { selection })
            .is_some_and(|command| {
                matches!(
                    scriptcraft_commands::execute_editor_command(&mut replacement, &command),
                    scriptcraft_commands::CommandOutcome::Changed
                        | scriptcraft_commands::CommandOutcome::NoOp
                )
            });
        if !restored_selection {
            if let Ok(selection) = scriptcraft_commands::Selection::from_core(Selection::caret(
                Position {
                    element: 0,
                    offset: 0,
                },
            )) {
                let _ = scriptcraft_commands::execute_editor_command(
                    &mut replacement,
                    &Command::EditorSelectionSet { selection },
                );
            }
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
        if let Some(warning) = migration_status {
            self.show_error(warning);
        } else {
            self.clear_error();
        }
        true
    }

    fn resolve_pending_fdx_import(&mut self, consent: FdxMigrationConsent) {
        let Some(pending) = self.pending_fdx_open.take() else {
            return;
        };
        if consent == FdxMigrationConsent::Cancel {
            self.clear_error();
            return;
        }
        if self.session.dirty() {
            self.show_error("OPEN_CANCELLED_DIRTY: unsaved project and active document unchanged");
            return;
        }
        if !self.finish_fdx_import(pending.path, pending.source, consent) {
            show_child_startup_open_failure();
        }
    }

    fn save_as_native(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Final Draft document", &["fdx"])
            .set_file_name("Untitled Screenplay.fdx")
            .save_file()
        {
            // A selected path means the native replacement prompt, if any, was accepted.
            self.dispatch_document_path_command(&path, true, true, ctx);
        }
    }

    fn dispatch_document_path_command(
        &mut self,
        path: &std::path::Path,
        save_as: bool,
        overwrite: bool,
        ctx: &egui::Context,
    ) {
        let Some(path) = path
            .to_str()
            .and_then(|path| DocumentPath::new(path).ok())
        else {
            self.show_error("DOCUMENT_PATH_REJECTED: selected path is not valid UTF-8; active document unchanged");
            return;
        };
        let command = if save_as {
            Command::FileSaveAs { path, overwrite }
        } else {
            Command::FileOpen { path }
        };
        self.dispatch_command(
            CommandInvocation::new(command, CommandOrigin::InternalFollowUp),
            ctx,
        );
    }

    fn export_pdf(&mut self) {
        let default_filename = pdf_export_filename(self.native_path.as_deref());
        let selected_path = rfd::FileDialog::new()
            .add_filter("PDF document", &["pdf"])
            .set_file_name(&default_filename)
            .save_file();

        match export_pdf_to_selected_path(
            selected_path.as_deref(),
            &StdFileSystem,
            || {
                exports::export_pdf_with_layout_inputs(
                    self.session.project(),
                    self.session.layout_inputs(),
                )
            },
        ) {
            Ok(false) => {}
            Ok(true) => self.clear_error(),
            Err(PdfExportAdapterError::Write(_)) => self.show_error(
                "PDF_EXPORT_WRITE_FAILED: filesystem write failed; screenplay unchanged; export output may be incomplete",
            ),
            Err(PdfExportAdapterError::Export(exports::ExportError::Layout(error))) => {
                self.show_error(format!(
                    "PDF_EXPORT_LAYOUT_FAILED: {}; screenplay unchanged; no complete export produced",
                    safe_layout_error_category(&error)
                ));
            }
        }
    }

    fn show_error(&mut self, detail: impl AsRef<str>) {
        let health = self.session_health();
        self.post_diagnostic(&format!("{}; {health}", detail.as_ref()));
    }

    fn show_edit_error(&mut self, error: EditError) {
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
                EditError::UnsupportedTransition { .. } => "EDIT_UNSUPPORTED_TRANSITION",
                EditError::Preservation(_) => "EDIT_PRESERVATION_CONSTRAINT",
                EditError::CastList(_) => "EDIT_CAST_LIST_CONSTRAINT",
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

    fn clear_error(&mut self) {
        self.pending_diagnostic_commands
            .push_back(Command::UiStatusSet { message: None });
    }

    fn sync_native_title(&mut self, ctx: &egui::Context) {
        if let Some(title) = changed_native_document_title(
            &self.native_title,
            self.native_path.as_deref(),
        ) {
            app_commands::DesktopViewportEffects::title(ctx, title.clone());
            self.native_title = title;
        }
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
        reveal_intent: RevealIntent,
    ) {
        match result {
            Ok(()) => {
                self.preferred_vertical_x = None;
                self.clear_error();
                self.rendered_revision = None;
                self.reveal_intent = reveal_intent;
            }
            Err(error) => self.show_edit_error(error),
        }
    }


}

fn chrome_frame(palette: theme::ToolbarPalette) -> egui::Frame {
    // The semantic divider is painted along the toolbar's lower edge; keep the
    // frame stroke-free so no top or vertical borders are introduced.
    egui::Frame::NONE
        .fill(palette.background)
        .stroke(egui::Stroke::NONE)
}

fn show_application_panels(app: &mut ScriptCraftApp, ui: &mut egui::Ui) -> egui::Rect {
    let ctx = ui.ctx().clone();
    let now = app.timer_now();
    app.reconcile_sprint_timer(now);
    let fullscreen = ctx.input(|input| input.viewport().fullscreen);
    app.focus_mode.observe(fullscreen, now);
    let (_resolved_theme, applied_appearance, theme_palette) = theme::configure_appearance(
        &ctx,
        app.appearance_mode,
        app.applied_appearance,
    );
    app.applied_appearance = applied_appearance;
    app.theme_palette = theme_palette;

    if app.menu_policy.renders_in_window() {
        let availability = app.app_command_availability();
        let menu_panel = egui::Panel::top("application-menu")
            .frame(chrome_frame(app.theme_palette.toolbar))
            .show_separator_line(false)
            .exact_size(menu::MENU_PANEL_HEIGHT)
            .show_inside(ui, |ui| {
                menu::show_menu_bar(
                    ui,
                    availability,
                    &mut app.pending_commands,
                );
            });
        let rect = menu_panel.response.rect;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom() - 0.5),
                egui::pos2(rect.right(), rect.bottom() - 0.5),
            ],
            egui::Stroke::new(1.0, app.theme_palette.toolbar.border),
        );
    }

    egui::Panel::top("titlebar-toolbar")
        .frame(chrome_frame(app.theme_palette.toolbar))
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
    let (modifiers, ime_event_in_frame) = ctx.input(|input| {
        (
            input.modifiers,
            input.events.iter().any(|event| matches!(event, egui::Event::Ime(_))),
        )
    });
    let center = status_bar::project_center(
        &app.session,
        app.surface_focused,
        app.ime_active || ime_event_in_frame,
        app.pending_fdx_open.is_some(),
        modifiers,
        shortcuts::ShortcutPlatform::current(),
    );
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
    let diagnostic_message = if status_bar::diagnostics_enabled() {
        app.diagnostic_message
            .as_ref()
            .map(|message| message.as_str().to_owned())
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
            diagnostic_message: diagnostic_message.as_deref(),
        }
    });
    let status_panel = egui::Panel::bottom("status")
        .frame(status_bar::status_panel_frame(&ctx, app.theme_palette.status))
        .default_size(status_bar::default_panel_height())
        .show_separator_line(false)
        .show_inside(ui, |ui| {
        status_bar::apply_status_palette(ui, app.theme_palette.status);
        let focus_view = app.focus_mode.view();
        let status_intents = status_bar::show_with_focus_mode(
                ui,
                app.theme_palette.status,
                page_number,
                &center,
                diagnostics,
                app.zoom_percent,
                app.appearance_mode,
                &focus_view,
            );
        let timer_button_rect = status_intents.timer_button_rect;
        for command in status_intents.commands {
            let opening_timer = matches!(&command, Command::ToolsSprintTimerTogglePopup)
                && !app.sprint_timer.popup_open;
            app.dispatch_command(
                CommandInvocation::new(command, CommandOrigin::Pointer),
                &ctx,
            );
            if opening_timer {
                app.sprint_timer.popup_anchor = timer_button_rect.unwrap_or(egui::Rect::NOTHING);
            }
        }
        });
    status_bar::paint_panel_top_border(
        ui,
        status_panel.response.rect,
        app.theme_palette.status,
    );
    let central = egui::CentralPanel::default()
        .frame(script_view::central_panel_frame(&ctx, app.theme_palette.script_workspace))
        .show_inside(ui, |ui| app.live_document_surface(ui));
    let timer_commands = sprint_timer::show_popup(&ctx, &mut app.sprint_timer, now);
        for command in timer_commands {
        app.dispatch_command(
            CommandInvocation::new(command, CommandOrigin::Pointer),
            &ctx,
        );
    }
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
        let resolution = match consent {
            FdxMigrationConsent::Continue => ImportResolution::Continue,
            FdxMigrationConsent::Cancel => ImportResolution::Cancel,
        };
        app.dispatch_command(
            CommandInvocation::new(
                Command::FileImportResolve { resolution },
                CommandOrigin::InternalFollowUp,
            ),
            ctx,
        );
    }
}

impl eframe::App for ScriptCraftApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        script_view::chrome_clear_color(self.theme_palette.script_workspace)
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if status_modifiers_changed(self.last_status_modifiers, raw_input.modifiers) {
            self.last_status_modifiers = raw_input.modifiers;
            // Modifier-only transitions may not produce a semantic editor
            // command or a key event; keep the live status projection current.
            self.request_repaint(ctx);
        }
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

        for event in &raw_input.events {
            if let egui::Event::Zoom(factor) = event {
                self.record_native_event("zoom");
                if let Ok(factor) = ZoomFactor::new(*factor) {
                    self.dispatch_command(
                        CommandInvocation::new(
                            Command::ViewZoomPinch { factor },
                            CommandOrigin::InternalFollowUp,
                        ),
                        ctx,
                    );
                }
            }
        }
        // The app has consumed the event for page zoom. Keep it out of egui's
        // later input-state zoom delta so no other surface can apply it again.
        raw_input
            .events
            .retain(|event| !matches!(event, egui::Event::Zoom(_)));

        // The active platform backend owns accelerators: native macOS while the
        // native menu is installed, otherwise the in-window backend on Windows.
        // Consume only recognized pressed keys; preserve unrelated input.
        let ime_event_in_frame = raw_input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)));
        if self.menu_policy.in_window_shortcuts_owned()
            && !self.ime_active
            && !ime_event_in_frame
        {
            let mut retained_events = Vec::with_capacity(raw_input.events.len());
            for event in std::mem::take(&mut raw_input.events) {
                let command = match &event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => menu::command_for_key_event(*key, *modifiers),
                    _ => None,
                };
                if let Some(command) = command {
                    self.pending_commands
                        .enqueue_id(command, CommandOrigin::Shortcut);
                } else {
                    retained_events.push(event);
                }
            }
            raw_input.events = retained_events;
        }
        if scale_changed {
            self.page_geometry.invalidate();
            self.page_scale = None;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ScriptCraftApp::update(self, ui);
    }
}

#[cfg(test)]
mod status_guidance_input_tests {
    use super::status_modifiers_changed;
    use eframe::egui;

    #[test]
    fn modifier_only_press_and_release_are_distinct_repaint_transitions() {
        let pressed = egui::Modifiers {
            command: true,
            ..egui::Modifiers::NONE
        };
        assert!(status_modifiers_changed(egui::Modifiers::NONE, pressed));
        assert!(status_modifiers_changed(pressed, egui::Modifiers::NONE));
        assert!(!status_modifiers_changed(pressed, pressed));
    }
}

impl ScriptCraftApp {
    fn update(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let frame_started = Instant::now();
        self.performance.frames = self.performance.frames.saturating_add(1);
        self.update_native_window_state(&ctx);
        if let Some(opened) = self.consume_startup_document_handoff() {
            if !opened {
                show_child_startup_open_failure();
            }
        }
        #[cfg(target_os = "macos")]
        if self.menu_policy.installs_native_macos() {
            if let Some(native_menu) = &self.native_menu {
                native_menu.drain_events(&mut self.pending_commands);
            }
        }
        // Menu activations rendered in the previous pass are dispatched once
        // before ordinary editor input, with command availability rechecked.
        self.dispatch_pending_commands(&ctx);
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
            let status_panel = egui::Panel::bottom("status")
                .frame(status_bar::status_panel_frame(&ctx, self.theme_palette.status))
                .default_size(status_bar::default_panel_height())
                .show_separator_line(false)
                .show_inside(ui, |ui| {
                    status_bar::show_modal_ready_status(ui, self.theme_palette.status);
                });
            status_bar::paint_panel_top_border(
                ui,
                status_panel.response.rect,
                self.theme_palette.status,
            );
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let _ = ui.allocate_space(ui.available_size());
            });
            show_fdx_migration_dialog(self, &ctx);
        } else {
            show_application_panels(self, ui);
        }
        self.flush_pending_diagnostic_commands(&ctx);
        self.schedule_sprint_timer_repaint(&ctx, self.timer_now());
        self.schedule_focus_mode_repaint(&ctx, self.timer_now());
        #[cfg(target_os = "macos")]
        if self.menu_policy.installs_native_macos() {
            if let Some(native_menu) = &self.native_menu {
                native_menu.synchronize(self.app_command_availability());
            }
        }
        self.sync_native_title(&ctx);
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
            self.restore_imported_window(commands, ctx);
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
        let scroll_changed = script_view::has_nonzero_mouse_wheel(&input.raw.events)
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

fn native_options() -> Result<eframe::NativeOptions, String> {
    let window_state = fdx::WindowState::DEFAULT;
    let icon = runtime_icon::egui_icon()?;
    Ok(eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(UNTITLED_DOCUMENT_TITLE)
            .with_icon(std::sync::Arc::new(icon))
            .with_inner_size(egui::vec2(
                window_state.width as f32,
                window_state.height as f32,
            ))
            .with_position(egui::pos2(
                window_state.left as f32,
                window_state.top as f32,
            ))
            .with_decorations(true)
            .with_transparent(false)
            .with_title_shown(true)
            .with_titlebar_shown(true)
            .with_fullsize_content_view(false)
            .with_titlebar_buttons_shown(true),
        ..Default::default()
    })
}

struct StartupPresentation {
    options: eframe::NativeOptions,
    menu_policy: menu::RuntimeMenuPolicy,
}

fn current_menu_host() -> menu::MenuHostPlatform {
    if cfg!(target_os = "macos") {
        menu::MenuHostPlatform::MacOS
    } else if cfg!(windows) {
        menu::MenuHostPlatform::Windows
    } else {
        menu::MenuHostPlatform::Other
    }
}

fn resolve_startup_menu_policy(
    host: menu::MenuHostPlatform,
    requested: menu::MenuPlatformChoice,
) -> Result<menu::RuntimeMenuPolicy, String> {
    menu::RuntimeMenuPolicy::resolve(host, requested).map_err(|unsupported| {
        let requested = match unsupported.requested {
            menu::MenuPlatformChoice::Auto => "auto",
            menu::MenuPlatformChoice::MacOS => "macos",
            menu::MenuPlatformChoice::Windows => "windows",
        };
        let host = match unsupported.host {
            menu::MenuHostPlatform::MacOS => "macOS",
            menu::MenuHostPlatform::Windows => "Windows",
            menu::MenuHostPlatform::Other => "this target",
        };
        format!(
            "--menu-platform={requested} is unsupported on {host}\n{STARTUP_USAGE}"
        )
    })
}

fn startup_presentation(
    requested: menu::MenuPlatformChoice,
) -> Result<StartupPresentation, String> {
    let menu_policy = resolve_startup_menu_policy(current_menu_host(), requested)?;
    Ok(StartupPresentation {
        options: native_options()?,
        menu_policy,
    })
}

fn main() -> eframe::Result {
    let startup = match parse_startup_args(std::env::args_os().skip(1)) {
        Ok(startup) => startup,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let StartupPresentation {
        options,
        menu_policy,
    } = match startup_presentation(startup.menu_platform) {
        Ok(presentation) => presentation,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    eframe::run_native(
        UNTITLED_DOCUMENT_TITLE,
        options,
        Box::new(move |cc| {
            document_font::install(&cc.egui_ctx);
            material_font::install(
                &cc.egui_ctx,
                &toolbar_items::custom_material_icon_axes(),
            );
            ScriptCraftApp::new_with_startup(
                menu_policy,
                startup.menu_platform,
                startup.document_path,
            )
                .and_then(|mut app| {
                    #[cfg(target_os = "macos")]
                    if app.menu_policy.installs_native_macos() {
                        let availability = app.app_command_availability();
                        app.native_menu = Some(
                            macos_menu::NativeMenuController::install(
                                availability,
                                cc.egui_ctx.clone(),
                            )
                            .map_err(|error| {
                                format!(
                                    "native macOS menu installation failed; startup aborted rather than showing a menu-less or duplicate-accelerator UI: {error}"
                                )
                            })?,
                        );
                    }
                    Ok(app)
                })
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|error| error.into())
        }),
    )
}

#[cfg(test)]
mod native_title_tests {
    use super::{changed_native_document_title, native_document_title, UNTITLED_DOCUMENT_TITLE};
    use std::path::Path;

    #[test]
    fn unsaved_and_new_documents_use_untitled_title() {
        assert_eq!(native_document_title(None), UNTITLED_DOCUMENT_TITLE);
        assert_eq!(
            native_document_title(Some(Path::new(""))),
            UNTITLED_DOCUMENT_TITLE
        );
        assert_eq!(
            changed_native_document_title(
                "Previously Saved.fdx",
                None
            ),
            Some(UNTITLED_DOCUMENT_TITLE.to_owned())
        );
        assert_eq!(
            changed_native_document_title(UNTITLED_DOCUMENT_TITLE, None),
            None
        );
    }

    #[test]
    fn named_document_uses_only_filename_and_keeps_extension() {
        assert_eq!(
            native_document_title(Some(Path::new(
                "/private/writer/projects/Screenplay Title.fdx"
            ))),
            "Screenplay Title.fdx"
        );
    }

    #[test]
    fn opened_and_save_as_paths_project_their_active_filenames() {
        assert_eq!(
            native_document_title(Some(Path::new("/opened/Opening Draft.fdx"))),
            "Opening Draft.fdx"
        );
        assert_eq!(
            native_document_title(Some(Path::new("/save-as/Final Draft.fdx"))),
            "Final Draft.fdx"
        );
    }

    #[test]
    fn unicode_filename_and_extension_are_preserved() {
        assert_eq!(
            native_document_title(Some(Path::new("/drafts/Été 東京 — scène.fdx"))),
            "Été 東京 — scène.fdx"
        );
    }

    #[test]
    fn unchanged_projected_title_does_not_request_an_update() {
        assert_eq!(
            changed_native_document_title(
                "Screenplay Title.fdx",
                Some(Path::new("/another/location/Screenplay Title.fdx"))
            ),
            None
        );
        assert_eq!(
            changed_native_document_title(
                "Untitled Screenplay",
                Some(Path::new("/drafts/First.fdx"))
            ),
            Some("First.fdx".to_owned())
        );
    }

    #[test]
    fn unchanged_active_path_for_failed_or_cancelled_operation_keeps_title() {
        let active_path = Path::new("/drafts/Current Draft.fdx");
        let active_title = native_document_title(Some(active_path));

        assert_eq!(
            changed_native_document_title(&active_title, Some(active_path)),
            None
        );
    }
}

#[cfg(test)]
mod pdf_export_adapter_tests {
    use super::{export_pdf_to_selected_path, pdf_export_filename, UNTITLED_PDF_FILENAME};
    use scriptcraft_core::persistence::{FileSystem, PersistenceError};
    use std::cell::{Cell, RefCell};
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    struct RecordingFileSystem {
        writes: RefCell<Vec<(PathBuf, Vec<u8>)>>,
    }

    impl FileSystem for RecordingFileSystem {
        fn read(&self, _path: &Path) -> Result<Vec<u8>, PersistenceError> {
            Err(PersistenceError::Io("unused test operation".to_owned()))
        }

        fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
            self.writes
                .borrow_mut()
                .push((path.to_owned(), bytes.to_vec()));
            Ok(())
        }

        fn rename(&self, _from: &Path, _to: &Path) -> Result<(), PersistenceError> {
            Err(PersistenceError::Io("unused test operation".to_owned()))
        }

        fn remove(&self, _path: &Path) -> Result<(), PersistenceError> {
            Err(PersistenceError::Io("unused test operation".to_owned()))
        }

        fn list(&self, _directory: &Path) -> Result<Vec<PathBuf>, PersistenceError> {
            Err(PersistenceError::Io("unused test operation".to_owned()))
        }
    }

    #[test]
    fn pdf_filename_replaces_only_the_final_extension_and_preserves_unicode() {
        assert_eq!(
            pdf_export_filename(Some(Path::new("/drafts/Draft.fdx"))),
            "Draft.pdf"
        );
        assert_eq!(
            pdf_export_filename(Some(Path::new("/drafts/Draft.final.fdx"))),
            "Draft.final.pdf"
        );
        assert_eq!(
            pdf_export_filename(Some(Path::new("/drafts/Été 東京.fdx"))),
            "Été 東京.pdf"
        );
    }

    #[test]
    fn pdf_filename_uses_fallback_for_unsaved_or_empty_filename() {
        assert_eq!(pdf_export_filename(None), UNTITLED_PDF_FILENAME);
        assert_eq!(
            pdf_export_filename(Some(Path::new(""))),
            UNTITLED_PDF_FILENAME
        );
    }

    #[cfg(unix)]
    #[test]
    fn pdf_filename_uses_fallback_for_non_utf8_filename() {
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(std::ffi::OsStr::from_bytes(b"Draft-\xff.fdx"));
        assert_eq!(pdf_export_filename(Some(path)), UNTITLED_PDF_FILENAME);
    }

    #[test]
    fn cancelled_pdf_picker_does_not_build_or_write_export() {
        let filesystem = RecordingFileSystem::default();
        let built = Cell::new(false);

        let outcome = export_pdf_to_selected_path(None, &filesystem, || {
            built.set(true);
            Ok(b"%PDF-1.4".to_vec())
        });

        assert!(matches!(outcome, Ok(false)));
        assert!(!built.get());
        assert!(filesystem.writes.borrow().is_empty());
    }

    #[test]
    fn pdf_export_writes_generated_bytes_only_to_selected_path() {
        let filesystem = RecordingFileSystem::default();
        let selected_path = Path::new("/writer-selected/renamed-output.pdf");
        let bytes = b"%PDF-1.4 test bytes".to_vec();

        let outcome = export_pdf_to_selected_path(Some(selected_path), &filesystem, || {
            Ok(bytes.clone())
        });

        assert!(matches!(outcome, Ok(true)));
        assert_eq!(
            filesystem.writes.borrow().as_slice(),
            &[(selected_path.to_owned(), bytes)]
        );
    }
}
