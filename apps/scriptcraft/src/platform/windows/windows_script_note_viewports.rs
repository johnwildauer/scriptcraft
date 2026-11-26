//! Best-effort, per-HWND suppression of DWM transitions for ScriptNote windows.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowLongW, GetWindowThreadProcessId, IsWindow, SetWindowLongW,
    SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SIZEBOX, WS_SYSMENU,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptNoteViewportKind {
    Card,
    ColorMenu,
    TypeMenu,
    TypeDialog,
}

impl ScriptNoteViewportKind {
    fn exact_title(self) -> &'static str {
        match self {
            Self::Card => "ScriptNote",
            Self::ColorMenu => "ScriptNote Color",
            Self::TypeMenu => "ScriptNote Type",
            Self::TypeDialog => "New ScriptNote Type",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransitionControlOutcome {
    Applied,
    Unavailable,
}

/// Applies the DWM transition attribute only to the exact-title HWND after
/// confirming that it is a valid window owned by this process.
#[allow(unsafe_code)]
pub(crate) fn disable_transitions(
    viewport: ScriptNoteViewportKind,
) -> TransitionControlOutcome {
    let title: Vec<u16> = viewport
        .exact_title()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: The title pointer remains valid for the synchronous lookup.
    // FindWindowW resolves only this dedicated title; ownership is verified
    // below before any attribute is applied.
    let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) };
    if hwnd.0.is_null() || !unsafe { IsWindow(hwnd) }.as_bool() {
        return TransitionControlOutcome::Unavailable;
    }

    let mut process_id = 0;
    // SAFETY: `process_id` is a valid writable output for this synchronous
    // query, and `hwnd` was validated immediately above.
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
    if thread_id == 0 || process_id != std::process::id() {
        return TransitionControlOutcome::Unavailable;
    }

    let disabled = BOOL(1);
    // SAFETY: The BOOL value and its correctly sized buffer remain alive for
    // the synchronous DWM call. The HWND passed is current-process verified.
    if unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            (&disabled as *const BOOL).cast(),
            std::mem::size_of::<BOOL>() as u32,
        )
    }
    .is_ok()
    {
        TransitionControlOutcome::Applied
    } else {
        TransitionControlOutcome::Unavailable
    }
}

/// Removes caption-button styles only from the current process's exact New
/// ScriptNote Type window, retaining its caption and native draggable title bar.
#[allow(unsafe_code)]
pub(crate) fn hide_type_dialog_caption_buttons() -> TransitionControlOutcome {
    let title: Vec<u16> = ScriptNoteViewportKind::TypeDialog
        .exact_title()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: The title pointer remains valid for the synchronous lookup.
    let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) };
    if hwnd.0.is_null() || !unsafe { IsWindow(hwnd) }.as_bool() {
        return TransitionControlOutcome::Unavailable;
    }

    let mut process_id = 0;
    // SAFETY: `process_id` is a valid writable output and `hwnd` was validated.
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
    if thread_id == 0 || process_id != std::process::id() {
        return TransitionControlOutcome::Unavailable;
    }

    // SAFETY: The current-process HWND is valid for the synchronous style query.
    let original_style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
    if original_style == 0 || original_style & (WS_CAPTION.0 as i32) == 0 {
        return TransitionControlOutcome::Unavailable;
    }
    let caption_buttons = (WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SIZEBOX).0 as i32;
    let updated_style = original_style & !caption_buttons;
    if updated_style != original_style {
        // SAFETY: The new style preserves WS_CAPTION and only removes the
        // requested caption-button/resize styles from this verified HWND.
        let previous_style = unsafe { SetWindowLongW(hwnd, GWL_STYLE, updated_style) };
        if previous_style == 0 {
            return TransitionControlOutcome::Unavailable;
        }
    }

    // SAFETY: The verified HWND is only reframed; these flags prohibit moving,
    // resizing, activation, or Z-order changes.
    if unsafe {
        SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    }
    .as_bool()
    {
        TransitionControlOutcome::Applied
    } else {
        TransitionControlOutcome::Unavailable
    }
}
