use masonry::core::keyboard::KeyboardEvent;

use super::*;
use crate::ui::keymap::{self, Command, Scope};

impl TextEditor {
    pub(in crate::ui::editor::widget) fn media_key(&mut self, ctx: &mut EventCtx<'_>, event: &KeyboardEvent) -> bool {
        let target = self.media.crop.map(|(s, _)| s).or(self.media.selected);
        let Some(start) = target else { return false };
        if event.state != KeyState::Down {
            return true;
        }
        let cropping = self.media.crop.is_some();
        match keymap::command(&[Scope::Image], &event.key, event.modifiers) {
            Some(Command::ImageSource) if cropping => {
                self.end_crop(true);
                self.changed(ctx);
            }
            Some(Command::Close) if cropping => {
                self.end_crop(false);
                ctx.request_layout();
            }
            Some(Command::ImageSource) => self.show_source(ctx, start),
            Some(Command::Close) => self.media.selected = None,
            Some(Command::ImageDelete) => {
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
