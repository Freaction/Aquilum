use masonry::kurbo::{Circle, Rect, Shape};
use masonry::peniko::Color;
use vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};

#[test]
fn reused_workers_match_single_threaded_rendering_after_resizes_and_flushes() {
    let mut contexts: Vec<_> = [0, 2, 4].into_iter().map(|threads| (RenderContext::new_with(1, 1, crate::render::render_settings(threads)), Resources::new())).collect();
    for (step, (w, h)) in [(1, 1), (65, 37), (9, 80), (160, 90), (8, 4), (4, 8), (65, 37)].into_iter().cycle().take(28).enumerate() {
        let mut expected = None;
        for (ctx, resources) in &mut contexts {
            // Empty resets must also reach the workers when work finally arrives.
            ctx.reset_and_resize(3, 5);
            ctx.reset_and_resize(w, h);
            let bounds = Rect::new(0.5, 0.5, f64::from(w) - 0.25, f64::from(h) - 0.25);
            ctx.push_clip_rect(&bounds);
            ctx.set_paint(Color::from_rgba8(180, 30, 90, 170));
            ctx.fill_path(&Circle::new((f64::from(w) * 0.5, f64::from(h) * 0.5), f64::from(w.max(h)) * 0.6).to_path(0.1));
            // Restarting the task loop must preserve strips and alpha data already recorded.
            ctx.flush();
            ctx.set_paint(Color::from_rgba8(20, 140, 200, 120));
            ctx.fill_rect(&Rect::new(1.0, 2.0, f64::from(w), f64::from(h)));
            ctx.pop_clip();
            ctx.flush();
            let mut pixels = Pixmap::new(180, 110);
            ctx.render_with(&mut pixels, resources, RasterizerSettings { offset: (4, 8), target_init: TargetInit::SrcOver, ..Default::default() });
            if let Some(expected) = &expected {
                assert_eq!(pixels.data(), expected, "step {step}, raster {w}x{h}");
            } else {
                expected = Some(pixels.data().to_vec());
            }
        }
    }
}
