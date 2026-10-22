use super::error::SearchError;
use super::models::SearchIndexStatus;
use super::paths::same_path;
use super::service::{workspace_root, OpenIndex, SearchService};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const IDLE_LIMIT: Duration = Duration::from_secs(300);

pub struct Guest {
    pub index: OpenIndex,
    last_used: Mutex<Instant>,
}

impl Guest {
    pub fn touch(&self) {
        if let Ok(mut last_used) = self.last_used.lock() {
            *last_used = Instant::now();
        }
    }

    fn idle(&self) -> Duration {
        self.last_used
            .lock()
            .map(|last_used| last_used.elapsed())
            .unwrap_or_default()
    }
}

impl SearchService {
    pub fn open_guest(&self, workspace: &str) -> Result<SearchIndexStatus, SearchError> {
        let _opening = self.opening()?;
        let root = workspace_root(workspace)?;
        if let Some(status) = self.status_for_root(&root) {
            return Ok(status);
        }
        if let Some(status) = self.guest_status(&root)? {
            return Ok(status);
        }
        let previous = self.guest.write().map_err(SearchError::task)?.take();
        if let Some(previous) = previous {
            eprintln!("[aquilum:guest] закрыта {}: агенту нужна другая база", previous.index.root.display());
        }
        let started = Instant::now();
        let opened = self.open_index(root, false)?;
        eprintln!(
            "[aquilum:guest] открыта {} за {} мс",
            opened.root.display(),
            started.elapsed().as_millis()
        );
        let status = opened.progress.snapshot();
        let generation = opened.generation;
        *self.guest.write().map_err(SearchError::task)? = Some(Guest {
            index: opened,
            last_used: Mutex::new(Instant::now()),
        });
        self.close_when_idle(generation);
        Ok(status)
    }

    pub fn take_guest(&self, root: &Path) -> Result<Option<OpenIndex>, SearchError> {
        let mut guest = self.guest.write().map_err(SearchError::task)?;
        if guest.as_ref().is_some_and(|guest| same_path(&guest.index.root, root)) {
            return Ok(guest.take().map(|guest| guest.index));
        }
        Ok(None)
    }

    fn guest_status(&self, root: &Path) -> Result<Option<SearchIndexStatus>, SearchError> {
        let guest = self.guest.read().map_err(SearchError::task)?;
        Ok(guest
            .as_ref()
            .filter(|guest| same_path(&guest.index.root, root))
            .map(|guest| {
                guest.touch();
                guest.index.progress.snapshot()
            }))
    }

    fn close_when_idle(&self, generation: u64) {
        let service = self.clone();
        let spawned = std::thread::Builder::new()
            .name("aquilum-guest-idle".to_owned())
            .spawn(move || {
                while let Some(wait) = service.close_if_idle(generation, IDLE_LIMIT) {
                    std::thread::sleep(wait);
                }
            });
        if let Err(error) = spawned {
            eprintln!("[aquilum:guest] таймер простоя не запущен: {error}");
        }
    }

    fn close_if_idle(&self, generation: u64, limit: Duration) -> Option<Duration> {
        let _guard = self.prepare_lock.lock().ok()?;
        let closed = {
            let mut slot = self.guest.write().ok()?;
            let guest = slot
                .as_ref()
                .filter(|guest| guest.index.generation == generation)?;
            let idle = guest.idle();
            if idle < limit {
                return Some(limit - idle);
            }
            slot.take()?
        };
        eprintln!(
            "[aquilum:guest] закрыта {} после {} с простоя",
            closed.index.root.display(),
            limit.as_secs()
        );
        None
    }
}

#[cfg(test)]
#[path = "guest_tests.rs"]
mod tests;
