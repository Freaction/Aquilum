use aquilum_core::search::wiki::target::split;
use masonry::parley::Alignment;

use crate::images;

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
    pub title: String,
}

fn embeddable(source: &str) -> bool {
    images::kind(source).is_some() || images::extension(source).is_none()
}

pub fn is_embed(line: &str) -> bool {
    let line = line.trim();
    if let Some(inner) = line.strip_prefix("![[").and_then(|l| l.strip_suffix("]]")) {
        return !inner.contains(['[', ']']) && images::kind(split(inner).note).is_some();
    }
    let Some(rest) = line.strip_prefix("![") else { return false };
    let Some(close) = rest.find(']') else { return false };
    let Some(body) = rest[close + 1..].strip_prefix('(').and_then(|b| b.strip_suffix(')')) else { return false };
    let body = body.trim();
    let balanced = match body.strip_prefix('<').and_then(|b| b.strip_suffix('>')) {
        Some(source) => !source.is_empty() && !source.contains(['<', '>']),
        None => !body.is_empty() && !body.contains(['(', ')']),
    };
    balanced && embeddable(&parse(line).src)
}

fn round(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

pub fn parse(line: &str) -> EmbedLine {
    let line = line.trim();
    let wiki = line.starts_with("![[");
    let (src, params, title) = if let Some(inner) = line.strip_prefix("![[").and_then(|l| l.strip_suffix("]]")) {
        let link = split(inner);
        (link.note.to_owned(), link.label.map_or_else(Vec::new, |label| label.split('|').map(str::to_owned).collect()), "")
    } else {
        let close = line.find("](").unwrap_or(line.len());
        let alt = &line[2.min(line.len())..close.min(line.len())];
        let body = line[(close + 2).min(line.len())..].trim_end_matches(')').trim();
        let (url, title) = match body.strip_prefix('<') {
            Some(rest) => rest.split_once('>').unwrap_or((rest, "")),
            None => body.split_once(char::is_whitespace).unwrap_or((body, "")),
        };
        (url.to_owned(), alt.split('|').map(str::to_owned).collect(), title.trim())
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
    EmbedLine { src, wiki, params: out, title: title.to_owned() }
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
        let title = if embed.title.is_empty() { String::new() } else { format!(" {}", embed.title) };
        format!("![{}]({src}{title})", params.join("|"))
    }
}

pub fn media_line(src: &str, width: Option<f64>) -> String {
    format(&EmbedLine { src: src.to_owned(), wiki: false, params: ImageParams { width, align: Alignment::Center, crop: None }, title: String::new() })
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

    #[test]
    fn title_is_not_part_of_the_address() {
        for (line, src) in [("![x](https://e.com/a.png?t=1 \"Подсказка\")", "https://e.com/a.png?t=1"), ("![x](<Files/image 6.png> 'Подсказка')", "Files/image 6.png")] {
            let e = parse(line);
            assert_eq!(e.src, src);
            assert_eq!(format(&e), line.replacen("![x]", "![]", 1), "заголовок потерялся при пересборке строки");
        }
    }

    #[test]
    fn only_decodable_images_and_playable_videos_are_embeds() {
        for line in ["![[a.png]]", "![[a.PNG|300]]", "![[clip.mp4]]", "![[a.png#x|10]]", "![](a.jpg)", "![](https://x.io/photo)", "![alt](<Files/a b.webp> \"t\")"] {
            assert!(is_embed(line), "{line}");
        }
        for line in ["![[a.svg]]", "![[a.avif|200]]", "![[doc.pdf]]", "![[Заметка]]", "![](a.svg)", "![](https://x.io/a.avif?w=1)", "![](a.png", "![](a.pdf)"] {
            assert!(!is_embed(line), "{line}");
        }
    }

    #[test]
    fn wiki_address_loses_anchor_and_label() {
        let e = parse("![[ a.png # p | 300 | left ]]");
        assert_eq!((e.src.as_str(), e.params.width, e.params.align), ("a.png", Some(300.0), Alignment::Start));
    }
}
