use std::path::Path;

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget, WidgetId};

use super::super::App;
use crate::file_ops;
use crate::i18n::t;
use crate::ui::titlebar::{TabButton, TabRename, TabStrip};

impl App {
    pub(in crate::app) fn tab_widgets(&mut self) -> Vec<NewWidget<dyn Widget>> {
        let active = self.tabs.active_id();
        self.rename_field = None;
        let mut widgets = Vec::new();
        for tab in self.tabs.tabs() {
            let title = if tab.graph { t("tabs.graph") } else { tab.title().unwrap_or_else(|| t("tabs.newTab")) };
            if self.renaming_tab == Some(tab.id) {
                let (widget, field) = TabRename::new(&title);
                self.rename_field = Some(field);
                widgets.push(widget.erased());
            } else {
                let icon = tab.graph.then_some(crate::ui::icons::NETWORK);
                widgets.push(TabButton::new(tab.id, &title, tab.id == active, tab.path.is_some(), icon).erased());
            }
        }
        widgets
    }

    pub(in crate::app) fn refresh_tabs(&mut self, root: &mut RenderRoot) {
        if let Some(session) = &mut self.session {
            session.save(&self.workspace.core, &mut self.tabs);
        }
        let widgets = self.tab_widgets();
        if let Some(chrome) = &self.chrome {
            let active = self.tabs.active_index();
            chrome.tab_strip.edit(root, |mut strip| TabStrip::set_tabs(&mut strip, widgets, active));
        }
        if let Some(field) = self.rename_field {
            field.focus_all(root);
        }
    }

    pub(in crate::app) fn start_tab_rename(&mut self, root: &mut RenderRoot, path: &Path) {
        let Some(tab) = self.tabs.tabs().iter().find(|t| t.path.as_deref() == Some(path)) else { return };
        self.renaming_tab = Some(tab.id);
        self.refresh_tabs(root);
    }

    pub(in crate::app) fn finish_tab_rename(&mut self, root: &mut RenderRoot, input: Option<&str>) {
        let Some(id) = self.renaming_tab.take() else { return };
        let field = self.rename_field.take();
        let input = input.map(str::to_owned).or_else(|| field.and_then(|f| f.get(root)).map(|f| f.inner().text()));
        let path = self.tabs.tabs().iter().find(|t| t.id == id).and_then(|t| t.path.clone());
        if let (Some(input), Some(from)) = (input, path)
            && let Some((to, _)) = file_ops::rename_target(&from, &input)
        {
            self.rename_file(root, from, to);
        }
        self.refresh_tabs(root);
    }

    pub(in crate::app) fn cancel_tab_rename(&mut self, root: &mut RenderRoot) {
        self.rename_field = None;
        if self.renaming_tab.take().is_some() {
            self.refresh_tabs(root);
        }
    }

    pub(in crate::app) fn is_tab_rename_field(&self, id: WidgetId) -> bool {
        self.rename_field.is_some_and(|field| field.id() == id)
    }
}
