use std::io::Cursor;

use super::shrink::Shrink;

pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub natural: (u32, u32),
}

pub fn fit(natural: (u32, u32), limit: u32, max_pixels: f64) -> (u32, u32) {
    let (w, h) = (f64::from(natural.0), f64::from(natural.1));
    let k = (f64::from(limit) / w).min(f64::from(u16::MAX) / w.max(h)).min((max_pixels / (w * h)).sqrt());
    if k >= 1.0 {
        return natural;
    }
    ((w * k).round().clamp(1.0, w) as u32, (h * k).round().clamp(1.0, h) as u32)
}

pub fn decode(bytes: &[u8], limit: u32) -> Option<Decoded> {
    let fitted = |natural| fit(natural, limit, super::MAX_PIXELS);
    match image::guess_format(bytes).ok()? {
        image::ImageFormat::Png => png(bytes, fitted),
        image::ImageFormat::Jpeg => jpeg(bytes, fitted),
        _ => None,
    }
    .or_else(|| any(bytes, fitted))
}

fn png(bytes: &[u8], fitted: impl Fn((u32, u32)) -> (u32, u32)) -> Option<Decoded> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    if info.interlaced {
        return None;
    }
    let natural = (info.width, info.height);
    let channels = reader.output_color_type().0.samples();
    let mut shrink = Shrink::new(natural, fitted(natural));
    while let Some(row) = reader.next_row().ok()? {
        shrink.push(row.data(), channels);
    }
    shrink.finish(natural)
}

fn jpeg(bytes: &[u8], fitted: impl Fn((u32, u32)) -> (u32, u32)) -> Option<Decoded> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let natural = (u32::from(info.width), u32::from(info.height));
    let target = fitted(natural);
    let (w, h) = decoder.scale(target.0 as u16, target.1 as u16).ok()?;
    if u32::from(w) == natural.0 {
        return None;
    }
    let pixels = decoder.decode().ok()?;
    let channels = match decoder.info()?.pixel_format {
        jpeg_decoder::PixelFormat::L8 => 1,
        jpeg_decoder::PixelFormat::RGB24 => 3,
        _ => return None,
    };
    let from = (u32::from(w), u32::from(h));
    let mut shrink = Shrink::new(from, (target.0.min(from.0), target.1.min(from.1)));
    for row in pixels.chunks_exact(from.0 as usize * channels) {
        shrink.push(row, channels);
    }
    shrink.finish(natural)
}

fn any(bytes: &[u8], fitted: impl Fn((u32, u32)) -> (u32, u32)) -> Option<Decoded> {
    use image::DynamicImage::{ImageLuma8, ImageLumaA8, ImageRgb8, ImageRgba8};
    let image = image::load_from_memory(bytes).ok()?;
    let natural = (image.width(), image.height());
    let target = fitted(natural);
    let image = match image {
        ImageRgba8(rgba) if target == natural => return Some(Decoded { width: natural.0, height: natural.1, pixels: rgba.into_raw(), natural }),
        ImageLuma8(_) | ImageLumaA8(_) | ImageRgb8(_) | ImageRgba8(_) => image,
        other => other.into_rgba8().into(),
    };
    let channels = usize::from(image.color().channel_count());
    let mut shrink = Shrink::new(natural, target);
    for row in image.as_bytes().chunks_exact(natural.0 as usize * channels) {
        shrink.push(row, channels);
    }
    shrink.finish(natural)
}

#[cfg(test)]
mod tests {
    use super::{decode, fit};

    fn encode(image: &image::RgbaImage, format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        match format {
            image::ImageFormat::Jpeg => image::DynamicImage::ImageRgba8(image.clone()).into_rgb8().write_to(&mut std::io::Cursor::new(&mut bytes), format).unwrap(),
            _ => image.write_to(&mut std::io::Cursor::new(&mut bytes), format).unwrap(),
        }
        bytes
    }

    #[test]
    fn large_images_are_decoded_straight_to_the_shown_size() {
        let image = image::RgbaImage::from_fn(1000, 600, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, if x < 500 { 255 } else { 0 }]));
        for format in [image::ImageFormat::Png, image::ImageFormat::Jpeg, image::ImageFormat::WebP] {
            let decoded = decode(&encode(&image, format), 300).unwrap();
            assert_eq!((decoded.width, decoded.height, decoded.natural), (300, 180, (1000, 600)), "{format:?}");
            assert_eq!(decoded.pixels.len(), 300 * 180 * 4);
        }
        let png = decode(&encode(&image, image::ImageFormat::Png), 300).unwrap();
        let pixel = |x: usize, y: usize| &png.pixels[(y * 300 + x) * 4..][..4];
        assert_eq!(pixel(10, 10)[3], 255);
        assert_eq!(pixel(290, 10), [0, 0, 0, 0]);
        let small = decode(&encode(&image, image::ImageFormat::Png), 2000).unwrap();
        assert_eq!((small.width, small.height), (1000, 600));
        assert_eq!(&small.pixels[..8], &image.as_raw()[..8]);
    }

    #[test]
    fn transparent_edges_keep_their_color() {
        let image = image::RgbaImage::from_fn(4, 2, |x, _| if x % 2 == 0 { image::Rgba([255, 0, 0, 255]) } else { image::Rgba([0, 0, 0, 0]) });
        let decoded = decode(&encode(&image, image::ImageFormat::Png), 2).unwrap();
        assert_eq!(&decoded.pixels[..4], [255, 0, 0, 128]);
        let webp = decode(&encode(&image, image::ImageFormat::WebP), 2).unwrap();
        assert_eq!(&webp.pixels[..4], [255, 0, 0, 128]);
    }

    #[test]
    fn tall_images_respect_the_pixel_budget() {
        assert_eq!(fit((1000, 5000), 1600, 1600.0 * 1600.0), (716, 3578));
        assert_eq!(fit((800, 600), 1600, 1600.0 * 1600.0), (800, 600));
        assert_eq!(fit((10, 1_000_000), 1600, 1600.0 * 1600.0), (1, 65535));
    }
}
