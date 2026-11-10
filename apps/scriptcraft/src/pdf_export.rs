use scriptcraft_commands::PdfExportOptions;
use std::path::PathBuf;

/// Desktop-local result of obtaining a PDF destination and its request options.
/// Adapter errors are categories only: they must never retain paths or document text.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum PdfExportPickerResult {
    Cancel,
    Accept {
        destination: PathBuf,
        options: PdfExportOptions,
    },
    Error(PdfExportPickerFailure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PdfExportPickerFailure {
    Unavailable,
    Failed,
    UnsupportedPlatform,
}
