use masonry::core::{EventCtx, PointerEvent};

use super::{Kind, RowAction, WorkspaceRow};
use crate::ui::keymap::pointer;

impl WorkspaceRow {
    pub(super) fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        let menu = self.has_menu();
        let over = move |ctx: &EventCtx<'_>, at| menu && Self::more_rect(ctx.content_box()).contains(ctx.local_position(at));
        match event {
            PointerEvent::Move(update) if over(ctx, update.current.position) != self.more_hovered => {
                self.more_hovered = !self.more_hovered;
                ctx.request_paint_only();
            }
            PointerEvent::Down(e) if pointer::context(e.button) && self.has_menu() => {
                let at = ctx.to_window(ctx.local_position(e.state.position));
                ctx.submit_action::<RowAction>(RowAction::Menu(at));
            }
            _ => {}
        }
        if let Some(e) = self.press.click(ctx, event) {
            if over(ctx, e.state.position) {
                ctx.submit_action::<RowAction>(RowAction::Menu(Self::menu_at(ctx)));
            } else if self.kind != Kind::Current && self.field.is_none() {
                ctx.submit_action::<RowAction>(RowAction::Open);
            }
        }
    }
}
