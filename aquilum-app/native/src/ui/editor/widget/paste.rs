use super::*;
use crate::ui::editor::resolve::Request;
use aquilum_core::markdown::escape;

#[derive(Clone, Copy, PartialEq)]
pub struct Smart {
    pub links: bool,
    pub dashes: bool,
    pub callouts: bool,
    pub tags: bool,
}

impl Default for Smart {
    fn default() -> Self {
        Self { links: true, dashes: true, callouts: true, tags: true }
    }
}

impl Smart {
    pub fn from_config(config: &aquilum_core::settings::models::AppConfig) -> Self {
        Self { links: config.editor.auto_link_title, dashes: config.editor.smart_dashes, callouts: config.editor.list_callouts, tags: config.editor.colored_tags }
    }
}

pub fn separate_pasted_frontmatter(text: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow::{Borrowed, Owned};
    let Some(end) = crate::frontmatter::end(text) else { return Borrowed(text) };
    match text[end..].strip_prefix('\n') {
        None if end == text.len() => Owned(format!("{text}\n\n")),
        Some(rest) if !rest.starts_with('\n') => Owned(format!("{}\n\n{rest}", &text[..end])),
        _ => Borrowed(text),
    }
}

fn is_bare_url(text: &str) -> bool {
    markdown::bare_urls(text).first().is_some_and(|url| *url == (0..text.len()))
}

impl TextEditor {
    pub fn with_smart(mut self, smart: Smart) -> Self {
        self.smart = smart;
        self.doc.set_flavors(smart.callouts, smart.tags);
        self
    }

    pub fn set_smart(this: &mut WidgetMut<'_, Self>, smart: Smart) {
        if this.widget.smart != smart {
            this.widget.smart = smart;
            this.widget.doc.set_flavors(smart.callouts, smart.tags);
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
        if !self.smart.links || self.doc.markdown.is_none() || !is_bare_url(url) || crate::images::kind(url) == Some(crate::images::MediaKind::Image) || in_link {
            return false;
        }
        let selected = full[range].trim().to_owned();
        if !selected.is_empty() {
            self.insert(&format!("[{}]({url})", escape::label(&selected)), Kind::Other);
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
        this.widget.dispatch(vec![(at..at + placeholder.len(), escape::label(title))], None, Kind::Other);
        this.ctx.submit_action::<TextAction>(this.widget.doc.changed());
        this.ctx.request_layout();
        this.ctx.request_render();
    }
}

#[cfg(test)]
mod tests {
    use super::separate_pasted_frontmatter;

    #[test]
    fn pasted_frontmatter_gets_a_blank_line_after_the_closing_fence() {
        assert_eq!(separate_pasted_frontmatter("---\na: 1\n---\nтекст"), "---\na: 1\n---\n\nтекст");
        assert_eq!(separate_pasted_frontmatter("---\na: 1\n---"), "---\na: 1\n---\n\n");
        assert_eq!(separate_pasted_frontmatter("---\na: 1\n---  \nтекст"), "---\na: 1\n---  \n\nтекст");
    }

    #[test]
    fn only_a_real_fence_line_counts() {
        for text in ["---\na: 1\n---\n\nтекст", "---\na: 1\n----\nтекст", "---\na: 1\n---abc\nтекст", "текст\n---\na: 1\n---\nтекст", "----\na: 1\n---\nтекст"] {
            assert!(matches!(separate_pasted_frontmatter(text), std::borrow::Cow::Borrowed(_)), "{text}");
        }
    }
}
