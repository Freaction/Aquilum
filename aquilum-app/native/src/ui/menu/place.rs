use masonry::kurbo::{Point, Rect, Size};

use super::Menu;
use super::item::MenuItem;
use crate::ui::tokens::size;

impl Menu {
    pub fn context_origin(at: Point, items: usize, window: Size) -> Point {
        let size = Self::estimated_size(items);
        let x = if at.x + size.width > window.width { (at.x - size.width).max(size::SPACE_8) } else { at.x };
        let y = if at.y + size.height > window.height { (at.y - size.height).max(size::SPACE_8) } else { at.y };
        Point::new(x, y)
    }

    pub fn list_origin(trigger: Rect, items: usize, window: Size) -> Point {
        let height = Self::estimated_size(items).height.min(Self::LIST_MAX_HEIGHT);
        let below = trigger.y1 + size::SPACE_4;
        let y = if below + height > window.height - size::SPACE_8 { (trigger.y0 - size::SPACE_4 - height).max(size::SPACE_8) } else { below };
        Point::new(trigger.x0, y)
    }

    pub fn dropdown_origin(trigger: Rect, window: Size, width: f64) -> Point {
        let right = (window.width - width - size::SPACE_8).max(size::SPACE_8);
        Point::new((trigger.x1 - width).clamp(size::SPACE_8, right), trigger.y1 + size::SPACE_4)
    }

    pub fn estimated_size(items: usize) -> Size {
        let items = items as f64;
        let height = items * MenuItem::height() + (items + 1.0) * size::GAP_XS;
        Size::new(Self::MIN_WIDTH + 32.0, height)
    }
}
