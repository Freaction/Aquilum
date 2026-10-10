use super::css::{Background, Extent, Gradient, Kind, Layer, RadialSize, Rgba, Val};

type Premul = [f32; 4];

fn premul(c: Rgba) -> Premul {
    let a = c[3] as f32;
    [c[0] as f32 * a, c[1] as f32 * a, c[2] as f32 * a, a]
}

enum Shape {
    Linear { origin: (f64, f64), dir: (f64, f64), length: f64 },
    Radial { center: (f64, f64), rx: f64, ry: f64 },
    Conic { center: (f64, f64), from: f64 },
}

struct Prepared {
    tile: (f64, f64),
    offset: (f64, f64),
    shape: Shape,
    stops: Vec<(f64, Premul)>,
    period: Option<(f64, f64)>,
}

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

fn prepare(layer: &Layer, area: (f64, f64)) -> Option<Prepared> {
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
    let stops: Vec<(f64, Premul)> = positions.iter().zip(&gradient.stops).map(|(p, s)| (p.unwrap_or(0.0), premul(s.color))).collect();
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

impl Prepared {
    fn color(&self, x: f64, y: f64) -> Premul {
        let u = (x - self.offset.0).rem_euclid(self.tile.0);
        let v = (y - self.offset.1).rem_euclid(self.tile.1);
        let mut t = match &self.shape {
            Shape::Linear { origin, dir, length } => ((u - origin.0) * dir.0 + (v - origin.1) * dir.1) / length,
            Shape::Radial { center, rx, ry } => ((u - center.0) / rx).hypot((v - center.1) / ry),
            Shape::Conic { center, from } => {
                let angle = (u - center.0).atan2(-(v - center.1)).to_degrees();
                (angle - from).rem_euclid(360.0) / 360.0
            }
        };
        if let Some((first, span)) = self.period {
            t = first + (t - first).rem_euclid(span);
        }
        sample(&self.stops, t)
    }
}

fn sample(stops: &[(f64, Premul)], t: f64) -> Premul {
    let first = stops[0];
    if t <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t < b.0 {
            if b.0 - a.0 <= 1e-12 {
                return b.1;
            }
            let k = ((t - a.0) / (b.0 - a.0)) as f32;
            return std::array::from_fn(|i| a.1[i] + (b.1[i] - a.1[i]) * k);
        }
    }
    stops[stops.len() - 1].1
}

fn over(dst: &mut Premul, src: Premul) {
    let k = 1.0 - src[3];
    for i in 0..4 {
        dst[i] = src[i] + dst[i] * k;
    }
}

pub fn render(backgrounds: &[&Background], width: u32, height: u32, scale: f64) -> Vec<u8> {
    let area = (f64::from(width) / scale, f64::from(height) / scale);
    let prepared: Vec<(Premul, Vec<Prepared>)> = backgrounds
        .iter()
        .map(|b| (premul(b.color), b.layers.iter().rev().filter_map(|l| prepare(l, area)).collect()))
        .collect();
    let mut out = vec![0u8; (width * height * 4) as usize];
    let row = (width * 4) as usize;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let rows_per = (height as usize).div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        for (chunk, rows) in out.chunks_mut(row * rows_per).enumerate() {
            let prepared = &prepared;
            scope.spawn(move || {
                let first = (chunk * rows_per) as u32;
                for (i, line) in rows.chunks_mut(row).enumerate() {
                    render_row(prepared, line, first + i as u32, width, scale);
                }
            });
        }
    });
    out
}

fn render_row(prepared: &[(Premul, Vec<Prepared>)], out: &mut [u8], py: u32, width: u32, scale: f64) {
    const SAMPLES: [(f64, f64); 4] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];
    {
        for px in 0..width {
            let mut sum = [0f32; 4];
            for (sx, sy) in SAMPLES {
                let x = (f64::from(px) + sx) / scale;
                let y = (f64::from(py) + sy) / scale;
                let mut pixel = [0f32; 4];
                for (color, layers) in prepared {
                    over(&mut pixel, *color);
                    for layer in layers {
                        over(&mut pixel, layer.color(x, y));
                    }
                }
                for i in 0..4 {
                    sum[i] += pixel[i] / SAMPLES.len() as f32;
                }
            }
            let at = (px * 4) as usize;
            for i in 0..4 {
                out[at + i] = (sum[i].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
}
