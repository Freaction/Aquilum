use crate::search::models::IndexRevision;
use crate::search::service::SearchService;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Fixture {
    service: SearchService,
    announced: Arc<Mutex<Vec<String>>>,
    root: PathBuf,
    _directory: tempfile::TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.service.close();
    }
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = crate::search::paths::canonical_path(directory.path());
        let announced = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&announced);
        let service = SearchService::new(
            root.join("indexes"),
            Arc::new(move |event: IndexRevision| {
                sink.lock().unwrap().push(event.workspace_path);
            }),
        );
        Self {
            service,
            announced,
            root,
            _directory: directory,
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
    fixture.service.queue(&[note], false);

    let deadline = Instant::now() + Duration::from_secs(20);
    while !fixture.finds(&other, "продолжение") {
        assert!(Instant::now() < deadline, "правка не дошла до гостевого индекса");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn same(root: &Path, workspace: &str) -> bool {
    crate::search::paths::same_workspace(root, workspace)
}
