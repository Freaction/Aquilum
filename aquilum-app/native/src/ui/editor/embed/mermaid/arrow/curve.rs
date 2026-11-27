use masonry::kurbo::{BezPath, CubicBez, ParamCurve, Point, Vec2};

use super::super::canvas::{Canvas, Pen};
use super::head::End;
use super::super::shape::Port;
use crate::ui::tokens::size;

const CORNER: f64 = size::RADIUS_XL;
const BEND: f64 = 16.0;
const SAMPLES: usize = 32;
const STEPS: usize = 40;

pub(in super::super) struct Arrow {
    path: BezPath,
    pub(in super::super) tips: [(Point, Vec2); 2],
    pub(in super::super) spine: Vec<Point>,
}

pub(in super::super) fn bow(a: Point, na: Vec2, b: Point, nb: Vec2) -> CubicBez {
    let reach = |p: Point, q: Point, n: Vec2| p + n * ((q - p).dot(n).abs() / 2.0).max(BEND);
    CubicBez::new(a, reach(a, b, na), reach(b, a, nb), b)
}

pub(in super::super) fn find(f: impl Fn(f64) -> f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..STEPS {
        let mid = (lo + hi) / 2.0;
        if f(mid) < 0.0 { lo = mid } else { hi = mid }
    }
    (lo + hi) / 2.0
}

impl Arrow {
    pub(super) fn curve([a, b]: [Port; 2], insets: [f64; 2]) -> Arrow {
        let normals = [a.side.normal(), b.side.normal()];
        Arrow::cubic(bow(a.point, normals[0], b.point, normals[1]), normals, insets)
    }

    pub(super) fn cubic(cubic: CubicBez, normals: [Vec2; 2], insets: [f64; 2]) -> Arrow {
        let t0 = if insets[0] > 0.0 { find(|t| cubic.eval(t).distance(cubic.p0) - insets[0]) } else { 0.0 };
        let t1 = if insets[1] > 0.0 { find(|t| insets[1] - cubic.eval(t).distance(cubic.p3)) } else { 1.0 };
        let part = cubic.subsegment(t0..t1);
        let mut path = BezPath::new();
        path.move_to(part.p0);
        path.curve_to(part.p1, part.p2, part.p3);
        let spine = (0..=SAMPLES).map(|k| part.eval(k as f64 / SAMPLES as f64)).collect();
        Arrow { path, tips: [(cubic.p0, -normals[0]), (cubic.p3, -normals[1])], spine }
    }

    pub(super) fn along(mut points: Vec<Point>, insets: [f64; 2]) -> Option<Arrow> {
        points.dedup_by(|a, b| a.distance(*b) < 0.5);
        let n = points.len();
        if n < 2 {
            return None;
        }
        let tips = [(points[0], (points[0] - points[1]).normalize()), (points[n - 1], (points[n - 1] - points[n - 2]).normalize())];
        for (k, (tip, dir)) in tips.iter().enumerate() {
            let near = if k == 0 { points[1] } else { points[n - 2] };
            let i = if k == 0 { 0 } else { n - 1 };
            points[i] = *tip - *dir * insets[k].min(tip.distance(near) - 1.0).max(0.0);
        }
        let mut path = BezPath::new();
        path.move_to(points[0]);
        for w in points.windows(3) {
            let r = CORNER.min(w[0].distance(w[1]) / 2.0).min(w[1].distance(w[2]) / 2.0);
            path.line_to(w[1] + (w[0] - w[1]).normalize() * r);
            path.quad_to(w[1], w[1] + (w[2] - w[1]).normalize() * r);
        }
        path.line_to(points[n - 1]);
        Some(Arrow { path, tips, spine: points })
    }

    pub(super) fn at(&self, t: f64) -> Point {
        let total: f64 = self.spine.windows(2).map(|w| w[0].distance(w[1])).sum();
        let mut left = total * t;
        for w in self.spine.windows(2) {
            let length = w[0].distance(w[1]);
            if left <= length {
                return w[0].lerp(w[1], left / length.max(1e-6));
            }
            left -= length;
        }
        self.spine[self.spine.len() - 1]
    }

    pub(super) fn draw(&self, pen: Pen, ends: [End; 2], canvas: &mut Canvas) {
        canvas.shape(self.path.clone(), None, Some(pen));
        for (end, (tip, dir)) in ends.into_iter().zip(self.tips) {
            end.draw(tip, dir, pen, canvas);
        }
    }
}
