mod prepare;
mod shape;

use super::css::{Background, Rgba, Val};
use prepare::prepare;
use shape::Prepared;

const BACKGROUND_THREADS: usize = 2;
const MAX_DRIFT: f64 = 0.3;

type Premul = [f32; 4];

fn premul(c: Rgba) -> Premul {
    let a = c[3] as f32;
    [c[0] as f32 * a, c[1] as f32 * a, c[2] as f32 * a, a]
}

fn over(dst: &mut Premul, src: Premul) {
    let k = 1.0 - src[3];
    for i in 0..4 {
        dst[i] = src[i] + dst[i] * k;
    }
}

fn prepared(backgrounds: &[&Background], area: (f64, f64)) -> Vec<(Premul, Vec<Prepared>)> {
    backgrounds.iter().map(|b| (premul(b.color), b.layers.iter().rev().filter_map(|l| prepare(l, area)).collect())).collect()
}

fn fixed(backgrounds: &[&Background], axis: fn(&(Val, Val)) -> Val) -> bool {
    backgrounds.iter().flat_map(|b| &b.layers).all(|l| l.size.as_ref().map(axis).is_some_and(|v| v.pct == 0.0))
}

fn period(lengths: impl Iterator<Item = f64> + Clone, scale: f64, span: u32) -> Option<u32> {
    let limit = span / 2;
    (1..=limit).find(|&n| {
        lengths.clone().all(|length| {
            let unit = scale * length;
            let times = (f64::from(n) / unit).round();
            times >= 1.0 && (f64::from(n) - times * unit).abs() * f64::from(span) / f64::from(n) <= MAX_DRIFT
        })
    })
}

pub fn tile(backgrounds: &[&Background], width: u32, height: u32, scale: f64) -> Option<(u32, u32)> {
    let area = (f64::from(width) / scale, f64::from(height) / scale);
    let layers = prepared(backgrounds, area);
    let tiles = || layers.iter().flat_map(|(_, layers)| layers.iter().map(|l| l.tile));
    let across = fixed(backgrounds, |s| s.0).then(|| period(tiles().map(|t| t.0), scale, width)).flatten();
    let down = fixed(backgrounds, |s| s.1).then(|| period(tiles().map(|t| t.1), scale, height)).flatten();
    (across.is_some() || down.is_some()).then(|| (across.unwrap_or(width), down.unwrap_or(height)))
}

pub struct Cell {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn render(backgrounds: &[&Background], width: u32, height: u32, scale: f64) -> Cell {
    let (across, down) = tile(backgrounds, width, height, scale).unwrap_or((width, height));
    Cell { pixels: rasterize(backgrounds, across, down, scale, (width, height)), width: across, height: down }
}

#[cfg(test)]
pub fn expand(cell: &Cell, width: u32, height: u32) -> Vec<u8> {
    let (across, down, width, height) = (cell.width as usize, cell.height as usize, width as usize, height as usize);
    let mut out = vec![0u8; width * height * 4];
    for (y, line) in out.chunks_exact_mut(width * 4).enumerate() {
        let source = &cell.pixels[(y % down) * across * 4..][..across * 4];
        for chunk in line.chunks_mut(across * 4) {
            chunk.copy_from_slice(&source[..chunk.len()]);
        }
    }
    out
}

pub(super) fn rasterize(backgrounds: &[&Background], width: u32, height: u32, scale: f64, whole: (u32, u32)) -> Vec<u8> {
    let area = (f64::from(whole.0) / scale, f64::from(whole.1) / scale);
    let prepared = prepared(backgrounds, area);
    let mut out = vec![0u8; (width * height * 4) as usize];
    let row = (width * 4) as usize;
    let rows_per = (height as usize).div_ceil(BACKGROUND_THREADS).max(1);
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
