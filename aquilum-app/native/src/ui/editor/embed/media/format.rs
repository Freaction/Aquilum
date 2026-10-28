use masonry::parley::Alignment;

use super::FULL;
use super::markdown::{EmbedLine, ImageParams, round};

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
    if let Some([l, t, r, b]) = embed.params.crop.filter(|c| *c != FULL) {
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
