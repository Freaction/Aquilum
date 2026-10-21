use super::line_diff::COARSE_MERGE_THRESHOLD;
use super::merge::MergeResult;
use super::{apply_edits, merge_external_change, TextEdit};

pub(super) struct Merged {
    pub(super) result: MergeResult,
    pub(super) text: String,
}

pub(super) fn merge(base: &str, disk: &str, current: &str) -> Merged {
    let result = merge_external_change(base, disk, current);
    let text = apply_edits(current, &result.edits);
    Merged { result, text }
}

#[test]
fn keeps_a_local_edit_in_a_different_paragraph_from_the_external_one() {
    let merged = merge(
        "первый\n\nвторой\n\nтретий\n",
        "первый\n\nвторой\n\nтретий переписан агентом\n",
        "первый абзац дополнен вручную\n\nвторой\n\nтретий\n",
    );
    assert_eq!(merged.text, "первый абзац дополнен вручную\n\nвторой\n\nтретий переписан агентом\n");
    assert!(merged.result.displaced.is_empty());
}

#[test]
fn produces_one_small_edit_per_changed_region() {
    let base = "a\nb\nc\nd\ne\nf\ng\n";
    let result = merge_external_change(base, "a\nB\nc\nd\ne\nf\nG\n", base);
    assert_eq!(result.edits.len(), 2);
    assert!(result.edits.iter().all(|edit| edit.to - edit.from < 4));
}

#[test]
fn applies_an_external_append_while_the_user_types_on_an_earlier_line() {
    let merged = merge(
        "заголовок\n\nтекст\n",
        "заголовок\n\nтекст\n\nдописано агентом\n",
        "заголовок\n\nтекст с добавкой\n",
    );
    assert_eq!(merged.text, "заголовок\n\nтекст с добавкой\n\nдописано агентом\n");
    assert!(merged.result.displaced.is_empty());
}

#[test]
fn reports_displaced_local_lines_when_both_sides_changed_the_same_line() {
    let merged = merge("один\nдва\nтри\n", "один\nдва от агента\nтри\n", "один\nдва от пользователя\nтри\n");
    assert_eq!(merged.text, "один\nдва от агента\nтри\n");
    assert_eq!(merged.result.displaced, vec!["два от пользователя".to_owned()]);
}

#[test]
fn does_nothing_when_the_file_did_not_move() {
    let base = "один\nдва\n";
    assert!(merge_external_change(base, base, "один\nдва изменённый\n").edits.is_empty());
}

#[test]
fn does_nothing_when_both_sides_already_agree() {
    assert!(merge_external_change("старое\n", "новое\n", "новое\n").edits.is_empty());
}

#[test]
fn handles_the_last_line_without_a_trailing_newline() {
    assert_eq!(merge("один\nдва", "один\nдва и хвост", "один\nдва").text, "один\nдва и хвост");
}

#[test]
fn handles_the_first_line() {
    assert_eq!(merge("один\nдва\n", "ОДИН\nдва\n", "один\nдва\n").text, "ОДИН\nдва\n");
}

#[test]
fn handles_deletion_of_a_middle_line() {
    assert_eq!(merge("a\nb\nc\n", "a\nc\n", "a\nb\nc\n").text, "a\nc\n");
}

#[test]
fn merges_an_external_deletion_with_an_unrelated_local_insertion() {
    assert_eq!(merge("a\nb\nc\nd\n", "a\nb\nd\n", "a\nb\nc\nd\nдобавлено\n").text, "a\nb\nd\nдобавлено\n");
}

#[test]
fn takes_the_whole_file_when_the_document_is_empty() {
    assert_eq!(merge("", "первая строка\nвторая\n", "").text, "первая строка\nвторая\n");
}

#[test]
fn keeps_the_header_on_top_when_an_empty_document_takes_a_file_with_a_blank_line() {
    let disk = "Дата: 26-03-2025\nТип: #учебник\n\n\n- тело\n\n***\n\nСсылки:\n- [[]]";
    assert_eq!(merge("", disk, "").text, disk);
}

#[test]
fn counts_positions_in_utf16_units_like_yjs() {
    let base = "😀 эмодзи\nкириллица\n𝄞 нота\n";
    let disk = "😀 эмодзи\nкириллица правка 🎉\n𝄞 нота\nхвост 👍\n";
    let current = "😀😀 эмодзи\nкириллица\n𝄞 нота\n";
    assert_eq!(merge(base, disk, current).text, "😀😀 эмодзи\nкириллица правка 🎉\n𝄞 нота\nхвост 👍\n");
}

#[test]
fn applies_edits_at_utf16_positions_in_any_order() {
    let edits = [TextEdit { from: 7, to: 8, insert: "Б".into() }, TextEdit { from: 0, to: 2, insert: "А".into() }];
    assert_eq!(apply_edits("😀 эмодзи", &edits), "А эмодБи");
}

fn divergent_disk() -> String {
    (0..COARSE_MERGE_THRESHOLD + 5).map(|index| format!("совсем другое {index}")).collect::<Vec<_>>().join("\n")
}

fn twenty_lines() -> String {
    (0..20).map(|index| format!("строка {index}")).collect::<Vec<_>>().join("\n")
}

#[test]
fn falls_back_to_a_coarse_merge_on_huge_divergence_and_says_so() {
    let base = twenty_lines();
    let disk = divergent_disk();
    let merged = merge(&base, &disk, &base);
    assert!(merged.result.coarse);
    assert_eq!(merged.text, disk);
}

#[test]
fn never_drops_local_lines_even_when_it_merges_coarsely() {
    let base = twenty_lines();
    let current = base.replace("строка 5", "моя правка");
    let merged = merge(&base, &divergent_disk(), &current);
    assert!(merged.result.coarse);
    assert!(merged.result.displaced.contains(&"моя правка".to_owned()));
}

#[test]
fn marks_an_ordinary_merge_as_precise() {
    assert!(!merge_external_change("a\nb\n", "a\nB\n", "a\nb\n").coarse);
}

#[test]
fn survives_a_document_that_lost_everything_locally() {
    assert!(merge("a\nb\nc\n", "a\nb\nc\nd\n", "").text.contains('d'));
}

#[test]
fn keeps_a_line_inserted_right_after_a_line_changed_elsewhere() {
    let merged = merge("a\nb\nc\n", "a\nB\nc\n", "a\nb\nX\nc\n");
    assert_eq!(merged.text, "a\nB\nX\nc\n");
}

#[test]
fn keeps_a_line_inserted_right_before_a_line_changed_elsewhere() {
    let merged = merge("a\nb\nc\n", "a\nB\nc\n", "a\nX\nb\nc\n");
    assert_eq!(merged.text, "a\nX\nB\nc\n");
}

#[test]
fn keeps_both_lines_inserted_at_the_same_place() {
    let merged = merge("a\nb\n", "a\nтам\nb\n", "a\nздесь\nb\n");
    assert!(merged.text.contains("там") && (merged.text.contains("здесь") || merged.result.displaced.contains(&"здесь".to_owned())));
}
