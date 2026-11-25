//! Win32 window tweaks (dark title bar).

use std::ffi::c_void;

#[link(name = "user32", kind = "raw-dylib")]
extern "system" {
    fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
}

#[link(name = "dwmapi", kind = "raw-dylib")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: isize, attribute: u32, value: *const c_void, size: u32) -> i32;
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn set_dark_titlebar(title: &str, dark: bool) {
    let title = wide(title);
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if hwnd == 0 {
        return;
    }
    let value: i32 = if dark { 1 } else { 0 };
    // 20 = DWMWA_USE_IMMERSIVE_DARK_MODE (Windows 10 2004+), 19 = older builds.
    unsafe {
        DwmSetWindowAttribute(hwnd, 20, &value as *const i32 as *const c_void, 4);
        DwmSetWindowAttribute(hwnd, 19, &value as *const i32 as *const c_void, 4);
    }
}

/// Rounds the window corners on Windows 11 via DwmSetWindowAttribute.
/// DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2.
/// Ignores the error on Windows 10 (attribute unsupported).
pub fn set_rounded_corners(title: &str) {
    let title = wide(title);
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if hwnd == 0 {
        return;
    }
    let value: u32 = 2; // DWMWCP_ROUND
    unsafe {
        DwmSetWindowAttribute(hwnd, 33, &value as *const u32 as *const c_void, 4);
    }
}
