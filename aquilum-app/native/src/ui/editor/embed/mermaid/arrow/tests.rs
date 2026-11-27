use masonry::kurbo::Point;

use super::Arrow;
use crate::ui::editor::embed::mermaid::shape::{Port, Side};

fn down(a: Point, b: Point) -> [Port; 2] {
    [Port { point: a, side: Side::Bottom }, Port { point: b, side: Side::Top }]
}

#[test]
fn an_arrow_is_one_curve_square_to_both_sides() {
    let arrow = Arrow::curve(down(Point::new(0.0, 0.0), Point::new(120.0, 150.0)), [0.0, 8.0]);
    assert_eq!(arrow.tips[0].1, -Side::Bottom.normal(), "выход перпендикулярно стороне");
    assert_eq!(arrow.tips[1].1, -Side::Top.normal(), "наконечник смотрит в узел перпендикулярно стороне");
    assert_eq!(arrow.tips[1].0, Point::new(120.0, 150.0), "острие — на самой стороне");
    let end = arrow.spine[arrow.spine.len() - 1];
    assert!((end.distance(arrow.tips[1].0) - 8.0).abs() < 0.01, "линия укорочена под наконечник");
    assert!((end.x - 120.0).abs() < 0.5, "у наконечника линия уже идёт прямо");
    assert!(arrow.spine.windows(2).all(|w| w[1].x >= w[0].x && w[1].y > w[0].y), "одна плавная кривая без петель и обходов");
}

#[test]
fn the_bend_grows_with_the_distance() {
    let reach = |dy: f64| {
        let arrow = Arrow::curve(down(Point::new(0.0, 0.0), Point::new(100.0, dy)), [0.0, 0.0]);
        arrow.spine.iter().position(|p| p.x > 10.0).map(|i| arrow.spine[i].y).unwrap_or_default()
    };
    assert!(reach(300.0) > reach(100.0) * 2.0, "чем дальше узлы, тем длиннее прямой выход из стороны");
}
