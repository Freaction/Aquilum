use crate::search::SearchService;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn base_rows_distinguish_warming_index_from_ready_empty_workspace() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("vault");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_str().unwrap();
    let service = SearchService::new(
        directory.path().join("index"),
        Arc::new(|_| {}),
        Arc::new(|_| {}),
    );
    assert!(service.base_rows(workspace).unwrap().is_none());
    service.prepare(workspace).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while service.base_rows(workspace).unwrap().is_none() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(service.base_rows(workspace).unwrap().unwrap().is_empty());
    service
        .with_index(workspace, |open| {
            open.progress.begin_scan();
            Ok(())
        })
        .unwrap();
    assert!(service.base_rows(workspace).unwrap().is_none());
    service
        .with_index(workspace, |open| {
            open.progress.completed(0);
            Ok(())
        })
        .unwrap();
    assert!(service.base_rows(workspace).unwrap().unwrap().is_empty());
}

#[test]
fn malformed_frontmatter_does_not_abort_index_and_reports_a_visible_base_error() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("vault");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(workspace.join("bad.md"), "---\nx: [broken\n---\nbody\n").unwrap();
    std::fs::write(workspace.join("good.md"), "---\nrank: \"2\"\n---\nbody\n").unwrap();
    let service = SearchService::new(
        directory.path().join("index"),
        Arc::new(|_| {}),
        Arc::new(|_| {}),
    );
    let workspace = workspace.to_str().unwrap();
    service.prepare(workspace).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let status = service.status(Some(workspace));
        if status.state == crate::search::models::SearchIndexState::Ready && !status.updating {
            assert_eq!(status.indexed_documents, 2);
            assert!(status.error.is_none());
            break;
        }
        assert!(Instant::now() < until, "{:?}", status.state);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(
        service.base_rows(workspace),
        Err(crate::search::error::SearchError::Query { .. })
    ));
}
