use std::cell::Cell;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Instant;

static ENABLED: AtomicBool = AtomicBool::new(false);
static EPOCH: OnceLock<Instant> = OnceLock::new();
static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());
static THREADS: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());
static NEXT_TID: AtomicU32 = AtomicU32::new(1);

thread_local! {
    static TID: Cell<u32> = const { Cell::new(0) };
}

enum Kind {
    Zone(f64),
    Mark,
    Counter(f64),
}

struct Event {
    name: &'static str,
    detail: &'static str,
    tid: u32,
    at: f64,
    kind: Kind,
}

pub fn start() {
    EPOCH.get_or_init(Instant::now);
    ENABLED.store(true, Ordering::Release);
}

#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

fn tid() -> u32 {
    TID.with(|t| {
        if t.get() == 0 {
            let id = NEXT_TID.fetch_add(1, Ordering::Relaxed);
            t.set(id);
            let name = std::thread::current().name().map_or_else(|| format!("поток {id}"), str::to_owned);
            THREADS.lock().unwrap().push((id, name));
        }
        t.get()
    })
}

fn micros(at: Instant) -> f64 {
    at.duration_since(*EPOCH.get().expect("aq_trace::start")).as_secs_f64() * 1e6
}

fn push(name: &'static str, detail: &'static str, at: Instant, kind: Kind) {
    let event = Event { name, detail, tid: tid(), at: micros(at), kind };
    EVENTS.lock().unwrap().push(event);
}

#[must_use]
pub struct Zone(Option<(&'static str, &'static str, Instant)>);

#[inline]
pub fn zone(name: &'static str) -> Zone {
    zone_of(name, "")
}

#[inline]
pub fn zone_of(name: &'static str, detail: &'static str) -> Zone {
    Zone(enabled().then(|| (name, detail, Instant::now())))
}

impl Drop for Zone {
    fn drop(&mut self) {
        if let Some((name, detail, start)) = self.0.take() {
            let dur = start.elapsed().as_secs_f64() * 1e6;
            push(name, detail, start, Kind::Zone(dur));
        }
    }
}

pub fn mark(name: &'static str) {
    if enabled() {
        push(name, "", Instant::now(), Kind::Mark);
    }
}

pub fn counter(name: &'static str, value: f64) {
    if enabled() {
        push(name, "", Instant::now(), Kind::Counter(value));
    }
}

pub fn write(path: &Path) -> std::io::Result<()> {
    let events = EVENTS.lock().unwrap();
    let mut out = String::from("{\"traceEvents\":[\n");
    for (id, name) in THREADS.lock().unwrap().iter() {
        let _ = writeln!(out, "{{\"ph\":\"M\",\"name\":\"thread_name\",\"pid\":1,\"tid\":{id},\"args\":{{\"name\":{name:?}}}}},");
    }
    for e in events.iter() {
        let name = if e.detail.is_empty() { e.name.to_owned() } else { format!("{}: {}", e.name, e.detail) };
        let _ = match e.kind {
            Kind::Zone(dur) => writeln!(out, "{{\"ph\":\"X\",\"name\":{name:?},\"pid\":1,\"tid\":{},\"ts\":{:.1},\"dur\":{:.1}}},", e.tid, e.at, dur),
            Kind::Mark => writeln!(out, "{{\"ph\":\"i\",\"s\":\"t\",\"name\":\"{}\",\"pid\":1,\"tid\":{},\"ts\":{:.1}}},", e.name, e.tid, e.at),
            Kind::Counter(v) => writeln!(out, "{{\"ph\":\"C\",\"name\":\"{}\",\"pid\":1,\"tid\":{},\"ts\":{:.1},\"args\":{{\"value\":{v}}}}},", e.name, e.tid, e.at),
        };
    }
    out.push_str("{\"ph\":\"M\",\"name\":\"process_name\",\"pid\":1,\"args\":{\"name\":\"aquilum-native\"}}]}\n");
    std::fs::write(path, out)
}
