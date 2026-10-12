use masonry::peniko::Color;

pub(super) fn hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let [r, g, b] = [r, g, b].map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue, if max == 0.0 { 0.0 } else { delta / max }, max)
}

pub(super) fn rgb(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let c = value * saturation;
    let h = (hue / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = value - c;
    [r, g, b].map(|v| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

pub(super) fn color(hue: f64, saturation: f64, value: f64) -> Color {
    let [r, g, b] = rgb(hue, saturation, value);
    Color::from_rgb8(r, g, b)
}

#[cfg(test)]
mod tests {
    #[test]
    fn hsv_round_trips_hex() {
        for hex in [[0x14, 0x71, 0xeb], [0xe0, 0x45, 0x7b], [0, 0, 0], [255, 255, 255], [0x2a, 0x9d, 0x8f]] {
            let (h, s, v) = super::hsv(hex[0], hex[1], hex[2]);
            assert_eq!(super::rgb(h, s, v), hex);
        }
    }
}
