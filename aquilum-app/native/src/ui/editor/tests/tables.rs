use masonry::core::keyboard::NamedKey;

use super::support::{click, command_key, markdown_root, named, scrolled, text, typing, wheel, PAGE};
use crate::ui::editor::TextEditor;

#[test]
fn table_cells_are_edited_in_place() {
    let mut root = markdown_root("| a | b |\n| --- | --- |\n| 1 | 2 |\nпосле");
    click(&mut root, 60.0, 50.0);
    typing(&mut root, "Z");
    assert!(text(&root).starts_with("| aZ | b |"), "{}", text(&root));
    named(&mut root, NamedKey::Tab);
    typing(&mut root, "Y");
    assert!(text(&root).starts_with("| aZ | bY |"), "{}", text(&root));
    named(&mut root, NamedKey::Tab);
    named(&mut root, NamedKey::Tab);
    named(&mut root, NamedKey::Tab);
    typing(&mut root, "Q");
    assert!(text(&root).contains("| Q | <!--q-empty--> |"), "{}", text(&root));
    named(&mut root, NamedKey::Escape);
    typing(&mut root, "!");
    assert!(text(&root).ends_with("!после"), "{}", text(&root));
    command_key(&mut root, "z");
    command_key(&mut root, "z");
    assert!(!text(&root).contains('Q'), "{}", text(&root));
}

#[test]
fn a_table_inside_a_quote_or_a_list_item_keeps_its_prefix_on_every_line() {
    for (source, edited) in [("> | a | b |\n> | --- | --- |\n> | 1 | 2 |", "> | aZ | b |\n> | --- | --- |\n> | 1 | 2 |"), ("- пункт\n  | a | b |\n  | --- | --- |\n  | 1 | 2 |", "- пункт\n  | aZ | b |\n  | --- | --- |\n  | 1 | 2 |")] {
        let mut root = markdown_root(source);
        click(&mut root, 90.0, if source.starts_with('>') { 50.0 } else { 70.0 });
        typing(&mut root, "Z");
        assert_eq!(text(&root), edited);
    }
}

#[test]
fn table_cells_take_spaces_and_pipes_while_typing() {
    let mut root = markdown_root("| a | b |\n| --- | --- |\n| 1 | 2 |\nпосле");
    click(&mut root, 60.0, 50.0);
    typing(&mut root, " x|y z");
    assert!(text(&root).starts_with("| a x\\|y z | b |"), "{}", text(&root));
    named(&mut root, NamedKey::Tab);
    typing(&mut root, "w");
    assert!(text(&root).starts_with("| a x\\|y z | bw |"), "{}", text(&root));
}

#[test]
fn vertical_gesture_over_wide_table_scrolls_the_page() {
    let cells: String = (0..6).map(|i| format!(" колонка {i} |")).collect();
    let rule: String = (0..6).map(|_| " --- |").collect();
    let table = format!("|{cells}\n|{rule}\n|{cells}\n<!--q-table:{{\"merges\":[],\"cols\":[300,300,300,300,300,300]}}-->\n");
    let body: String = format!("{table}\n{}", (0..200).map(|i| format!("Абзац {i}\n")).collect::<String>());
    let mut root = scrolled(TextEditor::new(&body).markdown(), 400, 300);
    for _ in 0..5 {
        wheel(&mut root, (200.0, 30.0), (-1.5, -12.0));
    }
    let offset = root.get_widget_with_tag(PAGE).expect("страница").inner().offset();
    assert!(offset >= 50.0, "жест над таблицей не прокрутил страницу: смещение {offset}");
}
