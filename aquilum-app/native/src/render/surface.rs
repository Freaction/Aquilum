use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};

use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality};
use vello_cpu::{PixelMetadata, Pixmap};

const KEPT: usize = 2;
const POOL: usize = 6;

type Published = (u64, Weak<()>, Arc<Pixmap>);

static PUBLISHED: Mutex<VecDeque<Published>> = Mutex::new(VecDeque::new());

pub struct Surfaces {
    alive: Arc<()>,
    pool: Vec<Arc<Pixmap>>,
}

impl Default for Surfaces {
    fn default() -> Self {
        Surfaces { alive: Arc::new(()), pool: Vec::new() }
    }
}

impl Drop for Surfaces {
    fn drop(&mut self) {
        let mine = Arc::as_ptr(&self.alive);
        PUBLISHED.lock().unwrap_or_else(|e| e.into_inner()).retain(|(_, owner, _)| owner.as_ptr() != mine);
    }
}

impl Surfaces {
    #[cfg(test)]
    pub fn pool_len(&self) -> usize {
        self.pool.len()
    }

    pub fn frame(&mut self, width: u16, height: u16, draw: impl FnOnce(&mut [u8])) -> ImageBrush {
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
        draw(pixmap.data_as_u8_slice_mut());
        let blob: Blob<u8> = Blob::new(Arc::new(Vec::new()));
        let id = blob.id();
        let mine = Arc::as_ptr(&self.alive);
        let mut published = PUBLISHED.lock().unwrap_or_else(|e| e.into_inner());
        published.push_back((id, Arc::downgrade(&self.alive), Arc::clone(&self.pool[index])));
        let mut count = published.iter().filter(|(_, owner, _)| owner.as_ptr() == mine).count();
        published.retain(|(_, owner, _)| {
            let stale = owner.as_ptr() == mine && count > KEPT;
            count -= usize::from(stale);
            !stale
        });
        let data = ImageData { data: blob, format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::AlphaPremultiplied, width: u32::from(width), height: u32::from(height) };
        ImageBrush::new(data).with_quality(ImageQuality::Low)
    }
}

pub fn find(id: u64) -> Option<(Arc<Pixmap>, Weak<()>)> {
    let published = PUBLISHED.lock().unwrap_or_else(|e| e.into_inner());
    published.iter().find(|(i, _, _)| *i == id).map(|(_, owner, pixmap)| (Arc::clone(pixmap), owner.clone()))
}
