mod prepare;
mod shape;

use super::css::{Background, Rgba};
use prepare::prepare;
use shape::Prepared;

const BACKGROUND_THREADS: usize = 2;

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

pub fn render(backgrounds: &[&Background], width: u32, height: u32, scale: f64) -> Vec<u8> {
    let area = (f64::from(width) / scale, f64::from(height) / scale);
    let prepared: Vec<(Premul, Vec<Prepared>)> = backgrounds
        .iter()
        .map(|b| (premul(b.color), b.layers.iter().rev().filter_map(|l| prepare(l, area)).collect()))
        .collect();
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
