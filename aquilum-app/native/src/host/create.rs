use super::*;

impl Host {
    pub(super) fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attributes = WindowAttributes::default()
            .with_title("Aquilum")
            .with_decorations(!ui::titlebar::OWN_CONTROLS)
            .with_visible(false)
            .with_window_icon(window_icon());
        #[cfg(target_os = "windows")]
        let attributes = attributes.with_platform_attributes(Box::new(
            winit::platform::windows::WindowAttributesWindows::default().with_undecorated_shadow(true),
        ));
        #[cfg(target_os = "linux")]
        let attributes = linux::attributes(attributes, event_loop);
        let attributes = match self.args.size {
            Some(size) => attributes.with_surface_size(size),
            None => attributes.with_surface_size(self.window_state.size()),
        };
        let attributes = match self.window_state.position().filter(|_| self.args.keep_position) {
            Some(position) => attributes.with_position(position),
            None => attributes,
        };
        let window = event_loop.create_window(attributes).expect("окно");
        self.accessibility = Accessibility::attach(&*window, Arc::clone(&self.wake));
        cloak(&*window, true);
        self.restore_maximized(&*window);
        self.metrics.window_created(self.started, window.scale_factor());
        self.system_theme.window(window.theme());
        let widget = self.app.root_widget(self.system_theme.take().unwrap_or(false));
        let options = RenderRootOptions {
            default_properties: Arc::new(ui::theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: window.surface_size(),
            scale_factor: window.scale_factor() * self.app.scale(),
            test_font: None,
        };
        let signals = self.signals.clone();
        let mut root = RenderRoot::new(widget, move |signal| signals.borrow_mut().push(signal), options);
        ui::theme::register_fonts(&mut root);
        #[cfg(not(feature = "installed"))]
        if self.args.paint_flashing {
            self.renderer.toggle_overlay();
        }
        if !self.args.restore {
            self.app.forget_session();
        }
        let logical = window.surface_size().to_logical::<f64>(window.scale_factor() * self.app.scale());
        self.app.attach_window_controls(&mut root, masonry::kurbo::Size::new(logical.width, logical.height));
        self.app.restore(&mut root);
        let window: std::rc::Rc<dyn Window> = std::rc::Rc::from(window);
        self.attach_output(event_loop, window.clone());
        self.touchpad = crate::touchpad::Touchpad::attach(&*window);
        self.window = Some(window);
        if let Some(note) = self.args.open.clone() {
            self.app.open(&mut root, note);
        }
        self.root = Some(root);
        self.fit_window();
        self.on_wake(event_loop);
        self.redraw();
        if self.output.is_none() {
            self.reveal();
        }
    }

    pub(super) fn attach_output(&mut self, event_loop: &dyn ActiveEventLoop, window: std::rc::Rc<dyn Window>) {
        match crate::output::Output::new(event_loop.owned_display_handle(), window) {
            Ok(output) => self.output = Some(output),
            Err(err) => report(&format!("вывод кадра: {err}")),
        }
        self.full_next = true;
    }
}
