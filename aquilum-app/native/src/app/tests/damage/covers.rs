use super::*;

struct Clock;
impl Drop for Clock { fn drop(&mut self) { crate::render::live::freeze(None); } }

#[test]
fn covers_and_book_thumbnails_match_full_frames_through_resize_cover_audit() {
    let _clock = Clock;
    crate::render::live::freeze(Some(0.0));
    for source in ["pattern:origami", "pattern:city", "pattern:tunnel", "photo.png"] {
        let mut h = sized(FIXED, None);
        image::RgbaImage::from_pixel(1200, 800, image::Rgba([40, 160, 210, 255])).save(h.vault.path().join("photo.png")).unwrap();
        let kind = if source == "photo.png" { "" } else { "type: book\n" };
        let note = h.write_note("Книга.md", &format!("---\ncover: true\n{kind}Page_cover: {source}\nBook_cover: photo.png\n---\n{}", plain_body(5)));
        h.open(note);
        h.settle();
        if source != "photo.png" { h.wait_cover(true); }
        h.settle();
        let mut frames = Frames::new(&mut h, FIXED, true);
        assert_eq!(frames.draw(&mut h).raster, 0, "unchanged cover was repainted: {source}");
        for (logical_w, logical_h, scale) in [(1100.0, 700.0, 1.75), (900.0, 650.0, 1.75), (590.0, 650.0, 1.75), (720.0, 650.0, 1.75), (400.0, 700.0, 2.625), (1000.0, 600.0, 1.75)] {
            frames.scale = scale;
            frames.width = (logical_w * scale) as u16;
            frames.height = (logical_h * scale) as u16;
            h.root.handle_window_event(masonry::core::WindowEvent::Rescale(scale));
            h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(u32::from(frames.width), u32::from(frames.height))));
            h.app.window_resized(&mut h.root, Size::new(logical_w, logical_h));
            let report = frames.draw(&mut h);
            assert_eq!(report.differ, 0, "{source}, {logical_w}x{logical_h}@{scale}: partial resize frame differs");
            let window = crate::output::Area { x: 0, y: 0, w: u32::from(frames.width), h: u32::from(frames.height) };
            assert!(report.shown.iter().all(|a| a.intersect(window) == Some(*a)), "output includes an old canvas area: {:?}", report.shown);
            let ids = &h.app.note_ids.as_ref().unwrap().hero.1;
            let bounds = rect_of(&h, ids.cover.unwrap().id(), scale);
            assert!(bounds.width() > 100.0 && bounds.height() > 100.0);
            if source == "photo.png" {
                let (x, y) = ((bounds.x1 - 40.0 * scale) as usize, (bounds.y0 + bounds.height() / 2.0) as usize);
                let pixel = frames.partial.frame_pixels()[y * usize::from(frames.width) + x];
                assert!(channel(pixel, 16).abs_diff(40) <= 3 && channel(pixel, 8).abs_diff(160) <= 3 && channel(pixel, 0).abs_diff(210) <= 3, "photo vanished at {logical_w}x{logical_h}@{scale}: {pixel:08x}");
            } else if source != "pattern:origami" {
                assert!(frames.partial.has_live(), "animated cover stopped during resize: {source}");
            }
        }
    }
}

#[test]
fn unchanged_book_buttons_do_not_repaint_and_state_changes_stay_local_cover_audit() {
    let mut h = setup(FIXED, "---\ncover: true\ntype: book\nPage_cover: pattern:polka\n---\nТекст");
    h.wait_cover(true);
    h.settle();
    let button = h.app.note_ids.as_ref().unwrap().hero.1.book_button.unwrap();
    let mut frames = Frames::new(&mut h, FIXED, true);
    for disabled in [false, true, true, false, false] {
        button.edit(&mut h.root, |mut button| {
            crate::ui::widgets::TextButton::set_disabled(&mut button, disabled);
            crate::ui::widgets::TextButton::set_pressed(&mut button, false);
        });
        let report = frames.draw(&mut h);
        assert_eq!(report.differ, 0, "button disabled={disabled}");
        assert!(report.raster < 50_000, "a button repainted the cover: {:?}", report.rects);
    }
    let report = frames.draw(&mut h);
    assert_eq!(report.raster, 0);
    assert!(report.shown.is_empty());
}
