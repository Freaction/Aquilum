pub(super) fn window_icon() -> Option<winit::icon::Icon> {
    let image = image::load_from_memory(include_bytes!("../../assets/icon-64.png")).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    winit::icon::RgbaIcon::new(image.into_raw(), width, height).ok().map(winit::icon::Icon::from)
}

#[cfg(target_os = "windows")]
mod win {
    use masonry::peniko::Color;
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAK, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE, DwmFlush, DwmSetWindowAttribute};
    use windows::Win32::Graphics::Gdi::{CreateSolidBrush, DeleteObject, HGDIOBJ, InvalidateRect};
    use windows::Win32::UI::WindowsAndMessaging::{GCLP_HBRBACKGROUND, SetClassLongPtrW};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;

    fn hwnd(window: &dyn Window) -> Option<HWND> {
        match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut std::ffi::c_void)),
            _ => None,
        }
    }

    fn set(hwnd: HWND, attribute: DWMWINDOWATTRIBUTE, value: i32) {
        let _ = unsafe { DwmSetWindowAttribute(hwnd, attribute, (&raw const value).cast(), 4) };
    }

    pub fn cloak(window: &dyn Window, cloaked: bool) {
        let Some(hwnd) = hwnd(window) else { return };
        if !cloaked {
            let _ = unsafe { DwmFlush() };
        }
        set(hwnd, DWMWA_CLOAK, i32::from(cloaked));
    }

    pub fn set_background(window: &dyn Window, color: Color) {
        let Some(hwnd) = hwnd(window) else { return };
        let rgba = color.to_rgba8();
        let dark = u32::from(rgba.r) + u32::from(rgba.g) + u32::from(rgba.b) < 384;
        set(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, i32::from(dark));
        unsafe {
            let brush = CreateSolidBrush(COLORREF(u32::from(rgba.r) | u32::from(rgba.g) << 8 | u32::from(rgba.b) << 16));
            let old = SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, brush.0 as isize);
            if old != 0 {
                let _ = DeleteObject(HGDIOBJ(old as *mut std::ffi::c_void));
            }
            let _ = InvalidateRect(Some(hwnd), None, true);
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) use win::{cloak, set_background};

#[cfg(not(target_os = "windows"))]
pub(super) fn cloak(_window: &dyn winit::window::Window, _cloaked: bool) {}

#[cfg(not(target_os = "windows"))]
pub(super) fn set_background(_window: &dyn winit::window::Window, _color: masonry::peniko::Color) {}
