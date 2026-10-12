use super::Premul;

pub(super) enum Shape {
    Linear { origin: (f64, f64), dir: (f64, f64), length: f64 },
    Radial { center: (f64, f64), rx: f64, ry: f64 },
    Conic { center: (f64, f64), from: f64 },
}

pub(super) struct Prepared {
    pub tile: (f64, f64),
    pub offset: (f64, f64),
    pub shape: Shape,
    pub stops: Vec<(f64, Premul)>,
    pub period: Option<(f64, f64)>,
}

impl Prepared {
    pub(super) fn color(&self, x: f64, y: f64) -> Premul {
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
