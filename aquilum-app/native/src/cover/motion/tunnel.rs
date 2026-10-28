use super::{Target, pack};

const TILE: f64 = 44.0;
const LINE: f64 = 2.0;
const PERSPECTIVE: f64 = 84.0;
const SECONDS: f64 = 2.0;
const ANGLE: f64 = 85.0;
const STREAK: [u8; 3] = [0xa2, 0xce, 0xf4];

struct Plane {
    top: f64,
    width: f64,
    height: f64,
    sin: f64,
    cos: f64,
    offset: (f64, f64),
}

impl Plane {
    fn limits(&self) -> (f64, f64) {
        let near = (PERSPECTIVE - 1.0) / self.sin;
        let half = self.height / 2.0;
        let (lo, hi) = if near > 0.0 { (-half, near.min(half)) } else { (near.max(-half), half) };
        (lo + half, hi + half)
    }

    fn depth(&self, y: f64, h: f64) -> Option<f64> {
        let screen = y - h / 2.0;
        let shift = self.top + self.height / 2.0 - h / 2.0;
        let denominator = PERSPECTIVE * self.cos + screen * self.sin;
        (denominator > 1e-9).then(|| PERSPECTIVE * (screen - shift) / denominator + self.height / 2.0)
    }

    fn scale_at(&self, depth: f64) -> f64 {
        PERSPECTIVE / (PERSPECTIVE - (depth - self.height / 2.0) * self.sin)
    }
}

fn lines_until(p: f64, offset: f64) -> f64 {
    let local = p - offset;
    (local / TILE).floor() * LINE + local.rem_euclid(TILE).min(LINE)
}

pub(super) fn draw(t: &mut Target<'_>, seconds: f64) {
    let (w, h) = t.size;
    let f = (seconds % SECONDS) / SECONDS;
    let (sin, cos) = ANGLE.to_radians().sin_cos();
    let planes = [
        Plane { top: 0.0, width: w * 2.0, height: h * 1.3, sin, cos, offset: (TILE * f, -TILE + TILE * f) },
        Plane { top: -h * 0.3, width: w * 2.0, height: h * 1.3, sin: -sin, cos, offset: (TILE * f, -TILE * f) },
    ];
    let mut lines: Vec<(usize, f32)> = Vec::new();
    let mut out = vec![0u32; t.width];
    for j in 0..t.height {
        lines.clear();
        let (mut base, mut span) = (0f32, (usize::MAX, 0usize));
        let y0 = (t.origin.1 + j as f64) / t.scale;
        let y1 = y0 + 1.0 / t.scale;
        for plane in &planes {
            let (Some(a), Some(b)) = (plane.depth(y0, h), plane.depth(y1, h)) else { continue };
            let (p0, p1) = (a.min(b), a.max(b));
            let (lo, hi) = plane.limits();
            let (c0, c1) = (p0.max(lo), p1.min(hi));
            if c1 <= c0 {
                continue;
            }
            let extent = (p1 - p0).max(1e-9);
            let weight = (c1 - c0) / extent;
            let horizontal = ((lines_until(c1, plane.offset.1) - lines_until(c0, plane.offset.1)) / extent) as f32;
            let s = plane.scale_at((c0 + c1) / 2.0);
            let to_buffer = |x: f64| (w / 2.0 + (x - w) * s) * t.scale - t.origin.0;
            if horizontal > 0.0 {
                let (left, right) = (to_buffer(0.0).max(0.0) as usize, (to_buffer(plane.width).max(0.0) as usize).min(t.width));
                base = 1.0 - (1.0 - base) * (1.0 - horizontal);
                span = (span.0.min(left), span.1.max(right));
            }
            let mut x = plane.offset.0.rem_euclid(TILE) - TILE;
            while x < plane.width {
                let (a, b) = (to_buffer(x.max(0.0)), to_buffer((x + LINE).min(plane.width)));
                x += TILE;
                if b <= 0.0 || a >= t.width as f64 || b <= a {
                    continue;
                }
                for i in (a.max(0.0).floor() as usize)..(b.ceil() as usize).min(t.width) {
                    lines.push((i, ((b.min(i as f64 + 1.0) - a.max(i as f64)).clamp(0.0, 1.0) * weight) as f32));
                }
            }
        }
        let filled = (base > 0.002 && span.1 > span.0).then_some(span.0..span.1);
        if let Some(range) = filled.clone() {
            out[range].fill(pack(STREAK, f64::from(base)));
        }
        for &(i, cover) in &lines {
            let under = if (span.0..span.1).contains(&i) { base } else { 0.0 };
            let current = f32::from((out[i] >> 24) as u8) / 255.0;
            let total = 1.0 - (1.0 - current.max(under)) * (1.0 - cover);
            if total > 0.002 {
                out[i] = pack(STREAK, f64::from(total));
            }
        }
        let mut row = t.row(j);
        if let Some(range) = filled {
            row.put_slice(range.start, &out[range.clone()]);
            out[range].fill(0);
        }
        for &(i, _) in &lines {
            let pixel = std::mem::take(&mut out[i]);
            if pixel != 0 {
                row.put(i, pixel);
            }
        }
    }
}
