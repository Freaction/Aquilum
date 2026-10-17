use super::{heading_at, headings, matching, Heading};

const NOTE: &str = "# Один\n\nтекст\n\n## Два\n\nещё\n\n### Три\n\nглубже\n\n## Два\n\nдубль\n";

fn line(source: &str, heading: &Heading) -> String {
    source[heading.start..heading.content.start].to_owned()
}

#[test]
fn reads_level_title_and_section_bounds() {
    let list = headings(NOTE);
    assert_eq!(
        list.iter()
            .map(|heading| (heading.level, heading.title.as_str()))
            .collect::<Vec<_>>(),
        [(1, "Один"), (2, "Два"), (3, "Три"), (2, "Два")]
    );
    assert_eq!(line(NOTE, &list[0]), "# Один\n");
    assert_eq!(&NOTE[list[1].content.clone()], "\nещё\n\n### Три\n\nглубже\n\n");
    assert_eq!(&NOTE[list[2].content.clone()], "\nглубже\n\n");
    assert_eq!(&NOTE[list[3].content.clone()], "\nдубль\n");
}

#[test]
fn a_heading_without_a_trailing_newline_closes_at_the_end_of_the_text() {
    let source = "# Один";
    let list = headings(source);
    assert_eq!(line(source, &list[0]), "# Один");
    assert!(list[0].content.is_empty());
}

#[test]
fn a_setext_heading_keeps_its_underline_out_of_the_section() {
    let source = "Заголовок\n=========\n\nтекст\n";
    let list = headings(source);
    assert_eq!((list[0].level, list[0].title.as_str()), (1, "Заголовок"));
    assert_eq!(&source[list[0].content.clone()], "\nтекст\n");
}

#[test]
fn ignores_hashes_inside_a_code_block() {
    let list = headings("# Настоящий\n\n```\n# Ненастоящий\n```\n");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "Настоящий");
}

#[test]
fn finds_the_heading_that_encloses_an_offset() {
    assert_eq!(heading_at(NOTE, NOTE.find("глубже").unwrap()).as_deref(), Some("Три"));
    assert_eq!(heading_at(NOTE, NOTE.find("дубль").unwrap()).as_deref(), Some("Два"));
    assert_eq!(heading_at(NOTE, 0).as_deref(), Some("Один"));
    assert_eq!(heading_at("текст\n# Позже\n", 0), None);
    let setext = "Заголовок\n=========\n\nтекст\n";
    assert_eq!(heading_at(setext, 2).as_deref(), Some("Заголовок"));
}

#[test]
fn reports_every_heading_with_the_same_title() {
    let list = headings(NOTE);
    assert_eq!(matching(&list, "## Два").len(), 2);
    assert_eq!(matching(&list, "три").len(), 1);
    assert!(matching(&list, "Нет такого").is_empty());
}
