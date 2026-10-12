use super::{Block, blocks};

#[test]
fn tables_without_outer_pipes() {
    let b = blocks("Заголовок 1 | Заголовок 2\n--- | ---\nА | Б\n\nтекст | с чертой\nобычный текст");
    assert_eq!(b[0], Block::Table { first: true, last: false });
    assert_eq!(b[2], Block::Table { first: false, last: true });
    assert_eq!(b[4], Block::Text, "строка с чертой без разделителя следом стала таблицей");
    let table = crate::ui::editor::table::parse(&["Заголовок 1 | Заголовок 2", "--- | ---", "А | Б"]).expect("таблица без крайних черт");
    assert_eq!(table.cells[1], ["А", "Б"]);
}

#[test]
fn tables_form_runs() {
    let b = blocks("текст\n| a | b |\n| --- | --- |\n| 1 | 2 |\n<!--q-table:{}-->\nпосле");
    assert_eq!(b[1], Block::Table { first: true, last: false });
    assert_eq!(b[3], Block::Table { first: false, last: false });
    assert_eq!(b[4], Block::Table { first: false, last: true });
    assert_eq!(b[5], Block::Text);
}

#[test]
fn the_meta_line_closes_a_table() {
    let b = blocks("| a |\n| --- |\n| 1 |\n<!--q-table:{}-->\n| 2 |\nтекст");
    assert_eq!(b[3], Block::Table { first: false, last: true });
    assert_eq!(b[4], Block::Text);
    assert_eq!(b[5], Block::Text);
}
