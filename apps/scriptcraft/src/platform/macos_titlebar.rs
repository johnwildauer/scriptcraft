//! Synchronous public AppKit action, with no retained window or pointer state.

use eframe::egui;
use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::{NSView, NSWindowButton};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[allow(unsafe_code)] // Only the live borrowed NSView retain below.
pub(super) fn perform_zoom(
    frame: &eframe::Frame,
    position: egui::Pos2,
    viewport: egui::Rect,
) -> Result<(), &'static str> {
    let _main_thread = MainThreadMarker::new().ok_or("not on the AppKit main thread")?;
    let handle = frame
        .window_handle()
        .map_err(|_| "window handle unavailable")?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return Err("window handle is not AppKit");
    };
    // SAFETY: Frame::window_handle supplies a live, non-null borrowed NSView,
    // not an NSWindow. The frame and its borrowed handle stay alive throughout
    // this synchronous call. MainThreadMarker was checked before any AppKit
    // access. Retain before use; view and current window are released on return,
    // on this same thread. No raw pointer or retained wrapper is cached/returned.
    let view = unsafe { Retained::<NSView>::retain(appkit.ns_view.cast().as_ptr()) }
        .ok_or("native view unavailable")?;
    let window = view.window().ok_or("native view has no current window")?;
    let zoom_button = window
        .standardWindowButton(NSWindowButton::ZoomButton)
        .ok_or("native Zoom action unavailable")?;
    if !zoom_button.isEnabled() {
        return Err("native Zoom action disabled");
    }

    // Convert egui logical coordinates to this live view's AppKit coordinates.
    // Ratios account for egui zoom without confusing backing pixels with points.
    let bounds = view.bounds();
    if !viewport.is_positive() || bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
        return Err("native view geometry unavailable");
    }
    let x = bounds.origin.x
        + f64::from((position.x - viewport.left()) / viewport.width()) * bounds.size.width;
    let y_fraction = f64::from((position.y - viewport.top()) / viewport.height());
    let y = bounds.origin.y
        + if view.isFlipped() {
            y_fraction
        } else {
            1.0 - y_fraction
        } * bounds.size.height;
    for kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
        NSWindowButton::ToolbarButton,
        NSWindowButton::DocumentIconButton,
        NSWindowButton::DocumentVersionsButton,
    ] {
        if let Some(button) = window.standardWindowButton(kind) {
            let rect = view.convertRect_fromView(button.bounds(), Some(&button));
            // Extra slop conservatively protects native hit targets, not just paint.
            if native_control_contains(
                (
                    rect.origin.x,
                    rect.origin.y,
                    rect.size.width,
                    rect.size.height,
                ),
                (x, y),
            ) {
                return Err("pointer belongs to a native window control");
            }
        }
    }
    // Selected initial candidate only; no claim about Fill or other settings.
    window.performZoom(None);
    Ok(())
}

fn native_control_contains(rect: (f64, f64, f64, f64), point: (f64, f64)) -> bool {
    let (x, y, width, height) = rect;
    let slop = 8.0;
    point.0 >= x - slop
        && point.0 <= x + width + slop
        && point.1 >= y - slop
        && point.1 <= y + height + slop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_double_click_excludes_native_control_bounds_and_hit_slop() {
        let rect = (10.0, 12.0, 14.0, 14.0);
        assert!(native_control_contains(rect, (17.0, 19.0)));
        assert!(native_control_contains(rect, (2.0, 4.0)));
        assert!(native_control_contains(rect, (32.0, 34.0)));
        assert!(!native_control_contains(rect, (33.0, 19.0)));
    }
}
