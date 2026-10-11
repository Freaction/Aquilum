use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{Inner, indexed};

#[derive(Default)]
pub(super) struct Links {
    known: HashMap<String, bool>,
    pending: HashSet<String>,
    unresolved: Arc<HashSet<String>>,
    stamp: u64,
}

impl Links {
    pub(super) fn unresolved(&self) -> (Arc<HashSet<String>>, u64) {
        (Arc::clone(&self.unresolved), self.stamp)
    }
}

pub(super) fn resolve(inner: &Arc<Inner>, targets: Vec<String>) {
    let links = inner.links.lock().unwrap_or_else(|e| e.into_inner());
    let fresh: HashSet<String> = targets.into_iter().filter(|t| !links.known.contains_key(t) && !links.pending.contains(t)).collect();
    drop(links);
    spawn(inner, fresh.into_iter().collect());
}

pub(super) fn refresh(inner: &Arc<Inner>) {
    let links = inner.links.lock().unwrap_or_else(|e| e.into_inner());
    let known = links.known.keys().filter(|t| !links.pending.contains(*t)).cloned().collect();
    drop(links);
    spawn(inner, known);
}

fn spawn(inner: &Arc<Inner>, targets: Vec<String>) {
    if targets.is_empty() {
        return;
    }
    inner.links.lock().unwrap_or_else(|e| e.into_inner()).pending.extend(targets.iter().cloned());
    let inner = Arc::clone(inner);
    std::thread::spawn(move || {
        let found = indexed(|| inner.core.search.resolve_wiki_links(&inner.workspace(), &inner.source(), &targets));
        let mut links = inner.links.lock().unwrap_or_else(|e| e.into_inner());
        for (i, target) in targets.into_iter().enumerate() {
            links.pending.remove(&target);
            let resolved = found.as_ref().map_or(true, |found| !found.complete || found.paths.get(i).is_some_and(Option::is_some));
            links.known.insert(target, resolved);
        }
        let unresolved: HashSet<String> = links.known.iter().filter(|(_, resolved)| !**resolved).map(|(target, _)| target.clone()).collect();
        if unresolved != *links.unresolved {
            links.unresolved = Arc::new(unresolved);
            links.stamp += 1;
            drop(links);
            inner.touch();
        }
    });
}
