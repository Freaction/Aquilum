use super::*;
use crate::ui::editor::TextAction;
use crate::ui::tokens::size;
use masonry::core::WidgetId;
use masonry::kurbo::Rect;

fn vault() -> tempfile::TempDir {
    let vault = crate::test_dir::tempdir();
    std::fs::create_dir_all(vault.path().join("Проекты")).unwrap();
    for name in ["Кофе.md", "Чай.md", "Проекты/Кофемолка.md", "Проекты/Заметка.md"] {
        std::fs::write(vault.path().join(name), "").unwrap();
    }
    vault
}

pub(super) fn start() -> (tempfile::TempDir, Harness) {
    let vault = vault();
    let mut h = Harness::with(Some(vault.path()), 1.0);
    h.deliver();
    (vault, h)
}

pub(super) fn bounds(h: &Harness, id: WidgetId) -> Rect {
    h.root.get_widget(id).unwrap().ctx().bounding_box()
}

pub(super) fn field(h: &Harness) -> WidgetId {
    h.app.tree.filter_field().unwrap()
}

pub(super) fn names(h: &Harness) -> Vec<String> {
    h.app.tree.names()
}

#[test]
fn the_search_field_fills_the_sidebar_at_size_xs() {
    let h = Harness::new();
    let tree = bounds(&h, h.app.tree.scroll_id().unwrap());
    let field = bounds(&h, field(&h));
    let inset = size::SPACE_8 + size::INPUT_XS_PADDING_X;
    assert_eq!(field.x0, tree.x0 + inset + size::INPUT_ICON_SIZE + size::INPUT_GAP, "поле не после иконки поиска");
    assert_eq!(field.x1, tree.x1 - inset, "поле не растянуто на ширину панели");
    assert_eq!(field.height(), size::INPUT_CONTENT_HEIGHT);
    assert_eq!(field.height() + size::INPUT_XS_PADDING_Y * 2.0, size::SIZE_28, "высота поля не 28");
}

#[test]
fn the_search_box_has_no_fill_but_keeps_its_ring() {
    let mut h = Harness::new();
    let field = bounds(&h, field(&h));
    let tree = bounds(&h, h.app.tree.scroll_id().unwrap());
    let frame = h.frame(W as u16, H as u16, 1.0);
    let pixels = frame.frame_pixels();
    let at = |x: f64, y: f64| pixels[y as usize * W as usize + x as usize];
    let y = field.center().y;
    let (left, background) = (field.x0 - size::INPUT_GAP - size::INPUT_ICON_SIZE - size::INPUT_XS_PADDING_X, at(tree.x0 + 2.0, y));
    assert_eq!(at(left + 3.0, y), background, "внутри поиска осталась заливка");
    assert!((-1..=1).any(|shift| at(left + f64::from(shift), y) != background), "у поиска пропала рамка");
}

#[test]
fn the_cross_shows_only_with_text_sits_at_the_right_edge_and_clears_the_field() {
    let (_vault, mut h) = start();
    let (cross, field) = (h.app.tree.clear_button().unwrap(), field(&h));
    assert!(h.root.get_widget(cross).unwrap().ctx().is_stashed(), "крестик виден у пустого поля");
    h.click(bounds(&h, field).center().x, bounds(&h, field).center().y);
    assert_eq!(h.root.focused_widget(), Some(field));
    h.type_text("коф");
    assert!(!h.root.get_widget(cross).unwrap().ctx().is_stashed(), "крестика нет, когда есть текст");
    let (tree, cross_box) = (bounds(&h, h.app.tree.scroll_id().unwrap()), bounds(&h, cross));
    assert_eq!(cross_box.x1, tree.x1 - size::SPACE_8 - size::INPUT_XS_PADDING_X, "крестик не у правого края поля");
    assert_eq!((cross_box.width(), cross_box.height()), (size::SIZE_20, size::SIZE_20));
    assert!(bounds(&h, field).x1 + size::INPUT_GAP <= cross_box.x0, "текст налезает на крестик");
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Кофемолка", "Кофе"]), "{:?}", names(&h));
    h.click(cross_box.center().x, cross_box.center().y);
    assert_eq!(h.app.tree.filter_text(&h.root), "");
    assert_eq!(names(&h), ["Проекты", "Кофе", "Чай"]);
    assert!(h.root.get_widget(cross).unwrap().ctx().is_stashed(), "крестик остался у пустого поля");
    assert_eq!(h.root.focused_widget(), Some(field), "после очистки поле потеряло фокус");
}

#[test]
fn real_keys_narrow_the_tree_as_you_type_and_widen_it_back_with_backspace() {
    let (_vault, mut h) = start();
    let (field, body_before) = (field(&h), h.app.note_ids.as_ref().map(|ids| ids.body.id()));
    h.click(bounds(&h, field).center().x, bounds(&h, field).center().y);
    h.type_text("к");
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Заметка", "Кофемолка", "Кофе"]), "{:?}", names(&h));
    h.type_text("о");
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Кофемолка", "Кофе"]), "{:?}", names(&h));
    h.type_text("ф");
    h.deliver();
    assert_eq!(names(&h), ["Проекты", "Кофемолка", "Кофе"]);
    assert_eq!(h.app.tree.built_rows().len(), 3, "построено не столько строк, сколько найдено");
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Заметка", "Кофемолка", "Кофе"]), "{:?}", names(&h));
    assert_eq!(h.app.tree.filter_text(&h.root), "к");
    assert_eq!(h.root.focused_widget(), Some(field), "набор увёл фокус из поля");
    assert_eq!(h.app.note_ids.as_ref().map(|ids| ids.body.id()), body_before, "набор в поле изменил заметку");
}

#[test]
fn escape_clears_the_field_and_a_miss_leaves_no_rows() {
    let (_vault, mut h) = start();
    let field = field(&h);
    h.send(field, TextAction::Changed("нет такого".into()));
    assert!(h.wait_for(|h| names(h).is_empty()));
    h.send(field, TextAction::Cancelled);
    h.deliver();
    assert_eq!(names(&h), ["Проекты", "Кофе", "Чай"]);
}

#[test]
fn revealing_a_hidden_note_clears_the_filter_first() {
    let (vault, mut h) = start();
    h.app.tree.filter(&mut h.root, "коф");
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Кофемолка", "Кофе"]));
    assert_eq!(h.app.tree.filter_text(&h.root), "коф");
    h.app.tree.reveal(&mut h.root, &vault.path().join("Чай.md"));
    h.deliver();
    assert_eq!(h.app.tree.filter_text(&h.root), "");
    assert!(names(&h).contains(&"Чай".to_string()), "{:?}", names(&h));
}

#[test]
fn files_that_appear_and_vanish_while_filtering_show_up_and_go_away() {
    let (vault, mut h) = start();
    h.app.tree.filter(&mut h.root, "коф");
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Кофемолка", "Кофе"]));
    let new = vault.path().join("Проекты").join("Кофейня.md");
    std::fs::write(&new, "").unwrap();
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&new));
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Кофейня", "Кофемолка", "Кофе"]), "{:?}", names(&h));
    std::fs::remove_file(&new).unwrap();
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&new));
    assert!(h.wait_for(|h| names(h) == ["Проекты", "Кофемолка", "Кофе"]), "{:?}", names(&h));
}

fn row_pixels(h: &mut Harness, name: &str) -> Vec<u32> {
    let rect = bounds(h, h.app.tree.row_id(name).unwrap());
    let frame = h.frame(W as u16, H as u16, 1.0);
    let pixels = frame.frame_pixels();
    let width = W as usize;
    (rect.y0 as usize..rect.y1 as usize).flat_map(|y| (rect.x0 as usize..rect.x1 as usize).map(move |x| (x, y))).map(|(x, y)| pixels[y * width + x]).collect()
}

#[test]
fn the_matched_letters_are_painted_in_the_row_and_follow_the_query() {
    let (_vault, mut h) = start();
    let shot = |h: &mut Harness, query: &str| {
        h.app.tree.filter(&mut h.root, query);
        assert!(h.wait_for(|h| h.app.tree.marks("Кофе").len() == 1), "{query}");
        row_pixels(h, "Кофе")
    };
    let (two, three, again) = (shot(&mut h, "ко"), shot(&mut h, "коф"), shot(&mut h, "коф"));
    assert!(differing_pixels(&two, &three) > 0, "подсветка не изменилась при удлинении запроса");
    assert_eq!(differing_pixels(&three, &again), 0, "один и тот же запрос рисуется по-разному");
}

#[test]
fn the_note_search_bar_keeps_its_size_after_sharing_the_row_builder() {
    let mut h = Harness::new();
    h.open(h.vault.path().join("a.md"));
    h.settle();
    h.app.open_find(&mut h.root);
    h.settle();
    let find = h.app.find.as_ref().unwrap();
    let (placeholder, counter) = (bounds(&h, find.placeholder.id()), bounds(&h, find.counter.id()));
    assert_eq!(placeholder.height(), size::SIZE_24);
    assert!(counter.x0 >= placeholder.x1 + size::INPUT_GAP, "счётчик вплотную к полю");
}
