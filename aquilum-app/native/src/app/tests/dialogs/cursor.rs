use super::*;
use crate::ui::settings::Binding;
use crate::ui::settings::schema::Action;
use masonry::core::CursorIcon;

#[test]
fn a_secondary_button_shows_a_hand() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    h.open_section(Section::System);
    let author = h.binding("кнопка автора", |b| matches!(b, Binding::Button(Action::OpenUrl(_))));
    let center = h.root.get_widget(author).unwrap().ctx().bounding_box().center();

    h.pointer_move(center.x, center.y);

    assert_eq!(h.root.cursor_icon(), CursorIcon::Pointer);
}

fn grab_below(h: &mut Harness, marker: &str, depth: f64) -> bool {
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = h.root.get_widget(ids.body.id()).unwrap().ctx().bounding_box();
    let editor = ids.body.get(&h.root).unwrap();
    let top = body.y0 + editor.inner().top_of(editor.inner().text().find(marker).unwrap()).unwrap();
    (0..depth as usize).any(|dy| {
        (0..40).map(|i| f64::from(i) * 6.0).any(|dx| {
            h.pointer_move(body.x0 + dx, top + dy as f64);
            h.root.cursor_icon() == CursorIcon::Grab
        })
    })
}

#[test]
fn a_table_scrollbar_shows_a_hand_so_a_finger_takes_the_thumb_at_once() {
    let mut h = Harness::new();
    let wide = format!("| {} |\n| {} |\n| {} |\n", (1..=30).map(|i| format!("Колонка_с_длинным_названием_{i}")).collect::<Vec<_>>().join(" | "), vec!["---"; 30].join(" | "), (1..=30).map(|i| format!("значение_{i}")).collect::<Vec<_>>().join(" | "));
    let note = h.write_note("Полосы.md", &format!("{wide}\nТекст."));
    h.open(note);
    assert!(grab_below(&mut h, "| Колонка_с_длинным_названием_1", 500.0), "у ползунка широкой таблицы нет руки — палец его не возьмёт");
}
