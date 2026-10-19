mod city;

pub use city::forget as forget_tile;
mod dreams;
mod tunnel;

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
        "city" => city::draw(target, seconds),
        "dreams" => dreams::draw(target, seconds, viewport),
        "tunnel" => tunnel::draw(target, seconds),
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
