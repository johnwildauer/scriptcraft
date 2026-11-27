//! App-private global settings and best-effort background persistence.

use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use directories::ProjectDirs;
use scriptcraft_fdx::{BuiltinTemplateId, DEFAULT_BUILTIN_TEMPLATE_ID};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const SETTINGS_FILE_NAME: &str = "settings.toml";
const SETTINGS_SCHEMA_VERSION: u32 = 1;
const SAVE_DEBOUNCE: Duration = Duration::from_millis(350);
const BACKUP_CANDIDATES: u32 = 1_000;
const TEMP_FILE_CANDIDATES: u32 = 1_000;

const DEFAULT_TOOLBAR_IDS: &[&str] = &[
    "flexible-space",
    "views",
    "script-page",
    "beat-board",
    "flexible-space",
    "title-page",
    "elements",
    "flexible-space",
    "show-hide",
    "navigator",
    "flexible-space",
    "overflow",
];

const DEFAULT_TITLE_PAGE_TOOLBAR_IDS: &[&str] = &[
    "flexible-space",
    "styles",
    "alignment",
    "flexible-space",
    "close-title-page",
    "overflow",
];

fn default_toolbar_ids() -> Vec<String> {
    DEFAULT_TOOLBAR_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect()
}

fn default_title_page_toolbar_ids() -> Vec<String> {
    DEFAULT_TITLE_PAGE_TOOLBAR_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect()
}

/// Durable global preferences. Missing fields inherit the version-1 defaults;
/// unknown fields are intentionally tolerated during alpha.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) schema_version: u32,
    pub(crate) theme: ThemeChoice,
    pub(crate) template: String,
    pub(crate) show_ruler: bool,
    pub(crate) show_story_map: bool,
    pub(crate) toolbar: Vec<String>,
    pub(crate) title_page_toolbar: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            theme: ThemeChoice::System,
            template: DEFAULT_BUILTIN_TEMPLATE_ID.as_str().to_owned(),
            show_ruler: true,
            show_story_map: false,
            toolbar: default_toolbar_ids(),
            title_page_toolbar: default_title_page_toolbar_ids(),
        }
    }
}

/// Persisted appearance choice, kept separate from runtime presentation types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ThemeChoice {
    #[default]
    System,
    Day,
    Night,
    Midnight,
}

impl Serialize for ThemeChoice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Self::System => "system",
            Self::Day => "day",
            Self::Night => "night",
            Self::Midnight => "midnight",
        };
        serializer.serialize_str(value)
    }
}

impl<'de> Deserialize<'de> for ThemeChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ThemeChoiceVisitor;

        impl<'de> Visitor<'de> for ThemeChoiceVisitor {
            type Value = ThemeChoice;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a theme string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(match value {
                    "system" => ThemeChoice::System,
                    "day" => ThemeChoice::Day,
                    "night" => ThemeChoice::Night,
                    "midnight" => ThemeChoice::Midnight,
                    _ => ThemeChoice::System,
                })
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_str(ThemeChoiceVisitor)
    }
}

/// App-thread-owned snapshot and its optional persistence channel.
pub(crate) struct SettingsStore {
    settings: Settings,
    path: Option<PathBuf>,
    writer: Option<Sender<Settings>>,
}

impl SettingsStore {
    /// Resolves, logs, and loads settings without making startup failure fatal.
    pub(crate) fn load() -> Self {
        let Some(project_dirs) = ProjectDirs::from("com", "ScriptCraft", "ScriptCraft") else {
            eprintln!("Warning: settings directory is unavailable; using in-memory defaults");
            return Self {
                settings: Settings::default(),
                path: None,
                writer: None,
            };
        };

        let path = project_dirs.config_dir().join(SETTINGS_FILE_NAME);
        eprintln!("Settings path: {}", path.display());

        let (settings, persistence_allowed) = load_from_path(&path);
        if BuiltinTemplateId::from_stable_id(&settings.template).is_none() {
            eprintln!(
                "Warning: unknown default template ID; using Screenplay as the effective fallback"
            );
        }

        let writer = if persistence_allowed {
            start_writer(path.clone())
        } else {
            None
        };

        Self {
            settings,
            path: Some(path),
            writer,
        }
    }

    pub(crate) fn settings(&self) -> &Settings {
        &self.settings
    }

    pub(crate) fn config_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(crate) fn theme(&self) -> ThemeChoice {
        self.settings.theme
    }

    /// Returns a catalog identity without changing the raw persisted ID.
    pub(crate) fn effective_template(&self) -> BuiltinTemplateId {
        BuiltinTemplateId::from_stable_id(&self.settings.template)
            .unwrap_or(DEFAULT_BUILTIN_TEMPLATE_ID)
    }

    pub(crate) fn ruler_visible(&self) -> bool {
        self.settings.show_ruler
    }

    pub(crate) fn story_map_visible(&self) -> bool {
        self.settings.show_story_map
    }


    /// Updates the active theme and publishes only a real change.
    pub(crate) fn set_theme(&mut self, theme: ThemeChoice) -> bool {
        if self.settings.theme == theme {
            return false;
        }
        self.settings.theme = theme;
        self.publish_snapshot();
        true
    }

    /// Updates the raw stable template ID and publishes only a real change.
    pub(crate) fn set_template(&mut self, template: String) -> bool {
        if self.settings.template == template {
            return false;
        }
        self.settings.template = template;
        self.publish_snapshot();
        true
    }

    /// Updates ruler visibility and publishes only a real change.
    pub(crate) fn set_ruler_visible(&mut self, visible: bool) -> bool {
        if self.settings.show_ruler == visible {
            return false;
        }
        self.settings.show_ruler = visible;
        self.publish_snapshot();
        true
    }

    /// Updates global Outline Editor visibility and publishes only a real change.
    pub(crate) fn set_story_map_visible(&mut self, visible: bool) -> bool {
        if self.settings.show_story_map == visible {
            return false;
        }
        self.settings.show_story_map = visible;
        self.publish_snapshot();
        true
    }


    /// Restores version-1 defaults and forces a write even when already equal.
    pub(crate) fn reset_to_defaults(&mut self) {
        if self.writer.is_none() {
            eprintln!("Warning: settings reset is memory-only because persistence is unavailable");
        }
        self.settings = Settings::default();
        self.publish_snapshot();
    }

    pub(crate) fn persistence_available(&self) -> bool {
        self.writer.is_some()
    }

    fn publish_snapshot(&mut self) {
        let Some(writer) = self.writer.as_ref() else {
            return;
        };
        if writer.send(self.settings.clone()).is_err() {
            self.writer = None;
            eprintln!(
                "Warning: settings writer is unavailable; persistence disabled for this process"
            );
        }
    }
}

fn load_from_path(path: &Path) -> (Settings, bool) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (Settings::default(), true);
        }
        Err(error) => {
            eprintln!("Warning: could not read settings file: {error}");
            return recover_existing_file(path);
        }
    };

    let settings = match toml::from_str::<Settings>(&text) {
        Ok(settings) => settings,
        Err(_) => {
            eprintln!("Warning: settings file is invalid; loading defaults");
            return recover_existing_file(path);
        }
    };

    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        eprintln!(
            "Warning: unsupported settings schema version {}; loading defaults",
            settings.schema_version
        );
        return recover_existing_file(path);
    }

    (settings, true)
}

fn recover_existing_file(path: &Path) -> (Settings, bool) {
    match backup_existing_file(path) {
        Ok(()) => (Settings::default(), true),
        Err(error) => {
            eprintln!(
                "Warning: could not preserve settings file; persistence disabled ({})",
                error.kind()
            );
            (Settings::default(), false)
        }
    }
}

/// Makes a no-clobber same-directory backup, then removes the original name.
/// Hard-link creation is atomic with respect to an occupied candidate, unlike
/// Unix rename-overwrite semantics; if unsupported, preservation safely fails.
fn backup_existing_file(path: &Path) -> std::io::Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SETTINGS_FILE_NAME);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    for index in 0..BACKUP_CANDIDATES {
        let suffix = if index == 0 {
            ".bak".to_owned()
        } else {
            format!(".bak-{index}")
        };
        let candidate = parent.join(format!("{file_name}{suffix}"));
        match fs::hard_link(path, &candidate) {
            Ok(()) => {
                if let Err(error) = fs::remove_file(path) {
                    // Keep both names if removing the original fails. Recovery
                    // cannot safely enable writes over the unrecovered source.
                    return Err(error);
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no unused settings backup name is available",
    ))
}

fn start_writer(path: PathBuf) -> Option<Sender<Settings>> {
    let (sender, receiver) = mpsc::channel();
    match thread::Builder::new()
        .name("scriptcraft-settings-writer".to_owned())
        .spawn(move || writer_loop(receiver, path))
    {
        Ok(_handle) => Some(sender),
        Err(error) => {
            eprintln!("Warning: could not start settings writer; persistence disabled: {error}");
            None
        }
    }
}

fn writer_loop(receiver: Receiver<Settings>, path: PathBuf) {
    while let Ok(mut pending) = receiver.recv() {
        loop {
            match receiver.recv_timeout(SAVE_DEBOUNCE) {
                Ok(newer) => pending = newer,
                Err(RecvTimeoutError::Timeout) => {
                    if let Err(error) = save_atomically(&path, &pending) {
                        eprintln!("Warning: could not save settings ({})", error.kind());
                    }
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

fn save_atomically(path: &Path, settings: &Settings) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let serialized = toml::to_string(settings).map_err(std::io::Error::other)?;

    let mut temporary_path = None;
    let mut temporary_file: Option<File> = None;
    for counter in 0..TEMP_FILE_CANDIDATES {
        let candidate = parent.join(format!(
            ".{SETTINGS_FILE_NAME}.tmp-{}-{counter}",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary_path = Some(candidate);
                temporary_file = Some(file);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    let Some(temporary_path) = temporary_path else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "no unused settings temporary name is available",
        ));
    };

    let result = (|| {
        let mut file = temporary_file.take().ok_or_else(|| {
            std::io::Error::other("settings temporary file handle is unavailable")
        })?;
        file.write_all(serialized.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}
