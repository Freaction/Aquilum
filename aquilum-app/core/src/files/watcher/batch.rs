use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const BATCH_WINDOW: Duration = Duration::from_millis(80);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum WatchScope {
    Content,
    Listing,
    Structure,
    Rescan,
}

#[derive(Debug)]
pub struct WatchBatch {
    pub paths: Vec<PathBuf>,
    pub scope: WatchScope,
}

impl WatchBatch {
    pub fn new(paths: Vec<PathBuf>, scope: WatchScope) -> Self {
        Self { paths, scope }
    }

    pub fn rescan() -> Self {
        Self::new(Vec::new(), WatchScope::Rescan)
    }
}

pub fn coalesce(first: WatchBatch, receiver: &mpsc::Receiver<WatchBatch>) -> WatchBatch {
    let deadline = Instant::now() + BATCH_WINDOW;
    let mut paths = first.paths.into_iter().collect::<HashSet<_>>();
    let mut scope = first.scope;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let Ok(next) = receiver.recv_timeout(remaining) else {
            break;
        };
        paths.extend(next.paths);
        scope = scope.max(next.scope);
    }
    WatchBatch::new(paths.into_iter().collect(), scope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeper_scope_wins_over_a_shallower_one() {
        assert!(WatchScope::Rescan > WatchScope::Structure);
        assert!(WatchScope::Structure > WatchScope::Listing);
        assert!(WatchScope::Listing > WatchScope::Content);
        assert!(WatchScope::Structure > WatchScope::Content);
    }

    #[test]
    fn merges_paths_and_keeps_the_deepest_scope() {
        let (sender, receiver) = mpsc::channel();
        sender
            .send(WatchBatch::new(vec![PathBuf::from("b.md")], WatchScope::Structure))
            .unwrap();
        sender
            .send(WatchBatch::new(vec![PathBuf::from("a.md")], WatchScope::Content))
            .unwrap();
        drop(sender);

        let merged = coalesce(
            WatchBatch::new(vec![PathBuf::from("a.md")], WatchScope::Content),
            &receiver,
        );

        assert_eq!(merged.scope, WatchScope::Structure);
        assert_eq!(merged.paths.len(), 2, "одинаковые пути схлопываются");
    }

    #[test]
    fn returns_the_first_batch_when_nothing_else_arrives() {
        let (_sender, receiver) = mpsc::channel::<WatchBatch>();
        let merged = coalesce(
            WatchBatch::new(vec![PathBuf::from("a.md")], WatchScope::Content),
            &receiver,
        );
        assert_eq!(merged.scope, WatchScope::Content);
        assert_eq!(merged.paths, vec![PathBuf::from("a.md")]);
    }
}
