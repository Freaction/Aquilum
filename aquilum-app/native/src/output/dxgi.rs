use windows::Win32::Foundation::{HMODULE, HWND, POINT, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_MWA_NO_ALT_ENTER, DXGI_PRESENT, DXGI_PRESENT_PARAMETERS, DXGI_SCALING_NONE,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2,
    IDXGISwapChain1,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::rc::Rc;

use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use super::Area;

struct Gpu {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    chain: IDXGISwapChain1,
    canvas: ID3D11Texture2D,
}

pub struct Output {
    hwnd: HWND,
    gpu: Option<Gpu>,
    size: (u32, u32),
    capacity: (u32, u32),
    fresh: bool,
}

const GROWTH: u32 = 256;

fn capacity(need: u32, have: u32) -> u32 {
    if need <= have { have } else { need.next_multiple_of(GROWTH) }
}

fn device(driver: D3D_DRIVER_TYPE) -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext)> {
    let (mut device, mut context) = (None, None);
    unsafe {
        D3D11CreateDevice(None, driver, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context))?;
    }
    Ok((device.expect("устройство D3D11"), context.expect("контекст D3D11")))
}

fn texture(device: &ID3D11Device, width: u32, height: u32) -> windows::core::Result<ID3D11Texture2D> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let mut texture = None;
    unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture))? };
    Ok(texture.expect("текстура холста"))
}

fn rect(a: Area) -> RECT {
    RECT { left: a.x as i32, top: a.y as i32, right: (a.x + a.w) as i32, bottom: (a.y + a.h) as i32 }
}

fn region(a: Area) -> D3D11_BOX {
    D3D11_BOX { left: a.x, top: a.y, front: 0, right: a.x + a.w, bottom: a.y + a.h, back: 1 }
}

fn create(hwnd: HWND, width: u32, height: u32) -> Result<Gpu, String> {
    let (device, context) = device(D3D_DRIVER_TYPE_HARDWARE).or_else(|_| device(D3D_DRIVER_TYPE_WARP)).map_err(|e| format!("D3D11: {e}"))?;
    let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }.map_err(|e| format!("DXGI: {e}"))?;
    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 3,
        Scaling: DXGI_SCALING_NONE,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        Flags: 0,
    };
    let chain = unsafe { factory.CreateSwapChainForHwnd(&device, hwnd, &desc, None, None) }.map_err(|e| format!("цепочка кадров: {e}"))?;
    unsafe {
        let _ = factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER);
    }
    let canvas = texture(&device, width, height).map_err(|e| format!("холст: {e}"))?;
    Ok(Gpu { device, context, chain, canvas })
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
        Ok(Output { hwnd, gpu: Some(gpu), size, capacity, fresh: true })
    }

    fn lost(&mut self, err: impl std::fmt::Display) {
        crate::host::report(&format!("вывод кадра: {err}; устройство пересоздаётся"));
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
            return Ok(());
        };
        if capacity == self.capacity {
            return Ok(());
        }
        let _zone = aq_trace::zone("размер цепочки");
        unsafe { gpu.chain.ResizeBuffers(0, capacity.0, capacity.1, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0)) }.map_err(|e| format!("размер цепочки: {e}"))?;
        gpu.canvas = texture(&gpu.device, capacity.0, capacity.1).map_err(|e| format!("холст: {e}"))?;
        self.capacity = capacity;
        Ok(())
    }

    pub fn fresh(&self) -> bool {
        self.fresh
    }

    pub fn upload(&mut self, area: Area, pixels: &[u32]) {
        let _zone = aq_trace::zone("загрузка");
        let Some(gpu) = &self.gpu else { return };
        unsafe {
            gpu.context.UpdateSubresource(&gpu.canvas, 0, Some(&region(area)), pixels.as_ptr().cast(), area.w * 4, 0);
        }
    }

    pub fn present(&mut self, dirty: Area) {
        let _zone = aq_trace::zone("вывод");
        let Some(gpu) = &self.gpu else { return };
        let mut dirty_rect = rect(dirty);
        let result = unsafe {
            gpu.chain.GetBuffer::<ID3D11Texture2D>(0).and_then(|back| {
                gpu.context.CopyResource(&back, &gpu.canvas);
                let params = DXGI_PRESENT_PARAMETERS {
                    DirtyRectsCount: if self.fresh { 0 } else { 1 },
                    pDirtyRects: &mut dirty_rect,
                    pScrollRect: std::ptr::null_mut(),
                    pScrollOffset: std::ptr::null_mut::<POINT>(),
                };
                gpu.chain.Present1(0, DXGI_PRESENT(0), &params).ok()
            })
        };
        match result {
            Ok(()) => self.fresh = false,
            Err(err) => self.lost(err),
        }
    }
}
