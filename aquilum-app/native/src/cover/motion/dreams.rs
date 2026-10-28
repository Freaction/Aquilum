use super::{Target, pack};

const PARTICLES: u32 = 40;
const COLOR: [u8; 3] = [153, 255, 255];

fn phase_opacity(phase: f64) -> f64 {
    if phase < 0.15 {
        0.9 * phase / 0.15
    } else if phase < 0.85 {
        0.9 - 0.2 * (phase - 0.15) / 0.7
    } else {
        0.7 * (1.0 - phase) / 0.15
    }
}

pub(super) fn draw(t: &mut Target<'_>, seconds: f64, viewport: f64) {
    let (w, h) = t.size;
    let millis = seconds * 1000.0;
    for i in 0..PARTICLES {
        let size = f64::from(4 + (i * 13) % 7);
        let delay = f64::from((i * 1700) % 28000);
        let duration = f64::from(24000 + (i * 2300) % 9000);
        let phase = ((millis + delay) % duration) / duration;
        let rise = viewport * (0.1 - 1.3 * phase);
        let grow = 0.5 + 1.3 * phase;
        let cx = (w * f64::from((i * 37) % 100) / 100.0 + size / 2.0) * t.scale - t.origin.0;
        let cy = (h * 1.1 - size / 2.0 + rise) * t.scale - t.origin.1;
        let radius = size / std::f64::consts::SQRT_2 * 0.65 * grow * t.scale;
        let opacity = phase_opacity(phase);
        let (x0, x1) = ((cx - radius).floor().max(0.0) as usize, ((cx + radius).ceil().max(0.0) as usize).min(t.width));
        let (y0, y1) = ((cy - radius).floor().max(0.0) as usize, ((cy + radius).ceil().max(0.0) as usize).min(t.height));
        for y in y0..y1 {
            let mut row = t.row(y);
            for x in x0..x1 {
                let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt() / radius;
                if d < 1.0 {
                    row.put(x, pack(COLOR, opacity * (1.0 - d)));
                }
            }
        }
    }
}
