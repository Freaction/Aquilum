use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct Generation(Arc<AtomicU64>);

impl Generation {
    pub fn next(&self) -> Cancel {
        let mine = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        Cancel(Some((Arc::clone(&self.0), mine)))
    }

    pub fn stop(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    pub fn current(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Default)]
pub struct Cancel(Option<(Arc<AtomicU64>, u64)>);

impl Cancel {
    pub fn never() -> Self {
        Self(None)
    }

    pub fn generation(&self) -> u64 {
        self.0.as_ref().map_or(0, |(_, mine)| *mine)
    }

    pub fn is_cancelled(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|(latest, mine)| latest.load(Ordering::Relaxed) != *mine)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cancel, Generation};

    #[test]
    fn a_newer_generation_cancels_the_older_one() {
        let generation = Generation::default();
        let first = generation.next();
        assert!(!first.is_cancelled());
        let second = generation.next();
        assert!(first.is_cancelled() && !second.is_cancelled());
        assert_eq!(second.generation(), generation.current());
        generation.stop();
        assert!(second.is_cancelled());
        assert!(!Cancel::never().is_cancelled());
    }
}
