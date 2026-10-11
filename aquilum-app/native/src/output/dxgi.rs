use std::rc::Rc;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Direct3D11::{D3D11_MAP_WRITE, D3D11_MAPPED_SUBRESOURCE, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;
use windows::Win32::Graphics::Dxgi::{DXGI_PRESENT, DXGI_PRESENT_PARAMETERS, DXGI_SWAP_CHAIN_FLAG};
use winit::event_loop::OwnedDisplayHandle;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::device::{capacity, create, rect, region, texture, Gpu, GROWTH};
use super::{Area, Canvas, Target};

pub struct Output {
    hwnd: HWND,
    gpu: Option<Gpu>,
    size: (u32, u32),
    capacity: (u32, u32),
    fresh: bool,
    background: u32,
}

impl Output {
    pub fn new(_display: OwnedDisplayHandle, window: Rc<dyn Window>) -> Result<Self, String> {
        let Ok(RawWindowHandle::Win32(handle)) = window.window_handle().map(|h| h.as_raw()) else {
            return Err("окно без HWND".into());
        };
        let hwnd = HWND(handle.hwnd.get() as *mut std::ffi::c_void);
        let size = window.surface_size();
        let size = (size.width.max(1), size.height.max(1));
        let capacity = (size.0.next_multiple_of(GROWTH), size.1.next_multiple_of(GROWTH));
        let gpu = create(hwnd, capacity.0, capacity.1)?;
        Ok(Output { hwnd, gpu: Some(gpu), size, capacity, fresh: true, background: 0 })
    }

    fn lost(&mut self, err: impl std::fmt::Display) {
        report!("вывод кадра: {err}; устройство пересоздаётся");
        self.gpu = None;
        self.fresh = true;
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if self.gpu.is_some() && (width, height) == self.size {
            return Ok(());
        }
        self.size = (width, height);
        self.fresh = true;
        let capacity = (capacity(width, self.capacity.0), capacity(height, self.capacity.1));
        let Some(gpu) = self.gpu.as_mut() else {
            self.gpu = Some(create(self.hwnd, capacity.0, capacity.1)?);
            self.capacity = capacity;
            self.clear();
            return Ok(());
        };
        if capacity == self.capacity {
            return Ok(());
        }
        let _zone = aq_trace::zone("размер цепочки");
        unsafe { gpu.chain.ResizeBuffers(0, capacity.0, capacity.1, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0)) }.map_err(|e| format!("размер цепочки: {e}"))?;
        gpu.canvas = texture(&gpu.device, capacity.0, capacity.1).map_err(|e| format!("холст: {e}"))?;
        gpu.back = 0;
        self.capacity = capacity;
        self.clear();
        Ok(())
    }

    pub fn trim(&mut self) {
        self.gpu = None;
        self.fresh = true;
    }

    pub fn set_background(&mut self, bgra: u32) {
        if self.background != bgra {
            self.background = bgra;
            self.clear();
        }
    }

    fn clear(&mut self) {
        let (background, (w, h)) = (self.background, self.capacity);
        self.with_target(&mut |target| target.pixels.fill(background));
        if let Some(gpu) = &mut self.gpu {
            gpu.stale = std::array::from_fn(|_| vec![Area { x: 0, y: 0, w, h }]);
        }
    }

    pub fn fresh(&self) -> bool {
        self.fresh
    }


    pub fn present(&mut self, dirty: &[Area]) {
        let _zone = aq_trace::zone("вывод");
        let Some(gpu) = &mut self.gpu else { return };
        for stale in &mut gpu.stale {
            stale.extend_from_slice(dirty);
            super::merge(stale);
        }
        let copy = std::mem::take(&mut gpu.stale[gpu.back]);
        if copy.is_empty() {
            return;
        }
        gpu.back = (gpu.back + 1) % gpu.stale.len();
        let mut rects: Vec<_> = copy.iter().map(|a| rect(*a)).collect();
        let result = unsafe {
            gpu.chain.GetBuffer::<ID3D11Texture2D>(0).and_then(|back| {
                for area in &copy {
                    gpu.context.CopySubresourceRegion(&back, 0, area.x, area.y, 0, &gpu.canvas, 0, Some(&region(*area)));
                }
                let params = DXGI_PRESENT_PARAMETERS {
                    DirtyRectsCount: if self.fresh { 0 } else { rects.len() as u32 },
                    pDirtyRects: rects.as_mut_ptr(),
                    pScrollRect: std::ptr::null_mut(),
                    pScrollOffset: std::ptr::null_mut::<POINT>(),
                };
                gpu.chain.Present1(1, DXGI_PRESENT(0), &params).ok()
            })
        };
        if aq_trace::enabled() {
            let mut stats = windows::Win32::Graphics::Dxgi::DXGI_FRAME_STATISTICS::default();
            if unsafe { gpu.chain.GetFrameStatistics(&mut stats) }.is_ok() {
                aq_trace::counter("показ: кадр", f64::from(stats.PresentCount));
                aq_trace::counter("показ: такт", f64::from(stats.PresentRefreshCount));
            }
        }
        match result {
            Ok(()) => self.fresh = false,
            Err(err) => self.lost(err),
        }
    }
}

impl Canvas for Output {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        let Some(gpu) = &self.gpu else { return };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if let Err(err) = unsafe { gpu.context.Map(&gpu.canvas, 0, D3D11_MAP_WRITE, 0, Some(&mut mapped)) } {
            self.lost(err);
            return;
        }
        let (width, height) = (mapped.RowPitch / 4, self.capacity.1);
        let pixels = unsafe { std::slice::from_raw_parts_mut(mapped.pData.cast::<u32>(), (width * height) as usize) };
        paint(Target { pixels, width: width as u16, height: height as u16 });
        unsafe { gpu.context.Unmap(&gpu.canvas, 0) };
    }
}
