use super::*;

use std::path::Path;
use std::time::{Duration, Instant};

use aquilum_core::search::hits::query_matcher;
use aquilum_core::search::query::{Request, analyzed_query_terms};
use aquilum_core::text::matcher::{IGNORE_CASE, Matcher};

use crate::memory_probe::heap;
use crate::stats::mb;
use crate::ui::editor::TextAction;

const STRESS: &str = "Полный тест Markdown";

fn core_query(core: &aquilum_core::Core, workspace: &str, query: &str) {
    for pass in ["холодный", "повтор"] {
        let before = heap();
        let (response, ms, peak) = measured(|| core.search.search(workspace, &Request::new(query)).unwrap());
        let held = mb(heap().saturating_sub(before));
        let (results, terms) = (&response.results, &response.query_terms);
        let stress = results.iter().position(|r| r.title == STRESS).map(|i| (i, results[i].match_count));
        let top: Vec<String> = results.iter().take(3).map(|r| format!("{} ({})", r.title, r.match_count)).collect();
        eprintln!("ядро «{query}» {pass}: {ms:.1} мс, пик +{peak:.0} МБ, осталось +{held:.0} МБ; результатов {}, термины {terms:?}, огромная заметка (место, совпадений) {stress:?}, первые {}", results.len(), top.join("; "));
    }
}

fn breakdown(note: &Path, query: &str) {
    let body = aquilum_core::files::document::read_text(note).unwrap();
    let matcher = query_matcher(&analyzed_query_terms(query));
    let (matches, all_ms, all_peak) = measured(|| matcher.find_all(&body));
    let (first, first_ms) = clock(|| matcher.first(&body).map_or(0, |found| found.start));
    let (heading, heading_ms, heading_peak) = measured(|| aquilum_core::search::headings::heading_at(&body, first));
    let find = Matcher::new(query, IGNORE_CASE);
    let (found, find_ms, find_peak) = measured(|| find.find_all(&body).len());
    eprintln!("тело огромной заметки «{query}»: все совпадения Ctrl+O {all_ms:.0} мс, пик +{all_peak:.0} МБ, {}; первое {first_ms:.2} мс (байт {first}); heading_at {heading_ms:.2} мс, пик +{heading_peak:.0} МБ, {heading:?}; Ctrl+F по всему тексту {find_ms:.0} мс, пик +{find_peak:.0} МБ, {found}", matches.len());
}

fn index_size(core: &aquilum_core::Core, workspace: &str) {
    let storage = core.search.with_index(workspace, |open| Ok(open.metadata_path.parent().unwrap().to_path_buf())).unwrap();
    let index = storage.join(format!("index-v{}", aquilum_core::search::ANALYZER_VERSION));
    let (mut total, mut store) = (0, 0);
    for entry in std::fs::read_dir(&index).unwrap().flatten() {
        let bytes = entry.metadata().unwrap().len();
        total += bytes;
        if entry.path().extension().is_some_and(|e| e == "store") {
            store += bytes;
        }
    }
    eprintln!("индекс {}: {:.1} МБ, из них хранилище .store {:.2} МБ", index.display(), total as f64 / 1048576.0, store as f64 / 1048576.0);
}

fn store(core: &aquilum_core::Core, workspace: &str) {
    let read = |open: &aquilum_core::search::service::OpenIndex| {
        let searcher = open.index.reader.searcher();
        let (mut docs, mut total, mut slow) = (0, 0.0, Vec::new());
        for (ord, segment) in searcher.segment_readers().iter().enumerate() {
            let store = segment.get_store_reader(0).unwrap();
            for doc in segment.doc_ids_alive() {
                let (bytes, ms, peak) = measured(|| store.get_document_bytes(doc).unwrap().len());
                (docs, total) = (docs + 1, total + ms);
                if ms > 20.0 {
                    slow.push(format!("сегмент {ord} документ {doc}: {ms:.0} мс, {:.0} КБ, пик +{peak:.0} МБ", bytes as f64 / 1024.0));
                }
            }
        }
        eprintln!("хранилище без кэша: {docs} документов, всего {total:.0} мс; дольше 20 мс: {}", slow.join("; "));
        Ok(())
    };
    core.search.with_index(workspace, read).unwrap();
}

fn wait_found(b: &mut Bench, n: usize) -> Vec<f64> {
    let (t, mut found, mut times) = (Instant::now(), Vec::new(), Vec::new());
    while found.len() < n && t.elapsed() < Duration::from_secs(300) {
        match b.h.app.search_rx.try_recv() {
            Ok(f) => {
                found.push(f);
                times.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            Err(_) => std::thread::sleep(Duration::from_micros(200)),
        }
    }
    found.into_iter().for_each(|f| b.h.app.search_tx.send(f).unwrap());
    times
}

fn type_query(b: &mut Bench, query: &str) {
    let Some(Popup::Search(state)) = &b.h.app.popup else { panic!("поиск не открыт") };
    let field = state.field_id();
    b.h.send(field, TextAction::Changed(query.into()));
}

fn frame(b: &mut Bench) -> (f64, f64) {
    b.h.deliver();
    let (scene, raster) = b.timed(false);
    (scene.as_secs_f64() * 1000.0, raster.as_secs_f64() * 1000.0)
}

fn ui_query(b: &mut Bench, query: &str) {
    let ((), typed) = clock(|| type_query(b, query));
    let (_, typed_frame) = clock(|| frame(b));
    let arrived = wait_found(b, 1)[0];
    let ((), render) = clock(|| b.h.app.on_search_results(&mut b.h.root));
    let ((scene, raster), shown) = clock(|| frame(b));
    let cards = if let Some(Popup::Search(state)) = &b.h.app.popup { state.result_count() } else { 0 };
    eprintln!("окно «{query}»: ввод {typed:.1} мс + кадр {typed_frame:.1} мс; ответ через {arrived:.0} мс; карточки {render:.1} мс; кадр {shown:.1} мс (сцена {scene:.1}, растр {raster:.1}); карточек {cards}");
}

fn timed_frames(b: &mut Bench, n: usize, mut step: impl FnMut(&mut Bench, usize)) -> String {
    let times: Vec<f64> = (0..n).map(|i| clock(|| (step(b, i), frame(b))).1).collect();
    let [p50, max] = percentiles(&times, [0.5, 1.0]);
    format!("p50 {p50:.2} мс, максимум {max:.2}")
}

fn list_frames(b: &mut Bench) {
    let keys = timed_frames(b, 30, |b, _| {
        b.h.app.search_key(&mut b.h.root, &Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    });
    let at = (f64::from(b.w) / 2.0, f64::from(b.hh) / 2.0);
    let wheel = timed_frames(b, 40, |b, i| b.h.scroll(at, b.scale, (0.0, if i < 20 { -60.0 } else { 60.0 })));
    eprintln!("список: стрелка вниз {keys}; колесо {wheel}");
}

fn wait_current(b: &mut Bench) -> (usize, f64) {
    let Some(Popup::Search(state)) = &b.h.app.popup else { panic!("поиск не открыт") };
    let (current, t, mut answers) = (state.generation(), Instant::now(), 0);
    while t.elapsed() < Duration::from_secs(300) {
        match b.h.app.search_rx.try_recv() {
            Ok(found) if found.generation() == current => {
                b.h.app.search_tx.send(found).unwrap();
                return (answers + 1, t.elapsed().as_secs_f64() * 1000.0);
            }
            Ok(_) => answers += 1,
            Err(_) => std::thread::sleep(Duration::from_micros(200)),
        }
    }
    panic!("нет ответа на последний запрос");
}

fn rapid(b: &mut Bench, word: &str, pause: u64) {
    let prefixes: Vec<String> = (2..=word.chars().count()).map(|n| word.chars().take(n).collect()).collect();
    let (cpu, before) = (crate::memory_probe::cpu_ms(), heap());
    let ((answers, last), ms, peak) = measured(|| {
        for prefix in &prefixes {
            type_query(b, prefix);
            std::thread::sleep(Duration::from_millis(pause));
        }
        wait_current(b)
    });
    b.h.app.on_search_results(&mut b.h.root);
    let (cpu, held) = (crate::memory_probe::cpu_ms() - cpu, mb(heap().saturating_sub(before)));
    eprintln!("набор «{word}», {} букв, {pause} мс между буквами: ответов {answers}, последний через {last:.0} мс после набора; всего {ms:.0} мс, процессор {cpu:.0} мс, пик +{peak:.0} МБ, осталось +{held:.0} МБ", prefixes.len());
}

#[test]
#[ignore]
fn search_big_vault() {
    aq_trace::start();
    let source = std::path::PathBuf::from(std::env::var("AQ_VAULT").expect("AQ_VAULT"));
    let queries = std::env::var("AQ_QUERIES").unwrap_or_else(|_| "зомби,созвон,т,те,тест,текст".into());
    let vault = crate::test_dir::tempdir();
    copy_notes(&source, vault.path());
    let mut b = Bench::start(Some(vault.path()));
    let core = Arc::clone(&b.h.app.workspace.core);
    let workspace = vault.path().to_string_lossy().into_owned();
    let (built, ms) = clock(|| wait_index(&core, &workspace));
    assert!(built, "индекс не готов");
    eprintln!("индекс готов за {ms:.0} мс");
    index_size(&core, &workspace);
    if std::env::var_os("AQ_STORE").is_some() {
        store(&core, &workspace);
    }
    for query in queries.split(',') {
        core_query(&core, &workspace, query);
    }
    if let Ok(words) = std::env::var("AQ_BREAKDOWN") {
        for word in words.split(',') {
            breakdown(&vault.path().join(format!("{STRESS}.md")), word);
        }
    }
    b.h.app.open_search(&mut b.h.root);
    frame(&mut b);
    for query in queries.split(',') {
        ui_query(&mut b, query);
    }
    list_frames(&mut b);
    if let Ok(words) = std::env::var("AQ_RAPID") {
        for word in words.split(',') {
            rapid(&mut b, word, 30);
            rapid(&mut b, word, 150);
        }
    }
    if let Ok(out) = std::env::var("AQ_TRACE") {
        aq_trace::write(Path::new(&out)).unwrap();
    }
}
