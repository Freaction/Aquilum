use masonry::app::{VisualLayerKind, VisualLayerPlan};
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::imaging::{PaintSink, Painter as Sink};
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::Color;
use vello_cpu::{PixelFormat, PixmapMut, RasterizerSettings, TargetInit};

use std::collections::HashMap;

use super::damage::{Damage, Tracker};
use super::layer::{Cache, Layer, area, area_of};
use super::live;
use super::overlay::{Mark, Overlay};
use super::paint::{PaintError, Painter};
use super::sprite::{self, Sprite};
use super::stamp::{Stamps, covering};
use crate::output::{Area, Canvas, Target};

pub(super) struct Frame<'a> {
    width: u16,
    height: u16,
    scale: f64,
    background: Color,
    base: &'a Scene,
    overlays: Vec<(&'a Scene, Affine)>,
    pub(super) fading: Option<(&'a Scene, Affine, super::fade::Fade, u64)>,
}

impl Frame<'_> {
    pub(super) fn paint_into(&self, sink: &mut dyn PaintSink) {
        Sink::new(sink).fill_rect(Rect::new(0.0, 0.0, f64::from(self.width), f64::from(self.height)), self.background);
        let scale = Affine::scale(self.scale);
        replay_transformed(self.base, sink, scale);
        for (scene, transform) in &self.overlays {
            replay_transformed(scene, sink, scale * *transform);
        }
    }
}

pub struct Renderer {
    pub(super) painter: Painter,
    pub(super) scratch: Vec<u32>,
    tracker: Tracker,
    pub(super) params: Option<(u16, u16, f64, Color)>,
    pub(super) live: Vec<Layer>,
    pub(super) caches: HashMap<u64, Cache>,
    pub(super) frames: HashMap<u64, i64>,
    pub(super) overlay: Overlay,
    pub(super) stamps: Stamps,
    pub(super) faded: Option<super::fade::Cached>,
    sprites: Vec<Sprite>,
    sprite_stamps: Stamps,
    sprite_changes: Option<Area>,
    stamped: Option<Vec<Sprite>>,
    shifted: Vec<Area>,
    #[cfg(test)]
    pub(super) frame: (u16, u16, Vec<u32>),
    #[cfg(test)]
    pub shifts: usize,
    #[cfg(test)]
    pub rastered: Vec<(u16, u16, u16, u16)>,
    #[cfg(test)]
    pub moved: Vec<(u16, u16, u16, u16, i32)>,
}

impl Renderer {
    pub fn new(threads: u16) -> Self {
        Renderer {
            painter: Painter::new(1, 1, super::paint::settings(threads)),
            scratch: Vec::new(),
            tracker: Tracker::new(),
            params: None,
            live: Vec::new(),
            caches: HashMap::new(),
            frames: HashMap::new(),
            overlay: Overlay::default(),
            stamps: Stamps::default(),
            faded: None,
            sprites: Vec::new(),
            sprite_stamps: Stamps::default(),
            sprite_changes: None,
            stamped: None,
            shifted: Vec::new(),
            #[cfg(test)]
            frame: (0, 0, Vec::new()),
            #[cfg(test)]
            shifts: 0,
            #[cfg(test)]
            rastered: Vec::new(),
            #[cfg(test)]
            moved: Vec::new(),
        }
    }

    pub fn toggle_overlay(&mut self) {
        self.overlay.on = !self.overlay.on;
        self.overlay.pending = true;
    }

    pub fn overlay_busy(&self) -> bool {
        self.overlay.on || self.overlay.pending
    }

    pub(super) fn frame<'a>(&self, layers: &'a VisualLayerPlan) -> Option<Frame<'a>> {
        let (width, height, scale, background) = self.params?;
        let Some(VisualLayerKind::Scene(base)) = layers.root_layer().map(|l| &l.kind) else { return None };
        let mut fading = None;
        let mut overlays = Vec::new();
        for layer in layers.overlay_layers() {
            let VisualLayerKind::Scene(scene) = &layer.kind else { continue };
            let id = layer.widget_id.to_raw();
            match super::fade::get(id) {
                Some(fade) if fading.is_none() => fading = Some((scene, layer.transform, fade, id)),
                _ => overlays.push((scene, layer.transform)),
            }
        }
        Some(Frame { width, height, scale, background, base, overlays, fading })
    }

    pub fn trim(&mut self) {
        self.painter.trim();
        self.stamps.forget();
        self.stamps.trim();
        self.sprite_stamps.forget();
        self.sprite_stamps.trim();
        self.stamped = None;
        self.faded = None;
        self.caches.clear();
        self.frames.clear();
        self.scratch = Vec::new();
        self.tracker.forget();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, layers: &VisualLayerPlan, width: u16, height: u16, scale: f64, background: Color, full: bool, canvas: &mut dyn Canvas) -> Result<Vec<Area>, PaintError> {
        self.params = Some((width, height, scale, background));
        if full {
            self.stamps.forget();
            self.sprite_stamps.forget();
            self.stamped = None;
        }
        let seconds = live::seconds();
        let restored = self.stamps.restore(canvas);
        let drawn = self.paint(layers, full, canvas, seconds)?;
        let live = self.composite(layers, canvas, seconds)?;
        self.put_sprites(canvas);
        let sprites = self.sprite_changes.take();
        let faded = self.fade(layers, canvas)?;
        let shown = self.decorate(canvas);
        Ok(drawn.into_iter().chain([restored, live, sprites, faded, shown].into_iter().flatten()).collect())
    }

    pub fn live(&mut self, layers: &VisualLayerPlan, canvas: &mut dyn Canvas) -> Result<Vec<Area>, PaintError> {
        let seconds = live::seconds();
        let restored = self.stamps.restore(canvas);
        let touched = self.due_areas(seconds);
        self.lift_sprites(canvas, &touched);
        let live = self.composite(layers, canvas, seconds)?;
        self.put_sprites(canvas);
        let faded = self.fade(layers, canvas)?;
        let shown = self.decorate(canvas);
        Ok([restored, live, faded, shown].into_iter().flatten().collect())
    }

    fn lift_sprites(&mut self, canvas: &mut dyn Canvas, touched: &[Area]) {
        let visible: Vec<Sprite> = self.sprites.iter().filter(|s| !s.covered).copied().collect();
        if !self.overlay.on && self.stamped.as_ref() == Some(&visible) && self.sprite_stamps.untouched(touched) {
            return;
        }
        self.sprite_stamps.restore(canvas);
        self.stamped = None;
    }

    fn put_sprites(&mut self, canvas: &mut dyn Canvas) {
        if self.stamped.is_some() {
            return;
        }
        sprite::stamp(&self.sprites, canvas, &mut self.sprite_stamps);
        self.stamped = Some(self.sprites.iter().filter(|s| !s.covered).copied().collect());
    }

    fn paint(&mut self, layers: &VisualLayerPlan, full: bool, canvas: &mut dyn Canvas, seconds: f64) -> Result<Vec<Area>, PaintError> {
        let Some((width, height, scale, _)) = self.params else {
            self.lift_sprites(canvas, &[]);
            return Ok(Vec::new());
        };
        let Some(frame) = self.frame(layers) else {
            self.lift_sprites(canvas, &[]);
            return Ok(Vec::new());
        };
        self.painter.regions = super::scroll::snapshot().iter().map(|r| r.viewport.scale_from_origin(scale)).collect();
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
                super::scroll::shift_rows(pixels, usize::from(width), *shift);
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
                let settings = RasterizerSettings { target_init: TargetInit::SrcOver, pixel_format: PixelFormat::Bgra8, offset: (x, y) };
                painter.ctx.render_with(pixmap, &mut painter.resources, settings);
            }
        });
        if result.is_err() {
            self.tracker.forget();
        }
        result.map(|()| dirty)
    }

    fn decorate(&mut self, canvas: &mut dyn Canvas) -> Option<Area> {
        self.overlay.pending = false;
        if !self.overlay.on {
            return self.overlay.show(canvas, &mut self.stamps, &[]);
        }
        let scale = self.params.map_or(1.0, |p| p.2);
        let mut outlines: Vec<(Area, Mark)> = super::scroll::snapshot().into_iter().map(|r| (covering(r.viewport.scale_from_origin(scale)), Mark::Region)).collect();
        outlines.extend(self.shifted.iter().map(|a| (*a, Mark::Shift)));
        outlines.extend(self.live.iter().map(|l| (area(l.area), Mark::Layer)));
        outlines.extend(self.sprites.iter().filter(|s| !s.covered).map(|s| (covering(s.bounds()), Mark::Sprite)));
        self.overlay.show(canvas, &mut self.stamps, &outlines)
    }
}
