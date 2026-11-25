use crate::{app::ScriptCraftApp, documents::lifecycle::SaveOutcome};
use eframe::egui;
use scriptcraft_commands::{
    Command, CommandId, CommandRejection, EditorSurface, ImportResolution, PdfExportOptions,
};
use scriptcraft_fdx::FdxMigrationConsent;

use super::{DesktopCommandOutcome, DesktopViewportEffects};

impl ScriptCraftApp {
    pub(super) fn dispatch_document_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
        export_pdf: impl FnOnce(&mut ScriptCraftApp),
        resolve_pdf_options: impl FnOnce(&mut ScriptCraftApp, PdfExportOptions),
        open_print_panel: impl FnOnce(&mut ScriptCraftApp),
    ) -> DesktopCommandOutcome {
        match command {
            Command::FileNew => {
                self.launch_blank_instance();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileOpenRequest => {
                self.open_native(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileOpen { path } => {
                self.stage_replacement_document_path(path.as_path());
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileClose => {
                // Body Close is guarded at the next root-update boundary:
                // CancelClose -> native prompt -> one-shot confirmed Close.
                // Title Page remains a separate child-only viewport close.
                match file_close_target(self.active_editor_surface()) {
                    FileCloseTarget::TitlePage => self.close_title_page_window(ctx),
                    FileCloseTarget::Root => DesktopViewportEffects::close_root(ctx),
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ApplicationQuit => {
                // Application Quit always requests root Close, even when the
                // Title Page owns focus. The root-close coordinator performs
                // the ordinary asynchronous CancelClose/prompt/save gate.
                DesktopViewportEffects::close_root(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSave => save_command_outcome(self.save_native(ctx)),
            Command::FileSaveAsRequest => {
                save_command_outcome(self.save_as_native(ctx))
            }
            Command::FileSaveAs { path, overwrite } => {
                DesktopCommandOutcome::SaveCompleted(self.save_fdx_as_at(path.as_path(), overwrite))
            }
            Command::FileImportResolve { resolution } => {
                let _outcome = self.resolve_pending_fdx_import(match resolution {
                    ImportResolution::Continue => FdxMigrationConsent::Continue,
                    ImportResolution::Cancel => FdxMigrationConsent::Cancel,
                });
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileSaveAsPdf => {
                #[cfg(target_os = "macos")]
                export_pdf(self);
                #[cfg(not(target_os = "macos"))]
                self.with_native_output_modal(export_pdf);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FileExportFountain => {
                #[cfg(target_os = "macos")]
                self.export_fountain();
                #[cfg(not(target_os = "macos"))]
                self.with_native_output_modal(|app| app.export_fountain());
                DesktopCommandOutcome::EffectDispatched
            }
            Command::FilePrint => {
                #[cfg(target_os = "macos")]
                open_print_panel(self);
                #[cfg(target_os = "windows")]
                {
                    // Windows printing outlives this dispatch: preparation runs
                    // in the background and the owner-HWND session is started
                    // and polled from a later eframe frame. Keep the shared
                    // output gate raised until that asynchronous owner releases it.
                    self.native_output_modal_active = true;
                    open_print_panel(self);
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                self.with_native_output_modal(open_print_panel);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::PdfIncludeTitlePageResolve { options } => {
                #[cfg(target_os = "macos")]
                resolve_pdf_options(self, options);
                #[cfg(not(target_os = "macos"))]
                self.with_native_output_modal(|app| resolve_pdf_options(app, options));
                DesktopCommandOutcome::EffectDispatched
            }
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    fn with_native_output_modal(&mut self, operation: impl FnOnce(&mut ScriptCraftApp)) {
        self.native_output_modal_active = true;
        operation(self);
        self.native_output_modal_active = false;
    }

    pub(super) fn unavailable_save_outcome(
        &mut self,
        command_id: CommandId,
        reason: super::DesktopUnavailableReason,
    ) -> Option<DesktopCommandOutcome> {
        if !matches!(
            command_id,
            CommandId::FileSave | CommandId::FileSaveAsRequest | CommandId::FileSaveAs
        ) {
            return None;
        }
        self.show_error(format!(
            "SAVE_DISPATCH_UNAVAILABLE: {reason:?}; document unchanged"
        ));
        Some(DesktopCommandOutcome::SaveCompleted(SaveOutcome::Failed))
    }
}

fn save_command_outcome(outcome: SaveOutcome) -> DesktopCommandOutcome {
    match outcome {
        SaveOutcome::Pending => DesktopCommandOutcome::EffectDispatched,
        completed => DesktopCommandOutcome::SaveCompleted(completed),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileCloseTarget {
    Root,
    TitlePage,
}

fn file_close_target(surface: EditorSurface) -> FileCloseTarget {
    match surface {
        EditorSurface::Body => FileCloseTarget::Root,
        EditorSurface::TitlePage => FileCloseTarget::TitlePage,
    }
}
