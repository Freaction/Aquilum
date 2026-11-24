use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
use vello_cpu::{PixelMetadata, Pixmap};

struct Shared(Arc<Pixmap>);

impl AsRef<[u8]> for Shared {
    fn as_ref(&self) -> &[u8] {
        self.0.data_as_u8_slice()
    }
}

type Registry = Mutex<HashMap<u64, (Weak<Pixmap>, bool)>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Mutex::default)
}

pub fn brush(pixmap: Pixmap) -> ImageBrush {
    shared_brush(Arc::new(pixmap), false)
}

pub(super) fn shared_brush(pixmap: Arc<Pixmap>, direct: bool) -> ImageBrush {
    let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
    let data = Blob::new(Arc::new(Shared(Arc::clone(&pixmap))));
    let mut registry = registry().lock().unwrap_or_else(|e| e.into_inner());
    registry.retain(|_, (weak, _)| weak.strong_count() > 0);
    registry.insert(data.id(), (Arc::downgrade(&pixmap), direct));
    ImageBrush::new(ImageData { data, format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::AlphaPremultiplied, width, height })
}

fn from_parts(data: Vec<u8>, width: u32, height: u32, metadata: PixelMetadata) -> Option<ImageBrush> {
    let (w, h) = (u16::try_from(width).ok()?, u16::try_from(height).ok()?);
    if data.len() != usize::from(w) * usize::from(h) * 4 {
        return None;
    }
    Some(brush(Pixmap::from_parts(data, w, h, metadata)))
}

pub fn from_rgba(data: Vec<u8>, width: u32, height: u32, alpha_type: ImageAlphaType) -> Option<ImageBrush> {
    from_parts(data, width, height, PixelMetadata::new(alpha_type, true))
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn from_bgrx(mut data: Vec<u8>, width: u32, height: u32) -> Option<ImageBrush> {
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    from_parts(data, width, height, PixelMetadata::new(ImageAlphaType::AlphaPremultiplied, false))
}

pub fn unpremultiply([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let straight = |c: u8| if a == 0 || a == 255 { c } else { ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8 };
    [straight(r), straight(g), straight(b), a]
}

pub fn marker() -> ImageBrush {
    brush(Pixmap::new(1, 1))
}

pub fn lookup(id: u64) -> Option<(Arc<Pixmap>, bool)> {
    let registry = registry().lock().unwrap_or_else(|e| e.into_inner());
    let (pixmap, direct) = registry.get(&id)?;
    Some((pixmap.upgrade()?, *direct))
}
