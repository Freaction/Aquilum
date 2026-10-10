use std::path::Path;

use masonry::peniko::ImageBrush;

#[cfg(target_os = "windows")]
pub fn video(path: &Path) -> Option<ImageBrush> {
    use std::os::windows::ffi::OsStrExt;
    use std::sync::Arc;

    use masonry::peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};
    use windows::Win32::Foundation::SIZE as Extent;
    use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits, GetObjectW, HGDIOBJ, ReleaseDC};
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK};
    use windows::core::PCWSTR;

    const SIZE: i32 = 1024;

    let native = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()).to_string_lossy().replace('/', "\\");
    let wide: Vec<u16> = std::ffi::OsStr::new(&native).encode_wide().chain(Some(0)).collect();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).ok()?;
        let bitmap = factory.GetImage(Extent { cx: SIZE, cy: SIZE }, SIIGBF_BIGGERSIZEOK).ok()?;
        let object = HGDIOBJ(bitmap.0);
        let mut info = BITMAP::default();
        GetObjectW(object, std::mem::size_of::<BITMAP>() as i32, Some((&raw mut info).cast()));
        let (width, height) = (info.bmWidth, info.bmHeight);
        let mut header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4).max(0) as usize];
        let dc = GetDC(None);
        let lines = GetDIBits(dc, bitmap, 0, height as u32, Some(pixels.as_mut_ptr().cast()), &mut header, DIB_RGB_COLORS);
        ReleaseDC(None, dc);
        let _ = DeleteObject(object);
        if lines == 0 || width <= 0 || height <= 0 {
            return None;
        }
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        let data = ImageData { data: Blob::new(Arc::new(pixels)), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::Alpha, width: width as u32, height: height as u32 };
        Some(ImageBrush::new(data))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn video(path: &Path) -> Option<ImageBrush> {
    crate::video::poster(path)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn video(_path: &Path) -> Option<ImageBrush> {
    None
}
