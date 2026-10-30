use super::*;
use crate::ui::dialog::DialogAnswer;
use crate::ui::search::CardKind;
use crate::ui::tokens::size;
use masonry::core::WidgetId;
use masonry::kurbo::Rect;

fn rect(h: &Harness, id: WidgetId) -> Rect {
    h.root.get_widget(id).unwrap().ctx().bounding_box()
}

fn fill_templates(h: &Harness, names: &[&str]) {
    let dir = h.app.templates_dir().expect("папка шаблонов");
    for name in names {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("Заготовка {name}")).unwrap();
    }
}

fn state_cards(h: &Harness) -> Vec<WidgetId> {
    match &h.app.popup {
        Some(Popup::Templates(state)) => state.cards.clone(),
        _ => panic!("окно шаблонов не открыто"),
    }
}

fn field_top(h: &Harness) -> f64 {
    match &h.app.popup {
        Some(Popup::Templates(state)) => rect(h, state.field.id()).y0,
        _ => panic!("окно шаблонов не открыто"),
    }
}

#[test]
fn the_templates_window_is_a_compact_list_that_stays_where_the_search_window_starts() {
    let mut h = Harness::new();
    fill_templates(&h, &["Книга.md", "Обычная заметка.md", "проекты/План.md"]);
    h.app.open_search(&mut h.root);
    h.settle();
    let search_top = match &h.app.popup {
        Some(Popup::Search(state)) => rect(&h, state.field_id()).y0,
        _ => panic!("поиск не открыт"),
    };
    h.escape();
    h.app.open_templates(&mut h.root);
    h.settle();
    assert_eq!(field_top(&h), search_top, "окно шаблонов начинается не там, где окно поиска");
    let cards = state_cards(&h);
    assert_eq!(cards.len(), 3);
    for &card in &cards {
        assert_eq!(rect(&h, card).height(), CardKind::Template.height(), "строка шаблона не сжата до {}", size::SIZE_44);
    }
    let last = rect(&h, *cards.last().unwrap());
    assert!(last.y1 < f64::from(H) * 0.5, "окно с тремя шаблонами растянуто на всю высоту: {last:?}");
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("templates.png"), h.frame(W as u16, H as u16, 1.0).png()).unwrap();
    }
}

#[test]
fn typing_filters_the_list_and_a_click_outside_closes_the_window() {
    let mut h = Harness::new();
    fill_templates(&h, &["Книга.md", "Обычная заметка.md", "проекты/План.md"]);
    h.app.open_templates(&mut h.root);
    h.settle();
    h.type_text("пла");
    h.settle();
    let cards = state_cards(&h);
    assert_eq!(cards.len(), 1, "фильтр по «пла» оставил не один шаблон");
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("templates-filter.png"), h.frame(W as u16, H as u16, 1.0).png()).unwrap();
    }
    h.type_text("zzz");
    h.settle();
    assert!(state_cards(&h).is_empty(), "список не опустел при ненайденном запросе");
    let layer = match &h.app.popup {
        Some(Popup::Templates(state)) => state.layer,
        _ => unreachable!(),
    };
    let cancel = masonry::core::ErasedAction::from(Box::new(DialogAnswer::Cancel));
    h.app.on_action(&mut h.root, layer, cancel);
    assert!(h.app.popup.is_none(), "щелчок вне окна не закрыл шаблоны");
}

#[test]
fn without_templates_the_window_offers_to_create_the_starter_folder() {
    let mut h = Harness::new();
    h.app.open_templates(&mut h.root);
    h.settle();
    let button = match &h.app.popup {
        Some(Popup::Templates(state)) => state.create_button.expect("кнопка создания папки с шаблонами"),
        _ => panic!("окно шаблонов не открыто"),
    };
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("templates-empty.png"), h.frame(W as u16, H as u16, 1.0).png()).unwrap();
    }
    let center = rect(&h, button).center();
    h.click(center.x, center.y);
    h.settle();
    assert!(!state_cards(&h).is_empty(), "после создания папки список шаблонов пуст");
}

#[test]
fn enter_inserts_the_chosen_template_into_the_open_note() {
    let mut h = Harness::new();
    fill_templates(&h, &["Книга.md", "Обычная заметка.md"]);
    let note = h.write_note("заметка.md", "");
    h.open(note);
    h.settle();
    h.app.open_templates(&mut h.root);
    h.settle();
    h.type_text("книга");
    let outcome = h.app.templates_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::empty());
    assert!(outcome.is_some() && h.app.popup.is_none(), "Enter не выбрал шаблон");
    h.deliver();
    assert!(h.app.text.to_string().contains("Заготовка Книга.md"), "{}", h.app.text.to_string());
}
