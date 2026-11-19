//! Immutable, platform-neutral input to the native print operation.
//!
//! This module builds only PDF-backed Print composition. Native panels and
//! printer APIs are intentionally owned by later macOS/Windows adapters.

use scriptcraft_commands::PdfExportOptions;
use scriptcraft_export::{self as exports, ExportError};
use scriptcraft_layout::LayoutEnvironment;
use scriptcraft_model::{LayoutInputs, Project};
use std::ffi::OsStr;
use std::path::Path;

const MAX_PRINT_JOB_TITLE_BYTES: usize = 128;
const UNTITLED_PRINT_JOB_TITLE: &str = "Untitled Screenplay";

/// Owned job data: the native operation can retain these bytes independently
/// of later project edits or layout-cache changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrintRequest {
    pdf_data: Vec<u8>,
    job_title: String,
    typography_report: exports::PdfTypographyReport,
}

/// Owned semantic/fixed-layout snapshot captured before the native panel opens.
/// The native settings result can retain it through acceptance without building
/// PDF bytes until the later delivery step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrintCompositionSnapshot {
    project: Project,
    layout_inputs: LayoutInputs,
    layout_environment: LayoutEnvironment,
    native_path: Option<std::path::PathBuf>,
}

impl PrintCompositionSnapshot {
    pub(crate) fn capture(
        project: &Project,
        layout_inputs: &LayoutInputs,
        layout_environment: &LayoutEnvironment,
        native_path: Option<&Path>,
    ) -> Self {
        Self {
            project: project.clone(),
            layout_inputs: layout_inputs.clone(),
            layout_environment: layout_environment.clone(),
            native_path: native_path.map(Path::to_path_buf),
        }
    }

    pub(crate) fn build_request(
        &self,
        options: PdfExportOptions,
    ) -> Result<PrintRequest, ExportError> {
        build_print_request(
            &self.project,
            &self.layout_inputs,
            &self.layout_environment,
            options,
            self.native_path.as_deref(),
        )
    }
}

impl PrintRequest {
    pub(crate) fn pdf_data(&self) -> &[u8] {
        &self.pdf_data
    }

    pub(crate) fn job_title(&self) -> &str {
        &self.job_title
    }

    pub(crate) fn fallback_summary(&self) -> String {
        self.typography_report.summary()
    }
}

/// Shared PDF builder used by both Save As PDF and native printing. The core
/// exporter owns `LayoutComposition::Print` and consumes fixed document inputs.
pub(super) fn build_pdf_bytes(
    project: &Project,
    layout_inputs: &LayoutInputs,
    layout_environment: &LayoutEnvironment,
    options: PdfExportOptions,
) -> Result<exports::PdfExport, ExportError> {
    exports::export_pdf_with_layout_inputs_and_options(
        project,
        layout_inputs,
        layout_environment,
        exports::PdfExportOptions {
            include_title_page: options.include_title_page,
        },
    )
}

/// Builds one complete request from borrowed semantic/layout inputs and
/// returns owned PDF bytes, a bounded job title, and the bounded typography
/// fallback report needed by PDF-backed print feedback.
pub(super) fn build_print_request(
    project: &Project,
    layout_inputs: &LayoutInputs,
    layout_environment: &LayoutEnvironment,
    options: PdfExportOptions,
    native_path: Option<&Path>,
) -> Result<PrintRequest, ExportError> {
    let export = build_pdf_bytes(project, layout_inputs, layout_environment, options)?;
    Ok(PrintRequest {
        pdf_data: export.bytes,
        job_title: print_job_title(native_path),
        typography_report: export.typography,
    })
}

pub(super) fn print_job_title(native_path: Option<&Path>) -> String {
    let Some(stem) = native_path
        .and_then(Path::file_stem)
        .filter(|stem| !stem.is_empty())
        .and_then(OsStr::to_str)
    else {
        return UNTITLED_PRINT_JOB_TITLE.to_owned();
    };

    let mut bounded = String::new();
    for character in stem.chars() {
        let character = if character.is_control() {
            ' '
        } else {
            character
        };
        if bounded.len() + character.len_utf8() > MAX_PRINT_JOB_TITLE_BYTES {
            break;
        }
        bounded.push(character);
    }
    let bounded = bounded.trim();
    if bounded.is_empty() {
        UNTITLED_PRINT_JOB_TITLE.to_owned()
    } else {
        bounded.to_owned()
    }
}
