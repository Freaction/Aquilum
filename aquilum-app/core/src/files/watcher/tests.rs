use super::*;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

fn collector() -> (WatchSink, mpsc::Receiver<WatchBatch>) {
    let (sender, receiver) = mpsc::channel();
    let sink: WatchSink = Arc::new(move |batch| {
        let _ = sender.send(batch);
    });
    (sink, receiver)
}

#[test]
fn refuses_to_start_on_a_missing_folder_instead_of_dying_quietly() {
    let (sink, _receiver) = collector();
    let watcher = WorkspaceWatcher::new(sink);

    let error = watcher
        .watch(Path::new("D:/такой/папки/нет/и/не/будет"))
        .unwrap_err();

    assert!(error.contains("не удалось наблюдать"));
}

#[test]
fn delivers_a_real_file_change_to_the_sink() {
    let directory = tempfile::tempdir().unwrap();
    let (sink, receiver) = collector();
    let watcher = WorkspaceWatcher::new(sink);
    watcher.watch(directory.path()).unwrap();

    std::fs::write(directory.path().join("Заметка.md"), "текст").unwrap();

    let batch = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("вотчер обязан доставить изменение файла");
    assert!(batch
        .paths
        .iter()
        .any(|path| path.file_name().is_some_and(|name| name == "Заметка.md")));
    watcher.close();
}

#[test]
fn retargeting_moves_the_watch_to_the_new_workspace() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let (sink, receiver) = collector();
    let watcher = WorkspaceWatcher::new(sink);
    watcher.watch(first.path()).unwrap();
    watcher.watch(second.path()).unwrap();

    std::fs::write(second.path().join("Вторая.md"), "текст").unwrap();
    let batch = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("новая база обязана наблюдаться");
    assert!(batch
        .paths
        .iter()
        .any(|path| path.file_name().is_some_and(|name| name == "Вторая.md")));

    std::fs::write(first.path().join("Первая.md"), "текст").unwrap();
    assert!(
        matches!(
            receiver.recv_timeout(Duration::from_millis(800)),
            Err(RecvTimeoutError::Timeout)
        ),
        "старая база больше не наблюдается"
    );
    watcher.close();
}

#[test]
fn close_does_not_wait_for_a_poll_interval() {
    let directory = tempfile::tempdir().unwrap();
    let (sink, _receiver) = collector();
    let watcher = WorkspaceWatcher::new(sink);
    watcher.watch(directory.path()).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let started = std::time::Instant::now();
    watcher.close();

    assert!(started.elapsed() < Duration::from_millis(500), "остановка ждала {:?}", started.elapsed());
}

#[test]
fn a_closed_watcher_delivers_nothing_and_does_not_start_again() {
    let directory = tempfile::tempdir().unwrap();
    let (sink, receiver) = collector();
    let watcher = WorkspaceWatcher::new(sink);
    watcher.watch(directory.path()).unwrap();
    watcher.close();

    std::fs::write(directory.path().join("После.md"), "текст").unwrap();

    assert!(matches!(
        receiver.recv_timeout(Duration::from_millis(800)),
        Err(RecvTimeoutError::Timeout)
    ));
    assert!(watcher.watch(directory.path()).is_err(), "закрытый наблюдатель не запускается снова");
}
