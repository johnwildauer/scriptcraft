#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Explicit output adapters: plain text remains environment-only, while PDF
//! also requires a transient exact resolved-face byte payload.
//!
//! Both adapters consume shared Print layout and only borrow project/layout
//! state, so exporting cannot change the editing session or its clean state.

mod pdf;
mod pdf_document;
mod pdf_font;
mod pdf_font_policy;
mod pdf_font_payload;
mod text;
mod fountain;

use scriptcraft_layout::LayoutError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Layout(LayoutError),
    /// The resolved-font/page environment is missing a request or does not
    /// match the supplied page inputs.
    InvalidTypographyEnvironment,
    /// A Print span's canonical scalar-boundary topology is inconsistent.
    InvalidLayoutGeometry,
    /// Exact resolved-face metadata or supplied bytes are missing or incoherent.
    InvalidPdfFontPayload,
    /// Required packaged Courier Prime bytes failed provenance validation.
    RequiredPackagedFontEmbedding,
    /// A bounded serialized output/resource limit was exceeded.
    OutputResourceLimitExceeded,
    /// Fountain source runs or style attributes are internally ambiguous.
    InvalidFountainSource,
}

impl From<LayoutError> for ExportError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

pub use pdf::{
    export_pdf, export_pdf_with_layout_inputs, export_pdf_with_layout_inputs_and_options,
    export_pdf_with_options, PdfBase14Family, PdfExport, PdfExportOptions, PdfFallbackEntry,
    PdfFallbackKind, PdfTypographyReport,
};
pub use pdf_font_policy::PdfFontEmbeddingFallbackReason;
pub use pdf_font_payload::{PdfFontPayload, PdfResolvedFaceBytes};
pub use text::{export_text, export_text_with_layout_inputs, LossReport, TextExport};
pub use fountain::export_fountain;
