use std::path::Path;
use std::sync::Arc;
use std::sync::Once;

use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::{CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, IWICBitmap, IWICImagingFactory, WICBitmapCacheOnLoad};
use windows::Win32::Media::MediaFoundation::{
    CLSID_MFMediaEngineClassFactory, IMFAttributes, IMFMediaEngine, IMFMediaEngineClassFactory, IMFMediaEngineNotify, IMFMediaEngineNotify_Impl, MF_MEDIA_ENGINE_CALLBACK,
    MF_MEDIA_ENGINE_VIDEO_OUTPUT_FORMAT, MF_VERSION, MFCreateAttributes, MFSTARTUP_FULL, MFStartup,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx};
use windows::core::{BSTR, implement};

const MAX_WIDTH: u32 = 1280;

#[implement(IMFMediaEngineNotify)]
struct Notify;

impl IMFMediaEngineNotify_Impl for Notify_Impl {
    fn EventNotify(&self, _event: u32, _param1: usize, _param2: u32) -> windows::core::Result<()> {
        Ok(())
    }
}

pub struct Engine {
    engine: IMFMediaEngine,
    wic: IWICImagingFactory,
    bitmap: Option<(IWICBitmap, u32, u32)>,
    last: i64,
    play: bool,
}

fn startup() {
    static START: Once = Once::new();
    START.call_once(|| unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let _ = MFStartup(MF_VERSION, MFSTARTUP_FULL);
    });
}

impl Engine {
    pub fn open(path: &Path) -> Option<Engine> {
        startup();
        unsafe {
            let factory: IMFMediaEngineClassFactory = CoCreateInstance(&CLSID_MFMediaEngineClassFactory, None, CLSCTX_INPROC_SERVER).ok()?;
            let mut attributes: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attributes, 2).ok()?;
            let attributes = attributes?;
            let notify: IMFMediaEngineNotify = Notify.into();
            attributes.SetUnknown(&MF_MEDIA_ENGINE_CALLBACK, &notify).ok()?;
            attributes.SetUINT32(&MF_MEDIA_ENGINE_VIDEO_OUTPUT_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM.0 as u32).ok()?;
            let engine = factory.CreateInstance(0, &attributes).ok()?;
            let native = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()).to_string_lossy().replace('/', "\\");
            engine.SetSource(&BSTR::from(native)).ok()?;
            let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
            Some(Engine { engine, wic, bitmap: None, last: -1, play: false })
        }
    }

    pub fn toggle(&mut self) {
        unsafe {
            if self.engine.IsEnded().as_bool() {
                let _ = self.engine.SetCurrentTime(0.0);
                self.play = false;
            }
            self.play = !self.play;
            let _ = if self.play { self.engine.Play() } else { self.engine.Pause() };
        }
    }

    pub fn seek(&mut self, fraction: f64) {
        unsafe {
            let duration = self.engine.GetDuration();
            if duration.is_finite() && duration > 0.0 {
                let _ = self.engine.SetCurrentTime(duration * fraction.clamp(0.0, 1.0));
            }
        }
    }

    pub fn mute(&mut self) {
        unsafe {
            let muted = self.engine.GetMuted().as_bool();
            let _ = self.engine.SetMuted(!muted);
        }
    }

    pub fn status(&mut self) -> (bool, f64, f64, bool) {
        unsafe {
            let duration = self.engine.GetDuration();
            if self.play && self.engine.IsEnded().as_bool() {
                self.play = false;
            }
            (self.play, self.engine.GetCurrentTime(), if duration.is_finite() { duration } else { 0.0 }, self.engine.GetMuted().as_bool())
        }
    }

    pub fn frame(&mut self) -> Option<ImageBrush> {
        unsafe {
            let pts = self.engine.OnVideoStreamTick().ok()?;
            if pts < 0 || pts == self.last {
                return None;
            }
            let (mut w, mut h) = (0u32, 0u32);
            self.engine.GetNativeVideoSize(Some(&mut w), Some(&mut h)).ok()?;
            if w == 0 || h == 0 {
                return None;
            }
            if w > MAX_WIDTH {
                h = (u64::from(h) * u64::from(MAX_WIDTH) / u64::from(w)).max(1) as u32;
                w = MAX_WIDTH;
            }
            if self.bitmap.as_ref().is_none_or(|(_, bw, bh)| (*bw, *bh) != (w, h)) {
                let bitmap = self.wic.CreateBitmap(w, h, &GUID_WICPixelFormat32bppBGRA, WICBitmapCacheOnLoad).ok()?;
                self.bitmap = Some((bitmap, w, h));
            }
            let (bitmap, _, _) = self.bitmap.as_ref()?;
            let target = RECT { left: 0, top: 0, right: w as i32, bottom: h as i32 };
            self.engine.TransferVideoFrame(bitmap, None, &target, None).ok()?;
            self.last = pts;
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            bitmap.CopyPixels(std::ptr::null(), w * 4, &mut pixels).ok()?;
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
            }
            Some(ImageBrush::new(ImageData { data: Blob::new(Arc::new(pixels)), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::Alpha, width: w, height: h }))
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            let _ = self.engine.Shutdown();
        }
    }
}
