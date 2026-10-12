use std::collections::HashMap;

use super::background::background;
use super::calc::value;
use super::split::block_end;
use super::types::Pattern;

pub fn parse_patterns(css: &str) -> HashMap<String, Pattern> {
    let css = strip_comments(css);
    let mut rules: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_owned();
        let Some(close) = block_end(rest, open) else { break };
        let body = &rest[open + 1..close];
        rest = &rest[close + 1..];
        if selector.starts_with('@') {
            continue;
        }
        let declarations: Vec<(String, String)> = body
            .split(';')
            .filter_map(|d| d.split_once(':'))
            .map(|(k, v)| (k.trim().to_owned(), v.split_whitespace().collect::<Vec<_>>().join(" ")))
            .collect();
        for selector in selector.split(',') {
            rules.push((selector.trim().to_owned(), declarations.clone()));
        }
    }
    let base_font = rules.iter().find(|(s, _)| s == ".q-cover-pattern").and_then(|(_, d)| font_size(d)).unwrap_or(16.0);
    let mut patterns: HashMap<String, Pattern> = HashMap::new();
    for (selector, declarations) in &rules {
        let Some(name) = selector.strip_prefix(".q-cover-pattern--") else { continue };
        let (id, overlay) = match name.split_once("::") {
            Some((id, "before")) => (id, true),
            Some(_) => continue,
            None if name.contains(' ') => continue,
            None => (name, false),
        };
        let own: Vec<_> = rules.iter().filter(|(s, _)| s == &format!(".q-cover-pattern--{id}")).flat_map(|(_, d)| d.clone()).collect();
        let mut vars: HashMap<String, String> = HashMap::new();
        for (k, v) in own.iter().chain(declarations.iter()) {
            if k.starts_with("--") {
                vars.insert(k.clone(), v.clone());
            }
        }
        let font = font_size(&own).unwrap_or(base_font);
        let background = background(declarations, &vars, font);
        let pattern = patterns.entry(id.to_owned()).or_default();
        if overlay {
            pattern.overlay = background;
        } else {
            pattern.base = background;
        }
    }
    patterns
}

fn font_size(declarations: &[(String, String)]) -> Option<f64> {
    declarations.iter().rev().find(|(k, _)| k == "font-size").and_then(|(_, v)| value(v, 16.0)).map(|v| v.px)
}

fn strip_comments(css: &str) -> String {
    let mut out = String::new();
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start + 2..].find("*/").map_or("", |end| &rest[start + 2 + end + 2..]);
    }
    out.push_str(rest);
    out
}
