//! AppKit adapter for the synchronous unsaved-changes decision.
//!
//! `NSAlert::runModal` presents an application-modal alert on the main thread.
//! Its first button is AppKit's default Return action; the titled Cancel button
//! receives Escape, while the second button remains a nondefault choice. Native
//! construction has no recoverable result in objc2, so unavailable main-thread
//! access and every response other than the first two known choices fail safe to
//! Cancel. This adapter only returns a choice; it never continues lifecycle work.

use objc2::MainThreadMarker;
use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn};
use objc2_foundation::NSString;

use crate::document_lifecycle::UnsavedChangesChoice;

const ALERT_TITLE: &str = "Unsaved Changes";
const MESSAGE_TEXT: &str = "Do you want to save the changes to this document?";
const INFORMATIVE_TEXT: &str = "Your changes will be lost if you don’t save them.";

/// Displays the native Save / Don’t Save / Cancel choice, failing safe to Cancel.
/// The supplied display title is intentionally not interpolated into alert copy;
/// this keeps the prompt generic and prevents a path from reaching its text.
pub(crate) fn prompt_unsaved_changes(_display_title: &str) -> UnsavedChangesChoice {
    let Some(main_thread) = MainThreadMarker::new() else {
        return UnsavedChangesChoice::Cancel;
    };

    let alert = NSAlert::new(main_thread);
    let title = NSString::from_str(ALERT_TITLE);
    let message = NSString::from_str(MESSAGE_TEXT);
    let informative = NSString::from_str(INFORMATIVE_TEXT);
    alert.window().setTitle(&title);
    alert.setMessageText(&message);
    alert.setInformativeText(&informative);

    let save = NSString::from_str("Save");
    let dont_save = NSString::from_str("Don’t Save");
    let cancel = NSString::from_str("Cancel");
    alert.addButtonWithTitle(&save);
    alert.addButtonWithTitle(&dont_save);
    alert.addButtonWithTitle(&cancel);

    choice_from_response(alert.runModal())
}

fn choice_from_response(response: objc2_app_kit::NSModalResponse) -> UnsavedChangesChoice {
    if response == NSAlertFirstButtonReturn {
        UnsavedChangesChoice::Save
    } else if response == NSAlertSecondButtonReturn {
        UnsavedChangesChoice::DontSave
    } else {
        // Includes explicit Cancel (third button), Escape, dismissal, and
        // unexpected native response values.
        UnsavedChangesChoice::Cancel
    }
}

#[cfg(test)]
mod tests {
    use super::choice_from_response;
    use crate::document_lifecycle::UnsavedChangesChoice;
    use objc2_app_kit::{
        NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSAlertThirdButtonReturn,
    };

    #[test]
    fn maps_known_buttons_and_fails_safe_for_cancel_or_unexpected_responses() {
        assert_eq!(
            choice_from_response(NSAlertFirstButtonReturn),
            UnsavedChangesChoice::Save
        );
        assert_eq!(
            choice_from_response(NSAlertSecondButtonReturn),
            UnsavedChangesChoice::DontSave
        );
        assert_eq!(
            choice_from_response(NSAlertThirdButtonReturn),
            UnsavedChangesChoice::Cancel
        );
        assert_eq!(choice_from_response(-1), UnsavedChangesChoice::Cancel);
    }
}
