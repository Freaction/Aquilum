use super::analyze;
use super::score::normalized_tf;
use crate::search::index::SearchIndex;
use std::fs;
use tantivy::doc;

#[test]
fn field_length_normalization_prefers_concise_candidates() {
    assert!(normalized_tf(2.0, 10, 10.0, 0.75) > normalized_tf(2.0, 100, 10.0, 0.75));
}

#[test]
fn ranks_title_matches_and_does_not_normalize_the_winner_to_one() {
    let directory = tempfile::tempdir().unwrap();
    let vault = directory.path().join("vault");
    fs::create_dir_all(&vault).unwrap();
    let source_path = vault.join("source.md");
    fs::write(&source_path, "aquilum architecture indexing").unwrap();

    let index = SearchIndex::open(&directory.path().join("index")).unwrap();
    let mut writer = index.index.writer(15_000_000).unwrap();
    let documents = [
        (&source_path, "source", "aquilum architecture indexing".to_owned()),
        (&vault.join("title.md"), "aquilum architecture", "notes".to_owned()),
        (&vault.join("short.md"), "short", "aquilum architecture".to_owned()),
        (
            &vault.join("long.md"),
            "long",
            format!("aquilum architecture {}", "filler ".repeat(200)),
        ),
    ];
    for (path, title, body) in documents {
        writer
            .add_document(doc!(
                index.fields.id => path.to_string_lossy().as_ref(),
                index.fields.path => path.to_string_lossy().as_ref(),
                index.fields.title => title,
                index.fields.body => body,
            ))
            .unwrap();
    }
    for index_offset in 0..80 {
        let path = vault.join(format!("noise-{index_offset}.md"));
        let title = format!("noise {index_offset}");
        writer
            .add_document(doc!(
                index.fields.id => path.to_string_lossy().as_ref(),
                index.fields.path => path.to_string_lossy().as_ref(),
                index.fields.title => title.as_str(),
                index.fields.body => "unrelated background material",
            ))
            .unwrap();
    }
    writer.commit().unwrap();
    index.reload().unwrap();

    let results = analyze(
        &index,
        &source_path,
        10,
        &crate::settings::models::Bm25fParams::default(),
        &crate::settings::models::SearchIndexSettings::default(),
    )
    .unwrap();
    assert!(results[0].raw_score > 1.0);
    let title_match = results
        .iter()
        .find(|result| result.title == "title")
        .unwrap();
    let short = results.iter().find(|result| result.title == "short").unwrap();
    let long = results.iter().find(|result| result.title == "long").unwrap();
    assert!(title_match.raw_score > long.raw_score);
    assert!(short.raw_score > long.raw_score);
    assert!(results.iter().all(|result| result.confidence.is_none()));
}

#[test]
fn reasons_show_surface_words_not_stems() {
    let directory = tempfile::tempdir().unwrap();
    let vault = directory.path().join("vault");
    fs::create_dir_all(&vault).unwrap();
    let source_path = vault.join("source.md");
    fs::write(&source_path, "новая книга про архитектуру").unwrap();
    let match_path = vault.join("match.md");

    let index = SearchIndex::open(&directory.path().join("index")).unwrap();
    let mut writer = index.index.writer(15_000_000).unwrap();
    for (path, title, body) in [
        (&source_path, "source", "новая книга про архитектуру"),
        (&match_path, "match", "новая книга и архитектура"),
    ] {
        writer
            .add_document(doc!(
                index.fields.id => path.to_string_lossy().as_ref(),
                index.fields.path => path.to_string_lossy().as_ref(),
                index.fields.title => title,
                index.fields.body => body,
            ))
            .unwrap();
    }
    writer.commit().unwrap();
    index.reload().unwrap();

    let results = analyze(
        &index,
        &source_path,
        5,
        &crate::settings::models::Bm25fParams::default(),
        &crate::settings::models::SearchIndexSettings::default(),
    )
    .unwrap();
    let matched = results
        .iter()
        .find(|result| result.path.contains("match.md"))
        .expect("expected match result");
    assert!(
        matched.reasons.iter().any(|reason| reason == "книга" || reason == "новая"),
        "expected surface words in reasons, got {:?}",
        matched.reasons
    );
    assert!(
        !matched.reasons.iter().any(|reason| reason == "книг" || reason == "нов"),
        "stems must not appear in tooltip reasons, got {:?}",
        matched.reasons
    );
}
