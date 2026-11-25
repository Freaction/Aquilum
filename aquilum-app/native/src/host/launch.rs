use super::*;

pub fn run(args: Args, mut event_loop: EventLoop) -> Result<(), String> {
    #[cfg(not(feature = "installed"))]
    if args.trace_frames.is_some() {
        aq_trace::start();
    }
    #[cfg(not(feature = "installed"))]
    let replay = args.replay.clone().map(|(kind, out)| crate::replay::Replay::new(kind, out));
    let data_dir = args.data_dir.clone().or_else(|| aquilum_core::paths::data_dir(crate::workspace::APP_ID)).ok_or("не найден каталог данных пользователя")?;
    let proxy = event_loop.create_proxy();
    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || proxy.wake_up());
    let shown = Arc::clone(&wake);
    let crate::instance::Instance::First(_lock) = crate::instance::acquire(&data_dir, move || shown()) else { return Ok(()) };
    crate::autostart::repoint();
    #[cfg(target_os = "android")]
    crate::android::ime::set_wake(Arc::clone(&wake));
    let workspace = Workspace::open(&data_dir, args.vault.as_deref(), Arc::clone(&wake))?;
    install_crash_log(workspace.data_dir.join("crash.log"));
    let window_state = WindowState::load(&workspace.data_dir, args.size.is_none());
    let app = App::new(workspace, args.text.clone());
    let threads = args.threads;
    let metrics = Metrics::new(threads, args.metrics.clone());
    #[cfg(feature = "memory-probe")]
    let probe = args.memory_probe.clone().map(|out| crate::memory_probe::Probe::new(out, args.probe_notes.clone()));
    let host = Host {
        args,
        app,
        vsync: crate::vsync::Vsync::new(Arc::clone(&wake)),
        system_theme: theme::SystemTheme::new(Arc::clone(&wake)),
        wake,
        started: Instant::now(),
        window: None,
        output: None,
        root: None,
        signals: Rc::default(),
        renderer: Renderer::new(threads),
        modifiers: Modifiers::empty(),
        pointer: PointerState::default(),
        moved: false,
        ime_enabled: false,
        ime_area: None,
        last_anim: None,
        last_vblank: None,
        anim_pending: false,
        redraw_pending: false,
        full_next: true,
        touchpad: None,
        metrics,
        accessibility: None,
        window_state,
        #[cfg(not(feature = "installed"))]
        replay,
        #[cfg(feature = "memory-probe")]
        probe,
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
        touch: touch::Touch::default(),
        ime_text: None,
    };
    event_loop.run_app_on_demand(host).map_err(|e| format!("цикл событий завершился с ошибкой: {e}"))
}
