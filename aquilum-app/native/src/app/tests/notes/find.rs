use super::*;

#[test]
fn page_search_counts_and_steps_through_matches() {
    let mut h = Harness::new();
    let note = h.write_note("Коты.md", "кот и Кот\n\nещё КОТ");
    h.open(note);
    let _ = h.root.redraw();
    h.app.open_find(&mut h.root);
    let _ = h.root.redraw();
    let counter = |h: &Harness| h.app.find.as_ref().unwrap().counter.get(&h.root).unwrap().inner().text().to_owned();
    h.type_text("кот");
    assert_eq!(counter(&h), "1/3");
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::empty()));
    assert_eq!(counter(&h), "2/3");
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::SHIFT));
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::SHIFT));
    assert_eq!(counter(&h), "3/3");
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Escape), Modifiers::empty()));
    assert!(h.app.find.is_none());
}

#[test]
fn page_search_marks_table_cells_and_counter_fits() {
    let mut h = Harness::new();
    let note = h.write_note("таблица.md", "Слово в тексте.\n\n| Колонка | Ещё |\n| --- | --- |\n| слово тут | нет |\n| и снова слово | да |\n");
    h.open(note);
    let _ = h.root.redraw();
    h.app.open_find(&mut h.root);
    h.type_text("слово");
    h.settle();
    let counter = h.app.find.as_ref().unwrap().counter.id();
    let width = h.root.get_widget(counter).unwrap().ctx().bounding_box().width();
    assert!(width > 15.0, "счётчик сжат до многоточия: {width}");
    let r = h.frame(W as u16, H as u16, 1.0);
    let pink = r.frame_pixels().iter().filter(|&&p| {
        let (red, green, blue) = (((p >> 16) & 0xff) as i32, ((p >> 8) & 0xff) as i32, (p & 0xff) as i32);
        red > green + 30 && red > blue
    }).count();
    dump("find-table.png", &r);
    assert!(pink > 200, "совпадения в таблице не подсвечены: {pink}");
}
