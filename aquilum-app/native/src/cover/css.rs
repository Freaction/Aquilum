use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Val {
    pub num: f64,
    pub px: f64,
    pub pct: f64,
    pub deg: f64,
}

impl Val {
    fn add(self, o: Val, sign: f64) -> Val {
        Val { num: self.num + sign * o.num, px: self.px + sign * o.px, pct: self.pct + sign * o.pct, deg: self.deg + sign * o.deg }
    }

    fn scale(self, k: f64) -> Val {
        Val { num: self.num * k, px: self.px * k, pct: self.pct * k, deg: self.deg * k }
    }

    fn is_number(&self) -> bool {
        self.px == 0.0 && self.pct == 0.0 && self.deg == 0.0
    }

    pub fn length(&self, reference: f64) -> f64 {
        self.px + self.num + self.pct / 100.0 * reference
    }

    pub fn angle(&self) -> f64 {
        self.deg + self.pct / 100.0 * 360.0 + self.num
    }
}

pub type Rgba = [f64; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extent {
    ClosestSide,
    FarthestSide,
    ClosestCorner,
    FarthestCorner,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RadialSize {
    Extent(Extent),
    Explicit(Val, Val),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Linear { angle: Option<f64>, corner: Option<(f64, f64)> },
    Radial { circle: bool, size: RadialSize, at: (Val, Val) },
    Conic { from: f64, at: (Val, Val) },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    pub color: Rgba,
    pub pos: Option<Val>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub kind: Kind,
    pub repeating: bool,
    pub stops: Vec<Stop>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub gradient: Gradient,
    pub position: (Val, Val),
    pub size: Option<(Val, Val)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Background {
    pub color: Rgba,
    pub layers: Vec<Layer>,
}

#[derive(Clone, Debug, Default)]
pub struct Pattern {
    pub base: Background,
    pub overlay: Background,
}

pub fn parse_patterns(css: &str) -> HashMap<String, Pattern> {
    let css = strip_comments(css);
    let mut rules: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_owned();
        let Some(close) = matching(rest, open) else { break };
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

fn matching(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

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

fn paren_end(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top(text: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        let split = if separator == ' ' { c.is_whitespace() } else { c == separator };
        if split && depth == 0 {
            if !current.trim().is_empty() {
                parts.push(current.trim().to_owned());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_owned());
    }
    parts
}

fn background(declarations: &[(String, String)], vars: &HashMap<String, String>, font: f64) -> Background {
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

pub fn value(text: &str, font: f64) -> Option<Val> {
    let tokens = tokenize(text)?;
    let mut parser = Calc { tokens, at: 0, font };
    let v = parser.expr()?;
    (parser.at == parser.tokens.len()).then_some(v)
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Num(f64, String),
    Op(char),
    Open,
    Close,
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    let text = text.trim();
    let text = text.strip_prefix("calc").unwrap_or(text);
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let unary = matches!(tokens.last(), None | Some(Token::Op(_)) | Some(Token::Open));
        if c.is_whitespace() {
            i += 1;
        } else if c == '(' {
            tokens.push(Token::Open);
            i += 1;
        } else if c == ')' {
            tokens.push(Token::Close);
            i += 1;
        } else if c.is_ascii_digit() || c == '.' || ((c == '-' || c == '+') && unary && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit() || *n == '.')) {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let number: f64 = chars[start..i].iter().collect::<String>().parse().ok()?;
            let unit_start = i;
            while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
                i += 1;
            }
            tokens.push(Token::Num(number, chars[unit_start..i].iter().collect()));
        } else if "+-*/".contains(c) {
            tokens.push(Token::Op(c));
            i += 1;
        } else {
            return None;
        }
    }
    Some(tokens)
}

struct Calc {
    tokens: Vec<Token>,
    at: usize,
    font: f64,
}

impl Calc {
    fn expr(&mut self) -> Option<Val> {
        let mut left = self.term()?;
        while let Some(Token::Op(op @ ('+' | '-'))) = self.tokens.get(self.at).cloned() {
            self.at += 1;
            let right = self.term()?;
            left = left.add(right, if op == '+' { 1.0 } else { -1.0 });
        }
        Some(left)
    }

    fn term(&mut self) -> Option<Val> {
        let mut left = self.factor()?;
        while let Some(Token::Op(op @ ('*' | '/'))) = self.tokens.get(self.at).cloned() {
            self.at += 1;
            let right = self.factor()?;
            left = match op {
                '*' if right.is_number() => left.scale(right.num),
                '*' if left.is_number() => right.scale(left.num),
                '/' if right.is_number() && right.num != 0.0 => left.scale(1.0 / right.num),
                _ => return None,
            };
        }
        Some(left)
    }

    fn factor(&mut self) -> Option<Val> {
        match self.tokens.get(self.at).cloned()? {
            Token::Open => {
                self.at += 1;
                let v = self.expr()?;
                (self.tokens.get(self.at) == Some(&Token::Close)).then_some(())?;
                self.at += 1;
                Some(v)
            }
            Token::Op('-') => {
                self.at += 1;
                Some(self.factor()?.scale(-1.0))
            }
            Token::Num(n, unit) => {
                self.at += 1;
                let mut v = Val::default();
                match unit.as_str() {
                    "" => v.num = n,
                    "px" => v.px = n,
                    "%" => v.pct = n,
                    "em" => v.px = n * self.font,
                    "deg" => v.deg = n,
                    "turn" => v.deg = n * 360.0,
                    _ => return None,
                }
                Some(v)
            }
            _ => None,
        }
    }
}

pub fn parse_color(text: &str) -> Option<Rgba> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        let digits: Vec<f64> = hex.chars().map(|c| c.to_digit(16).map(f64::from)).collect::<Option<_>>()?;
        let (r, g, b, a) = match digits.len() {
            3 => (digits[0] * 17.0, digits[1] * 17.0, digits[2] * 17.0, 255.0),
            4 => (digits[0] * 17.0, digits[1] * 17.0, digits[2] * 17.0, digits[3] * 17.0),
            6 => (digits[0] * 16.0 + digits[1], digits[2] * 16.0 + digits[3], digits[4] * 16.0 + digits[5], 255.0),
            8 => (digits[0] * 16.0 + digits[1], digits[2] * 16.0 + digits[3], digits[4] * 16.0 + digits[5], digits[6] * 16.0 + digits[7]),
            _ => return None,
        };
        return Some([r / 255.0, g / 255.0, b / 255.0, a / 255.0]);
    }
    match text {
        "transparent" => return Some([0.0; 4]),
        "white" => return Some([1.0, 1.0, 1.0, 1.0]),
        "black" => return Some([0.0, 0.0, 0.0, 1.0]),
        _ => {}
    }
    let open = text.find('(')?;
    let name = &text[..open];
    let inner = text[open + 1..].strip_suffix(')')?;
    let (main, alpha) = match inner.split_once('/') {
        Some((main, alpha)) => (main, Some(alpha.trim())),
        None => (inner, None),
    };
    let parts: Vec<&str> = main.split(|c: char| c == ',' || c.is_whitespace()).filter(|p| !p.is_empty()).collect();
    let alpha_of = |s: &str| -> Option<f64> {
        if let Some(p) = s.strip_suffix('%') { p.parse::<f64>().ok().map(|v| v / 100.0) } else { s.parse().ok() }
    };
    let alpha = match (alpha, parts.get(3)) {
        (Some(a), _) => alpha_of(a)?,
        (None, Some(a)) => alpha_of(a)?,
        (None, None) => 1.0,
    };
    match name {
        "rgb" | "rgba" => {
            let channel = |s: &str| -> Option<f64> {
                if let Some(p) = s.strip_suffix('%') { p.parse::<f64>().ok().map(|v| v / 100.0) } else { s.parse::<f64>().ok().map(|v| v / 255.0) }
            };
            Some([channel(parts.first()?)?, channel(parts.get(1)?)?, channel(parts.get(2)?)?, alpha])
        }
        "hsl" | "hsla" => {
            let h: f64 = parts.first()?.trim_end_matches("deg").parse().ok()?;
            let s: f64 = parts.get(1)?.trim_end_matches('%').parse::<f64>().ok()? / 100.0;
            let l: f64 = parts.get(2)?.trim_end_matches('%').parse::<f64>().ok()? / 100.0;
            let f = |n: f64| {
                let k = (n + h / 30.0).rem_euclid(12.0);
                let a = s * l.min(1.0 - l);
                l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
            };
            Some([f(0.0), f(8.0), f(4.0), alpha])
        }
        _ => None,
    }
}

fn is_color(token: &str) -> bool {
    token.starts_with('#') || ["rgb(", "rgba(", "hsl(", "hsla("].iter().any(|p| token.starts_with(p)) || matches!(token, "transparent" | "white" | "black")
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_values_and_colors() {
        assert_eq!(value("calc(250% / 3)", 16.0).unwrap().pct, 250.0 / 3.0);
        assert_eq!(value("calc(-46px / 4)", 16.0).unwrap().px, -11.5);
        assert_eq!(value("calc(3 * 46px / 4)", 16.0).unwrap().px, 34.5);
        assert!((value("calc(25% / -6)", 16.0).unwrap().pct + 25.0 / 6.0).abs() < 1e-9);
        assert_eq!(value("1.2em", 8.0).unwrap().px, 9.6);
        assert_eq!(value("-120deg", 16.0).unwrap().deg, -120.0);
        assert_eq!(parse_color("#f76").unwrap(), [1.0, 119.0 / 255.0, 102.0 / 255.0, 1.0]);
        assert_eq!(parse_color("rgba(0, 0, 0, 0.5)").unwrap()[3], 0.5);
        let hsl = parse_color("hsl(180deg 100% 80% / 0)").unwrap();
        assert!((hsl[0] - 0.6).abs() < 1e-9 && hsl[3] == 0.0);
    }

    #[test]
    fn every_pattern_parses() {
        let patterns = parse_patterns(super::super::CSS);
        for id in super::super::PATTERN_IDS {
            let pattern = patterns.get(*id).unwrap_or_else(|| panic!("узор {id} не разобран"));
            if !matches!(*id, "city" | "dreams" | "tunnel") {
                assert!(!pattern.base.layers.is_empty(), "у узора {id} нет слоёв");
            }
        }
        let cubes = &patterns["cubes"].base.layers;
        assert_eq!(cubes.len(), 7);
        assert!(matches!(cubes[1].gradient.kind, Kind::Conic { from, .. } if from == -120.0));
        let zigzag = &patterns["zigzag"].base;
        assert_eq!(zigzag.layers[0].position.0.px, -40.0);
        assert_eq!(zigzag.layers[0].size.unwrap().0.px, 80.0);
        assert!(zigzag.color[3] == 1.0);
        assert_eq!(patterns["dreams"].overlay.layers.len(), 1);
    }
}
