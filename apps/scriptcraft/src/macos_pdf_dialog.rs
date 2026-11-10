//! macOS NSSavePanel adapter for one-step PDF destination/options selection.
//!
//! The panel runs as a native application-modal panel because the current eframe
//! command adapter does not expose a borrowed `NSWindow` owner here. The retained
//! checkbox and its accessory view remain alive until `runModal` returns.

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSButton, NSControlStateValueOff, NSControlStateValueOn, NSModalResponseCancel,
    NSModalResponseOK, NSSavePanel,
};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_uniform_type_identifiers::{UTType, UTTypePDF};
use scriptcraft_commands::PdfExportOptions;
use std::ffi::{CStr, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use crate::pdf_export::{PdfExportPickerFailure, PdfExportPickerResult};

#[derive(Debug, PartialEq, Eq)]
enum PanelResult {
    Accepted(Option<PathBuf>),
    Cancelled,
    Failed,
}

/// Opens one NSSavePanel with a fresh, enabled Include Title Page checkbox.
/// The native panel owns overwrite confirmation; only its selected URL and
/// checkbox value leave this adapter.
pub(super) fn show_pdf_save_panel(default_filename: &str) -> PdfExportPickerResult {
    let Some(main_thread) = MainThreadMarker::new() else {
        return PdfExportPickerResult::Error(PdfExportPickerFailure::Unavailable);
    };

    let panel = NSSavePanel::savePanel(main_thread);
    let title = NSString::from_str("Save As PDF");
    panel.setTitle(Some(&*title));
    let file_name = NSString::from_str(default_filename);
    panel.setNameFieldStringValue(&file_name);
    panel.setAllowedContentTypes(&NSArray::arrayWithObject(pdf_content_type()));
    panel.setAllowsOtherFileTypes(false);

    let checkbox = include_title_page_checkbox(main_thread);
    panel.setAccessoryView(Some(&checkbox));

    let response = panel.runModal();
    let result = if response == NSModalResponseOK {
        let destination = panel
            .URL()
            .filter(|url| url.isFileURL())
            .and_then(|url| path_buf_from_file_url(&url));
        PanelResult::Accepted(destination)
    } else if response == NSModalResponseCancel {
        PanelResult::Cancelled
    } else {
        PanelResult::Failed
    };

    picker_result_from_panel_result(
        result,
        checkbox.state() == NSControlStateValueOn,
    )
}

/// The sole unsafe AppKit construction: nil target/action means the standard
/// checkbox tracks its own state and no callback can cross this adapter boundary.
#[allow(unsafe_code)]
fn include_title_page_checkbox(main_thread: MainThreadMarker) -> objc2::rc::Retained<NSButton> {
    let label = NSString::from_str("Include Title Page");
    // SAFETY: The control has no target and no action, so AppKit cannot message
    // an incorrectly typed target or invoke an invalid selector. MainThreadMarker
    // proves the AppKit object is created on the UI thread.
    let checkbox = unsafe {
        NSButton::checkboxWithTitle_target_action(&label, None, None, main_thread)
    };
    let default_include_title_page = PdfExportOptions::default().include_title_page;
    checkbox.setState(if default_include_title_page {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    checkbox
}

/// Returns AppKit's immutable system PDF type constant.
#[allow(unsafe_code)]
fn pdf_content_type() -> &'static UTType {
    // SAFETY: UTTypePDF is an immutable Uniform Type Identifiers framework
    // constant with static storage duration, as declared by its binding.
    unsafe { UTTypePDF }
}

fn picker_result_from_panel_result(
    result: PanelResult,
    include_title_page: bool,
) -> PdfExportPickerResult {
    match result {
        PanelResult::Cancelled => PdfExportPickerResult::Cancel,
        PanelResult::Failed => PdfExportPickerResult::Error(PdfExportPickerFailure::Failed),
        PanelResult::Accepted(Some(destination)) => PdfExportPickerResult::Accept {
            destination,
            options: PdfExportOptions {
                include_title_page,
            },
        },
        PanelResult::Accepted(None) => {
            PdfExportPickerResult::Error(PdfExportPickerFailure::Failed)
        }
    }
}

/// Copies the URL's native filesystem bytes so invalid UTF-8 reaches the shared
/// continuation unchanged and is rejected before layout/PDF building or writing.
#[allow(unsafe_code)]
fn path_buf_from_file_url(url: &NSURL) -> Option<PathBuf> {
    let representation = url.fileSystemRepresentation();
    // SAFETY: NSURL returns a non-null, NUL-terminated filesystem representation;
    // the URL remains retained by the caller for the duration of this copy.
    let bytes = unsafe { CStr::from_ptr(representation.as_ptr()).to_bytes() };
    if bytes.is_empty() {
        return None;
    }
    Some(PathBuf::from(OsStr::from_bytes(bytes)))
}

#[cfg(test)]
mod tests {
    use super::{picker_result_from_panel_result, PanelResult};
    use crate::pdf_export::{PdfExportPickerFailure, PdfExportPickerResult};
    use scriptcraft_commands::PdfExportOptions;
    use std::path::PathBuf;

    #[test]
    fn every_accepted_request_carries_its_fresh_checkbox_value() {
        let destination = PathBuf::from("/selected/output.pdf");
        let fresh_default = PdfExportOptions::default().include_title_page;

        let first = picker_result_from_panel_result(
            PanelResult::Accepted(Some(destination.clone())),
            fresh_default,
        );
        let toggled_off = picker_result_from_panel_result(
            PanelResult::Accepted(Some(destination.clone())),
            false,
        );
        let next_request = picker_result_from_panel_result(
            PanelResult::Accepted(Some(destination.clone())),
            PdfExportOptions::default().include_title_page,
        );

        assert_eq!(
            first,
            PdfExportPickerResult::Accept {
                destination: destination.clone(),
                options: PdfExportOptions {
                    include_title_page: true,
                },
            }
        );
        assert_eq!(
            toggled_off,
            PdfExportPickerResult::Accept {
                destination: destination.clone(),
                options: PdfExportOptions {
                    include_title_page: false,
                },
            }
        );
        assert_eq!(
            next_request,
            PdfExportPickerResult::Accept {
                destination,
                options: PdfExportOptions {
                    include_title_page: true,
                },
            }
        );
    }

    #[test]
    fn cancellation_and_incomplete_acceptance_are_bounded_results() {
        assert_eq!(
            picker_result_from_panel_result(PanelResult::Cancelled, true),
            PdfExportPickerResult::Cancel
        );
        assert_eq!(
            picker_result_from_panel_result(PanelResult::Accepted(None), true),
            PdfExportPickerResult::Error(PdfExportPickerFailure::Failed)
        );
        assert_eq!(
            picker_result_from_panel_result(PanelResult::Failed, false),
            PdfExportPickerResult::Error(PdfExportPickerFailure::Failed)
        );
    }
}
