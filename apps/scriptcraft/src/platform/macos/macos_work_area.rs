//! Current root-window monitor work area from AppKit.

use crate::widgets::script_note::state::WorkAreaBounds;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSView, NSWindow};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Observe the visible frame of the screen currently associated with eframe's
/// root NSWindow. No foreground or secondary-window lookup is performed.
#[allow(unsafe_code)]
pub(crate) fn current_work_area(frame: &eframe::Frame) -> Option<WorkAreaBounds> {
    let handle = frame.window_handle().ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    let _main_thread = MainThreadMarker::new()?;
    let view_pointer = appkit.ns_view.cast::<NSView>();

    // SAFETY: eframe supplies the borrowed AppKit view for its live root frame;
    // the main-thread marker above satisfies AppKit's thread requirement. The
    // view is borrowed only to resolve its owning root window.
    let view = unsafe { view_pointer.as_ref() };
    let window: objc2::rc::Retained<NSWindow> = view.window()?;
    let screen = window.screen()?;
    let visible = screen.visibleFrame();
    let full = screen.frame();

    // AppKit's screen rectangles use a bottom-left origin. Convert once to the
    // top-left logical-point convention used by native viewport placement. Use
    // the full screen frame's height, not visibleFrame's height, so reserved
    // areas remain represented as offsets inside the monitor.
    let left = visible.origin.x as f32;
    let right = left + visible.size.width as f32;
    let top = (full.origin.y + full.size.height - visible.origin.y - visible.size.height) as f32;
    let bottom = (full.origin.y + full.size.height - visible.origin.y) as f32;
    WorkAreaBounds::new(left, top, right, bottom)
}
