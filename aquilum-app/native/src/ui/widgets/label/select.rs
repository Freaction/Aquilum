use masonry::core::keyboard::{Key, KeyState};
use masonry::core::{BrushIndex, EventCtx, PointerButton, PointerButtonEvent, PointerEvent, PointerUpdate, TextEvent};
use masonry::imaging::Painter;
use masonry::kurbo::{Rect, Vec2};
use masonry::parley::{Affinity, Cursor, Layout, Selection};

use super::Label;
use crate::ui::theme;

fn whole(layout: &Layout<BrushIndex>, len: usize) -> Selection {
    Selection::new(Cursor::from_byte_index(layout, 0, Affinity::Downstream), Cursor::from_byte_index(layout, len, Affinity::Upstream))
}

pub(super) fn paint_selection(painter: &mut Painter<'_>, layout: &Layout<BrushIndex>, selection: Selection, origin: Vec2) {
    if selection.is_collapsed() {
        return;
    }
    let fill = theme::current().selection_bg;
    for (rect, _) in selection.geometry(layout) {
        painter.fill(Rect::new(rect.x0, rect.y0, rect.x1, rect.y1) + origin, fill).draw();
    }
}

impl Label {
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }

    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        let Some((_, layout)) = &self.cached else { return };
        let origin = ctx.content_box().origin().to_vec2();
        let local = |position| {
            let point = ctx.local_position(position) - origin;
            (point.x as f32, point.y as f32)
        };
        match event {
            PointerEvent::Down(PointerButtonEvent { button: None | Some(PointerButton::Primary), state, .. }) => {
                let (x, y) = local(state.position);
                self.selection = match state.count {
                    1 => Selection::from_point(layout, x, y),
                    2 => Selection::word_from_point(layout, x, y),
                    _ => whole(layout, self.text.len()),
                };
                ctx.request_focus();
                ctx.capture_pointer();
                ctx.request_paint_only();
                ctx.set_handled();
            }
            PointerEvent::Move(PointerUpdate { current, .. }) if ctx.is_active() => {
                let (x, y) = local(current.position);
                self.selection = self.selection.extend_to_point(layout, x, y);
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    pub(super) fn keyboard(&mut self, ctx: &mut EventCtx<'_>, event: &TextEvent) {
        let TextEvent::Keyboard(key) = event else { return };
        let action = if cfg!(target_os = "macos") { key.modifiers.meta() } else { key.modifiers.ctrl() };
        let Key::Character(letter) = &key.key else { return };
        if key.state != KeyState::Down || !action {
            return;
        }
        if letter.eq_ignore_ascii_case("c") {
            if let Some(text) = self.text.get(self.selection.text_range()).filter(|text| !text.is_empty()) {
                ctx.set_clipboard(text.to_owned());
            }
            ctx.set_handled();
        } else if letter.eq_ignore_ascii_case("a")
            && let Some((_, layout)) = &self.cached
        {
            self.selection = whole(layout, self.text.len());
            ctx.request_paint_only();
            ctx.set_handled();
        }
    }
}
