use super::calc::value;
use super::color::{is_color, parse_color};
use super::split::{paren_end, split_top};
use super::types::{Extent, Gradient, Kind, RadialSize, Stop, Val};

pub fn gradient(text: &str, font: f64) -> Option<Gradient> {
    let open = text.find('(')?;
    let name = text[..open].trim();
    let inner = &text[open + 1..paren_end(text, open)?];
    let repeating = name.starts_with("repeating-");
    let base = name.trim_start_matches("repeating-");
    let mut args = split_top(inner, ',');
    let config = match args.first() {
        Some(first) if !is_color(split_top(first, ' ').first()?) => Some(args.remove(0)),
        _ => None,
    };
    let config_tokens = config.as_deref().map(|c| split_top(c, ' ')).unwrap_or_default();
    let kind = match base {
        "linear-gradient" => linear(&config_tokens, font)?,
        "radial-gradient" => radial(&config_tokens, font)?,
        "conic-gradient" => conic(&config_tokens, font)?,
        _ => return None,
    };
    let mut stops = Vec::new();
    for arg in args {
        let tokens = split_top(&arg, ' ');
        let color = parse_color(tokens.first()?)?;
        let positions: Vec<Val> = tokens[1..].iter().filter_map(|t| value(t, font)).collect();
        if positions.is_empty() {
            stops.push(Stop { color, pos: None });
        }
        for pos in positions {
            stops.push(Stop { color, pos: Some(pos) });
        }
    }
    (!stops.is_empty()).then_some(Gradient { kind, repeating, stops })
}

fn linear(tokens: &[String], font: f64) -> Option<Kind> {
    if tokens.first().map(String::as_str) == Some("to") {
        let mut x = 0.0;
        let mut y = 0.0;
        for t in &tokens[1..] {
            match t.as_str() {
                "left" => x = -1.0,
                "right" => x = 1.0,
                "top" => y = -1.0,
                "bottom" => y = 1.0,
                _ => return None,
            }
        }
        return Some(match (x, y) {
            (0.0, -1.0) => Kind::Linear { angle: Some(0.0), corner: None },
            (1.0, 0.0) => Kind::Linear { angle: Some(90.0), corner: None },
            (0.0, 1.0) => Kind::Linear { angle: Some(180.0), corner: None },
            (-1.0, 0.0) => Kind::Linear { angle: Some(270.0), corner: None },
            corner => Kind::Linear { angle: None, corner: Some(corner) },
        });
    }
    let angle = match tokens.first() {
        Some(t) => value(t, font)?.angle(),
        None => 180.0,
    };
    Some(Kind::Linear { angle: Some(angle), corner: None })
}

fn position(tokens: &[String], font: f64) -> Option<(Val, Val)> {
    let pct = |p: f64| Val { pct: p, ..Val::default() };
    let keyword = |t: &str| match t {
        "left" | "top" => Some(pct(0.0)),
        "center" => Some(pct(50.0)),
        "right" | "bottom" => Some(pct(100.0)),
        _ => None,
    };
    match tokens {
        [] => Some((pct(50.0), pct(50.0))),
        [single] => match single.as_str() {
            "top" | "bottom" => Some((pct(50.0), keyword(single)?)),
            _ => Some((keyword(single).or_else(|| value(single, font))?, pct(50.0))),
        },
        [x, y, ..] => {
            let (x, y) = if matches!(x.as_str(), "top" | "bottom") || matches!(y.as_str(), "left" | "right") { (y, x) } else { (x, y) };
            Some((keyword(x).or_else(|| value(x, font))?, keyword(y).or_else(|| value(y, font))?))
        }
    }
}

fn radial(tokens: &[String], font: f64) -> Option<Kind> {
    let split = tokens.iter().position(|t| t == "at").unwrap_or(tokens.len());
    let at = if split < tokens.len() { position(&tokens[split + 1..], font)? } else { position(&[], font)? };
    let mut circle = None;
    let mut extent = None;
    let mut lengths = Vec::new();
    for t in &tokens[..split] {
        match t.as_str() {
            "circle" => circle = Some(true),
            "ellipse" => circle = Some(false),
            "closest-side" => extent = Some(Extent::ClosestSide),
            "farthest-side" => extent = Some(Extent::FarthestSide),
            "closest-corner" => extent = Some(Extent::ClosestCorner),
            "farthest-corner" => extent = Some(Extent::FarthestCorner),
            other => lengths.push(value(other, font)?),
        }
    }
    let (circle, size) = match lengths.as_slice() {
        [r] => (true, RadialSize::Explicit(*r, *r)),
        [rx, ry, ..] => (false, RadialSize::Explicit(*rx, *ry)),
        [] => (circle.unwrap_or(false), RadialSize::Extent(extent.unwrap_or(Extent::FarthestCorner))),
    };
    Some(Kind::Radial { circle, size, at })
}

fn conic(tokens: &[String], font: f64) -> Option<Kind> {
    let mut from = 0.0;
    let mut at = position(&[], font)?;
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "from" => {
                from = value(tokens.get(i + 1)?, font)?.angle();
                i += 2;
            }
            "at" => {
                let end = tokens[i + 1..].iter().position(|t| t == "from").map_or(tokens.len(), |p| i + 1 + p);
                at = position(&tokens[i + 1..end], font)?;
                i = end;
            }
            _ => return None,
        }
    }
    Some(Kind::Conic { from, at })
}
