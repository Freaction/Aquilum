mod pass;

use masonry::app::{VisualLayerKind, VisualLayerPlan};
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::imaging::{PaintSink, Painter as Sink};
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::Color;

use std::collections::HashMap;

use super::damage::Tracker;
use super::layer::{Cache, Layer};
use super::live;
use super::overlay::Overlay;
use super::paint::{PaintError, Painter};
use super::sprite::Sprite;
use super::stamp::Stamps;
use crate::output::{Area, Canvas};

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
}
