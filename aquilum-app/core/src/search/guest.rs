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
        let _guard = self.prepare_lock.lock().map_err(SearchError::task)?;
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
            previous.index.stop();
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
        closed.index.stop();
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::search::models::IndexRevision;
    use crate::search::service::SearchService;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    struct Fixture {
        _directory: tempfile::TempDir,
        service: SearchService,
        announced: Arc<Mutex<Vec<String>>>,
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let root = crate::search::paths::canonical_path(directory.path());
            let announced = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&announced);
            let service = SearchService::new(
                root.join("indexes"),
                Arc::new(|_: Vec<PathBuf>| {}),
                Arc::new(move |event: IndexRevision| {
                    sink.lock().unwrap().push(event.workspace_path);
                }),
            );
            Self {
                _directory: directory,
                service,
                announced,
                root,
            }
        }

        fn workspace(&self, name: &str, note: &str) -> String {
            let path = self.root.join(name);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join(format!("{note}.md")), format!("# {note}\n\n{note} текст")).unwrap();
            path.to_string_lossy().into_owned()
        }

        fn wait_scanned(&self, workspace: &str) {
            let deadline = Instant::now() + Duration::from_secs(20);
            while self.service.status(Some(workspace)).updating {
                assert!(Instant::now() < deadline, "индекс {workspace} не догнал диск");
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        fn finds(&self, workspace: &str, word: &str) -> bool {
            self.service
                .search(workspace, &crate::search::query::Request::new(word))
                .is_ok_and(|response| !response.results.is_empty())
        }

        fn generation(&self) -> u64 {
            self.service
                .guest
                .read()
                .unwrap()
                .as_ref()
                .map(|guest| guest.index.generation)
                .expect("гостевой индекс открыт")
        }
    }

    #[test]
    fn a_guest_opens_beside_the_active_index_without_touching_it() {
        let fixture = Fixture::new();
        let home = fixture.workspace("home", "Домашняя");
        let other = fixture.workspace("other", "Чужая");
        fixture.service.prepare(&home).unwrap();
        fixture.wait_scanned(&home);

        fixture.service.open_guest(&other).unwrap();
        fixture.wait_scanned(&other);

        assert!(same(&fixture.service.active_root().unwrap(), &home));
        assert!(fixture.finds(&home, "домашняя"));
        assert!(fixture.finds(&other, "чужая"));
        assert!(!fixture.finds(&home, "чужая"), "базы не смешиваются");
        assert!(
            !fixture
                .announced
                .lock()
                .unwrap()
                .iter()
                .any(|announced| same(Path::new(announced), &other)),
            "интерфейс не слышит индекс гостя"
        );
    }

    #[test]
    fn a_guest_for_the_active_workspace_is_the_active_index() {
        let fixture = Fixture::new();
        let home = fixture.workspace("home", "Домашняя");
        fixture.service.prepare(&home).unwrap();
        fixture.service.open_guest(&home).unwrap();
        assert!(fixture.service.guest.read().unwrap().is_none());
    }

    #[test]
    fn a_third_workspace_replaces_the_second() {
        let fixture = Fixture::new();
        let second = fixture.workspace("second", "Вторая");
        let third = fixture.workspace("third", "Третья");
        fixture.service.open_guest(&second).unwrap();
        fixture.wait_scanned(&second);
        fixture.service.open_guest(&third).unwrap();
        fixture.wait_scanned(&third);

        assert!(fixture.finds(&third, "третья"));
        assert!(!fixture.finds(&second, "вторая"), "вторая база закрыта");
    }

    #[test]
    fn switching_to_the_guest_reuses_its_index() {
        let fixture = Fixture::new();
        let home = fixture.workspace("home", "Домашняя");
        let other = fixture.workspace("other", "Чужая");
        fixture.service.prepare(&home).unwrap();
        fixture.service.open_guest(&other).unwrap();
        let generation = fixture.generation();

        let status = fixture.service.prepare(&other).unwrap();

        assert_eq!(status.generation, generation, "индекс не открыт заново");
        assert!(fixture.service.guest.read().unwrap().is_none());
        assert!(same(&fixture.service.active_root().unwrap(), &other));
        fixture.wait_scanned(&other);
        assert!(fixture.finds(&other, "чужая"));
    }

    #[test]
    fn an_idle_guest_closes() {
        let fixture = Fixture::new();
        let other = fixture.workspace("other", "Чужая");
        fixture.service.open_guest(&other).unwrap();
        let generation = fixture.generation();

        assert!(fixture.service.close_if_idle(generation, Duration::from_secs(60)).is_some());
        assert!(fixture.service.close_if_idle(generation, Duration::ZERO).is_none());
        assert!(fixture.service.guest.read().unwrap().is_none());
        assert!(fixture.service.close_if_idle(generation, Duration::ZERO).is_none());
    }

    #[test]
    fn a_write_reaches_the_guest_index() {
        let fixture = Fixture::new();
        let other = fixture.workspace("other", "Чужая");
        fixture.service.open_guest(&other).unwrap();
        fixture.wait_scanned(&other);

        let note = Path::new(&other).join("Новая.md");
        fs::write(&note, "# Новая\n\nпродолжение").unwrap();
        fixture.service.notify_paths(vec![note]);

        let deadline = Instant::now() + Duration::from_secs(20);
        while !fixture.finds(&other, "продолжение") {
            assert!(Instant::now() < deadline, "правка не дошла до гостевого индекса");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn same(root: &Path, workspace: &str) -> bool {
        crate::search::paths::same_workspace(root, workspace)
    }
}
