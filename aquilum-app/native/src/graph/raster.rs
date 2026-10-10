use std::ops::Range;

pub type Rgb = [u8; 3];

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub color: Rgb,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Disc {
    pub center: [f32; 2],
    pub radius: f32,
    pub body: Rgb,
    pub rim: Rgb,
    pub inner: f32,
    pub alpha: f32,
}

pub struct Sprite {
    pub width: usize,
    pub height: usize,
    pub fill: Vec<u8>,
    pub halo: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct Stamp<'a> {
    pub sprite: &'a Sprite,
    pub x: i32,
    pub y: i32,
    pub fill: Rgb,
    pub halo: Rgb,
    pub alpha: f32,
}


pub const BAND: usize = 64;

#[inline]
fn blend(px: &mut [u8], color: Rgb, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    if alpha >= 1.0 {
        px[..3].copy_from_slice(&color);
        return;
    }
    let a = (alpha * 256.0) as i32;
    for i in 0..3 {
        let d = i32::from(px[i]);
        px[i] = (d + (((i32::from(color[i]) - d) * a) >> 8)) as u8;
    }
}

pub struct Band<'b> {
    pub rows: Range<usize>,
    pub width: usize,
    pixels: &'b mut [u8],
}

impl Band<'_> {
    #[inline]
    fn plot(&mut self, x: i32, y: i32, color: Rgb, alpha: f32) {
        if x < 0 || y < self.rows.start as i32 || y >= self.rows.end as i32 || x >= self.width as i32 {
            return;
        }
        let at = ((y as usize - self.rows.start) * self.width + x as usize) * 4;
        blend(&mut self.pixels[at..at + 4], color, alpha);
    }

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

pub fn bands(width: usize, height: usize, pixels: &mut [u8], draw: impl Fn(&mut Band<'_>) + Sync) {
    if width == 0 || height == 0 || pixels.len() < width * height * 4 {
        return;
    }
    use rayon::prelude::*;
    pixels[..width * height * 4].par_chunks_mut(BAND * width * 4).enumerate().for_each(|(index, chunk)| {
        let rows = index * BAND..((index + 1) * BAND).min(height);
        draw(&mut Band { rows, width, pixels: chunk });
    });
}

pub fn dilate(mask: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; mask.len()];
    for y in 0..height {
        for x in 0..width {
            let mut best = 0u8;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                        best = best.max(mask[ny as usize * width + nx as usize]);
                    }
                }
            }
            out[y * width + x] = best;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 3] {
        let i = (y * width + x) * 4;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    }

    #[test]
    fn draws_discs_lines_and_sprites_in_every_band() {
        let sprite = Sprite { width: 2, height: 2, fill: vec![255; 4], halo: vec![0; 4] };
        let edge = Edge { from: [10.5, 10.5], to: [290.5, 190.5], color: [0, 255, 0], alpha: 1.0 };
        let disc = Disc { center: [150.0, 100.0], radius: 20.0, body: [255, 0, 0], rim: [255, 255, 255], inner: 18.0, alpha: 1.0 };
        let stamp = Stamp { sprite: &sprite, x: 5, y: 130, fill: [0, 0, 255], halo: [0, 0, 0], alpha: 1.0 };
        let mut pixels = vec![0u8; 300 * 200 * 4];
        bands(300, 200, &mut pixels, |band| {
            band.line(&edge);
            band.disc(&disc);
            band.stamp(&stamp);
        });
        assert_eq!(at(&pixels, 300, 150, 100), [255, 0, 0]);
        assert_eq!(at(&pixels, 300, 150, 81), [255, 255, 255]);
        assert_eq!(at(&pixels, 300, 5, 130), [0, 0, 255]);
        assert!(at(&pixels, 300, 270, 177)[1] > 100 || at(&pixels, 300, 270, 178)[1] > 100);
        assert_eq!(at(&pixels, 300, 299, 0), [0, 0, 0]);
    }
}
