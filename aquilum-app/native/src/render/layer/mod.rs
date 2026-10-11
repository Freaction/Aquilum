//! Кадр живого слоя: кэш «под» меткой + узор + кэш «над» меткой, без сцены Masonry.

mod split;

use masonry::app::VisualLayerPlan;
use masonry::kurbo::{Affine, Rect, RoundedRect};

use super::damage::Damage;
use super::live::{self, Motion};
use super::paint::PaintError;
use super::renderer::Renderer;
use crate::cover::over;
use crate::output::{Area, Canvas, Target};
use split::Part;

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Layer {
    pub id: u64,
    pub area: (u16, u16, u16, u16),
    pub transform: Affine,
    pub shape: RoundedRect,
}

pub(super) struct Cache {
    area: (u16, u16, u16, u16),
    below: Vec<u32>,
    above: Vec<u32>,
    spans: Vec<(usize, usize)>,
}

fn spans(above: &[u32], width: usize) -> Vec<(usize, usize)> {
    above
        .chunks_exact(width)
        .map(|row| {
            let start = row.iter().position(|p| p >> 24 != 0).unwrap_or(width);
            let end = row.iter().rposition(|p| p >> 24 != 0).map_or(start, |i| i + 1);
            (start, end)
        })
        .collect()
}

pub(super) fn area_of(rect: Rect, frame: Rect) -> Option<(u16, u16, u16, u16)> {
    let r = rect.expand().intersect(frame);
    let r = Rect::new((r.x0 / 4.0).floor() * 4.0, (r.y0 / 4.0).floor() * 4.0, r.x1, r.y1);
    (r.width() > 0.0 && r.height() > 0.0).then(|| (r.x0 as u16, r.y0 as u16, r.width() as u16, r.height() as u16))
}

pub(super) fn area((x, y, w, h): (u16, u16, u16, u16)) -> Area {
    Area { x: u32::from(x), y: u32::from(y), w: u32::from(w), h: u32::from(h) }
}

fn touches(damage: &Damage, (x, y, w, h): (u16, u16, u16, u16)) -> bool {
    let layer = Rect::new(f64::from(x), f64::from(y), f64::from(x) + f64::from(w), f64::from(y) + f64::from(h));
    damage.areas.iter().copied().chain(damage.moves.iter().map(|m| (m.x, m.y, m.w, m.h))).any(|(dx, dy, dw, dh)| {
        let i = layer.intersect(Rect::new(f64::from(dx), f64::from(dy), f64::from(dx) + f64::from(dw), f64::from(dy) + f64::from(dh)));
        i.width() > 0.0 && i.height() > 0.0
    })
}

impl Renderer {
    pub fn has_live(&self) -> bool {
        !self.live.is_empty()
    }

    pub fn next_live(&self) -> Option<std::time::Duration> {
        let seconds = live::seconds();
        self.live
            .iter()
            .filter_map(|layer| live::motion(layer.id))
            .map(|motion| {
                let fps = crate::cover::motion_fps(motion.pattern);
                ((live::frame(motion, seconds) + 1) as f64 / fps - seconds).max(0.0)
            })
            .reduce(f64::min)
            .map(std::time::Duration::from_secs_f64)
    }

    pub(super) fn refresh_live(&mut self, layers: Vec<Layer>, damage: &mut Damage, full: bool, seconds: f64) {
        self.live = layers;
        let live = &self.live;
        self.caches.retain(|id, _| live.iter().any(|l| l.id == *id && !full && !touches(damage, l.area)));
        self.frames.retain(|id, _| live.iter().any(|l| l.id == *id));
        for layer in &self.live {
            let Some(motion) = live::motion(layer.id) else { continue };
            let index = live::frame(motion, seconds);
            let touched = full || touches(damage, layer.area);
            if touched && self.frames.get(&layer.id) != Some(&index) {
                damage.areas.push(layer.area);
                self.frames.insert(layer.id, index);
            }
        }
    }

    fn due(&self, seconds: f64) -> Vec<(Layer, Motion, i64)> {
        self.live
            .iter()
            .filter_map(|layer| {
                let motion = live::motion(layer.id)?;
                let frame = live::frame(motion, seconds);
                (self.frames.get(&layer.id) != Some(&frame)).then_some((*layer, motion, frame))
            })
            .collect()
    }

    pub(super) fn due_areas(&self, seconds: f64) -> Vec<Area> {
        self.due(seconds).iter().map(|(layer, _, _)| area(layer.area)).collect()
    }

    pub(super) fn composite(&mut self, layers: &VisualLayerPlan, canvas: &mut dyn Canvas, seconds: f64) -> Result<Option<Area>, PaintError> {
        let _zone = aq_trace::zone("живой слой");
        let due = self.due(seconds);
        let Some(frame) = due.first().and_then(|_| self.frame(layers)) else { return Ok(None) };
        let mut bounds: Option<Area> = None;
        for (layer, motion, index) in due {
            let cache = match self.caches.remove(&layer.id).filter(|c| c.area == layer.area) {
                Some(cache) => cache,
                None => {
                    let _zone = aq_trace::zone("живой слой: кэши");
                    let below = self.part(&frame, layer.id, layer.area, Part::Below, seconds)?;
                    let above = self.part(&frame, layer.id, layer.area, Part::Above, seconds)?;
                    let spans = spans(&above, usize::from(layer.area.2));
                    Cache { area: layer.area, below, above, spans }
                }
            };
            let (x, y, w, h) = layer.area;
            {
                let _zone = aq_trace::zone("живой слой: узор");
                let rect = Rect::new(f64::from(x), f64::from(y), f64::from(x) + f64::from(w), f64::from(y) + f64::from(h));
                live::draw(layer.transform, layer.shape, motion, rect, seconds, &mut self.scratch);
            }
            let motion_pixels = &self.scratch;
            let _zone = aq_trace::zone("живой слой: холст");
            canvas.with_target(&mut |Target { pixels, width, .. }| {
                let stride = usize::from(width);
                let (x, y, w) = (usize::from(x), usize::from(y), usize::from(w));
                for (row, ((below, top), (above, &(from, to)))) in cache.below.chunks_exact(w).zip(motion_pixels.chunks_exact(w)).zip(cache.above.chunks_exact(w).zip(&cache.spans)).enumerate() {
                    let line = &mut pixels[(y + row) * stride + x..][..w];
                    line.copy_from_slice(below);
                    for (dest, &source) in line.iter_mut().zip(top) {
                        if source != 0 {
                            *dest = over((source & 0xff00_ff00) | (source & 0xff) << 16 | (source >> 16) & 0xff, *dest);
                        }
                    }
                    for (dest, &source) in line[from..to].iter_mut().zip(&above[from..to]) {
                        if source >> 24 != 0 {
                            *dest = over(source, *dest);
                        }
                    }
                }
            });
            self.caches.insert(layer.id, cache);
            self.frames.insert(layer.id, index);
            self.overlay.flash(area(layer.area), super::overlay::Mark::Live);
            bounds = Some(bounds.map_or(area(layer.area), |b| b.union(area(layer.area))));
        }
        Ok(bounds)
    }
}
