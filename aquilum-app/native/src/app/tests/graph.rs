use super::*;

#[test]
fn graph_tab_loads_opens_notes_and_unloads() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "[[a]] [[c]]").unwrap();
    std::fs::write(vault.join("c.md"), "[[a]]").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md"), vault.join("c.md")]);
    let button = h.app.chrome.as_ref().unwrap().graph_button.expect("кнопка графа");
    let rect = h.root.get_widget(button).unwrap().ctx().bounding_box();
    h.click(rect.center().x, rect.center().y);
    assert!(h.app.tabs.active().graph, "вкладка графа не открылась");
    assert!(h.wait_for(|h| h.app.graph_loaded() == Some(3)), "граф не загрузился: {:?}", h.app.graph_loaded());
    h.click(rect.center().x, rect.center().y);
    assert_eq!(h.app.tabs.tabs().iter().filter(|t| t.graph).count(), 1, "второй граф");
    let view = h.app.graph_view().unwrap();
    let area = h.root.get_widget(view).unwrap().ctx().bounding_box();
    let mut opened = false;
    'scan: for y in (area.y0 as i32..area.y1 as i32).step_by(6) {
        for x in (area.x0 as i32..(area.x1 - 260.0) as i32).step_by(6) {
            h.root.handle_pointer_event(moved(pointer_at(f64::from(x), f64::from(y), 1.0)));
            if h.root.cursor_icon() == masonry::core::CursorIcon::Pointer {
                h.click(f64::from(x), f64::from(y));
                opened = true;
                break 'scan;
            }
        }
    }
    assert!(opened, "под курсором не нашлось ни одного узла");
    assert!(!h.app.tabs.active().graph && h.app.tabs.active().path.is_some(), "щелчок по узлу не открыл заметку");
    assert!(h.app.tabs.has_graph() && h.app.graph_released(), "граф остался в памяти на соседней вкладке");
    let graph = h.app.tabs.tabs().iter().find(|t| t.graph).unwrap().id;
    assert!(h.app.tabs.select(graph));
    h.app.show_active(&mut h.root);
    assert!(h.wait_for(|h| h.app.graph_loaded() == Some(3)), "граф не загрузился заново");
    let note = h.app.tabs.tabs().iter().find(|t| !t.graph).unwrap().id;
    assert!(h.app.tabs.select(note));
    h.app.show_active(&mut h.root);
    h.app.tabs.close(graph);
    h.app.show_active(&mut h.root);
    assert!(h.app.graph_released(), "граф остался в памяти после закрытия вкладки");
}

#[test]
fn a_graph_left_for_a_note_is_unloaded_and_drawn_again_on_return() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "[[a]] [[c]]").unwrap();
    std::fs::write(vault.join("c.md"), "[[a]]").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md"), vault.join("c.md")]);
    let note = h.app.tabs.active_id();
    h.app.open_graph(&mut h.root);
    h.wait_for(|h| h.app.graph_loaded() == Some(3));
    let graph = h.app.tabs.active_id();
    let retained = |h: &Harness| {
        let view = h.app.graph_view().expect("виджет графа");
        h.root.get_widget(view).unwrap().downcast::<crate::ui::graph::GraphView>().unwrap().inner().retained()
    };
    for _ in 0..20 {
        h.settle();
    }
    let (sprites, frames) = retained(&h);
    assert!(sprites > 0 && frames > 0, "граф не нарисован: подписей {sprites}, кадров {frames}");
    assert!(h.app.tabs.select(note));
    h.app.show_active(&mut h.root);
    h.settle();
    assert!(h.app.graph_view().is_none() && h.app.graph_released(), "спрятанный граф остался в памяти");
    assert!(h.app.tabs.select(graph));
    h.app.show_active(&mut h.root);
    for _ in 0..300 {
        h.app.on_core_events(&mut h.root);
        h.settle();
        if h.app.graph_loaded() == Some(3) && retained(&h).1 > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let (sprites, frames) = retained(&h);
    assert!(sprites > 0 && frames > 0, "после возврата граф не нарисован: подписей {sprites}, кадров {frames}");
}

#[test]
#[ignore]
fn graph_snapshot() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let scale = 1.5;
    let mut h = Harness::with(Some(&vault), scale);
    h.app.open_graph(&mut h.root);
    for _ in 0..600 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if h.app.graph_loaded().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    eprintln!("узлов: {:?}", h.app.graph_loaded());
    let size = h.root.size();
    let shot = |h: &mut Harness, name: &str| {
        for _ in 0..40 {
            h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(16)));
        }
        let started = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let built = started.elapsed();
        let mut r = crate::render::Renderer::new(4);
        r.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
        eprintln!("{name}: сцена {built:?}, всего {:?}", started.elapsed());
        std::fs::write(out.join(format!("{name}.png")), r.png()).unwrap();
    };
    shot(&mut h, "graph-fit");
    let center = PhysicalPosition::new(f64::from(size.width) * 0.4, f64::from(size.height) * 0.5);
    for _ in 0..12 {
        let state = pointer_at(center.x, center.y, scale);
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::LineDelta(0.0, 1.0), state }));
    }
    shot(&mut h, "graph-zoom");
    h.root.handle_pointer_event(moved(pointer_at(center.x, center.y, scale)));
    shot(&mut h, "graph-hover");
}

#[test]
fn a_quick_return_to_the_graph_still_loads_it() {
    let mut h = Harness::new();
    std::fs::write(h.vault.path().join("b.md"), "[[a]]").unwrap();
    h.app.open_graph(&mut h.root);
    let graph = h.app.tabs.active_id();
    let _ = h.app.new_tab(&mut h.root);
    assert!(h.app.tabs.select(graph));
    h.app.show_active(&mut h.root);
    let loaded = (0..300).any(|_| {
        h.app.on_core_events(&mut h.root);
        h.settle();
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.app.graph_loaded().is_some()
    });
    assert!(loaded, "граф остался в загрузке после быстрого возврата");
}

#[test]
fn the_graph_tab_is_copied_into_the_frame_past_the_rasterizer() {
    let scale = 3.75;
    let mut h = Harness::with(None, scale);
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "[[a]] [[c]]").unwrap();
    std::fs::write(vault.join("c.md"), "[[a]]").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md"), vault.join("c.md")]);
    h.app.open_graph(&mut h.root);
    assert!(h.wait_for(|h| h.app.graph_loaded() == Some(3)), "граф не загрузился");
    let (layers, _) = h.root.redraw();
    let renderer = render_full(&layers, (f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16, scale);
    assert!(renderer.blitted, "картинка графа не копируется напрямую");
}
