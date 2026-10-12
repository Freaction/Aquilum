use std::hash::{Hash, Hasher};
use std::sync::Arc;

use krilla::geom::Size as KSize;
use krilla::image::{BitsPerComponent, CustomImage, Image, ImageColorspace};
use krilla::num::NormalizedF32;
use masonry::imaging::FillRef;
use masonry::kurbo::Affine;
use masonry::peniko::{self, ImageAlphaType, ImageBrushRef, ImageFormat};

use super::PdfSink;
use super::geometry::{rule, transform};

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

fn pixels(image: &peniko::ImageData) -> Pixels {
    let bytes = image.data.data();
    let count = (image.width * image.height) as usize;
    let (mut rgb, mut alpha) = (Vec::with_capacity(count * 3), Vec::with_capacity(count));
    for px in bytes.chunks_exact(4).take(count) {
        let (r, g, b) = match image.format {
            ImageFormat::Bgra8 => (px[2], px[1], px[0]),
            _ => (px[0], px[1], px[2]),
        };
        let a = px[3];
        let straight = |c: u8| if image.alpha_type == ImageAlphaType::AlphaPremultiplied && a > 0 && a < 255 { ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8 } else { c };
        rgb.extend([straight(r), straight(g), straight(b)]);
        alpha.push(a);
    }
    Pixels { id: image.data.id(), size: (image.width, image.height), rgb: Arc::new(rgb), alpha: Arc::new(alpha) }
}

impl PdfSink<'_, '_> {
    pub(super) fn fill_image(&mut self, draw: &FillRef<'_>, brush: &ImageBrushRef<'_>) {
        let pushed = self.clip(draw.transform, &draw.shape, rule(draw.fill_rule));
        let image = Image::from_custom(pixels(&brush.image), true);
        let placed = self.base * draw.transform * draw.brush_transform.unwrap_or(Affine::IDENTITY);
        if let (Ok(image), Some(size)) = (image, KSize::from_wh(brush.image.width as f32, brush.image.height as f32)) {
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
