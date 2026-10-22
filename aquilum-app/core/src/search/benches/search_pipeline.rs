use super::support::{best, directory_size, index_vault, ms, open_bench_index, record};
use crate::search::analysis::bm25f;
use crate::search::paths::canonical_path;
use crate::search::query::Request;
use std::fs;
use std::path::Path;
use std::time::Instant;

const QUERIES: [&str; 8] = ["поиск заметок", "к", "markdown", "полный тест", "архитектура", "кадр замер frame", "заметк", "как настроить синхронизацию заметок с облаком dropbox"];

#[test]
#[ignore = "run with AQUILUM_BENCH_VAULT=<папка базы> or AQUILUM_BENCH_SIZES=1000,10000,100000 cargo test --release benchmark_search_pipeline -- --ignored --nocapture"]
fn benchmark_search_pipeline() {
    if let Ok(vault) = std::env::var("AQUILUM_BENCH_VAULT") {
        return real_vault(&vault);
    }
    let sizes = std::env::var("AQUILUM_BENCH_SIZES").unwrap_or_else(|_| "1000".to_owned());
    for count in sizes.split(',').filter_map(|value| value.trim().parse::<usize>().ok()) {
        run_case(count);
    }
}

fn run_case(document_count: usize) {
    let mut bench = open_bench_index(document_count);

    let started = Instant::now();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let initial_scan_ms = started.elapsed().as_millis();

    let started = Instant::now();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let warm_scan_ms = started.elapsed().as_millis();

    let started = Instant::now();
    let search_results = bench
        .search_index
        .search(&crate::search::query::Request::new("architecture topic"))
        .unwrap()
        .results;
    let search_us = started.elapsed().as_micros();

    let search_settings = crate::settings::models::SearchIndexSettings::default();
    let bm25f_params = crate::settings::models::Bm25fParams::default();

    let started = Instant::now();
    let analysis_results = bm25f::analyze(
        &bench.search_index,
        &bench.vault.join("Note 0.md"),
        50,
        &bm25f_params,
        &search_settings,
    )
    .unwrap();
    let bm25f_cold_us = started.elapsed().as_micros();

    let started = Instant::now();
    let _ = bm25f::analyze(
        &bench.search_index,
        &bench.vault.join("Note 0.md"),
        50,
        &bm25f_params,
        &search_settings,
    )
    .unwrap();
    let bm25f_warm_us = started.elapsed().as_micros();
    let index_bytes = directory_size(&bench.directory.path().join("index"));
    let metadata_bytes = fs::metadata(&bench.metadata).map_or(0, |value| value.len());

    let line = format!(
        "documents={document_count} initial_scan_ms={initial_scan_ms} warm_scan_ms={warm_scan_ms} search_us={search_us} bm25f_cold_us={bm25f_cold_us} bm25f_warm_us={bm25f_warm_us} candidate_pool={} max_query_terms={} search_results={} analysis_results={} index_bytes={index_bytes} metadata_bytes={metadata_bytes}",
        search_settings.candidate_pool_size,
        search_settings.max_query_terms,
        search_results.len(),
        analysis_results.len(),
    );
    println!("BENCH-SEARCH {line}");
    record("search_pipeline", &line);
}

fn real_vault(vault: &str) {
    let mut bench = index_vault(tempfile::tempdir().unwrap(), canonical_path(Path::new(vault)));
    let started = Instant::now();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let scan_ms = started.elapsed().as_millis();
    let index_bytes = directory_size(&bench.directory.path().join("index"));
    record("search_pipeline", &format!("vault={vault} documents={} scan_ms={scan_ms} index_bytes={index_bytes}", bench.search_index.document_count()));
    for query in QUERIES {
        let request = Request { metadata: Some(&bench.metadata), ..Request::new(query) };
        let started = Instant::now();
        let found = bench.search_index.search(&request).unwrap();
        let cold = ms(started.elapsed());
        let warm = ms(best(|| drop(bench.search_index.search(&request).unwrap())));
        let top: Vec<String> = found.results.iter().take(5).map(|r| format!("{} ({} {:?})", r.title, r.match_count, r.matched_terms)).collect();
        let line = format!("query=«{query}» cold_ms={cold:.1} warm_ms={warm:.1} results={} has_more={} top={}", found.results.len(), found.has_more, top.join("; "));
        println!("BENCH-QUERY {line}");
        record("search_pipeline", &line);
    }
}
