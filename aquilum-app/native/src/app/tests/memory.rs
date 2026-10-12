use super::*;

fn mb(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}

fn settle(h: &mut Harness, renderer: &mut crate::render::Renderer) {
    for _ in 0..300 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if !h.app.embeds.as_ref().is_some_and(super::super::embeds::Embeds::busy) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for _ in 0..25 {
        std::thread::sleep(std::time::Duration::from_millis(80));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
    h.settle();
    let (layers, _) = h.root.redraw();
    let size = h.root.size();
    let _ = renderer.render(&layers, size.width as u16, size.height as u16, 1.75, theme::background());
}

fn line(step: &str) {
    eprintln!("{step:<32} куча {:>7.1} МБ", mb(crate::memory_probe::heap()));
}

#[test]
#[ignore]
fn memory_session_report() {
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").expect("AQ_VAULT"));
    let notes: Vec<std::path::PathBuf> = std::env::var("AQ_NOTES").expect("AQ_NOTES").split('|').map(|n| vault.join(n)).collect();
    crate::memory_probe::trace(std::env::var_os("AQ_PROBE_TRACE").is_some());
    let zones = std::env::var_os("AQ_TRACE");
    if zones.is_some() {
        aq_trace::start();
    }
    let mut h = Harness::with(Some(&vault), 1.75);
    let mut renderer = crate::render::Renderer::new(2);
    settle(&mut h, &mut renderer);
    line("старт");
    for note in &notes {
        let started = std::time::Instant::now();
        h.app.open_in(&mut h.root, note.clone(), true);
        let _ = h.root.redraw();
        let shown = started.elapsed().as_millis();
        settle(&mut h, &mut renderer);
        let name = note.file_stem().unwrap_or_default().to_string_lossy().chars().take(20).collect::<String>();
        line(&format!("открыта {name}, кадр {shown} мс"));
        if let Some(out) = &zones {
            aq_trace::write(std::path::Path::new(out)).unwrap();
        }
        if std::env::var_os("AQ_REPORT_EACH").is_some() {
            eprintln!("{}", crate::memory_probe::report());
        }
    }
    for round in 0..3 {
        let ids: Vec<_> = h.app.tabs.tabs().iter().map(|t| t.id).collect();
        for id in ids {
            if h.app.tabs.select(id) {
                h.app.show_active(&mut h.root);
            }
            settle(&mut h, &mut renderer);
        }
        line(&format!("круг вкладок {}", round + 1));
    }
    h.app.open_graph(&mut h.root);
    for _ in 0..200 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if h.app.graph_loaded().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    settle(&mut h, &mut renderer);
    line("граф");
    let graph_tab = h.app.tabs.active_id();
    let note_tab = h.app.tabs.tabs().iter().map(|t| t.id).find(|id| *id != graph_tab);
    if let Some(id) = note_tab
        && h.app.tabs.select(id)
    {
        h.app.show_active(&mut h.root);
        settle(&mut h, &mut renderer);
        line("граф в фоне, открыта заметка");
        if h.app.tabs.select(graph_tab) {
            h.app.show_active(&mut h.root);
            settle(&mut h, &mut renderer);
        }
    }
    let _ = h.app.close_active_tab(&mut h.root);
    settle(&mut h, &mut renderer);
    line("закрыта вкладка графа");
    if std::env::var_os("AQ_REPORT_GRAPH").is_some() {
        eprintln!("{}", crate::memory_probe::report());
    }
    while h.app.tabs.tabs().len() > 1 {
        let _ = h.app.close_active_tab(&mut h.root);
        settle(&mut h, &mut renderer);
    }
    let _ = h.app.close_active_tab(&mut h.root);
    settle(&mut h, &mut renderer);
    line("все вкладки закрыты");
    eprintln!("{}", crate::memory_probe::report());
}
