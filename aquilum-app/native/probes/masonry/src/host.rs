//! Хост `RenderRoot`: окно winit 0.31, перевод событий в ui-events, сигналы Masonry, отрисовка
//! через `imaging_vello_cpu` в буфер softbuffer.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use imaging_vello_cpu::VelloCpuRenderer;
use masonry::app::{RenderRoot, RenderRootOptions, RenderRootSignal, VisualLayerKind, WindowSizePolicy};
use masonry::core::keyboard::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey};
use masonry::core::{
    Ime as MasonryIme, PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo,
    PointerScrollEvent, PointerState, PointerType, PointerUpdate, ScrollDelta, TextEvent,
    WindowEvent as MasonryWindowEvent,
};
use masonry::imaging::render::{ImageBufferFormat, ImageBufferTarget, ImageRenderer};
use masonry::peniko::color::AlphaColor;
use masonry::theme::default_property_set;
use masonry_imaging::{Layer, PreparedFrame};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{self as wk, ModifiersState, PhysicalKey};
use winit::window::{
    ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData, Window, WindowAttributes, WindowId,
};

use crate::Args;

/// Шаг анимационных тиков (~60 Гц). Тик дешёвый: рисуется только то, что изменилось.
const ANIM_INTERVAL: Duration = Duration::from_millis(16);

const PRIMARY_MOUSE: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Mouse,
};

struct Host {
    args: Args,
    started: Instant,
    surface: Option<Surface<OwnedDisplayHandle, Box<dyn Window>>>,
    root: Option<RenderRoot>,
    signals: Rc<RefCell<Vec<RenderRootSignal>>>,
    renderer: VelloCpuRenderer,
    rgba: Vec<u8>,
    modifiers: Modifiers,
    pointer: PointerState,
    ime_enabled: bool,
    /// Последняя отправленная в IME область каретки: Masonry шлёт ImeMoved после каждого прохода.
    ime_area: Option<(LogicalPosition<f64>, LogicalSize<f64>)>,
    clipboard: Option<arboard::Clipboard>,
    /// Время прошлого анимационного тика, пока анимация продолжается.
    last_anim: Option<Instant>,
    /// Когда сделать следующий анимационный тик. Тики идут по таймеру и сами не рисуют:
    /// кадр растеризуется, только если после тика Masonry попросил перерисовку.
    next_anim: Option<Instant>,
    frames: u64,
    anim_ticks: u64,
    first_frame_ms: f64,
    frame_ms_total: f64,
}

pub fn run(args: Args) {
    let host = Host {
        args,
        started: Instant::now(),
        surface: None,
        root: None,
        signals: Rc::default(),
        renderer: VelloCpuRenderer::new(1, 1),
        rgba: Vec::new(),
        modifiers: Modifiers::empty(),
        pointer: PointerState::default(),
        ime_enabled: false,
        ime_area: None,
        clipboard: arboard::Clipboard::new().ok(),
        last_anim: None,
        next_anim: None,
        frames: 0,
        anim_ticks: 0,
        first_frame_ms: 0.0,
        frame_ms_total: 0.0,
    };
    let event_loop = EventLoop::new().expect("цикл событий");
    event_loop.run_app(host).expect("цикл событий завершился с ошибкой");
}

impl Host {
    fn window(&self) -> &dyn Window {
        &**self.surface.as_ref().expect("окно создано").window()
    }

    fn time_ns(&self) -> u64 {
        self.started.elapsed().as_nanos() as u64
    }

    fn pointer_state(&mut self) -> PointerState {
        self.pointer.time = self.time_ns();
        self.pointer.modifiers = self.modifiers;
        self.pointer.scale_factor = self.window().scale_factor();
        self.pointer.clone()
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
        for signal in signals {
            let window = self.surface.as_ref().expect("окно создано").window();
            match signal {
                RenderRootSignal::RequestRedraw => window.request_redraw(),
                RenderRootSignal::RequestAnimFrame => {
                    if self.next_anim.is_none() {
                        self.next_anim = Some(Instant::now() + ANIM_INTERVAL);
                    }
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
                RenderRootSignal::SetCursor(cursor) => window.set_cursor(cursor.into()),
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
                // Действия виджетов, перетаскивание окна, меню окна и т. п. пробе не нужны.
                _ => {}
            }
        }
    }

    fn redraw(&mut self) {
        let t0 = Instant::now();
        let (Some(surface), Some(root)) = (self.surface.as_mut(), self.root.as_mut()) else { return };
        let size = surface.window().surface_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        surface.resize(w, h).expect("softbuffer: размер");

        let (layers, _access_update) = root.redraw();
        let overlays: Vec<Layer<'_>> = layers
            .overlay_layers()
            .filter_map(|layer| match &layer.kind {
                VisualLayerKind::Scene(scene) => Some(Layer { scene, transform: layer.transform }),
                _ => None,
            })
            .collect();
        let Some(VisualLayerKind::Scene(base)) = layers.root_layer().map(|l| &l.kind) else { return };
        let mut frame = PreparedFrame::new(
            size.width,
            size.height,
            surface.window().scale_factor(),
            AlphaColor::from_rgb8(0x1f, 0x1f, 0x22),
            base,
            &overlays,
        );
        self.rgba.resize(size.width as usize * size.height as usize * 4, 0);
        let target = ImageBufferTarget {
            data: &mut self.rgba,
            width: size.width,
            height: size.height,
            bytes_per_row: size.width as usize * 4,
            format: ImageBufferFormat::Rgba8Unorm,
        };
        self.renderer.render_source_into(&mut frame, target).expect("vello_cpu: отрисовка");

        surface.window().pre_present_notify();
        let mut buffer = surface.buffer_mut().expect("softbuffer: буфер");
        for (dst, px) in buffer.iter_mut().zip(self.rgba.chunks_exact(4)) {
            *dst = (px[0] as u32) << 16 | (px[1] as u32) << 8 | px[2] as u32;
        }
        buffer.present().expect("softbuffer: present");

        self.frames += 1;
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if self.frames == 1 {
            self.first_frame_ms = self.started.elapsed().as_secs_f64() * 1000.0;
            self.write_metrics();
        } else {
            self.frame_ms_total += ms;
        }
    }

    /// Анимационный тик: проходы обновления Masonry без растеризации.
    fn anim_tick(&mut self, event_loop: &dyn ActiveEventLoop) {
        let Some(root) = &mut self.root else { return };
        let now = Instant::now();
        let elapsed = self.last_anim.take().map(|t| now - t).unwrap_or(Duration::ZERO);
        root.handle_window_event(MasonryWindowEvent::AnimFrame(elapsed));
        self.last_anim = root.needs_anim().then_some(now);
        self.anim_ticks += 1;
        self.handle_signals(event_loop);
    }

    fn write_metrics(&self) {
        let scale = self.surface.as_ref().map(|s| s.window().scale_factor()).unwrap_or(0.0);
        let avg = if self.frames > 1 { self.frame_ms_total / (self.frames - 1) as f64 } else { 0.0 };
        let json = format!(
            "{{\"scaleFactor\":{scale},\"firstFrameMs\":{:.1},\"frames\":{},\"animTicks\":{},\"frameAvgMs\":{avg:.2}}}",
            self.first_frame_ms, self.frames, self.anim_ticks
        );
        match &self.args.metrics {
            Some(path) => {
                let _ = std::fs::write(path, json);
            }
            None => eprintln!("{json}"),
        }
    }
}

impl ApplicationHandler for Host {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attributes = WindowAttributes::default()
            .with_title("Aquilum (Masonry probe)")
            .with_surface_size(self.args.size);
        let window = event_loop.create_window(attributes).expect("окно");
        let options = RenderRootOptions {
            default_properties: Arc::new(default_property_set()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: window.surface_size(),
            scale_factor: window.scale_factor(),
            test_font: None,
        };
        let signals = self.signals.clone();
        let root = RenderRoot::new(
            crate::widget_tree(&self.args.text).erased(),
            move |signal| signals.borrow_mut().push(signal),
            options,
        );
        let context = Context::new(event_loop.owned_display_handle()).expect("softbuffer: контекст");
        self.surface = Some(Surface::new(&context, window).expect("softbuffer: поверхность"));
        let mut root = root;
        if self.args.focus {
            let editor = root.get_widget_with_tag(crate::EDITOR_TAG).map(|w| w.id());
            root.focus_on(editor);
        }
        self.root = Some(root);
        self.handle_signals(event_loop);
        self.window().request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.next_anim.is_some_and(|t| Instant::now() >= t) {
            self.next_anim = None;
            self.anim_tick(event_loop);
        }
        event_loop.set_control_flow(match self.next_anim {
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.args.trace {
            eprintln!("{:>8.1} мс  {event:?}", self.started.elapsed().as_secs_f64() * 1000.0);
        }
        match event {
            WindowEvent::CloseRequested => {
                self.write_metrics();
                event_loop.exit();
                return;
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::SurfaceResized(size) => {
                if let Some(root) = &mut self.root {
                    root.handle_window_event(MasonryWindowEvent::Resize(size));
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(root) = &mut self.root {
                    root.handle_window_event(MasonryWindowEvent::Rescale(scale_factor));
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
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(p) => ScrollDelta::PixelDelta(p),
                    _ => return,
                };
                let state = self.pointer_state();
                self.pointer_event(PointerEvent::Scroll(PointerScrollEvent {
                    pointer: PRIMARY_MOUSE,
                    delta,
                    state,
                }));
            }
            _ => {}
        }
        self.handle_signals(event_loop);
    }
}
