//! Windows Common Controls Task Dialog adapter for the synchronous
//! unsaved-changes decision.
//!
//! The eframe frame is borrowed on the UI thread and its Win32 window handle is
//! retained through `TaskDialogIndirect`, so the native dialog is owned and
//! modal to the actual root window. The embedded application manifest enables
//! Common Controls v6. Save is the default custom button; Don’t Save is the
//! other custom choice, and the standard Cancel button plus Escape, dialog
//! close, unavailable owner, unexpected IDs, and API errors all map to Cancel.
//! This adapter only returns a choice and has no session or filesystem effects.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Controls::{
    TaskDialogIndirect, TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDCBF_CANCEL_BUTTON,
    TDF_ALLOW_DIALOG_CANCELLATION, TDF_POSITION_RELATIVE_TO_WINDOW,
};

use crate::document_lifecycle::UnsavedChangesChoice;

const MAIN_INSTRUCTION: &str = "Unsaved Changes";
const MESSAGE_TEXT: &str = "Do you want to save the changes to this document?";
const INFORMATIVE_TEXT: &str = "Your changes will be lost if you don’t save them.";
const UNTITLED_DOCUMENT_TITLE: &str = "Untitled Screenplay";
const SAVE_BUTTON_ID: i32 = 1000;
const DONT_SAVE_BUTTON_ID: i32 = 1001;
const CANCEL_BUTTON_ID: i32 = 2;

/// Presents the native Save / Don’t Save / Cancel choice, failing safe to Cancel.
/// `display_title` is reduced to its final path component before it is used as
/// the native window title, so a caller cannot expose a document path.
#[allow(unsafe_code)]
pub(crate) fn prompt_unsaved_changes(
    display_title: &str,
    frame: &eframe::Frame,
) -> UnsavedChangesChoice {
    // Keep the borrowed eframe handle alive for the entire synchronous native
    // call. This is the root viewport's HWND, never a foreground-window guess.
    let Ok(handle) = frame.window_handle() else {
        return UnsavedChangesChoice::Cancel;
    };
    let RawWindowHandle::Win32(owner) = handle.as_raw() else {
        return UnsavedChangesChoice::Cancel;
    };
    let hwnd = HWND(owner.hwnd.get() as *mut _);
    if hwnd.0.is_null() {
        return UnsavedChangesChoice::Cancel;
    }

    let window_title = wide_null_terminated(final_component(display_title));
    let main_instruction = wide_null_terminated(MAIN_INSTRUCTION);
    let content = wide_null_terminated(&format!("{MESSAGE_TEXT}\n\n{INFORMATIVE_TEXT}"));
    let save_label = wide_null_terminated("Save");
    let dont_save_label = wide_null_terminated("Don’t Save");
    let buttons = [
        TASKDIALOG_BUTTON {
            nButtonID: SAVE_BUTTON_ID,
            pszButtonText: PCWSTR(save_label.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: DONT_SAVE_BUTTON_ID,
            pszButtonText: PCWSTR(dont_save_label.as_ptr()),
        },
    ];
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        hwndParent: hwnd,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION | TDF_POSITION_RELATIVE_TO_WINDOW,
        dwCommonButtons: TDCBF_CANCEL_BUTTON,
        pszWindowTitle: PCWSTR(window_title.as_ptr()),
        pszMainInstruction: PCWSTR(main_instruction.as_ptr()),
        pszContent: PCWSTR(content.as_ptr()),
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        nDefaultButton: SAVE_BUTTON_ID,
        ..Default::default()
    };

    let mut button_id = 0;
    // SAFETY: This synchronous call runs on eframe's UI thread. The borrowed
    // root HWND, UTF-16 strings, config, and custom button records all remain
    // alive and unchanged until TaskDialogIndirect returns.
    let result = unsafe {
        TaskDialogIndirect(&config, Some(&mut button_id), None, None)
    };
    if result.is_err() {
        return UnsavedChangesChoice::Cancel;
    }

    choice_from_button_id(button_id)
}

fn final_component(display_title: &str) -> &str {
    display_title
        .rsplit(|character| character == '\\' || character == '/')
        .find(|component| !component.is_empty())
        .unwrap_or(UNTITLED_DOCUMENT_TITLE)
}

fn wide_null_terminated(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn choice_from_button_id(button_id: i32) -> UnsavedChangesChoice {
    match button_id {
        SAVE_BUTTON_ID => UnsavedChangesChoice::Save,
        DONT_SAVE_BUTTON_ID => UnsavedChangesChoice::DontSave,
        // Includes standard Cancel, Escape/dialog close, and unexpected IDs.
        _ => UnsavedChangesChoice::Cancel,
    }
}

#[cfg(test)]
mod tests {
    use super::{choice_from_button_id, final_component, DONT_SAVE_BUTTON_ID, SAVE_BUTTON_ID};
    use crate::document_lifecycle::UnsavedChangesChoice;

    #[test]
    fn maps_only_the_two_custom_ids_to_choices() {
        assert_eq!(choice_from_button_id(SAVE_BUTTON_ID), UnsavedChangesChoice::Save);
        assert_eq!(
            choice_from_button_id(DONT_SAVE_BUTTON_ID),
            UnsavedChangesChoice::DontSave
        );
        assert_eq!(choice_from_button_id(2), UnsavedChangesChoice::Cancel);
        assert_eq!(choice_from_button_id(-1), UnsavedChangesChoice::Cancel);
    }

    #[test]
    fn native_window_title_uses_only_a_final_component() {
        assert_eq!(final_component(r"C:\drafts\screenplay.fdx"), "screenplay.fdx");
        assert_eq!(final_component("/drafts/screenplay.fdx"), "screenplay.fdx");
        assert_eq!(final_component(""), "Untitled Screenplay");
    }
}
