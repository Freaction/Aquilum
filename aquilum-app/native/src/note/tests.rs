use super::*;

fn opened(text: &str) -> (tempfile::TempDir, Note) {
    let dir = crate::test_dir::tempdir();
    let path = dir.path().join("a.md");
    std::fs::write(&path, text).unwrap();
    let (note, _) = Note::open(path).unwrap();
    (dir, note)
}

#[test]
fn reload_ignores_own_write_and_sees_external_one() {
    let (_dir, mut note) = opened("раз");
    assert_eq!(note.reload(), None);
    std::fs::write(&note.path, "другой текст").unwrap();
    assert_eq!(note.reload().map(|text| text.to_string()).as_deref(), Some("другой текст"));
    note.edit();
    assert!(note.deadline().is_some());
    assert_eq!(note.reload(), None, "несохранённая правка не затирается");
}

#[test]
fn typing_is_written_after_a_short_pause() {
    let (_dir, mut note) = opened("раз");
    let before = Instant::now();
    note.edit();
    let due = note.deadline().unwrap();
    assert!(due >= before + QUIET && due <= Instant::now() + QUIET, "запись через {QUIET:?} после правки");
    note.save_soon();
    assert!(note.deadline().unwrap() <= Instant::now(), "правка извне записывается сразу");
}

#[test]
fn nonstop_typing_is_still_written_within_the_longest_wait() {
    let (_dir, mut note) = opened("раз");
    let first = Instant::now() - LONGEST + Duration::from_millis(500);
    note.since = Some(first);
    note.edit();
    assert_eq!(note.deadline(), Some(first + LONGEST), "непрерывный набор не откладывает запись дольше {LONGEST:?}");
}
