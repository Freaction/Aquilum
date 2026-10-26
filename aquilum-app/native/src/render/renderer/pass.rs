use masonry::app::VisualLayerPlan;
use masonry::kurbo::Rect;
use vello_cpu::{PixmapMut, RasterizerSettings, TargetInit};

use super::Renderer;
use crate::output::{Area, Canvas, Target};
use crate::render::damage::Damage;
use crate::render::layer::{Layer, area, area_of};
use crate::render::overlay::Mark;
use crate::render::paint::PaintError;
use crate::render::sprite::{self, Sprite};
use crate::render::stamp::covering;

impl Renderer {
    pub(super) fn lift_sprites(&mut self, canvas: &mut dyn Canvas, touched: &[Area]) {
        let visible: Vec<Sprite> = self.sprites.iter().filter(|s| !s.covered).copied().collect();
        if !self.overlay.on && self.stamped.as_ref() == Some(&visible) && self.sprite_stamps.untouched(touched) {
            return;
        }
        self.sprite_stamps.restore(canvas);
        self.stamped = None;
    }

    pub(super) fn put_sprites(&mut self, canvas: &mut dyn Canvas) {
        if self.stamped.is_some() {
            return;
        }
        sprite::stamp(&self.sprites, canvas, &mut self.sprite_stamps);
        self.stamped = Some(self.sprites.iter().filter(|s| !s.covered).copied().collect());
    }

    pub(super) fn paint(&mut self, layers: &VisualLayerPlan, full: bool, canvas: &mut dyn Canvas, seconds: f64) -> Result<Vec<Area>, PaintError> {
        let Some((width, height, scale, _)) = self.params else {
            self.lift_sprites(canvas, &[]);
            return Ok(Vec::new());
        };
        let Some(frame) = self.frame(layers) else {
            self.lift_sprites(canvas, &[]);
            return Ok(Vec::new());
        };
        self.painter.regions = crate::render::scroll::snapshot().iter().map(|r| r.viewport.scale_from_origin(scale)).collect();
        let damage = {
            let _zone = aq_trace::zone("поиск изменений");
            self.tracker.begin(width, height, scale);
            frame.paint_into(&mut self.tracker);
            self.tracker.damage()
        };
        let mut damage = if full { Damage { areas: vec![(0, 0, width, height)], moves: Vec::new() } } else { damage };
        let bounds = Rect::new(0.0, 0.0, f64::from(width), f64::from(height));
        let layers = self.tracker.live.iter().filter_map(|&(id, rect, transform, shape)| Some(Layer { id, area: area_of(rect, bounds)?, transform, shape })).collect();
        self.refresh_live(layers, &mut damage, full, seconds);
        let sprites = std::mem::take(&mut self.tracker.sprites);
        self.sprite_changes = sprite::changes(&self.sprites, &sprites);
        self.sprites = sprites;
        self.shifted = damage.moves.iter().map(|m| area((m.x, m.y, m.w, m.h))).collect();
        #[cfg(test)]
        {
            self.shifts += damage.moves.len();
            self.rastered = damage.areas.clone();
            self.moved = damage.moves.iter().map(|m| (m.x, m.y, m.w, m.h, m.dy)).collect();
            self.causes = std::mem::take(&mut self.tracker.causes);
        }
        let dirty: Vec<Area> = damage.areas.iter().copied().chain(damage.moves.iter().map(|m| (m.x, m.y, m.w, m.h))).map(area).collect();
        let touched: Vec<Area> = dirty.iter().copied().chain(self.due_areas(seconds)).collect();
        self.lift_sprites(canvas, &touched);
        if dirty.is_empty() {
            return Ok(dirty);
        }
        for &rect in &damage.areas {
            self.overlay.flash(area(rect), Mark::Raster);
        }
        let painter = &mut self.painter;
        painter.seconds = seconds;
        let mut result = Ok(());
        canvas.with_target(&mut |Target { pixels, width, height }| {
            for shift in &damage.moves {
                let _zone = aq_trace::zone("сдвиг прокрутки");
                crate::render::scroll::shift_rows(pixels, usize::from(width), *shift);
            }
            for &(x, y, w, h) in &damage.areas {
                {
                    let _zone = aq_trace::zone("кодирование");
                    painter.begin(x, y, w, h);
                    frame.paint_into(painter);
                    if let Err(err) = painter.finish() {
                        result = Err(err);
                        return;
                    }
                }
                let _zone = aq_trace::zone("растеризация");
                let pixmap = PixmapMut::new(width, height, bytemuck::cast_slice_mut(&mut *pixels)).expect("холст вывода");
                let settings = RasterizerSettings { target_init: TargetInit::SrcOver, pixel_format: crate::render::PIXEL_FORMAT, offset: (x, y) };
                painter.ctx.render_with(pixmap, &mut painter.resources, settings);
            }
        });
        if result.is_err() {
            self.tracker.forget();
        }
        result.map(|()| dirty)
    }

    pub(super) fn decorate(&mut self, canvas: &mut dyn Canvas) -> Option<Area> {
        self.overlay.pending = false;
        if !self.overlay.on {
            return self.overlay.show(canvas, &mut self.stamps, &[]);
        }
        let scale = self.params.map_or(1.0, |p| p.2);
        let mut outlines: Vec<(Area, Mark)> = crate::render::scroll::snapshot().into_iter().map(|r| (covering(r.viewport.scale_from_origin(scale)), Mark::Region)).collect();
        outlines.extend(self.shifted.iter().map(|a| (*a, Mark::Shift)));
        outlines.extend(self.live.iter().map(|l| (area(l.area), Mark::Layer)));
        outlines.extend(self.sprites.iter().filter(|s| !s.covered).map(|s| (covering(s.bounds()), Mark::Sprite)));
        self.overlay.show(canvas, &mut self.stamps, &outlines)
    }
}
