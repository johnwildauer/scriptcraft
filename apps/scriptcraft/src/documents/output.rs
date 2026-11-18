//! Desktop PDF export and native Print coordination.

use super::{pdf, print};
use crate::app::ScriptCraftApp;
use eframe::egui;
use scriptcraft_export as exports;
use scriptcraft_session::persistence::{FileSystem, StdFileSystem};
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use scriptcraft_model::Project;

const UNTITLED_PDF_FILENAME: &str = "Untitled Screenplay.pdf";

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
    Picker(pdf::PdfExportPickerFailure),
    Path,
    Export(exports::ExportError),
    Write(scriptcraft_session::persistence::PersistenceError),
}

fn export_pdf_to_selected_path<F: FileSystem>(
    picker_result: pdf::PdfExportPickerResult,
    filesystem: &F,
    build_pdf: impl FnOnce(
        scriptcraft_commands::PdfExportOptions,
    ) -> Result<Vec<u8>, exports::ExportError>,
) -> Result<bool, PdfExportAdapterError> {
    let (path, options) = match picker_result {
        pdf::PdfExportPickerResult::Cancel => return Ok(false),
        pdf::PdfExportPickerResult::Error(error) => {
            return Err(PdfExportAdapterError::Picker(error));
        }
        pdf::PdfExportPickerResult::Accept {
            destination,
            options,
        } => (destination, options),
    };
    if path.to_str().is_none() {
        return Err(PdfExportAdapterError::Path);
    }
    let bytes = build_pdf(options).map_err(PdfExportAdapterError::Export)?;
    filesystem
        .write(&path, &bytes)
        .map_err(PdfExportAdapterError::Write)?;
    Ok(true)
}

impl ScriptCraftApp {
    pub(crate) fn export_pdf(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.export_pdf_with_picker(
                crate::platform::macos::macos_pdf_dialog::show_pdf_save_panel,
            );
        }
        #[cfg(target_os = "windows")]
        {
            self.export_pdf_with_picker(
                crate::platform::windows::windows_pdf_dialog::show_pdf_save_dialog,
            );
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            self.finish_pdf_export(pdf::PdfExportPickerResult::Error(
                pdf::PdfExportPickerFailure::UnsupportedPlatform,
            ));
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn export_pdf_with_picker(
        &mut self,
        show_picker: impl FnOnce(&str) -> pdf::PdfExportPickerResult,
    ) {
        let default_filename = pdf_export_filename(self.native_path.as_deref());
        let picker_result = show_picker(&default_filename);
        self.finish_pdf_export(picker_result);
    }

    pub(crate) fn export_pdf_with_options(
        &mut self,
        options: scriptcraft_commands::PdfExportOptions,
    ) {
        let default_filename = pdf_export_filename(self.native_path.as_deref());
        let picker_result = rfd::FileDialog::new()
            .add_filter("PDF document", &["pdf"])
            .set_file_name(&default_filename)
            .save_file()
            .map_or(pdf::PdfExportPickerResult::Cancel, |destination| {
                pdf::PdfExportPickerResult::Accept {
                    destination,
                    options,
                }
            });

        self.finish_pdf_export(picker_result);
    }

    fn finish_pdf_export(&mut self, picker_result: pdf::PdfExportPickerResult) {
        match export_pdf_to_selected_path(picker_result, &StdFileSystem, |selected_options| {
            self.build_pdf_with_options(selected_options)
        }) {
            Ok(false) => {}
            Ok(true) => self.clear_error(),
            Err(PdfExportAdapterError::Picker(pdf::PdfExportPickerFailure::Unavailable)) => {
                self.show_error(
                    "PDF_EXPORT_PICKER_UNAVAILABLE: destination picker unavailable; screenplay unchanged",
                )
            }
            Err(PdfExportAdapterError::Picker(pdf::PdfExportPickerFailure::Failed)) => {
                self.show_error(
                    "PDF_EXPORT_PICKER_FAILED: destination selection failed; screenplay unchanged",
                )
            }
            Err(PdfExportAdapterError::Picker(
                pdf::PdfExportPickerFailure::UnsupportedPlatform,
            )) => self.show_error(
                "PDF_EXPORT_UNSUPPORTED_PLATFORM: native PDF destination selection is unsupported on this platform; screenplay unchanged",
            ),
            Err(PdfExportAdapterError::Path) => self.show_error(
                "PDF_EXPORT_PATH_REJECTED: selected path is not valid UTF-8; screenplay unchanged",
            ),
            Err(PdfExportAdapterError::Write(_)) => self.show_error(
                "PDF_EXPORT_WRITE_FAILED: filesystem write failed; screenplay unchanged; export output may be incomplete",
            ),
            Err(PdfExportAdapterError::Export(exports::ExportError::Layout(error))) => {
                self.show_error(format!(
                    "PDF_EXPORT_LAYOUT_FAILED: {}; screenplay unchanged; no complete export produced",
                    crate::app::diagnostics::safe_layout_error_category(&error)
                ));
            }
        }
    }

    fn build_pdf_with_options(
        &self,
        options: scriptcraft_commands::PdfExportOptions,
    ) -> Result<Vec<u8>, exports::ExportError> {
        print::build_pdf_bytes(
            self.session.project(),
            self.session.layout_inputs(),
            options,
        )
    }

    fn build_print_request(
        &self,
        options: scriptcraft_commands::PdfExportOptions,
    ) -> Result<print::PrintRequest, exports::ExportError> {
        print::build_print_request(
            self.session.project(),
            self.session.layout_inputs(),
            options,
            self.native_path.as_deref(),
        )
    }

    pub(crate) fn open_print_panel(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.accepted_print_settings = None;
            let snapshot = print::PrintCompositionSnapshot::capture(
                self.session.project(),
                self.session.layout_inputs(),
                self.native_path.as_deref(),
            );
            let panel_result = crate::platform::macos::macos_print_dialog::show_print_panel(snapshot);
            let delivery = crate::platform::macos::macos_print_dialog::dispatch_panel_result(panel_result, |settings| {
                self.accepted_print_settings = Some(settings);
                match self.accepted_print_settings.take() {
                    Some(settings) => crate::platform::macos::macos_print_dialog::deliver_print(settings),
                    None => crate::platform::macos::macos_print_dialog::PrintDeliveryResult::PrintOperationUnavailable,
                }
            });
            match delivery {
                crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Cancelled => self.clear_error(),
                    crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Accepted(
                    crate::platform::macos::macos_print_dialog::PrintDeliveryResult::OperationReportedSuccess,
                ) => self.show_error(
                    "PRINT_OPERATION_REPORTED_SUCCESS: macOS reported successful operation; physical printer delivery is unconfirmed; screenplay unchanged",
                ),
                    crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Accepted(
                    crate::platform::macos::macos_print_dialog::PrintDeliveryResult::OperationCancelledOrFailed,
                ) => self.show_error(
                    "PRINT_OPERATION_CANCELLED_OR_FAILED: native print operation was cancelled or failed; AppKit does not distinguish these results; printer delivery is unknown; screenplay unchanged",
                ),
                    crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Accepted(
                    crate::platform::macos::macos_print_dialog::PrintDeliveryResult::PdfBuildFailed,
                ) => self.show_error(
                    "PRINT_PDF_BUILD_FAILED: Print-composed PDF could not be built; no operation submitted; screenplay unchanged",
                ),
                    crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Accepted(
                    crate::platform::macos::macos_print_dialog::PrintDeliveryResult::PdfDocumentInvalid,
                ) => self.show_error(
                    "PRINT_PDF_INVALID: PDFKit could not open the generated PDF; no operation submitted; screenplay unchanged",
                ),
                    crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Accepted(
                    crate::platform::macos::macos_print_dialog::PrintDeliveryResult::MainThreadUnavailable,
                ) => self.show_error(
                    "PRINT_OPERATION_UNAVAILABLE: PDFKit printing requires the AppKit main thread; no operation submitted; screenplay unchanged",
                ),
                    crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Accepted(
                    crate::platform::macos::macos_print_dialog::PrintDeliveryResult::PrintOperationUnavailable,
                ) => self.show_error(
                    "PRINT_OPERATION_UNAVAILABLE: PDFKit could not prepare a native print operation; no operation submitted; screenplay unchanged",
                ),
                crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Unavailable => self.show_error(
                    "PRINT_PANEL_UNAVAILABLE: native print panel unavailable; screenplay unchanged",
                ),
                crate::platform::macos::macos_print_dialog::PrintPanelDispatch::Failed => self.show_error(
                    "PRINT_PANEL_FAILED: native print settings could not be collected; screenplay unchanged",
                ),
            }
        }
        #[cfg(target_os = "windows")]
        {
            // Capture all mutable editor inputs synchronously at command
            // acceptance. Only detached project/layout data enters the worker;
            // the real owner HWND is acquired later from the live eframe frame.
            let request = WindowsPrintRequestSnapshot::capture(self);
            match crate::platform::windows::windows_print::PrintPreparation::start(
                request.project,
                request.layout_inputs,
                request.revision,
                request.job_title,
            ) {
                Ok(preparation) => {
                    self.windows_print_preparation = Some(preparation);
                    self.show_error("WINDOWS_PRINT_PREPARING: immutable Print layouts are being prepared off the UI thread");
                }
                Err(_) => {
                    self.native_output_modal_active = false;
                    self.show_error("WINDOWS_PRINT_FAILED: bounded immutable print preparation could not start; no system UI was opened");
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
struct WindowsPrintRequestSnapshot {
    project: Project,
    layout_inputs: scriptcraft_model::LayoutInputs,
    revision: u64,
    job_title: String,
}

#[cfg(target_os = "windows")]
impl WindowsPrintRequestSnapshot {
    fn capture(app: &ScriptCraftApp) -> Self {
        Self {
            project: app.session.project().clone(),
            layout_inputs: app.session.layout_inputs().clone(),
            revision: app.session.revision(),
            job_title: print::print_job_title(app.native_path.as_deref()),
        }
    }
}

#[cfg(target_os = "windows")]
impl ScriptCraftApp {
    /// Poll background preparation and native callbacks without blocking the
    /// eframe loop. The current frame is the sole source of the owner HWND.
    pub(crate) fn update_windows_print_session(
        &mut self,
        ui: &mut egui::Ui,
        frame: &eframe::Frame,
    ) {
        let ctx = ui.ctx().clone();
        let submitted = self
            .windows_print_session
            .as_mut()
            .is_some_and(crate::platform::windows::windows_print::PrintSession::take_submitted);
        if submitted {
            self.show_error("WINDOWS_PRINT_SUBMITTED: Windows accepted the captured screenplay request; physical delivery is unconfirmed");
        }

        let terminal = self
            .windows_print_session
            .as_mut()
            .and_then(crate::platform::windows::windows_print::PrintSession::poll);
        if let Some(result) = terminal {
            let message = match result {
                crate::platform::windows::windows_print::PrintResult::Submitted => "WINDOWS_PRINT_SUBMITTED: system reported submission; physical delivery is unconfirmed",
                crate::platform::windows::windows_print::PrintResult::Completed => "WINDOWS_PRINT_COMPLETED: native task reached its terminal callback; physical delivery is unconfirmed",
                crate::platform::windows::windows_print::PrintResult::Cancelled => "WINDOWS_PRINT_CANCELLED: system print task was cancelled before completion",
                crate::platform::windows::windows_print::PrintResult::Failed => "WINDOWS_PRINT_FAILED: native print session failed or stopped reporting after a lifecycle callback",
                crate::platform::windows::windows_print::PrintResult::Unsupported => "WINDOWS_PRINT_UNSUPPORTED: modern system print session is unavailable",
            };
            self.show_error(message);
            self.finish_windows_print_session(&ctx);
        }

        let prepared = self
            .windows_print_preparation
            .as_ref()
            .and_then(crate::platform::windows::windows_print::PrintPreparation::take_result);
        if let Some(prepared) = prepared {
            self.windows_print_preparation = None;
            match prepared {
                Ok(packages) => match crate::platform::windows::windows_print::PrintSession::start(frame, packages) {
                    Ok(session) => {
                        self.windows_print_session = Some(session);
                        self.show_error("WINDOWS_PRINT_PREVIEW_REQUESTED: modern Windows print UI opened for the immutable request-time snapshot; later edits require a new request");
                    }
                    Err(failure) => {
                        self.report_windows_print_start_failure(failure);
                        self.finish_windows_print_session(&ctx);
                    }
                },
                Err(failure) => {
                    self.report_windows_print_start_failure(failure);
                    self.finish_windows_print_session(&ctx);
                }
            }
        }

        if self.windows_print_session.is_some() || self.windows_print_preparation.is_some() {
            // Session events are delivered from native callbacks and do not
            // necessarily wake egui, so schedule bounded nonblocking polling.
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn finish_windows_print_session(&mut self, ctx: &egui::Context) {
        self.windows_print_session = None;
        self.windows_print_preparation = None;
        self.native_output_modal_active = false;
        self.restore_editor_focus(ctx);
    }

    fn report_windows_print_start_failure(&mut self, failure: crate::platform::windows::windows_print::StartFailure) {
        let message = match failure {
            crate::platform::windows::windows_print::StartFailure::UnsupportedWindow => {
                "WINDOWS_PRINT_UNSUPPORTED: eframe did not expose a Win32 owner window"
            }
            crate::platform::windows::windows_print::StartFailure::ApartmentUnavailable => {
                "WINDOWS_PRINT_UNSUPPORTED: UI-thread WinRT apartment could not be initialized"
            }
            crate::platform::windows::windows_print::StartFailure::ManagerUnavailable => {
                "WINDOWS_PRINT_UNSUPPORTED: PrintManager interop for the owner HWND is unavailable"
            }
            crate::platform::windows::windows_print::StartFailure::RegistrationFailed => {
                "WINDOWS_PRINT_FAILED: PrintTaskRequested registration failed"
            }
            crate::platform::windows::windows_print::StartFailure::UiUnavailable => {
                "WINDOWS_PRINT_FAILED: modern system print UI could not be opened"
            }
            crate::platform::windows::windows_print::StartFailure::PreparationFailed => {
                "WINDOWS_PRINT_FAILED: immutable screenplay package preparation failed"
            }
        };
        self.show_error(message);
    }
}
