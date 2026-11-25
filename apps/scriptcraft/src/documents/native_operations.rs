//! App-level polling seam for the shared macOS native-operation coordinator.

use crate::app::ScriptCraftApp;
use crate::documents::{lifecycle::SaveOutcome, pdf::PdfExportPickerResult};
use crate::platform::macos::macos_native_dialogs::{
    CompletedNativeOperation, NativeFilePanelContinuation, NativeFilePanelSelection,
    NativeOperationKind,
};
use eframe::egui;

impl ScriptCraftApp {
    /// Drain the bounded native completion before ordinary pending commands.
    /// Accepted destinations are inspected and resumed here on the main thread;
    /// native callbacks only enqueue a scoped primitive response.
    pub(crate) fn poll_native_operation_completion(&mut self, ctx: &egui::Context) {
        if let Some(mut completed) = self
            .native_operation_coordinator
            .take_matching_completion()
        {
            self.command_projection_cache = None;
            match completed.operation.kind {
                NativeOperationKind::PrintPanel => {
                    self.resume_print_panel_completion(completed);
                }
                NativeOperationKind::PrintDelivery => {
                    if let Some(outcome) = completed.take_print_delivery_outcome() {
                        self.finish_print_delivery(outcome);
                    }
                }
                _ => self.resume_file_panel_completion(completed, ctx),
            }
        }
    }

    fn resume_print_panel_completion(&mut self, mut completed: CompletedNativeOperation) {
        use crate::platform::macos::macos_print_dialog::PrintPanelCompletion;

        match completed.take_print_panel_completion() {
            Some(PrintPanelCompletion::Cancelled) => self.clear_error(),
            Some(PrintPanelCompletion::Accepted(settings)) => {
                if let Err(outcome) = self.native_operation_coordinator.start_print_delivery(settings) {
                    self.finish_print_delivery(outcome);
                }
            }
            Some(PrintPanelCompletion::Failed) | None => self.show_error(
                "PRINT_PANEL_FAILED: native print settings could not be collected; screenplay unchanged",
            ),
        }
    }

    fn resume_file_panel_completion(
        &mut self,
        mut completed: CompletedNativeOperation,
        ctx: &egui::Context,
    ) {
        let Some((continuation, selection)) = completed.take_file_panel_selection() else {
            return;
        };
        match (continuation, selection) {
            (NativeFilePanelContinuation::Open, NativeFilePanelSelection::Accepted { destination, .. }) => {
                self.dispatch_document_path_command(&destination, ctx);
            }
            (NativeFilePanelContinuation::Open, NativeFilePanelSelection::Cancelled) => {}
            (NativeFilePanelContinuation::Open, NativeFilePanelSelection::Failed) => self.show_error(
                "OPEN_PICKER_FAILED: selected document could not be obtained; active document unchanged",
            ),
            (NativeFilePanelContinuation::SaveAs, NativeFilePanelSelection::Accepted { destination, .. }) => {
                let outcome = self.dispatch_save_as_path(&destination, true, ctx);
                self.finish_pending_lifecycle_save(outcome);
            }
            (NativeFilePanelContinuation::SaveAs, NativeFilePanelSelection::Cancelled) => {
                self.finish_pending_lifecycle_save(SaveOutcome::Cancelled);
            }
            (NativeFilePanelContinuation::SaveAs, NativeFilePanelSelection::Failed) => {
                self.show_error("SAVE_AS_PICKER_FAILED: selected destination could not be obtained; document unchanged");
                self.finish_pending_lifecycle_save(SaveOutcome::Failed);
            }
            (NativeFilePanelContinuation::FountainExport, NativeFilePanelSelection::Accepted { destination, .. }) => {
                self.finish_fountain_export_to(destination);
            }
            (NativeFilePanelContinuation::FountainExport, NativeFilePanelSelection::Cancelled) => {}
            (NativeFilePanelContinuation::FountainExport, NativeFilePanelSelection::Failed) => self.show_error(
                "FOUNTAIN_EXPORT_PICKER_FAILED: destination selection failed; screenplay unchanged",
            ),
            (NativeFilePanelContinuation::PdfExport, NativeFilePanelSelection::Accepted { destination, include_title_page }) => {
                if let Some(include_title_page) = include_title_page {
                    self.finish_pdf_export(PdfExportPickerResult::Accept {
                        destination,
                        options: scriptcraft_commands::PdfExportOptions { include_title_page },
                    });
                } else {
                    self.finish_pdf_export(PdfExportPickerResult::Error(
                        crate::documents::pdf::PdfExportPickerFailure::Failed,
                    ));
                }
            }
            (NativeFilePanelContinuation::PdfExportWithOptions(options), NativeFilePanelSelection::Accepted { destination, .. }) => {
                self.finish_pdf_export(PdfExportPickerResult::Accept { destination, options });
            }
            (NativeFilePanelContinuation::PdfExport | NativeFilePanelContinuation::PdfExportWithOptions(_), NativeFilePanelSelection::Cancelled) => {
                self.finish_pdf_export(PdfExportPickerResult::Cancel);
            }
            (NativeFilePanelContinuation::PdfExport | NativeFilePanelContinuation::PdfExportWithOptions(_), NativeFilePanelSelection::Failed) => {
                self.finish_pdf_export(PdfExportPickerResult::Error(
                    crate::documents::pdf::PdfExportPickerFailure::Failed,
                ));
            }
        }
    }
}
