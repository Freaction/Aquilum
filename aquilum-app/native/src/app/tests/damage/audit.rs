use super::*;

#[test]
fn right_sidebar_scroll_stays_inside_its_viewport_scroll_audit() {
    let spec = FIXED;
    let mut h = setup(spec, &plain_body(8));
    if !h.app.interface.right_open() {
        let toggle = h.app.chrome.as_ref().unwrap().panel_toggle;
        h.act(toggle, crate::ui::widgets::Pressed);
    }
    h.app.listing = Some(crate::links::Listing {
        generation: h.app.panel_generation,
        notice: None,
        items: (0..80).map(|i| crate::links::Item { title: format!("Связанная заметка {i:03}"), counter: None, target: crate::links::Target::Wiki(format!("Заметка {i}")), reasons: Vec::new() }).collect(),
    });
    h.app.render_panel(&mut h.root);
    h.settle();
    let panel = rect_of(&h, h.app.chrome.as_ref().unwrap().panel_parts.body.id(), spec.scale);
    assert!(panel.width() > 10.0);
    let mut frames = Frames::new(&mut h, spec, true);
    let mut reports = Vec::new();
    for step in 0..12 {
        h.scroll((panel.center().x, panel.center().y), spec.scale, (0.0, if step < 8 { -17.3 } else { 13.1 }));
        let report = frames.draw(&mut h);
        assert_eq!(report.differ, 0, "right sidebar, step {step}");
        for &(x, y, w, h) in &report.rects {
            assert!(f64::from(x) >= panel.x0.floor() - 4.0 && f64::from(x + w) <= panel.x1.ceil() + 4.0 && f64::from(y) >= panel.y0.floor() - 4.0 && f64::from(y + h) <= panel.y1.ceil() + 4.0, "raster escaped the sidebar: {:?}", report.rects);
        }
        reports.push(report);
    }
    assert!(reports.iter().filter(|r| !r.moved.is_empty()).count() >= 8, "sidebar was rasterized instead of shifted");
    summary("правая панель", &reports);
}

#[test]
fn settings_scroll_matches_full_frames_on_desktop_and_phone_scroll_audit() {
    for spec in [FIXED, Spec { scale: 2.625, width: 1080, height: 1600 }] {
        let mut h = sized(spec, None);
        h.app.open_settings(&mut h.root);
        h.settle();
        for section in [Section::Ui, Section::Editor] {
            h.act(h.nav(section), crate::ui::widgets::Pressed);
            h.settle();
            assert_eq!(h.section(), Some(section));
            let content = match &h.app.popup { Some(Popup::Settings { dialog, .. }) => dialog.content.id(), _ => unreachable!() };
            let viewport = rect_of(&h, content, spec.scale);
            let mut frames = Frames::new(&mut h, spec, true);
            let mut reports = Vec::new();
            let mut moved = 0;
            for step in 0..12 {
                let before = scroll_offset(&h, content);
                h.scroll((viewport.center().x, viewport.center().y), spec.scale, (0.0, if step < 8 { -17.3 } else { 13.1 }));
                let changed = scroll_offset(&h, content) != before;
                let report = frames.draw(&mut h);
                if report.differ > 0 {
                    report.print("settings mismatch");
                    dump("settings-partial.png", &frames.partial);
                    let (layers, _) = h.root.redraw();
                    dump("settings-full.png", &render_full(&layers, frames.width, frames.height, spec.scale));
                }
                assert_eq!(report.differ, 0, "settings {section:?} at scale {}, step {step}", spec.scale);
                if changed {
                    moved += 1;
                    assert!(!report.moved.is_empty(), "settings {section:?} at scale {}, step {step}: scrolling did not shift the canvas", spec.scale);
                }
                reports.push(report);
            }
            assert!(moved > 0, "the settings content never scrolled");
            summary(&format!("настройки {section:?} @{}", spec.scale), &reports);
        }
    }
}

#[test]
fn reader_frames_match_and_an_opaque_reader_stops_hidden_covers_scroll_audit() {
    struct Clock;
    impl Drop for Clock { fn drop(&mut self) { crate::render::live::freeze(None); } }
    let _clock = Clock;
    crate::render::live::freeze(Some(0.0));
    let spec = FIXED;
    let mut h = setup(spec, "---\ncover: true\nPage_cover: tunnel\n---\nТекст за читалкой");
    h.wait_cover(true);
    let file = h.vault.path().join("книга.fb2");
    std::fs::write(&file, crate::reader::tests::sample_fb2()).unwrap();
    let mut config = h.config();
    config.reader.flow = "scrolled".into();
    h.app.workspace.core.settings.update_config(config).unwrap();
    h.app.open_reader(&mut h.root, crate::app::reader::Opening { file, page: None, title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    h.settle();
    let mut frames = Frames::new(&mut h, spec, true);
    assert!(!frames.partial.has_live(), "the animated cover behind an opaque reader is still running");
    let mut reports = Vec::new();
    for step in 0..10 {
        h.scroll((f64::from(spec.width) / 2.0, f64::from(spec.height) / 2.0), spec.scale, (0.0, if step < 6 { -31.3 } else { 19.1 }));
        h.anim(std::time::Duration::from_millis(16));
        h.deliver();
        let report = frames.draw(&mut h);
        assert_eq!(report.differ, 0, "reader, step {step}");
        reports.push(report);
    }
    summary("читалка", &reports);
    h.escape();
    h.settle();
    assert_eq!(frames.draw(&mut h).differ, 0, "the note did not return after closing the reader");
    assert!(frames.partial.has_live(), "the visible cover did not resume");
}
