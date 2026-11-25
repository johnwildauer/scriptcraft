//! Current root-window monitor work area from Win32.

use crate::widgets::script_note::state::WorkAreaBounds;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};

/// Observe rcWork for the monitor nearest eframe's actual root HWND and convert
/// its physical-pixel edges using that root viewport's native scale.
#[allow(unsafe_code)]
pub(crate) fn current_work_area(
    frame: &eframe::Frame,
    native_pixels_per_point: Option<f32>,
) -> Option<WorkAreaBounds> {
    let scale = native_pixels_per_point.filter(|scale| scale.is_finite() && *scale > 0.0)?;
    let handle = frame.window_handle().ok()?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return None;
    };
    let hwnd = HWND(win32.hwnd.get() as *mut _);
    if hwnd.0.is_null() {
        return None;
    }

    // SAFETY: `hwnd` is borrowed from eframe's live root frame. The monitor and
    // info calls are synchronous and use initialized, correctly sized output.
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.0.is_null() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return None;
    }

    let work = info.rcWork;
    let left = work.left as f32 / scale;
    let top = work.top as f32 / scale;
    let right = work.right as f32 / scale;
    let bottom = work.bottom as f32 / scale;
    WorkAreaBounds::new(left, top, right, bottom)
}
