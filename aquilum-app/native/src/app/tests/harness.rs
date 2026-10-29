use super::*;

impl Harness {
    pub(super) fn new() -> Self {
        Self::with(None, 1.0)
    }

    pub(super) fn with(real_vault: Option<&std::path::Path>, scale: f64) -> Self {
        let theme = theme::test_lock();
        let vault = crate::test_dir::tempdir();
        let data = crate::test_dir::tempdir();
        std::fs::write(vault.path().join("a.md"), "текст").unwrap();
        let mut config = AppConfig::default();
        config.ui.theme = if std::env::var_os("AQUILUM_BENCH_LIGHT").is_some() { "light".into() } else { "dark".into() };
        config.ui.language = "ru".into();
        std::fs::write(data.path().join("settings.json"), serde_json::to_string(&config).unwrap()).unwrap();
        Self::assemble(vault, data, theme, real_vault, scale)
    }

    pub(super) fn restarted(self, scale: f64) -> Self {
        let Harness { app, root, vault, _data, _theme, .. } = self;
        drop((app, root));
        Self::assemble(vault, _data, _theme, None, scale)
    }

    fn assemble(vault: tempfile::TempDir, data: tempfile::TempDir, theme: std::sync::MutexGuard<'static, ()>, real_vault: Option<&std::path::Path>, scale: f64) -> Self {
        let workspace = Workspace::open(Some(data.path()), Some(real_vault.unwrap_or(vault.path())), || {}).unwrap();
        let mut app = App::new(workspace, String::new());
        let signals: Rc<RefCell<Vec<RenderRootSignal>>> = Rc::default();
        let sink = signals.clone();
        let widget = app.root_widget(false);
        let options = RenderRootOptions {
            default_properties: Arc::new(theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: bench_size().map_or(PhysicalSize::new((f64::from(W) * scale) as u32, (f64::from(H) * scale) as u32), |(w, h)| PhysicalSize::new(w, h)),
            scale_factor: scale,
            test_font: None,
        };
        let mut root = RenderRoot::new(widget, move |s| sink.borrow_mut().push(s), options);
        theme::register_fonts(&mut root);
        app.attach_window_controls(&mut root, Size::new(f64::from(W), f64::from(H)));
        app.window_resized(&mut root, Size::new(f64::from(W), f64::from(H)));
        let _ = root.redraw();
        let mut harness = Harness { app, root, signals, vault, _data: data, _theme: theme };
        harness.settle();
        harness
    }

    pub(super) fn settle(&mut self) {
        self.parked();
        self.anim(std::time::Duration::from_secs(1));
        let _ = self.root.redraw();
    }

    pub(super) fn parked(&mut self) {
        while self.app.opening.is_some() {
            crate::disk::wait();
            self.app.on_opened(&mut self.root);
        }
    }

    pub(super) fn open(&mut self, path: std::path::PathBuf) -> bool {
        let opened = self.app.open(&mut self.root, path);
        self.parked();
        opened
    }

    pub(super) fn anim(&mut self, elapsed: std::time::Duration) {
        self.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(elapsed));
    }

    pub(super) fn send(&mut self, id: masonry::core::WidgetId, action: impl std::any::Any + std::fmt::Debug + Send) -> Outcome {
        let outcome = self.app.on_action(&mut self.root, id, masonry::core::ErasedAction::from(Box::new(action)), Size::new(1000.0, 600.0));
        self.parked();
        outcome
    }

    pub(super) fn act(&mut self, id: masonry::core::WidgetId, action: impl std::any::Any + std::fmt::Debug + Send) -> Outcome {
        let outcome = self.send(id, action);
        let _ = self.root.redraw();
        outcome
    }

    pub(super) fn deliver(&mut self) -> Outcome {
        let mut outcome = Outcome::default();
        loop {
            self.parked();
            let _ = self.root.redraw();
            let actions: Vec<_> = std::mem::take(&mut *self.signals.borrow_mut())
                .into_iter()
                .filter_map(|s| match s {
                    RenderRootSignal::Action(action, id) => Some((action, id)),
                    _ => None,
                })
                .collect();
            if actions.is_empty() {
                return outcome;
            }
            for (action, id) in actions {
                let next = self.app.on_action(&mut self.root, id, action, Size::new(f64::from(W), f64::from(H)));
                outcome.title |= next.title;
                outcome.rescale |= next.rescale;
            }
        }
    }

    pub(super) fn click(&mut self, x: f64, y: f64) -> Outcome {
        self.click_with(x, y, Modifiers::empty())
    }

    pub(super) fn click_with(&mut self, x: f64, y: f64, modifiers: Modifiers) -> Outcome {
        self.settle();
        let mut state = PointerState { modifiers, ..pointer_at(x, y, 1.0) };
        self.root.handle_pointer_event(moved(state.clone()));
        state.buttons.insert(PointerButton::Primary);
        self.app.pointer_event(&mut self.root, PointerEvent::Down(primary(state.clone())));
        state.buttons.remove(PointerButton::Primary);
        self.root.handle_pointer_event(PointerEvent::Up(primary(state)));
        self.deliver()
    }

    pub(super) fn key(&mut self, key: Key, modifiers: Modifiers) -> Outcome {
        crate::test_dir::key(&mut self.root, key, modifiers);
        self.deliver()
    }

    pub(super) fn escape(&mut self) -> Outcome {
        self.key(Key::Named(NamedKey::Escape), Modifiers::empty())
    }

    pub(super) fn config(&self) -> AppConfig {
        self.app.workspace.core.settings.get_config()
    }

    pub(super) fn section(&self) -> Option<Section> {
        match &self.app.popup {
            Some(Popup::Settings { dialog, .. }) => Some(dialog.section),
            _ => None,
        }
    }

    pub(super) fn choose(&mut self, index: usize) -> Outcome {
        let Some(Popup::Settings { menu: Some(menu), .. }) = &self.app.popup else { panic!("список выбора не открыт") };
        let layer = menu.layer;
        self.act(layer, crate::ui::menu::MenuChoice::Item(index))
    }

    pub(super) fn menu_open(&self) -> bool {
        matches!(&self.app.popup, Some(Popup::Settings { menu: Some(_), .. }))
    }
}
