use super::blocks::Container;
use crate::markdown::escape;

pub const BOLD: u8 = 1;
pub const ITALIC: u8 = 2;
pub const STRIKE: u8 = 4;
pub const CODE: u8 = 8;

const MARKERS: [(u8, &str); 4] = [(BOLD, "**"), (ITALIC, "*"), (STRIKE, "~~"), (CODE, "`")];

struct Link {
    url: String,
    at: Option<usize>,
}

#[derive(Default)]
pub struct Writer {
    pub(super) out: String,
    pub(super) containers: Vec<Container>,
    pub(super) lead: String,
    pub(super) blank: bool,
    pub(super) rich: bool,
    open: Vec<u8>,
    wanted: u8,
    link: Option<Link>,
    space: bool,
    line: bool,
}

fn marker(flag: u8) -> &'static str {
    MARKERS.iter().find(|(mark, _)| *mark == flag).map_or("", |(_, text)| text)
}

fn invisible(c: char) -> bool {
    matches!(c, '\u{200b}' | '\u{200e}' | '\u{200f}' | '\u{2060}' | '\u{feff}' | '\u{202a}'..='\u{202e}')
}

impl Writer {
    pub fn marks(&mut self, marks: u8) {
        self.wanted = marks;
    }

    pub(super) fn begin(&mut self) {
        if self.line {
            return;
        }
        if self.blank && !self.out.is_empty() {
            let quotes = self.containers.iter().filter(|c| matches!(c, Container::Quote)).count();
            self.out.push_str("> ".repeat(quotes).trim_end());
            self.out.push('\n');
        }
        self.blank = false;
        for container in &mut self.containers {
            match container {
                Container::Quote => self.out.push_str("> "),
                Container::List { marker, .. } => match marker.take() {
                    Some(marker) => self.out.push_str(&marker),
                    None => self.out.push('\t'),
                },
            }
        }
        self.out.push_str(&std::mem::take(&mut self.lead));
        self.line = true;
    }

    fn close_all(&mut self) {
        for flag in self.open.drain(..).rev() {
            self.out.push_str(marker(flag));
        }
    }

    pub fn end_line(&mut self) {
        self.close_all();
        self.space = false;
        self.lead.clear();
        if self.line {
            self.out.push('\n');
            self.line = false;
        }
    }

    pub fn block(&mut self) {
        self.end_line();
        if !self.containers.iter().any(|c| matches!(c, Container::List { .. })) {
            self.blank = true;
        }
    }

    pub fn newline(&mut self) {
        if self.line {
            self.end_line();
        } else {
            self.blank = true;
        }
    }

    pub fn text(&mut self, text: &str) {
        let text: String = text.chars().filter(|&c| !invisible(c)).collect();
        if text.starts_with(char::is_whitespace) {
            self.space = true;
        }
        let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !words.is_empty() {
            self.write(&words);
            self.space = text.ends_with(char::is_whitespace);
        }
    }

    fn write(&mut self, words: &str) {
        let fresh = !self.line;
        let heading = !self.lead.is_empty();
        self.begin();
        let space = std::mem::take(&mut self.space) && !fresh;
        let keep = self.open.iter().position(|flag| self.wanted & flag == 0).unwrap_or(self.open.len());
        for flag in self.open.drain(keep..).rev() {
            self.out.push_str(marker(flag));
        }
        if space {
            self.out.push(' ');
        }
        if let Some(link) = self.link.as_mut().filter(|link| link.at.is_none()) {
            link.at = Some(self.out.len());
            self.out.push('[');
        }
        for (flag, text) in MARKERS {
            if self.wanted & flag != 0 && !self.open.contains(&flag) {
                self.out.push_str(text);
                self.open.push(flag);
                self.rich = true;
            }
        }
        if self.wanted & CODE != 0 {
            self.out.push_str(words);
        } else {
            let line_start = fresh && !heading && self.wanted == 0 && self.link.is_none();
            let escaped = escape::prose(words, self.out.chars().next_back(), line_start);
            self.out.push_str(&escaped);
        }
    }

    pub fn checkbox(&mut self, checked: bool) {
        self.begin();
        self.out.push_str(if checked { "[x]" } else { "[ ]" });
        self.space = true;
        self.rich = true;
    }

    pub fn link(&mut self, url: String) {
        self.close_all();
        self.link = Some(Link { url, at: None });
    }

    pub fn unlink(&mut self) {
        let Some(Link { url, at: Some(at) }) = self.link.take() else { return };
        self.close_all();
        if self.out[at + 1..] == url {
            self.out.truncate(at);
            self.out.push_str(&url);
        } else {
            self.out.push_str(&format!("]({})", url.replace(' ', "%20").replace('(', "%28").replace(')', "%29")));
        }
        self.rich = true;
    }

    pub fn finish(mut self) -> Option<String> {
        self.unlink();
        self.end_line();
        let text = self.out.trim_end().to_owned();
        (self.rich && !text.is_empty()).then_some(text)
    }
}
