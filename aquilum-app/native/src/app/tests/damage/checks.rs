use super::*;

fn fixed_frames(h: &mut Harness) -> Frames {
    Frames::new(h, FIXED, true)
}

#[test]
fn editor_scroll_shifts_only_the_text_column() {
    let mut h = setup(FIXED, &plain_body(80));
    let column = rect_of(&h, h.app.note_ids.as_ref().unwrap().body.id(), FIXED.scale);
    let mut frames = fixed_frames(&mut h);
    let reports = scroll_frames(&mut h, &mut frames, 9);
    for (i, r) in reports.iter().enumerate() {
        assert_eq!(r.differ, 0, "шаг {i}: прокрутка расходится с полной отрисовкой в {} пикселях", r.differ);
        assert!(r.shown.iter().all(|a| f64::from(a.w) <= column.width() + 56.0), "шаг {i}: в окно выводится шире колонки текста: {:?}", r.shown);
    }
    let moved: Vec<_> = reports.iter().flat_map(|r| r.moved.iter()).collect();
    assert!(moved.len() >= 6, "прокрутка не сдвигала пиксели: {moved:?}");
    for m in moved {
        assert!(f64::from(m.0) >= column.x0 - 16.0 && f64::from(m.0 + m.2) <= column.x1 + 40.0, "сдвиг {m:?} шире колонки текста {column:?}");
    }
}

#[test]
fn sorting_repaints_only_the_tree() {
    let mut h = setup(FIXED, &plain_body(10));
    let vault = h.vault.path().to_path_buf();
    let notes: Vec<_> = (1..=12).map(|i| vault.join(format!("Заметка {i:02}.md"))).collect();
    for note in &notes {
        std::fs::write(note, "").unwrap();
    }
    h.app.tree.paths_changed(&mut h.root, &notes);
    let tree = rect_of(&h, h.app.tree.scroll_id().unwrap(), FIXED.scale);
    let mut frames = fixed_frames(&mut h);
    h.app.tree.set_sort(&mut h.root, crate::ui::file_tree::Sort::NameDesc);
    let report = frames.draw(&mut h);
    assert_eq!(report.differ, 0, "сортировка расходится с полной отрисовкой в {} пикселях", report.differ);
    assert!(report.raster > 0, "сортировка ничего не перерисовала");
    for &(x, y, w, hgt) in &report.rects {
        let (x0, y0, x1, y1) = (f64::from(x), f64::from(y), f64::from(x + w), f64::from(y + hgt));
        assert!(x0 >= tree.x0.floor() && x1 <= tree.x1.ceil() && y0 >= tree.y0.floor() && y1 <= tree.y1.ceil(), "перерисовано вне дерева {tree:?}: {:?}", report.rects);
    }
    assert!(report.moved.is_empty(), "сортировка сдвигала пиксели: {:?}", report.moved);
}

#[test]
fn table_row_resize_repaints_only_the_table_and_below() {
    let mut h = setup(FIXED, &table_body(2));
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = rect_of(&h, ids.body.id(), FIXED.scale);
    let offset = ids.body.get(&h.root).unwrap().inner().text().find("| Имя").unwrap();
    let top = body.y0 + ids.body.get(&h.root).unwrap().inner().top_of(offset).unwrap() * FIXED.scale;
    let mut frames = fixed_frames(&mut h);
    let (reports, _) = drag_border(&mut h, &mut frames, 4);
    assert_ne!(h.body_text(), table_body(2), "строка таблицы не изменила высоту");
    for (i, r) in reports.iter().enumerate() {
        assert_eq!(r.differ, 0, "шаг {i}: изменение высоты строки расходится с полной отрисовкой в {} пикселях", r.differ);
        assert!(r.raster > 0, "шаг {i}: таблица не перерисована");
        assert!(!r.above(top), "шаг {i}: перерисовано то, что выше таблицы (верх таблицы {top}): {:?}", r.rects);
    }
}

#[test]
fn caret_repaints_only_what_the_reveal_changes() {
    let list: String = (1..=12).map(|i| format!("{i}. пункт списка номер {i}, длинная строка текста, которая занимает почти всю ширину колонки\n")).collect();
    let mut h = setup(FIXED, &format!("Обычная строка текста без разметки, достаточно длинная.\n\n{list}"));
    let column = rect_of(&h, h.app.note_ids.as_ref().unwrap().body.id(), FIXED.scale);
    let mut frames = fixed_frames(&mut h);
    let point = line_point(&h, 0, FIXED.scale);
    click_at(&mut h, point, FIXED.scale);
    let plain = frames.draw(&mut h);
    assert_eq!(plain.differ, 0, "каретка в обычной строке: расхождение с полной отрисовкой в {} пикселях", plain.differ);
    assert!(plain.raster < 3000, "каретка в обычной строке перерисовала больше самой каретки: {:?}", plain.rects);
    assert_eq!(plain.shown_area(), plain.raster, "в окно выводится больше перерисованного: {:?}", plain.shown);
    let ids = h.app.note_ids.as_ref().unwrap();
    let editor = ids.body.get(&h.root).unwrap();
    let top = editor.inner().top_of(editor.inner().text().find("3. пункт").unwrap()).unwrap();
    click_at(&mut h, (column.x0 + 2.0 * FIXED.scale, column.y0 + (top + 10.0) * FIXED.scale), FIXED.scale);
    let marker = frames.draw(&mut h);
    assert_eq!(marker.differ, 0, "каретка у номера пункта: расхождение с полной отрисовкой в {} пикселях", marker.differ);
    assert!(marker.raster > 0, "номер пункта не перерисован");
    let widest = marker.rects.iter().map(|r| f64::from(r.2)).fold(0.0, f64::max);
    assert!(widest < column.width() / 2.0, "каретка у номера пункта перерисовала всю строку: {:?}", marker.rects);
}

#[test]
fn settings_editor_scroll_shifts_pixels() {
    let mut h = sized(FIXED, None);
    h.app.open_settings(&mut h.root);
    h.settle();
    let nav = h.root.get_widget(h.nav(Section::Editor)).unwrap().ctx().bounding_box();
    h.click(nav.center().x, nav.center().y);
    h.settle();
    for _ in 0..10 {
        h.anim(std::time::Duration::from_millis(16));
        h.settle();
    }
    let mut frames = fixed_frames(&mut h);
    let content_id = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.content.id(),
        _ => panic!("настройки не открылись"),
    };
    let content_rect = rect_of(&h, content_id, FIXED.scale);
    h.scroll((content_rect.center().x, content_rect.center().y), FIXED.scale, (0.0, -30.0));
    h.anim(std::time::Duration::from_millis(16));
    let report = frames.draw(&mut h);
    assert!(!report.moved.is_empty(), "прокрутка настроек не сдвигает пиксели!");
}

#[test]
fn phone_settings_editor_scroll_shifts_pixels() {
    const PHONE: Spec = Spec { scale: 2.625, width: 1080, height: 1600 };
    let mut h = sized(PHONE, None);
    h.app.open_settings(&mut h.root);
    h.settle();
    h.open_section(Section::Editor);
    h.settle();
    for _ in 0..15 {
        h.anim(std::time::Duration::from_millis(16));
        h.settle();
    }
    let mut frames = Frames::new(&mut h, PHONE, true);
    let content_id = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.content.id(),
        _ => panic!("настройки не открылись"),
    };
    let content_rect = rect_of(&h, content_id, PHONE.scale);
    h.scroll((content_rect.center().x, content_rect.center().y), PHONE.scale, (0.0, -30.0));
    h.anim(std::time::Duration::from_millis(16));
    let report = frames.draw(&mut h);
    assert!(!report.moved.is_empty(), "прокрутка настроек на телефоне не сдвигает пиксели!");
}

