use super::{append, find, format};

#[test]
fn formats_like_tauri() {
    let cfi = "epubcfi(/6/4!/4/2/1:0,/1:12)";
    let line = format("Текст  [с]\n скобками", cfi, Some("Files/книга 1.fb2"), 3);
    assert_eq!(
        line,
        "> [!quote] Текст \\[с\\] скобками [3](aquilum-reader:book=Files%2F%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0+1.fb2&cfi=epubcfi%28%2F6%2F4%21%2F4%2F2%2F1%3A0%2C%2F1%3A12%29)\n"
    );
    let found = find(&format!("Текст\n\n{line}"));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].cfi, cfi);
    assert_eq!(found[0].book.as_deref(), Some("Files/книга 1.fb2"));
    assert_eq!(found[0].label, "3");
    let doc = format!("Текст\n\n{line}");
    assert_eq!(&doc[found[0].text.clone()], "Текст \\[с\\] скобками");
}

#[test]
fn a_quote_inside_a_code_block_is_not_a_bookmark() {
    let doc = "```md\n> [!quote] пример [1](aquilum-reader:cfi=x)\n```\n> [!quote] настоящая [2](aquilum-reader:cfi=y)";
    let found = find(doc);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].cfi, "y");
}

#[test]
fn reads_legacy_forms() {
    let doc = "> старая цитата[[→]](aquilum-reader:cfi=epubcfi%28%2F6%2F2%29)\n> [inline](aquilum-reader:cfi=x)\n> [!quote] без ссылки\n> [!book] [[Книга]]";
    let found = find(doc);
    assert_eq!(found.iter().map(|q| (q.cfi.as_str(), q.label.as_str())).collect::<Vec<_>>(), [("epubcfi(/6/2)", "1"), ("x", "2")]);
    assert_eq!(append("a", "q\n"), "a\n\nq\n");
    assert_eq!(append("a\n", "q\n"), "a\n\nq\n");
}
