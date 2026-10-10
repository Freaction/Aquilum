use std::ops::Range;

pub const SCHEME: &str = "aquilum-reader:";

#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    pub line: Range<usize>,
    pub text: Range<usize>,
    pub cfi: String,
    pub book: Option<String>,
    pub label: String,
}

fn form_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for &byte in text.as_bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => out.push(char::from(byte)),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn percent_decode(text: &str, plus: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if let Some(byte) = text.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) => {
                out.push(byte);
                i += 3;
                continue;
            }
            b'+' if plus => out.push(b' '),
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn href(cfi: &str, book: Option<&str>) -> String {
    let mut params = Vec::new();
    if let Some(book) = book.map(str::trim).filter(|b| !b.is_empty()) {
        params.push(format!("book={}", form_encode(book)));
    }
    params.push(format!("cfi={}", form_encode(cfi)));
    format!("{SCHEME}{}", params.join("&"))
}

pub fn parse_href(url: &str) -> Option<(String, Option<String>)> {
    let rest = url.trim().strip_prefix(SCHEME)?;
    if let Some(cfi) = rest.strip_prefix("cfi=") {
        return Some((percent_decode(cfi, false), None));
    }
    let mut cfi = None;
    let mut book = None;
    for pair in rest.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value, true);
        match percent_decode(key, true).as_str() {
            "cfi" if cfi.is_none() => cfi = Some(value),
            "book" if book.is_none() => book = Some(value.trim().to_owned()).filter(|b| !b.is_empty()),
            _ => {}
        }
    }
    Some((cfi.filter(|c| !c.is_empty())?, book))
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]")
}

pub fn format(text: &str, cfi: &str, book: Option<&str>, number: usize) -> String {
    let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if cfi.is_empty() {
        return format!("> [!quote] {cleaned}\n");
    }
    format!("> [!quote] {} [{number}]({})\n", escape(&cleaned), href(cfi, book))
}

fn header_len(line: &str) -> Option<usize> {
    let rest = line.strip_prefix('>')?;
    let rest = rest.trim_start();
    let tag = rest.get(..8).filter(|t| t.eq_ignore_ascii_case("[!quote]"))?;
    let after = rest[tag.len()..].trim_start();
    Some(line.len() - after.len())
}

fn link_at_end(body: &str) -> Option<(usize, &str)> {
    let trimmed = body.trim_end();
    let inner = trimmed.strip_suffix(')')?;
    let open = inner.rfind("(aquilum-reader:")?;
    let href = &inner[open + 1..];
    (!href.contains(')') && href.len() > SCHEME.len()).then_some((open, href))
}

fn parse_body(body: &str) -> Option<(usize, String, Option<String>)> {
    let (open, href) = link_at_end(body)?;
    let before = &body[..open];
    if let Some(label) = before.strip_suffix(']').and_then(|b| b.rfind('[').map(|at| (at, &b[at + 1..]))).filter(|(_, d)| !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit())) {
        let (at, digits) = label;
        let text = &before[..at];
        if text.ends_with(char::is_whitespace) && !text.trim_end().is_empty() {
            return Some((text.trim_end().len(), href.to_owned(), Some(digits.to_owned())));
        }
    }
    for marker in ["[[→]]", "[[↗]]"] {
        if let Some(text) = before.strip_suffix(marker).filter(|t| !t.is_empty()) {
            return Some((text.trim_end().len(), href.to_owned(), None));
        }
    }
    None
}

fn parse_line(line: &str, start: usize) -> Option<(Range<usize>, String, Option<String>)> {
    let trimmed = line.trim_end();
    if let Some(header) = header_len(trimmed) {
        let (len, href, label) = parse_body(&trimmed[header..])?;
        return Some((start + header..start + header + len, href, label));
    }
    let body = trimmed.strip_prefix("> ")?;
    let body_at = start + 2;
    if let Some((len, href, label)) = parse_body(body) {
        return Some((body_at..body_at + len, href, label));
    }
    let (open, href) = link_at_end(body)?;
    let text = body[..open].strip_prefix('[')?.strip_suffix(']')?;
    (!text.is_empty()).then(|| (body_at + 1..body_at + 1 + text.len(), href.to_owned(), None))
}

fn is_book_header(line: &str) -> bool {
    line.strip_prefix('>').map(str::trim_start).and_then(|r| r.get(..7)).is_some_and(|t| t.eq_ignore_ascii_case("[!book]"))
}

pub fn find(doc: &str) -> Vec<Quote> {
    let mut lines = Vec::new();
    let mut at = 0;
    for line in doc.split('\n') {
        lines.push((at, line));
        at += line.len() + 1;
    }
    let mut quotes = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (start, line) = lines[i];
        let header = header_len(line.trim_end()).is_some();
        if let Some((text, href, label)) = parse_line(line, start)
            && let Some((cfi, book)) = parse_href(&href)
        {
            let label = label.unwrap_or_else(|| (quotes.len() + 1).to_string());
            quotes.push(Quote { line: start..start + line.len(), text, cfi, book, label });
        }
        i += 1;
        if header {
            while i < lines.len() && lines[i].1.starts_with('>') && header_len(lines[i].1.trim_end()).is_none() && !is_book_header(lines[i].1) {
                i += 1;
            }
        }
    }
    quotes
}

pub fn count(doc: &str) -> usize {
    let mut lines = doc.split('\n').peekable();
    let mut total = 0;
    while let Some(line) = lines.next() {
        let header = header_len(line.trim_end()).is_some();
        if parse_line(line, 0).is_some() {
            total += 1;
        }
        if header {
            while lines.peek().is_some_and(|l| l.starts_with('>') && header_len(l.trim_end()).is_none() && !is_book_header(l)) {
                lines.next();
            }
        }
    }
    total
}

pub fn append(doc: &str, quote: &str) -> String {
    let gap = if !doc.is_empty() && !doc.ends_with('\n') { "\n\n" } else { "\n" };
    format!("{doc}{gap}{quote}")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn reads_legacy_forms() {
        let doc = "> старая цитата[[→]](aquilum-reader:cfi=epubcfi%28%2F6%2F2%29)\n> [inline](aquilum-reader:cfi=x)\n> [!quote] без ссылки\n> [!book] [[Книга]]";
        let found = find(doc);
        assert_eq!(found.iter().map(|q| (q.cfi.as_str(), q.label.as_str())).collect::<Vec<_>>(), [("epubcfi(/6/2)", "1"), ("x", "2")]);
        assert_eq!(count(doc), 2);
        assert_eq!(append("a", "q\n"), "a\n\nq\n");
        assert_eq!(append("a\n", "q\n"), "a\n\nq\n");
    }
}
