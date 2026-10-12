use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use super::{Area, Canvas, Target};

pub struct Output {
    surface: Surface<OwnedDisplayHandle, Rc<dyn Window>>,
    pixels: Vec<u32>,
    width: u32,
    height: u32,
    fresh: bool,
    background: u32,
}

impl Output {
    pub fn new(display: OwnedDisplayHandle, window: Rc<dyn Window>) -> Result<Self, String> {
        let context = Context::new(display).map_err(|e| format!("softbuffer: {e}"))?;
        let surface = Surface::new(&context, window).map_err(|e| format!("softbuffer: {e}"))?;
        Ok(Output { surface, pixels: Vec::new(), width: 0, height: 0, fresh: true, background: 0 })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else { return Ok(()) };
        if self.width == width && self.height == height && !self.pixels.is_empty() {
            return Ok(());
        }
        self.surface.resize(w, h).map_err(|e| format!("softbuffer: {e}"))?;
        let len = (width * height) as usize;
        self.pixels.resize(len, self.background);
        self.pixels.fill(self.background);
        self.width = width;
        self.height = height;
        self.fresh = true;
        Ok(())
    }

    pub fn trim(&mut self) {
        let one = NonZeroU32::MIN;
        let _ = self.surface.resize(one, one);
        self.pixels = Vec::new();
        self.width = 0;
        self.height = 0;
        self.fresh = true;
    }

    pub fn set_background(&mut self, bgra: u32) {
        if self.background != bgra {
            self.background = bgra;
            self.pixels.fill(bgra);
            self.fresh = true;
        }
    }

    pub fn fresh(&self) -> bool {
        self.fresh
    }

    pub fn present(&mut self, dirty: &[Area]) {
        let Ok(mut buffer) = self.surface.buffer_mut() else { return };
        if buffer.len() == self.pixels.len() {
            buffer.copy_from_slice(&self.pixels);
        }
        let mut areas = dirty.to_vec();
        super::merge(&mut areas);
        let rects: Vec<softbuffer::Rect> = areas.iter().filter_map(|a| Some(softbuffer::Rect { x: a.x, y: a.y, width: NonZeroU32::new(a.w)?, height: NonZeroU32::new(a.h)? })).collect();
        let _ = buffer.present_with_damage(&rects);
        self.fresh = false;
    }
}

impl Canvas for Output {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        let (width, height) = (self.width as u16, self.height as u16);
        paint(Target { pixels: &mut self.pixels, width, height });
    }
}

