use std::sync::Arc;

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, WidgetTag};
use masonry::peniko::Color;
use masonry::widgets::TextArea;

use super::*;

const W: u16 = 400;
const H: u16 = 120;
const BG: Color = Color::from_rgb8(0x1f, 0x1f, 0x22);
const TAG: WidgetTag<TextArea<true>> = WidgetTag::named("t");

fn root(text: &str, scale: f64) -> RenderRoot {
    let options = crate::test_dir::options(u32::from(W), u32::from(H), scale);
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
    let options = crate::test_dir::options(1200, 800, 1.0);
    let mut root = RenderRoot::new(NewWidget::new(image).erased(), |_| (), options);
    let mut renderer = Renderer::new(2);
    let frames = 20;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        let (layers, _) = root.redraw();
        renderer.draw_frame(&layers, 1200, 800, 1.0, BG, true).unwrap();
    }
    eprintln!("кадр с картинкой 3000×2000: {:?}", start.elapsed() / frames);
}

#[test]
fn published_frame_survives_regions_that_miss_it() {
    use masonry::kurbo::Affine;
    let mut surfaces = surface::Surfaces::default();
    let frame = surfaces.frame(8, 8, |pixels| pixels.chunks_exact_mut(4).for_each(|pixel| pixel.copy_from_slice(&[200, 30, 30, 255])));
    let mut painter = paint::Painter::new(8, 8, paint::settings(0));
    let mut center = |with_image: bool| {
        painter.begin(0, 0, 8, 8);
        if with_image {
            masonry::imaging::Painter::new(&mut painter).draw_image(&frame, Affine::IDENTITY);
        }
        painter.finish().unwrap();
        let mut pixmap = vello_cpu::Pixmap::new(8, 8);
        painter.ctx.render_with(&mut pixmap, &mut painter.resources, vello_cpu::RasterizerSettings::default());
        pixmap.data()[4 * 8 + 4].r
    };
    assert_eq!(center(true), 200, "кадр не нарисован");
    for _ in 0..3 {
        center(false);
    }
    assert_eq!(center(true), 200, "кадр пропал после областей, которые его не задевали");
}

#[test]
fn overlay_leaves_no_trace() {
    use super::overlay::{Mark, Overlay};
    use super::stamp::Stamps;
    use crate::output::Area;
    let (w, h) = (64u16, 48u16);
    let clean: Vec<u32> = (0..u32::from(w) * u32::from(h)).map(|i| 0xff00_0000 | i.wrapping_mul(2_654_435_761) >> 8).collect();
    let mut canvas = (w, h, clean.clone());
    let mut overlay = Overlay::default();
    let mut stamps = Stamps::default();
    overlay.on = true;
    overlay.flash(Area { x: 4, y: 6, w: 20, h: 10 }, Mark::Raster);
    let shown = overlay.show(&mut canvas, &mut stamps, &[(Area { x: 0, y: 0, w: 64, h: 48 }, Mark::Region), (Area { x: 60, y: 40, w: 30, h: 30 }, Mark::Layer)]);
    assert!(shown.is_some());
    assert_ne!(canvas.2, clean, "подсветка видна");
    stamps.restore(&mut canvas);
    assert_eq!(canvas.2, clean, "после подсветки холст прежний");
}

#[test]
fn graph_frame_is_copied_exactly_like_the_rasterizer_draws_it() {
    use crate::graph::Graph;
    use crate::ui::graph::{Display, GraphView, Status};
    let days = vec![20_000.0f32; 4];
    let paths = (0..4).map(|i| format!("{i}.md")).collect();
    let graph = Graph::new(vec![[0.0, 0.0], [1.0, 0.5], [0.3, 1.2], [1.4, 1.1]], days.clone(), days, vec![2, 2, 1, 1], vec![[0, 1], [1, 2], [0, 3]], paths);
    let scale = 1.5;
    let view = GraphView::new(Some(Arc::new(graph)), Status::Ready, Display::default(), None, None);
    let mut root = RenderRoot::new(view.erased(), |_| (), crate::test_dir::options(u32::from(W), u32::from(H), scale));
    let _ = root.redraw();
    let (layers, _) = root.redraw();
    let mut renderer = Renderer::new(0);
    renderer.render(&layers, W, H, scale, BG).expect("кадр");
    assert!(renderer.blitted, "картинка графа копируется напрямую");
    let frame = renderer.frame(&layers).expect("кадр");
    let mut painter = paint::Painter::new(W, H, paint::settings(0));
    painter.begin(0, 0, W, H);
    frame.paint_into(&mut painter);
    painter.finish().unwrap();
    let mut expected = vec![0u32; usize::from(W) * usize::from(H)];
    let pixmap = vello_cpu::PixmapMut::new(W, H, bytemuck::cast_slice_mut(&mut expected)).unwrap();
    painter.ctx.render_with(pixmap, &mut painter.resources, vello_cpu::RasterizerSettings { pixel_format: PIXEL_FORMAT, ..Default::default() });
    let differ = renderer.frame_pixels().iter().zip(&expected).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 0, "кадр с прямым копированием совпадает с растеризатором");
}
