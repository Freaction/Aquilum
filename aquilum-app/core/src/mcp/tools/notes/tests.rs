use super::{changes, edit_from, Edit};
use serde_json::json;

fn compose(text: &str, edit: &Edit) -> Result<String, String> {
    let mut composed = text.to_owned();
    for (range, insert) in changes(&text, edit)?.into_iter().rev() {
        composed.replace_range(range, &insert);
    }
    Ok(composed)
}

const NOTE: &str = "# Заголовок\n\nПервый абзац.\n";
const SECTIONS: &str = "# Один\n\nтекст\n\n## Два\n\nещё\n\n### Три\n\nглубже\n";

fn edit(mode: &str, content: &str) -> Edit {
    Edit {
        mode: mode.to_owned(),
        content: content.to_owned(),
        find: None,
        heading: None,
        all: false,
    }
}

fn under(mode: &str, heading: &str, content: &str) -> Edit {
    Edit {
        heading: Some(heading.to_owned()),
        ..edit(mode, content)
    }
}

fn replacing(find: &str, content: &str, all: bool) -> Edit {
    Edit {
        find: Some(find.to_owned()),
        all,
        ..edit("findReplace", content)
    }
}

#[test]
fn appends_and_prepends_with_a_blank_line() {
    assert_eq!(
        compose(NOTE, &edit("append", "Новый абзац.")).unwrap(),
        "# Заголовок\n\nПервый абзац.\n\nНовый абзац."
    );
    assert_eq!(
        compose(NOTE, &edit("prepend", "Сверху.")).unwrap(),
        "Сверху.\n\n# Заголовок\n\nПервый абзац.\n"
    );
}

#[test]
fn appends_to_an_empty_note_without_leading_blank_lines() {
    assert_eq!(compose("", &edit("append", "Первый текст")).unwrap(), "Первый текст");
}

#[test]
fn reports_missing_find_target_instead_of_silently_doing_nothing() {
    assert!(compose(NOTE, &replacing("нет такого", "Замена", false)).is_err());
    assert_eq!(
        compose(NOTE, &replacing("Первый", "Второй", false)).unwrap(),
        "# Заголовок\n\nВторой абзац.\n"
    );
}

#[test]
fn refuses_to_replace_several_occurrences_unless_asked() {
    let note = "маркер и ещё маркер\n";
    let error = compose(note, &replacing("маркер", "замена", false)).unwrap_err();
    assert!(error.contains("2"), "в ошибке названо число вхождений: {error}");
    assert_eq!(
        compose(note, &replacing("маркер", "замена", true)).unwrap(),
        "замена и ещё замена\n"
    );
}

#[test]
fn inserts_under_the_requested_heading_and_keeps_the_rest() {
    let updated = compose(SECTIONS, &under("underHeading", "## Два", "вставка")).unwrap();
    assert!(updated.contains("## Два\n\nвставка\n\nещё"));
    assert!(updated.contains("### Три\n\nглубже"));
    assert!(updated.ends_with('\n'), "финальный перевод строки не теряется");
    assert!(compose(SECTIONS, &under("underHeading", "Нет такого", "вставка")).is_err());
}

#[test]
fn replaces_a_section_without_touching_its_neighbours() {
    let updated = compose(SECTIONS, &under("replaceSection", "### Три", "новое")).unwrap();
    assert!(updated.contains("### Три\n\nновое"));
    assert!(!updated.contains("глубже"));
    assert!(updated.contains("## Два\n\nещё"));
    assert!(updated.ends_with('\n'));
}

#[test]
fn replacing_a_section_stops_at_the_next_heading_of_the_same_level() {
    let updated = compose(SECTIONS, &under("replaceSection", "## Два", "новое")).unwrap();
    assert!(updated.contains("## Два\n\nновое"));
    assert!(!updated.contains("глубже"), "вложенный раздел входит в заменяемый");
    assert!(updated.starts_with("# Один\n\nтекст\n"));
}

#[test]
fn refuses_an_ambiguous_heading() {
    let note = "## Два\n\nпервый\n\n## Два\n\nвторой\n";
    let error = compose(note, &under("replaceSection", "Два", "новое")).unwrap_err();
    assert!(error.contains("2"), "в ошибке названо число заголовков: {error}");
}

#[test]
fn an_empty_replacement_deletes_the_text_or_empties_the_section() {
    let deleting = edit_from(&json!({ "mode": "findReplace", "find": " абзац", "content": "" })).unwrap();
    assert_eq!(compose(NOTE, &deleting).unwrap(), "# Заголовок\n\nПервый.\n");
    let emptying = edit_from(&json!({ "mode": "replaceSection", "heading": "## Два", "content": "" })).unwrap();
    assert_eq!(compose(SECTIONS, &emptying).unwrap(), "# Один\n\nтекст\n\n## Два\n");
    assert!(edit_from(&json!({ "mode": "replace", "content": "" })).is_err(), "пустой replace стёр бы заметку");
    assert!(edit_from(&json!({ "mode": "findReplace", "find": "x" })).is_err(), "content обязателен");
}

#[test]
fn rejects_an_unknown_mode_and_a_section_edit_without_a_heading() {
    assert!(compose(NOTE, &edit("patch", "текст")).is_err());
    assert!(compose(NOTE, &edit("replaceSection", "текст")).is_err());
}

#[test]
fn finds_text_across_a_page_boundary_like_one_pass() {
    let text: String = (0..60_000).map(|index| format!("абзац {index}\n\n")).collect();
    let boundary = {
        let mut pages = crate::search::pages::Pages::of(&text);
        let first = pages.next().unwrap().unwrap();
        first.start + first.text.len()
    };
    assert!(boundary < text.len());
    let previous = text[..boundary - 2].rfind('\n').unwrap() + 1;
    for needle in [&text[previous..boundary + "абзац".len()], "\n\nа", "абзац 1"] {
        let expected: Vec<_> = text.match_indices(needle).map(|(at, _)| at..at + needle.len()).collect();
        let found: Vec<_> = changes(&text, &replacing(needle, "", true)).unwrap().into_iter().map(|(range, _)| range).collect();
        assert_eq!(found, expected, "{needle:?}");
    }
}
