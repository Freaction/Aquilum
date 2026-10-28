use windows::Win32::Foundation::{HMODULE, HWND, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_MWA_NO_ALT_ENTER, DXGI_SCALING_NONE,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2,
    IDXGISwapChain1,
};

use super::Area;

pub(super) const GROWTH: u32 = 256;
pub(super) const BUFFERS: u32 = 3;

pub(super) struct Gpu {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub chain: IDXGISwapChain1,
    pub canvas: ID3D11Texture2D,
}

pub(super) fn capacity(need: u32, have: u32) -> u32 {
    if need <= have { have } else { need.next_multiple_of(GROWTH) }
}

fn device(driver: D3D_DRIVER_TYPE) -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext)> {
    let (mut device, mut context) = (None, None);
    unsafe {
        D3D11CreateDevice(None, driver, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_SINGLETHREADED, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context))?;
    }
    Ok((device.expect("устройство D3D11"), context.expect("контекст D3D11")))
}

pub(super) fn texture(device: &ID3D11Device, width: u32, height: u32) -> windows::core::Result<ID3D11Texture2D> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: (D3D11_CPU_ACCESS_READ.0 | D3D11_CPU_ACCESS_WRITE.0) as u32,
        MiscFlags: 0,
    };
    let mut texture = None;
    unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture))? };
    Ok(texture.expect("текстура холста"))
}

pub(super) fn rect(a: Area) -> RECT {
    RECT { left: a.x as i32, top: a.y as i32, right: (a.x + a.w) as i32, bottom: (a.y + a.h) as i32 }
}

pub(super) fn region(a: Area) -> D3D11_BOX {
    D3D11_BOX { left: a.x, top: a.y, front: 0, right: a.x + a.w, bottom: a.y + a.h, back: 1 }
}

pub(super) fn create(hwnd: HWND, width: u32, height: u32) -> Result<Gpu, String> {
    let warp = cfg!(feature = "memory-probe") && std::env::var_os("AQ_PROBE_WARP").is_some();
    let (device, context) = if warp { device(D3D_DRIVER_TYPE_WARP) } else {
        device(D3D_DRIVER_TYPE_HARDWARE).or_else(|error| {
            report!("видеокарта недоступна ({error}), вывод через WARP");
            device(D3D_DRIVER_TYPE_WARP)
        })
    }
    .map_err(|e| format!("D3D11: {e}"))?;
    let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }.map_err(|e| format!("DXGI: {e}"))?;
    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: BUFFERS,
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
