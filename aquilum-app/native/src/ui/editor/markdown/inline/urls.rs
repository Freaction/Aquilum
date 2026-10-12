use std::ops::Range;

pub fn bare_urls(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(start) = ["https://", "http://", "www."].iter().filter_map(|p| text[from..].find(p).map(|i| from + i)).min() {
        let boundary = text[..start].chars().next_back().is_none_or(|c| c.is_whitespace() || "([<*_~".contains(c));
        let mut end = text[start..].find(|c: char| c.is_whitespace() || c == '<').map_or(text.len(), |i| start + i);
        while let Some(last) = text[..end].chars().next_back() {
            let unbalanced = last == ')' && text[start..end].matches(')').count() > text[start..end].matches('(').count();
            if ".,:;!?\"'*_~".contains(last) || unbalanced {
                end -= last.len_utf8();
            } else {
                break;
            }
        }
        let host = text[start..end].trim_start_matches("https://").trim_start_matches("http://");
        if boundary && host.contains('.') && !host.starts_with('.') && end > start {
            out.push(start..end);
        }
        from = end.max(start + 1);
    }
    out
}
