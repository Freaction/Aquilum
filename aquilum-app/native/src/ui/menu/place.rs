use masonry::core::WidgetMut;
use masonry::kurbo::{Point, Rect, Size};

use super::Menu;
use crate::ui::tokens::size;

impl Menu {
    pub fn around(trigger: Rect, menu: Size, area: Rect) -> (Point, f64) {
        let (gap, margin) = (size::SPACE_4, size::SPACE_8);
        let below_from = (trigger.y1 + gap).max(area.y0 + margin);
        let above_to = (trigger.y0 - gap).min(area.y1 - margin);
        let (below, above) = (area.y1 - margin - below_from, above_to - area.y0 - margin);
        let down = menu.height <= below || below >= above;
        let height = menu.height.min(if down { below } else { above }).max(0.0);
        let y = if down { below_from } else { above_to - height };
        let x = trigger.x0.min(area.x1 - menu.width - margin).max(area.x0 + margin);
        (Point::new(x, y).round(), height)
    }

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
