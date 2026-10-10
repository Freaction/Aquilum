use std::sync::Arc;

use masonry::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
use masonry::core::{NewWidget, WidgetTag};
use masonry::dpi::PhysicalSize;
use masonry::peniko::Color;
use masonry::theme::default_property_set;
use masonry::widgets::TextArea;

use super::*;

const W: u16 = 400;
const H: u16 = 120;
const BG: Color = Color::from_rgb8(0x1f, 0x1f, 0x22);
const TAG: WidgetTag<TextArea<true>> = WidgetTag::named("t");

fn root(text: &str, scale: f64) -> RenderRoot {
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(u32::from(W), u32::from(H)),
        scale_factor: scale,
        test_font: None,
    };
    let area = TextArea::new_editable(text);
    RenderRoot::new(NewWidget::new(area).with_tag(TAG).erased(), |_| (), options)
}

struct Px {
    r: u8,
    g: u8,
    b: u8,
}

fn color_bbox(renderer: &Renderer) -> Option<(u32, u32, u32, u32)> {
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 0..u32::from(H) {
        for x in 0..u32::from(W) {
            let p = renderer.frame.2[usize::try_from(y * u32::from(W) + x).unwrap()];
            let px = Px { r: (p >> 16) as u8, g: (p >> 8) as u8, b: p as u8 };
            let bg_dist = (i32::from(px.r) - 0x1f).abs() + (i32::from(px.g) - 0x1f).abs() + (i32::from(px.b) - 0x22).abs();
            if bg_dist < 40 {
                continue;
            }
            let is_gray = (i32::from(px.r) - i32::from(px.g)).abs() < 15 && (i32::from(px.g) - i32::from(px.b)).abs() < 15;
            if is_gray {
                continue;
            }
            bbox = Some(match bbox {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bbox
}

fn render_emoji(scale: f64) -> (u32, u32, u32, u32) {
    let (layers, _) = root("ab 🦀", scale).redraw();
    let mut renderer = Renderer::new(0);
    renderer.render(&layers, W, H, scale, BG).expect("кадр");
    color_bbox(&renderer).expect("эмодзи нарисован")
}

/// На imaging_vello_cpu 0.0.1 (vello_cpu 0.0.7) масштаб окна не применялся к цветным глифам.
#[test]
fn color_glyphs_follow_window_scale() {
    let (x0, y0, x1, _) = render_emoji(1.0);
    let (sx0, sy0, sx1, _) = render_emoji(1.75);
    let near = |a: u32, b: f64| (f64::from(a) - b).abs() <= 3.0;
    assert!(near(sx0, f64::from(x0) * 1.75), "x: {x0} → {sx0}");
    assert!(near(sy0, f64::from(y0) * 1.75), "y: {y0} → {sy0}");
    assert!(near(sx1 - sx0, f64::from(x1 - x0) * 1.75), "ширина: {} → {}", x1 - x0, sx1 - sx0);
}

#[test]
#[ignore]
fn big_image_frame_cost() {
    let (w, h) = (3000u32, 2000u32);
    let data = masonry::peniko::ImageData {
        data: masonry::peniko::Blob::new(Arc::new(vec![200u8; (w * h * 4) as usize])),
        format: masonry::peniko::ImageFormat::Rgba8,
        alpha_type: masonry::peniko::ImageAlphaType::Alpha,
        width: w,
        height: h,
    };
    let image = masonry::widgets::Image::new(masonry::peniko::ImageBrush::new(data));
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(1200, 800),
        scale_factor: 1.0,
        test_font: None,
    };
    let mut root = RenderRoot::new(NewWidget::new(image).erased(), |_| (), options);
    let mut renderer = Renderer::new(2);
    let frames = 20;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        let (layers, _) = root.redraw();
        renderer.draw(&layers, 1200, 800, 1.0, BG, true).unwrap();
    }
    eprintln!("кадр с картинкой 3000×2000: {:?}", start.elapsed() / frames);
}
