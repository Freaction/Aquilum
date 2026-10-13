use super::*;
use masonry::core::WidgetId;
use masonry::kurbo::Rect;
use std::path::PathBuf;

fn rect(h: &Harness, id: WidgetId) -> Rect {
    h.root.get_widget(id).unwrap().ctx().bounding_box()
}

fn counted(h: &mut Harness) {
    assert!(wait_until(500, 10, || {
        h.app.tree.poll(&mut h.root);
        h.app.tree.counted()
    }), "число элементов не посчитано");
}

fn folder_with_notes(h: &mut Harness, name: &str, notes: usize) -> PathBuf {
    let folder = h.vault.path().join(name);
    std::fs::create_dir(&folder).unwrap();
    for i in 0..notes {
        std::fs::write(folder.join(format!("заметка {i:02}.md")), "").unwrap();
    }
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&folder));
    folder
}

fn open_folder(h: &mut Harness, name: &str) {
    let center = rect(h, h.app.tree.row_id(name).expect("строка папки")).center();
    h.click(center.x, center.y);
}

fn scroll_tree(h: &mut Harness, dy: f64) {
    let center = rect(h, h.app.tree.scroll_id().unwrap()).center();
    h.scroll((center.x, center.y), 1.0, (0.0, dy));
    h.settle();
    h.deliver();
}

#[test]
fn folder_rows_count_everything_inside_at_any_depth() {
    let mut h = Harness::new();
    counted(&mut h);
    let folder = folder_with_notes(&mut h, "Проекты", 2);
    assert_eq!(h.app.tree.items("Проекты"), Some(2));
    let nested = folder.join("Вложенная");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(nested.join("глубоко.md"), "").unwrap();
    std::fs::write(nested.join("картинка.png"), "").unwrap();
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&nested));
    assert_eq!(h.app.tree.items("Проекты"), Some(5), "не учтены вложенная папка, заметка и картинка");
    std::fs::remove_file(nested.join("глубоко.md")).unwrap();
    h.app.tree.paths_changed(&mut h.root, &[nested.join("глубоко.md")]);
    assert_eq!(h.app.tree.items("Проекты"), Some(4));
    std::fs::remove_dir_all(&nested).unwrap();
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&nested));
    assert_eq!(h.app.tree.items("Проекты"), Some(2));
    std::fs::remove_dir_all(&folder).unwrap();
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&folder));
    assert_eq!(h.app.tree.items("Проекты"), None);
}

#[test]
fn expanded_folder_stays_pinned_and_a_click_on_it_collapses_the_folder() {
    let mut h = Harness::new();
    folder_with_notes(&mut h, "Проекты", 60);
    open_folder(&mut h, "Проекты");
    assert!(h.app.tree.pinned_names(&h.root).is_empty(), "папка закреплена, пока её строка на месте");
    scroll_tree(&mut h, -400.0);
    assert_eq!(h.app.tree.pinned_names(&h.root), ["Проекты"]);
    let tree = rect(&h, h.app.tree.scroll_id().unwrap());
    h.click(tree.x0 + 60.0, tree.y0 + 10.0);
    assert!(h.app.tree.pinned_names(&h.root).is_empty(), "после сворачивания папка осталась закреплённой");
    assert_eq!(h.app.tree.names(), ["Проекты", "a"]);
    assert!(h.app.tree.offset() < 40.0, "дерево не вернулось к строке свёрнутой папки: {}", h.app.tree.offset());
}
