use super::support::{directory_size, open_bench_index, record, write_note};
use std::fs;
use std::time::Instant;

#[test]
#[ignore = "run with AQUILUM_BENCH_SIZES=1000,10000,100000 cargo test --release benchmark_indexing -- --ignored --nocapture"]
fn benchmark_indexing() {
    let sizes = std::env::var("AQUILUM_BENCH_SIZES").unwrap_or_else(|_| "1000".to_owned());
    for count in sizes.split(',').filter_map(|value| value.trim().parse::<usize>().ok()) {
        run_case(count);
    }
}

fn run_case(document_count: usize) {
    let mut bench = open_bench_index(document_count);

    let started = Instant::now();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let cold_ms = started.elapsed().as_millis();

    let started = Instant::now();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let warm_ms = started.elapsed().as_millis();

    let touched = (document_count / 100).clamp(1, document_count);
    for index in 0..touched {
        write_note(&bench.vault, index, 55);
    }
    let started = Instant::now();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let incremental_ms = started.elapsed().as_millis();

    for index in 0..touched {
        fs::remove_file(bench.vault.join(format!("Note {index}.md"))).unwrap();
    }
    let started = Instant::now();
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    let deletion_ms = started.elapsed().as_millis();

    let saved = bench.vault.join(format!("Note {touched}.md"));
    let mut saves = (0..5)
        .map(|round| {
            write_note(&bench.vault, touched, 56 + round);
            let started = Instant::now();
            bench
                .synchronizer
                .apply_paths(std::collections::HashSet::from([saved.clone()]))
                .unwrap();
            started.elapsed().as_secs_f64() * 1000.0
        })
        .collect::<Vec<_>>();
    saves.sort_by(f64::total_cmp);
    let (save_min_ms, save_median_ms) = (saves[0], saves[saves.len() / 2]);
    bench.synchronizer.release_writer().unwrap();
    write_note(&bench.vault, touched, 62);
    let started = Instant::now();
    bench
        .synchronizer
        .apply_paths(std::collections::HashSet::from([saved.clone()]))
        .unwrap();
    let save_after_idle_ms = started.elapsed().as_secs_f64() * 1000.0;

    let index_bytes = directory_size(&bench.directory.path().join("index"));
    let docs_per_sec_cold = document_count as f64 / (cold_ms.max(1) as f64 / 1000.0);

    let line = format!(
        "documents={document_count} cold_ms={cold_ms} warm_ms={warm_ms} incremental_touched={touched} incremental_ms={incremental_ms} deletion_ms={deletion_ms} save_min_ms={save_min_ms:.1} save_median_ms={save_median_ms:.1} save_after_idle_ms={save_after_idle_ms:.1} docs_per_sec_cold={docs_per_sec_cold:.0} index_bytes={index_bytes}"
    );
    println!("BENCH-INDEX {line}");
    record("indexing", &line);
}

#[test]
#[ignore = "run with cargo test --release benchmark_save_breakdown -- --ignored --nocapture"]
fn benchmark_save_breakdown() {
    let mut bench = open_bench_index(1000);
    bench.synchronizer.full_scan(&bench.cancelled).unwrap();
    bench.synchronizer.release_writer().unwrap();
    let mut writer = bench
        .search_index
        .index
        .writer::<tantivy::TantivyDocument>(crate::search::sync::INDEX_MEMORY_BUDGET)
        .unwrap();
    let mut connection = crate::search::metadata::open(&bench.metadata).unwrap();
    let saved = bench.vault.join("Note 7.md");
    let mut steps = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for round in 0..7 {
        write_note(&bench.vault, 7, 60 + round);
        let transaction = connection.transaction().unwrap();
        let started = Instant::now();
        let probe = crate::search::index_document::probe_one(&transaction, &saved, &saved.metadata().unwrap()).unwrap().expect("заметка изменилась");
        crate::search::index_document::apply(&bench.vault, &bench.search_index, &writer, &transaction, &saved, probe).unwrap();
        steps[0].push(started.elapsed());
        let started = Instant::now();
        writer.commit().unwrap();
        steps[1].push(started.elapsed());
        let started = Instant::now();
        bench.search_index.reload().unwrap();
        steps[2].push(started.elapsed());
        let started = Instant::now();
        transaction.commit().unwrap();
        steps[3].push(started.elapsed());
        let started = Instant::now();
        crate::search::metadata::maintain(&connection).unwrap();
        steps[4].push(started.elapsed());
    }
    let fsync = (0..7)
        .map(|round| {
            let path = bench.directory.path().join(format!("fsync-{round}.bin"));
            let mut file = fs::File::create(&path).unwrap();
            std::io::Write::write_all(&mut file, &[7u8; 4096]).unwrap();
            let started = Instant::now();
            file.sync_all().unwrap();
            started.elapsed()
        })
        .collect::<Vec<_>>();
    let median = |values: &[std::time::Duration]| {
        let mut sorted = values.to_vec();
        sorted.sort();
        sorted[sorted.len() / 2].as_secs_f64() * 1000.0
    };
    let line = format!(
        "apply_ms={:.2} tantivy_commit_ms={:.2} reload_ms={:.2} sqlite_commit_ms={:.2} maintain_ms={:.2} fsync_4k_ms={:.2}",
        median(&steps[0]),
        median(&steps[1]),
        median(&steps[2]),
        median(&steps[3]),
        median(&steps[4]),
        median(&fsync),
    );
    println!("BENCH-SAVE {line}");
    record("save_breakdown", &line);
}

