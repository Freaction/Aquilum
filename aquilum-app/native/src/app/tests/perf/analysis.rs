use super::*;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::links::{Method, Mode, Request};
use crate::stats::mb;
use crate::ui::editor::TextEditor;

const STRESS: &str = "Полный тест Markdown.md";
const REGULAR: [&str; 3] = ["Base/📖 Путеводитель по бессмертию.md", "Идеи для игры с зомби.md", "markdown-test.md"];
const TERMS: [&str; 3] = ["текст", "Пункт", "формула"];

fn request(vault: &Path, note: &Path, method: Method) -> Request {
    Request { generation: 1, workspace: vault.to_path_buf(), document: note.to_path_buf(), mode: Mode::Analysis, method, text: String::new() }
}

fn analysis(core: &aquilum_core::Core, vault: &Path, note: &Path) {
    let name = note.file_name().unwrap().to_string_lossy().into_owned();
    for method in [Method::Bm25f, Method::AdamicAdar] {
        let before = crate::memory_probe::heap();
        let (listing, ms, peak) = measured(|| crate::links::run(core, &request(vault, note, method)));
        let held = mb(crate::memory_probe::heap().saturating_sub(before));
        let reasons = listing.items.iter().map(|i| i.reasons.len()).max().unwrap_or(0);
        eprintln!("{name}: {method:?} {ms:.1} мс, пик кучи +{peak:.1} МБ, осталось +{held:.1} МБ, найдено {}, причин до {reasons}", listing.items.len());
    }
    let head = aquilum_core::files::document::read_head(note).unwrap();
    let text = head.as_str();
    let path = note.to_string_lossy();
    let (extracted, ms, peak) = measured(|| aquilum_core::wikixiv::extract::extract_queries(text, Some(&path)));
    let (_, hash_ms, _) = measured(|| aquilum_core::wikixiv::cache::content_hash(Some(&path), text));
    eprintln!("{name}: Wikixiv извлечение {ms:.1} мс, пик кучи +{peak:.1} МБ, хеш {hash_ms:.1} мс, запросы {:?} {:?}", extracted.body_terms, extracted.embedded_titles.len());
}

fn timed_frames(b: &mut Bench, n: usize, mut step: impl FnMut(&mut Bench, usize)) -> String {
    let mut times = Vec::new();
    for i in 0..n {
        let t = Instant::now();
        step(b, i);
        b.h.deliver();
        b.timed(false);
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let [p50, max] = percentiles(&times, [0.5, 1.0]);
    format!("кадров {n}, p50 {p50:.2} мс, максимум {max:.2} мс")
}

fn frames(b: &mut Bench, n: usize) -> String {
    let at = (f64::from(b.w) / 2.0, f64::from(b.hh) / 2.0);
    timed_frames(b, n, |b, i| b.h.scroll(at, b.scale, (0.0, if i % 2 == 0 { -40.0 } else { 40.0 })))
}

fn editor(b: &mut Bench, note: PathBuf) {
    b.h.app.open(&mut b.h.root, note);
    let busy = |b: &Bench| b.h.app.embeds.as_ref().is_some_and(crate::app::embeds::Embeds::busy);
    let t = Instant::now();
    while busy(b) && t.elapsed() < Duration::from_secs(60) {
        b.h.app.on_core_events(&mut b.h.root);
        b.timed(false);
    }
    let scroll = b.h.app.note_ids.as_ref().unwrap().scroll.clone();
    scroll.edit(&mut b.h.root, |mut area| crate::ui::scroll::ScrollArea::set_fraction(&mut area, 0.5));
    b.h.deliver();
    b.timed(false);
    eprintln!("без подсветки: {}", frames(b, 40));
    let body = b.h.app.note_ids.as_ref().unwrap().body.clone();
    let terms: Vec<String> = TERMS.iter().map(|t| (*t).to_owned()).collect();
    let (_, ms, peak) = measured(|| body.edit(&mut b.h.root, |mut ed| TextEditor::set_hover_terms(&mut ed, terms)));
    eprintln!("наведение: set_hover_terms {ms:.1} мс, пик кучи +{peak:.1} МБ");
    eprintln!("с подсветкой наведения: {}", frames(b, 10));
    let orders = [TERMS.map(str::to_owned).to_vec(), TERMS.iter().rev().map(|t| (*t).to_owned()).collect()];
    let repaint = timed_frames(b, 20, |b, i| {
        body.edit(&mut b.h.root, |mut ed| TextEditor::set_hover_terms(&mut ed, orders[i % 2].clone()));
    });
    eprintln!("наведение со сменой слов и перерисовкой: {repaint}");
    body.edit(&mut b.h.root, |mut ed| TextEditor::set_hover_terms(&mut ed, Vec::new()));
    b.h.focus_editor();
    let smart = crate::ui::editor::Smart { matches: true, ..Default::default() };
    body.edit(&mut b.h.root, |mut ed| TextEditor::set_smart(&mut ed, smart));
    let offset = super::super::support::note_offset(&b.h);
    let words: Vec<usize> = {
        let widget = body.get(&b.h.root).unwrap();
        let editor = widget.inner();
        let start = editor.anchor_at(offset).map_or(0, |(at, _)| at);
        editor.text()[start..].match_indices(TERMS[0]).take(2).map(|(at, _)| start + at).collect()
    };
    let select = |b: &mut Bench, at: usize| {
        body.edit(&mut b.h.root, |mut ed| TextEditor::select_byte_range(&mut ed, at, at + TERMS[0].len()));
    };
    select(b, words[0]);
    b.h.deliver();
    eprintln!("с выделенным словом «{}»: {}", TERMS[0], frames(b, 40));
    eprintln!("смена выделения с перерисовкой: {}", timed_frames(b, 20, |b, i| select(b, words[i % 2])));
    let (_, ms, peak) = measured(|| body.edit(&mut b.h.root, |mut ed| TextEditor::find(&mut ed, TERMS[0])));
    eprintln!("Ctrl+F «{}» по загруженному окну редактора:{ms:.1} мс, пик кучи +{peak:.1} МБ; кадр после: {}", TERMS[0], timed_frames(b, 1, |_, _| {}));
    body.edit(&mut b.h.root, |mut ed| TextEditor::find(&mut ed, ""));
    let (_, ms, peak) = measured(|| b.h.app.request_panel(&mut b.h.root));
    eprintln!("request_panel: {ms:.1} мс, пик кучи +{peak:.1} МБ");
}

#[test]
#[ignore]
fn analysis_big_note() {
    let source = PathBuf::from(std::env::var("AQ_VAULT").expect("AQ_VAULT"));
    let vault = crate::test_dir::tempdir();
    copy_notes(&source, vault.path());
    let mut b = Bench::start(Some(vault.path()));
    let core = Arc::clone(&b.h.app.workspace.core);
    let t = Instant::now();
    assert!(wait_index(&core, &vault.path().to_string_lossy()), "индекс не готов");
    eprintln!("индекс готов за {:?}", t.elapsed());
    for name in [STRESS].iter().chain(&REGULAR) {
        analysis(&core, vault.path(), &vault.path().join(name));
    }
    let (mut quality, mut slowest) = (String::new(), Vec::new());
    for note in walk(vault.path()) {
        let name = note.strip_prefix(vault.path()).unwrap().display().to_string();
        for method in [Method::Bm25f, Method::AdamicAdar] {
            let (listing, ms, _) = measured(|| crate::links::run(&core, &request(vault.path(), &note, method)));
            slowest.push((ms, format!("{name} {method:?}")));
            let titles: Vec<String> = listing.items.into_iter().take(10).map(|i| i.title).collect();
            quality.push_str(&format!("{name}\t{method:?}\t{}\n", titles.join("|")));
        }
    }
    slowest.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top: Vec<String> = slowest.iter().take(5).map(|(ms, name)| format!("{name} {ms:.0} мс")).collect();
    eprintln!("заметки 16 КиБ…1 МиБ ({}): самые долгие {}", slowest.len() / 2, top.join("; "));
    if let Ok(out) = std::env::var("AQ_QUALITY") {
        std::fs::write(out, quality).unwrap();
    }
    editor(&mut b, vault.path().join(STRESS));
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut notes = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            notes.extend(walk(&path));
        } else if path.metadata().is_ok_and(|m| m.len() > 16 * 1024 && m.len() < 1 << 20) {
            notes.push(path);
        }
    }
    notes
}
