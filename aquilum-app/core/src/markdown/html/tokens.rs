use crate::link_title::entities::decode;

pub enum Token<'a> {
    Open { name: String, attrs: &'a str },
    Close(String),
    Text(&'a str),
}

pub struct Tokens<'a>(&'a str);

impl<'a> Tokens<'a> {
    pub fn new(html: &'a str) -> Self {
        Self(html)
    }
}

fn after<'a>(text: &'a str, marker: &str) -> &'a str {
    text.find(marker).map_or("", |at| &text[at + marker.len()..])
}

fn tag_end(tag: &str) -> usize {
    let mut quote = None;
    for (at, c) in tag.char_indices() {
        match (quote, c) {
            (None, '>') => return at,
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if c == open => quote = None,
            _ => {}
        }
    }
    tag.len()
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        loop {
            let rest = self.0;
            if !rest.starts_with('<') {
                let end = rest.find('<').unwrap_or(rest.len());
                self.0 = &rest[end..];
                return (end > 0).then(|| Token::Text(&rest[..end]));
            }
            let tag = &rest[1..];
            match tag.chars().next() {
                Some('!') if tag.starts_with("!--") => self.0 = after(&tag[3..], "-->"),
                Some('!' | '?') => self.0 = after(tag, ">"),
                Some('/') => {
                    let end = tag_end(tag);
                    self.0 = tag.get(end + 1..).unwrap_or("");
                    return Some(Token::Close(tag[1..end].trim().to_ascii_lowercase()));
                }
                Some(c) if c.is_ascii_alphabetic() => {
                    let end = tag_end(tag);
                    let name_end = tag.find(|c: char| c.is_whitespace() || c == '/' || c == '>').unwrap_or(end);
                    let name = tag[..name_end].to_ascii_lowercase();
                    let attrs = &tag[name_end..end];
                    self.0 = tag.get(end + 1..).unwrap_or("");
                    if name == "script" || name == "style" {
                        let close = format!("</{name}");
                        self.0 = self.0.to_ascii_lowercase().find(&close).map_or("", |at| after(&self.0[at..], ">"));
                        continue;
                    }
                    return Some(Token::Open { name, attrs });
                }
                _ => {
                    let end = tag.find('<').map_or(rest.len(), |at| at + 1);
                    self.0 = &rest[end..];
                    return Some(Token::Text(&rest[..end]));
                }
            }
        }
    }
}

pub fn attr(attrs: &str, name: &str) -> Option<String> {
    let mut rest = attrs;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if rest.is_empty() {
            return None;
        }
        let end = rest.find(|c: char| c.is_whitespace() || c == '=' || c == '/').unwrap_or(rest.len());
        let (key, tail) = rest.split_at(end);
        let (value, tail) = match tail.trim_start().strip_prefix('=') {
            Some(value) => {
                let value = value.trim_start();
                match value.chars().next() {
                    Some(quote @ ('"' | '\'')) => {
                        let body = &value[1..];
                        let close = body.find(quote).unwrap_or(body.len());
                        (&body[..close], body.get(close + 1..).unwrap_or(""))
                    }
                    _ => value.split_at(value.find(char::is_whitespace).unwrap_or(value.len())),
                }
            }
            None => ("", tail),
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(decode(value));
        }
        rest = tail;
    }
}

pub fn prop(css: &str, name: &str) -> Option<String> {
    css.split(';')
        .filter_map(|rule| rule.split_once(':'))
        .rfind(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim().trim_end_matches("!important").trim().to_ascii_lowercase())
}
