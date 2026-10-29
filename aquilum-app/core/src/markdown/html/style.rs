use super::tokens::{attr, prop};
use super::writer::{BOLD, CODE, ITALIC, STRIKE};

pub fn language(attrs: &str) -> Option<String> {
    let class = attr(attrs, "class")?;
    class.split_whitespace().find_map(|c| c.strip_prefix("language-").or_else(|| c.strip_prefix("lang-"))).map(str::to_owned)
}

pub fn marks_of(parent: u8, name: &str, css: &str, heading: bool) -> u8 {
    let mut marks = parent
        | match name {
            "b" | "strong" => BOLD,
            "i" | "em" | "cite" | "dfn" | "var" => ITALIC,
            "s" | "strike" | "del" => STRIKE,
            "code" | "kbd" | "samp" | "tt" => CODE,
            _ => 0,
        };
    let weight = prop(css, "font-weight").and_then(|w| match w.as_str() {
        "bold" | "bolder" => Some(true),
        "normal" | "lighter" => Some(false),
        number => number.parse::<u32>().ok().map(|n| n >= 600),
    });
    let slant = prop(css, "font-style").and_then(|s| match s.as_str() {
        "italic" | "oblique" => Some(true),
        "normal" => Some(false),
        _ => None,
    });
    let line = prop(css, "text-decoration-line").or_else(|| prop(css, "text-decoration")).map(|l| l.contains("line-through"));
    for (flag, on) in [(BOLD, weight), (ITALIC, slant), (STRIKE, line)] {
        match on {
            Some(true) => marks |= flag,
            Some(false) => marks &= !flag,
            None => {}
        }
    }
    if heading {
        marks &= !BOLD;
    }
    marks
}

pub fn link_url(href: &str) -> Option<String> {
    let href = href.trim();
    let lower = href.to_ascii_lowercase();
    if !["http://", "https://", "mailto:", "tel:"].iter().any(|scheme| lower.starts_with(scheme)) {
        return None;
    }
    let target = url::Url::parse(href)
        .ok()
        .filter(|u| u.host_str() == Some("www.google.com") && u.path() == "/url")
        .and_then(|u| u.query_pairs().find(|(key, _)| key == "q").map(|(_, value)| value.into_owned()));
    Some(target.unwrap_or_else(|| href.to_owned()))
}
