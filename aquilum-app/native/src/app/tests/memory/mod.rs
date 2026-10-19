use super::*;
use crate::stats::mb;

mod cycles;

fn settle(h: &mut Harness, renderer: &mut crate::render::Renderer) -> u128 {
    let started = std::time::Instant::now();
    for _ in 0..300 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if !h.app.embeds.as_ref().is_some_and(super::super::embeds::Embeds::busy) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let ready = started.elapsed().as_millis();
    for _ in 0..25 {
        std::thread::sleep(std::time::Duration::from_millis(80));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
    h.settle();
    h.deliver();
    if crate::render::take_trim() {
        renderer.trim();
    }
    let (layers, _) = h.root.redraw();
    let size = h.root.size();
    let _ = renderer.render(&layers, size.width as u16, size.height as u16, 1.75, theme::background());
    ready
}

fn exercise(h: &mut Harness, renderer: &mut crate::render::Renderer) -> String {
    let Some(body) = h.app.note_ids.as_ref().map(|ids| ids.body) else { return String::new() };
    let timed = |h: &mut Harness, work: &mut dyn FnMut(&mut Harness)| {
        let started = std::time::Instant::now();
        work(h);
        let _ = h.root.redraw();
        started.elapsed().as_millis()
    };
    h.root.focus_on(Some(body.id()));
    let end = body.get(&h.root).map_or(0, |b| b.inner().len());
    body.edit(&mut h.root, |mut b| crate::ui::editor::TextEditor::select_byte_range(&mut b, end, end));
    let typed = timed(h, &mut |h| h.type_text(" проверка"));
    let erased = timed(h, &mut |h| (0..9).for_each(|_| drop(h.key(Key::Named(NamedKey::Backspace), Modifiers::empty()))));
    let found = timed(h, &mut |h| {
        h.app.open_find(&mut h.root);
        h.type_text("тест");
    });
    h.escape();
    let searched = timed(h, &mut |h| {
        h.app.open_search(&mut h.root);
        h.type_text("граф");
    });
    h.escape();
    settle(h, renderer);
    format!("[набор {typed} стирание {erased} поиск {found}/{searched}]")
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
    if let Ok(mode) = std::env::var("AQ_PANEL") {
        h.app.interface.set_panel_mode(&mode);
    }
    let mut renderer = crate::render::Renderer::new(2);
    assert!(super::perf::wait_index(&h.app.workspace.core, &vault.to_string_lossy()), "индекс не готов");
    settle(&mut h, &mut renderer);
    line("старт");
    for note in &notes {
        let started = std::time::Instant::now();
        h.app.open_in(&mut h.root, note.clone(), true);
        h.parked();
        let _ = h.root.redraw();
        let shown = started.elapsed().as_millis();
        let ready = settle(&mut h, &mut renderer);
        let name = note.file_stem().unwrap_or_default().to_string_lossy().chars().take(20).collect::<String>();
        line(&format!("открыта {name}, кадр {shown}+{ready} мс"));
        if let Some(out) = &zones {
            aq_trace::write(std::path::Path::new(out)).unwrap();
        }
        if std::env::var_os("AQ_REPORT_EACH").is_some() {
            eprintln!("{}", crate::memory_probe::report());
        }
    }
    let panels: Vec<String> = std::env::var("AQ_PANELS").map_or(Vec::new(), |p| p.split('|').map(str::to_owned).collect());
    let edit = std::env::var_os("AQ_EDIT").is_some();
    for round in 0..5 {
        if let Some(panel) = (!panels.is_empty()).then(|| &panels[round % panels.len()]) {
            let (mode, method) = panel.split_once(':').unwrap_or((panel, ""));
            h.app.interface.set_panel_mode(mode);
            if !method.is_empty() {
                h.app.interface.set_analysis_method(method);
            }
            line(&format!("панель {panel}"));
        }
        let ids: Vec<_> = h.app.tabs.tabs().iter().map(|t| t.id).collect();
        let mut spent = Vec::new();
        for id in ids {
            let started = std::time::Instant::now();
            if h.app.tabs.select(id) {
                h.app.show_active(&mut h.root);
                h.parked();
            }
            let (layers, _) = h.root.redraw();
            let size = h.root.size();
            let _ = renderer.render(&layers, size.width as u16, size.height as u16, 1.75, theme::background());
            let frame = started.elapsed().as_millis();
            let ready = settle(&mut h, &mut renderer);
            let work = if edit { exercise(&mut h, &mut renderer) } else { String::new() };
            spent.push(format!("{frame}+{ready}{work}"));
        }
        if let Some(idle) = std::env::var("AQ_IDLE").ok().and_then(|ms| ms.parse().ok()) {
            std::thread::sleep(std::time::Duration::from_millis(idle));
            settle(&mut h, &mut renderer);
        }
        line(&format!("круг вкладок {}: кадр+вставки, мс {}", round + 1, spent.join(" ")));
        if std::env::var_os("AQ_REPORT_ROUNDS").is_some() {
            eprintln!("{}", crate::memory_probe::report());
        }
        if let Some(out) = &zones {
            aq_trace::write(std::path::Path::new(out)).unwrap();
        }
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
