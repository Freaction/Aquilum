use super::support::{best, index_vault, ms, open_bench_index, record, BenchIndex};
use crate::search::analysis::bm25f;
use crate::search::analysis::graph::GraphSnapshot;
use crate::search::paths::canonical_path;
use crate::search::wiki;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[test]
#[ignore = "run with AQUILUM_BENCH_VAULT=<папка базы> or AQUILUM_BENCH_SIZES=1000 cargo test --release benchmark_link_pipeline -- --ignored --nocapture"]
fn benchmark_link_pipeline() {
    if let Ok(vault) = std::env::var("AQUILUM_BENCH_VAULT") {
        let threads_before = process_threads();
        let bench = index_vault(tempfile::tempdir().unwrap(), canonical_path(Path::new(&vault)));
        run_case(bench, &vault, threads_before);
        return;
    }
    let sizes = std::env::var("AQUILUM_BENCH_SIZES").unwrap_or_else(|_| "1000".to_owned());
    for count in sizes.split(',').filter_map(|value| value.trim().parse::<usize>().ok()) {
        let threads_before = process_threads();
        run_case(open_bench_index(count), &format!("synthetic-{count}"), threads_before);
    }
}

fn run_case(mut bench: BenchIndex, label: &str, threads_before: usize) {
    let threads_with_index = process_threads();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let root = bench.vault.clone();
    let connection = Connection::open(&bench.metadata).unwrap();
    let links: i64 = connection
        .query_row("SELECT count(*) FROM wiki_links", [], |row| row.get(0))
        .unwrap();
    let documents = wiki::read_documents(&connection)
        .unwrap()
        .into_iter()
        .map(|document| PathBuf::from(document.path))
        .collect::<Vec<_>>();

    let graph_load = best(|| {
        GraphSnapshot::load(&connection, &root).unwrap();
    });
    let graph = GraphSnapshot::load(&connection, &root).unwrap();
    let incoming = best(|| {
        graph.incoming_counts(&documents);
    });
    let canonicalize = best(|| {
        for document in &documents {
            canonical_path(document);
        }
    }) / documents.len().max(1) as u32;
    let sample = documents.iter().take(20).collect::<Vec<_>>();
    let backlinks20 = best(|| {
        for document in &sample {
            wiki::backlinks(&connection, &root, document).unwrap();
        }
    });
    let connection_open = best(|| {
        Connection::open(&bench.metadata).unwrap();
    });
    let search_settings = crate::settings::models::SearchIndexSettings::default();
    let bm25f_params = crate::settings::models::Bm25fParams::default();
    let bm25f = best(|| {
        bm25f::analyze(&bench.search_index, &documents[0], 20, &bm25f_params, &search_settings).unwrap();
    });
    let search20 = best(|| {
        bench.search_index.search(&crate::search::query::Request { limit: Some(21), ..crate::search::query::Request::new("кадр замер frame") }).unwrap();
    });

    let line = format!(
        "vault={label} documents={} links={links} graph_load_ms={:.1} incoming_ms={:.1} canonicalize_us={} backlinks20_ms={:.1} connection_open_us={} bm25f_ms={:.1} search20_ms={:.1} threads_start={threads_before} threads_open={threads_with_index} threads_end={}",
        documents.len(),
        ms(graph_load),
        ms(incoming),
        canonicalize.as_micros(),
        ms(backlinks20),
        connection_open.as_micros(),
        ms(bm25f),
        ms(search20),
        process_threads(),
    );
    println!("BENCH-LINKS {line}");
    record("link_pipeline", &line);
}

fn process_threads() -> usize {
    let command = format!("(Get-Process -Id {}).Threads.Count", std::process::id());
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &command])
        .output()
        .ok()
        .and_then(|output| String::from_utf8_lossy(&output.stdout).trim().parse().ok())
        .unwrap_or(0)
}
