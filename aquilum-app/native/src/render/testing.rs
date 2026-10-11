use masonry::app::VisualLayerPlan;
use masonry::peniko::Color;

use super::paint::PaintError;
use super::renderer::Renderer;
use crate::output::{Area, Canvas, Target};

impl Renderer {
    pub fn draw_frame(&mut self, layers: &VisualLayerPlan, width: u16, height: u16, scale: f64, background: Color, full: bool) -> Result<Vec<Area>, PaintError> {
        if (self.frame.0, self.frame.1) != (width, height) {
            self.frame = (width, height, vec![0; usize::from(width) * usize::from(height)]);
        }
        let mut frame = std::mem::take(&mut self.frame);
        let drawn = self.draw(layers, width, height, scale, background, full, &mut frame);
        self.frame = frame;
        drawn
    }

    pub fn live_frame(&mut self, layers: &VisualLayerPlan) -> Result<Vec<Area>, PaintError> {
        let mut frame = std::mem::take(&mut self.frame);
        let drawn = self.live(layers, &mut frame);
        self.frame = frame;
        drawn
    }

    pub fn render(&mut self, layers: &VisualLayerPlan, width: u16, height: u16, scale: f64, background: Color) -> Result<(), PaintError> {
        self.draw_frame(layers, width, height, scale, background, true)?;
        Ok(())
    }

    pub fn frame_pixels(&self) -> &[u32] {
        &self.frame.2
    }

    pub fn png(&self) -> Vec<u8> {
        let (w, h, pixels) = &self.frame;
        let rgba: Vec<u8> = pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]).collect();
        let mut out = Vec::new();
        image::RgbaImage::from_raw(u32::from(*w), u32::from(*h), rgba)
            .expect("кадр")
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("кадр кодируется в PNG");
        out
    }
}

impl Canvas for (u16, u16, Vec<u32>) {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        paint(Target { pixels: &mut self.2, width: self.0, height: self.1 });
    }
}
