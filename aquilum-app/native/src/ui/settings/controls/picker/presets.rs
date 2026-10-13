use masonry::kurbo::{Point, Rect};
use masonry::peniko::Color;

use super::{FIELD, PAD};
use crate::ui::theme;

pub(super) const COUNT: usize = 6;
pub(super) const SWATCH: f64 = 22.0;

pub(super) fn colors() -> [Color; COUNT] {
    let t = theme::current();
    [t.accent_preset_1, t.accent_preset_2, t.accent_preset_3, t.accent_preset_4, t.accent_preset_5, t.accent_preset_6]
}

pub(super) fn hex(color: Color) -> String {
    let rgba = color.to_rgba8();
    format!("#{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b)
}

pub(super) fn swatch(bounds: Rect, index: usize) -> Rect {
    let gap = (FIELD.width - SWATCH * COUNT as f64) / (COUNT - 1) as f64;
    let x = bounds.x0 + PAD + index as f64 * (SWATCH + gap);
    Rect::new(x, bounds.y0 + PAD, x + SWATCH, bounds.y0 + PAD + SWATCH)
}

pub(super) fn at(bounds: Rect, point: Point) -> Option<usize> {
    (0..COUNT).find(|&index| swatch(bounds, index).inflate(2.0, 2.0).contains(point))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_distinct_colors_come_from_the_tokens_and_start_with_the_default_accent() {
        let list: Vec<String> = colors().into_iter().map(hex).collect();
        assert_eq!(list.len(), COUNT);
        assert_eq!(list[0], "#1471eb");
        let _lock = crate::ui::theme::test_lock();
        let before = theme::is_dark();
        theme::apply_setting("dark", true, "#1471eb");
        let dark: Vec<String> = colors().into_iter().map(hex).collect();
        theme::apply_setting(if before { "dark" } else { "light" }, before, "#1471eb");
        assert_eq!(dark, list, "готовые цвета не должны зависеть от темы");
        let mut unique = list.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), COUNT, "{list:?}");
    }

    #[test]
    fn swatches_sit_in_one_row_inside_the_card_and_are_found_by_a_click() {
        let bounds = Rect::new(0.0, 0.0, 232.0, 220.0);
        let first = swatch(bounds, 0);
        let last = swatch(bounds, COUNT - 1);
        assert_eq!((first.x0, first.y0), (PAD, PAD));
        assert_eq!(last.x1, PAD + FIELD.width);
        for index in 0..COUNT {
            assert_eq!(at(bounds, swatch(bounds, index).center()), Some(index));
        }
        assert_eq!(at(bounds, Point::new(5.0, 5.0)), None);
        assert_eq!(at(bounds, Point::new(116.0, 100.0)), None);
    }
}
