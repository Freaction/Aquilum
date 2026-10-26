use std::num::NonZeroI32;
use std::ptr;
use std::rc::Rc;

use ndk::hardware_buffer_format::HardwareBufferFormat;
use ndk::native_window::NativeWindow;
use winit::event_loop::OwnedDisplayHandle;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::{Area, Canvas, Target};

pub struct Output {
    native_window: NativeWindow,
    pixels: Vec<u32>,
    width: u32,
    height: u32,
    fresh: bool,
    background: u32,
}

impl Output {
    pub fn new(_display: OwnedDisplayHandle, window: Rc<dyn Window>) -> Result<Self, String> {
        let raw = window.window_handle().map_err(|e| format!("window handle: {e}"))?.as_raw();
        let RawWindowHandle::AndroidNdk(handle) = raw else {
            return Err("not an Android window".into());
        };
        // SAFETY: The a_native_window pointer is valid as long as the window handle is alive.
        let native_window = unsafe { NativeWindow::clone_from_ptr(handle.a_native_window.cast()) };
        Ok(Output {
            native_window,
            pixels: Vec::new(),
            width: 0,
            height: 0,
            fresh: true,
            background: 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if self.width == width && self.height == height && !self.pixels.is_empty() {
            return Ok(());
        }
        let (Some(w), Some(h)) = (NonZeroI32::new(width as i32), NonZeroI32::new(height as i32)) else {
            return Ok(());
        };
        self.native_window
            .set_buffers_geometry(w.into(), h.into(), Some(HardwareBufferFormat::R8G8B8A8_UNORM))
            .map_err(|e| format!("ANativeWindow::set_buffers_geometry: {e}"))?;

        let len = (width * height) as usize;
        self.pixels.resize(len, self.background);
        self.pixels.fill(self.background);
        self.width = width;
        self.height = height;
        self.fresh = true;
        Ok(())
    }

    pub fn trim(&mut self) {
        let _ = self.native_window.set_buffers_geometry(1, 1, Some(HardwareBufferFormat::R8G8B8A8_UNORM));
        self.pixels = Vec::new();
        self.width = 0;
        self.height = 0;
        self.fresh = true;
    }

    pub fn set_background(&mut self, rgba: u32) {
        if self.background != rgba {
            self.background = rgba;
            self.pixels.fill(rgba);
            self.fresh = true;
        }
    }

    pub fn fresh(&self) -> bool {
        self.fresh
    }

    pub fn present(&mut self, _dirty: &[Area]) {
        let Ok(mut guard) = self.native_window.lock(None) else { return };
        let dst_bits = guard.bits() as *mut u32;
        if dst_bits.is_null() {
            return;
        }
        let dst_stride = guard.stride();
        let height = guard.height().min(self.height as usize);
        let width = guard.width().min(self.width as usize);
        let src_stride = self.width as usize;
        let src = self.pixels.as_ptr();

        if dst_stride == src_stride && width == src_stride {
            unsafe {
                ptr::copy_nonoverlapping(src, dst_bits, width * height);
            }
        } else {
            for y in 0..height {
                unsafe {
                    let src_row = src.add(y * src_stride);
                    let dst_row = dst_bits.add(y * dst_stride);
                    ptr::copy_nonoverlapping(src_row, dst_row, width);
                }
            }
        }
        self.fresh = false;
    }
}

impl Canvas for Output {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        let (width, height) = (self.width as u16, self.height as u16);
        paint(Target { pixels: &mut self.pixels, width, height });
    }
}
