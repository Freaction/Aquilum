use masonry::core::{EventCtx, PointerEvent};
use masonry::kurbo::Point;

use super::{RowAction, TreeRow};
use crate::ui::file_tree::model::Modifiers;
use crate::ui::keymap::pointer;

impl TreeRow {
    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) if pointer::press(e.button) => {
                self.pressed = true;
                self.dragging = false;
                self.down_at = Some(logical(e.state.position, e.state.scale_factor));
                ctx.capture_pointer();
            }
            PointerEvent::Move(update) if self.pressed => {
                let at = logical(update.current.position, update.current.scale_factor);
                let slop = pointer::drag_slop(update.pointer.pointer_type);
                if !self.dragging
                    && let Some(start) = self.down_at
                    && ((at.x - start.x).abs() >= slop || (at.y - start.y).abs() >= slop)
                {
                    self.dragging = true;
                    ctx.submit_action::<RowAction>(RowAction::DragStart(at));
                }
                if self.dragging {
                    ctx.submit_action::<RowAction>(RowAction::DragMove(at));
                }
            }
            PointerEvent::Up(e) if pointer::press(e.button) && self.pressed && self.dragging => {
                self.pressed = false;
                self.dragging = false;
                let at = logical(e.state.position, e.state.scale_factor);
                ctx.submit_action::<RowAction>(RowAction::Drop(at));
            }
            PointerEvent::Up(e) if pointer::press(e.button) && self.pressed => {
                self.pressed = false;
                if ctx.is_hovered() {
                    let m = e.state.modifiers;
                    ctx.submit_action::<RowAction>(RowAction::Press(Modifiers { action: pointer::toggle(m), shift: crate::ui::keymap::extend(m) }));
                }
            }
            PointerEvent::Down(e) if pointer::context(e.button) => {
                let at = logical(e.state.position, e.state.scale_factor);
                ctx.submit_action::<RowAction>(RowAction::ContextMenu(at));
            }
            PointerEvent::Cancel(_) => {
                self.pressed = false;
                self.dragging = false;
            }
            _ => {}
        }
    }
}

fn logical(position: masonry::dpi::PhysicalPosition<f64>, scale: f64) -> Point {
    Point::new(position.x / scale, position.y / scale)
}
