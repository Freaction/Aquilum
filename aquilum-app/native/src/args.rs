use std::path::PathBuf;

use winit::dpi::PhysicalSize;

#[cfg(not(feature = "installed"))]
use crate::replay;

pub struct Args {
    pub vault: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub size: Option<PhysicalSize<u32>>,
    pub text: String,
    pub threads: u16,
    pub metrics: Option<PathBuf>,
    pub trace: bool,
    pub paint_flashing: bool,
    pub restore: bool,
    pub keep_position: bool,
    pub focus: bool,
    #[cfg(not(feature = "installed"))]
    pub replay: Option<(replay::Kind, PathBuf)>,
    pub open: Option<PathBuf>,
    #[cfg(not(feature = "installed"))]
    pub trace_frames: Option<PathBuf>,
    #[cfg(feature = "memory-probe")]
    pub memory_probe: Option<PathBuf>,
    #[cfg(feature = "memory-probe")]
    pub probe_notes: Vec<PathBuf>,
}

pub fn parse() -> Result<Args, String> {
    let mut args = Args {
        vault: None,
        data_dir: None,
        size: None,
        text: String::new(),
        threads: std::thread::available_parallelism().map_or(2, |n| n.get().clamp(2, 4) as u16),
        metrics: None,
        trace: false,
        paint_flashing: false,
        focus: false,
        restore: true,
        keep_position: false,
        #[cfg(not(feature = "installed"))]
        replay: None,
        open: None,
        #[cfg(not(feature = "installed"))]
        trace_frames: None,
        #[cfg(feature = "memory-probe")]
        memory_probe: None,
        #[cfg(feature = "memory-probe")]
        probe_notes: Vec::new(),
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
            #[cfg(not(feature = "installed"))]
            "--paint-flashing" => args.paint_flashing = true,
            "--focus" => args.focus = true,
            "--keep-position" => args.keep_position = true,
            #[cfg(not(feature = "installed"))]
            "--trace-frames" => args.trace_frames = Some(value()?.into()),
            #[cfg(feature = "memory-probe")]
            "--memory-probe" => args.memory_probe = Some(value()?.into()),
            #[cfg(feature = "memory-probe")]
            "--probe-note" => args.probe_notes.push(value()?.into()),
            "--open" => args.open = Some(value()?.into()),
            #[cfg(not(feature = "installed"))]
            "--replay-scroll" => args.replay = Some((replay::Kind::Scroll, value()?.into())),
            #[cfg(not(feature = "installed"))]
            "--replay-fling" => args.replay = Some((replay::Kind::Fling, value()?.into())),
            #[cfg(not(feature = "installed"))]
            "--replay-graph" => args.replay = Some((replay::Kind::Graph, value()?.into())),
            #[cfg(not(feature = "installed"))]
            "--replay-focus" => args.replay = Some((replay::Kind::Focus, value()?.into())),
            other => return Err(format!("неизвестный аргумент {other}")),
        }
    }
    Ok(args)
}
