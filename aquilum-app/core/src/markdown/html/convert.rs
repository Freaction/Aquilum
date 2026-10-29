use super::style::{language, link_url, marks_of};
use super::tokens::{attr, prop, Token, Tokens};
use super::writer::Writer;
use crate::link_title::entities::decode;

const LIMIT: usize = 4 << 20;
const SKIP: [&str; 9] = ["head", "title", "svg", "button", "template", "noscript", "iframe", "select", "textarea"];
const VOID: [&str; 14] = ["br", "hr", "img", "input", "meta", "link", "wbr", "col", "area", "base", "source", "track", "embed", "param"];
const BLOCKS: [&str; 20] = [
    "p", "div", "section", "article", "header", "footer", "main", "aside", "figure", "figcaption", "address", "details", "summary", "dl", "dt", "dd", "form", "fieldset", "center", "body",
];

enum Kind {
    Plain,
    Skip,
    Block,
    Quote,
    List,
    Item,
    Link,
}

struct Frame {
    name: String,
    marks: u8,
    heading: bool,
    kind: Kind,
}

#[derive(Default)]
struct Code {
    lang: String,
    text: String,
}

fn close(writer: &mut Writer, kind: Kind) {
    match kind {
        Kind::Block | Kind::Item => writer.block(),
        Kind::Quote | Kind::List => writer.pop(),
        Kind::Link => writer.unlink(),
        Kind::Plain | Kind::Skip => {}
    }
}

pub fn to_markdown(html: &str) -> Option<String> {
    if html.len() > LIMIT || !html.contains('<') {
        return None;
    }
    let mut writer = Writer::default();
    let mut stack: Vec<Frame> = Vec::new();
    let mut code: Option<Code> = None;
    for token in Tokens::new(html) {
        if let Some(open) = code.as_mut() {
            match token {
                Token::Text(text) => open.text.push_str(&decode(text)),
                Token::Open { name, .. } if name == "br" => open.text.push('\n'),
                Token::Open { name, attrs } if name == "code" => {
                    if let Some(lang) = language(attrs) {
                        open.lang = lang;
                    }
                }
                Token::Close(name) if name == "pre" => {
                    let done = code.take().unwrap_or_default();
                    writer.code_block(&done.lang, &done.text);
                    stack.truncate(stack.iter().rposition(|f| f.name == "pre").unwrap_or(stack.len()));
                    writer.marks(stack.last().map_or(0, |f| f.marks));
                }
                _ => {}
            }
            continue;
        }
        let skipping = stack.iter().any(|f| matches!(f.kind, Kind::Skip));
        match token {
            Token::Text(text) if !skipping => writer.text(&decode(text)),
            Token::Text(_) => {}
            Token::Open { name, attrs } => {
                let void = VOID.contains(&name.as_str()) || attrs.trim_end().ends_with('/');
                if void {
                    if !skipping {
                        match name.as_str() {
                            "br" => writer.newline(),
                            "hr" => writer.rule(),
                            "input" if attr(attrs, "type").as_deref() == Some("checkbox") && stack.iter().any(|f| matches!(f.kind, Kind::Item)) => {
                                writer.checkbox(attr(attrs, "checked").is_some());
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                if skipping {
                    stack.push(Frame { name, marks: 0, heading: false, kind: Kind::Skip });
                    continue;
                }
                let css = attr(attrs, "style").unwrap_or_default();
                if name == "table" || (name != "pre" && prop(&css, "white-space").as_deref() == Some("pre")) {
                    return None;
                }
                let level = name.strip_prefix('h').filter(|_| name.len() == 2).and_then(|digit| digit.parse::<usize>().ok()).filter(|level| (1..=6).contains(level));
                let hidden = prop(&css, "display").as_deref() == Some("none") || prop(&css, "visibility").as_deref() == Some("hidden");
                let block = level.is_some() || BLOCKS.contains(&name.as_str()) || matches!(name.as_str(), "blockquote" | "ul" | "ol" | "pre");
                if block && !stack.iter().any(|f| matches!(f.kind, Kind::Item)) {
                    writer.end_implicit();
                }
                let kind = if hidden || SKIP.contains(&name.as_str()) {
                    Kind::Skip
                } else if let Some(level) = level {
                    writer.heading(level);
                    Kind::Block
                } else {
                    match name.as_str() {
                        "blockquote" => {
                            writer.quote();
                            Kind::Quote
                        }
                        "ul" | "ol" => {
                            writer.list(name == "ol", attr(attrs, "start").and_then(|start| start.trim().parse().ok()).unwrap_or(1));
                            Kind::List
                        }
                        "li" => {
                            writer.item();
                            Kind::Item
                        }
                        "pre" => {
                            writer.block();
                            code = Some(Code { lang: language(attrs).unwrap_or_default(), text: String::new() });
                            Kind::Plain
                        }
                        "a" => match attr(attrs, "href").and_then(|href| link_url(&href)) {
                            Some(url) => {
                                writer.link(url);
                                Kind::Link
                            }
                            None => Kind::Plain,
                        },
                        _ if BLOCKS.contains(&name.as_str()) => {
                            writer.block();
                            Kind::Block
                        }
                        _ => Kind::Plain,
                    }
                };
                let (parent, inherited) = stack.last().map_or((0, false), |f| (f.marks, f.heading));
                let heading = inherited || level.is_some();
                let marks = marks_of(parent, &name, &css, heading);
                writer.marks(marks);
                stack.push(Frame { name, marks, heading, kind });
            }
            Token::Close(name) => {
                let Some(at) = stack.iter().rposition(|f| f.name == name) else { continue };
                while stack.len() > at {
                    if let Some(frame) = stack.pop() {
                        close(&mut writer, frame.kind);
                    }
                }
                writer.marks(stack.last().map_or(0, |f| f.marks));
            }
        }
    }
    if let Some(done) = code {
        writer.code_block(&done.lang, &done.text);
    }
    while let Some(frame) = stack.pop() {
        close(&mut writer, frame.kind);
    }
    writer.finish()
}
