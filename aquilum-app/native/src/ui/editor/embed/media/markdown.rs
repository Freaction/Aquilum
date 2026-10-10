use masonry::parley::Alignment;

#[derive(Clone, Debug, PartialEq)]
pub struct ImageParams {
    pub width: Option<f64>,
    pub align: Alignment,
    pub crop: Option<[f64; 4]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmbedLine {
    pub src: String,
    pub wiki: bool,
    pub params: ImageParams,
}

const VIDEO: [&str; 7] = ["mp4", "webm", "ogv", "ogg", "mov", "m4v", "mkv"];

pub fn is_video(source: &str) -> bool {
    let clean = source.split(['?', '#']).next().unwrap_or(source);
    std::path::Path::new(clean).extension().is_some_and(|e| VIDEO.iter().any(|v| e.eq_ignore_ascii_case(v)))
}

fn round(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

pub fn parse(line: &str) -> EmbedLine {
    let line = line.trim();
    let wiki = line.starts_with("![[");
    let (src, params) = if let Some(inner) = line.strip_prefix("![[").and_then(|l| l.strip_suffix("]]")) {
        let mut parts = inner.split('|');
        (parts.next().unwrap_or_default().trim().to_owned(), parts.map(str::to_owned).collect::<Vec<_>>())
    } else {
        let close = line.find("](").unwrap_or(line.len());
        let alt = &line[2.min(line.len())..close.min(line.len())];
        let url = line[(close + 2).min(line.len())..].trim_end_matches(')').trim();
        let url = url.strip_prefix('<').and_then(|u| u.strip_suffix('>')).unwrap_or(url);
        (url.to_owned(), alt.split('|').map(str::to_owned).collect())
    };
    let mut out = ImageParams { width: None, align: Alignment::Center, crop: None };
    for p in params {
        match p.trim() {
            "left" => out.align = Alignment::Start,
            "right" => out.align = Alignment::End,
            "center" => out.align = Alignment::Center,
            p => {
                if let Some(crop) = p.strip_prefix("crop=") {
                    let v: Vec<f64> = crop.split(',').filter_map(|n| n.trim().parse::<f64>().ok()).map(|n| round(n.clamp(0.0, 100.0))).collect();
                    if let [l, t, r, b] = v[..]
                        && r > l
                        && b > t
                    {
                        out.crop = Some([l, t, r, b]);
                    }
                } else if let Ok(w) = p.parse::<f64>()
                    && w > 0.0
                    && p.bytes().all(|b| b.is_ascii_digit())
                {
                    out.width = Some(w);
                }
            }
        }
    }
    EmbedLine { src, wiki, params: out }
}

fn number(v: f64) -> String {
    let v = round(v);
    if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") }
}

pub fn format(embed: &EmbedLine) -> String {
    let mut params = Vec::new();
    if let Some(w) = embed.params.width {
        params.push(format!("{}", w.round() as i64));
    }
    match embed.params.align {
        Alignment::Start | Alignment::Left => params.push("left".into()),
        Alignment::End | Alignment::Right => params.push("right".into()),
        _ => {}
    }
    if let Some([l, t, r, b]) = embed.params.crop.filter(|c| *c != [0.0, 0.0, 100.0, 100.0]) {
        params.push(format!("crop={},{},{},{}", number(l), number(t), number(r), number(b)));
    }
    if embed.wiki {
        let mut parts = vec![embed.src.clone()];
        parts.extend(params);
        format!("![[{}]]", parts.join("|"))
    } else {
        let src = if embed.src.chars().any(char::is_whitespace) { format!("<{}>", embed.src) } else { embed.src.clone() };
        format!("![{}]({src})", params.join("|"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_both_forms() {
        for line in ["![601|left](<Files/image 6.png>)", "![[file.png|520|right|crop=10,12.5,90,80]]", "![](Files/a.png)"] {
            assert_eq!(format(&parse(line)), line);
        }
        let e = parse("![alt text|300](a.png)");
        assert_eq!(e.params.width, Some(300.0));
        assert_eq!(format(&e), "![300](a.png)");
    }
}
