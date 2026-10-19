use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use masonry::app::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
use masonry::core::keyboard::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey};
use masonry::core::{
    Ime as MasonryIme, PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo,
    PointerScrollEvent, PointerState, PointerType, PointerUpdate, ScrollDelta, TextEvent,
    WindowEvent as MasonryWindowEvent,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::run_on_demand::EventLoopExtRunOnDemand;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{self as wk, ModifiersState, PhysicalKey};
use winit::cursor::CursorIcon;
use winit::window::{
    ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData, ResizeDirection, Window, WindowAttributes, WindowId,
};

use crate::Args;
use crate::a11y::{self, Accessibility};
use crate::app::{App, WindowCommand};
use crate::window_state::WindowState;
use crate::workspace::Workspace;
use crate::metrics::Metrics;
use crate::render::Renderer;
use crate::ui;

mod create;
mod crash;
mod edges;
mod events;
mod frame;
mod keyboard;
mod launch;
mod pointer;
mod replay;
mod signals;
mod state;
mod window;
mod window_events;

pub(crate) use crash::report;
pub use launch::run;
use crash::install_crash_log;
use window::{cloak, set_background, window_icon};

pub(crate) const PRIMARY_MOUSE: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Mouse,
};

const RESIZE_BORDER: f64 = 6.0;
const NATIVE_FRAME: bool = cfg!(target_os = "macos");

struct Host {
    args: Args,
    app: App,
    started: Instant,
    window: Option<std::rc::Rc<dyn Window>>,
    output: Option<crate::output::Output>,
    root: Option<RenderRoot>,
    signals: Rc<RefCell<Vec<RenderRootSignal>>>,
    renderer: Renderer,
    modifiers: Modifiers,
    pointer: PointerState,
    moved: bool,
    ime_enabled: bool,
    ime_area: Option<(LogicalPosition<f64>, LogicalSize<f64>)>,
    last_anim: Option<Instant>,
    last_vblank: Option<Instant>,
    anim_pending: bool,
    redraw_pending: bool,
    full_next: bool,
    touchpad: Option<crate::touchpad::Touchpad>,
    vsync: crate::vsync::Vsync,
    metrics: Metrics,
    accessibility: Option<Accessibility>,
    window_state: WindowState,
    #[cfg(not(feature = "installed"))]
    replay: Option<crate::replay::Replay>,
    #[cfg(feature = "memory-probe")]
    probe: Option<crate::memory_probe::Probe>,
    cursor: CursorIcon,
    resize_edge: Option<ResizeDirection>,
    background: Option<masonry::peniko::Color>,
    shown: bool,
    occluded: bool,
    minimized: bool,
    layers: Option<masonry::app::VisualLayerPlan>,
    live_at: Option<Instant>,
    drop_at: winit::dpi::LogicalPosition<f64>,
    click: Option<(PointerButton, Instant, winit::dpi::PhysicalPosition<f64>, u8)>,
}
