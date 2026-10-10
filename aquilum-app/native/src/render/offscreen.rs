//! Отрисовка элементов интерфейса во внеэкранные растровые кисти peniko.

use vello_cpu::{Pixmap, RasterizerSettings, RenderSettings};

use super::paint::Painter;

pub fn offscreen(width: u16, height: u16, draw: impl FnOnce(&mut masonry::imaging::Painter<'_, Painter>)) -> Option<masonry::peniko::ImageBrush> {
    use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
    if width == 0 || height == 0 {
        return None;
    }
    let mut sink = Painter::new(width, height, RenderSettings { num_threads: 0, ..Default::default() });
    sink.begin(0, 0, width, height);
    draw(&mut masonry::imaging::Painter::new(&mut sink));
    sink.finish().ok()?;
    let mut pixmap = Pixmap::new(width, height);
    sink.ctx.render_with(&mut pixmap, &mut sink.resources, RasterizerSettings::default());
    let data = ImageData {
        data: Blob::new(std::sync::Arc::new(pixmap.data_as_u8_slice().to_vec())),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::AlphaPremultiplied,
        width: u32::from(width),
        height: u32::from(height),
    };
    Some(ImageBrush::new(data))
}
