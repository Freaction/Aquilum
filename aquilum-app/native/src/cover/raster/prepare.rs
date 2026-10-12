use crate::cover::css::{Extent, Gradient, Kind, Layer, RadialSize, Val};
use super::shape::{Prepared, Shape};
use super::{Premul, premul};

fn side_extents(center: (f64, f64), w: f64, h: f64) -> ((f64, f64), (f64, f64)) {
    let dx = (center.0.abs(), (w - center.0).abs());
    let dy = (center.1.abs(), (h - center.1).abs());
    ((dx.0.min(dx.1), dy.0.min(dy.1)), (dx.0.max(dx.1), dy.0.max(dy.1)))
}

fn radii(circle: bool, size: &RadialSize, center: (f64, f64), w: f64, h: f64) -> (f64, f64) {
    let (closest, farthest) = side_extents(center, w, h);
    match size {
        RadialSize::Explicit(rx, ry) => (rx.length(w), if circle { rx.length(w) } else { ry.length(h) }),
        RadialSize::Extent(extent) => {
            let side = match extent {
                Extent::ClosestSide | Extent::ClosestCorner => closest,
                Extent::FarthestSide | Extent::FarthestCorner => farthest,
            };
            let corner = matches!(extent, Extent::ClosestCorner | Extent::FarthestCorner);
            if circle {
                let r = if corner { side.0.hypot(side.1) } else if matches!(extent, Extent::ClosestSide) { side.0.min(side.1) } else { side.0.max(side.1) };
                (r, r)
            } else if corner {
                if side.1 == 0.0 || side.0 == 0.0 {
                    let r = side.0.hypot(side.1);
                    (r, r)
                } else {
                    let ratio = side.0 / side.1;
                    let ry = ((side.0 / ratio).powi(2) + side.1.powi(2)).sqrt();
                    (ratio * ry, ry)
                }
            } else {
                (side.0, side.1)
            }
        }
    }
}

fn resolve_stops(gradient: &Gradient, reference: f64, angular: bool) -> Vec<(f64, Premul)> {
    let mut positions: Vec<Option<f64>> = gradient
        .stops
        .iter()
        .map(|s| s.pos.map(|p| if angular { p.angle() / 360.0 } else { p.length(reference) / reference }))
        .collect();
    let n = positions.len();
    if positions[0].is_none() {
        positions[0] = Some(0.0);
    }
    if positions[n - 1].is_none() {
        positions[n - 1] = Some(1.0);
    }
    let mut max = f64::MIN;
    for p in positions.iter_mut().flatten() {
        if *p < max {
            *p = max;
        }
        max = max.max(*p);
    }
    let mut i = 0;
    while i < n {
        if positions[i].is_none() {
            let start = i - 1;
            let mut end = i;
            while positions[end].is_none() {
                end += 1;
            }
            let (a, b) = (positions[start].unwrap_or(0.0), positions[end].unwrap_or(1.0));
            for (k, p) in positions.iter_mut().enumerate().take(end).skip(i) {
                *p = Some(a + (b - a) * (k - start) as f64 / (end - start) as f64);
            }
            i = end;
        }
        i += 1;
    }
    positions.iter().zip(&gradient.stops).map(|(p, s)| (p.unwrap_or(0.0), premul(s.color))).collect()
}

pub(super) fn prepare(layer: &Layer, area: (f64, f64)) -> Option<Prepared> {
    let full = Val { pct: 100.0, ..Val::default() };
    let (sw, sh) = layer.size.unwrap_or((full, full));
    let tile = (sw.length(area.0), sh.length(area.1));
    if tile.0 <= 0.0 || tile.1 <= 0.0 {
        return None;
    }
    let offset = (layer.position.0.length(area.0 - tile.0), layer.position.1.length(area.1 - tile.1));
    let (w, h) = tile;
    let gradient: &Gradient = &layer.gradient;
    let (shape, reference, angular) = match &gradient.kind {
        Kind::Linear { angle, corner } => {
            let radians = match (angle, corner) {
                (Some(a), _) => a.to_radians(),
                (None, Some((x, y))) => {
                    let a = h.atan2(w);
                    match (*x > 0.0, *y < 0.0) {
                        (true, true) => a,
                        (true, false) => std::f64::consts::PI - a,
                        (false, false) => std::f64::consts::PI + a,
                        (false, true) => 2.0 * std::f64::consts::PI - a,
                    }
                }
                (None, None) => std::f64::consts::PI,
            };
            let dir = (radians.sin(), -radians.cos());
            let length = (w * dir.0).abs() + (h * dir.1).abs();
            let origin = (w / 2.0 - dir.0 * length / 2.0, h / 2.0 - dir.1 * length / 2.0);
            (Shape::Linear { origin, dir, length }, length, false)
        }
        Kind::Radial { circle, size, at } => {
            let center = (at.0.length(w), at.1.length(h));
            let (rx, ry) = radii(*circle, size, center, w, h);
            (Shape::Radial { center, rx: rx.max(1e-6), ry: ry.max(1e-6) }, rx.max(1e-6), false)
        }
        Kind::Conic { from, at } => {
            let center = (at.0.length(w), at.1.length(h));
            (Shape::Conic { center, from: *from }, 360.0, true)
        }
    };
    let stops = resolve_stops(gradient, reference, angular);
    let period = if gradient.repeating {
        let first = stops.first()?.0;
        let span = stops.last()?.0 - first;
        (span > 1e-9).then_some((first, span))
    } else {
        None
    };
    if gradient.repeating && period.is_none() {
        return None;
    }
    Some(Prepared { tile, offset, shape, stops, period })
}
