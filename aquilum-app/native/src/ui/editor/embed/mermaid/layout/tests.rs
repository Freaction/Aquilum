use masonry::kurbo::{Point, Rect};
use mermaid_rs_renderer::NodeShape;

use super::ports::{Tip, plan};
use crate::ui::editor::embed::mermaid::shape::{Node, Side};

fn node(x0: f64, y0: f64, x1: f64, y1: f64, shape: NodeShape) -> Node {
    Node { rect: Rect::new(x0, y0, x1, y1), shape }
}

fn tips<'a>(from: (&'a str, Node), to: (&'a str, Node), sides: (Side, Side)) -> Option<[Tip<'a>; 2]> {
    let (a, b) = (from.1.rect.center(), to.1.rect.center());
    Some([Tip { id: from.0, node: from.1, side: sides.0, toward: b, marked: false }, Tip { id: to.0, node: to.1, side: sides.1, toward: a, marked: false }])
}

#[test]
fn arrows_meet_at_the_middle_of_a_side_and_only_twins_part() {
    let (left, right, target) = (node(0.0, 0.0, 80.0, 40.0, NodeShape::Rectangle), node(220.0, 0.0, 300.0, 40.0, NodeShape::Rectangle), node(100.0, 120.0, 200.0, 160.0, NodeShape::Rectangle));
    let down = (Side::Bottom, Side::Top);
    let ports = plan(&[tips(("l", left), ("t", target), down), tips(("r", right), ("t", target), down)]);
    for port in &ports {
        assert_eq!(port.unwrap()[1].point, Point::new(150.0, 120.0), "стрелки от разных узлов входят в центр стороны");
    }
    let aim = |x: f64| [Tip { id: "l", node: left, side: Side::Bottom, toward: Point::new(x, 80.0), marked: false }, Tip { id: "t", node: target, side: Side::Top, toward: Point::new(x, 80.0), marked: false }];
    let twins = plan(&[Some(aim(20.0)), Some(aim(150.0))]);
    let (a, b) = (twins[0].unwrap(), twins[1].unwrap());
    assert!(a[0].point.x < b[0].point.x && a[1].point.x < b[1].point.x, "две стрелки между одной парой узлов не сливаются и не пересекаются");
    assert!(a[1].point.y == 120.0 && b[1].point.y == 120.0, "точки на верхней стороне");
}

#[test]
fn a_diamond_sends_every_arrow_from_its_vertex() {
    let diamond = node(100.0, 0.0, 200.0, 100.0, NodeShape::Diamond);
    let down = (Side::Bottom, Side::Top);
    let ports = plan(&[tips(("d", diamond), ("a", node(0.0, 200.0, 80.0, 240.0, NodeShape::Rectangle)), down), tips(("d", diamond), ("b", node(220.0, 200.0, 300.0, 240.0, NodeShape::Rectangle)), down)]);
    for port in ports {
        assert_eq!(port.unwrap()[0].point, Point::new(150.0, 100.0));
    }
}

#[test]
fn a_circle_takes_arrows_on_its_edge() {
    let (a, b) = (node(0.0, 0.0, 80.0, 40.0, NodeShape::Rectangle), node(140.0, 10.0, 220.0, 50.0, NodeShape::Circle));
    let ports = plan(&[tips(("a", a), ("b", b), (Side::Right, Side::Left))]);
    assert_eq!(ports[0].unwrap()[1].point, Point::new(160.0, 30.0), "в круг — по его краю, а не по углу рамки");
}
