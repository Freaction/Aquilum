pub const SCHEME: &str = "aquilum-reader:";
pub const LEGACY_SCHEME: &str = "quantum-reader:";

pub fn strip_scheme(url: &str) -> Option<&str> {
    url.strip_prefix(SCHEME).or_else(|| url.strip_prefix(LEGACY_SCHEME))
}

pub fn is_reader(url: &str) -> bool {
    strip_scheme(url.trim()).is_some()
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

pub fn href(cfi: &str, book: Option<&str>) -> String {
    let mut params = Vec::new();
    if let Some(book) = book.map(str::trim).filter(|b| !b.is_empty()) {
        params.push(format!("book={}", form_encode(book)));
    }
    params.push(format!("cfi={}", form_encode(cfi)));
    format!("{SCHEME}{}", params.join("&"))
}

pub fn parse_href(url: &str) -> Option<(String, Option<String>)> {
    let rest = strip_scheme(url.trim())?;
    if let Some(cfi) = rest.strip_prefix("cfi=") {
        let cfi = crate::paths::percent_decode(cfi);
        return (!cfi.is_empty()).then_some((cfi, None));
    }
    let mut cfi = None;
    let mut book = None;
    for pair in rest.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = crate::paths::percent_decode(&value.replace('+', " "));
        match crate::paths::percent_decode(&key.replace('+', " ")).as_str() {
            "cfi" if cfi.is_none() => cfi = Some(value),
            "book" if book.is_none() => book = Some(value.trim().to_owned()).filter(|b| !b.is_empty()),
            _ => {}
        }
    }
    Some((cfi.filter(|c| !c.is_empty())?, book))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_in_a_book_survives_the_round_trip() {
        let url = href("epubcfi(/6/4!/4/2/1:0,/1:12)", Some("Files/книга 1.fb2"));
        assert!(is_reader(&url) && is_reader(&format!(" {url} ")));
        assert_eq!(parse_href(&url), Some(("epubcfi(/6/4!/4/2/1:0,/1:12)".to_owned(), Some("Files/книга 1.fb2".to_owned()))));
        assert_eq!(parse_href(&href("x", None)), Some(("x".to_owned(), None)));
    }

    #[test]
    fn the_old_scheme_and_the_short_form_are_read() {
        assert_eq!(parse_href("quantum-reader:cfi=epubcfi%28%2F6%2F2%29"), Some(("epubcfi(/6/2)".to_owned(), None)));
        assert_eq!(parse_href("aquilum-reader:cfi="), None);
        assert_eq!(parse_href("https://example.com"), None);
        assert!(!is_reader("aquilum"));
    }
}
