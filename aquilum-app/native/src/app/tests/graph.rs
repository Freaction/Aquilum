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
    let wait = |h: &mut Harness, done: &dyn Fn(&Harness) -> bool| {
        for _ in 0..300 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if done(h) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    };
    h.click(rect.center().x, rect.center().y);
    assert!(h.app.tabs.active().graph, "вкладка графа не открылась");
    assert!(wait(&mut h, &|h| h.app.graph_loaded() == Some(3)), "граф не загрузился: {:?}", h.app.graph_loaded());
    h.click(rect.center().x, rect.center().y);
    assert_eq!(h.app.tabs.tabs().iter().filter(|t| t.graph).count(), 1, "второй граф");
    let view = h.app.graph_view().unwrap();
    let area = h.root.get_widget(view).unwrap().ctx().bounding_box();
    let mut opened = false;
    'scan: for y in (area.y0 as i32..area.y1 as i32).step_by(6) {
        for x in (area.x0 as i32..(area.x1 - 260.0) as i32).step_by(6) {
            h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate {
                pointer: MOUSE,
                current: PointerState { position: PhysicalPosition::new(f64::from(x), f64::from(y)), count: 1, scale_factor: 1.0, ..PointerState::default() },
                coalesced: vec![],
                predicted: vec![],
            }));
            if h.root.cursor_icon() == masonry::core::CursorIcon::Pointer {
                h.click(f64::from(x), f64::from(y));
                opened = true;
                break 'scan;
            }
        }
    }
    assert!(opened, "под курсором не нашлось ни одного узла");
    assert!(!h.app.tabs.active().graph && h.app.tabs.active().path.is_some(), "щелчок по узлу не открыл заметку");
    assert!(h.app.tabs.has_graph() && h.app.graph_loaded() == Some(3), "граф закрылся вместе с переходом к заметке");
    let graph = h.app.tabs.tabs().iter().find(|t| t.graph).unwrap().id;
    h.app.tabs.close(graph);
    h.app.show_active(&mut h.root);
    assert!(h.app.graph_released(), "граф остался в памяти после закрытия вкладки");
}

#[test]
fn hidden_graph_gives_back_frames_and_draws_labels_again() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "[[a]] [[c]]").unwrap();
    std::fs::write(vault.join("c.md"), "[[a]]").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md"), vault.join("c.md")]);
    let note = h.app.tabs.active_id();
    h.app.open_graph(&mut h.root);
    for _ in 0..300 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if h.app.graph_loaded() == Some(3) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
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
    assert_eq!(retained(&h), (0, 0), "спрятанный граф держит подписи и кадры");
    assert!(h.app.tabs.select(graph));
    h.app.show_active(&mut h.root);
    for _ in 0..20 {
        h.settle();
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
        let state = PointerState { position: center, count: 1, scale_factor: scale, ..PointerState::default() };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::LineDelta(0.0, 1.0), state }));
    }
    shot(&mut h, "graph-zoom");
    let state = PointerState { position: center, count: 1, scale_factor: scale, ..PointerState::default() };
    h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
    shot(&mut h, "graph-hover");
}

#[test]
fn typing_suggests_notes_and_enter_links_them() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("Проектирование.md"), "x").unwrap();
    let note = vault.join("Черновик.md");
    std::fs::write(&note, "").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("Проектирование.md"), note.clone()]);
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    let editor = h.app.editor_id().unwrap();
    h.root.handle_text_event(TextEvent::WindowFocusChange(true));
    h.root.focus_on(Some(editor));
    let text = |h: &Harness| h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text();
    for round in ["", "[["] {
        for ch in round.chars().chain("Прое".chars()) {
            h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
        }
        let mut shown = false;
        for _ in 0..200 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if h.app.suggest_layer.is_some() {
                shown = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(shown, "список подсказок не появился ({round:?}): {:?}", text(&h));
        if let Ok(out) = std::env::var("AQ_OUT") {
            let (layers, _) = h.root.redraw();
            let mut r = crate::render::Renderer::new(2);
            r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
            std::fs::write(std::path::Path::new(&out).join(format!("suggest-{}.png", round.len())), r.png()).unwrap();
        }
        h.key(Key::Named(NamedKey::Enter), Modifiers::empty());
        let _ = h.root.redraw();
        h.app.on_core_events(&mut h.root);
        assert!(text(&h).ends_with("[[Проектирование]]"), "{:?}", text(&h));
        assert!(h.app.suggest_layer.is_none(), "список не закрылся");
        h.key(Key::Character(" ".into()), Modifiers::empty());
    }
    assert_eq!(text(&h), "[[Проектирование]] [[Проектирование]] ");
}
