//! Мелкие элементы оформления: кнопка-иконка (`IconButton.tsx`), кнопка в стиле строки дерева
//! (переключатель базы в подвале), граница с одной стороны.

mod buttons;
mod appear;
mod caption;
mod collapse;
mod document_item;
mod items;
mod kbd;
mod label;
mod layout;
mod sash;
mod visible;

pub use buttons::*;
pub use appear::*;
pub use caption::Caption;
pub use collapse::*;
pub use document_item::*;
pub use items::*;
pub use kbd::kbd;
pub use label::{Font, Label, Leading, Paint, Scale};
pub use layout::*;
pub use sash::{Edge, Edged, Sash, SashAction};
pub use visible::Visible;

use masonry::core::{EventCtx, PointerButton, PointerButtonEvent, PointerEvent};

/// Нажатие кнопки.
#[derive(Debug)]
pub struct Pressed;

/// Общая логика нажатия: захват указателя, действие при отпускании над кнопкой.
#[derive(Default)]
pub struct Press {
    pub down: bool,
}

impl Press {
    pub fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) -> bool {
        self.click(ctx, event).is_some()
    }

    pub fn click<'e>(&mut self, ctx: &mut EventCtx<'_>, event: &'e PointerEvent) -> Option<&'e PointerButtonEvent> {
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.down = true;
                ctx.capture_pointer();
                ctx.request_paint_only();
                None
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.down => {
                self.down = false;
                ctx.request_paint_only();
                ctx.is_hovered().then_some(e)
            }
            PointerEvent::Cancel(_) => {
                self.down = false;
                None
            }
            _ => None,
        }
    }
}
