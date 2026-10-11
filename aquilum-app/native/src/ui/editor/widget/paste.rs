use super::*;
use crate::ui::editor::resolve::Request;

#[derive(Clone, Copy, PartialEq)]
pub struct Smart {
    pub links: bool,
    pub dashes: bool,
    pub callouts: bool,
}

impl Default for Smart {
    fn default() -> Self {
        Self { links: true, dashes: true, callouts: true }
    }
}

impl Smart {
    pub fn from_config(config: &aquilum_core::settings::models::AppConfig) -> Self {
        Self { links: config.editor.auto_link_title, dashes: config.editor.smart_dashes, callouts: config.editor.list_callouts }
    }
}

pub fn separate_pasted_frontmatter(text: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow::{Borrowed, Owned};
    let Some(first) = text.strip_prefix("---").and_then(|_| text.find('\n')) else { return Borrowed(text) };
    let mut from = first + 1;
    while let Some(rel) = text[from..].find("---") {
        let close = from + rel;
        if text[..close].ends_with('\n') {
            let after = close + 3;
            return match text[after..].strip_prefix('\n') {
                None if after == text.len() => Owned(format!("{text}\n\n")),
                Some(rest) if !rest.starts_with('\n') => Owned(format!("{}\n\n{rest}", &text[..after])),
                _ => Borrowed(text),
            };
        }
        from = close + 3;
    }
    Borrowed(text)
}


fn is_bare_url(text: &str) -> bool {
    let rest = text.strip_prefix("https://").or_else(|| text.strip_prefix("http://")).unwrap_or(text);
    let host = rest.strip_prefix("www.").unwrap_or(rest);
    (rest.len() < text.len() || text.starts_with("www."))
        && !text.chars().any(char::is_whitespace)
        && host.split(['/', '?', '#']).next().is_some_and(|h| h.contains('.') && h.len() >= 4)
}

fn is_image(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit_once('.').is_some_and(|(_, ext)| crate::images::decodable(ext))
}

pub(super) fn escape_title(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '*' | '_' | '`' | '|' | '<' | '>' | '~' | '\\' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

impl TextEditor {
    pub fn with_smart(mut self, smart: Smart) -> Self {
        self.smart = smart;
        self.doc.set_callouts(smart.callouts);
        self
    }

    pub fn set_smart(this: &mut WidgetMut<'_, Self>, smart: Smart) {
        if this.widget.smart != smart {
            this.widget.smart = smart;
            this.widget.doc.set_callouts(smart.callouts);
            this.ctx.request_layout();
            this.ctx.request_render();
        }
    }

    pub(super) fn smart_dash(&mut self, text: &str) -> bool {
        let at = self.focus;
        let before = &self.doc.text()[..at];
        if !self.smart.dashes || self.doc.markdown.is_none() || text != " " || self.anchor != at || !before.ends_with("--") || before.ends_with("---") {
            return false;
        }
        self.replace(at - 2..at, "— ", Kind::Typing);
        true
    }

    pub(super) fn paste_link(&mut self, id: masonry::core::WidgetId, text: &str) -> bool {
        let url = text.trim();
        let range = self.selection();
        let full = self.doc.text();
        let before = &full[full.floor_char_boundary(range.start.saturating_sub(64))..range.start];
        let in_link = before.rfind("](").is_some_and(|i| !before[i..].contains([')', ' ']));
        if !self.smart.links || self.doc.markdown.is_none() || !is_bare_url(url) || is_image(url) || in_link {
            return false;
        }
        let selected = full[range].trim().to_owned();
        if !selected.is_empty() {
            self.insert(&format!("[{}]({url})", escape_title(&selected)), Kind::Other);
            return true;
        }
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let placeholder = format!("Fetching Title#{stamp:x}");
        self.insert(&format!("[{placeholder}]({url})"), Kind::Other);
        if let Some(resolver) = &self.doc.resolver {
            resolver.request(Request::LinkTitle { editor: id, placeholder, url: url.to_owned() });
        }
        true
    }

    pub fn resolve_link_title(this: &mut WidgetMut<'_, Self>, placeholder: &str, title: &str) {
        let Some(at) = this.widget.doc.text().find(placeholder) else { return };
        let (anchor, focus) = (this.widget.anchor, this.widget.focus);
        let title = escape_title(title);
        this.widget.replace(at..at + placeholder.len(), &title, Kind::Other);
        let shift = |p: usize| if p > at { p + title.len() - placeholder.len() } else { p };
        (this.widget.anchor, this.widget.focus) = (shift(anchor), shift(focus));
        this.ctx.submit_action::<TextAction>(this.widget.doc.changed());
        this.ctx.request_layout();
        this.ctx.request_render();
    }
}
