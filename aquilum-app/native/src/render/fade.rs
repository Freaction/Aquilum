use std::cell::RefCell;
use std::collections::HashMap;

use masonry::app::VisualLayerPlan;
use masonry::imaging::Painter as Sink;
use masonry::imaging::record::replay_transformed;
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::{Color, ImageBrush};
use vello_cpu::{PixelFormat, Pixmap, PixmapMut, RasterizerSettings, TargetInit};

use super::paint::PaintError;
use super::renderer::Renderer;
use super::stamp::covering;
use crate::output::{Area, Canvas, Target};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fade {
    pub backdrop: Color,
    pub bounds: Rect,
    pub transform: Affine,
    pub alpha: f32,
}

thread_local! {
    static FADES: RefCell<HashMap<u64, Fade>> = RefCell::new(HashMap::new());
}

pub fn publish(id: u64, fade: Fade) {
    FADES.with_borrow_mut(|f| f.insert(id, fade));
}

pub fn forget(id: u64) {
    FADES.with_borrow_mut(|f| f.remove(&id));
}

pub(super) fn get(id: u64) -> Option<Fade> {
    FADES.with_borrow(|f| f.get(&id).copied())
}

pub(super) struct Cached {
    id: u64,
    area: Area,
    image: ImageBrush,
}

impl Renderer {
    pub(super) fn fade(&mut self, layers: &VisualLayerPlan, canvas: &mut dyn Canvas) -> Result<Option<Area>, PaintError> {
        let Some((width, height, scale, _)) = self.params else { return Ok(None) };
        let Some((scene, layer, fade, id)) = self.frame(layers).and_then(|f| f.fading) else {
            if self.faded.take().is_some() {
                self.stamps.trim();
            }
            return Ok(None);
        };
        let _zone = aq_trace::zone("появление слоя");
        let window = Area { x: 0, y: 0, w: u32::from(width), h: u32::from(height) };
        let Some(area) = covering(layer.transform_rect_bbox(fade.bounds).scale_from_origin(scale)).intersect(window) else { return Ok(None) };
        if self.faded.as_ref().is_none_or(|c| c.id != id || c.area != area) {
            let _zone = aq_trace::zone("появление слоя: растр");
            let (x, y, w, h) = (area.x as u16, area.y as u16, area.w as u16, area.h as u16);
            self.painter.begin(x, y, w, h);
            replay_transformed(scene, &mut self.painter, Affine::scale(scale) * layer);
            self.painter.finish()?;
            let mut pixmap = Pixmap::new(w, h);
            self.painter.ctx.render_with(&mut pixmap, &mut self.painter.resources, RasterizerSettings::default());
            self.faded = Some(Cached { id, area, image: super::brush(pixmap) });
        }
        let Some(cached) = &self.faded else { return Ok(None) };
        let image = cached.image.clone().with_alpha(fade.alpha);
        let placed = Affine::scale(scale) * layer * fade.transform * layer.inverse() * Affine::scale(scale.recip()) * Affine::translate((f64::from(cached.area.x), f64::from(cached.area.y)));
        let painter = &mut self.painter;
        painter.begin(0, 0, width, height);
        let mut sink = Sink::new(painter);
        sink.fill_rect(Rect::new(0.0, 0.0, f64::from(width), f64::from(height)), fade.backdrop);
        sink.draw_image(&image, placed);
        painter.finish()?;
        let stamps = &mut self.stamps;
        canvas.with_target(&mut |mut target| {
            stamps.save(&mut target, window);
            let Target { pixels, width, height } = target;
            let pixmap = PixmapMut::new(width, height, bytemuck::cast_slice_mut(&mut *pixels)).expect("холст вывода");
            let settings = RasterizerSettings { target_init: TargetInit::SrcOver, pixel_format: PixelFormat::Bgra8, offset: (0, 0) };
            painter.ctx.render_with(pixmap, &mut painter.resources, settings);
        });
        Ok(Some(window))
    }
}
