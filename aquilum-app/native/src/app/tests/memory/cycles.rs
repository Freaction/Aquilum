use super::*;

fn private(step: &str) {
    let report = crate::memory_probe::report();
    let first = report.lines().next().unwrap_or_default().split(" · пик").next().unwrap_or_default();
    let heap = report.lines().find(|line| line.contains("основная куча")).unwrap_or_default().trim();
    let threads = crate::memory_probe::threads().len();
    eprintln!("{step:<28} куча {:>6.1} МБ · потоков {threads} · {first} · кучи Windows {heap}", mb(crate::memory_probe::heap()));
}

#[test]
#[ignore]
fn graph_and_empty_tab_cycles() {
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").expect("AQ_VAULT"));
    let cycles: usize = std::env::var("AQ_CYCLES").ok().and_then(|n| n.parse().ok()).unwrap_or(10);
    crate::memory_probe::trace(std::env::var_os("AQ_PROBE_TRACE").is_some());
    let mut h = Harness::with(Some(&vault), 1.75);
    let mut renderer = crate::render::Renderer::new(2);
    assert!(super::super::perf::wait_index(&h.app.workspace.core, &vault.to_string_lossy()), "индекс не готов");
    let loaded = |h: &mut Harness, renderer: &mut crate::render::Renderer| {
        for _ in 0..400 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if h.app.graph_loaded().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        settle(h, renderer);
    };
    h.app.open_graph(&mut h.root);
    loaded(&mut h, &mut renderer);
    let graph = h.app.tabs.active_id();
    let _ = h.app.new_tab(&mut h.root);
    let empty = h.app.tabs.active_id();
    settle(&mut h, &mut renderer);
    private("старт: граф и пустая вкладка");
    for cycle in 1..=cycles {
        assert!(h.app.tabs.select(graph));
        h.app.show_active(&mut h.root);
        loaded(&mut h, &mut renderer);
        let shown = mb(crate::memory_probe::heap());
        assert!(h.app.tabs.select(empty));
        h.app.show_active(&mut h.root);
        settle(&mut h, &mut renderer);
        private(&format!("цикл {cycle} (на графе {shown:.1} МБ)"));
    }
    eprintln!("{}", crate::memory_probe::report());
}
