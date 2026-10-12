use super::*;

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
    let mut event_loop = EventLoop::new().map_err(|e| format!("цикл событий: {e}"))?;
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
    event_loop.run_app_on_demand(host).map_err(|e| format!("цикл событий завершился с ошибкой: {e}"))
}
