use std::sync::Arc;

use super::Embeds;
use super::media;
use crate::ui::editor::resolve::{Book, Fetch};

impl Embeds {
    #[cfg(test)]
    pub fn watch(&self) -> impl Fn() -> bool + use<> {
        let weak = Arc::downgrade(&self.0);
        move || weak.strong_count() > 0
    }

    #[cfg(test)]
    pub fn cached_media(&self) -> usize {
        self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner()).len() + media::recent_count(&self.shared(""))
    }

    #[cfg(test)]
    pub fn cached_book(&self, target: &str) -> Option<Book> {
        match &self.0.books.entries.lock().unwrap_or_else(|e| e.into_inner()).get(target)?.value {
            Fetch::Ready(book) => Some(book.clone()),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn cached_rows(&self, query: &str) -> Option<usize> {
        match &self.0.grids.entries.lock().unwrap_or_else(|e| e.into_inner()).get(query)?.value {
            Fetch::Ready(grid) => Some(grid.rows.len()),
            _ => None,
        }
    }
}
