use masonry::core::{EventCtx, PointerButton, PointerEvent};
use masonry::kurbo::Point;

use super::{RowAction, TreeRow};
use crate::ui::file_tree::model::Modifiers;

const DRAG_THRESHOLD: f64 = 5.0;

impl TreeRow {
    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.pressed = true;
                self.dragging = false;
                self.down_at = Some(logical(e.state.position, e.state.scale_factor));
                ctx.capture_pointer();
            }
            PointerEvent::Move(update) if self.pressed => {
                let at = logical(update.current.position, update.current.scale_factor);
                if !self.dragging
                    && let Some(start) = self.down_at
                    && ((at.x - start.x).abs() >= DRAG_THRESHOLD || (at.y - start.y).abs() >= DRAG_THRESHOLD)
                {
                    self.dragging = true;
                    ctx.submit_action::<RowAction>(RowAction::DragStart(at));
                }
                if self.dragging {
                    ctx.submit_action::<RowAction>(RowAction::DragMove(at));
                }
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.pressed && self.dragging => {
                self.pressed = false;
                self.dragging = false;
                let at = logical(e.state.position, e.state.scale_factor);
                ctx.submit_action::<RowAction>(RowAction::Drop(at));
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.pressed => {
                self.pressed = false;
                if ctx.is_hovered() {
                    let m = e.state.modifiers;
                    let action = crate::ui::keymap::action(m);
                    ctx.submit_action::<RowAction>(RowAction::Press(Modifiers { action, shift: m.shift() }));
                }
            }
            PointerEvent::Down(e) if e.button == Some(PointerButton::Secondary) => {
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
