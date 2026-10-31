use std::sync::atomic::{AtomicUsize, Ordering};

use masonry::peniko::Color;
use vello_cpu::Pixmap;

pub const VARIANTS: [&str; 2] = ["red", "warm"];

enum Filter {
    Luma { black: u32, gain: u32 },
    Scale([u32; 3]),
}

const fn luma(black: u32, white: u32) -> Filter {
    Filter::Luma { black, gain: (255 << 10) / (white - black) }
}

const FILTERS: [Filter; 2] = [luma(10, 204), Filter::Scale([256, 191, 138])];

const CANVAS_RED: u32 = if matches!(super::PIXEL_FORMAT, vello_cpu::PixelFormat::Rgba8) { 0 } else { 16 };

static ACTIVE: AtomicUsize = AtomicUsize::new(usize::MAX);

type Area = (usize, usize, usize, usize);

impl Filter {
    #[inline(always)]
    fn opaque(&self, pixel: u32, red: u32) -> u32 {
        let blue = 16 - red;
        let (r, g, b) = (pixel >> red & 0xff, pixel >> 8 & 0xff, pixel >> blue & 0xff);
        let alpha = pixel & 0xff00_0000;
        match *self {
            Filter::Luma { black, gain } => alpha | ((((54 * r + 183 * g + 19 * b) >> 8).saturating_sub(black) * gain) >> 10).min(255) << red,
            Filter::Scale([kr, kg, kb]) => alpha | ((r * kr) >> 8) << red | ((g * kg) >> 8) << 8 | ((b * kb) >> 8) << blue,
        }
    }

    #[inline(always)]
    fn rows(&self, pixels: &mut [u32], stride: usize, (x, y, w, h): Area) {
        for row in pixels.chunks_exact_mut(stride).skip(y).take(h) {
            for pixel in &mut row[x..x + w] {
                *pixel = self.opaque(*pixel, CANVAS_RED);
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    fn rows_avx2(&self, pixels: &mut [u32], stride: usize, area: Area) {
        self.rows(pixels, stride, area);
    }

    fn canvas(&self, pixels: &mut [u32], stride: usize, area: Area) {
        #[cfg(target_arch = "x86_64")]
        if std::arch::is_x86_feature_detected!("avx2") {
            return unsafe { self.rows_avx2(pixels, stride, area) };
        }
        self.rows(pixels, stride, area);
    }

    fn pixel(&self, pixel: u32, red: u32) -> u32 {
        match pixel >> 24 {
            255 => self.opaque(pixel, red),
            0 => pixel,
            alpha => {
                let straight = |at: u32| ((pixel >> at & 0xff) * 255 / alpha).min(255) << at;
                let opaque = self.opaque(0xff00_0000 | straight(0) | straight(8) | straight(16), red);
                let scaled = |at: u32| (((opaque >> at & 0xff) * alpha + 127) / 255) << at;
                alpha << 24 | scaled(0) | scaled(8) | scaled(16)
            }
        }
    }
}

pub fn apply(theme: &str, variant: &str) {
    let index = if theme == "night" { VARIANTS.iter().position(|v| *v == variant).unwrap_or(0) } else { usize::MAX };
    ACTIVE.store(index, Ordering::Relaxed);
}

fn active() -> Option<&'static Filter> {
    FILTERS.get(ACTIVE.load(Ordering::Relaxed))
}

pub fn color(color: Color) -> Color {
    let Some(filter) = active() else { return color };
    let [r, g, b, a] = color.to_rgba8().to_u8_array();
    let [r, g, b, _] = filter.opaque(u32::from_le_bytes([r, g, b, 255]), 0).to_le_bytes();
    Color::from_rgba8(r, g, b, a)
}

pub(super) fn canvas(pixels: &mut [u32], stride: usize, (x, y, w, h): (u16, u16, u16, u16)) {
    if let Some(filter) = active() {
        filter.canvas(pixels, stride, (usize::from(x), usize::from(y), usize::from(w), usize::from(h)));
    }
}

pub(super) fn image(pixmap: &mut Pixmap) {
    let Some(filter) = active() else { return };
    for pixel in pixmap.data_as_u8_slice_mut().chunks_exact_mut(4) {
        let rgba = filter.pixel(u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]), 0);
        pixel.copy_from_slice(&rgba.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: u32) -> f64 {
        let linear = f64::from(c) / 255.0;
        0.2126 * if linear <= 0.04045 { linear / 12.92 } else { ((linear + 0.055) / 1.055).powf(2.4) }
    }

    fn contrast(text: u32, background: u32) -> f64 {
        (luminance(text) + 0.05) / (luminance(background) + 0.05)
    }

    fn over(alpha: u32, under: [u32; 3]) -> [u32; 3] {
        under.map(|c| (255 * alpha + c * (255 - alpha)) / 255)
    }

    fn rgb(filter: &Filter, [r, g, b]: [u32; 3]) -> [u32; 3] {
        let pixel = filter.opaque(0xff00_0000 | b << 16 | g << 8 | r, 0);
        [pixel & 0xff, pixel >> 8 & 0xff, pixel >> 16 & 0xff]
    }

    #[test]
    fn the_red_filter_keeps_only_red_and_lifts_dim_text_off_dark_cards() {
        let red = &FILTERS[0];
        let card = [20, 24, 31];
        let [card_red, g, b] = rgb(red, card);
        assert_eq!([g, b], [0, 0]);
        let hint = rgb(red, over(153, card))[0];
        let body = rgb(red, over(230, card))[0];
        assert!(contrast(hint, card_red) >= 3.0, "подсказка на карточке: {:.2}", contrast(hint, card_red));
        assert!(contrast(body, card_red) >= 4.5, "текст на карточке: {:.2}", contrast(body, card_red));
        assert!(rgb(red, over(31, [9, 11, 17]))[0] > rgb(red, [9, 11, 17])[0], "граница видна на фоне окна");
    }

    #[test]
    fn the_warm_filter_keeps_red_and_cuts_blue_more_than_green() {
        let [r, g, b] = rgb(&FILTERS[1], [255, 255, 255]);
        assert_eq!(r, 255);
        assert!(b < g && g < 255, "белый становится тёплым: {r} {g} {b}");
        assert_eq!(rgb(&FILTERS[1], [0, 0, 0]), [0, 0, 0]);
    }

    #[test]
    fn canvas_order_and_translucent_pixels_are_filtered_the_same_way() {
        let white = 0xff00_0000 | 255 << 16 | 255 << 8 | 255;
        let bgra = FILTERS[1].opaque(white, 16);
        assert_eq!([bgra >> 16 & 0xff, bgra & 0xff], [255, 137], "в порядке BGRA красный остаётся красным");
        let half_white = 128 << 24 | 128 << 16 | 128 << 8 | 128;
        let pixel = FILTERS[1].pixel(half_white, 0);
        assert_eq!(pixel >> 24, 128);
        assert!((pixel & 0xff) <= 128 && (pixel >> 16 & 0xff) < (pixel & 0xff));
        assert_eq!(FILTERS[0].pixel(0, 0), 0);
    }
}
