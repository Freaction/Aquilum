use super::*;

#[test]
#[ignore]
fn scroll_damage_report() {
    let spec = size();
    let mut h = match std::env::var("AQ_VAULT") {
        Ok(vault) => {
            let vault = std::path::PathBuf::from(vault);
            let mut h = sized(spec, Some(&vault));
            h.open(vault.join(std::env::var("AQ_NOTE").expect("AQ_NOTE")));
            h
        }
        Err(_) => setup(spec, &plain_body(120)),
    };
    let ids = h.app.note_ids.as_ref().unwrap();
    let (viewport, column) = (rect_of(&h, ids.scroll.id(), spec.scale), rect_of(&h, ids.body.id(), spec.scale));
    eprintln!("окно {}x{}@{}; область прокрутки {viewport:?}; колонка текста x {:.0}..{:.0}", spec.width, spec.height, spec.scale, column.x0, column.x1);
    let mut frames = Frames::new(&mut h, spec, std::env::var_os("AQ_CHECK").is_some());
    let screen = u64::from(frames.width) * u64::from(frames.height);
    let dy: f64 = std::env::var("AQ_DY").ok().and_then(|v| v.parse().ok()).unwrap_or(-25.0);
    let (mut last, mut wide) = (None, 0);
    let mut reports = Vec::new();
    for step in 0..200 {
        h.scroll((f64::from(frames.width) * 0.55, f64::from(frames.height) * 0.5), spec.scale, (0.0, dy));
        h.anim(std::time::Duration::from_millis(8));
        h.app.on_core_events(&mut h.root);
        let report = frames.draw(&mut h);
        let ids = h.app.note_ids.as_ref().unwrap();
        let column = rect_of(&h, ids.body.id(), spec.scale);
        let edges = (column.x0.round(), column.x1.round(), column.height().round());
        let big = report.raster * 4 > screen;
        if big || last.is_some_and(|l: (f64, f64, f64)| (l.0, l.1) != (edges.0, edges.1)) {
            wide += usize::from(big);
            report.print(&format!("шаг {step}: колонка x {:.0}..{:.0} высота {:.0}, расхождений {}", edges.0, edges.1, edges.2, report.differ));
        }
        last = Some(edges);
        reports.push(report);
    }
    eprintln!("кадров с растром больше четверти экрана: {wide} из 200");
    summary("прокрутка", &reports);
}

#[test]
#[ignore]
fn caret_damage_report() {
    let spec = size();
    let mut h = setup(spec, &format!("Строка с **жирным** и `кодом`.\n\n{}", plain_body(40)));
    let mut frames = Frames::new(&mut h, spec, false);
    for (label, paragraph) in [("каретка: первый щелчок в обычную строку", 3), ("каретка: другая обычная строка", 5), ("каретка: строка с разметкой", 0), ("каретка: обратно в обычную", 4)] {
        let point = line_point(&h, paragraph, spec.scale);
        click_at(&mut h, point, spec.scale);
        frames.draw(&mut h).print(label);
    }
    for i in 0..2 {
        h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
        frames.draw(&mut h).print(&format!("каретка: стрелка вниз {i}"));
    }
    h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
    frames.draw(&mut h).print("каретка: стрелка вправо");
}

#[test]
#[ignore]
fn table_damage_report() {
    let spec = size();
    let mut h = setup(spec, &table_body(6));
    let mut frames = Frames::new(&mut h, spec, false);
    let (reports, border) = drag_border(&mut h, &mut frames, 8);
    eprintln!("граница строки таблицы на y = {border:.0}");
    for (i, r) in reports.iter().enumerate().take(3) {
        r.print(&format!("таблица {i}"));
    }
    summary("таблица", &reports);
}

#[test]
#[ignore]
fn full_frame_report() {
    let spec = size();
    let body = std::env::var("AQ_NOTE").map_or_else(|_| plain_body(60), |source| std::fs::read_to_string(source).unwrap());
    let mut h = setup(spec, &body);
    let (layers, _) = h.root.redraw();
    let mut renderer = Renderer::new(raster_threads());
    let mut times: Vec<f64> = (0..12)
        .map(|_| {
            let t = std::time::Instant::now();
            renderer.draw_frame(&layers, spec.width as u16, spec.height as u16, spec.scale, theme::background(), true).unwrap();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(f64::total_cmp);
    eprintln!("полный кадр {}x{}@{}: медиана {:.2} мс, лучший {:.2} мс", spec.width, spec.height, spec.scale, times[6], times[0]);
}

#[test]
#[ignore]
fn caret_note_report() {
    let spec = size();
    let source = std::env::var("AQ_NOTE").unwrap();
    let mut h = setup(spec, &std::fs::read_to_string(source).unwrap());
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = rect_of(&h, ids.body.id(), spec.scale);
    eprintln!("колонка текста {body:?}");
    let mut frames = Frames::new(&mut h, spec, true);
    let dx: f64 = std::env::var("AQ_X").ok().and_then(|v| v.parse().ok()).unwrap_or(300.0);
    for step in 0..12 {
        let (x, y) = (body.x0 + dx * spec.scale, body.y0 + (40.0 + 61.0 * f64::from(step)) * spec.scale);
        if y > f64::from(spec.height) - 20.0 {
            break;
        }
        click_at(&mut h, (x, y), spec.scale);
        let report = frames.draw(&mut h);
        report.print(&format!("щелчок {step} в ({x:.0}, {y:.0}), расхождений {}", report.differ));
    }
}
