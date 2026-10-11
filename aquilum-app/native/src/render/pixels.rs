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

fn registry() -> &'static Mutex<HashMap<u64, Weak<Pixmap>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<u64, Weak<Pixmap>>>> = OnceLock::new();
    REGISTRY.get_or_init(Mutex::default)
}

pub fn brush(pixmap: Pixmap) -> ImageBrush {
    let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
    let pixmap = Arc::new(pixmap);
    let data = Blob::new(Arc::new(Shared(Arc::clone(&pixmap))));
    let mut registry = registry().lock().unwrap_or_else(|e| e.into_inner());
    registry.retain(|_, weak| weak.strong_count() > 0);
    registry.insert(data.id(), Arc::downgrade(&pixmap));
    ImageBrush::new(ImageData { data, format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::AlphaPremultiplied, width, height })
}

pub fn from_rgba(data: Vec<u8>, width: u32, height: u32, alpha_type: ImageAlphaType) -> Option<ImageBrush> {
    let (w, h) = (u16::try_from(width).ok()?, u16::try_from(height).ok()?);
    if data.len() != usize::from(w) * usize::from(h) * 4 {
        return None;
    }
    Some(brush(Pixmap::from_parts(data, w, h, PixelMetadata::new(alpha_type, true))))
}

pub fn marker() -> ImageBrush {
    brush(Pixmap::new(1, 1))
}

pub fn lookup(id: u64) -> Option<Arc<Pixmap>> {
    registry().lock().unwrap_or_else(|e| e.into_inner()).get(&id)?.upgrade()
}
