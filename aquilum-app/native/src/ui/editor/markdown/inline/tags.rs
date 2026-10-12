use std::ops::Range;

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

pub fn hashtags(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut prev = None;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '#' && prev.is_none_or(char::is_whitespace) {
            let end = text[i + 1..].char_indices().find(|(_, c)| !is_tag_char(*c)).map_or(text.len(), |(j, _)| i + 1 + j);
            let name = text[i + 1..end].trim_end_matches(['-', '/']);
            if name.chars().any(|c| !c.is_ascii_digit()) {
                let end = i + 1 + name.len();
                out.push(i..end);
                while chars.peek().is_some_and(|(j, _)| *j < end) {
                    chars.next();
                }
                prev = text[..end].chars().next_back();
                continue;
            }
        }
        prev = Some(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::hashtags;

    fn tags(text: &str) -> Vec<&str> {
        hashtags(text).into_iter().map(|r| &text[r]).collect()
    }

    #[test]
    fn a_tag_is_letters_digits_and_three_joiners_after_whitespace() {
        assert_eq!(tags("#тег #tag #вложенный/тег #tag-with-dash #tag_with_underscore"), ["#тег", "#tag", "#вложенный/тег", "#tag-with-dash", "#tag_with_underscore"]);
        assert_eq!(tags("#2026 #2026a #1/2"), ["#2026a", "#1/2"]);
        assert_eq!(tags("Конец #тег. И #тег, и #тег!"), ["#тег", "#тег", "#тег"]);
        assert_eq!(tags("# пробел, C# или http://example.com/#anchor, a#no #"), Vec::<&str>::new());
        assert_eq!(tags("#tag- и #path/"), ["#tag", "#path"]);
    }
}
