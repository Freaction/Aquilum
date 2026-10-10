use masonry::app::{VisualLayerKind, VisualLayerPlan};
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::imaging::{PaintSink, Painter as Sink};
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::Color;
use vello_cpu::{PixelFormat, PixmapMut, RasterizerSettings, RenderSettings};

use super::damage::Tracker;
use crate::output::Area;
use super::paint::{PaintError, Painter};

struct Frame<'a> {
    width: u16,
    height: u16,
    scale: f64,
    background: Color,
    base: &'a Scene,
    overlays: Vec<(&'a Scene, Affine)>,
}

impl Frame<'_> {
    fn paint_into(&self, sink: &mut dyn PaintSink) {
        Sink::new(sink).fill_rect(Rect::new(0.0, 0.0, f64::from(self.width), f64::from(self.height)), self.background);
        let scale = Affine::scale(self.scale);
        replay_transformed(self.base, sink, scale);
        for (scene, transform) in &self.overlays {
            replay_transformed(scene, sink, scale * *transform);
        }
    }
}

pub struct Renderer {
    painter: Painter,
    tracker: Tracker,
    scratch: Vec<u32>,
    #[cfg(test)]
    pub(super) frame: (u16, u16, Vec<u32>),
}

impl Renderer {
    pub fn new(threads: u16) -> Self {
        let settings = RenderSettings { num_threads: threads, ..Default::default() };
        Renderer {
            painter: Painter::new(1, 1, settings),
            tracker: Tracker::new(),
            scratch: Vec::new(),
            #[cfg(test)]
            frame: (0, 0, Vec::new()),
        }
    }

    pub fn draw(&mut self, layers: &VisualLayerPlan, width: u16, height: u16, scale: f64, background: Color, full: bool) -> Result<Option<(Area, &[u32])>, PaintError> {
        let Some(VisualLayerKind::Scene(base)) = layers.root_layer().map(|l| &l.kind) else {
            return Ok(None);
        };
        let overlays: Vec<(&Scene, Affine)> = layers
            .overlay_layers()
            .filter_map(|layer| match &layer.kind {
                VisualLayerKind::Scene(scene) => Some((scene, layer.transform)),
                _ => None,
            })
            .collect();
        let frame = Frame { width, height, scale, background, base, overlays };
        let damage = {
            let _zone = aq_trace::zone("поиск изменений");
            self.tracker.begin(width, height);
            frame.paint_into(&mut self.tracker);
            self.tracker.damage()
        };
        let Some((x, y, w, h)) = (if full { Some((0, 0, width, height)) } else { damage }) else {
            return Ok(None);
        };
        {
            let _zone = aq_trace::zone("кодирование");
            self.painter.begin(x, y, w, h);
            frame.paint_into(&mut self.painter);
            if let Err(err) = self.painter.finish() {
                self.tracker.forget();
                return Err(err);
            }
        }
        let _zone = aq_trace::zone("растеризация");
        self.scratch.resize(usize::from(w) * usize::from(h), 0);
        let pixmap = PixmapMut::new(w, h, bytemuck::cast_slice_mut(&mut self.scratch)).expect("буфер области");
        let settings = RasterizerSettings { pixel_format: PixelFormat::Bgra8, ..Default::default() };
        self.painter.ctx.render_with(pixmap, &mut self.painter.resources, settings);
        let area = Area { x: u32::from(x), y: u32::from(y), w: u32::from(w), h: u32::from(h) };
        Ok(Some((area, &self.scratch)))
    }

    #[cfg(test)]
    pub fn render(&mut self, layers: &VisualLayerPlan, width: u16, height: u16, scale: f64, background: Color) -> Result<(), PaintError> {
        let pixels = self.draw(layers, width, height, scale, background, true)?.map(|(_, p)| p.to_vec()).unwrap_or_default();
        self.frame = (width, height, pixels);
        Ok(())
    }

    #[cfg(test)]
    pub fn frame_pixels(&self) -> &[u32] {
        &self.frame.2
    }

    #[cfg(test)]
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
