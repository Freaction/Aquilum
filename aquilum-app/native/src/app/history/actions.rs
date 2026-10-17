use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;

use super::{VersionCommand, App};
use crate::app::{Outcome, Popup};
use crate::history;
use crate::i18n::t;
use crate::ui::editor::TextAction;
use crate::ui::menu::{Menu, MenuChoice};
use crate::ui::widgets::{ItemAction, Pressed};

impl App {
    pub(in crate::app) fn on_history_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        if let Some(view) = &self.version_view {
            let pressed = action.downcast_ref::<Pressed>().is_some();
            let command = if !pressed {
                None
            } else if Some(id) == view.revert {
                Some(VersionCommand::Revert)
            } else if id == view.restore {
                Some(VersionCommand::Restore)
            } else if id == view.back {
                self.close_version(root);
                return Some(Outcome::default());
            } else {
                None
            };
            if let Some(command) = command {
                let version = self.opened_versions.get(&self.tabs.active_id())?.clone();
                self.run_version_command(root, &version, command);
                return Some(Outcome::default());
            }
        }
        let body = self.history_body.as_ref()?;
        if let Some(field) = body.rename
            && field.id() == id
        {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Entered(name)) => self.finish_version_rename(root, Some(name.clone())),
                Some(TextAction::Cancelled) => self.finish_version_rename(root, None),
                _ => {}
            }
            return Some(Outcome::default());
        }
        if action.downcast_ref::<Pressed>().is_some() && body.more == Some(id) {
            if let Some(state) = &mut self.history {
                state.limit += history::PAGE_SIZE;
            }
            self.load_history(root);
            return Some(Outcome::default());
        }
        let item = action.downcast_ref::<ItemAction>()?;
        if id == body.current {
            if matches!(item, ItemAction::Open { .. }) {
                self.close_version(root);
            }
            return Some(Outcome::default());
        }
        let index = body.rows.iter().find(|(row, _)| *row == id)?.1;
        let version = self.history.as_ref()?.versions.get(index)?.clone();
        match item {
            ItemAction::Open { .. } => self.open_version(root, version),
            ItemAction::Menu(at) => self.open_version_menu(root, index, *at),
            ItemAction::Hover(_) => {}
        }
        Some(Outcome::default())
    }

    fn open_version_menu(&mut self, root: &mut RenderRoot, index: usize, at: Point) {
        let Some(state) = &self.history else { return };
        let Some(version) = state.versions.get(index) else { return };
        let oldest = state.versions.len() == state.total && index + 1 == state.versions.len();
        let mut commands = Vec::new();
        if !version.is_current {
            commands.push((VersionCommand::Restore, t("history.restore")));
        }
        if !oldest {
            commands.push((VersionCommand::Revert, t("history.revert")));
        }
        commands.push((VersionCommand::Rename, t("common.rename")));
        let labels: Vec<&str> = commands.iter().map(|(_, label)| label.as_str()).collect();
        let menu = Menu::new(&t("history.versionActions"), &labels);
        let origin = Menu::context_origin(at, labels.len(), self.window);
        self.show_layer(root, menu, origin, |layer| Popup::VersionMenu { layer, index, commands: commands.into_iter().map(|(c, _)| c).collect() });
    }

    pub(in crate::app) fn on_version_menu(&mut self, root: &mut RenderRoot, layer: WidgetId, index: usize, commands: Vec<VersionCommand>, action: &ErasedAction) {
        let Some(choice) = action.downcast_ref::<MenuChoice>() else {
            self.popup = Some(Popup::VersionMenu { layer, index, commands });
            return;
        };
        let MenuChoice::Item(item) = choice else { return };
        root.remove_layer(layer);
        let Some(&command) = commands.get(*item) else { return };
        if command == VersionCommand::Rename {
            if let Some(state) = &mut self.history {
                state.renaming = Some(index);
            }
            self.render_history(root);
            return;
        }
        let Some(version) = self.history.as_ref().and_then(|s| s.versions.get(index)).cloned() else { return };
        self.run_version_command(root, &version, command);
    }

    fn finish_version_rename(&mut self, root: &mut RenderRoot, name: Option<String>) {
        let Some(state) = &mut self.history else { return };
        let Some(index) = state.renaming.take() else { return };
        if let (Some(name), Some(version)) = (name, state.versions.get(index)) {
            let label = history::label(version);
            if name.trim() != label && Some(name.trim()) != version.name.as_deref() {
                aquilum_core::files::gate::name_version(&self.workspace.core, &state.path, Some(&version.id), name.trim());
            }
        }
        self.load_history(root);
    }
}
