use std::path::Path;

use masonry::peniko::ImageBrush;

pub struct Engine;

impl Engine {
    pub fn open(_path: &Path) -> Option<Engine> {
        None
    }

    pub fn toggle(&mut self) {}

    pub fn seek(&mut self, _fraction: f64) {}

    pub fn mute(&mut self) {}

    pub fn status(&mut self) -> (bool, f64, f64, bool) {
        (false, 0.0, 0.0, false)
    }

    pub fn frame(&mut self) -> Option<ImageBrush> {
        None
    }
}
