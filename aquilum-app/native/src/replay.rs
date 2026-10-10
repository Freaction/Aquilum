use std::path::PathBuf;
use std::time::{Duration, Instant};

const STEP: Duration = Duration::from_millis(64);
const REFRESH_MS: f64 = 1000.0 / 120.0;

pub enum Step {
    Wheel(f32),
    Pinch(f32),
    Pan(f32, f32),
}

pub struct Replay {
    graph: bool,
    length: Duration,
    output: PathBuf,
    started: Option<Instant>,
    next: Option<Instant>,
    presents: Vec<Instant>,
    work: Vec<f64>,
}

impl Replay {
    pub fn new(seconds: f64, output: PathBuf) -> Self {
        Replay { graph: false, length: Duration::from_secs_f64(seconds), output, started: None, next: None, presents: Vec::new(), work: Vec::new() }
    }

    pub fn graph(output: PathBuf) -> Self {
        Replay { graph: true, ..Replay::new(20.0, output) }
    }

    pub fn is_graph(&self) -> bool {
        self.graph
    }

    pub fn step(&mut self, now: Instant) -> Option<Step> {
        if !self.graph {
            return self.scroll(now).map(Step::Wheel);
        }
        let started = *self.started.get_or_insert(now);
        let next = *self.next.get_or_insert(now);
        if now < next {
            return None;
        }
        let elapsed = now - started;
        if elapsed >= self.length {
            self.next = None;
            return None;
        }
        self.next = Some(next + Duration::from_millis(16));
        let phase = (elapsed.as_secs_f64() % 10.0) / 10.0;
        Some(match phase {
            p if p < 0.35 => Step::Pinch(0.02),
            p if p < 0.65 => Step::Pan(6.0, 3.0),
            _ => Step::Pinch(-0.02),
        })
    }

    pub fn wake(&self) -> Option<Instant> {
        self.next
    }

    pub fn scroll(&mut self, now: Instant) -> Option<f32> {
        let started = *self.started.get_or_insert(now);
        let next = *self.next.get_or_insert(now);
        if now < next {
            return None;
        }
        let elapsed = now - started;
        if elapsed >= self.length {
            self.next = None;
            return None;
        }
        self.next = Some(next + STEP);
        Some(if elapsed < self.length / 2 { -1.0 } else { 1.0 })
    }

    pub fn finished(&self) -> bool {
        self.started.is_some() && self.next.is_none()
    }

    pub fn frame(&mut self, presented: Instant, work: Duration) {
        if self.started.is_some() && self.next.is_some() {
            self.presents.push(presented);
            self.work.push(work.as_secs_f64() * 1000.0);
        }
    }

    pub fn write(&self) {
        let intervals: Vec<f64> = self.presents.windows(2).map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0).collect();
        let span = self.presents.last().zip(self.presents.first()).map_or(0.0, |(l, f)| (*l - *f).as_secs_f64());
        let average = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
        let mut sorted = intervals.clone();
        sorted.sort_by(f64::total_cmp);
        let p95 = sorted.get(((sorted.len().max(1) - 1) as f64 * 0.95) as usize).copied().unwrap_or(0.0);
        let missed = intervals.iter().filter(|&&i| i > REFRESH_MS * 1.5).count();
        let json = format!(
            "{{\"quarters\":{:?},\"frames\":{},\"fps\":{:.1},\"intervalAvgMs\":{:.2},\"intervalP95Ms\":{:.2},\"missed\":{},\"workAvgMs\":{:.2},\"long\":{:?}}}",
            self.work.chunks(self.work.len().div_ceil(4).max(1)).map(|c| (c.iter().sum::<f64>() / c.len() as f64 * 100.0).round() / 100.0).collect::<Vec<_>>(),
            self.presents.len(),
            if span > 0.0 { (self.presents.len().saturating_sub(1)) as f64 / span } else { 0.0 },
            average(&intervals),
            p95,
            missed,
            average(&self.work),
            intervals.iter().enumerate().filter(|(_, i)| **i > 30.0).map(|(n, i)| (n, i.round())).collect::<Vec<_>>(),
        );
        if let Err(error) = std::fs::write(&self.output, json) {
            eprintln!("замер прокрутки не записан: {error}");
        }
    }
}
