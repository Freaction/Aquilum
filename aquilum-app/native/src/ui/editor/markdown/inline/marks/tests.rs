use super::{Kind, extra};

fn kinds(text: &str) -> Vec<(Kind, &str)> {
    extra(text, &[]).into_iter().map(|s| (s.kind, &text[s.inner])).collect()
}

#[test]
fn highlight_needs_flanking_double_marks() {
    assert_eq!(kinds("==Выделенный текст== тут"), [(Kind::Highlight, "Выделенный текст")]);
    assert_eq!(kinds("a == b == c"), []);
    assert_eq!(kinds("==a == b=="), [(Kind::Highlight, "a == b")]);
    assert_eq!(kinds("===x==="), []);
    assert_eq!(kinds("==один== и ==два=="), [(Kind::Highlight, "один"), (Kind::Highlight, "два")]);
    assert_eq!(kinds("==не закрыто"), []);
}

#[test]
fn a_comment_keeps_its_markers_in_the_inner_range() {
    assert_eq!(kinds("до %%скрыто%% после"), [(Kind::Comment, "%%скрыто%%")]);
    assert_eq!(kinds("100%% не пара"), []);
}

#[test]
fn footnote_references_and_block_ids_are_found() {
    assert_eq!(kinds("Текст[^1] и ещё[^note-2]."), [(Kind::Footnote, "[^1]"), (Kind::Footnote, "[^note-2]")]);
    assert_eq!(kinds("[^ нет] и [^] тоже нет"), []);
    assert_eq!(kinds("Абзац ^block-id"), [(Kind::BlockId, "^block-id")]);
    assert_eq!(kinds("^abc"), [(Kind::BlockId, "^abc")]);
    assert_eq!(kinds("E = mc^2 и дальше"), []);
    assert_eq!(kinds("x^2^"), [(Kind::Sup, "2")]);
}

#[test]
fn scripts_work_inside_a_word_and_never_swallow_spaces() {
    assert_eq!(kinds("H~2~O и E = mc^2^."), [(Kind::Sub, "2"), (Kind::Sup, "2")]);
    assert_eq!(kinds("a ~ b ~ c, cd ~/foo и ~/bar, 2^10 и x^"), []);
    assert_eq!(kinds("~~зачёркнуто~~"), []);
    assert_eq!(kinds("[^1][^2] и ^id"), [(Kind::Footnote, "[^1]"), (Kind::Footnote, "[^2]"), (Kind::BlockId, "^id")]);
    assert_eq!(kinds("x^(n+1)^"), [(Kind::Sup, "(n+1)")]);
}

#[test]
fn html_tags_and_backslash_escapes_become_spans() {
    let text = r"<kbd>Ctrl</kbd> + <mark>да</mark> и <u>под</u>, \*не курсив\*, \\ и C:\Users";
    let found: Vec<(Kind, &str)> = extra(text, &[]).into_iter().map(|s| (s.kind, &text[s.inner])).collect();
    assert!(found.contains(&(Kind::Code, "Ctrl")), "{found:?}");
    assert!(found.contains(&(Kind::Highlight, "да")), "{found:?}");
    assert!(found.contains(&(Kind::Underline, "под")), "{found:?}");
    assert_eq!(found.iter().filter(|(kind, _)| *kind == Kind::Escape).map(|(_, s)| *s).collect::<Vec<_>>(), ["*", "*", r"\"]);
    assert!(extra("<kbd></kbd> <b>без закрытия", &[]).is_empty());
}

#[test]
fn marks_inside_taken_ranges_are_dropped() {
    let text = "`==код==` и ==да==";
    let found = extra(text, &[0..text.find(" и").unwrap()]);
    assert_eq!(found.len(), 1);
    assert_eq!(&text[found[0].inner.clone()], "да");
}
