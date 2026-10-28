use std::num::NonZeroI32;
use std::rc::Rc;

use ndk::hardware_buffer_format::HardwareBufferFormat;
use ndk::native_window::{NativeWindow, Rect};
use winit::event_loop::OwnedDisplayHandle;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::{Area, Canvas, Pixels, Target};

pub struct Output {
    native_window: NativeWindow,
    pixels: Pixels,
}

impl Output {
    pub fn new(_display: OwnedDisplayHandle, window: Rc<dyn Window>) -> Result<Self, String> {
        let raw = window.window_handle().map_err(|e| format!("window handle: {e}"))?.as_raw();
        let RawWindowHandle::AndroidNdk(handle) = raw else {
            return Err("not an Android window".into());
        };
        // SAFETY: The a_native_window pointer is valid as long as the window handle is alive.
        let native_window = unsafe { NativeWindow::clone_from_ptr(handle.a_native_window.cast()) };
        Ok(Output { native_window, pixels: Pixels::new() })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if self.pixels.sized(width, height) {
            return Ok(());
        }
        let (Some(w), Some(h)) = (NonZeroI32::new(width as i32), NonZeroI32::new(height as i32)) else {
            return Ok(());
        };
        self.native_window
            .set_buffers_geometry(w.into(), h.into(), Some(HardwareBufferFormat::R8G8B8A8_UNORM))
            .map_err(|e| format!("ANativeWindow::set_buffers_geometry: {e}"))?;
        self.pixels.resize(width, height);
        Ok(())
    }

    pub fn trim(&mut self) {
        let _ = self.native_window.set_buffers_geometry(1, 1, Some(HardwareBufferFormat::R8G8B8A8_UNORM));
        self.pixels.trim();
    }

    pub fn set_background(&mut self, rgba: u32) {
        self.pixels.set_background(rgba);
    }

    pub fn fresh(&self) -> bool {
        self.pixels.fresh
    }

    pub fn present(&mut self, dirty: &[Area]) {
        let whole = Area { x: 0, y: 0, w: self.pixels.width, h: self.pixels.height };
        let area = if self.pixels.fresh { whole } else { dirty.iter().copied().reduce(Area::union).unwrap_or(whole) };
        let mut bounds = Rect { left: area.x as i32, top: area.y as i32, right: (area.x + area.w) as i32, bottom: (area.y + area.h) as i32 };
        let Ok(mut guard) = self.native_window.lock(Some(&mut bounds)) else { return };
        let bits = guard.bits() as *mut u32;
        if bits.is_null() {
            return;
        }
        let (stride, width, height) = (guard.stride(), guard.width().min(self.pixels.width as usize), guard.height().min(self.pixels.height as usize));
        let clamp = |v: i32, max: usize| (v.max(0) as usize).min(max) as u32;
        let (x, y) = (clamp(bounds.left, width), clamp(bounds.top, height));
        let area = Area { x, y, w: clamp(bounds.right, width).saturating_sub(x), h: clamp(bounds.bottom, height).saturating_sub(y) };
        let window = unsafe { std::slice::from_raw_parts_mut(bits, stride * guard.height()) };
        self.pixels.copy_to(window, stride, area);
        self.pixels.fresh = false;
    }
}

impl Canvas for Output {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        self.pixels.with_target(paint);
    }
}
