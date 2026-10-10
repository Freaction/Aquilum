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
    anim_pending: bool,
    redraw_pending: bool,
    full_next: bool,
    touchpad: Option<crate::touchpad::Touchpad>,
    vsync: crate::vsync::Vsync,
    metrics: Metrics,
    accessibility: Option<Accessibility>,
    window_state: WindowState,
    perf: Option<Perf>,
    replay: Option<crate::replay::Replay>,
    cursor: CursorIcon,
    resize_edge: Option<ResizeDirection>,
}

const RESIZE_BORDER: f64 = 6.0;

#[derive(Default)]
struct Perf {
    since: Option<Instant>,
    frames: u32,
    sums: [Duration; 4],
    worst: Duration,
    summary: String,
}

impl Perf {
    fn frame(&mut self, parts: [Duration; 4]) -> bool {
        let since = *self.since.get_or_insert_with(Instant::now);
        self.frames += 1;
        for (sum, part) in self.sums.iter_mut().zip(parts) {
            *sum += part;
        }
        self.worst = self.worst.max(parts[3]);
        let elapsed = since.elapsed();
        if elapsed < Duration::from_secs(1) {
            return false;
        }
        let ms = |d: Duration| d.as_secs_f64() * 1000.0 / f64::from(self.frames);
        self.summary = format!(
            "{:.0} к/с · кадр {:.1} мс, худший {:.1} (сцена {:.1}, растр {:.1}, вывод {:.1})",
            f64::from(self.frames) / elapsed.as_secs_f64(),
            ms(self.sums[3]),
            self.worst.as_secs_f64() * 1000.0,
            ms(self.sums[0]),
            ms(self.sums[1]),
            ms(self.sums[2]),
        );
        *self = Perf { summary: std::mem::take(&mut self.summary), ..Perf::default() };
        true
    }
}

static CRASH_LOG: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

fn latin(code: Code) -> Option<char> {
    const LETTERS: [Code; 26] = [
        Code::KeyA, Code::KeyB, Code::KeyC, Code::KeyD, Code::KeyE, Code::KeyF, Code::KeyG, Code::KeyH, Code::KeyI,
        Code::KeyJ, Code::KeyK, Code::KeyL, Code::KeyM, Code::KeyN, Code::KeyO, Code::KeyP, Code::KeyQ, Code::KeyR,
        Code::KeyS, Code::KeyT, Code::KeyU, Code::KeyV, Code::KeyW, Code::KeyX, Code::KeyY, Code::KeyZ,
    ];
    LETTERS.iter().position(|&c| c == code).map(|i| char::from(b'a' + i as u8))
}

pub(crate) fn report(message: &str) {
    eprintln!("{message}");
    let Some(file) = CRASH_LOG.get() else { return };
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    if let Ok(mut log) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        let _ = std::io::Write::write_all(&mut log, format!("[{stamp}] {message}\n").as_bytes());
    }
}

fn install_crash_log(file: std::path::PathBuf) {
    let _ = CRASH_LOG.set(file.clone());
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let thread = std::thread::current().name().unwrap_or("?").to_owned();
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{stamp}] поток {thread}: {info}
{backtrace}

");
        if let Ok(mut log) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
            let _ = std::io::Write::write_all(&mut log, entry.as_bytes());
        }
        default(info);
    }));
}

#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(hwnd: *mut std::ffi::c_void, attribute: u32, value: *const std::ffi::c_void, size: u32) -> i32;
}

fn cloak(window: &dyn Window, cloaked: bool) {
    #[cfg(target_os = "windows")]
    {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Some(RawWindowHandle::Win32(handle)) = window.window_handle().ok().map(|h| h.as_raw()) else { return };
        let value = i32::from(cloaked);
        unsafe {
            DwmSetWindowAttribute(handle.hwnd.get() as *mut std::ffi::c_void, 13, (&raw const value).cast(), 4);
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (window, cloaked);
}


fn window_icon() -> Option<winit::icon::Icon> {
    let image = image::load_from_memory(include_bytes!("../assets/icon-64.png")).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    winit::icon::RgbaIcon::new(image.into_raw(), width, height).ok().map(winit::icon::Icon::from)
}

pub fn run(args: Args) -> Result<(), String> {
    if args.trace_frames.is_some() {
        aq_trace::start();
    }
    let replay = args.replay.clone().map(|o| if args.replay_graph { crate::replay::Replay::graph(o) } else { crate::replay::Replay::new(4.0, o) });
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
    let host = Host {
        args,
        app,
        started: Instant::now(),
        window: None,
        output: None,
        root: None,
        signals: Rc::default(),
        renderer: Renderer::new(threads),
        modifiers: Modifiers::empty(),
        pointer: PointerState::default(),
        ime_enabled: false,
        ime_area: None,
        clipboard: arboard::Clipboard::new().ok(),
        last_anim: None,
        anim_pending: false,
        redraw_pending: false,
        full_next: true,
        touchpad: None,
        vsync,
        metrics: Metrics::new(threads),
        accessibility: None,
        window_state,
        perf: None,
        replay,
        cursor: CursorIcon::Default,
        resize_edge: None,
    };
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
        self.vsync.request();
    }

    fn show_title(&self) {
        let title = match &self.perf {
            Some(perf) => format!("{} · {}", self.app.title(), perf.summary),
            None => self.app.title(),
        };
        self.window().set_title(&title);
    }

    fn replay_scroll(&mut self, event_loop: &dyn ActiveEventLoop, now: Instant) {
        if self.metrics.frames == 0 {
            return;
        }
        if self.replay.as_ref().is_some_and(crate::replay::Replay::is_graph) {
            let Some(root) = &mut self.root else { return };
            if !self.app.graph_ready() {
                self.app.open_graph_for_replay(root);
                return;
            }
        }
        let Some(step) = self.replay.as_mut().and_then(|r| r.step(now)) else {
            if self.replay.as_ref().is_some_and(crate::replay::Replay::finished) {
                if let Some(replay) = &self.replay {
                    replay.write();
                }
                self.close(event_loop);
            }
            return;
        };
        let _zone = aq_trace::zone("прогон: колесо");
        let scale = self.scale();
        let target = self.app.graph_view().or_else(|| self.app.editor_id());
        let center = self.root.as_ref().zip(target).and_then(|(root, id)| root.get_widget(id)).map(|body| body.ctx().bounding_box().center());
        if let Some(center) = center {
            self.pointer.position = winit::dpi::PhysicalPosition::new(center.x * scale * 0.8, (center.y * scale).min(f64::from(self.window().surface_size().height) * 0.8));
        }
        let state = self.pointer_state();
        match step {
            crate::replay::Step::Wheel(lines) => {
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta: ScrollDelta::LineDelta(0.0, lines), state }));
            }
            crate::replay::Step::Pinch(delta) => self.pinch(f64::from(delta)),
            crate::replay::Step::Pan(dx, dy) => {
                let delta = ScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(f64::from(dx), f64::from(dy)));
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta, state }));
            }
        }
        self.handle_signals(event_loop);
    }

    fn touchpad_scroll(&mut self, event_loop: &dyn ActiveEventLoop) {
        let Some(touchpad) = &self.touchpad else { return };
        let (dx, dy, pinch) = touchpad.update();
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

    fn close(&mut self, event_loop: &dyn ActiveEventLoop) {
        if let Some(out) = &self.args.trace_frames {
            let _ = aq_trace::write(out);
        }
        if self.args.size.is_none() {
            if let Some(window) = &self.window {
                self.window_state.persist(&**window);
            }
        }
        self.app.shutdown();
        self.metrics.write(self.args.metrics.as_deref());
        event_loop.exit();
    }

    fn update_resize_edge(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let window = self.window();
        let edge = if window.is_maximized() {
            None
        } else {
            let size = window.surface_size();
            let border = RESIZE_BORDER * window.scale_factor();
            let corner = border * 2.0;
            let (w, h) = (f64::from(size.width), f64::from(size.height));
            let (x, y) = (position.x, position.y);
            let left = x < border;
            let right = x > w - border;
            let top = y < border;
            let bottom = y > h - border;
            let near_left = x < corner;
            let near_right = x > w - corner;
            let near_top = y < corner;
            let near_bottom = y > h - corner;
            if (top && near_left) || (left && near_top) {
                Some(ResizeDirection::NorthWest)
            } else if (top && near_right) || (right && near_top) {
                Some(ResizeDirection::NorthEast)
            } else if (bottom && near_left) || (left && near_bottom) {
                Some(ResizeDirection::SouthWest)
            } else if (bottom && near_right) || (right && near_bottom) {
                Some(ResizeDirection::SouthEast)
            } else if top {
                Some(ResizeDirection::North)
            } else if bottom {
                Some(ResizeDirection::South)
            } else if left {
                Some(ResizeDirection::West)
            } else if right {
                Some(ResizeDirection::East)
            } else {
                None
            }
        };
        if edge != self.resize_edge {
            self.resize_edge = edge;
            let cursor = edge.map_or(self.cursor, CursorIcon::from);
            self.window().set_cursor(cursor.into());
        }
    }

    fn time_ns(&self) -> u64 {
        self.started.elapsed().as_nanos() as u64
    }

    fn pointer_state(&mut self) -> PointerState {
        self.pointer.time = self.time_ns();
        self.pointer.modifiers = self.modifiers;
        self.pointer.scale_factor = self.scale();
        self.pointer.clone()
    }

    fn pinch(&mut self, delta: f64) {
        if !delta.is_finite() || delta == 0.0 {
            return;
        }
        let state = self.pointer_state();
        let gesture = masonry::core::PointerGestureEvent { pointer: PRIMARY_MOUSE, gesture: masonry::core::PointerGesture::Pinch(delta as f32), state };
        self.pointer_event(PointerEvent::Gesture(gesture));
    }

    fn pointer_event(&mut self, event: PointerEvent) {
        if let Some(root) = &mut self.root {
            root.handle_pointer_event(event);
        }
    }

    fn text_event(&mut self, event: TextEvent) {
        if let Some(root) = &mut self.root {
            root.handle_text_event(event);
        }
    }

    fn keyboard(&mut self, event: winit::event::KeyEvent) {
        let key = match event.logical_key {
            wk::Key::Named(named) => Key::Named(named),
            wk::Key::Character(s) => Key::Character(s.to_string()),
            wk::Key::Dead(_) => Key::Named(NamedKey::Dead),
            wk::Key::Unidentified(_) => Key::Named(NamedKey::Unidentified),
        };
        let code = match event.physical_key {
            PhysicalKey::Code(code) => code,
            PhysicalKey::Unidentified(_) => Code::Unidentified,
        };
        let state = match event.state {
            ElementState::Pressed => KeyState::Down,
            ElementState::Released => KeyState::Up,
        };
        let action_modifier = if cfg!(target_os = "macos") { self.modifiers.meta() } else { self.modifiers.ctrl() };
        let key = match latin(code) {
            Some(letter) if action_modifier && !self.modifiers.alt() => Key::Character(letter.to_string()),
            _ => key,
        };
        if state == KeyState::Down
            && let Some(root) = &mut self.root
        {
            let modifiers = self.modifiers;
            if action_modifier && code == Code::KeyO {
                self.app.open_search(root);
                return;
            }
            if action_modifier && modifiers.shift() && code == Code::KeyP {
                self.perf = if self.perf.is_some() { None } else { Some(Perf::default()) };
                self.show_title();
                return;
            }
            if action_modifier && modifiers.shift() && code == Code::KeyF {
                self.app.toggle_focus_mode(root);
                return;
            }
            if let Some(outcome) = self.app.search_key(root, &key, modifiers) {
                if outcome.title {
                    self.show_title();
                }
                return;
            }
        }
        if state == KeyState::Down
            && action_modifier
            && let Key::Character(c) = &key
            && self.app.zoom_key(c)
        {
            self.rescale();
            if let Some(root) = &mut self.root {
                self.app.scale_changed(root);
            }
            return;
        }
        let is_paste = state == KeyState::Down
            && self.modifiers.ctrl()
            && matches!(&key, Key::Character(c) if c.eq_ignore_ascii_case("v"));
        if is_paste {
            if let Some(text) = self.clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                self.text_event(TextEvent::ClipboardPaste(text));
            }
            return;
        }
        let location: Location = event.location;
        self.text_event(TextEvent::Keyboard(KeyboardEvent {
            state,
            key,
            code,
            location,
            modifiers: self.modifiers,
            repeat: event.repeat,
            is_composing: false,
        }));
    }

    fn handle_signals(&mut self, event_loop: &dyn ActiveEventLoop) {
        let signals = std::mem::take(&mut *self.signals.borrow_mut());
        let mut actions = Vec::new();
        for signal in signals {
            if matches!(signal, RenderRootSignal::RequestRedraw) {
                self.request_redraw();
                continue;
            }
            let window = self.window.as_deref().expect("окно создано");
            match signal {
                RenderRootSignal::RequestRedraw => {}
                RenderRootSignal::RequestAnimFrame => {
                    self.anim_pending = true;
                    self.vsync.request();
                }
                RenderRootSignal::StartIme => {
                    if !self.ime_enabled {
                        // winit 0.31 отклоняет запрос, если возможность заявлена без начального
                        // значения; настоящую позицию каретки Masonry пришлёт сигналом ImeMoved.
                        let data = ImeRequestData::default().with_cursor_area(
                            Position::Logical((0.0, 0.0).into()),
                            Size::Logical((1.0, 1.0).into()),
                        );
                        let request = ImeEnableRequest::new(ImeCapabilities::new().with_cursor_area(), data)
                            .expect("запрос IME: возможности и данные согласованы");
                        self.ime_enabled = window.request_ime_update(ImeRequest::Enable(request)).is_ok();
                    }
                }
                RenderRootSignal::EndIme => {
                    let _ = window.request_ime_update(ImeRequest::Disable);
                    self.ime_enabled = false;
                    self.ime_area = None;
                }
                RenderRootSignal::ImeMoved(position, size) => {
                    if self.ime_enabled && self.ime_area != Some((position, size)) {
                        self.ime_area = Some((position, size));
                        // Логические пиксели Masonry включают масштаб интерфейса, а winit ждёт
                        // логические пиксели окна.
                        let k = self.app.scale();
                        let position = LogicalPosition::new(position.x * k, position.y * k);
                        let size = LogicalSize::new(size.width * k, size.height * k);
                        let data = ImeRequestData::default()
                            .with_cursor_area(Position::Logical(position), Size::Logical(size));
                        let _ = window.request_ime_update(ImeRequest::Update(data));
                    }
                }
                RenderRootSignal::ClipboardStore(text) => {
                    if let Some(clipboard) = &mut self.clipboard {
                        let _ = clipboard.set_text(text);
                    }
                }
                RenderRootSignal::SetCursor(cursor) => {
                    self.cursor = cursor;
                    if self.resize_edge.is_none() {
                        window.set_cursor(cursor.into());
                    }
                }
                RenderRootSignal::SetTitle(title) => window.set_title(&title),
                RenderRootSignal::TakeFocus => window.focus_window(),
                RenderRootSignal::Exit => event_loop.exit(),
                RenderRootSignal::NewLayer(_, root, pos) => {
                    self.root.as_mut().expect("корень есть").add_layer(root, pos)
                }
                RenderRootSignal::RemoveLayer(id) => self.root.as_mut().expect("корень есть").remove_layer(id),
                RenderRootSignal::RepositionLayer(id, pos) => {
                    self.root.as_mut().expect("корень есть").reposition_layer(id, pos)
                }
                RenderRootSignal::Action(action, id) => actions.push((action, id)),
                // Перетаскивание окна, меню окна — по мере переноса интерфейса.
                _ => {}
            }
        }
        // Действия обрабатываются после разбора сигналов: им нужен изменяемый RenderRoot.
        let mut title_changed = false;
        let mut rescale = false;
        let mut commands = Vec::new();
        let window = self.logical_size();
        if let Some(root) = &mut self.root {
            for (action, id) in actions {
                let outcome = self.app.on_action(root, id, action, window);
                title_changed |= outcome.title;
                rescale |= outcome.rescale;
                commands.extend(outcome.window);
            }
        }
        if rescale {
            self.rescale();
        }
        for command in commands {
            let window = self.window();
            match command {
                WindowCommand::Drag => {
                    let _ = window.drag_window();
                }
                WindowCommand::ToggleMaximize => window.set_maximized(!window.is_maximized()),
                WindowCommand::Minimize => window.set_minimized(true),
                WindowCommand::Close => {
                    self.close(event_loop);
                    return;
                }
            }
        }
        if title_changed {
            self.show_title();
        }
        // Обработка действий могла породить новые сигналы (перерисовка, фокус).
        if !self.signals.borrow().is_empty() {
            self.handle_signals(event_loop);
        }
    }


    fn redraw(&mut self) {
        let _zone = aq_trace::zone("кадр");
        let t0 = Instant::now();
        let (Some(window), Some(output), Some(root)) = (self.window.clone(), self.output.as_mut(), self.root.as_mut()) else { return };
        let size = window.surface_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        if let Err(err) = output.resize(size.width, size.height) {
            report(&err);
            return;
        }
        let (layers, access_update) = {
            let _zone = aq_trace::zone("сцена");
            root.run_rewrite_passes();
            self.app.restore_view(root);
            root.redraw()
        };
        let scene = t0.elapsed();
        if let (Some(update), Some(accessibility)) = (access_update, self.accessibility.as_mut()) {
            if self.args.trace {
                eprintln!("доступность: обновление дерева, узлов {}", update.nodes.len());
            }
            accessibility.update(update);
        }
        if self.app.holding() {
            return;
        }
        let (width, height) = (size.width.min(u32::from(u16::MAX)) as u16, size.height.min(u32::from(u16::MAX)) as u16);
        let scale = window.scale_factor() * self.app.scale();
        window.pre_present_notify();
        let full = std::mem::take(&mut self.full_next) || output.fresh();
        let raster_start = Instant::now();
        let drawn = match self.renderer.draw(&layers, width, height, scale, ui::theme::background(), full) {
            Ok(drawn) => drawn,
            Err(err) => {
                report(&format!("кадр не нарисован: {err}"));
                self.full_next = true;
                return;
            }
        };
        let raster = raster_start.elapsed();
        let present_start = Instant::now();
        if let Some((area, pixels)) = drawn {
            output.upload(area, pixels);
            output.present(area);
        }
        let present = present_start.elapsed();
        if let Some(replay) = &mut self.replay {
            replay.frame(Instant::now(), t0.elapsed());
        }

        self.metrics.frame(self.started, raster, t0.elapsed());
        if self.metrics.frames == 1 {
            self.metrics.write(self.args.metrics.as_deref());
        }
        if let Some(perf) = &mut self.perf
            && perf.frame([scene, raster, present, t0.elapsed()])
        {
            self.show_title();
        }
    }

    /// Анимационный тик: проходы обновления Masonry без растеризации.
    fn anim_tick(&mut self, event_loop: &dyn ActiveEventLoop) {
        let _zone = aq_trace::zone("тик анимации");
        let Some(root) = &mut self.root else { return };
        let now = Instant::now();
        let elapsed = self.last_anim.take().map(|t| now - t).unwrap_or(Duration::ZERO);
        root.handle_window_event(MasonryWindowEvent::AnimFrame(elapsed));
        self.last_anim = root.needs_anim().then_some(now);
        self.metrics.anim_ticks += 1;
        self.handle_signals(event_loop);
    }
}

impl ApplicationHandler for Host {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attributes = WindowAttributes::default()
            .with_title("Aquilum")
            .with_decorations(false)
            .with_visible(false)
            .with_window_icon(window_icon());
        #[cfg(target_os = "windows")]
        let attributes = attributes.with_platform_attributes(Box::new(
            winit::platform::windows::WindowAttributesWindows::default().with_undecorated_shadow(true).with_class_name(crate::instance::WINDOW_CLASS),
        ));
        let attributes = match self.args.size {
            Some(size) => attributes.with_surface_size(size),
            None => attributes.with_surface_size(self.window_state.size()),
        };
        let attributes = match self.window_state.position().filter(|_| self.args.keep_position) {
            Some(position) => attributes.with_position(position),
            None => attributes,
        };
        let window = event_loop.create_window(attributes).expect("окно");
        self.accessibility = Accessibility::attach(&*window, event_loop.create_proxy());
        cloak(&*window, true);
        window.set_visible(true);
        if self.args.size.is_none() && self.window_state.maximized() {
            window.set_maximized(true);
        }
        self.metrics.window_created(self.started, window.scale_factor());
        let options = RenderRootOptions {
            default_properties: Arc::new(ui::theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: window.surface_size(),
            scale_factor: window.scale_factor() * self.app.scale(),
            test_font: None,
        };
        let signals = self.signals.clone();
        let root = RenderRoot::new(
            self.app.root_widget(window.theme() == Some(winit::window::Theme::Dark)),
            move |signal| signals.borrow_mut().push(signal),
            options,
        );
        let mut root = root;
        ui::theme::register_fonts(&mut root);
        if !self.args.restore {
            self.app.forget_session();
        }
        let logical = window.surface_size().to_logical::<f64>(window.scale_factor() * self.app.scale());
        self.app.attach_window_controls(&mut root, masonry::kurbo::Size::new(logical.width, logical.height));
        if self.app.restore(&mut root) {
            window.set_title(&self.app.title());
        }
        let window: std::rc::Rc<dyn Window> = std::rc::Rc::from(window);
        match crate::output::Output::new(event_loop.owned_display_handle(), window.clone()) {
            Ok(output) => self.output = Some(output),
            Err(err) => report(&format!("вывод кадра: {err}")),
        }
        self.touchpad = crate::touchpad::Touchpad::attach(&*window);
        self.window = Some(window);
        if let Some(note) = self.args.open.clone() {
            self.app.open(&mut root, note);
        }
        if self.args.focus {
            root.focus_on(self.app.editor_id());
        }
        self.root = Some(root);
        self.handle_signals(event_loop);
        self.redraw();
        cloak(self.window(), false);
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        let _zone = aq_trace::zone("пробуждение");
        let mut retitle = false;
        if let Some(root) = &mut self.root {
            retitle = self.app.on_core_events(root);
            for request in self.accessibility.as_ref().map(Accessibility::drain).unwrap_or_default() {
                match request {
                    a11y::Request::Activate => {
                        if self.args.trace {
                            eprintln!("доступность: клиент подключился");
                        }
                        root.handle_window_event(MasonryWindowEvent::EnableAccessTree);
                    }
                    a11y::Request::Action(action) => root.handle_access_event(action),
                }
            }
        }
        if retitle {
            self.show_title();
        }
        self.handle_signals(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        let _zone = aq_trace::zone("конец пачки событий");
        let now = Instant::now();
        self.app.tick(now);
        if self.app.hold_due(now) {
            self.request_redraw();
        }
        self.replay_scroll(event_loop, now);
        if self.touchpad.as_ref().is_some_and(crate::touchpad::Touchpad::active) {
            self.vsync.request();
        }
        if self.vsync.take() {
            self.touchpad_scroll(event_loop);
            if std::mem::take(&mut self.anim_pending) {
                self.anim_tick(event_loop);
            }
            if std::mem::take(&mut self.redraw_pending) {
                self.redraw();
            }
        }
        let replay = self.replay.as_ref().and_then(crate::replay::Replay::wake);
        let wake = [self.app.deadline(), replay].into_iter().flatten().min();
        event_loop.set_control_flow(match wake {
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let _zone = aq_trace::zone(match &event {
            WindowEvent::RedrawRequested => "WM_PAINT",
            WindowEvent::MouseWheel { .. } => "колесо",
            WindowEvent::PointerMoved { .. } => "указатель",
            _ => "событие окна",
        });
        if self.args.trace {
            eprintln!("{:>8.1} мс  {event:?}", self.started.elapsed().as_secs_f64() * 1000.0);
        }
        if let (Some(accessibility), Some(window)) = (self.accessibility.as_mut(), self.window.as_deref()) {
            accessibility.window_event(window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                self.close(event_loop);
                return;
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Occluded(false) => self.window().request_redraw(),
            WindowEvent::SurfaceResized(size) => {
                let maximized = self.window().is_maximized();
                let logical = self.logical_size();
                if let Some(root) = &mut self.root {
                    self.app.set_maximized(root, maximized);
                }
                if let Some(root) = &mut self.root {
                    root.handle_window_event(MasonryWindowEvent::Resize(size));
                    self.app.window_resized(root, logical);
                }
                if self.args.size.is_none()
                    && let Some(window) = &self.window
                {
                    self.window_state.observe(&**window);
                }
                self.redraw();
                self.redraw_pending = false;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let scale = scale_factor * self.app.scale();
                if let Some(root) = &mut self.root {
                    ui::rescale(root, scale);
                }
            }
            WindowEvent::Focused(focused) => self.text_event(TextEvent::WindowFocusChange(focused)),
            WindowEvent::ModifiersChanged(m) => {
                let s = m.state();
                let mut modifiers = Modifiers::empty();
                modifiers.set(Modifiers::SHIFT, s.contains(ModifiersState::SHIFT));
                modifiers.set(Modifiers::CONTROL, s.contains(ModifiersState::CONTROL));
                modifiers.set(Modifiers::ALT, s.contains(ModifiersState::ALT));
                modifiers.set(Modifiers::META, s.contains(ModifiersState::META));
                self.modifiers = modifiers;
            }
            WindowEvent::KeyboardInput { event, is_synthetic: false, .. } => self.keyboard(event),
            WindowEvent::Ime(ime) => {
                let ime = match ime {
                    Ime::Enabled => MasonryIme::Enabled,
                    Ime::Disabled => MasonryIme::Disabled,
                    Ime::Preedit(text, cursor) => MasonryIme::Preedit(text, cursor),
                    Ime::Commit(text) => MasonryIme::Commit(text),
                    _ => return,
                };
                self.text_event(TextEvent::Ime(ime));
            }
            WindowEvent::PointerEntered { .. } => self.pointer_event(PointerEvent::Enter(PRIMARY_MOUSE)),
            WindowEvent::PointerLeft { .. } => self.pointer_event(PointerEvent::Leave(PRIMARY_MOUSE)),
            WindowEvent::PointerMoved { position, .. } => {
                self.update_resize_edge(position);
                self.pointer.position = position;
                let current = self.pointer_state();
                self.pointer_event(PointerEvent::Move(PointerUpdate {
                    pointer: PRIMARY_MOUSE,
                    current,
                    coalesced: vec![],
                    predicted: vec![],
                }));
            }
            WindowEvent::PointerButton { state, position, button, .. } => {
                if state == ElementState::Pressed
                    && button.clone().mouse_button() == Some(MouseButton::Left)
                    && let Some(edge) = self.resize_edge
                {
                    let _ = self.window().drag_resize_window(edge);
                    return;
                }
                self.pointer.position = position;
                let button = button.mouse_button().and_then(|b| match b {
                    MouseButton::Left => Some(PointerButton::Primary),
                    MouseButton::Right => Some(PointerButton::Secondary),
                    MouseButton::Middle => Some(PointerButton::Auxiliary),
                    MouseButton::Back => Some(PointerButton::X1),
                    MouseButton::Forward => Some(PointerButton::X2),
                    _ => None,
                });
                let pressed = state == ElementState::Pressed;
                if let Some(b) = button {
                    if pressed {
                        self.pointer.buttons.insert(b);
                    } else {
                        self.pointer.buttons.remove(b);
                    }
                }
                // Двойной щелчок (count) проба не считает: всегда 1.
                self.pointer.count = 1;
                let state = self.pointer_state();
                let event = PointerButtonEvent { pointer: PRIMARY_MOUSE, button, state };
                self.pointer_event(if pressed { PointerEvent::Down(event) } else { PointerEvent::Up(event) });
                let delta = match button {
                    Some(PointerButton::X1) => -1,
                    Some(PointerButton::X2) => 1,
                    _ => 0,
                };
                if !pressed
                    && delta != 0
                    && let Some(root) = &mut self.root
                    && self.app.navigate(root, delta).title
                {
                    self.show_title();
                }
            }
            WindowEvent::PinchGesture { delta, .. } => self.pinch(delta),
            WindowEvent::MouseWheel { delta, .. } if (self.modifiers.ctrl() || self.modifiers.meta()) && !self.app.graph_shown() => {
                let up = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y > 0.0,
                    MouseScrollDelta::PixelDelta(p) => p.y > 0.0,
                    _ => return,
                };
                if self.app.zoom_wheel(up) {
                    self.rescale();
                    if let Some(root) = &mut self.root {
                        self.app.scale_changed(root);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        aq_trace::counter("колесо: строки", f64::from(y));
                        ScrollDelta::LineDelta(x, y)
                    }
                    MouseScrollDelta::PixelDelta(p) => {
                        aq_trace::counter("колесо: пиксели", p.y);
                        ScrollDelta::PixelDelta(p)
                    }
                    _ => return,
                };
                let state = self.pointer_state();
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: PRIMARY_MOUSE, delta, state }));
            }
            _ => {}
        }
        self.handle_signals(event_loop);
    }
}
