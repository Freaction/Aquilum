use super::{indexing_tokenizer, ANALYZER_VERSION};
use tantivy::tokenizer::TokenStream;

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    indexing_tokenizer().token_stream(text).process(&mut |token| {
        out.push(token.text.clone());
    });
    out
}

#[test]
fn analyzer_version_tracks_morphology_pipeline() {
    assert_eq!(ANALYZER_VERSION, 14);
}

#[test]
fn frontmatter_author_is_indexed_metadata_keys_are_not() {
    let input = r#"---
type: book
author: Isaac Asimov
status: to-read
tags: []
cover_url: https://x.ru/cover.jpg
---
Foundation trilogy content."#;
    let got = tokens(input);
    assert!(got.iter().any(|t| t.contains("asimov") || t == "isaac"));
    assert!(got.iter().any(|t| t.contains("foundat") || t == "trilog"));
    assert!(!got.iter().any(|t| t == "book" || t == "to-read" || t == "tags"));
    assert!(!got.iter().any(|t| t == "https" || t == "cover_url"));
}

#[test]
fn markdown_link_indexes_label_not_url() {
    let got = tokens("see [NeuroNet docs](https://example.ru/path) here");
    assert!(got.contains(&"neuronet".to_owned()) || got.iter().any(|t| t.contains("neuro")));
    assert!(got.iter().any(|t| t == "docs" || t.starts_with("doc")));
    assert!(!got.iter().any(|t| t == "https" || t == "ru" || t == "example" || t == "path"));
    assert!(got.contains(&"see".to_owned()));
    assert!(got.contains(&"here".to_owned()));
}

#[test]
fn excluded_url_crumbs_never_appear_as_tokens() {
    let got = tokens("https www com ru org net");
    assert!(got.is_empty(), "expected no tokens, got {got:?}");
}

#[test]
fn russian_forms_collapse_to_same_stem() {
    let singular = tokens("машина");
    let plural = tokens("машины");
    let dative = tokens("машине");
    assert_eq!(singular, plural);
    assert_eq!(singular, dative);
    assert!(!singular.is_empty());
}

#[test]
fn english_forms_collapse_to_same_stem() {
    assert_eq!(tokens("machines"), tokens("machine"));
    assert_eq!(tokens("programming"), tokens("programs"));
    assert_eq!(tokens("Python"), vec!["python".to_owned()]);
}

#[test]
fn short_latin_tokens_stay_intact() {
    assert_eq!(tokens("dog api"), vec!["dog".to_owned(), "api".to_owned()]);
}

#[test]
fn short_cyrillic_tokens_stay_intact() {
    assert_eq!(tokens("на дом"), vec!["на".to_owned(), "дом".to_owned()]);
}

#[test]
fn stemmed_query_matches_inflected_document() {
    use crate::search::index::SearchIndex;
    use crate::search::query::Request;
    use std::fs;
    use tantivy::doc;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("note.md");
    fs::write(&path, "про нейронные сети").unwrap();
    let index = SearchIndex::open(&directory.path().join("index")).unwrap();
    let mut writer = index.index.writer(15_000_000).unwrap();
    writer
        .add_document(doc!(
            index.fields.id => path.to_string_lossy().as_ref(),
            index.fields.title => "сети",
            index.fields.body => "про нейронные сети",
        ))
        .unwrap();
    writer.commit().unwrap();
    index.reload().unwrap();

    assert_eq!(tokens("нейронные"), tokens("нейронных"));
    let results = index.search(&Request::new("нейронных")).unwrap().results;
    assert!(
        results.iter().any(|result| result.path.contains("note.md")),
        "omnisearch should hit via analyzer, got {results:?}"
    );
}
