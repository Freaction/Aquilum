use masonry::kurbo::{CubicBez, Point, Rect, Vec2};
use mermaid_rs_renderer::Edge;

use super::super::canvas::{Text, chip_rect, single};
use super::super::scene::Wire;
use super::super::shape::Side;
use super::Arrow;
use crate::ui::tokens::size::GAP_MD;

const REACH: f64 = 40.0;
const LEGS: f64 = 0.2;

fn chip(edge: &Edge, normal: Vec2) -> (f64, Option<Text>) {
    let label = edge.label.as_deref().filter(|t| !t.trim().is_empty()).map(single);
    let half = label.as_ref().map_or(0.0, |t| {
        let r = chip_rect(Point::ZERO, t);
        (r.width() * normal.x.abs() + r.height() * normal.y.abs()) / 2.0
    });
    (half, label)
}

pub(in super::super) fn room(edge: &Edge, across: bool) -> f64 {
    let (half, _) = chip(edge, if across { Vec2::new(0.0, 1.0) } else { Vec2::new(1.0, 0.0) });
    REACH * 0.75 + GAP_MD + half * 2.0
}

pub(in super::super) fn looped(edge: &Edge, r: Rect, side: Side) -> Wire {
    let n = side.normal();
    let along = Vec2::new(n.y.abs(), n.x.abs());
    let length = r.width() * along.x + r.height() * along.y;
    let mid = r.center() + Vec2::new(n.x * r.width(), n.y * r.height()) / 2.0;
    let (half, label) = chip(edge, n);
    let anchor = label.as_ref().map(|_| mid + n * (REACH * 0.75 + GAP_MD + half));
    Wire { edge: edge.clone(), points: vec![mid - along * length * LEGS, mid + along * length * LEGS], label, anchor, sides: None }
}

pub(super) fn arrow(points: &[Point], center: Point, insets: [f64; 2]) -> Option<Arrow> {
    let &[a, b] = points else { return None };
    let d = (b - a).normalize();
    let n = Vec2::new(-d.y, d.x);
    let n = if n.dot(a - center) < 0.0 { -n } else { n };
    Some(Arrow::cubic(CubicBez::new(a, a + n * REACH, b + n * REACH, b), [n, n], insets))
}
