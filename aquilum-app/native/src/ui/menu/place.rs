use masonry::kurbo::{Point, Rect, Size};

use super::Menu;
use crate::ui::tokens::size;

impl Menu {
    pub fn around(trigger: Rect, menu: Size, area: Rect) -> Point {
        let below = trigger.y1 + size::SPACE_4;
        let y = if below + menu.height > area.y1 - size::SPACE_8 { trigger.y0 - size::SPACE_4 - menu.height } else { below };
        let x = trigger.x0.min(area.x1 - menu.width - size::SPACE_8).max(area.x0 + size::SPACE_8);
        Point::new(x, y.min(area.y1 - menu.height - size::SPACE_8).max(area.y0 + size::SPACE_8)).round()
    }

    pub fn dropdown_origin(trigger: Rect, window: Size, width: f64) -> Point {
        let right = (window.width - width - size::SPACE_8).max(size::SPACE_8);
        Point::new((trigger.x1 - width).clamp(size::SPACE_8, right), trigger.y1 + size::SPACE_4)
    }
}
