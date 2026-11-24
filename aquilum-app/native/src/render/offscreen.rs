use std::cell::RefCell;

use super::paint::Painter;

thread_local! {
    static PAINTER: RefCell<Option<Painter>> = const { RefCell::new(None) };
}

pub fn offscreen(width: u16, height: u16, draw: impl FnOnce(&mut masonry::imaging::Painter<'_, Painter>)) -> Option<masonry::peniko::ImageBrush> {
    if width == 0 || height == 0 {
        return None;
    }
    PAINTER.with_borrow_mut(|kept| {
        let sink = kept.get_or_insert_with(|| Painter::new(width, height, super::render_settings(0)));
        sink.begin(0, 0, width, height);
        draw(&mut masonry::imaging::Painter::new(sink));
        sink.finish().ok()?;
        Some(super::brush(sink.pixmap()))
    })
}

pub(super) fn trim() {
    PAINTER.take();
}
