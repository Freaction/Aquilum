use aquilum_core::Core;
use aquilum_core::files::document::hash_file;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

use aquilum_core::history::service::VersionView;

use super::{Change, PAGE_SIZE, diff, hunk_lines, page, restore, revert, view};

#[test]
fn diff_marks_added_and_removed_lines() {
    let lines = diff(Some("a\nb\nc"), "a\nB\nc\nd");
    let kinds: Vec<_> = lines.iter().map(|(k, l)| (*k, l.as_str())).collect();
    assert_eq!(
        kinds,
        [(Change::Same, "a"), (Change::Removed, "b"), (Change::Added, "B"), (Change::Same, "c"), (Change::Added, "d")]
    );
}

fn with_versions(initial: &str, edits: [(&str, Source); 2]) -> (tempfile::TempDir, tempfile::TempDir, std::sync::Arc<Core>, std::path::PathBuf) {
    let vault = crate::test_dir::tempdir();
    let data = crate::test_dir::tempdir();
    let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
    core.ui_state.resolve_workspace(&vault.path().to_string_lossy(), 0).unwrap();
    let note = vault.path().join("a.md");
    std::fs::write(&note, initial).unwrap();
    for (text, source) in edits {
        let hash = hash_file(&note).unwrap();
        gate::write(&core, &note, text, Some(&hash), source, None).unwrap();
    }
    (vault, data, core, note)
}

#[test]
fn restore_and_revert_rewrite_note() {
    let (_vault, _data, core, note) = with_versions("один\nдва\n", [("один\nдва\nтри\n", Source::Me), ("ноль\nодин\nдва\nтри\n", Source::Agent)]);
    let versions = page(&core, &note, None, PAGE_SIZE).versions;
    let with_three = versions
        .iter()
        .find(|v| matches!(view(&core, &note, &v.id), Some(VersionView::Full(texts)) if texts.text == "один\nдва\nтри\n"))
        .unwrap_or_else(|| panic!("версия с «три»; версий {}", versions.len()));
    assert!(revert(&core, &note, with_three).unwrap());
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "ноль\nодин\nдва\n");
    assert!(restore(&core, &note, with_three).unwrap());
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "один\nдва\nтри\n");
    core.shutdown();
}

#[test]
fn a_big_version_shows_only_its_changed_lines() {
    let hunk = aquilum_core::history::changes::Hunk {
        before: "раз\nдва ".to_owned(),
        removed: "три".to_owned(),
        added: "ТРИ".to_owned(),
        after: " конец\nчетыре\n".to_owned(),
    };
    let lines = hunk_lines(&[hunk]);
    let kinds: Vec<_> = lines.iter().map(|(k, l)| (*k, l.as_str())).collect();
    assert_eq!(kinds, [(Change::Same, "раз"), (Change::Removed, "два три конец"), (Change::Added, "два ТРИ конец"), (Change::Same, "четыре")]);
}

#[test]
fn a_revert_over_a_later_edit_keeps_the_later_text_in_a_conflict_copy() {
    let (vault, _data, core, note) = with_versions("раз\nдва\nтри\nчетыре\n", [("раз\nДВА\nтри\nчетыре\n", Source::Me), ("раз\nДВА и ещё\nтри\nчетыре!\n", Source::Agent)]);
    let versions = page(&core, &note, None, PAGE_SIZE).versions;
    let changed = versions.iter().find(|v| v.source == Source::Me).expect("версия с ДВА");
    assert!(revert(&core, &note, changed).unwrap());
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "раз\nдва\nтри\nчетыре!\n");
    let copy = std::fs::read_dir(vault.path()).unwrap().flatten().map(|e| e.path()).find(|p| p.to_string_lossy().contains("конфликт")).expect("копия-конфликт");
    assert!(std::fs::read_to_string(copy).unwrap().contains("ДВА и ещё"));
    core.shutdown();
}
