#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Explicit, non-canonical plain-text and PDF export adapters.
//!
//! Both adapters consume shared Print layout and only borrow the project, so
//! exporting cannot change the editing session or its clean state.

mod pdf;
mod text;

use scriptcraft_layout::LayoutError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Layout(LayoutError),
}

impl From<LayoutError> for ExportError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

pub use pdf::{
    export_pdf, export_pdf_with_layout_inputs, export_pdf_with_layout_inputs_and_options,
    export_pdf_with_options, PdfExportOptions,
};
pub use text::{export_text, export_text_with_layout_inputs, LossReport, TextExport};
