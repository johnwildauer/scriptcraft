//! Synchronous boundary for the native unsaved-changes prompt.
//!
//! Prompt construction or platform-adapter failure must fail safe to Cancel.
//! Platform adapters may use only the supplied display title and borrowed eframe
//! frame, and must not save, close, mutate a session, or access the filesystem.

use crate::document_lifecycle::UnsavedChangesChoice;

/// Dispatches a native Save / Don't Save / Cancel prompt, failing safe to Cancel.
pub(crate) fn prompt_unsaved_changes(
    title: &str,
    frame: &eframe::Frame,
) -> UnsavedChangesChoice {
    #[cfg(target_os = "macos")]
    {
        let _ = frame;
        crate::macos_unsaved_changes_prompt::prompt_unsaved_changes(title)
    }

    #[cfg(target_os = "windows")]
    {
        crate::windows_unsaved_changes_prompt::prompt_unsaved_changes(title, frame)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (title, frame);
        UnsavedChangesChoice::Cancel
    }
}
