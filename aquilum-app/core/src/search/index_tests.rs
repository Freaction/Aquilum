use super::index::SearchIndex;
use super::index_document::add_page;
use super::pages::{Keys, Pages};
use super::query::{analyzed_query_terms, Request};
use std::path::Path;

#[test]
fn normalizes_case_and_deduplicates_query_terms() {
    assert_eq!(analyzed_query_terms("Aquilum АКВИЛ aquilum"), vec!["aquilum".to_string(), "акв".to_string()]);
}

fn indexed(documents: &[(&str, &str)]) -> (tempfile::TempDir, SearchIndex) {
    let directory = tempfile::tempdir().unwrap();
    let index = SearchIndex::open(&directory.path().join("index")).unwrap();
    let mut writer = index.index.writer(15_000_000).unwrap();
    for (name, body) in documents {
        let path = directory.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        let (mut pages, note, mut keys) = (Pages::open(&path).unwrap(), path.to_string_lossy().into_owned(), Keys::default());
        while let Some(page) = pages.next().unwrap() {
            add_page(directory.path(), &index, &writer, &path, &page, &keys.next(&note, &page)).unwrap();
        }
    }
    writer.commit().unwrap();
    index.reload().unwrap();
    (directory, index)
}

fn search(index: &SearchIndex, text: &str) -> Vec<super::models::SearchResult> {
    index.search(&Request::new(text)).unwrap().results
}

#[test]
fn disk_index_searches_title_and_body_by_case_insensitive_prefix() {
    let (_guard, index) = indexed(&[("Квантовая заметка.md", "Внутри есть быстрый поиск и ещё один поиск.")]);

    assert_eq!(search(&index, "КВАН").len(), 1);
    let body_results = search(&index, "ПОИ");
    assert_eq!(body_results[0].match_count, 2);
    assert!(body_results[0].snippet.contains("поиск"));
}

#[test]
fn the_body_is_indexed_but_not_kept_in_the_index_store() {
    let (_guard, index) = indexed(&[("Заметка.md", "длинный текст заметки")]);
    let searcher = index.reader.searcher();
    let document = searcher.doc::<tantivy::TantivyDocument>(tantivy::DocAddress::new(0, 0)).unwrap();
    assert!(document.get_first(index.fields.body).is_none());
    assert_eq!(search(&index, "текст").len(), 1);
}

#[test]
fn the_first_match_comes_from_the_file_with_its_offset_and_section() {
    let body = "---\ntags: []\n---\n# Раздел\n\nСтрока 😀 с Тестом и ещё тесты\n";
    let (_guard, index) = indexed(&[("Заметка.md", body)]);
    let result = &search(&index, "тест")[0];
    let at = body.find("Тестом").unwrap();
    assert_eq!(result.match_offset, body[..at].encode_utf16().count());
    assert_eq!(result.heading.as_deref(), Some("Раздел"));
    assert_eq!(result.match_count, 2);
    assert!(result.snippet.contains("Тестом"));
}

#[test]
fn a_note_without_a_body_match_shows_its_beginning() {
    let (_guard, index) = indexed(&[("Тест.md", "Начало заметки без слова")]);
    let result = &search(&index, "тест")[0];
    assert_eq!((result.match_count, result.match_offset), (1, 0));
    assert_eq!(result.snippet, "Начало заметки без слова");
    assert_eq!(result.heading, None);
    assert_eq!(result.matched_terms, ["тест"]);
}

#[test]
fn a_single_letter_finds_and_counts_words_that_start_with_it() {
    let (_guard, index) = indexed(&[("Кофе.md", "кофе и какао"), ("Чай.md", "чай")]);
    let results = search(&index, "к");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].match_count, 3);
}

#[test]
fn a_folder_limits_the_search_and_more_results_are_reported() {
    let (guard, index) = indexed(&[
        ("Проекты/Кофе.md", "кофе"),
        ("Проекты/Чай.md", "кофе и чай"),
        ("Архив/Кофе.md", "кофе"),
    ]);
    let folder = guard.path().join("Проекты");
    let inside = index.search(&Request { folder: Some(&folder), ..Request::new("кофе") }).unwrap();
    assert_eq!(inside.results.len(), 2);
    assert!(inside.results.iter().all(|result| Path::new(&result.path).starts_with(&folder)));
    assert!(!inside.has_more);
    let first = index.search(&Request { limit: Some(1), ..Request::new("кофе") }).unwrap();
    assert!(first.has_more && first.results.len() == 1);
}

#[test]
fn a_cancelled_request_returns_no_results() {
    let (_guard, index) = indexed(&[("Кофе.md", "кофе")]);
    let generation = crate::text::cancel::Generation::default();
    let cancel = generation.next();
    generation.stop();
    let request = Request { cancel, ..Request::new("кофе") };
    assert!(matches!(index.search(&request), Err(super::error::SearchError::Cancelled)));
}

#[test]
fn a_multi_word_query_ranks_partial_matches_instead_of_returning_nothing() {
    let (_guard, index) = indexed(&[
        ("Камера.md", "# Ощущение камеры\n\nФизика тика влияет на кадр.\n"),
        ("Свет.md", "# Освещение\n\nОдин источник на всю сцену.\n"),
        ("Быстрота.md", "# Плавность\n\nКадр рисуется быстро.\n"),
    ]);

    let results = search(&index, "физика тика кадр плавность");
    assert_eq!(results.len(), 2, "ни одна заметка не содержит все слова, но частичные совпадения возвращаются");
    assert!(results[0].path.ends_with("Камера.md"), "заметка с большим числом совпавших слов идёт первой");
    assert_eq!(results[0].matched_terms.len(), 3);
    assert_eq!(results[1].matched_terms.len(), 2);
    assert_eq!(results[0].heading.as_deref(), Some("Ощущение камеры"), "фрагмент подписан разделом, в котором нашлось совпадение");
}

#[test]
fn a_title_match_comes_before_any_body_match() {
    let body = "Конец дня, конец недели, конец месяца. ".repeat(50);
    let (_guard, index) = indexed(&[("Дневник.md", body.as_str()), ("Конец года.md", "Итоги.")]);

    let results = search(&index, "конец");
    assert!(results[0].path.ends_with("Конец года.md"), "совпадение в названии — выше любого совпадения в тексте");
    assert!(results[1].path.ends_with("Дневник.md"));
}
