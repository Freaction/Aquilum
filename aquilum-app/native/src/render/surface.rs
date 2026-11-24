use std::sync::Arc;

use masonry::peniko::{ImageAlphaType, ImageBrush, ImageQuality};
use vello_cpu::{PixelMetadata, Pixmap};

#[derive(Default)]
pub struct Surfaces {
    pool: Vec<Arc<Pixmap>>,
}

impl Surfaces {
    #[cfg(test)]
    pub fn pool_len(&self) -> usize {
        self.pool.len()
    }

    fn draw(&mut self, width: u16, height: u16, draw: impl FnOnce(&mut Pixmap)) -> Arc<Pixmap> {
        let mut spare = false;
        self.pool.retain(|p| {
            if Arc::strong_count(p) > 1 { return true; }
            p.width() == width && p.height() == height && !std::mem::replace(&mut spare, true)
        });
        let free = self.pool.iter().position(|p| p.width() == width && p.height() == height && Arc::strong_count(p) == 1);
        let index = free.unwrap_or_else(|| {
            let data = vec![255u8; usize::from(width) * usize::from(height) * 4];
            self.pool.push(Arc::new(Pixmap::from_parts(data, width, height, PixelMetadata::new(ImageAlphaType::AlphaPremultiplied, false))));
            self.pool.len() - 1
        });
        let pixmap = Arc::make_mut(&mut self.pool[index]);
        draw(pixmap);
        Arc::clone(&self.pool[index])
    }

    pub fn owned_frame(&mut self, width: u16, height: u16, draw: impl FnOnce(&mut Pixmap)) -> ImageBrush {
        super::pixels::shared_brush(self.draw(width, height, draw), false)
    }

    pub fn frame(&mut self, width: u16, height: u16, draw: impl FnOnce(&mut [u8])) -> ImageBrush {
        super::pixels::shared_brush(self.draw(width, height, |pixmap| draw(pixmap.data_as_u8_slice_mut())), true).with_quality(ImageQuality::Low)
    }
}
