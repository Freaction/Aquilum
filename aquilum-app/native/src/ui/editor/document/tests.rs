use super::*;

fn markdown(text: &str) -> Document {
    let mut doc = Document::new(text);
    doc.set_markdown(Some(Fonts { size: 16.0, mono: crate::ui::theme::editor_font("iA Writer Mono"), ch: 0.0, callouts: true }));
    doc
}

fn blocks(doc: &Document) -> Vec<Block> {
    doc.paragraphs().iter().map(|p| p.block.clone()).collect()
}

#[test]
fn partial_classification_matches_a_full_pass() {
    let base = "# Заголовок\n\nтекст\n\n- пункт\n\n\t- вложенный\n\t  продолжение\n\n> цитата\n> [!note] выноска\n\n| а | б |\n| --- | --- |\n| 1 | 2 |\n\n```rust\nlet x = 1;\n```\n\nконец";
    let edits: [(&str, &str); 15] = [
        ("\n```rust", "| 5 | 6 |\n```rust"),
        ("| --- | --- |", "текст2"),
        ("текст2", "| --- | --- |"),
        ("текст", "```\nоткрыли"),
        ("открыли", "открыли\n```"),
        ("| 1 | 2 |", "| 1 | 2 |\n| 3 | 4 |"),
        ("\n| --- | --- |", ""),
        ("| а | б |", "| а | б |\n| --- | --- |"),
        ("- пункт", "1. пункт"),
        ("\n\n\t- вложенный", "\n\t- вложенный"),
        ("> цитата", "цитата"),
        ("```rust", "~~~rust"),
        ("~~~rust", "```rust"),
        ("let x = 1;\n```", "let x = 1;"),
        ("конец", "конец\n```"),
    ];
    let mut text = base.to_owned();
    let mut doc = markdown(&text);
    for (from, to) in edits {
        let at = text.find(from).unwrap_or_else(|| panic!("нет «{from}» в тексте"));
        doc.replace(at..at + from.len(), to);
        text.replace_range(at..at + from.len(), to);
        assert_eq!(doc.text(), text);
        assert_eq!(blocks(&doc), markdown::classify(text.split('\n')), "после правки «{from}» → «{to}»");
    }
}
