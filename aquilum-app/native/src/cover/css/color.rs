use super::types::Rgba;

pub fn parse_color(text: &str) -> Option<Rgba> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        return hex_color(hex);
    }
    match text {
        "transparent" => return Some([0.0; 4]),
        "white" => return Some([1.0, 1.0, 1.0, 1.0]),
        "black" => return Some([0.0, 0.0, 0.0, 1.0]),
        _ => {}
    }
    functional_color(text)
}

fn hex_color(hex: &str) -> Option<Rgba> {
    let digits: Vec<f64> = hex.chars().map(|c| c.to_digit(16).map(f64::from)).collect::<Option<_>>()?;
    let (r, g, b, a) = match digits.len() {
        3 => (digits[0] * 17.0, digits[1] * 17.0, digits[2] * 17.0, 255.0),
        4 => (digits[0] * 17.0, digits[1] * 17.0, digits[2] * 17.0, digits[3] * 17.0),
        6 => (digits[0] * 16.0 + digits[1], digits[2] * 16.0 + digits[3], digits[4] * 16.0 + digits[5], 255.0),
        8 => (digits[0] * 16.0 + digits[1], digits[2] * 16.0 + digits[3], digits[4] * 16.0 + digits[5], digits[6] * 16.0 + digits[7]),
        _ => return None,
    };
    Some([r / 255.0, g / 255.0, b / 255.0, a / 255.0])
}

fn fraction(text: &str, whole: f64) -> Option<f64> {
    match text.strip_suffix('%') {
        Some(percent) => percent.parse::<f64>().ok().map(|v| v / 100.0),
        None => text.parse::<f64>().ok().map(|v| v / whole),
    }
}

fn functional_color(text: &str) -> Option<Rgba> {
    let open = text.find('(')?;
    let name = &text[..open];
    let inner = text[open + 1..].strip_suffix(')')?;
    let (main, alpha) = match inner.split_once('/') {
        Some((main, alpha)) => (main, Some(alpha.trim())),
        None => (inner, None),
    };
    let parts: Vec<&str> = main.split(|c: char| c == ',' || c.is_whitespace()).filter(|p| !p.is_empty()).collect();
    let alpha = match (alpha, parts.get(3)) {
        (Some(a), _) => fraction(a, 1.0)?,
        (None, Some(a)) => fraction(a, 1.0)?,
        (None, None) => 1.0,
    };
    match name {
        "rgb" | "rgba" => Some([fraction(parts.first()?, 255.0)?, fraction(parts.get(1)?, 255.0)?, fraction(parts.get(2)?, 255.0)?, alpha]),
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

pub(super) fn is_color(token: &str) -> bool {
    token.starts_with('#') || ["rgb(", "rgba(", "hsl(", "hsla("].iter().any(|p| token.starts_with(p)) || matches!(token, "transparent" | "white" | "black")
}
