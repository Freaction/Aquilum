use std::collections::HashMap;

use super::calc::value;
use super::color::parse_color;
use super::gradient::gradient;
use super::split::{paren_end, split_top};
use super::types::{Background, Gradient, Layer, Rgba, Val};

fn substitute(text: &str, vars: &HashMap<String, String>) -> String {
    let mut out = text.to_owned();
    for _ in 0..4096 {
        let Some(start) = out.find("var(") else { break };
        let Some(end) = paren_end(&out, start + 3) else { break };
        let name = out[start + 4..end].trim();
        let replacement = vars.get(name).cloned().unwrap_or_default();
        out.replace_range(start..=end, &replacement);
    }
    out
}

pub(super) fn background(declarations: &[(String, String)], vars: &HashMap<String, String>, font: f64) -> Background {
    let mut color: Rgba = [0.0; 4];
    let mut images: Vec<Gradient> = Vec::new();
    let mut positions: Vec<(Val, Val)> = Vec::new();
    let mut sizes: Vec<Option<(Val, Val)>> = Vec::new();
    for (key, raw) in declarations {
        let text = substitute(raw, vars);
        match key.as_str() {
            "background-color" => color = parse_color(&text).unwrap_or(color),
            "background-image" => images = split_top(&text, ',').iter().filter_map(|g| gradient(g, font)).collect(),
            "background-position" => positions = split_top(&text, ',').iter().map(|p| pair(p, font, Val::default())).collect(),
            "background-size" => sizes = split_top(&text, ',').iter().map(|s| size_pair(s, font)).collect(),
            "background" => {
                color = [0.0; 4];
                images.clear();
                positions.clear();
                sizes.clear();
                for layer in split_top(&text, ',') {
                    let mut rest = Vec::new();
                    let mut image = None;
                    for token in split_top(&layer, ' ') {
                        if token.contains("gradient(") {
                            image = gradient(&token, font);
                        } else if let Some(c) = parse_color(&token) {
                            color = c;
                        } else {
                            rest.push(token);
                        }
                    }
                    if let Some(image) = image {
                        images.push(image);
                        positions.push(pair(&rest.join(" "), font, Val::default()));
                    }
                }
            }
            _ => {}
        }
    }
    let layers = images
        .into_iter()
        .enumerate()
        .map(|(i, gradient)| Layer {
            gradient,
            position: if positions.is_empty() { (Val::default(), Val::default()) } else { positions[i % positions.len()] },
            size: if sizes.is_empty() { None } else { sizes[i % sizes.len()] },
        })
        .collect();
    Background { color, layers }
}

fn pair(text: &str, font: f64, missing: Val) -> (Val, Val) {
    let values: Vec<Val> = split_top(text, ' ').iter().filter_map(|t| value(t, font)).collect();
    match values.as_slice() {
        [x, y, ..] => (*x, *y),
        [x] => (*x, missing),
        [] => (missing, missing),
    }
}

fn size_pair(text: &str, font: f64) -> Option<(Val, Val)> {
    let full = Val { pct: 100.0, ..Val::default() };
    let tokens = split_top(text, ' ');
    if tokens.iter().all(|t| t == "auto") {
        return None;
    }
    let get = |t: Option<&String>| t.filter(|t| *t != "auto").and_then(|t| value(t, font)).unwrap_or(full);
    Some((get(tokens.first()), get(tokens.get(1))))
}
