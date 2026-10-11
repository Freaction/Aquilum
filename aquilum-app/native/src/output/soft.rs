use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use super::{Area, Canvas, Target};

pub struct Output {
    surface: Surface<OwnedDisplayHandle, Rc<dyn Window>>,
    width: u32,
    fresh: bool,
}

impl Output {
    pub fn new(display: OwnedDisplayHandle, window: Rc<dyn Window>) -> Result<Self, String> {
        let context = Context::new(display).map_err(|e| format!("softbuffer: {e}"))?;
        let surface = Surface::new(&context, window).map_err(|e| format!("softbuffer: {e}"))?;
        Ok(Output { surface, width: 0, fresh: true })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else { return Ok(()) };
        self.surface.resize(w, h).map_err(|e| format!("softbuffer: {e}"))?;
        let age = self.surface.buffer_mut().map_err(|e| format!("softbuffer: {e}"))?.age();
        self.fresh = self.width != width || age != 1;
        self.width = width;
        Ok(())
    }

    pub fn trim(&mut self) {
        let one = NonZeroU32::MIN;
        let _ = self.surface.resize(one, one);
        self.width = 0;
        self.fresh = true;
    }

    pub fn set_background(&mut self, _bgra: u32) {}

    pub fn fresh(&self) -> bool {
        self.fresh
    }


    pub fn present(&mut self, dirty: &[Area]) {
        let Ok(buffer) = self.surface.buffer_mut() else { return };
        let mut areas = dirty.to_vec();
        super::merge(&mut areas);
        let rects: Vec<softbuffer::Rect> = areas.iter().filter_map(|a| Some(softbuffer::Rect { x: a.x, y: a.y, width: NonZeroU32::new(a.w)?, height: NonZeroU32::new(a.h)? })).collect();
        let _ = buffer.present_with_damage(&rects);
        self.fresh = false;
    }
}

impl Canvas for Output {
    fn with_target(&mut self, paint: &mut dyn FnMut(Target<'_>)) {
        let width = self.width as u16;
        let Ok(mut buffer) = self.surface.buffer_mut() else { return };
        let height = (buffer.len() / usize::from(width.max(1))) as u16;
        paint(Target { pixels: &mut buffer, width, height });
    }
}
