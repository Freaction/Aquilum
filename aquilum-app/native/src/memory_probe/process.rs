#[cfg(target_os = "windows")]
use std::collections::HashMap;

pub(super) fn mb(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}

#[cfg(target_os = "windows")]
pub(super) fn cpu_ms() -> f64 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let (mut a, mut b, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    let _ = unsafe { GetProcessTimes(GetCurrentProcess(), &mut a, &mut b, &mut kernel, &mut user) };
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32 | u64::from(t.dwLowDateTime)) as f64;
    (ticks(kernel) + ticks(user)) / 10_000.0
}

#[cfg(not(target_os = "windows"))]
pub(super) fn cpu_ms() -> f64 {
    0.0
}

#[cfg(not(target_os = "windows"))]
pub(super) fn breakdown() -> String {
    String::new()
}

#[cfg(target_os = "windows")]
#[derive(Default, Clone, Copy)]
struct Usage {
    commit: usize,
    ws: usize,
    private_ws: usize,
}

#[cfg(target_os = "windows")]
pub(super) fn breakdown() -> String {
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
