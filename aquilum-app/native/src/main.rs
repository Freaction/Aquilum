//! Нативный интерфейс Aquilum без WebView: ядро виджетов Masonry на своём хосте
//! (winit 0.31 + vello_cpu 0.3, вывод через DXGI на Windows). См. `tauri-exit-plan` в базе знаний.
//!
//!   aquilum-native [--vault папка] [--data-dir папка] [--size ШxВ] [--file заметка.md] [--threads N]
//!                  [--metrics файл.json] [--trace] [--focus] [--open заметка.md]
//!                  [--replay-scroll файл.json] [--trace-frames файл.json]

// В релизе без консоли, иначе Start-Process в скрипте замера откроет лишнее окно.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod a11y;
mod autostart;
mod app;
mod cover;
mod poster;
mod graph;
mod reader;
mod video;
mod dates;
mod file_ops;
mod frontmatter;
mod i18n;
mod images;
mod instance;
mod interface;
mod links;
mod history;
mod host;
mod metrics;
mod navigation;
mod note;
mod output;
mod render;
mod replay;
mod session;
mod system;
mod tabs;
#[cfg(target_os = "windows")]
mod touchpad;
#[cfg(not(target_os = "windows"))]
mod touchpad {
    pub struct Touchpad;

    impl Touchpad {
        pub fn attach(_: &dyn winit::window::Window) -> Option<Self> {
            None
        }

        pub fn active(&self) -> bool {
            false
        }

        pub fn update(&self) -> (f32, f32, f32) {
            (0.0, 0.0, 1.0)
        }
    }
}
mod vault_files;
mod vsync;
mod ui;
mod updater;
mod window_state;
mod workspace;

use std::path::PathBuf;

use winit::dpi::PhysicalSize;

pub struct Args {
    /// База знаний; без неё — последняя открытая в каталоге данных.
    pub vault: Option<PathBuf>,
    /// Каталог данных ядра; по умолчанию отдельный от рабочей версии (см. `workspace`).
    pub data_dir: Option<PathBuf>,
    /// Физический размер окна для замеров; без него — сохранённый размер, как в рабочей версии.
    pub size: Option<PhysicalSize<u32>>,
    /// Текст для области редактора из `--file` (замеры отрисовки).
    pub text: String,
    /// Дополнительные потоки растеризации vello_cpu (0 — в основном потоке). По умолчанию 2:
    /// кадр на 5 Мп быстрее на треть ценой ~1 МБ (замер 2026-10-08).
    pub threads: u16,
    pub metrics: Option<PathBuf>,
    /// Печатать оконные события в stderr (консоль есть только в debug-сборке).
    pub trace: bool,
    /// Открыть заметку, открытую при прошлом закрытии (нет, если текст задан `--file`).
    pub restore: bool,
    /// Поставить окно туда, где оно было (`--keep-position`).
    pub keep_position: bool,
    /// Сразу поставить фокус в текст: замер с мигающей кареткой без участия мыши.
    pub focus: bool,
    pub replay: Option<PathBuf>,
    pub replay_graph: bool,
    pub open: Option<PathBuf>,
    pub trace_frames: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        vault: None,
        data_dir: None,
        size: None,
        text: String::new(),
        threads: std::thread::available_parallelism().map_or(2, |n| n.get().clamp(2, 4) as u16),
        metrics: None,
        trace: false,
        focus: false,
        restore: true,
        keep_position: false,
        replay: None,
        replay_graph: false,
        open: None,
        trace_frames: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag}: нет значения"));
        match flag.as_str() {
            "--vault" => args.vault = Some(value()?.into()),
            "--data-dir" => args.data_dir = Some(value()?.into()),
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size: нужно ШxВ, например 2881x1717")?;
                let parse = |s: &str| s.parse::<u32>().map_err(|e| format!("--size: {e}"));
                args.size = Some(PhysicalSize::new(parse(w)?, parse(h)?));
            }
            "--file" => {
                let path = value()?;
                args.text = std::fs::read_to_string(&path).map_err(|e| format!("--file {path}: {e}"))?;
                args.restore = false;
            }
            "--threads" => args.threads = value()?.parse().map_err(|e| format!("--threads: {e}"))?,
            "--metrics" => args.metrics = Some(value()?.into()),
            "--trace" => args.trace = true,
            "--focus" => args.focus = true,
            // Окно встаёт туда же, где было: для перезапуска при правке кода (scripts/watch.ps1).
            "--keep-position" => args.keep_position = true,
            "--trace-frames" => args.trace_frames = Some(value()?.into()),
            "--open" => args.open = Some(value()?.into()),
            "--replay-scroll" => args.replay = Some(value()?.into()),
            "--replay-graph" => {
                args.replay = Some(value()?.into());
                args.replay_graph = true;
            }
            // Флаг прогона прокрутки из шага 1: скрипт замера передаёт его всем окнам.
            "--bench-frames" => {
                value()?;
            }
            other => return Err(format!("неизвестный аргумент {other}")),
        }
    }
    Ok(args)
}

fn main() {
    if std::env::args().any(|argument| argument == "--mcp-stdio") {
        std::process::exit(aquilum_core::mcp::run_stdio_bridge(workspace::APP_ID));
    }
    let result = parse_args().and_then(host::run);
    if let Err(err) = &result {
        eprintln!("{err}");
    }
    std::process::exit(i32::from(result.is_err()));
}
