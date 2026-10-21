use super::*;
use crate::text::content::collect;

const MOSCOW: i64 = 3 * 3600;
const AUGUST_17_0530_UTC: i64 = 1_786_944_600 * 1_000_000_000;

fn moment() -> LocalMinute {
    LocalMinute::at(AUGUST_17_0530_UTC, MOSCOW)
}

fn report(cause: ConflictCause) -> ConflictReport {
    ConflictReport { cause, synced_hash: Some("hash:сверка".into()) }
}

fn body(content: &str) -> &str {
    let marker = "## Текст, который не попал в заметку\n";
    &content[content.find(marker).expect("marker") + marker.len()..]
}

#[test]
fn renders_the_local_minute_of_the_conflict() {
    let moment = moment();
    assert_eq!((moment.date.as_str(), moment.time.as_str()), ("2026-08-17", "08:30"));
}

#[test]
fn names_the_copy_with_a_readable_stamp() {
    assert_eq!(copy_stem("Идея", &moment(), "a1b2c3d4"), "Идея (конфликт 2026-08-17 08-30 a1b2c3d4)");
}

#[test]
fn creates_nothing_for_text_that_carries_nothing() {
    assert!(copy_content("D:/База/Идея.md", "Идея", &report(ConflictCause::Diverged), &moment(), &"   \n\n").is_none());
}

#[test]
fn keeps_the_text_and_ends_it_with_exactly_one_newline() {
    let with = collect(&copy_content("p", "Идея", &report(ConflictCause::Diverged), &moment(), &"строка\n").expect("copy"));
    let without = collect(&copy_content("p", "Идея", &report(ConflictCause::Diverged), &moment(), &"мой текст").expect("copy"));
    assert_eq!(body(&with), "строка\n");
    assert_eq!(body(&without), "мой текст\n");
}

#[test]
fn names_the_note_the_moment_and_the_reason() {
    let content =
        collect(&copy_content("D:/База/Идея.md", "Идея", &report(ConflictCause::Diverged), &moment(), &"текст").expect("copy"));
    for expected in [
        "конфликт: 2026-08-17 08:30",
        "заметка: \"Идея\"",
        "причина: \"файл и документ изменились одновременно\"",
        "`D:/База/Идея.md`",
        "`hash:сверка`",
    ] {
        assert!(content.contains(expected), "missing {expected}");
    }
}

#[test]
fn explains_each_cause_in_its_own_words() {
    let render = |cause| collect(&copy_content("p", "Идея", &report(cause), &moment(), &"текст").expect("copy"));
    let displaced = render(ConflictCause::Displaced);
    assert!(displaced.contains("строки не поместились при слиянии") && displaced.contains("вытеснило строки ниже"));
    let reverted = render(ConflictCause::Reverted);
    assert!(reverted.contains("причина: \"изменения версии отменены поверх более поздних правок\""));
}

#[test]
fn leaves_out_a_fingerprint_it_was_not_given() {
    let bare = ConflictReport { cause: ConflictCause::Diverged, synced_hash: None };
    let content = collect(&copy_content("p", "Идея", &bare, &moment(), &"текст").expect("copy"));
    assert!(!content.contains("Отпечаток"));
}
