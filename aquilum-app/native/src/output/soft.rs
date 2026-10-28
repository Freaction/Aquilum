use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use super::{Area, Canvas, History, Pixels, Target};

pub struct Output {
    surface: Surface<OwnedDisplayHandle, Rc<dyn Window>>,
    pixels: Pixels,
    history: History,
}

impl Output {
    pub fn new(display: OwnedDisplayHandle, window: Rc<dyn Window>) -> Result<Self, String> {
        let context = Context::new(display).map_err(|e| format!("softbuffer: {e}"))?;
        let surface = Surface::new(&context, window).map_err(|e| format!("softbuffer: {e}"))?;
        Ok(Output { surface, pixels: Pixels::new(), history: History::default() })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else { return Ok(()) };
        if self.pixels.sized(width, height) {
            return Ok(());
        }
        self.surface.resize(w, h).map_err(|e| format!("softbuffer: {e}"))?;
        self.pixels.resize(width, height);
        self.history.reset();
        Ok(())
    }

    pub fn trim(&mut self) {
        let one = NonZeroU32::MIN;
        let _ = self.surface.resize(one, one);
        self.pixels.trim();
    }

    pub fn set_background(&mut self, bgra: u32) {
        self.pixels.set_background(bgra);
    }

    pub fn fresh(&self) -> bool {
        self.pixels.fresh
    }

    pub fn present(&mut self, dirty: &[Area]) {
        let Ok(mut buffer) = self.surface.buffer_mut() else { return };
        if buffer.len() == self.pixels.data.len() {
            let age = if self.pixels.fresh { 0 } else { usize::from(buffer.age()) };
            let (areas, missing) = self.history.frame(dirty, age);
            match missing {
                Some(missing) => {
                    for area in missing {
                        self.pixels.copy_to(&mut buffer, self.pixels.width as usize, area);
                    }
                }
                None => buffer.copy_from_slice(&self.pixels.data),
            }
            let rects: Vec<softbuffer::Rect> = areas.iter().filter_map(|a| Some(softbuffer::Rect { x: a.x, y: a.y, width: NonZeroU32::new(a.w)?, height: NonZeroU32::new(a.h)? })).collect();
            let _ = buffer.present_with_damage(&rects);
        }
        self.pixels.fresh = false;
    }
}

impl Canvas for Output {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        self.pixels.with_target(paint);
    }
}
