use super::parse::parse;
use super::Request;
use crate::search::index_tests::{indexed, search};
use crate::search::models::SearchResult;

fn names(results: &[SearchResult]) -> Vec<&str> {
    results.iter().map(|result| result.title.as_str()).collect()
}

fn sorted(results: &[SearchResult]) -> Vec<&str> {
    let mut names = names(results);
    names.sort_unstable();
    names
}

#[test]
fn the_syntax_marks_headings_phrases_exclusions_and_the_open_last_word() {
    let parsed = parse("#заголовок \"точная фраза\" -минус слово");
    let words: Vec<_> = parsed.words.iter().map(|word| (word.stem.as_str(), word.headings, word.exact, word.prefix)).collect();
    assert_eq!(words, [("заголовок", true, false, false), ("точн", false, true, false), ("фраз", false, true, false), ("слов", false, false, true)]);
    assert_eq!(parsed.phrases, [["точн", "фраз"]]);
    assert_eq!(parsed.excluded, [["минус"]]);
    for odd in ["", "-", "\"", "#", "-\"\"", "a\"b", "- кот", "кот кот", "\"незакрытая фраза"] {
        assert!(parse(odd).words.len() <= 2, "разбор не падает и не плодит слов: {odd:?}");
    }
    assert_eq!(parse("кот кот").words.len(), 1, "повтор слова не меняет запрос");
}

#[test]
fn all_words_rank_above_one_word_even_in_the_title_and_repeated() {
    let (_guard, index) = indexed(&[("Поиск.md", &"поиск ".repeat(10)), ("Дневник.md", "Сегодня делал поиск заметок в базе.")]);
    assert_eq!(names(&search(&index, "поиск заметок")), ["Дневник", "Поиск"]);
}

#[test]
fn a_typo_is_counted_and_highlighted_but_ranks_below_the_exact_form() {
    let (_guard, index) = indexed(&[("Заметка.md", "Короткий текст."), ("Список.md", "Много заметок в списке.")]);
    let found = index.search(&Request::new("заметок")).unwrap();
    assert_eq!(names(&found.results), ["Список", "Заметка"]);
    assert_eq!((found.results[1].match_count, found.results[1].matched_terms.as_slice()), (1, ["заметок".to_owned()].as_slice()));
    assert!(found.terms.contains(&"заметк".to_owned()), "форма с опечаткой подсвечивается: {:?}", found.terms);
}

#[test]
fn every_result_has_a_counted_match_and_folder_names_do_not_match() {
    let (_guard, index) = indexed(&[
        ("Замеры/Свет.md", "Один источник на сцену."),
        ("Заметка.md", "Шаблон."),
        ("Задержка.md", "Между сознанием и подсознанием."),
        ("Кадры.md", "Каждый кадр и замер времени frame."),
    ]);
    let results = search(&index, "кадр замер frame");
    assert_eq!(names(&results), ["Кадры"]);
    assert!(results.iter().all(|result| result.match_count > 0 && !result.matched_terms.is_empty()));
}

#[test]
fn typos_need_five_letters_and_the_same_first_letter() {
    let (_guard, index) = indexed(&[("Кит.md", "кит"), ("Машина.md", "машина")]);
    assert!(search(&index, "кот").is_empty(), "короткое слово ищется без опечаток");
    assert!(search(&index, "вашина").is_empty(), "опечатка в первой букве не допускается");
    assert_eq!(names(&search(&index, "мошина")), ["Машина"]);
}

#[test]
fn only_the_last_word_is_a_prefix() {
    let (_guard, index) = indexed(&[("Обучение.md", "машинное обучение"), ("Маш.md", "маш и обучение")]);
    let results = search(&index, "маш обуч");
    assert_eq!(names(&results), ["Маш", "Обучение"]);
    assert_eq!(results[1].matched_terms, ["обуч"]);
    assert_eq!(names(&search(&index, "маш")), ["Маш", "Обучение"], "последнее слово ищется и как начало");
}

#[test]
fn the_title_beats_headings_and_headings_beat_the_body() {
    let (_guard, index) = indexed(&[("Заметки.md", "rust rust rust каждый день."), ("Языки.md", "# Rust\n\nТекст."), ("Rust.md", "Про язык.")]);
    assert_eq!(names(&search(&index, "rust")), ["Rust", "Языки", "Заметки"]);
}

#[test]
fn a_title_equal_to_the_query_comes_first() {
    let (_guard, index) = indexed(&[("Rust память.md", "rust rust rust"), ("Rust.md", "x")]);
    assert_eq!(names(&search(&index, "rust")), ["Rust", "Rust память"]);
}

#[test]
fn a_quoted_phrase_needs_the_words_side_by_side_in_any_form() {
    let (_guard, index) = indexed(&[
        ("Рядом.md", "Нужен быстрый поиск по базе."),
        ("Склонение.md", "Ради быстрого поиска."),
        ("Врозь.md", "Поиск бывает разный, но быстрый лучше."),
    ]);
    assert_eq!(sorted(&search(&index, "\"быстрый поиск\"")), ["Рядом", "Склонение"]);
    assert_eq!(search(&index, "\"быстрый поиск").len(), 2, "незакрытая кавычка — фраза до конца строки");
}

#[test]
fn a_minus_excludes_a_word_or_a_phrase() {
    let (_guard, index) = indexed(&[("Кофе.md", "кофе"), ("Утро.md", "кофе и чай"), ("Вечер.md", "кофе, зелёный чай")]);
    assert_eq!(names(&search(&index, "кофе -чай")), ["Кофе"]);
    assert_eq!(sorted(&search(&index, "кофе -\"зелёный чай\"")), ["Кофе", "Утро"]);
}

#[test]
fn a_hash_looks_for_the_word_in_headings_only() {
    let (_guard, index) = indexed(&[("Раз.md", "# Архитектура\n\nпоиск"), ("Два.md", "Архитектура поиска в тексте.")]);
    assert_eq!(names(&search(&index, "#архитектура поиск")), ["Раз"]);
}

#[test]
fn a_long_note_adds_up_its_pages_and_an_exclusion_on_any_page_hides_it() {
    let filler: String = (0..40_000).map(|line| format!("строка номер {line}\n")).collect();
    let (guard, index) = indexed(&[("Длинная.md", &format!("кофе\n\n{filler}\nчай\n")), ("Кофе.md", "кофе")]);
    let mut pages = crate::search::pages::Pages::open(&guard.path().join("Длинная.md")).unwrap();
    assert!(pages.next().unwrap().is_some() && pages.next().unwrap().is_some(), "заметка из нескольких страниц");
    let long = search(&index, "кофе чай");
    assert_eq!((long[0].title.as_str(), long[0].match_count, long[0].matched_terms.len()), ("Длинная", 2, 2));
    assert_eq!(names(&search(&index, "кофе -чай")), ["Кофе"]);
}

#[test]
fn a_multi_word_query_ranks_partial_matches_instead_of_returning_nothing() {
    let (_guard, index) = indexed(&[
        ("Камера.md", "# Ощущение камеры\n\nФизика тика влияет на кадр.\n"),
        ("Свет.md", "# Освещение\n\nОдин источник на всю сцену.\n"),
        ("Быстрота.md", "# Плавность\n\nКадр рисуется быстро.\n"),
    ]);
    let results = search(&index, "физика тика кадр плавность");
    assert_eq!(names(&results), ["Камера", "Быстрота"], "заметка с большим числом совпавших слов идёт первой");
    assert_eq!((results[0].matched_terms.len(), results[1].matched_terms.len()), (3, 2));
    assert_eq!(results[0].heading.as_deref(), Some("Ощущение камеры"), "фрагмент подписан разделом, в котором нашлось совпадение");
}

#[test]
fn a_title_match_comes_before_any_body_match() {
    let body = "Конец дня, конец недели, конец месяца. ".repeat(50);
    let (_guard, index) = indexed(&[("Дневник.md", body.as_str()), ("Конец года.md", "Итоги.")]);
    assert_eq!(names(&search(&index, "конец")), ["Конец года", "Дневник"]);
}
