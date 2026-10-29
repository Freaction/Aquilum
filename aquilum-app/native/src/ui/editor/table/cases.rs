use super::commands::Command;
use super::markdown::{parse, parse_with};
use super::model::{Align, Range2, Table};
use super::session::{Outcome, Session};
use super::syntax::{is_meta, is_row, is_separator};
use super::tests::{blank_view, table};

fn typed(source: &str, cell: (usize, usize), caret: usize, text: &str) -> Table {
    let t = table(source);
    let view = blank_view(&t);
    let mut session = Session::new(0);
    session.active = Some(cell);
    session.caret = caret;
    session.anchor = caret;
    let Outcome::Write(model) = session.insert(&view, text) else { panic!("правка не записана") };
    model
}

fn round_trip(source: &str) {
    let t = table(source);
    assert_eq!(t.serialize(), source);
    assert_eq!(parse(&t.serialize().split('\n').collect::<Vec<_>>()).unwrap(), t);
}

#[test]
fn escaped_pipe_is_shown_without_slash_and_written_back_once() {
    let source = "| a\\|b | [[Заметка\\|алиас]] |\n| --- | --- |\n| `\\|` | 2 |";
    let t = table(source);
    assert_eq!((t.value(0, 0), t.value(0, 1), t.value(1, 0)), ("a|b", "[[Заметка|алиас]]", "`|`"));
    round_trip(source);
    let appended = typed(source, (0, 0), 3, "c");
    assert_eq!(appended.value(0, 0), "a|bc");
    assert_eq!(appended.serialize().lines().next(), Some("| a\\|bc | [[Заметка\\|алиас]] |"));
    let inserted = typed(source, (0, 0), 2, "c");
    assert_eq!(inserted.serialize().lines().next(), Some("| a\\|cb | [[Заметка\\|алиас]] |"));
    let alias = typed(source, (0, 1), "[[Заметка".len(), "x");
    assert_eq!(alias.serialize().lines().next(), Some("| a\\|b | [[Заметкаx\\|алиас]] |"));
}

#[test]
fn typed_pipe_is_escaped_and_survives_a_second_pass() {
    let first = typed("| a | b |\n| --- | --- |\n| 1 | 2 |", (0, 0), 1, "|");
    assert_eq!(first.value(0, 0), "a|");
    let text = first.serialize();
    assert_eq!(text.lines().next(), Some("| a\\| | b |"));
    let again = table(&text);
    assert_eq!(again.value(0, 0), "a|");
    assert_eq!(again.serialize(), text);
}

#[test]
fn backslash_before_a_pipe_never_splits_a_cell() {
    let source = "| x\\\\|y | z |\n| --- | --- |\n| 1 | 2 |";
    let t = table(source);
    assert_eq!(t.cols(), 2);
    assert_eq!(t.value(0, 0), "x\\|y");
    round_trip(source);
}

#[test]
fn short_rows_are_padded_with_explicit_empty_cells() {
    let t = table("| A | B | C |\n|---|---|---|\n| только одна |");
    assert_eq!(t.cols(), 3);
    assert_eq!((t.value(1, 0), t.value(1, 1), t.value(1, 2)), ("только одна", "", ""));
    assert!(t.merges.is_empty());
    assert_eq!(t.serialize(), "| A | B | C |\n| --- | --- | --- |\n| только одна | <!--q-empty--> | <!--q-empty--> |");
    let single = table("| A | B |\n|---|---|\n| 1 |");
    assert!(single.merges.is_empty());
    assert_eq!(single.value(1, 1), "");
}

#[test]
fn real_empty_cells_stay_distinct_from_padding() {
    let t = table("| A | B | C |\n|---|---|---|\n| 1 |   | 3 |\n| 4 |");
    assert_eq!(t.merges, [Range2 { top: 1, left: 0, bottom: 1, right: 1 }]);
    assert_eq!(t.value(2, 0), "4");
    assert!(t.merge_at(2, 1).is_none() && t.merge_at(2, 2).is_none());
}

#[test]
fn rows_of_different_width_never_lose_cells() {
    let t = table("| a | b |\n| --- | --- |\n| 1 | 2 | 3 |\n| x |");
    assert_eq!(t.cols(), 3);
    assert_eq!((t.value(0, 2), t.value(1, 2), t.value(2, 1)), ("", "3", ""));
    assert_eq!(t.aligns, [None, None, None]);
    let wide_header = table("| a | b | c |\n| --- | --- |\n| 1 |");
    assert_eq!((wide_header.cols(), wide_header.value(0, 2)), (3, "c"));
    let text = t.serialize();
    assert_eq!(table(&text), t);
    assert!(text.contains("| 1 | 2 | 3 |"));
}

#[test]
fn blank_table_has_explicit_cells_and_parses_back() {
    let text = Table::empty(2, 2).serialize();
    assert_eq!(text, "| <!--q-empty--> | <!--q-empty--> |\n| --- | --- |\n| <!--q-empty--> | <!--q-empty--> |");
    let t = table(&text);
    assert_eq!((t.rows(), t.cols()), (2, 2));
    assert!(t.merges.is_empty());
    assert_eq!(t, Table::empty(2, 2));
}

#[test]
fn meta_line_closes_the_table() {
    let meta = "<!--q-table:{\"merges\":[],\"cols\":[100,100]}-->";
    assert!(is_meta(meta) && !is_row(meta) && !is_separator(meta));
    let with_tail = parse(&["| a | b |", "| --- | --- |", "| 1 | 2 |", meta, "| 3 | 4 |"]).unwrap();
    assert_eq!(with_tail.rows(), 2);
    assert_eq!(with_tail.serialize().lines().last(), Some(meta));
    assert_eq!(with_tail.serialize().lines().count(), 4);
    let piped = "<!--q-table:{\"x\":\"a|b\"}-->";
    assert!(is_meta(piped) && !is_row(piped));
}

#[test]
fn section_seven_cells_round_trip() {
    round_trip("| Левое | По центру | Правое |\n| :--- | :---: | ---: |\n| ячейка | `код` | **жирный** |");
    round_trip("| Символ | Описание | Ссылка |\n| --- | --- | --- |\n| `\\|` | вертикальная черта: \\| | [пример](https://example.com) |\n| `<>` | угловые скобки: &lt;&gt; | <https://example.com> |\n| ~~x~~ | зачёркнутый | ![img](https://via.placeholder.com/20) |");
    round_trip("| [[вики-ссылкой]] | [[a\\|b]] | <https://example.com> |\n| --- | --- | --- |\n| ~~зачёркнутым~~ | <!--q-empty--> | x |");
}

#[test]
fn section_seven_alignment_and_outer_pipes() {
    let aligned = table("|:-------------------|:---------:|--------------------:|\n|:-------------------|:---------:|--------------------:|\n| ячейка             | ячейка    |              ячейка |");
    assert_eq!(aligned.aligns, [Some(Align::Left), Some(Align::Center), Some(Align::Right)]);
    assert_eq!(aligned.value(1, 2), "ячейка");
    let bare = table("Заголовок 1 | Заголовок 2 | Заголовок 3\n--- | --- | ---\nА | Б | В\nГ | Д | Е");
    assert_eq!((bare.rows(), bare.cols()), (3, 3));
    assert_eq!((bare.value(0, 0), bare.value(1, 2), bare.value(2, 1)), ("Заголовок 1", "В", "Д"));
    assert_eq!(bare.serialize(), "| Заголовок 1 | Заголовок 2 | Заголовок 3 |\n| --- | --- | --- |\n| А | Б | В |\n| Г | Д | Е |");
}

#[test]
fn section_seven_spec_table_with_empty_cells_is_stable() {
    let t = table("| A | B | C |\n|---|---|---|\n| 1 |   | 3 |\n|   | 2 |   |\n| только одна |");
    let again = table(&t.serialize());
    assert_eq!(again, t);
    assert_eq!(again.value(3, 0), "только одна");
    assert!(again.merge_at(3, 1).is_none() && again.merge_at(3, 2).is_none());
}

#[test]
fn trailing_spaces_are_kept_only_in_the_cell_being_edited() {
    let source = ["| a  | b  |", "| --- | --- |", "| ячейка             | 2 |"];
    let plain = parse(&source).unwrap();
    assert_eq!((plain.value(0, 0), plain.value(1, 0)), ("a", "ячейка"));
    let editing = parse_with(&source, Some((0, 0))).unwrap();
    assert_eq!((editing.value(0, 0), editing.value(0, 1)), ("a ", "b"));
    let padded = parse_with(&source, Some((1, 0))).unwrap();
    assert_eq!(padded.value(1, 0), format!("ячейка{}", " ".repeat(12)));
    assert_eq!(padded.serialize().lines().last(), Some("| ячейка             | 2 |"));
}

#[test]
fn words_with_spaces_can_be_typed_one_key_at_a_time() {
    let mut text = "| a | b |\n| --- | --- |\n| 1 | 2 |".to_owned();
    let mut session = Session::new(0);
    session.active = Some((0, 0));
    session.caret = 1;
    session.anchor = 1;
    for key in " hello  world".chars() {
        let lines: Vec<&str> = text.split('\n').collect();
        let view = blank_view(&parse_with(&lines, Some((0, 0))).unwrap());
        let Outcome::Write(model) = session.insert(&view, &key.to_string()) else { panic!("правка не записана") };
        text = model.serialize();
    }
    assert_eq!(table(&text).value(0, 0), "a hello  world");
    assert_eq!(session.caret, "a hello  world".len());
}

#[test]
fn a_cell_deletes_by_character_and_by_word_in_both_directions() {
    let deleted = |forward: bool, word: bool, caret: usize| {
        let t = table("| раз два | b |\n| --- | --- |\n| 1 | 2 |");
        let view = blank_view(&t);
        let mut session = Session::new(0);
        session.active = Some((0, 0));
        (session.caret, session.anchor) = (caret, caret);
        let Outcome::Write(model) = session.delete(&view, forward, word) else { panic!("правка не записана") };
        model.value(0, 0).to_owned()
    };
    assert_eq!(deleted(false, false, 13), "раз дв");
    assert_eq!(deleted(false, true, 13), "раз ");
    assert_eq!(deleted(true, false, 0), "аз два");
    assert_eq!(deleted(true, true, 0), " два");
    assert_eq!(deleted(true, true, 6), "раз");
}

#[test]
fn the_cell_menu_starts_with_copying_the_whole_table() {
    let t = table("| a | b |\n| --- | --- |\n| 1 | 2 |");
    let view = blank_view(&t);
    let mut session = Session::new(0);
    assert_eq!(session.context(&view, 1, 0)[0], (Command::Copy, true));
    assert!(matches!(session.run(&view, Command::Copy), Outcome::Copy(text) if text == t.serialize()));
}
