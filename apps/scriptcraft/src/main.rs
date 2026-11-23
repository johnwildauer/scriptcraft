#![deny(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use std::io::Write as _;

mod app;
mod chrome;
mod commands;
mod documents;
mod platform;
mod presentation;
mod settings;
mod views;
mod widgets;
use documents::lifecycle::show_native_lifecycle_error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupFailureKind {
    InvalidInvocation,
    PresentationUnavailable,
}

impl StartupFailureKind {
    fn exit_code(self) -> i32 {
        match self {
            Self::InvalidInvocation | Self::PresentationUnavailable => 2,
        }
    }
}

fn startup_failure_copy(kind: StartupFailureKind) -> (&'static str, String) {
    let title = "Unable to Start ScriptCraft";
    let message = match kind {
        StartupFailureKind::InvalidInvocation => {
            format!(
                "The startup invocation is invalid.\n\n{}",
                app::startup::STARTUP_USAGE
            )
        }
        StartupFailureKind::PresentationUnavailable => {
            "ScriptCraft could not prepare its startup presentation. No document was opened."
                .to_owned()
        }
    };
    (title, message)
}

fn exit_for_startup_failure(kind: StartupFailureKind, diagnostic: &str) -> ! {
    // Keep detailed diagnostics for development, but never expose arguments or
    // presentation errors in the bounded Windows GUI-subsystem dialog. A
    // missing stderr must not prevent native feedback from appearing.
    let _ = writeln!(std::io::stderr(), "{diagnostic}");
    #[cfg(target_os = "windows")]
    {
        let (title, message) = startup_failure_copy(kind);
        show_native_lifecycle_error(title, &message);
    }
    std::process::exit(kind.exit_code());
}

fn main() -> eframe::Result {
    app::startup::run()
}
