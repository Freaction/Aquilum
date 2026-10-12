use super::{Band, Disc, Edge, Stamp, blend};

impl Band<'_> {
    pub fn line(&mut self, e: &Edge) {
        let Some(([mut x0, mut y0], [mut x1, mut y1])) = clip(e.from, e.to, -2.0, self.rows.start as f32 - 2.0, self.width as f32 + 2.0, self.rows.end as f32 + 2.0) else { return };
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }
        let dx = x1 - x0;
        let gradient = if dx < 1e-6 { 0.0 } else { (y1 - y0) / dx };
        let (top, bottom) = (self.rows.start as f32, self.rows.end as f32);
        let (mut start, mut end) = (x0.floor() as i64, x1.ceil() as i64);
        if steep {
            start = start.max(top as i64 - 1);
            end = end.min(bottom as i64 + 1);
        } else {
            start = start.max(-1);
            end = end.min(self.width as i64 + 1);
        }
        if start > end {
            return;
        }
        for i in start..=end {
            let x = i as f32 + 0.5;
            if x < x0 - 0.5 || x > x1 + 0.5 {
                continue;
            }
            let reach = if x < x0 { 1.0 - (x0 - x) } else if x > x1 { 1.0 - (x - x1) } else { 1.0 };
            let y = y0 + gradient * (x.clamp(x0, x1) - x0) - 0.5;
            let base = y.floor();
            let fraction = y - base;
            let a0 = e.alpha * reach * (1.0 - fraction);
            let a1 = e.alpha * reach * fraction;
            if steep {
                self.plot(base as i32, i as i32, e.color, a0);
                self.plot(base as i32 + 1, i as i32, e.color, a1);
            } else {
                self.plot(i as i32, base as i32, e.color, a0);
                self.plot(i as i32, base as i32 + 1, e.color, a1);
            }
        }
    }

    pub fn disc(&mut self, d: &Disc) {
        let [cx, cy] = d.center;
        if d.radius < 1.0 {
            let area = (std::f32::consts::PI * d.radius * d.radius).min(1.0) * d.alpha;
            let (x, y) = (cx - 0.5, cy - 0.5);
            let (bx, by) = (x.floor(), y.floor());
            let (fx, fy) = (x - bx, y - by);
            let (bx, by) = (bx as i32, by as i32);
            self.plot(bx, by, d.body, area * (1.0 - fx) * (1.0 - fy));
            self.plot(bx + 1, by, d.body, area * fx * (1.0 - fy));
            self.plot(bx, by + 1, d.body, area * (1.0 - fx) * fy);
            self.plot(bx + 1, by + 1, d.body, area * fx * fy);
            return;
        }
        let r = d.radius;
        let first = ((cy - r - 1.0).floor() as i64).max(self.rows.start as i64);
        let last = ((cy + r + 1.0).ceil() as i64).min(self.rows.end as i64 - 1);
        let outer = r + 0.5;
        for y in first..=last {
            let dy = y as f32 + 0.5 - cy;
            if dy.abs() > outer {
                continue;
            }
            let span = (outer * outer - dy * dy).max(0.0).sqrt();
            let x_from = ((cx - span).floor() as i64).max(0);
            let x_to = ((cx + span).ceil() as i64).min(self.width as i64 - 1);
            let row = (y as usize - self.rows.start) * self.width;
            for x in x_from..=x_to {
                let dx = x as f32 + 0.5 - cx;
                let distance = (dx * dx + dy * dy).sqrt();
                let coverage = (outer - distance).clamp(0.0, 1.0);
                if coverage <= 0.0 {
                    continue;
                }
                let at = (row + x as usize) * 4;
                let px = &mut self.pixels[at..at + 4];
                if d.inner < r {
                    let body = (d.inner + 0.5 - distance).clamp(0.0, 1.0);
                    if body < 1.0 {
                        blend(px, d.rim, coverage * d.alpha);
                    }
                    blend(px, d.body, body * coverage * d.alpha);
                } else {
                    blend(px, d.body, coverage * d.alpha);
                }
            }
        }
    }

    pub fn stamp(&mut self, s: &Stamp<'_>) {
        let sprite = s.sprite;
        let first = (s.y as i64).max(self.rows.start as i64);
        let last = ((s.y as i64) + sprite.height as i64).min(self.rows.end as i64);
        let x_from = (-s.x).max(0) as usize;
        let x_to = (self.width as i64 - s.x as i64).min(sprite.width as i64);
        if x_to <= x_from as i64 {
            return;
        }
        for y in first..last {
            let sy = (y - s.y as i64) as usize;
            let row = (y as usize - self.rows.start) * self.width;
            for sx in x_from..x_to as usize {
                let i = sy * sprite.width + sx;
                let (halo, fill) = (sprite.halo[i], sprite.fill[i]);
                if halo == 0 && fill == 0 {
                    continue;
                }
                let at = (row + (s.x + sx as i32) as usize) * 4;
                let px = &mut self.pixels[at..at + 4];
                blend(px, s.halo, f32::from(halo) / 255.0 * s.alpha);
                blend(px, s.fill, f32::from(fill) / 255.0 * s.alpha);
            }
        }
    }
}

fn clip(a: [f32; 2], b: [f32; 2], x0: f32, y0: f32, x1: f32, y1: f32) -> Option<([f32; 2], [f32; 2])> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-dx, a[0] - x0), (dx, x1 - a[0]), (-dy, a[1] - y0), (dy, y1 - a[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = t0.max(r);
        } else {
            t1 = t1.min(r);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some(([a[0] + dx * t0, a[1] + dy * t0], [a[0] + dx * t1, a[1] + dy * t1]))
}
