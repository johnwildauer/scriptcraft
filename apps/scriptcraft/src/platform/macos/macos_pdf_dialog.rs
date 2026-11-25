//! Native controls and Uniform Type Identifiers shared by asynchronous file panels.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSControlStateValueOff, NSControlStateValueOn};
use objc2_foundation::NSString;
use objc2_uniform_type_identifiers::{UTType, UTTypePDF};
use scriptcraft_commands::PdfExportOptions;

/// The sole unsafe AppKit construction: nil target/action means the standard
/// checkbox tracks its own state and no callback can cross this adapter boundary.
#[allow(unsafe_code)]
pub(crate) fn include_title_page_checkbox(
    main_thread: MainThreadMarker,
) -> Retained<NSButton> {
    let label = NSString::from_str("Include Title Page");
    // SAFETY: The control has no target and no action, so AppKit cannot message
    // an incorrectly typed target or invoke an invalid selector. MainThreadMarker
    // proves the AppKit object is created on the UI thread.
    let checkbox =
        unsafe { NSButton::checkboxWithTitle_target_action(&label, None, None, main_thread) };
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
pub(crate) fn pdf_content_type() -> &'static UTType {
    // SAFETY: UTTypePDF is an immutable Uniform Type Identifiers framework
    // constant with static storage duration, as declared by its binding.
    unsafe { UTTypePDF }
}
