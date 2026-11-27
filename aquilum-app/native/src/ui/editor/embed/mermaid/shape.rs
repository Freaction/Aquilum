use std::f64::consts::PI;

use masonry::kurbo::{Arc, BezPath, Circle, Line, PathEl, Point, Rect, RoundedRect, Shape, Vec2};
use mermaid_rs_renderer::NodeShape;

use crate::ui::tokens::size;

const TOLERANCE: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub fn along(v: Vec2, vertical: bool) -> Side {
        match (vertical, v.y >= 0.0, v.x >= 0.0) {
            (true, true, _) => Side::Bottom,
            (true, false, _) => Side::Top,
            (false, _, true) => Side::Right,
            (false, _, false) => Side::Left,
        }
    }

    pub fn normal(self) -> Vec2 {
        match self {
            Side::Top => Vec2::new(0.0, -1.0),
            Side::Bottom => Vec2::new(0.0, 1.0),
            Side::Left => Vec2::new(-1.0, 0.0),
            Side::Right => Vec2::new(1.0, 0.0),
        }
    }
}

pub(super) fn polygon(points: &[Point], closed: bool) -> BezPath {
    let mut path = BezPath::new();
    for (i, p) in points.iter().enumerate() {
        if i == 0 { path.move_to(*p) } else { path.line_to(*p) }
    }
    if closed {
        path.close_path();
    }
    path
}

#[derive(Clone, Copy)]
pub(super) struct Port {
    pub point: Point,
    pub side: Side,
}

#[derive(Clone, Copy)]
pub(super) struct Node {
    pub rect: Rect,
    pub shape: NodeShape,
}

impl Node {
    fn skew(self) -> f64 {
        (self.rect.height() * 0.35).min(self.rect.width() * 0.25)
    }

    fn lip(self) -> f64 {
        (self.rect.height() * 0.12).clamp(5.0, 12.0)
    }

    fn half(self, cy: f64, start: f64, sweep: f64) -> impl Iterator<Item = PathEl> {
        Arc::new((self.rect.center().x, cy), (self.rect.width() / 2.0, self.lip()), start, sweep, 0.0).append_iter(TOLERANCE)
    }

    pub fn outline(self) -> Option<BezPath> {
        let Rect { x0, y0, x1, y1 } = self.rect;
        let (c, h, s) = (self.rect.center(), self.rect.height(), self.skew());
        let p = Point::new;
        let rounded = |radius: f64| RoundedRect::from_rect(self.rect, radius).to_path(TOLERANCE);
        Some(match self.shape {
            NodeShape::Text => return None,
            NodeShape::Rectangle | NodeShape::ActorBox | NodeShape::Subroutine => rounded(size::RADIUS_LG),
            NodeShape::RoundRect | NodeShape::MindmapDefault => rounded(size::RADIUS_2XL.min(h / 2.0)),
            NodeShape::Stadium => rounded(h / 2.0),
            NodeShape::ForkJoin => rounded(size::RADIUS_SM),
            NodeShape::Circle | NodeShape::DoubleCircle => Circle::new(c, self.rect.width().min(h) / 2.0).to_path(TOLERANCE),
            NodeShape::Diamond => polygon(&[p(c.x, y0), p(x1, c.y), p(c.x, y1), p(x0, c.y)], true),
            NodeShape::Hexagon => {
                let i = (h / 2.0).min(self.rect.width() * 0.2);
                polygon(&[p(x0 + i, y0), p(x1 - i, y0), p(x1, c.y), p(x1 - i, y1), p(x0 + i, y1), p(x0, c.y)], true)
            }
            NodeShape::Parallelogram => polygon(&[p(x0 + s, y0), p(x1, y0), p(x1 - s, y1), p(x0, y1)], true),
            NodeShape::ParallelogramAlt => polygon(&[p(x0, y0), p(x1 - s, y0), p(x1, y1), p(x0 + s, y1)], true),
            NodeShape::Trapezoid => polygon(&[p(x0 + s, y0), p(x1 - s, y0), p(x1, y1), p(x0, y1)], true),
            NodeShape::TrapezoidAlt => polygon(&[p(x0, y0), p(x1, y0), p(x1 - s, y1), p(x0 + s, y1)], true),
            NodeShape::Asymmetric => polygon(&[p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1), p(x0 + s, c.y)], true),
            NodeShape::Cylinder => {
                let mut path = BezPath::new();
                path.move_to(p(x0, y0 + self.lip()));
                path.extend(self.half(y0 + self.lip(), PI, PI));
                path.line_to(p(x1, y1 - self.lip()));
                path.extend(self.half(y1 - self.lip(), 0.0, PI));
                path.close_path();
                path
            }
        })
    }

    pub fn details(self) -> Option<BezPath> {
        let (r, inset) = (self.rect, size::GAP_LG);
        match self.shape {
            NodeShape::Subroutine => {
                let mut path = Line::new((r.x0 + inset, r.y0), (r.x0 + inset, r.y1)).to_path(TOLERANCE);
                path.extend(Line::new((r.x1 - inset, r.y0), (r.x1 - inset, r.y1)).path_elements(TOLERANCE));
                Some(path)
            }
            NodeShape::DoubleCircle => Some(Circle::new(r.center(), r.width().min(r.height()) / 2.0 - size::GAP_SM).to_path(TOLERANCE)),
            NodeShape::Cylinder => {
                let mut path = BezPath::new();
                path.move_to(Point::new(r.x0, r.y0 + self.lip()));
                path.extend(self.half(r.y0 + self.lip(), PI, -PI));
                Some(path)
            }
            _ => None,
        }
    }

    pub fn spreads(self, side: Side) -> bool {
        let flat = matches!(side, Side::Top | Side::Bottom);
        match self.shape {
            NodeShape::Rectangle | NodeShape::RoundRect | NodeShape::Subroutine | NodeShape::ActorBox | NodeShape::MindmapDefault | NodeShape::Text => true,
            NodeShape::Stadium | NodeShape::Cylinder | NodeShape::Parallelogram | NodeShape::ParallelogramAlt | NodeShape::Trapezoid | NodeShape::TrapezoidAlt => flat,
            _ => false,
        }
    }

    pub fn port(self, side: Side, t: f64) -> Point {
        let r = self.rect;
        let c = r.center();
        let t = if self.spreads(side) { t } else { 0.5 };
        let (x, y) = (r.x0 + r.width() * t, r.y0 + r.height() * t);
        let slant = match self.shape {
            NodeShape::Parallelogram | NodeShape::ParallelogramAlt | NodeShape::Trapezoid | NodeShape::TrapezoidAlt => self.skew() / 2.0,
            NodeShape::Asymmetric if side == Side::Left => self.skew(),
            _ => 0.0,
        };
        let dip = if self.shape == NodeShape::Cylinder { self.lip() * (1.0 - (1.0 - (2.0 * t - 1.0).powi(2)).sqrt()) } else { 0.0 };
        match (self.shape, side) {
            (NodeShape::Circle | NodeShape::DoubleCircle, _) => c + side.normal() * (r.width().min(r.height()) / 2.0),
            (_, Side::Top) => Point::new(x, r.y0 + dip),
            (_, Side::Bottom) => Point::new(x, r.y1 - dip),
            (_, Side::Left) => Point::new(r.x0 + slant, y),
            (_, Side::Right) => Point::new(r.x1 - slant, y),
        }
    }
}
