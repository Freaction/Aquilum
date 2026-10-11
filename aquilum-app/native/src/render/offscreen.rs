//! Отрисовка элементов интерфейса во внеэкранные растровые кисти peniko.

use vello_cpu::{Pixmap, RasterizerSettings};

use super::paint::Painter;

pub fn offscreen(width: u16, height: u16, draw: impl FnOnce(&mut masonry::imaging::Painter<'_, Painter>)) -> Option<masonry::peniko::ImageBrush> {
    if width == 0 || height == 0 {
        return None;
    }
    let mut sink = Painter::new(width, height, super::paint::settings(0));
    sink.begin(0, 0, width, height);
    draw(&mut masonry::imaging::Painter::new(&mut sink));
    sink.finish().ok()?;
    let mut pixmap = Pixmap::new(width, height);
    sink.ctx.render_with(&mut pixmap, &mut sink.resources, RasterizerSettings::default());
    Some(super::brush(pixmap))
}
