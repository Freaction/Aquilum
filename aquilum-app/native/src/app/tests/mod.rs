mod adaptive;
mod compact_focus;
mod damage;
mod dialogs;
mod external;
mod folder_colors;
mod graph;
mod harness;
mod keys;
#[cfg(feature = "memory-probe")]
mod memory;
mod misc;
mod navigation;
mod notes;
mod panels;
mod perf;
mod reader;
mod snapshots;
mod sync_snapshot;
mod support;
mod tab_list;
mod templates;
mod text_files;
mod tree_pins;
mod tree_toolbar;
mod wide;
mod wiki;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use aquilum_core::settings::models::AppConfig;
use masonry::app::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
use masonry::core::keyboard::{Code, Key, KeyState, Location, Modifiers, NamedKey};
use masonry::core::{KeyboardEvent, PointerButton, PointerEvent, PointerState, TextEvent};
use masonry::dpi::{PhysicalPosition, PhysicalSize};
use masonry::kurbo::Size;

use super::{App, Outcome, Popup};
use crate::ui::settings::schema::Section;
use crate::ui::theme;
use crate::workspace::Workspace;
use crate::test_dir::{MOUSE, moved, pointer_at, primary};
use support::*;

const W: u32 = 1000;
const H: u32 = 600;

fn bench_size() -> Option<(u32, u32)> {
    let size = std::env::var("AQUILUM_BENCH_SIZE").ok()?;
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

struct Harness {
    app: App,
    root: RenderRoot,
    signals: Rc<RefCell<Vec<RenderRootSignal>>>,
    vault: tempfile::TempDir,
    _data: tempfile::TempDir,
    _theme: std::sync::MutexGuard<'static, ()>,
}
