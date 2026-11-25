//! Desktop PDF export and native Print coordination.

use super::{pdf, print};
use crate::app::ScriptCraftApp;
use eframe::egui;
use scriptcraft_export as exports;
use crate::presentation::document_font::PreparedDocumentFonts;
use scriptcraft_layout::LayoutEnvironment;
use scriptcraft_model::typography::ResolvedFontSourceKind;
use scriptcraft_session::persistence::{FileSystem, StdFileSystem};
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use scriptcraft_model::Project;

const UNTITLED_PDF_FILENAME: &str = "Untitled Screenplay.pdf";
const UNTITLED_FOUNTAIN_FILENAME: &str = "Untitled Screenplay.fountain";

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

fn fountain_export_filename(active_path: Option<&Path>) -> String {
    let Some(file_name) = active_path
        .and_then(Path::file_name)
        .filter(|file_name| !file_name.is_empty())
        .and_then(std::ffi::OsStr::to_str)
    else {
        return UNTITLED_FOUNTAIN_FILENAME.to_owned();
    };

    let mut projected = PathBuf::from(file_name);
    if !projected.set_extension("fountain") {
        return UNTITLED_FOUNTAIN_FILENAME.to_owned();
    }

    projected
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|file_name| !file_name.is_empty())
        .unwrap_or(UNTITLED_FOUNTAIN_FILENAME)
        .to_owned()
}

#[derive(Debug)]
enum PdfExportAdapterError {
    Picker(pdf::PdfExportPickerFailure),
    Path,
    Export(exports::ExportError),
    Write(scriptcraft_session::persistence::PersistenceError),
}

#[derive(Debug)]
enum FountainExportFailure {
    Export(exports::ExportError),
    Write,
}

fn export_pdf_to_selected_path<F: FileSystem>(
    picker_result: pdf::PdfExportPickerResult,
    filesystem: &F,
    build_pdf: impl FnOnce(
        scriptcraft_commands::PdfExportOptions,
    ) -> Result<exports::PdfExport, exports::ExportError>,
) -> Result<Option<exports::PdfTypographyReport>, PdfExportAdapterError> {
    let (path, options) = match picker_result {
        pdf::PdfExportPickerResult::Cancel => return Ok(None),
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
    let export = build_pdf(options).map_err(PdfExportAdapterError::Export)?;
    filesystem
        .write(&path, &export.bytes)
        .map_err(PdfExportAdapterError::Write)?;
    Ok(Some(export.typography))
}

impl ScriptCraftApp {
    pub(crate) fn export_fountain(&mut self) {
        let default_filename = fountain_export_filename(self.native_path.as_deref());
        #[cfg(target_os = "macos")]
        {
            use crate::platform::macos::macos_native_dialogs::{
                NativeFilePanelContinuation, NativeFilePanelRequest,
            };
            if self
                .native_operation_coordinator
                .present_file_panel(NativeFilePanelRequest {
                    continuation: NativeFilePanelContinuation::FountainExport,
                    title: "Export Fountain Text",
                    default_filename: &default_filename,
                    allowed_extensions: &["fountain"],
                    include_title_page: false,
                })
                .is_err()
            {
                self.show_error("FOUNTAIN_EXPORT_PICKER_UNAVAILABLE: destination picker could not be presented; screenplay unchanged");
            }
            return;
        }
        #[cfg(not(target_os = "macos"))]
        {
        let Some(destination) = rfd::FileDialog::new()
            .add_filter("Fountain screenplay", &["fountain"])
            .set_file_name(&default_filename)
            .save_file()
        else {
            return;
        };

        let result = exports::export_fountain(self.session.project())
            .map_err(FountainExportFailure::Export)
            .and_then(|text| {
                StdFileSystem
                    .write(&destination, text.as_bytes())
                    .map_err(|_| FountainExportFailure::Write)
        });
        self.finish_fountain_export(result);
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn finish_fountain_export_to(&mut self, destination: PathBuf) {
        if destination.to_str().is_none() {
            self.show_error("FOUNTAIN_EXPORT_PATH_REJECTED: selected path is not valid UTF-8; screenplay unchanged");
            return;
        }
        let result = exports::export_fountain(self.session.project())
            .map_err(FountainExportFailure::Export)
            .and_then(|text| {
                StdFileSystem
                    .write(&destination, text.as_bytes())
                    .map_err(|_| FountainExportFailure::Write)
            });
        self.finish_fountain_export(result);
    }

    fn finish_fountain_export(&mut self, result: Result<(), FountainExportFailure>) {
        match result {
            Ok(()) => self.clear_error(),
            Err(FountainExportFailure::Export(exports::ExportError::InvalidFountainSource)) => {
                self.show_error(
                    "FOUNTAIN_EXPORT_SOURCE_INVALID: screenplay content could not be serialized; screenplay unchanged; no complete export produced",
                );
            }
            Err(FountainExportFailure::Export(
                exports::ExportError::OutputResourceLimitExceeded,
            )) => self.show_error(
                "FOUNTAIN_EXPORT_RESOURCE_LIMIT: bounded Fountain output resources were exceeded; screenplay unchanged; no complete export produced",
            ),
            Err(FountainExportFailure::Write) => self.show_error(
                "FOUNTAIN_EXPORT_WRITE_FAILED: filesystem write failed; screenplay unchanged; export output may be incomplete",
            ),
            Err(FountainExportFailure::Export(_)) => self.show_error(
                "FOUNTAIN_EXPORT_FAILED: Fountain export could not be completed; screenplay unchanged; no complete export produced",
            ),
        }
    }

    pub(crate) fn export_pdf(&mut self) {
        #[cfg(target_os = "macos")]
        {
            use crate::platform::macos::macos_native_dialogs::{
                NativeFilePanelContinuation, NativeFilePanelRequest,
            };
            let default_filename = pdf_export_filename(self.native_path.as_deref());
            if self
                .native_operation_coordinator
                .present_file_panel(NativeFilePanelRequest {
                    continuation: NativeFilePanelContinuation::PdfExport,
                    title: "Save As PDF",
                    default_filename: &default_filename,
                    allowed_extensions: &["pdf"],
                    include_title_page: true,
                })
                .is_err()
            {
                self.finish_pdf_export(pdf::PdfExportPickerResult::Error(
                    pdf::PdfExportPickerFailure::Unavailable,
                ));
            }
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
        #[cfg(target_os = "macos")]
        {
            use crate::platform::macos::macos_native_dialogs::{
                NativeFilePanelContinuation, NativeFilePanelRequest,
            };
            let default_filename = pdf_export_filename(self.native_path.as_deref());
            if self
                .native_operation_coordinator
                .present_file_panel(NativeFilePanelRequest {
                    continuation: NativeFilePanelContinuation::PdfExportWithOptions(options),
                    title: "Save As PDF",
                    default_filename: &default_filename,
                    allowed_extensions: &["pdf"],
                    include_title_page: false,
                })
                .is_err()
            {
                self.finish_pdf_export(pdf::PdfExportPickerResult::Error(
                    pdf::PdfExportPickerFailure::Unavailable,
                ));
            }
            return;
        }
        #[cfg(not(target_os = "macos"))]
        {
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
    }

    pub(crate) fn finish_pdf_export(&mut self, picker_result: pdf::PdfExportPickerResult) {
        match export_pdf_to_selected_path(picker_result, &StdFileSystem, |selected_options| {
            self.build_pdf_with_options(selected_options)
        }) {
            Ok(None) => {}
            Ok(Some(report)) if report.is_empty() => self.clear_error(),
            Ok(Some(report)) => self.show_error(format!(
                "PDF_EXPORT_COMPLETED: {}; screenplay unchanged",
                report.summary()
            )),
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
            Err(PdfExportAdapterError::Export(
                exports::ExportError::InvalidTypographyEnvironment,
            )) => self.show_error(
                "PDF_EXPORT_TYPOGRAPHY_UNAVAILABLE: resolved typography environment was incomplete or mismatched; screenplay unchanged; no complete export produced",
            ),
            Err(PdfExportAdapterError::Export(exports::ExportError::InvalidLayoutGeometry)) => {
                self.show_error(
                    "PDF_EXPORT_GEOMETRY_INVALID: canonical layout geometry was inconsistent; screenplay unchanged; no complete export produced",
                );
            }
            Err(PdfExportAdapterError::Export(exports::ExportError::InvalidPdfFontPayload)) => {
                self.show_error(
                    "PDF_EXPORT_FONT_PAYLOAD_INVALID: exact resolved font data was incomplete or inconsistent; screenplay unchanged; no complete export produced",
                );
            }
            Err(PdfExportAdapterError::Export(
                exports::ExportError::RequiredPackagedFontEmbedding,
            )) => self.show_error(
                "PDF_EXPORT_PACKAGED_FONT_UNAVAILABLE: required packaged font data could not be embedded under the bounded policy; screenplay unchanged; no complete export produced",
            ),
            Err(PdfExportAdapterError::Export(
                exports::ExportError::OutputResourceLimitExceeded,
            )) => self.show_error(
                "PDF_EXPORT_RESOURCE_LIMIT: bounded PDF output resources were exceeded; screenplay unchanged; no complete export produced",
            ),
            Err(PdfExportAdapterError::Export(exports::ExportError::InvalidFountainSource)) => {
                self.show_error(
                    "PDF_EXPORT_SOURCE_INVALID: export source data was invalid; screenplay unchanged; no complete export produced",
                );
            }
        }
    }

    fn build_pdf_with_options(
        &mut self,
        options: scriptcraft_commands::PdfExportOptions,
    ) -> Result<exports::PdfExport, exports::ExportError> {
        let (environment, font_payload) = self.output_pdf_inputs()?;
        print::build_pdf_bytes(
            self.session.project(),
            self.session.layout_inputs(),
            &environment,
            &font_payload,
            options,
        )
    }

    fn output_layout_environment(&mut self) -> Result<LayoutEnvironment, exports::ExportError> {
        let prepared = self.output_prepared_document_fonts()?;
        Ok(self.layout_environment_from_prepared(&prepared))
    }

    /// Couples PDF-only exact bytes with the environment from one prepared
    /// snapshot. Windows receives environment only, and plain text remains
    /// intentionally lossy and environment-only.
    fn output_pdf_inputs(
        &mut self,
    ) -> Result<(LayoutEnvironment, exports::PdfFontPayload), exports::ExportError> {
        let prepared = self.output_prepared_document_fonts()?;
        let environment = self.layout_environment_from_prepared(&prepared);
        let payload = Self::pdf_font_payload_from_prepared(&prepared)?;
        Ok((environment, payload))
    }

    fn output_prepared_document_fonts(
        &mut self,
    ) -> Result<PreparedDocumentFonts, exports::ExportError> {
        let prepared = self
            .document_font_store
            .prepare_document(self.session.project())
            .map_err(|_| exports::ExportError::InvalidTypographyEnvironment)?;
        self.prepared_document_fonts = Some(prepared.clone());
        Ok(prepared)
    }

    fn layout_environment_from_prepared(
        &self,
        prepared: &PreparedDocumentFonts,
    ) -> LayoutEnvironment {
        LayoutEnvironment::new(
            self.session.layout_inputs().page_only(),
            prepared.environment.clone(),
        )
    }

    fn pdf_font_payload_from_prepared(
        prepared: &PreparedDocumentFonts,
    ) -> Result<exports::PdfFontPayload, exports::ExportError> {
        let mut entries = Vec::new();
        for id in prepared.resolved_ids() {
            let face = prepared
                .environment
                .require_face(id)
                .map_err(|_| exports::ExportError::InvalidPdfFontPayload)?;
            if face.source_kind == ResolvedFontSourceKind::Bundled
                && !prepared.is_exact_packaged_courier_prime(id)
            {
                return Err(exports::ExportError::RequiredPackagedFontEmbedding);
            }
            let bytes = prepared
                .face_bytes(id)
                .ok_or(exports::ExportError::InvalidPdfFontPayload)?;
            let face_index = prepared
                .face_index(id)
                .ok_or(exports::ExportError::InvalidPdfFontPayload)?;
            entries.push((
                id.clone(),
                exports::PdfResolvedFaceBytes {
                    face: face.clone(),
                    bytes: bytes.clone(),
                    face_index,
                },
            ));
        }
        exports::PdfFontPayload::new(entries)
    }

    pub(crate) fn open_print_panel(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let (environment, font_payload) = match self.output_pdf_inputs() {
                Ok(inputs) => inputs,
                Err(_) => {
                    self.show_error(
                        "PRINT_TYPOGRAPHY_UNAVAILABLE: resolved typography environment could not be prepared; no print panel opened",
                    );
                    return;
                }
            };
            let snapshot = print::PrintCompositionSnapshot::capture(
                self.session.project(),
                self.session.layout_inputs(),
                &environment,
                font_payload,
                self.native_path.as_deref(),
            );
            if self
                .native_operation_coordinator
                .present_print_panel(snapshot)
                .is_err()
            {
                self.show_error(
                    "PRINT_PANEL_UNAVAILABLE: native print panel unavailable; screenplay unchanged",
                );
            }
        }
        #[cfg(target_os = "windows")]
        {
            // Capture all mutable editor inputs synchronously at command
            // acceptance. Only detached project/layout data enters the worker;
            // the real owner HWND is acquired later from the live eframe frame.
            let request = match WindowsPrintRequestSnapshot::capture(self) {
                Ok(request) => request,
                Err(_) => {
                    self.native_output_modal_active = false;
                    self.show_error(
                        "WINDOWS_PRINT_TYPOGRAPHY_UNAVAILABLE: resolved typography environment could not be prepared or did not match Project-derived page inputs; no system print UI was opened; screenplay unchanged",
                    );
                    return;
                }
            };
            match crate::platform::windows::windows_print::PrintPreparation::start(
                request.project,
                request.layout_inputs,
                request.layout_environment,
                request.revision,
                request.job_title,
            ) {
                Ok(preparation) => {
                    self.windows_print_preparation = Some(preparation);
                    self.show_error("WINDOWS_PRINT_PREPARING: immutable Print layouts are being prepared off the UI thread");
                }
                Err(failure) => {
                    self.native_output_modal_active = false;
                    self.report_windows_print_start_failure(failure);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn finish_print_delivery(
        &mut self,
        outcome: crate::platform::macos::macos_print_dialog::PrintDeliveryOutcome,
    ) {
        use crate::platform::macos::macos_print_dialog::PrintDeliveryResult;

        let message = match outcome.result {
            PrintDeliveryResult::OperationReportedSuccess => "PRINT_OPERATION_REPORTED_SUCCESS: macOS reported successful operation; physical printer delivery is unconfirmed; screenplay unchanged",
            PrintDeliveryResult::OperationCancelledOrFailed => "PRINT_OPERATION_CANCELLED_OR_FAILED: native print operation was cancelled or failed; AppKit does not distinguish these results; printer delivery is unknown; screenplay unchanged",
            PrintDeliveryResult::PdfBuildFailed => "PRINT_PDF_BUILD_FAILED: Print-composed PDF could not be built; no operation submitted; screenplay unchanged",
            PrintDeliveryResult::PdfDocumentInvalid => "PRINT_PDF_INVALID: PDFKit could not open the generated PDF; no operation submitted; screenplay unchanged",
            PrintDeliveryResult::MainThreadUnavailable => "PRINT_OPERATION_UNAVAILABLE: PDFKit printing requires the AppKit main thread; no operation submitted; screenplay unchanged",
            PrintDeliveryResult::PrintOperationUnavailable => "PRINT_OPERATION_UNAVAILABLE: PDFKit could not prepare a native print operation; no operation submitted; screenplay unchanged",
        };
        if let Some(summary) = outcome.fallback_summary {
            self.show_error(format!("{message}; {summary}"));
        } else {
            self.show_error(message);
        }
    }
}

#[cfg(target_os = "windows")]
struct WindowsPrintRequestSnapshot {
    project: Project,
    layout_inputs: scriptcraft_model::LayoutInputs,
    layout_environment: LayoutEnvironment,
    revision: u64,
    job_title: String,
}

#[cfg(target_os = "windows")]
impl WindowsPrintRequestSnapshot {
    fn capture(app: &mut ScriptCraftApp) -> Result<Self, exports::ExportError> {
        let project = app.session.project().clone();
        let layout_inputs = project.session_layout_inputs();
        let layout_environment = app.output_layout_environment()?;
        if layout_inputs.page_only() != layout_environment.page {
            return Err(exports::ExportError::InvalidTypographyEnvironment);
        }
        Ok(Self {
            project,
            layout_inputs,
            layout_environment,
            revision: app.session.revision(),
            job_title: print::print_job_title(app.native_path.as_deref()),
        })
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
            self.show_error("WINDOWS_PRINT_SUBMITTED: Windows accepted the captured screenplay request; physical delivery is unconfirmed; visual parity is not claimed");
        }

        let terminal = self
            .windows_print_session
            .as_mut()
            .and_then(crate::platform::windows::windows_print::PrintSession::poll);
        if let Some(result) = terminal {
            let message = match result {
                crate::platform::windows::windows_print::PrintResult::Submitted => "WINDOWS_PRINT_SUBMITTED: system reported submission; physical delivery is unconfirmed; visual parity is not claimed",
                crate::platform::windows::windows_print::PrintResult::Completed => "WINDOWS_PRINT_COMPLETED: native task reached its terminal callback; physical delivery is unconfirmed; visual parity is not claimed",
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
                Ok(packages) => {
                    let typography_summary = packages.typography_report_summary();
                    match crate::platform::windows::windows_print::PrintSession::start(
                        frame, packages,
                    ) {
                        Ok(session) => {
                            self.windows_print_session = Some(session);
                            let message = "WINDOWS_PRINT_PREVIEW_REQUESTED: modern Windows print UI opened for the immutable request-time snapshot; later edits require a new request; physical delivery is unconfirmed and visual parity is not claimed";
                            if let Some(summary) = typography_summary {
                                self.show_error(format!("{message}; {summary}"));
                            } else {
                                self.show_error(message);
                            }
                        }
                        Err(failure) => {
                            self.report_windows_print_start_failure(failure);
                            self.finish_windows_print_session(&ctx);
                        }
                    }
                }
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

    fn report_windows_print_start_failure(
        &mut self,
        failure: crate::platform::windows::windows_print::StartFailure,
    ) {
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
                "WINDOWS_PRINT_FAILED: immutable screenplay package preparation failed; no system UI opened; screenplay unchanged"
            }
            crate::platform::windows::windows_print::StartFailure::LayoutFailure => {
                "WINDOWS_PRINT_LAYOUT_FAILED: canonical Print composition failed; no system UI opened; screenplay unchanged"
            }
            crate::platform::windows::windows_print::StartFailure::InvalidTypographyEnvironment => {
                "WINDOWS_PRINT_TYPOGRAPHY_UNAVAILABLE: prepared typography or Project-derived page inputs were incomplete or mismatched; no system UI opened; screenplay unchanged"
            }
            crate::platform::windows::windows_print::StartFailure::InvalidCanonicalGeometry => {
                "WINDOWS_PRINT_GEOMETRY_INVALID: canonical line or span geometry was inconsistent; no system UI opened; screenplay unchanged"
            }
            crate::platform::windows::windows_print::StartFailure::ResourceFailure => {
                "WINDOWS_PRINT_RESOURCE_LIMIT: immutable Print package resource or allocation bound was exceeded; no system UI opened; screenplay unchanged"
            }
        };
        self.show_error(message);
    }
}
