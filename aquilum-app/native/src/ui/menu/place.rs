use masonry::core::WidgetMut;
use masonry::kurbo::{Point, Rect, Size};

use super::Menu;
use crate::ui::tokens::size;

impl Menu {
    pub fn set_max_height(this: &mut WidgetMut<'_, Self>, height: f64) {
        if height < this.widget.max_height {
            this.widget.max_height = height;
            this.ctx.request_layout();
        }
    }

    pub fn dropdown_origin(trigger: Rect, window: Size, width: f64) -> Point {
        let right = (window.width - width - size::SPACE_8).max(size::SPACE_8);
        Point::new((trigger.x1 - width).clamp(size::SPACE_8, right), trigger.y1 + size::SPACE_4)
    }
}
