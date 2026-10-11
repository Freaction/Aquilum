use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

pub struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

const LARGE: usize = 256 * 1024;

thread_local! {
    static INSIDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

static LIVE: std::sync::Mutex<Option<HashMap<usize, (usize, std::backtrace::Backtrace)>>> = std::sync::Mutex::new(None);

fn track(ptr: *mut u8, size: usize) {
    if size < LARGE || !TRACE.load(Ordering::Relaxed) || INSIDE.with(|i| i.replace(true)) {
        return;
    }
    let trace = std::backtrace::Backtrace::force_capture();
    if let Ok(mut live) = LIVE.lock() {
        live.get_or_insert_with(HashMap::new).insert(ptr as usize, (size, trace));
    }
    INSIDE.with(|i| i.set(false));
}

fn untrack(ptr: *mut u8, size: usize) {
    if size < LARGE || !TRACE.load(Ordering::Relaxed) || INSIDE.with(|i| i.replace(true)) {
        return;
    }
    if let Ok(mut live) = LIVE.lock()
        && let Some(live) = live.as_mut()
    {
        live.remove(&(ptr as usize));
    }
    INSIDE.with(|i| i.set(false));
}

fn large_report() -> String {
    INSIDE.with(|i| i.set(true));
    let mut groups: HashMap<String, (usize, usize)> = HashMap::new();
    if let Ok(live) = LIVE.lock()
        && let Some(live) = live.as_ref()
    {
        for (size, trace) in live.values() {
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
            let entry = groups.entry(frames.join("
        <- ")).or_default();
            entry.0 += size;
            entry.1 += 1;
        }
    }
    let mut rows: Vec<_> = groups.into_iter().collect();
    rows.sort_by_key(|(_, (size, _))| std::cmp::Reverse(*size));
    let mut text = String::from("  крупные живые выделения Rust (≥256 КБ):
");
    for (key, (size, count)) in rows.into_iter().take(14) {
        text.push_str(&format!("    {:>7.1} МБ ×{count}  {key}
", mb(size)));
    }
    INSIDE.with(|i| i.set(false));
    text
}

fn grow(by: usize) {
    let now = CURRENT.fetch_add(by, Ordering::Relaxed) + by;
    PEAK.fetch_max(now, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            grow(layout.size());
            track(ptr, layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            grow(layout.size());
            track(ptr, layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        untrack(ptr, layout.size());
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            untrack(ptr, layout.size());
            CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
            grow(new_size);
            track(new, new_size);
        }
        new
    }
}

#[derive(Clone)]
pub enum Step {
    OpenNote(PathBuf),
    Same(PathBuf),
    Tab(usize),
    OpenSettings,
    CloseSettings,
    OpenGraph,
    CloseTab,
    Reader(PathBuf),
    CloseReader,
    Scroll(f64),
    Video,
    Wait,
    Mark(String),
    Evict(String),
    Minimize(bool),
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

#[cfg(target_os = "windows")]
fn cpu_ms() -> f64 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let (mut a, mut b, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    let _ = unsafe { GetProcessTimes(GetCurrentProcess(), &mut a, &mut b, &mut kernel, &mut user) };
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32 | u64::from(t.dwLowDateTime)) as f64;
    (ticks(kernel) + ticks(user)) / 10_000.0
}

#[cfg(not(target_os = "windows"))]
fn cpu_ms() -> f64 {
    0.0
}

const PAUSE: Duration = Duration::from_millis(3000);

impl Probe {
    pub fn new(output: PathBuf, notes: Vec<PathBuf>) -> Self {
        let _ = std::fs::write(&output, "");
        TRACE.store(std::env::var_os("AQ_PROBE_TRACE").is_some(), Ordering::Relaxed);
        if let Some(script) = std::env::var_os("AQ_PROBE_SCRIPT") {
            let text = std::fs::read_to_string(script).unwrap_or_default();
            let (steps, pauses) = text.lines().filter_map(parse).unzip();
            return Probe { output, steps, pauses, next: 0, due: None, cpu: std::cell::Cell::new(cpu_ms()), started: std::cell::Cell::new(Instant::now()) };
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

fn parse(line: &str) -> Option<((String, Step), Duration)> {
    let mut words = line.trim().splitn(3, ' ');
    let pause = Duration::from_millis(words.next()?.parse().ok()?);
    let action = words.next()?;
    let arg = words.next().unwrap_or("").to_owned();
    let step = match action {
        "open" => Step::OpenNote(arg.clone().into()),
        "same" => Step::Same(arg.clone().into()),
        "tab" => Step::Tab(arg.parse().ok()?),
        "settings" => Step::OpenSettings,
        "close-settings" => Step::CloseSettings,
        "graph" => Step::OpenGraph,
        "close-tab" => Step::CloseTab,
        "reader" => Step::Reader(arg.clone().into()),
        "close-reader" => Step::CloseReader,
        "scroll" => Step::Scroll(arg.parse().ok()?),
        "video" => Step::Video,
        "wait" => Step::Wait,
        "mark" => Step::Mark(arg.clone()),
        "evict" => Step::Evict(arg.clone()),
        "minimize" => Step::Minimize(true),
        "restore" => Step::Minimize(false),
        _ => return None,
    };
    Some(((arg, step), pause))
}

fn mb(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}

#[derive(Default, Clone, Copy)]
struct Usage {
    commit: usize,
    ws: usize,
    private_ws: usize,
}

#[cfg(target_os = "windows")]
fn breakdown() -> String {
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
    use windows::Win32::System::Memory::{MEM_COMMIT, MEM_IMAGE, MEM_MAPPED, MEMORY_BASIC_INFORMATION, VirtualQuery};
    use windows::Win32::System::ProcessStatus::{GetMappedFileNameW, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX, PSAPI_WORKING_SET_EX_INFORMATION, QueryWorkingSetEx};
    use windows::Win32::System::Threading::GetCurrentProcess;

    const PAGE: usize = 4096;
    let process = unsafe { GetCurrentProcess() };
    let mut groups: HashMap<String, Usage> = HashMap::new();
    let mut address = 0usize;
    let mut info = MEMORY_BASIC_INFORMATION::default();
    let mut name_buf = [0u16; 512];
    while unsafe { VirtualQuery(Some(address as *const _), &mut info, size_of::<MEMORY_BASIC_INFORMATION>()) } != 0 {
        let base = info.BaseAddress as usize;
        let size = info.RegionSize;
        address = base + size;
        if info.State != MEM_COMMIT {
            continue;
        }
        let key = if info.Type == MEM_IMAGE {
            let len = unsafe { GetModuleFileNameW(Some(HMODULE(info.AllocationBase)), &mut name_buf) } as usize;
            let path = String::from_utf16_lossy(&name_buf[..len]);
            format!("dll {}", path.rsplit('\\').next().unwrap_or(&path))
        } else if info.Type == MEM_MAPPED {
            let len = unsafe { GetMappedFileNameW(process, info.BaseAddress, &mut name_buf) } as usize;
            if len == 0 {
                "отображение без файла".to_owned()
            } else {
                let path = String::from_utf16_lossy(&name_buf[..len]);
                let file = path.rsplit('\\').next().unwrap_or(&path).to_owned();
                let ext = file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
                match ext.as_str() {
                    "ttf" | "otf" | "ttc" | "fon" => "файл шрифта".to_owned(),
                    "idx" | "pos" | "term" | "store" | "fast" | "fieldnorm" => "файл индекса tantivy".to_owned(),
                    _ => format!("файл {file}"),
                }
            }
        } else {
            "частная (кучи, стеки)".to_owned()
        };
        let pages = size / PAGE;
        let mut query: Vec<PSAPI_WORKING_SET_EX_INFORMATION> = (0..pages)
            .map(|i| PSAPI_WORKING_SET_EX_INFORMATION { VirtualAddress: (base + i * PAGE) as *mut _, ..Default::default() })
            .collect();
        let mut usage = Usage { commit: size, ..Usage::default() };
        if unsafe { QueryWorkingSetEx(process, query.as_mut_ptr().cast(), (query.len() * size_of::<PSAPI_WORKING_SET_EX_INFORMATION>()) as u32) }.is_ok() {
            for entry in &query {
                let flags = unsafe { entry.VirtualAttributes.Flags };
                if flags & 1 != 0 {
                    usage.ws += PAGE;
                    if flags & (1 << 15) == 0 {
                        usage.private_ws += PAGE;
                    }
                }
            }
        }
        let group = groups.entry(key).or_default();
        group.commit += usage.commit;
        group.ws += usage.ws;
        group.private_ws += usage.private_ws;
    }

    let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
    let _ = unsafe { GetProcessMemoryInfo(process, (&raw mut counters).cast(), size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32) };
    let total = groups.values().fold(Usage::default(), |a, u| Usage { commit: a.commit + u.commit, ws: a.ws + u.ws, private_ws: a.private_ws + u.private_ws });
    let mut text = format!(
        "частный рабочий набор {:.1} МБ · рабочий набор {:.1} МБ · private bytes {:.1} МБ · пик рабочего набора {:.1} МБ\n",
        mb(total.private_ws),
        mb(counters.WorkingSetSize),
        mb(counters.PrivateUsage),
        mb(counters.PeakWorkingSetSize)
    );
    text.push_str(&heaps());
    let mut sorted: Vec<_> = groups.into_iter().collect();
    sorted.sort_by_key(|(_, u)| std::cmp::Reverse(u.private_ws));
    for (name, u) in sorted.iter().take(12) {
        text.push_str(&format!("  {:>7.1} частн. · {:>7.1} РН · {:>7.1} выделено  {name}\n", mb(u.private_ws), mb(u.ws), mb(u.commit)));
    }
    text
}

#[cfg(target_os = "windows")]
fn heaps() -> String {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Memory::{GetProcessHeap, GetProcessHeaps, HeapLock, HeapUnlock, HeapWalk, PROCESS_HEAP_ENTRY};
    const PROCESS_HEAP_REGION: u16 = 1;
    const PROCESS_HEAP_ENTRY_BUSY: u16 = 4;

    let mut handles = vec![HANDLE::default(); 256];
    let count = unsafe { GetProcessHeaps(&mut handles) } as usize;
    handles.truncate(count.min(256));
    let mut sums = vec![(0usize, 0usize, 0usize); handles.len()];
    for (heap, sum) in handles.iter().zip(sums.iter_mut()) {
        if unsafe { HeapLock(*heap) }.is_err() {
            continue;
        }
        let mut entry = PROCESS_HEAP_ENTRY::default();
        while unsafe { HeapWalk(*heap, &mut entry) }.is_ok() {
            if entry.wFlags & PROCESS_HEAP_REGION != 0 {
                sum.0 += unsafe { entry.Anonymous.Region.dwCommittedSize } as usize;
            } else if entry.wFlags & PROCESS_HEAP_ENTRY_BUSY != 0 {
                sum.1 += entry.cbData as usize;
                if entry.cbData as usize >= 1 << 20 {
                    sum.2 += entry.cbData as usize;
                }
            }
        }
        let _ = unsafe { HeapUnlock(*heap) };
    }
    let process = unsafe { GetProcessHeap() }.ok();
    let mut text = String::from("  кучи Windows (выделено регионов · занято блоков · из них блоков ≥1 МБ):\n");
    let mut rows: Vec<_> = handles.iter().zip(sums).collect();
    rows.sort_by_key(|(_, s)| std::cmp::Reverse(s.0 + s.2));
    for (heap, (committed, busy, large)) in rows.into_iter().take(8) {
        let name = if Some(*heap) == process { "основная куча процесса" } else { "другая куча" };
        text.push_str(&format!("    {:>7.1} · {:>7.1} · {:>7.1}  {name} {:?}\n", mb(committed), mb(busy), mb(large), heap.0));
    }
    text
}

#[cfg(not(target_os = "windows"))]
fn breakdown() -> String {
    String::new()
}

#[cfg(test)]
pub(crate) fn heap() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn trace(on: bool) {
    TRACE.store(on, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn report() -> String {
    let mut text = breakdown();
    if TRACE.load(Ordering::Relaxed) {
        text.push_str(&large_report());
    }
    text
}
