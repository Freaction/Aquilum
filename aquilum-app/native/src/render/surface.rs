use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality};
use vello_cpu::{PixelMetadata, Pixmap};

const KEPT: usize = 4;
const POOL: usize = 6;

static PUBLISHED: Mutex<VecDeque<(u64, Arc<Pixmap>)>> = Mutex::new(VecDeque::new());

pub struct Surfaces {
    pool: Vec<Arc<Pixmap>>,
}

impl Default for Surfaces {
    fn default() -> Self {
        Surfaces { pool: Vec::new() }
    }
}

impl Surfaces {
    pub fn frame(&mut self, width: u16, height: u16, fill: [u8; 3], draw: impl FnOnce(&mut [u8])) -> ImageBrush {
        if self.pool.len() > POOL {
            let mut excess = self.pool.len() - POOL;
            self.pool.retain_mut(|p| {
                let drop = excess > 0 && (p.width() != width || p.height() != height) && Arc::get_mut(p).is_some();
                excess -= usize::from(drop);
                !drop
            });
        }
        let free = self.pool.iter_mut().position(|p| p.width() == width && p.height() == height && Arc::get_mut(p).is_some());
        let index = free.unwrap_or_else(|| {
            let data = vec![255u8; usize::from(width) * usize::from(height) * 4];
            self.pool.push(Arc::new(Pixmap::from_parts(data, width, height, PixelMetadata::new(ImageAlphaType::AlphaPremultiplied, false))));
            self.pool.len() - 1
        });
        let pixmap = Arc::get_mut(&mut self.pool[index]).expect("свободный буфер");
        let color = vello_cpu::peniko::color::PremulRgba8 { r: fill[0], g: fill[1], b: fill[2], a: 255 };
        pixmap.data_mut().fill(color);
        draw(pixmap.data_as_u8_slice_mut());
        let blob: Blob<u8> = Blob::new(Arc::new(Vec::new()));
        let id = blob.id();
        let mut published = PUBLISHED.lock().unwrap_or_else(|e| e.into_inner());
        published.push_back((id, Arc::clone(&self.pool[index])));
        while published.len() > KEPT {
            published.pop_front();
        }
        let data = ImageData { data: blob, format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::AlphaPremultiplied, width: u32::from(width), height: u32::from(height) };
        ImageBrush::new(data).with_quality(ImageQuality::Low)
    }
}

pub fn take(id: u64) -> Option<Arc<Pixmap>> {
    let mut published = PUBLISHED.lock().unwrap_or_else(|e| e.into_inner());
    let at = published.iter().position(|(i, _)| *i == id)?;
    published.remove(at).map(|(_, p)| p)
}
