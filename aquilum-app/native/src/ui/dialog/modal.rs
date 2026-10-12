use masonry::accesskit::Role;
use masonry::core::{NewWidget, WidgetMut};
use masonry::kurbo::{Point, Rect, Size};

use super::{CardSize, Modal};
use crate::ui::widgets::{Intro, NORMAL};

impl Drop for Modal {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            crate::render::fade::forget(id);
        }
    }
}

impl Modal {
    pub fn new(card: NewWidget<masonry::widgets::Flex>, title: &str, size: CardSize, role: Role, enter_confirms: bool) -> NewWidget<Self> {
        NewWidget::new(Modal { card: card.to_pod(), title: title.to_owned(), size, role, enter_confirms, card_rect: Rect::ZERO, intro: Intro::new(NORMAL), closing: false, id: None })
    }

    pub fn close(this: &mut WidgetMut<'_, Self>) {
        if this.widget.closing {
            return;
        }
        this.widget.closing = true;
        this.widget.intro.reverse();
        this.ctx.request_anim_frame();
        this.ctx.request_paint_only();
    }

    pub fn set_card(this: &mut WidgetMut<'_, Self>, card: NewWidget<masonry::widgets::Flex>, title: &str) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.card, card.to_pod()));
        this.widget.title = title.to_owned();
        this.ctx.children_changed();
        this.ctx.request_layout();
        this.ctx.request_paint_only();
        this.ctx.request_accessibility_update();
    }

    pub(super) fn centered(window: Size, card: Size) -> Rect {
        let origin = Point::new(((window.width - card.width) / 2.0).round(), ((window.height - card.height) / 2.0).round());
        Rect::from_origin_size(origin, card)
    }
}
