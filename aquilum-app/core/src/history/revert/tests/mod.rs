use super::{apply, as_map, between, windows, Map};
use crate::history::delta::{compose, Region};
use crate::text::content::{collect, Splice};

mod random;

pub(crate) struct Edited {
    pub(crate) text: String,
    pub(crate) regions: Vec<Region>,
}

impl Edited {
    pub(crate) fn new(text: &str) -> Self {
        Self { text: text.to_owned(), regions: Vec::new() }
    }

    fn splice(&mut self, at: usize, removed: usize, insert: &str) {
        self.text.replace_range(at..at + removed, insert);
        self.regions = compose(&self.regions, Splice { at, removed, inserted: insert.len() });
    }

    pub(crate) fn replace(&mut self, old: &str, new: &str) {
        let at = self.text.find(old).unwrap_or_else(|| panic!("в тексте нет {old:?}"));
        let same = |old: &mut dyn Iterator<Item = char>, new: &mut dyn Iterator<Item = char>| old.zip(new).take_while(|(a, b)| a == b).map(|(a, _)| a.len_utf8()).sum::<usize>();
        let head = same(&mut old.chars(), &mut new.chars());
        let tail = same(&mut old[head..].chars().rev(), &mut new[head..].chars().rev());
        self.splice(at + head, old.len() - head - tail, &new[head..new.len() - tail]);
    }

    fn done(self) -> (String, Map) {
        (self.text, as_map(&self.regions))
    }
}

fn edited(base: &str, edits: &[(&str, &str)]) -> (String, Map) {
    let mut edited = Edited::new(base);
    for (old, new) in edits {
        edited.replace(old, new);
    }
    edited.done()
}

fn undo(previous: &str, own: &Map, version: &str, later: &Map, current: &str) -> (String, Vec<String>) {
    let found = windows(&version, own, later).unwrap();
    let reverted = apply(&found, &previous, &version, Box::new(current.to_owned())).unwrap();
    (collect(&reverted.text), reverted.displaced)
}

fn revert(previous: &str, version_edits: &[(&str, &str)], later_edits: &[(&str, &str)]) -> (String, Vec<String>) {
    let (version, own) = edited(previous, version_edits);
    let (current, later) = edited(&version, later_edits);
    undo(previous, &own, &version, &later, &current)
}

#[test]
fn a_deleted_whole_line_comes_back() {
    let (text, _) = revert("раз\nдва\nтри\n", &[("два\n", "")], &[]);
    assert_eq!(text, "раз\nдва\nтри\n", "отмена удаления целой строки возвращает строку");
}

#[test]
fn a_deleted_line_start_comes_back() {
    let (text, _) = revert("раз\nПривет, мир\nтри\n", &[("Привет, ", "")], &[]);
    assert_eq!(text, "раз\nПривет, мир\nтри\n", "отмена удаления начала строки возвращает начало");
}

#[test]
fn a_deleted_first_line_comes_back() {
    let (text, _) = revert("первая\nвторая\n", &[("первая\n", "")], &[]);
    assert_eq!(text, "первая\nвторая\n", "отмена удаления первой строки возвращает её");
}

#[test]
fn a_deleted_last_line_comes_back() {
    let (text, _) = revert("раз\nдва\nконец", &[("\nконец", "")], &[]);
    assert_eq!(text, "раз\nдва\nконец", "отмена удаления хвоста без перевода строки");
    let (text, _) = revert("раз\nдва\nконец\n", &[("конец\n", "")], &[]);
    assert_eq!(text, "раз\nдва\nконец\n", "отмена удаления последней строки");
}

#[test]
fn an_emptied_line_gets_its_text_back() {
    let (text, _) = revert("раз\nдва\nтри\n", &[("два", "")], &[]);
    assert_eq!(text, "раз\nдва\nтри\n", "отмена стирания текста строки оставляет строку с текстом");
}

#[test]
fn an_inserted_line_and_a_changed_word_go_away() {
    let (text, _) = revert("раз\nтри\n", &[("раз\n", "раз\nдва\n")], &[]);
    assert_eq!(text, "раз\nтри\n", "отмена вставки строки");
    let (text, _) = revert("кошка спит на окне\n", &[("спит", "сидит")], &[]);
    assert_eq!(text, "кошка спит на окне\n", "отмена правки слова в середине строки");
}

#[test]
fn every_region_of_one_version_is_undone_once() {
    let previous = "а\nб\nв\nг\nд\nе\n";
    let edits = [("а\n", "А\n"), ("б\n", ""), ("в\n", "В\n"), ("д\n", "д\nвставка\n"), ("е\n", "")];
    let (text, displaced) = revert(previous, &edits, &[]);
    assert_eq!(text, previous, "несколько участков одной версии, в том числе соседние, отменяются ровно один раз");
    assert!(displaced.is_empty());
}

#[test]
fn later_edits_elsewhere_survive() {
    let previous = "раз\nдва\nтри\nчетыре\nпять\n";
    let (text, displaced) = revert(previous, &[("два\n", "")], &[("раз", "РАЗ"), ("пять", "ПЯТЬ"), ("три\n", "три\nтри с половиной\n")]);
    assert_eq!(text, "РАЗ\nдва\nтри\nтри с половиной\nчетыре\nПЯТЬ\n", "поздние правки других строк остаются");
    assert!(displaced.is_empty());
}

#[test]
fn a_later_insertion_where_the_line_was_deleted_stays() {
    let (text, displaced) = revert("раз\nдва\nтри\n", &[("два\n", "")], &[("раз\n", "раз\nновая\n")]);
    assert_eq!(text, "раз\nдва\nновая\nтри\n", "поздняя вставка на месте удалённой строки остаётся, строка возвращается");
    assert!(displaced.is_empty());
}

#[test]
fn a_line_typed_again_where_it_was_deleted_is_not_doubled() {
    let (text, displaced) = revert("раз\nдва\nтри\n", &[("два\n", "")], &[("раз\n", "раз\nдва\n")]);
    assert_eq!(text, "раз\nдва\nтри\n", "строка, уже набранная заново на своём месте, не удваивается");
    assert!(displaced.is_empty());
    let (text, _) = revert("а\nб\nв\n", &[("а", "А"), ("б\n", "")], &[("А\n", "А\nб\n")]);
    assert_eq!(text, "а\nб\nв\n", "соседняя правка версии отменяется, а набранная заново строка не удваивается");
}

#[test]
fn later_lines_right_next_to_the_reverted_one_stay_in_place() {
    let (text, displaced) = revert("раз\nдва\nтри\n", &[("два", "ДВА")], &[("раз\n", "раз\nперед\n"), ("ДВА\n", "ДВА\nпосле\n")]);
    assert_eq!(text, "раз\nперед\nдва\nпосле\nтри\n", "строки, вставленные позже вплотную к отменяемой, остаются на месте");
    assert!(displaced.is_empty());
}

#[test]
fn a_later_edit_of_the_same_line_loses_to_the_revert_but_is_kept_aside() {
    let (text, displaced) = revert("раз\nдва\nтри\n", &[("два", "ДВА")], &[("ДВА", "ДВА и ещё")]);
    assert_eq!(text, "раз\nдва\nтри\n", "отмена побеждает позднюю правку той же строки");
    assert_eq!(displaced, ["ДВА и ещё"], "поздний текст уходит в копию");
}

#[test]
fn a_line_deleted_later_comes_back_as_it_was_before_the_version() {
    let (text, displaced) = revert("раз\nдва\nтри\n", &[("два", "ДВА")], &[("ДВА\n", "")]);
    assert_eq!(text, "раз\nдва\nтри\n", "строка, удалённая позже, возвращается в прежнем виде");
    assert!(displaced.is_empty());
}

#[test]
fn multibyte_characters_are_never_split() {
    let previous = "жук 🐞 и ёж\nзмея 🐍\n";
    let version = "жук 🐝 и ёж\nзмея 🐍\n";
    let own = between(&previous, &version).unwrap();
    let current = "жук 🐝 и ёж\nзмея 🐍 и 🦎\n";
    let later = between(&version, &current).unwrap();
    let (text, _) = undo(previous, &own, version, &later, current);
    assert_eq!(text, "жук 🐞 и ёж\nзмея 🐍 и 🦎\n", "отмена правки эмодзи при побайтовых участках не ломает символы");
    assert!(!text.contains('\u{FFFD}'));
}

#[test]
fn crlf_lines_are_restored_with_their_endings() {
    let (text, _) = revert("раз\r\nдва\r\nтри\r\n", &[("два\r\n", "")], &[("три", "ТРИ")]);
    assert_eq!(text, "раз\r\nдва\r\nТРИ\r\n", "строки с CRLF возвращаются с тем же концом строки");
}

#[test]
fn an_emptied_and_a_filled_note_are_reverted() {
    let (text, _) = revert("весь текст\nзаметки\n", &[("весь текст\nзаметки\n", "")], &[]);
    assert_eq!(text, "весь текст\nзаметки\n", "отмена очистки заметки");
    let (text, _) = revert("", &[("", "новый текст\n")], &[]);
    assert_eq!(text, "", "отмена первого наполнения пустой заметки");
}
