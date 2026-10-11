//! Анимированные узоры обложек, нарисованные прямо в пиксели, без растеризатора.
//! Буфер — premultiplied RGBA (`r | g << 8 | b << 16 | a << 24`), прозрачный на входе.

use std::sync::{Arc, Mutex};

pub fn fps(id: &str) -> f64 {
    match id {
        "city" => 20.0,
        "dreams" => 30.0,
        _ => 45.0,
    }
}

pub fn animated(id: &str) -> bool {
    matches!(id, "city" | "dreams" | "tunnel")
}

/// Буфер под обложкой: `origin` — где его левый верхний пиксель относительно угла обложки,
/// в физических пикселях; `size` и `radius` — размер и скругление обложки в логических.
pub struct Target<'a> {
    pub pixels: &'a mut [u32],
    pub width: usize,
    pub height: usize,
    pub origin: (f64, f64),
    pub scale: f64,
    pub size: (f64, f64),
    pub radius: f64,
}

pub fn render(id: &str, target: &mut Target<'_>, seconds: f64, viewport: f64) {
    match id {
        "city" => city(target, seconds),
        "dreams" => dreams(target, seconds, viewport),
        "tunnel" => tunnel(target, seconds),
        _ => return,
    }
    round_corners(target);
}

fn pack(rgb: [u8; 3], alpha: f64) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    let c = |v: u8| (u32::from(v) * a + 127) / 255;
    c(rgb[0]) | c(rgb[1]) << 8 | c(rgb[2]) << 16 | a << 24
}

fn scale_pixel(p: u32, k: f64) -> u32 {
    let m = |shift: u32| ((f64::from((p >> shift) & 0xff) * k).round() as u32) << shift;
    m(0) | m(8) | m(16) | m(24)
}

pub fn over(source: u32, dest: u32) -> u32 {
    let inverse = 255 - (source >> 24);
    let rb = (dest & 0x00ff_00ff) * inverse;
    let ag = ((dest >> 8) & 0x00ff_00ff) * inverse;
    let rb = ((rb + ((rb >> 8) & 0x00ff_00ff) + 0x0080_0080) >> 8) & 0x00ff_00ff;
    let ag = (ag + ((ag >> 8) & 0x00ff_00ff) + 0x0080_0080) & 0xff00_ff00;
    source + (rb | ag)
}

fn round_corners(t: &mut Target<'_>) {
    let (w, h) = (t.size.0 * t.scale, t.size.1 * t.scale);
    let r = t.radius * t.scale;
    let column = |x: f64| ((x - t.origin.0).max(0.0) as usize).min(t.width);
    for j in 0..t.height {
        let y = t.origin.1 + j as f64 + 0.5;
        let reach = if y < r + 1.0 || y > h - r - 1.0 { r + 1.0 } else { 1.0 };
        let (left, right) = if y < 1.0 || y > h - 1.0 { (t.width, t.width) } else { (column(reach), column(w - reach).max(column(reach))) };
        let row = &mut t.pixels[j * t.width..(j + 1) * t.width];
        for i in (0..left).chain(right..t.width) {
            let pixel = &mut row[i];
            if *pixel == 0 {
                continue;
            }
            let x = t.origin.0 + i as f64 + 0.5;
            let (cx, cy) = (x.clamp(r, w - r), y.clamp(r, h - r));
            let inside = r - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            let coverage = (inside.min(x.min(w - x).min(y).min(h - y)) + 0.5).clamp(0.0, 1.0);
            if coverage < 1.0 {
                *pixel = scale_pixel(*pixel, coverage);
            }
        }
    }
}

const TUNNEL_TILE: f64 = 44.0;
const TUNNEL_LINE: f64 = 2.0;
const TUNNEL_PERSPECTIVE: f64 = 84.0;
const TUNNEL_SECONDS: f64 = 2.0;
const TUNNEL_STREAK: [u8; 3] = [0xa2, 0xce, 0xf4];

struct Plane {
    top: f64,
    width: f64,
    height: f64,
    angle: f64,
    offset: (f64, f64),
}

impl Plane {
    fn limits(&self) -> (f64, f64) {
        let near = (TUNNEL_PERSPECTIVE - 1.0) / self.angle.sin();
        let half = self.height / 2.0;
        let (lo, hi) = if near > 0.0 { (-half, near.min(half)) } else { (near.max(-half), half) };
        (lo + half, hi + half)
    }

    /// Точка плоскости (по глубине) для экранной строки `y` (логические px от верха обложки).
    fn depth(&self, y: f64, h: f64) -> Option<f64> {
        let (sin, cos) = self.angle.sin_cos();
        let screen = y - h / 2.0;
        let shift = self.top + self.height / 2.0 - h / 2.0;
        let denominator = TUNNEL_PERSPECTIVE * cos + screen * sin;
        (denominator > 1e-9).then(|| TUNNEL_PERSPECTIVE * (screen - shift) / denominator + self.height / 2.0)
    }

    fn scale_at(&self, depth: f64) -> f64 {
        TUNNEL_PERSPECTIVE / (TUNNEL_PERSPECTIVE - (depth - self.height / 2.0) * self.angle.sin())
    }
}

/// Сколько линий шириной `TUNNEL_LINE` с шагом `TUNNEL_TILE` от `offset` попадает в `[0, p]`.
fn lines_until(p: f64, offset: f64) -> f64 {
    let local = p - offset;
    (local / TUNNEL_TILE).floor() * TUNNEL_LINE + local.rem_euclid(TUNNEL_TILE).min(TUNNEL_LINE)
}

fn tunnel(t: &mut Target<'_>, seconds: f64) {
    let (w, h) = t.size;
    let f = (seconds % TUNNEL_SECONDS) / TUNNEL_SECONDS;
    let planes = [
        Plane { top: 0.0, width: w * 2.0, height: h * 1.3, angle: 85f64.to_radians(), offset: (TUNNEL_TILE * f, -TUNNEL_TILE + TUNNEL_TILE * f) },
        Plane { top: -h * 0.3, width: w * 2.0, height: h * 1.3, angle: -85f64.to_radians(), offset: (TUNNEL_TILE * f, -TUNNEL_TILE * f) },
    ];
    let mut lines: Vec<(usize, f32)> = Vec::new();
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
            let mut x = plane.offset.0.rem_euclid(TUNNEL_TILE) - TUNNEL_TILE;
            while x < plane.width {
                let (a, b) = (to_buffer(x.max(0.0)), to_buffer((x + TUNNEL_LINE).min(plane.width)));
                x += TUNNEL_TILE;
                if b <= 0.0 || a >= t.width as f64 || b <= a {
                    continue;
                }
                for i in (a.max(0.0).floor() as usize)..(b.ceil() as usize).min(t.width) {
                    lines.push((i, ((b.min(i as f64 + 1.0) - a.max(i as f64)).clamp(0.0, 1.0) * weight) as f32));
                }
            }
        }
        let out = &mut t.pixels[j * t.width..(j + 1) * t.width];
        if base > 0.002 && span.1 > span.0 {
            out[span.0..span.1].fill(pack(TUNNEL_STREAK, f64::from(base)));
        }
        for &(i, cover) in &lines {
            let under = if (span.0..span.1).contains(&i) { base } else { 0.0 };
            let current = f32::from((out[i] >> 24) as u8) / 255.0;
            let total = 1.0 - (1.0 - current.max(under)) * (1.0 - cover);
            if total > 0.002 {
                out[i] = pack(TUNNEL_STREAK, f64::from(total));
            }
        }
    }
}

const CITY_TILE: &[u8] = include_bytes!("../../assets/city-cover.png");
const CITY_WIDTH: f64 = 640.0;
const CITY_HEIGHT: f64 = 442.0;
const CITY_STRIP: f64 = CITY_WIDTH * 9.0;
const CITY_SECONDS: f64 = 60.0;

type Tile = (u32, usize, usize, Arc<Vec<u32>>);

/// Плитка «города» в физических пикселях для данного масштаба (premultiplied RGBA).
fn city_tile(scale: f64) -> Option<Tile> {
    static TILE: Mutex<Option<Tile>> = Mutex::new(None);
    let key = (scale * 1000.0).round() as u32;
    let mut cached = TILE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tile) = cached.as_ref().filter(|t| t.0 == key) {
        return Some(tile.clone());
    }
    let (w, h) = ((CITY_WIDTH * scale).round() as u32, (CITY_HEIGHT * scale).round() as u32);
    let image = image::load_from_memory(CITY_TILE).ok()?.resize_exact(w.max(1), h.max(1), image::imageops::FilterType::Triangle).into_rgba8();
    let pixels = image.pixels().map(|p| pack([p[0], p[1], p[2]], f64::from(p[3]) / 255.0)).collect();
    let tile = (key, w as usize, h as usize, Arc::new(pixels));
    *cached = Some(tile.clone());
    Some(tile)
}

fn city(t: &mut Target<'_>, seconds: f64) {
    let Some((_, tw, th, tile)) = city_tile(t.scale) else { return };
    let (w, h) = t.size;
    let shift = -CITY_WIDTH + CITY_WIDTH * (seconds % CITY_SECONDS) / CITY_SECONDS;
    let start = w / 2.0 - CITY_STRIP / 2.0 + shift;
    let top = ((h - CITY_HEIGHT) * t.scale - t.origin.1).round() as isize;
    let first = ((-start) / CITY_WIDTH).floor().max(0.0);
    let mut x = start + first * CITY_WIDTH;
    while x < w && x < start + CITY_STRIP {
        let left = (x * t.scale - t.origin.0).round() as isize;
        x += CITY_WIDTH;
        for j in 0..th {
            let y = top + j as isize;
            if y < 0 || y >= t.height as isize {
                continue;
            }
            let (from, to) = (left.max(0), (left + tw as isize).min(t.width as isize));
            if to <= from {
                continue;
            }
            let row = &tile[j * tw..(j + 1) * tw];
            let dest = &mut t.pixels[y as usize * t.width..];
            dest[from as usize..to as usize].copy_from_slice(&row[(from - left) as usize..(to - left) as usize]);
        }
    }
}

const DREAM_PARTICLES: u32 = 40;
const DREAM_COLOR: [u8; 3] = [153, 255, 255];

fn dream_opacity(phase: f64) -> f64 {
    if phase < 0.15 {
        0.9 * phase / 0.15
    } else if phase < 0.85 {
        0.9 - 0.2 * (phase - 0.15) / 0.7
    } else {
        0.7 * (1.0 - phase) / 0.15
    }
}

fn dreams(t: &mut Target<'_>, seconds: f64, viewport: f64) {
    let (w, h) = t.size;
    let millis = seconds * 1000.0;
    for i in 0..DREAM_PARTICLES {
        let size = f64::from(4 + (i * 13) % 7);
        let delay = f64::from((i * 1700) % 28000);
        let duration = f64::from(24000 + (i * 2300) % 9000);
        let phase = ((millis + delay) % duration) / duration;
        let rise = viewport * (0.1 - 1.3 * phase);
        let grow = 0.5 + 1.3 * phase;
        let cx = (w * f64::from((i * 37) % 100) / 100.0 + size / 2.0) * t.scale - t.origin.0;
        let cy = (h * 1.1 - size / 2.0 + rise) * t.scale - t.origin.1;
        let radius = size / std::f64::consts::SQRT_2 * 0.65 * grow * t.scale;
        let opacity = dream_opacity(phase);
        let (x0, x1) = ((cx - radius).floor().max(0.0) as usize, ((cx + radius).ceil().max(0.0) as usize).min(t.width));
        let (y0, y1) = ((cy - radius).floor().max(0.0) as usize, ((cy + radius).ceil().max(0.0) as usize).min(t.height));
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt() / radius;
                if d < 1.0 {
                    let pixel = &mut t.pixels[y * t.width + x];
                    *pixel = over(pack(DREAM_COLOR, opacity * (1.0 - d)), *pixel);
                }
            }
        }
    }
}
