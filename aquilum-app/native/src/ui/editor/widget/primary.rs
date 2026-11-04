use super::*;
use crate::ui::keymap::pointer;
use masonry::core::PointerButtonEvent;

impl TextEditor {
    pub(super) fn select_primary(&self) {
        if let Some(text) = self.selected_text() {
            crate::clipboard::select(text);
        }
    }

    pub(super) fn paste_primary(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) -> bool {
        let PointerEvent::Down(PointerButtonEvent { button, state, .. }) = event else { return false };
        if !pointer::paste_selection(*button) {
            return false;
        }
        let target = self.hit(ctx.local_position(state.position));
        self.move_to(target, false);
        if let Some(text) = crate::clipboard::selection() {
            self.insert(&text, Kind::Other);
            self.changed(ctx);
        }
        ctx.request_focus();
        ctx.set_handled();
        true
    }
}
