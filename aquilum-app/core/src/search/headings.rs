use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Debug)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub start: usize,
    pub content: Range<usize>,
}

fn scan(source: &str) -> impl Iterator<Item = Heading> + '_ {
    let mut events = Parser::new_ext(source, Options::all()).into_offset_iter();
    std::iter::from_fn(move || {
        let mut open: Option<(u8, usize, String)> = None;
        for (event, range) in events.by_ref() {
            match event {
                Event::Start(Tag::Heading { level, .. }) => open = Some((level as u8, range.start, String::new())),
                Event::Text(text) | Event::Code(text) => {
                    if let Some((_, _, title)) = open.as_mut() {
                        title.push_str(&text);
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some((level, start, title)) = open.take() {
                        let title = title.trim().to_owned();
                        return Some(Heading { level, title, start, content: range.end..source.len() });
                    }
                }
                _ => {}
            }
        }
        None
    })
}

pub fn headings(source: &str) -> Vec<Heading> {
    let mut found = scan(source).collect::<Vec<_>>();
    for index in 0..found.len() {
        let level = found[index].level;
        found[index].content.end = found[index + 1..]
            .iter()
            .find(|next| next.level <= level)
            .map_or(source.len(), |next| next.start);
    }
    found
}

pub fn heading_at(text: &str, offset: usize) -> Option<String> {
    let line_end = |from: usize| text[from..].find('\n').map_or(text.len(), |at| from + at + 1);
    let parsed = &text[..line_end(line_end(offset.min(text.len())))];
    scan(parsed)
        .take_while(|heading| heading.start <= offset)
        .last()
        .map(|heading| heading.title)
}

pub fn matching<'a>(headings: &'a [Heading], wanted: &str) -> Vec<&'a Heading> {
    let needle = wanted.trim().trim_start_matches('#').trim().to_lowercase();
    headings
        .iter()
        .filter(|heading| heading.title.to_lowercase() == needle)
        .collect()
}

#[cfg(test)]
mod tests {
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
}
