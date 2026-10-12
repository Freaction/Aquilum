use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::{Embeds, Entry, book, claim, dataview, links, media, refresh_after};
use crate::ui::editor::resolve::{Book, Fetch, Grid, Media, Request, Resolver};

const SUGGESTIONS: usize = 12;

impl Resolver for Embeds {
    fn dataview(&self, query: &str) -> (Fetch<Grid>, u64) {
        let text = query.to_owned();
        let weak = Arc::downgrade(&self.0);
        self.fetch(|i| &i.grids, query, move |inner| {
            let (result, refresh) = dataview::run(inner, &text);
            if let (Some(seconds), Some(strong)) = (refresh, weak.upgrade()) {
                refresh_after(&strong, text, seconds);
            }
            result
        })
    }

    fn media(&self, source: &str, wiki: bool) -> (Fetch<Media>, u64) {
        let source_owned = source.to_owned();
        let key = format!("{}{source}", if wiki { "[[" } else { "" });
        let shared = self.shared(&key);
        {
            let mut entries = self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner());
            if !entries.contains_key(&key)
                && let Some(media) = media::recent(&shared)
            {
                let epoch = self.0.media.epoch.load(Ordering::Acquire);
                let stamp = self.0.stamps.fetch_add(1, Ordering::AcqRel) + 1;
                entries.insert(key.clone(), Entry { value: Fetch::Ready(media), epoch, running: false, stamp, used: true, next: None });
            }
        }
        self.fetch(|i| &i.media, &key, move |inner| {
            let fetched = media::load(inner, &source_owned, wiki);
            if let Fetch::Ready(media) = &fetched {
                media::remember(shared, media);
            }
            fetched
        })
    }

    fn book(&self, target: &str) -> (Fetch<Book>, u64) {
        let (value, stamp, epoch) = claim(&self.0.books, target);
        let Some(epoch) = epoch else { return (value, stamp) };
        let mut queue = self.0.book_queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.pending.push((target.to_owned(), epoch));
        if !queue.running {
            queue.running = true;
            let inner = Arc::clone(&self.0);
            std::thread::spawn(move || book::drain(&inner));
        }
        (value, stamp)
    }

    fn suggest(&self, query: &str) -> Option<Vec<String>> {
        let text = query.to_owned();
        let (fetched, _) = self.fetch(|i| &i.suggestions, &query.to_lowercase(), move |inner| {
            match inner.core.search.suggest_notes(&inner.workspace(), &text, SUGGESTIONS) {
                Ok(notes) => Fetch::Ready(notes.into_iter().map(|n| n.title).collect()),
                Err(_) => Fetch::Ready(Vec::new()),
            }
        });
        match fetched {
            Fetch::Ready(titles) => Some(titles),
            _ => None,
        }
    }

    fn unresolved(&self) -> (Arc<HashSet<String>>, u64) {
        self.0.links.lock().unwrap_or_else(|e| e.into_inner()).unresolved()
    }

    fn resolve(&self, targets: Vec<String>) {
        links::resolve(&self.0, targets);
    }

    fn suggest_min(&self) -> Option<usize> {
        let editor = self.0.core.settings.get_config().editor;
        editor.link_suggest.then_some(editor.link_suggest_min_chars.max(1) as usize)
    }

    fn sweep(&self) {
        let mut entries = self.0.media.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|key, entry| {
            let keep = entry.used;
            if !keep {
                media::forget_key(&self.shared(key));
            }
            entry.used = false;
            keep
        });
    }

    fn request(&self, request: Request) {
        self.0.requests.lock().unwrap_or_else(|e| e.into_inner()).push(request);
        (self.0.wake)();
    }
}
