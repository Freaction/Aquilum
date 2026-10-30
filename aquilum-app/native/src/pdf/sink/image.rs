use std::hash::{Hash, Hasher};
use std::sync::Arc;

use image::RgbaImage;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use krilla::Data;
use krilla::geom::Size as KSize;
use krilla::image::{BitsPerComponent, CustomImage, Image, ImageColorspace};
use krilla::num::NormalizedF32;
use masonry::imaging::FillRef;
use masonry::kurbo::{Affine, Rect, Size};
use masonry::peniko::{self, ImageAlphaType, ImageBrushRef, ImageFormat};

use super::PdfSink;
use super::geometry::{rule, transform};

const DPI: f64 = 150.0;
const QUALITY: u8 = 80;

#[derive(Clone)]
struct Pixels {
    id: u64,
    size: (u32, u32),
    rgb: Arc<Vec<u8>>,
    alpha: Arc<Vec<u8>>,
}

impl Hash for Pixels {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.size.hash(state);
    }
}

impl CustomImage for Pixels {
    fn color_channel(&self) -> &[u8] {
        &self.rgb
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        Some(&self.alpha)
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        None
    }

    fn color_space(&self) -> ImageColorspace {
        ImageColorspace::Rgb
    }
}

fn straight(image: &peniko::ImageData) -> Option<RgbaImage> {
    let count = (image.width * image.height) as usize;
    let mut rgba = Vec::with_capacity(count * 4);
    for px in image.data.data().chunks_exact(4).take(count) {
        let (r, g, b, a) = match image.format {
            ImageFormat::Bgra8 => (px[2], px[1], px[0], px[3]),
            _ => (px[0], px[1], px[2], px[3]),
        };
        let straight = |c: u8| if image.alpha_type == ImageAlphaType::AlphaPremultiplied && a > 0 && a < 255 { ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8 } else { c };
        rgba.extend([straight(r), straight(g), straight(b), a]);
    }
    RgbaImage::from_raw(image.width, image.height, rgba)
}

fn compact(image: &peniko::ImageData, shown: Size) -> Option<Image> {
    let mut rgba = straight(image)?;
    let (w, h) = (f64::from(rgba.width()), f64::from(rgba.height()));
    let scale = (shown.width * DPI / 72.0 / w).max(shown.height * DPI / 72.0 / h).min(1.0);
    if scale < 1.0 {
        rgba = image::imageops::resize(&rgba, ((w * scale).ceil() as u32).max(1), ((h * scale).ceil() as u32).max(1), FilterType::Triangle);
    }
    if rgba.pixels().all(|p| p[3] == 255) {
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, QUALITY).encode_image(&image::DynamicImage::ImageRgba8(rgba).to_rgb8()).ok()?;
        return Image::from_jpeg(Data::from(jpeg), true).ok();
    }
    let size = (rgba.width(), rgba.height());
    let (rgb, alpha): (Vec<[u8; 3]>, Vec<u8>) = rgba.pixels().map(|p| ([p[0], p[1], p[2]], p[3])).unzip();
    Image::from_custom(Pixels { id: image.data.id(), size, rgb: Arc::new(rgb.concat()), alpha: Arc::new(alpha) }, true).ok()
}

impl PdfSink<'_, '_> {
    pub(super) fn fill_image(&mut self, draw: &FillRef<'_>, brush: &ImageBrushRef<'_>) {
        let pushed = self.clip(draw.transform, &draw.shape, rule(draw.fill_rule));
        let placed = self.base * draw.transform * draw.brush_transform.unwrap_or(Affine::IDENTITY);
        let (w, h) = (f64::from(brush.image.width), f64::from(brush.image.height));
        let shown = placed.transform_rect_bbox(Rect::new(0.0, 0.0, w, h)).size();
        if let (Some(image), Some(size)) = (compact(&brush.image, shown), KSize::from_wh(w as f32, h as f32)) {
            self.surface.push_transform(&transform(placed));
            if draw.composite.alpha < 1.0 || brush.sampler.alpha < 1.0 {
                self.surface.push_opacity(NormalizedF32::new((draw.composite.alpha * brush.sampler.alpha).clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE));
                self.surface.draw_image(image, size);
                self.surface.pop();
            } else {
                self.surface.draw_image(image, size);
            }
            self.surface.pop();
        }
        for _ in 0..pushed {
            self.surface.pop();
        }
    }
}
