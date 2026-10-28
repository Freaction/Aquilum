use super::*;

fn cell(h: &Harness, row: usize, col: usize, scale: f64) -> (f64, f64) {
    let (x, border) = table_border(h, scale);
    let (x0, y0) = (x / scale - 120.0, border / scale - 78.0);
    ((x0 + 40.0 + col as f64 * 160.0) * scale, (y0 + 18.0 + row as f64 * 40.0) * scale)
}

fn drag(h: &mut Harness, from: (f64, f64), to: (f64, f64), count: u8, scale: f64) {
    let mut state = pointer(h, from.0, from.1, scale, true);
    state.count = count;
    h.root.handle_pointer_event(PointerEvent::Down(primary(state)));
    pointer(h, to.0, to.1, scale, true);
    release(h, to.0, to.1, scale);
    h.deliver();
}

#[test]
fn a_lost_release_does_not_keep_selecting_cells() {
    let spec = FIXED;
    let mut h = setup(spec, &table_body(2));
    let (a, b) = (cell(&h, 1, 0, spec.scale), cell(&h, 2, 0, spec.scale));
    click_at(&mut h, a, spec.scale);
    press(&mut h, a.0, a.1, spec.scale);
    pointer(&mut h, b.0, b.1, spec.scale, false);
    h.deliver();
    let editor = h.app.note_ids.as_ref().unwrap().body.get(&h.root).unwrap();
    assert_eq!(editor.inner().table_marks(), Some((false, 0)), "указатель без нажатой кнопки продолжил выделять ячейки");
}

#[test]
fn table_selection_leaves_no_stale_pixels_when_cleared_or_scrolled() {
    let spec = FIXED;
    let mut h = setup(spec, &table_body(2));
    let mut frames = Frames::new(&mut h, spec, true);
    let check = |h: &mut Harness, frames: &mut Frames, label: &str, marks: Option<(bool, usize)>| {
        let report = frames.draw(h);
        assert_eq!(report.differ, 0, "{label}: частичный кадр расходится с полным в {} пикселях", report.differ);
        let editor = h.app.note_ids.as_ref().unwrap().body.get(&h.root).unwrap();
        assert_eq!(editor.inner().table_marks().map(|(range, text)| (range, text > 0)), marks.map(|(range, text)| (range, text > 0)), "{label}: не то выделение");
    };
    let (a, b, c) = (cell(&h, 1, 0, spec.scale), cell(&h, 2, 0, spec.scale), cell(&h, 2, 1, spec.scale));
    click_at(&mut h, a, spec.scale);
    check(&mut h, &mut frames, "щелчок в ячейку", Some((false, 0)));
    drag(&mut h, a, b, 1, spec.scale);
    check(&mut h, &mut frames, "протянули выделение ячеек", Some((true, 0)));
    click_at(&mut h, c, spec.scale);
    check(&mut h, &mut frames, "щелчок снял выделение ячеек", Some((false, 0)));
    drag(&mut h, c, c, 2, spec.scale);
    check(&mut h, &mut frames, "двойной щелчок выделил слово", Some((false, 1)));
    for step in 0..6 {
        h.scroll((f64::from(spec.width) * 0.55, f64::from(spec.height) * 0.5), spec.scale, (0.0, if step < 3 { -40.0 } else { 40.0 }));
        h.anim(std::time::Duration::from_millis(16));
        check(&mut h, &mut frames, &format!("прокрутка {step}"), Some((false, 1)));
    }
    click_at(&mut h, c, spec.scale);
    check(&mut h, &mut frames, "щелчок снял выделение слова", Some((false, 0)));
}

const FINGER: masonry::core::PointerInfo = masonry::core::PointerInfo { pointer_id: Some(masonry::core::PointerId::PRIMARY), persistent_device_id: None, pointer_type: masonry::core::PointerType::Touch };

fn finger(h: &mut Harness, x: f64, y: f64, pressed: bool) {
    let mut current = pointer_at(x, y, 1.0);
    if pressed {
        current.buttons.insert(PointerButton::Primary);
    }
    h.root.handle_pointer_event(PointerEvent::Move(masonry::core::PointerUpdate { pointer: FINGER, current, coalesced: vec![], predicted: vec![] }));
}

fn thumb(h: &mut Harness, marker: &str) -> (f64, f64) {
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = h.root.get_widget(ids.body.id()).unwrap().ctx().bounding_box();
    let editor = ids.body.get(&h.root).unwrap();
    let top = body.y0 + editor.inner().top_of(editor.inner().text().find(marker).unwrap()).unwrap();
    (0..500).flat_map(|dy| (0..40).map(move |i| (body.x0 + f64::from(i) * 6.0, top + f64::from(dy))))
        .find(|&(x, y)| {
            h.pointer_move(x, y);
            h.root.cursor_icon() == masonry::core::CursorIcon::Grab
        })
        .expect("ползунок таблицы не найден")
}

fn drag_thumb_with_a_finger(compose: bool) -> (String, Option<(bool, usize)>, String, bool) {
    let mut h = Harness::new();
    let wide = format!("| {} |\n| {} |\n| {} |\n", (1..=30).map(|i| format!("Колонка_{i}_с_длинным_названием")).collect::<Vec<_>>().join(" | "), vec!["---"; 30].join(" | "), (1..=30).map(|i| format!("значение_{i}")).collect::<Vec<_>>().join(" | "));
    let note = h.write_note("Таблица.md", &format!("Абзац над таблицей.\n\n{wide}\nАбзац под таблицей с текстом.\n"));
    h.open(note);
    let _ = h.root.redraw();
    if compose {
        let ids = h.app.note_ids.as_ref().unwrap();
        let body = h.root.get_widget(ids.body.id()).unwrap().ctx().bounding_box();
        let editor = ids.body.get(&h.root).unwrap();
        let line = body.y0 + editor.inner().top_of(editor.inner().text().find("Абзац над").unwrap()).unwrap() + 10.0;
        h.click(body.x0 + 60.0, line);
        h.root.handle_text_event(masonry::core::TextEvent::Ime(masonry::core::Ime::Preedit("слово".into(), Some((10, 10)))));
        let id = h.app.note_ids.as_ref().unwrap().body.id();
        h.root.edit_widget(id, |mut widget| crate::ui::editor::TextEditor::commit_compose(&mut widget.downcast()));
    }
    let (x, y) = thumb(&mut h, "| Колонка_1");
    finger(&mut h, x, y, false);
    let mut state = pointer_at(x, y, 1.0);
    state.buttons.insert(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Down(masonry::core::PointerButtonEvent { pointer: FINGER, button: Some(PointerButton::Primary), state }));
    for step in 1..=10 {
        finger(&mut h, x + f64::from(step) * 15.0, y, true);
    }
    h.root.handle_pointer_event(PointerEvent::Up(masonry::core::PointerButtonEvent { pointer: FINGER, button: Some(PointerButton::Primary), state: pointer_at(x + 150.0, y, 1.0) }));
    h.deliver();
    let editor = h.app.note_ids.as_ref().unwrap().body.get(&h.root).unwrap();
    let focused = h.root.focused_widget() == Some(editor.ctx().widget_id());
    (editor.inner().selected(), editor.inner().table_marks(), editor.inner().text().to_owned(), focused)
}

#[test]
fn a_finger_on_the_table_scrollbar_scrolls_the_table_and_selects_nothing() {
    for compose in [false, true] {
        let (selected, marks, text, focused) = drag_thumb_with_a_finger(compose);
        assert_eq!(focused, compose, "набор слова {compose}: полоса таблицы сменила фокус и подняла бы клавиатуру");
        assert_eq!(text.contains("слово"), compose, "касание не зафиксировало набираемое слово");
        assert!(selected.is_empty(), "набор слова {compose}: протяжка ползунка выделила текст {selected:?}");
        assert!(marks.is_none_or(|(range, text)| !range && text == 0), "набор слова {compose}: протяжка ползунка выделила ячейки {marks:?}");
    }
}
