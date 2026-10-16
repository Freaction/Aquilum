use super::*;

impl App {
    pub fn root_widget(&mut self, system_dark: bool) -> NewWidget<dyn Widget> {
        self.system_dark = system_dark;
        ui::widgets::Appear::new(Slot::new(self.build_ui()).with_tag(ROOT_TAG), ui::widgets::NORMAL).erased()
    }

    pub(in crate::app) fn visible_panels(&self) -> (bool, bool) {
        (self.interface.sidebar_open() && !self.interface.focus_mode(), self.panel_visible())
    }

    pub(in crate::app) fn replace_root(&mut self, root: &mut RenderRoot) {
        let widget = self.build_ui();
        root.set_default_properties(Arc::new(theme::default_properties()));
        root.edit_widget_with_tag(ROOT_TAG, |mut slot| Slot::set_child(&mut slot, widget));
        self.render_panel(root);
    }

    pub(in crate::app) fn restyle(&mut self, root: &mut RenderRoot, fonts: bool) {
        self.apply_appearance();
        root.restyle(Arc::new(theme::default_properties()), fonts);
    }

    fn apply_appearance(&self) {
        let config = self.workspace.core.settings.get_config();
        theme::apply_setting(&config.ui.theme, self.system_dark, &config.ui.primary_color);
        crate::ui::motion::set(config.ui.animations);
        i18n::set_language(&config.ui.language);
        theme::set_ui_font(UiFont {
            family: config.ui.font.font_family.clone(),
            size: config.ui.font.font_size_base as f32,
            weight: config.ui.font.font_weight as f32,
        });
        theme::set_editor_font(theme::EditorFont {
            family: config.editor.font.font_family.clone(),
            size: config.editor.font.font_size_base as f32,
            weight: [400.0, 450.0, 500.0, 550.0, 600.0]
                .into_iter()
                .min_by(|a: &f32, b: &f32| (a - config.editor.font.font_weight as f32).abs().total_cmp(&(b - config.editor.font.font_weight as f32).abs()))
                .unwrap_or(400.0),
            line_height: config.editor.line_height,
            max_width_ch: config.editor.max_width_ch,
        });
    }

    fn build_ui(&mut self) -> NewWidget<dyn Widget> {
        self.apply_appearance();
        let workspace_name = self
            .tree
            .root()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tabs = self.tab_widgets();
        let note = self.note_widget();
        let graph = self.graph_widget();
        let graph_active = self.tabs.active().graph && self.tree.root().is_some();
        let target = self.visible_panels();
        let from = self.shown.replace(target).unwrap_or(target);
        let parts = ui::RootParts {
            workspace_name: &workspace_name,
            note,
            sidebar_open: target.0,
            sidebar_from: from.0,
            sidebar_width: self.panel_width(crate::interface::Side::Sidebar),
            panel_width: self.panel_width(crate::interface::Side::Panel),
            focus_mode: self.interface.focus_mode(),
            home: self.home.is_some(),
            tabs,
            active_tab: self.tabs.active_index(),
            empty_tab: self.tabs.active().path.is_none() && self.note.is_none() && self.tree.root().is_some(),
            vault_open: self.tree.root().is_some(),
            vault_error: self.vault_error.as_deref(),
            panel_open: target.1,
            panel_from: from.1,
            panel_mode: self.panel_mode(),
            has_analysis: !self.panel_methods().is_empty(),
            graph,
            graph_active,
        };
        let (widget, chrome) = ui::root(self.tree.widget(), parts);
        self.chrome = Some(chrome);
        widget.erased()
    }
}
