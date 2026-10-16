use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::stats::mb;

static LIMIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(256 * 1024);

pub(super) fn configure() {
    let kb = std::env::var("AQ_PROBE_LARGE_KB").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(256);
    LIMIT.store(kb.max(1) * 1024, Ordering::Relaxed);
    TRACE.store(std::env::var_os("AQ_PROBE_TRACE").is_some(), Ordering::Relaxed);
}

thread_local! {
    static INSIDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) static TRACE: AtomicBool = AtomicBool::new(false);

static LIVE: std::sync::Mutex<Option<HashMap<usize, (usize, std::backtrace::Backtrace)>>> = std::sync::Mutex::new(None);

pub(super) fn track(ptr: *mut u8, size: usize) {
    if size < LIMIT.load(Ordering::Relaxed) || !TRACE.load(Ordering::Relaxed) || INSIDE.with(|i| i.replace(true)) {
        return;
    }
    let trace = std::backtrace::Backtrace::force_capture();
    if let Ok(mut live) = LIVE.lock() {
        live.get_or_insert_with(HashMap::new).insert(ptr as usize, (size, trace));
    }
    INSIDE.with(|i| i.set(false));
}

pub(super) fn untrack(ptr: *mut u8, size: usize) {
    if size < LIMIT.load(Ordering::Relaxed) || !TRACE.load(Ordering::Relaxed) || INSIDE.with(|i| i.replace(true)) {
        return;
    }
    if let Ok(mut live) = LIVE.lock()
        && let Some(live) = live.as_mut()
    {
        live.remove(&(ptr as usize));
    }
    INSIDE.with(|i| i.set(false));
}

fn frames(trace: &std::backtrace::Backtrace) -> Vec<String> {
    let text = trace.to_string();
    let mut frames = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let line = line.trim();
        let Some((_, name)) = line.split_once(": ") else { continue };
        if name.starts_with("std::") || name.starts_with("alloc::") || name.starts_with("core::") || name.contains("memory_probe") || name.starts_with("<alloc::") || name.starts_with("<core::") {
            continue;
        }
        let at = lines.clone().next().map(str::trim).unwrap_or("");
        let at = at.strip_prefix("at ").unwrap_or(at);
        let at = at.rsplit(['\\', '/']).take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/");
        let name = name.rsplit_once("::h").map_or(name, |(n, _)| n);
        frames.push(format!("{name} [{at}]"));
        if frames.len() == 4 {
            break;
        }
    }
    frames
}

pub(super) fn large_report() -> String {
    INSIDE.with(|i| i.set(true));
    let mut groups: HashMap<String, (usize, usize)> = HashMap::new();
    if let Ok(live) = LIVE.lock()
        && let Some(live) = live.as_ref()
    {
        for (size, trace) in live.values() {
            let entry = groups.entry(frames(trace).join("\n        <- ")).or_default();
            entry.0 += size;
            entry.1 += 1;
        }
    }
    let mut rows: Vec<_> = groups.into_iter().collect();
    rows.sort_by_key(|(_, (size, _))| std::cmp::Reverse(*size));
    let mut text = String::from("  крупные живые выделения Rust (от порога AQ_PROBE_LARGE_KB, по умолчанию 256 КБ):\n");
    for (key, (size, count)) in rows.into_iter().take(24) {
        text.push_str(&format!("    {:>7.1} МБ ×{count}  {key}\n", mb(size)));
    }
    INSIDE.with(|i| i.set(false));
    text
}
