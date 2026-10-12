mod context;

use super::*;
use crate::ui::editor::resolve::{Request, Suggestions};
use masonry::core::WidgetId;

const OPEN: &str = "[[";
const CLOSE: &str = "]]";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Link { wrap: bool, close: bool },
    Key,
    Value,
}

impl Source {
    fn lead(self) -> usize {
        match self {
            Source::Link { wrap: false, .. } => OPEN.len(),
            _ => 0,
        }
    }

    fn local(self) -> bool {
        !matches!(self, Source::Link { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Suggest {
    from: usize,
    to: usize,
    query: String,
    items: Vec<String>,
    selected: usize,
    source: Source,
}

impl TextEditor {
    fn keep_selected(&self, query: &str, items: &[String]) -> usize {
        self.suggest.as_ref().filter(|s| s.query == query).map_or(0, |s| s.selected.min(items.len().saturating_sub(1)))
    }

    pub(super) fn update_suggest(&mut self, id: WidgetId) {
        if let Some(next) = self.property_suggest() {
            let selected = self.keep_selected(&next.query, &next.items);
            self.suggest = Some(Suggest { selected, ..next });
            return self.publish_suggest(id);
        }
        let Some(resolver) = self.doc.resolver.clone() else { return };
        let Some(min) = resolver.suggest_min() else { return self.close_suggest() };
        let Some((from, to, query, source)) = self.suggest_context().filter(|c| c.2.chars().count() >= min) else {
            return self.close_suggest();
        };
        let items = match resolver.suggest(&query) {
            Some(items) => items,
            None => self.suggest.as_ref().map(|s| s.items.clone()).unwrap_or_default(),
        };
        let selected = self.keep_selected(&query, &items);
        self.suggest = Some(Suggest { from, to, query, items, selected, source });
        self.publish_suggest(id);
    }

    pub(super) fn poll_suggest(&mut self, id: WidgetId) {
        let Some(current) = self.suggest.as_ref().filter(|s| !s.source.local()) else { return };
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
            caret: self.caret_at(s.from - s.source.lead(), 0, Affinity::Downstream),
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
            this.ctx.submit_action::<TextAction>(this.widget.doc.changed());
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
        let insert = match s.source {
            Source::Link { wrap, close } => format!("{}{title}{}", if wrap { OPEN } else { "" }, if close { CLOSE } else { "" }),
            Source::Key => format!("{title}: "),
            Source::Value => title.clone(),
        };
        self.replace(s.from..s.to, &insert, Kind::Other);
        if matches!(s.source, Source::Link { close: false, .. }) && self.doc.text()[self.focus..].starts_with(CLOSE) {
            self.focus += CLOSE.len();
            self.anchor = self.focus;
        }
        self.close_suggest();
        true
    }
}
