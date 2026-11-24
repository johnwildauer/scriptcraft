//! Process startup parsing, presentation, and native launch orchestration.

use crate::documents::{
    replacement_protocol,
    save::fresh_document_title_for_builtin_template,
};
use crate::{
    app::ScriptCraftApp,
    chrome::{menu, theme, toolbar_items},
    presentation::{document_font, material_font, runtime_icon},
    settings::{SettingsStore, ThemeChoice},
    StartupFailureKind,
};
use std::ffi::OsString;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
use crate::platform::macos::{
    macos_document_open, macos_lifecycle, macos_menu, macos_open_documents,
};

use scriptcraft_commands::DocumentPath;
use scriptcraft_fdx::BuiltinTemplateId;

// Native argument boundaries are authoritative: shell quoting has already
// produced one argument, so paths are never split or reparsed here. Ordinary
// shell associations should pass the selected path as one quoted native path.
// `--open-handoff=<id>` is internal and only accompanies the custom option.
pub(crate) const STARTUP_USAGE: &str = "usage: scriptcraft [--dev] [--reset-settings] [--menu-platform=auto|macos|windows] [--new-template=screenplay|stageplay|dramatists-guild-musical (internal; exclusive of document paths) | <UTF-8 FDX or FDXT path> | --document-path=<UTF-8 FDX or FDXT path> [--open-handoff=<id> (internal)]]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartupDocumentSource {
    CustomOption,
    PositionalArgument,
    NativeOpenDocuments,
}

/// Startup document and its optional replacement acknowledgement identity.
/// Keeping these values together prevents ordinary document launches from
/// being mistaken for acknowledged replacement children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupDocumentInput {
    pub(crate) path: PathBuf,
    pub(crate) source: StartupDocumentSource,
    pub(crate) handoff_id: Option<replacement_protocol::HandoffId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartupConfig {
    development: bool,
    menu_platform: menu::MenuPlatformChoice,
    startup_document: Option<StartupDocumentInput>,
    startup_template: Option<BuiltinTemplateId>,
    reset_settings: bool,
}

fn parse_startup_args(args: impl IntoIterator<Item = OsString>) -> Result<StartupConfig, String> {
    let mut seen_development = false;
    let mut seen_reset_settings = false;
    let mut seen_menu_platform = false;
    let mut seen_document_path = false;
    let mut seen_open_handoff = false;
    let mut seen_new_template = false;
    let mut development = false;
    let mut menu_platform = menu::MenuPlatformChoice::Auto;
    let mut document_input: Option<(PathBuf, StartupDocumentSource)> = None;
    let mut handoff_id = None;
    let mut startup_template = None;
    let mut reset_settings = false;

    for argument in args {
        let Some(argument) = argument.to_str() else {
            return Err(format!("arguments must be valid UTF-8\n{STARTUP_USAGE}"));
        };
        if argument == "--dev" {
            if seen_development {
                return Err(format!("duplicate --dev option\n{STARTUP_USAGE}"));
            }
            seen_development = true;
            development = true;
        } else if argument.starts_with("--dev=") {
            return Err(format!("--dev does not accept a value\n{STARTUP_USAGE}"));
        } else if argument == "--reset-settings" {
            if seen_reset_settings {
                return Err(format!("duplicate --reset-settings option\n{STARTUP_USAGE}"));
            }
            seen_reset_settings = true;
            reset_settings = true;
        } else if argument.starts_with("--reset-settings=") {
            return Err(format!("--reset-settings does not accept a value\n{STARTUP_USAGE}"));
        } else if argument == "--menu-platform" {
            if seen_menu_platform {
                return Err(format!("duplicate --menu-platform option\n{STARTUP_USAGE}"));
            }
            seen_menu_platform = true;
            return Err(format!(
                "--menu-platform requires an equals-form value\n{STARTUP_USAGE}"
            ));
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
        } else if argument == "--new-template" {
            return Err(format!(
                "--new-template requires an equals-form value\n{STARTUP_USAGE}"
            ));
        } else if let Some(value) = argument.strip_prefix("--new-template=") {
            if seen_new_template {
                return Err(format!("duplicate --new-template option\n{STARTUP_USAGE}"));
            }
            seen_new_template = true;
            startup_template = BuiltinTemplateId::from_stable_id(value);
            if startup_template.is_none() {
                return Err(format!("invalid --new-template value\n{STARTUP_USAGE}"));
            }
        } else if argument == "--document-path" {
            if seen_document_path {
                return Err(format!("duplicate --document-path option\n{STARTUP_USAGE}"));
            }
            seen_document_path = true;
            if document_input
                .as_ref()
                .is_some_and(|(_, source)| *source == StartupDocumentSource::PositionalArgument)
            {
                return Err(format!(
                    "ambiguous document sources: use one positional path or --document-path, not both\n{STARTUP_USAGE}"
                ));
            }
            if startup_template.is_some() {
                return Err(format!(
                    "--new-template cannot be combined with a document path\n{STARTUP_USAGE}"
                ));
            }
            return Err(format!(
                "--document-path requires an equals-form value\n{STARTUP_USAGE}"
            ));
        } else if let Some(value) = argument.strip_prefix("--document-path=") {
            if seen_document_path {
                return Err(format!("duplicate --document-path option\n{STARTUP_USAGE}"));
            }
            seen_document_path = true;
            if document_input
                .as_ref()
                .is_some_and(|(_, source)| *source == StartupDocumentSource::PositionalArgument)
            {
                return Err(format!(
                    "ambiguous document sources: use one positional path or --document-path, not both\n{STARTUP_USAGE}"
                ));
            }
            if startup_template.is_some() {
                return Err(format!(
                    "--new-template cannot be combined with a document path\n{STARTUP_USAGE}"
                ));
            }
            let path = DocumentPath::new(value.to_owned()).map_err(|_| {
                let reason = if value.is_empty() {
                    "--document-path value must not be empty"
                } else {
                    "invalid --document-path value"
                };
                format!("{reason}\n{STARTUP_USAGE}")
            })?;
            document_input = Some((
                path.as_path().to_path_buf(),
                StartupDocumentSource::CustomOption,
            ));
        } else if argument == "--open-handoff" {
            if seen_open_handoff {
                return Err(format!("duplicate --open-handoff option\n{STARTUP_USAGE}"));
            }
            seen_open_handoff = true;
            return Err(format!(
                "--open-handoff requires an equals-form value\n{STARTUP_USAGE}"
            ));
        } else if let Some(value) = argument.strip_prefix("--open-handoff=") {
            if seen_open_handoff {
                return Err(format!("duplicate --open-handoff option\n{STARTUP_USAGE}"));
            }
            seen_open_handoff = true;
            handoff_id = Some(
                replacement_protocol::HandoffId::parse_decimal(value)
                    .map_err(|_| format!("invalid --open-handoff identity\n{STARTUP_USAGE}"))?,
            );
        } else {
            if argument.starts_with('-') {
                return Err(format!("unknown option\n{STARTUP_USAGE}"));
            }
            if startup_template.is_some() {
                return Err(format!(
                    "--new-template cannot be combined with a document path\n{STARTUP_USAGE}"
                ));
            }
            match document_input.as_ref().map(|(_, source)| source) {
                Some(StartupDocumentSource::CustomOption) => {
                    return Err(format!(
                        "ambiguous document sources: use one positional path or --document-path, not both\n{STARTUP_USAGE}"
                    ));
                }
                Some(StartupDocumentSource::PositionalArgument) => {
                    return Err(format!(
                        "multiple positional document paths are not supported\n{STARTUP_USAGE}"
                    ));
                }
                Some(StartupDocumentSource::NativeOpenDocuments) | None => {}
            }
            let path = DocumentPath::new(argument.to_owned())
                .map_err(|_| format!("invalid positional document path\n{STARTUP_USAGE}"))?;
            document_input = Some((
                path.as_path().to_path_buf(),
                StartupDocumentSource::PositionalArgument,
            ));
        };
    }

    if handoff_id.is_some()
        && !document_input
            .as_ref()
            .is_some_and(|(_, source)| *source == StartupDocumentSource::CustomOption)
    {
        return Err(format!(
            "--open-handoff requires --document-path\n{STARTUP_USAGE}"
        ));
    }
    if startup_template.is_some() && document_input.is_some() {
        return Err(format!(
            "--new-template cannot be combined with a document path\n{STARTUP_USAGE}"
        ));
    }

    Ok(StartupConfig {
        development,
        menu_platform,
        startup_document: document_input.map(|(path, source)| StartupDocumentInput {
            path,
            source,
            handoff_id: if source == StartupDocumentSource::CustomOption {
                handoff_id
            } else {
                None
            },
        }),
        startup_template,
        reset_settings,
    })
}

pub(super) fn appearance_mode_for_theme(choice: ThemeChoice) -> theme::AppearanceMode {
    match choice {
        ThemeChoice::System => theme::AppearanceMode::System,
        ThemeChoice::Day => theme::AppearanceMode::Day,
        ThemeChoice::Night => theme::AppearanceMode::Night,
        ThemeChoice::Midnight => theme::AppearanceMode::Midnight,
    }
}

pub(crate) fn menu_platform_argument(choice: menu::MenuPlatformChoice) -> OsString {
    let value = match choice {
        menu::MenuPlatformChoice::Auto => "auto",
        menu::MenuPlatformChoice::MacOS => "macos",
        menu::MenuPlatformChoice::Windows => "windows",
    };
    OsString::from(format!("--menu-platform={value}"))
}

fn native_options(initial_title: &str) -> Result<eframe::NativeOptions, String> {
    let window_state = scriptcraft_fdx::WindowState::DEFAULT;
    let icon = runtime_icon::egui_icon()?;
    Ok(eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(initial_title)
            .with_icon(std::sync::Arc::new(icon))
            .with_inner_size(eframe::egui::vec2(
                window_state.width as f32,
                window_state.height as f32,
            ))
            .with_position(eframe::egui::pos2(
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
        format!("--menu-platform={requested} is unsupported on {host}\n{STARTUP_USAGE}")
    })
}

fn startup_presentation(
    requested: menu::MenuPlatformChoice,
    initial_title: &str,
) -> Result<StartupPresentation, String> {
    let menu_policy = resolve_startup_menu_policy(current_menu_host(), requested)?;
    Ok(StartupPresentation {
        options: native_options(initial_title)?,
        menu_policy,
    })
}

// Windows GUI-subsystem startup failures need native feedback before exit;
// neither failure path retries nor falls back to constructing a blank document.
pub(crate) fn run() -> eframe::Result {
    let startup = match parse_startup_args(std::env::args_os().skip(1)) {
        Ok(startup) => startup,
        Err(error) => {
            crate::exit_for_startup_failure(StartupFailureKind::InvalidInvocation, &error);
        }
    };
    let mut settings_store = SettingsStore::load();
    if startup.reset_settings {
        settings_store.reset_to_defaults();
    }
    let appearance_mode = appearance_mode_for_theme(settings_store.theme());
    let effective_template = startup.startup_template.unwrap_or_else(|| {
        if startup.startup_document.is_some() {
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID
        } else {
            settings_store.effective_template()
        }
    });
    let initial_native_title = fresh_document_title_for_builtin_template(
        effective_template,
    );
    let StartupPresentation {
        options,
        menu_policy,
    } = match startup_presentation(startup.menu_platform, &initial_native_title) {
        Ok(presentation) => presentation,
        Err(error) => {
            crate::exit_for_startup_failure(StartupFailureKind::PresentationUnavailable, &error);
        }
    };
    #[cfg(target_os = "macos")]
    let native_open_documents = {
        let inbox = macos_document_open::MacOsDocumentOpenInbox::new();
        match macos_open_documents::NativeOpenDocumentsController::install(inbox) {
            Ok(controller) => controller,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
    };
    eframe::run_native(
        &initial_native_title,
        options,
        Box::new(move |cc| {
            let startup_document = startup.startup_document;
            let startup_template = startup.startup_template;

            // ScriptCraft owns page-zoom commands; document Ctrl+Shift+0 must
            // not also match egui's global Ctrl+0 reset.
            cc.egui_ctx.options_mut(|options| {
                options.zoom_with_keyboard = false;
            });

            let document_font_store = document_font::install(&cc.egui_ctx);
            material_font::install(&cc.egui_ctx, &toolbar_items::custom_material_icon_axes());
            ScriptCraftApp::new_with_startup(
                menu_policy,
                startup.menu_platform,
                startup_document,
                startup_template,
                effective_template,
                appearance_mode,
                settings_store,
                startup.development,
            )
                .and_then(|mut app| {
                    let (_, applied, palette) = theme::configure_appearance(
                        &cc.egui_ctx,
                        app.appearance_mode,
                        None,
                    );
                    app.appearance_applications
                        .set(theme::AppearanceContext::Root, applied);
                    app.theme_palette = palette;
                    app.document_font_store = document_font_store;
                    #[cfg(target_os = "macos")]
                    {
                        native_open_documents.attach_repaint_context(cc.egui_ctx.clone());
                        app.native_open_documents = Some(native_open_documents);
                        app.native_lifecycle = Some(
                            macos_lifecycle::NativeLifecycleController::install(
                                cc.egui_ctx.clone(),
                            )
                            .map_err(|_| {
                                "native macOS Quit handler installation failed; startup aborted"
                                    .to_owned()
                            })?,
                        );
                    }
                    #[cfg(target_os = "macos")]
                    if app.menu_policy.installs_native_macos() {
                        let availability = app.app_command_availability();
                        let dynamic_elements = app.dynamic_element_menu_projection();
                        let formatting = crate::chrome::formatting_menu::project_for_target(
                            &app.session,
                            app.active_editor_surface,
                            app.root_document_view == crate::app::RootDocumentView::BeatBoard,
                        );
                        app.native_menu = Some(
                            macos_menu::NativeMenuController::install(
                                availability,
                                app.script_view_mode,
                                &dynamic_elements,
                                &formatting,
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
