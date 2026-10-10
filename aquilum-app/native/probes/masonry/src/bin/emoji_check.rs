//! Проверка без окна: остаются ли пиксели эмодзи в буфере, если следующий кадр рисуется без них.
use std::sync::Arc;

use imaging_vello_cpu::VelloCpuRenderer;
use masonry::app::{RenderRoot, RenderRootOptions, VisualLayerKind, WindowSizePolicy};
use masonry::core::{NewWidget, WidgetTag};
use masonry::dpi::PhysicalSize;
use masonry::imaging::render::{ImageBufferFormat, ImageBufferTarget, ImageRenderer};
use masonry::peniko::color::AlphaColor;
use masonry::theme::default_property_set;
use masonry::widgets::TextArea;
use masonry_imaging::PreparedFrame;

const TAG: WidgetTag<TextArea<true>> = WidgetTag::named("t");
const W: u32 = 400;
const H: u32 = 120;

fn render(root: &mut RenderRoot, renderer: &mut VelloCpuRenderer, buf: &mut [u8]) {
    let (layers, _) = root.redraw();
    let Some(VisualLayerKind::Scene(base)) = layers.root_layer().map(|l| &l.kind) else { panic!() };
    let mut frame = PreparedFrame::new(W, H, 1.0, AlphaColor::from_rgb8(0x1f, 0x1f, 0x22), base, &[]);
    let target = ImageBufferTarget { data: buf, width: W, height: H, bytes_per_row: W as usize * 4, format: ImageBufferFormat::Rgba8Unorm };
    renderer.render_source_into(&mut frame, target).unwrap();
}

fn non_bg(buf: &[u8]) -> usize {
    buf.chunks_exact(4).filter(|p| (p[0], p[1], p[2]) != (0x1f, 0x1f, 0x22)).count()
}

/// Рамка «цветных» пикселей (краб оранжевый, текст серый).
fn color_bbox(buf: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for (i, p) in buf.chunks_exact(4).enumerate() {
        if (p[0] as i32 - p[2] as i32).abs() > 60 {
            let (x, y) = (i as u32 % W, i as u32 / W);
            b = Some(match b {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    b
}

fn render_scaled(root: &mut RenderRoot, scale: f64) -> Vec<u8> {
    let (layers, _) = root.redraw();
    let Some(VisualLayerKind::Scene(base)) = layers.root_layer().map(|l| &l.kind) else { panic!() };
    let mut frame = PreparedFrame::new(W, H, scale, AlphaColor::from_rgb8(0x1f, 0x1f, 0x22), base, &[]);
    let mut buf = vec![0u8; (W * H * 4) as usize];
    let target = ImageBufferTarget { data: &mut buf, width: W, height: H, bytes_per_row: W as usize * 4, format: ImageBufferFormat::Rgba8Unorm };
    VelloCpuRenderer::new(1, 1).render_source_into(&mut frame, target).unwrap();
    buf
}

fn main() {
    for (pad, scale) in [(0.0, 1.0), (40.0, 1.0), (0.0, 1.75)] {
        let options = RenderRootOptions {
            default_properties: Arc::new(default_property_set()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new(W, H),
            scale_factor: scale,
            test_font: None,
        };
        let area = NewWidget::new(TextArea::new_editable("ab 🦀"))
            .with_props(masonry::core::PropertySet::new().with(masonry::properties::Padding::all(masonry::layout::Length::px(pad))));
        let mut root = RenderRoot::new(area.erased(), |_| {}, options);
        let buf = render_scaled(&mut root, scale);
        println!("сдвиг {pad}, масштаб {scale}: эмодзи в {:?}", color_bbox(&buf));
    }

    for text in ["🦀🦀🦀 emoji", "plain text"] {
        let options = RenderRootOptions {
            default_properties: Arc::new(default_property_set()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new(W, H),
            scale_factor: 1.0,
            test_font: None,
        };
        let mut root = RenderRoot::new(NewWidget::new(TextArea::new_editable(text)).with_tag(TAG).erased(), |_| {}, options);
        let mut renderer = VelloCpuRenderer::new(1, 1);
        let mut buf = vec![0u8; (W * H * 4) as usize];
        render(&mut root, &mut renderer, &mut buf);
        let first = non_bg(&buf);
        root.edit_widget_with_tag(TAG, |mut t| TextArea::reset_text(&mut t, ""));
        render(&mut root, &mut renderer, &mut buf);
        let reused = non_bg(&buf);
        let mut fresh = vec![0u8; (W * H * 4) as usize];
        render(&mut root, &mut renderer, &mut fresh);
        println!("{text:?}: кадр 1 — {first} пикс. текста; кадр 2 в тот же буфер — {reused}; в чистый — {}", non_bg(&fresh));
    }
}
