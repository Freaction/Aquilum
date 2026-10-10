use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use super::Area;

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

    pub fn fresh(&self) -> bool {
        self.fresh
    }

    pub fn upload(&mut self, area: Area, pixels: &[u32]) {
        let width = self.width as usize;
        let Ok(mut buffer) = self.surface.buffer_mut() else { return };
        for (row, line) in pixels.chunks_exact(area.w as usize).enumerate() {
            let start = (area.y as usize + row) * width + area.x as usize;
            buffer[start..start + line.len()].copy_from_slice(line);
        }
    }


    pub fn present(&mut self, dirty: Area) {
        let Ok(buffer) = self.surface.buffer_mut() else { return };
        let (Some(w), Some(h)) = (NonZeroU32::new(dirty.w), NonZeroU32::new(dirty.h)) else { return };
        let _ = buffer.present_with_damage(&[softbuffer::Rect { x: dirty.x, y: dirty.y, width: w, height: h }]);
        self.fresh = false;
    }
}
