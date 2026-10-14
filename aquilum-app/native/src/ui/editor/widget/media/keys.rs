use masonry::core::keyboard::KeyboardEvent;

use super::*;

impl TextEditor {
    pub(in crate::ui::editor::widget) fn media_key(&mut self, ctx: &mut EventCtx<'_>, event: &KeyboardEvent) -> bool {
        let target = self.media.crop.map(|(s, _)| s).or(self.media.selected);
        let Some(start) = target else { return false };
        if event.state != KeyState::Down {
            return true;
        }
        let cropping = self.media.crop.is_some();
        match &event.key {
            Key::Named(NamedKey::Enter) if cropping => {
                self.end_crop(true);
                self.changed(ctx);
            }
            Key::Named(NamedKey::Escape) if cropping => {
                self.end_crop(false);
                ctx.request_layout();
            }
            Key::Named(NamedKey::Enter) => self.show_source(ctx, start),
            Key::Named(NamedKey::Escape) => self.media.selected = None,
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
                let p = self.doc.paragraph(self.doc.at_offset(start));
                let range = p.start..(p.end() + 1).min(self.doc.len());
                self.media.selected = None;
                self.media.hover = None;
                self.replace(range, "", Kind::Other);
                self.changed(ctx);
            }
            _ => {
                self.media.selected = None;
                ctx.request_render();
                return false;
            }
        }
        ctx.set_handled();
        ctx.request_render();
        true
    }
}
