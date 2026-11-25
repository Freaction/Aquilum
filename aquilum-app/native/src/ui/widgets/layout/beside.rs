use masonry::kurbo::{Axis, Point, Rect, Size};

use crate::ui::tokens::size;

fn span(rect: Rect, axis: Axis) -> (f64, f64) {
    (Point::new(rect.x0, rect.y0).get_coord(axis), Point::new(rect.x1, rect.y1).get_coord(axis))
}

pub fn beside(trigger: Rect, wanted: Size, area: Rect, axis: Axis, centered: bool) -> (Point, f64) {
    let (gap, margin) = (size::SPACE_4, size::SPACE_8);
    let ((near, far), (start, end)) = (span(trigger, axis), span(area, axis));
    let after_from = (far + gap).max(start + margin);
    let before_to = (near - gap).min(end - margin);
    let (after, before) = (end - margin - after_from, before_to - start - margin);
    let length = wanted.get_coord(axis);
    let forward = length <= after || after >= before;
    let fitted = length.min(if forward { after } else { before }).max(0.0);
    let main = if forward { after_from } else { before_to - fitted };
    let cross = axis.cross();
    let ((low, high), (from, to)) = (span(trigger, cross), span(area, cross));
    let width = wanted.get_coord(cross);
    let lead = if centered { (low + high - width) / 2.0 } else { low };
    (axis.pack_point(main, lead.min(to - width - margin).max(from + margin)).round(), fitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect::new(0.0, 0.0, 400.0, 300.0);
    const TIP: Size = Size::new(120.0, 24.0);

    #[test]
    fn below_and_centered_when_there_is_room() {
        let (at, _) = beside(Rect::new(140.0, 40.0, 172.0, 72.0), TIP, AREA, Axis::Vertical, true);
        assert_eq!(at, Point::new(96.0, 76.0));
    }

    #[test]
    fn flips_above_at_the_bottom_and_stays_inside_at_the_corners() {
        let (at, _) = beside(Rect::new(380.0, 270.0, 400.0, 290.0), TIP, AREA, Axis::Vertical, true);
        assert_eq!(at, Point::new(272.0, 242.0), "над кнопкой, прижата к правому краю");
        let (at, _) = beside(Rect::new(0.0, 0.0, 20.0, 20.0), TIP, AREA, Axis::Vertical, true);
        assert_eq!(at, Point::new(8.0, 24.0), "под кнопкой, прижата к левому краю");
    }

    #[test]
    fn beside_goes_right_and_flips_left_at_the_right_edge() {
        let (at, _) = beside(Rect::new(0.0, 100.0, 40.0, 140.0), TIP, AREA, Axis::Horizontal, true);
        assert_eq!(at, Point::new(44.0, 108.0), "справа от рельса, по центру кнопки");
        let (at, _) = beside(Rect::new(360.0, 280.0, 400.0, 300.0), TIP, AREA, Axis::Horizontal, true);
        assert_eq!(at, Point::new(236.0, 268.0), "слева, не ниже нижнего поля");
    }

    #[test]
    fn a_box_wider_than_the_area_starts_at_the_margin() {
        let (at, _) = beside(Rect::new(100.0, 10.0, 120.0, 30.0), Size::new(500.0, 24.0), AREA, Axis::Vertical, true);
        assert_eq!(at.x, 8.0);
    }
}
