use super::*;

#[test]
#[ignore]
fn scroll_damage_report() {
    let spec = size();
    let mut h = setup(spec, &plain_body(120));
    let ids = h.app.note_ids.as_ref().unwrap();
    let (viewport, column) = (rect_of(&h, ids.scroll.id(), spec.scale), rect_of(&h, ids.body.id(), spec.scale));
    eprintln!("окно {}x{}@{}; область прокрутки {viewport:?}; колонка текста x {:.0}..{:.0}", spec.width, spec.height, spec.scale, column.x0, column.x1);
    let mut frames = Frames::new(&mut h, spec, false);
    let reports = scroll_frames(&mut h, &mut frames, 30);
    for (i, r) in reports.iter().enumerate().take(4) {
        r.print(&format!("прокрутка {i}"));
    }
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
