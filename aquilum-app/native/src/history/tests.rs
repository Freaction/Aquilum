use aquilum_core::Core;
use aquilum_core::files::document::read_file_snapshot_impl;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

use super::{Change, PAGE_SIZE, diff, page, restore, revert, texts};

#[test]
fn diff_marks_added_and_removed_lines() {
    let lines = diff(Some("a\nb\nc"), "a\nB\nc\nd");
    let kinds: Vec<_> = lines.iter().map(|(k, l)| (*k, l.as_str())).collect();
    assert_eq!(
        kinds,
        [(Change::Same, "a"), (Change::Removed, "b"), (Change::Added, "B"), (Change::Same, "c"), (Change::Added, "d")]
    );
}

#[test]
fn restore_and_revert_rewrite_note() {
    let vault = crate::test_dir::tempdir();
    let data = crate::test_dir::tempdir();
    let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
    core.ui_state.resolve_workspace(&vault.path().to_string_lossy(), 0).unwrap();
    let note = vault.path().join("a.md");
    std::fs::write(&note, "один\nдва\n").unwrap();
    for (text, source) in [("один\nдва\nтри\n", Source::Me), ("ноль\nодин\nдва\nтри\n", Source::Agent)] {
        let hash = read_file_snapshot_impl(&note).unwrap().hash;
        gate::write(&core, &note, text, Some(&hash), source, None).unwrap();
    }
    let versions = page(&core, &note, PAGE_SIZE).versions;
    let with_three = versions
        .iter()
        .find(|v| texts(&core, &note, &v.id).is_some_and(|t| t.text == "один\nдва\nтри\n"))
        .unwrap_or_else(|| panic!("версия с «три»; версий {}", versions.len()));
    assert!(revert(&core, &note, with_three).unwrap());
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "ноль\nодин\nдва\n");
    assert!(restore(&core, &note, with_three).unwrap());
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "один\nдва\nтри\n");
    core.shutdown();
}
