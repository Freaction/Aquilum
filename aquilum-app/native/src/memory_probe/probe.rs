use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use super::counting::{CURRENT, PEAK};
use super::process::{breakdown, cpu_ms};
use crate::stats::mb;
use super::script::{Step, parse};
use super::trace::{TRACE, large_report};

const PAUSE: Duration = Duration::from_millis(3000);
const PEAK_STEP: usize = 64 << 20;
const HEAP_EVERY: Duration = Duration::from_millis(250);

fn watch_peaks(output: &std::path::Path) {
    let (peaks, series) = (output.with_extension("peak.txt"), output.with_extension("heap.txt"));
    std::thread::spawn(move || {
        let started = Instant::now();
        let (mut top, mut logged) = (CURRENT.load(Ordering::Relaxed), Duration::ZERO);
        let mut log = std::fs::File::create(&series).ok();
        loop {
            let heap = CURRENT.load(Ordering::Relaxed);
            if heap > top + PEAK_STEP {
                top = heap;
                let text = format!("## пик rust-кучи {:.0} МБ через {:.1} с\n{}{}", mb(heap), started.elapsed().as_secs_f64(), breakdown(), large_report());
                let _ = std::fs::write(&peaks, text);
            }
            if started.elapsed() >= logged + HEAP_EVERY {
                logged = started.elapsed();
                if let Some(log) = &mut log {
                    let _ = writeln!(log, "{:.2} с\t{:.0} МБ", logged.as_secs_f64(), mb(heap));
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    });
}

pub struct Probe {
    output: PathBuf,
    steps: Vec<(String, Step)>,
    pauses: Vec<Duration>,
    next: usize,
    due: Option<Instant>,
    cpu: std::cell::Cell<f64>,
    started: std::cell::Cell<Instant>,
}

impl Probe {
    pub fn new(output: PathBuf, notes: Vec<PathBuf>) -> Self {
        let _ = std::fs::write(&output, "");
        super::trace::configure();
        if std::env::var_os("AQ_PROBE_PEAK").is_some() {
            watch_peaks(&output);
        }
        if let Some(script) = std::env::var_os("AQ_PROBE_SCRIPT") {
            let text = std::fs::read_to_string(script).unwrap_or_default();
            let (steps, pauses) = text.lines().filter_map(parse).unzip();
            return Probe::with(output, steps, pauses);
        }
        let mut steps = vec![("старт".to_owned(), Step::Wait)];
        for (i, note) in notes.into_iter().enumerate() {
            steps.push((format!("заметка {}", i + 1), Step::OpenNote(note)));
        }
        steps.push(("настройки открыты".to_owned(), Step::OpenSettings));
        steps.push(("настройки закрыты".to_owned(), Step::CloseSettings));
        steps.push(("граф открыт".to_owned(), Step::OpenGraph));
        steps.push(("граф +3 с".to_owned(), Step::Wait));
        steps.push(("вкладка графа закрыта".to_owned(), Step::CloseTab));
        steps.push(("простой".to_owned(), Step::Wait));
        let steps: Vec<_> = steps.into_iter().flat_map(|(name, step)| [(String::new(), step), (name.clone(), Step::Mark(name))]).collect();
        let pauses = steps.iter().map(|(name, _)| if name.is_empty() { PAUSE } else { Duration::ZERO }).collect();
        Probe::with(output, steps, pauses)
    }

    fn with(output: PathBuf, steps: Vec<(String, Step)>, pauses: Vec<Duration>) -> Self {
        Probe { output, steps, pauses, next: 0, due: None, cpu: std::cell::Cell::new(cpu_ms()), started: std::cell::Cell::new(Instant::now()) }
    }

    pub fn wake(&self) -> Option<Instant> {
        self.due
    }

    pub fn finished(&self) -> bool {
        self.next > self.steps.len()
    }

    pub fn step(&mut self, now: Instant) -> Option<&Step> {
        let due = *self.due.get_or_insert(now + PAUSE);
        if now < due || self.finished() {
            return None;
        }
        self.next += 1;
        if self.next > self.steps.len() {
            return None;
        }
        self.due = Some(Instant::now() + self.pauses[self.next - 1]);
        self.steps.get(self.next - 1).map(|(_, step)| step)
    }

    pub fn record(&self, name: &str, extra: &str) {
        let heap = CURRENT.load(Ordering::Relaxed);
        let peak = PEAK.load(Ordering::Relaxed);
        let cpu = cpu_ms();
        let wall = self.started.get().elapsed().as_secs_f64() * 1000.0;
        let mut text = format!("## {name}\nЦП за шаг {:.0} мс за {:.0} мс ({:.1}% одного ядра)\nrust-куча {:.1} МБ (пик {:.1} МБ)\n", cpu - self.cpu.get(), wall, (cpu - self.cpu.get()) / wall * 100.0, mb(heap), mb(peak));
        self.cpu.set(cpu);
        self.started.set(Instant::now());
        text.push_str(extra);
        text.push_str(&breakdown());
        if TRACE.load(Ordering::Relaxed) {
            text.push_str(&large_report());
        }
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&self.output) {
            let _ = file.write_all(text.as_bytes());
        }
    }
}
