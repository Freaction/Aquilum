use super::parse::unwrap;
use super::{Cfi, Part, Steps};

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '^' | '[' | ']' | '(' | ')' | ',' | ';' | '=') {
            out.push('^');
        }
        out.push(c);
    }
    out
}

fn part_string(part: &Part) -> String {
    let param = part.side.as_ref().map_or(String::new(), |s| format!(";s={s}"));
    let mut out = format!("/{}", part.index);
    if let Some(id) = &part.id {
        out.push_str(&format!("[{}{param}]", escape(id)));
    }
    if let Some(offset) = part.offset.filter(|_| part.index % 2 == 1) {
        out.push_str(&format!(":{offset}"));
    }
    if let Some(t) = part.temporal.filter(|t| *t != 0.0) {
        out.push_str(&format!("~{t}"));
    }
    if !part.spatial.is_empty() {
        out.push_str(&format!("@{}", part.spatial.iter().map(f64::to_string).collect::<Vec<_>>().join(":")));
    }
    if !part.text.is_empty() || (part.id.is_none() && part.side.is_some()) {
        out.push_str(&format!("[{}{param}]", part.text.iter().map(|t| escape(t)).collect::<Vec<_>>().join(",")));
    }
    out
}

fn steps_string(steps: &Steps) -> String {
    steps.iter().map(|path| path.iter().map(part_string).collect::<String>()).collect::<Vec<_>>().join("!")
}

impl std::fmt::Display for Cfi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = match self {
            Cfi::Point(steps) => steps_string(steps),
            Cfi::Range { parent, start, end } => [parent, start, end].map(steps_string).join(","),
        };
        write!(f, "epubcfi({inner})")
    }
}

pub fn join(base: &str, local: &str) -> String {
    format!("epubcfi({}!{})", unwrap(base), unwrap(local))
}
