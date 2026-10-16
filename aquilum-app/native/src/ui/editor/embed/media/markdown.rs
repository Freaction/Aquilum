use aquilum_core::search::wiki::target::split;
use masonry::parley::Alignment;

use crate::images;

#[derive(Clone, Debug, PartialEq)]
pub struct ImageParams {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub align: Alignment,
    pub crop: Option<[f64; 4]>,
}

impl ImageParams {
    pub fn resize(&mut self, width: f64) {
        self.height = self.height.zip(self.width).map(|(height, old)| height * width / old);
        self.width = Some(width);
    }
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

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let at = tag.match_indices(name).map(|(at, _)| at + name.len()).find(|&end| {
        let before = tag[..end - name.len()].chars().next_back();
        before.is_some_and(char::is_whitespace) && tag[end..].trim_start().starts_with('=')
    })?;
    let value = tag[at..].trim_start()[1..].trim_start();
    match value.chars().next()? {
        quote @ ('"' | '\'') => value[1..].split(quote).next(),
        _ => value.split(|c: char| c.is_whitespace() || c == '>' || c == '/').next(),
    }
}

pub fn is_html_image(text: &str) -> bool {
    text.get(..4).is_some_and(|start| start.eq_ignore_ascii_case("<img"))
}

pub fn parse(line: &str) -> EmbedLine {
    let line = line.trim();
    let wiki = line.starts_with("![[");
    let (src, params, title) = if is_html_image(line) {
        let size = [attribute(line, "width"), attribute(line, "height")].map(|v| v.unwrap_or_default().trim_end_matches("px"));
        let params = if size[0].is_empty() { Vec::new() } else { vec![if size[1].is_empty() { size[0].to_owned() } else { size.join("x") }] };
        (attribute(line, "src").unwrap_or_default().to_owned(), params, attribute(line, "title").unwrap_or_default())
    } else if let Some(inner) = line.strip_prefix("![[").and_then(|l| l.strip_suffix("]]")) {
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
    let mut out = ImageParams { width: None, height: None, align: Alignment::Center, crop: None };
    let size = |p: &str| p.parse::<f64>().ok().filter(|v| *v > 0.0 && p.bytes().all(|b| b.is_ascii_digit()));
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
                } else if let Some(w) = size(p) {
                    out.width = Some(w);
                } else if let Some((w, h)) = p.split_once('x').and_then(|(w, h)| size(w).zip(size(h))) {
                    (out.width, out.height) = (Some(w), Some(h));
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
    match (embed.params.width, embed.params.height) {
        (Some(w), Some(h)) => params.push(format!("{}x{}", w.round() as i64, h.round() as i64)),
        (Some(w), None) => params.push(format!("{}", w.round() as i64)),
        _ => {}
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
    format(&EmbedLine { src: src.to_owned(), wiki: false, params: ImageParams { width, height: None, align: Alignment::Center, crop: None }, title: String::new() })
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
    fn width_by_height_and_html_images_keep_their_size() {
        let e = parse("![[a.png|300x200]]");
        assert_eq!((e.params.width, e.params.height), (Some(300.0), Some(200.0)));
        assert_eq!(format(&e), "![[a.png|300x200]]");
        let mut resized = e.params.clone();
        resized.resize(150.0);
        assert_eq!((resized.width, resized.height), (Some(150.0), Some(100.0)));
        let html = parse(r#"<img alt="x" src="Files/a b.png" width="150px" data-src="no" />"#);
        assert_eq!((html.src.as_str(), html.wiki, html.params.width, html.params.height), ("Files/a b.png", false, Some(150.0), None));
        assert!(is_html_image("<IMG src=a.png>") && !is_html_image("<image>x"));
    }

    #[test]
    fn wiki_address_loses_anchor_and_label() {
        let e = parse("![[ a.png # p | 300 | left ]]");
        assert_eq!((e.src.as_str(), e.params.width, e.params.align), ("a.png", Some(300.0), Alignment::Start));
    }
}
