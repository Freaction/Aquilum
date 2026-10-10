use super::*;
use crate::ui::editor::resolve::{Request, Suggestions};
use masonry::core::WidgetId;

const OPEN: &str = "[[";
const CLOSE: &str = "]]";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Suggest {
    from: usize,
    to: usize,
    query: String,
    wrap: bool,
    close: bool,
    items: Vec<String>,
    selected: usize,
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

impl TextEditor {
    fn suggest_context(&self) -> Option<(usize, usize, String, bool, bool)> {
        if self.single_line || self.doc.markdown.is_none() || !self.selection().is_empty() {
            return None;
        }
        let caret = self.focus;
        let paragraph = self.doc.paragraph(self.doc.at_offset(caret));
        if paragraph.block.is_code() {
            return None;
        }
        let text = self.doc.text();
        let line_start = text[..caret].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
        let before = &text[line_start..caret];
        if before.matches('`').count() % 2 == 1 {
            return None;
        }
        if let Some(open) = before.rfind(OPEN) {
            let query = &before[open + OPEN.len()..];
            if !query.contains(CLOSE) && !query.contains('|') && !query.contains('#') {
                let after = &text[caret..line_end];
                let close = after.find(CLOSE);
                let to = close.map_or(caret, |c| caret + c);
                return Some((line_start + open + OPEN.len(), to, query.to_owned(), false, close.is_none()));
            }
        }
        let start = before.char_indices().rev().take_while(|&(_, c)| word_char(c)).last().map(|(i, _)| i)?;
        if before[..start].ends_with('#') {
            return None;
        }
        Some((line_start + start, caret, before[start..].to_owned(), true, true))
    }

    pub(super) fn update_suggest(&mut self, id: WidgetId) {
        let Some(resolver) = self.doc.resolver.clone() else { return };
        let Some(min) = resolver.suggest_min() else { return self.close_suggest() };
        let Some((from, to, query, wrap, close)) = self.suggest_context().filter(|c| c.2.chars().count() >= min) else {
            return self.close_suggest();
        };
        let items = match resolver.suggest(&query) {
            Some(items) => items,
            None => self.suggest.as_ref().map(|s| s.items.clone()).unwrap_or_default(),
        };
        let selected = self.suggest.as_ref().filter(|s| s.query == query).map_or(0, |s| s.selected.min(items.len().saturating_sub(1)));
        self.suggest = Some(Suggest { from, to, query, wrap, close, items, selected });
        self.publish_suggest(id);
    }

    pub(super) fn poll_suggest(&mut self, id: WidgetId) {
        let Some(current) = &self.suggest else { return };
        let Some(items) = self.doc.resolver.as_ref().and_then(|r| r.suggest(&current.query)) else { return };
        if items != current.items {
            if let Some(s) = &mut self.suggest {
                s.items = items;
                s.selected = 0;
            }
            self.publish_suggest(id);
        }
    }

    pub(super) fn publish_suggest(&mut self, id: WidgetId) {
        let Some(resolver) = self.doc.resolver.clone() else { return };
        let view = self.suggest.as_ref().filter(|s| !s.items.is_empty() && self.laid_out()).map(|s| Suggestions {
            editor: id,
            caret: self.caret_at(s.from - if s.wrap { 0 } else { OPEN.len() }, 0, Affinity::Downstream),
            query: s.query.clone(),
            items: s.items.clone(),
            selected: s.selected,
        });
        if view != self.suggest_shown {
            self.suggest_shown = view.clone();
            resolver.request(Request::Suggest(view));
        }
    }

    pub(super) fn close_suggest(&mut self) {
        self.suggest = None;
        if self.suggest_shown.take().is_some()
            && let Some(resolver) = &self.doc.resolver
        {
            resolver.request(Request::Suggest(None));
        }
    }

    pub(super) fn hide_suggest(&mut self) {
        if self.suggest_shown.take().is_some()
            && let Some(resolver) = &self.doc.resolver
        {
            resolver.request(Request::Suggest(None));
        }
    }

    pub fn accept_suggestion(this: &mut WidgetMut<'_, Self>, index: usize) {
        if this.widget.accept_suggest(index) {
            this.ctx.submit_action::<TextAction>(TextAction::Changed(this.widget.doc.text().to_owned()));
            this.ctx.request_layout();
            this.ctx.request_render();
        }
    }

    pub(super) fn suggest_open(&self) -> bool {
        self.suggest_shown.is_some()
    }

    pub(super) fn suggest_key(&mut self, id: WidgetId, key: &Key) -> Option<bool> {
        let count = self.suggest.as_ref().map_or(0, |s| s.items.len());
        if !self.suggest_open() || count == 0 {
            return None;
        }
        let page = 8;
        let current = self.suggest.as_ref().map_or(0, |s| s.selected);
        let selected = match key {
            Key::Named(NamedKey::ArrowDown) => (current + 1) % count,
            Key::Named(NamedKey::ArrowUp) => (current + count - 1) % count,
            Key::Named(NamedKey::PageDown) => (current + page).min(count - 1),
            Key::Named(NamedKey::PageUp) => current.saturating_sub(page),
            Key::Named(NamedKey::Enter) => return Some(self.accept_suggest(current)),
            Key::Named(NamedKey::Escape) => {
                self.close_suggest();
                return Some(false);
            }
            _ => return None,
        };
        if let Some(s) = &mut self.suggest {
            s.selected = selected;
        }
        self.publish_suggest(id);
        Some(false)
    }

    pub(super) fn accept_suggest(&mut self, index: usize) -> bool {
        let Some(s) = self.suggest.clone() else { return false };
        let Some(title) = s.items.get(index) else { return false };
        let insert = format!("{}{title}{}", if s.wrap { OPEN } else { "" }, if s.close { CLOSE } else { "" });
        self.replace(s.from..s.to, &insert, Kind::Other);
        self.close_suggest();
        true
    }
}
