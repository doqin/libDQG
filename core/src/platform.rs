use winit::window::Window;

#[cfg(windows)]
pub fn round_window_corners(window: &Window) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };

    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::Win32(win32_handle) = handle.as_raw() else { return };
    let hwnd = win32_handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    let preference = DWMWCP_ROUND;

    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &preference as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&preference) as u32,
        );
    }
}

#[cfg(not(windows))]
pub fn round_window_corners(_window: &Window) {}
