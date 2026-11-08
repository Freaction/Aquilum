use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::Color;

use crate::output::{Canvas, Target};
use crate::render::{Surfaces, damage, scroll, sprite, stamp::Stamps};

#[test]
fn resized_surfaces_keep_only_a_reusable_buffer_scroll_audit() {
    let mut surfaces = Surfaces::default();
    let held = surfaces.owned_frame(64, 64, |_| {});
    for width in 65..80 {
        drop(surfaces.owned_frame(width, 64, |_| {}));
        assert!(surfaces.pool_len() <= 3, "unused surfaces of old sizes accumulate: {}", surfaces.pool_len());
    }
    assert!(crate::render::pixels::lookup(held.image.data.id()).is_some(), "a live frame was discarded");
    drop(held);
    drop(surfaces.owned_frame(79, 64, |_| {}));
    assert_eq!(surfaces.pool_len(), 1);
}

#[test]
fn absent_scrollbar_sprites_do_not_map_the_canvas_scroll_audit() {
    struct Unused;
    impl Canvas for Unused {
        fn with_target(&mut self, _: &mut dyn FnMut(Target<'_>)) { panic!("mapped a canvas without sprites"); }
    }
    sprite::stamp(&[], &mut Unused, &mut Stamps::default());
}

#[test]
fn shifted_rows_preserve_padding_and_pixels_outside_the_viewport_scroll_audit() {
    let (stride, height) = (29, 23);
    let before: Vec<u32> = (0..stride * height).map(|i| i as u32).collect();
    for dy in [-9i32, -1, 1, 9] {
        let shift = damage::Move { x: 3, y: 4, w: 20, h: 15, dy };
        let mut pixels = before.clone();
        scroll::shift_rows(&mut pixels, stride, shift);
        for y in 0..height {
            for x in 0..stride {
                let source_y = y as i32 - dy;
                let moved = (3..23).contains(&x) && (4..19).contains(&y) && (4..19).contains(&source_y);
                let expected = if moved { before[source_y as usize * stride + x] } else { before[y * stride + x] };
                assert_eq!(pixels[y * stride + x], expected, "dy {dy}, pixel {x},{y}");
            }
        }
    }
}

#[test]
fn a_resized_viewport_is_repainted_instead_of_shifting_old_pixels_scroll_audit() {
    const ID: u64 = u64::MAX - 1;
    struct Region;
    impl Drop for Region { fn drop(&mut self) { scroll::forget(ID); } }
    let _region = Region;
    let mut tracker = damage::Tracker::new();
    let mut frame = |viewport: Rect, offset: f64| {
        scroll::publish(ID, viewport, offset);
        tracker.begin(120, 100, 1.0);
        let mut painter = Painter::new(&mut tracker);
        painter.fill_rect(Rect::new(0.0, 0.0, 120.0, 100.0), Color::WHITE);
        painter.with_fill_clip(viewport, |painter| {
            for line in 0..20 {
                let y = f64::from(line) * 9.0;
                painter.fill(Rect::new(0.0, y, 70.0, y + 5.0), Color::BLACK).transform(Affine::translate((viewport.x0, viewport.y0 - offset))).draw();
            }
        });
        tracker.damage()
    };
    frame(Rect::new(10.0, 10.0, 110.0, 90.0), 0.0);
    let resized = frame(Rect::new(20.0, 12.0, 100.0, 84.0), 5.0);
    assert!(resized.moves.is_empty(), "moved pixels from a differently placed viewport");
}
