//! Хост `RenderRoot`: окно winit 0.31, перевод событий в ui-events, сигналы Masonry, отрисовка
//! через свой адаптер на vello_cpu 0.3 (`crate::render`) и вывод кадра (`crate::output`).
//!
//! `masonry_winit` не используется: у него только GPU-бэкенды и winit 0.30.

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

mod crash;
mod events;
mod frame;
mod input;
mod signals;
mod replay;
mod window;

pub(crate) use crash::report;
use crash::install_crash_log;
use window::{cloak, set_background, window_icon};

const PRIMARY_MOUSE: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Mouse,
};

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
    ime_enabled: bool,
    /// Последняя отправленная в IME область каретки: Masonry шлёт ImeMoved после каждого прохода.
    ime_area: Option<(LogicalPosition<f64>, LogicalSize<f64>)>,
    clipboard: Option<arboard::Clipboard>,
    /// Время прошлого анимационного тика, пока анимация продолжается.
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

const RESIZE_BORDER: f64 = 6.0;
const NATIVE_FRAME: bool = cfg!(target_os = "macos");

pub fn run(args: Args) -> Result<(), String> {
    #[cfg(not(feature = "installed"))]
    if args.trace_frames.is_some() {
        aq_trace::start();
    }
    #[cfg(not(feature = "installed"))]
    let replay = args.replay.clone().map(|(kind, out)| crate::replay::Replay::new(kind, out));
    let data_dir = crate::workspace::data_dir_or_default(args.data_dir.as_deref())?;
    let crate::instance::Instance::First(_lock) = crate::instance::acquire(&data_dir) else {
        crate::instance::activate();
        return Ok(());
    };
    crate::autostart::repoint();
    let event_loop = EventLoop::new().map_err(|e| format!("цикл событий: {e}"))?;
    let proxy = event_loop.create_proxy();
    let vsync_proxy = event_loop.create_proxy();
    let vsync = crate::vsync::Vsync::new(move || vsync_proxy.wake_up());
    let workspace = Workspace::open(Some(&data_dir), args.vault.as_deref(), move || proxy.wake_up())?;
    install_crash_log(workspace.data_dir.join("crash.log"));
    let window_state = WindowState::load(&workspace.data_dir);
    let app = App::new(workspace, args.text.clone());
    let threads = args.threads;
    let metrics = Metrics::new(threads, args.metrics.clone());
    let mut renderer = Renderer::new(threads);
    if args.paint_flashing {
        renderer.toggle_overlay();
    }
    #[allow(unused_mut)]
    let mut host = Host {
        args,
        app,
        started: Instant::now(),
        window: None,
        output: None,
        root: None,
        signals: Rc::default(),
        renderer,
        modifiers: Modifiers::empty(),
        pointer: PointerState::default(),
        ime_enabled: false,
        ime_area: None,
        clipboard: arboard::Clipboard::new().ok(),
        last_anim: None,
        last_vblank: None,
        anim_pending: false,
        redraw_pending: false,
        full_next: true,
        touchpad: None,
        vsync,
        metrics,
        accessibility: None,
        window_state,
        #[cfg(not(feature = "installed"))]
        replay,
        #[cfg(feature = "memory-probe")]
        probe: None,
        cursor: CursorIcon::Default,
        resize_edge: None,
        background: None,
        shown: false,
        occluded: false,
        minimized: false,
        layers: None,
        live_at: None,
        drop_at: winit::dpi::LogicalPosition::new(0.0, 0.0),
        click: None,
    };
    #[cfg(feature = "memory-probe")]
    {
        host.probe = host.args.memory_probe.clone().map(|out| crate::memory_probe::Probe::new(out, host.args.probe_notes.clone()));
    }
    event_loop.run_app(host).map_err(|e| format!("цикл событий завершился с ошибкой: {e}"))
}

impl Host {
    fn window(&self) -> &dyn Window {
        self.window.as_deref().expect("окно создано")
    }

    /// Масштаб, которым живёт Masonry: масштаб окна, умноженный на масштаб интерфейса.
    /// Логические пиксели Masonry — «rem» интерфейса, как во фронтенде.
    fn scale(&self) -> f64 {
        self.window().scale_factor() * self.app.scale()
    }

    /// Масштаб интерфейса изменился: Masonry перекладывает окно, кадр рисуется заново.
    fn rescale(&mut self) {
        let scale = self.scale();
        let window = self.logical_size();
        if let Some(root) = &mut self.root {
            ui::rescale(root, scale);
            self.app.window_resized(root, window);
        }
        self.request_redraw();
    }

    fn logical_size(&self) -> masonry::kurbo::Size {
        let w = self.window();
        let size = w.surface_size().to_logical::<f64>(w.scale_factor() * self.app.scale());
        masonry::kurbo::Size::new(size.width, size.height)
    }

    fn request_redraw(&mut self) {
        self.redraw_pending = true;
        if !self.occluded {
            self.vsync.request();
        }
    }

    fn show_title(&self) {
        let Some(window) = self.window.as_deref() else { return };
        let title = match self.metrics.summary() {
            Some(summary) => format!("{} · {summary}", self.app.title()),
            None => self.app.title(),
        };
        window.set_title(&title);
    }

    fn touchpad_scroll(&mut self, event_loop: &dyn ActiveEventLoop, vblank: Instant) {
        let Some(touchpad) = &self.touchpad else { return };
        let (dx, dy, pinch) = touchpad.update(vblank);
        if pinch != 1.0 {
            self.pinch(f64::from(pinch) - 1.0);
        }
        if (dx, dy) == (0.0, 0.0) {
            self.handle_signals(event_loop);
            return;
        }
        aq_trace::counter("тачпад: пиксели", f64::from(dy));
        let state = self.pointer_state();
        let delta = ScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(f64::from(dx), f64::from(dy)));
        self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta, state }));
        self.handle_signals(event_loop);
    }

    fn trim(&mut self) {
        self.renderer.trim();
        if let Some(output) = &mut self.output {
            output.trim();
        }
        self.layers = None;
        self.full_next = true;
        crate::images::trim();
        crate::cover::trim();
    }

    fn close(&mut self, event_loop: &dyn ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }
        #[cfg(not(feature = "installed"))]
        if let Some(out) = &self.args.trace_frames {
            let _ = aq_trace::write(out);
        }
        event_loop.exit();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if self.args.size.is_none() {
            if let Some(window) = &self.window {
                self.window_state.persist(&**window);
            }
        }
        self.app.shutdown();
        self.metrics.write();
    }
}
