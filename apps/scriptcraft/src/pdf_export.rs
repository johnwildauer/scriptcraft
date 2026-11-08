use eframe::egui;
use scriptcraft_commands::PdfExportOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PdfExportAction {
    Continue(PdfExportOptions),
    Cancel,
}

/// Transient app-owned pre-picker step. Every request starts from the typed
/// options default, so a previous choice never leaks into the next export.
#[derive(Debug, Default)]
pub(super) struct PdfExportController {
    pending: Option<PdfExportOptions>,
}

impl PdfExportController {
    pub(super) fn begin(&mut self) {
        self.pending = Some(PdfExportOptions::default());
    }

    pub(super) fn is_open(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn cancel(&mut self) -> bool {
        self.pending.take().is_some()
    }

    pub(super) fn show(&mut self, ctx: &egui::Context) -> Option<PdfExportAction> {
        let mut options = self.pending?;
        let mut window_open = true;
        let mut action = None;
        egui::Window::new("Save As PDF")
            .id(egui::Id::new("pdf-export-options"))
            .collapsible(false)
            .resizable(false)
            .open(&mut window_open)
            .show(ctx, |ui| {
                ui.checkbox(&mut options.include_title_page, "Include title page");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Continue").clicked() {
                        action = Some(PdfExportAction::Continue(options));
                    }
                    if ui.button("Cancel").clicked() {
                        action = Some(PdfExportAction::Cancel);
                    }
                });
            });

        if !window_open && action.is_none() {
            action = Some(PdfExportAction::Cancel);
        }
        if action.is_some() {
            self.pending = None;
        } else {
            self.pending = Some(options);
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::PdfExportController;
    use scriptcraft_commands::PdfExportOptions;

    #[test]
    fn each_export_request_starts_with_title_page_enabled() {
        let mut controller = PdfExportController::default();
        controller.begin();
        assert_eq!(controller.pending, Some(PdfExportOptions::default()));

        controller.pending = Some(PdfExportOptions {
            include_title_page: false,
        });
        assert!(controller.cancel());
        assert!(!controller.is_open());

        controller.begin();
        assert_eq!(controller.pending, Some(PdfExportOptions::default()));
    }

    #[test]
    fn cancel_closes_the_options_step_without_producing_export_options() {
        let mut controller = PdfExportController::default();
        controller.begin();
        assert!(controller.cancel());
        assert!(!controller.is_open());
        assert!(!controller.cancel());
    }
}
