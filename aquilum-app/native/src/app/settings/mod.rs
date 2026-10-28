use std::collections::HashMap;
use std::path::Path;

use aquilum_core::files::gate;
use crate::ui::editor::TextEditor;
use aquilum_core::files::trash::{cleanup_trash_impl, empty_trash_impl, ensure_trash_impl};
use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers};
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use crate::ui::editor::TextAction;

use super::{App, Outcome, Popup};

mod action;
mod apply;
mod bindings;
mod cleanup;
mod refresh;

use crate::system;
use crate::i18n::t;
use crate::ui::dialog::ConfirmDialog;
use crate::ui::menu::Menu;
use crate::ui::settings::controls::{Changed, ColorPicker, Committed, DropdownButton, Switch};
use crate::ui::keymap::{self, Command, Scope};
use crate::ui::tokens::size;
use crate::ui::settings::schema::{self, Action, Context, Section, Setter, Value, Wipe};
use crate::ui::settings::{self, Binding, SettingsDialog};
use crate::ui::widgets::Pressed;

pub(super) struct OpenMenu {
    pub(super) layer: WidgetId,
    button: WidgetId,
}

pub(super) enum Effect {
    None,
    Smart(crate::ui::editor::Smart),
    Rescale,
    Panel,
    Content,
    Restyle(bool),
    Everything,
}

impl App {
    pub(super) fn open_settings(&mut self, root: &mut RenderRoot) {
        let section = self.interface.settings_section().and_then(Section::from_key).unwrap_or(Section::Ui);
        self.show_settings(root, section);
    }

    fn settings_context(&self, copied: Option<usize>, trash_page: usize) -> Context {
        let workspace = self.tree.root().map(Path::to_path_buf);
        let known = workspace.as_deref().and_then(|root| self.workspace.core.ui_state.workspace(root));
        let home_page = known.as_ref().map(|known| known.home_page.clone()).unwrap_or_default();
        let large = self.workspace.core.sync.large().into_iter();
        let sync_large = large.filter(|large| workspace.as_deref().is_some_and(|root| root.join(&large.path).exists())).collect();
        Context {
            workspace,
            scale: self.interface.scale(),
            home_page,
            mcp: Some(self.workspace.core.mcp.status()),
            copied,
            trash_page,
            autostart: crate::autostart::available().then(crate::autostart::enabled),
            update: crate::updater::available().then(|| self.updater.status()),
            sync: self.workspace.core.sync.status(),
            vault_sync: known.map(|known| known.sync),
            sync_large,
            sync_size: self.workspace.core.sync.size(),
            sync_due: self.workspace.core.sync.due(),
            sync_last: self.workspace.core.sync.last(),
        }
    }

    fn show_settings(&mut self, root: &mut RenderRoot, section: Section) {
        let config = self.workspace.core.settings.get_config();
        let ctx = self.settings_context(None, 0);
        let (layer, dialog) = settings::dialog(&config, &ctx, section);
        self.show_layer(root, layer, Point::ORIGIN, |_| Popup::Settings { dialog, menu: None });
        self.fit_settings(root);
    }

    pub(in crate::app) fn fit_settings(&mut self, root: &mut RenderRoot) {
        let screen = self.interface.screen();
        if let Some(Popup::Settings { dialog, .. }) = &mut self.popup {
            dialog.fit(root, screen);
        }
    }

    pub(super) fn rebuild(&mut self, root: &mut RenderRoot) {
        self.replace_root(root);
        self.reload_settings(root);
    }

    fn keep(&mut self, dialog: SettingsDialog, menu: Option<OpenMenu>, effect: Effect, root: &mut RenderRoot) -> Outcome {
        self.popup = Some(Popup::Settings { dialog, menu });
        self.finish(root, effect)
    }

    fn finish(&mut self, root: &mut RenderRoot, effect: Effect) -> Outcome {
        match effect {
            Effect::None => Outcome::default(),
            Effect::Smart(smart) => {
                if let Some(ids) = &self.note_ids {
                    ids.body.edit(root, |mut body| TextEditor::set_smart(&mut body, smart));
                }
                Outcome::default()
            }
            Effect::Rescale => Outcome { rescale: true, ..Outcome::default() },
            Effect::Panel => {
                self.refresh_analysis(root);
                Outcome::default()
            }
            Effect::Content => {
                self.refresh_settings(root);
                self.refresh_analysis(root);
                Outcome::default()
            }
            Effect::Restyle(fonts) => {
                self.restyle(root, fonts, None);
                Outcome::default()
            }
            Effect::Everything => {
                self.rebuild(root);
                Outcome::default()
            }
        }
    }

    pub fn settings_key(&mut self, root: &mut RenderRoot, key: &Key, modifiers: Modifiers) -> Option<Outcome> {
        let direction = match keymap::command(&[Scope::Number], key, modifiers)? {
            Command::Increase => 1.0,
            Command::Decrease => -1.0,
            _ => return None,
        };
        let field = root.focused_widget()?;
        let &Binding::Number { setter, .. } = self.popup_bindings()?.get(&field)? else { return None };
        let mut bindings = std::mem::take(self.popup_bindings()?);
        let effect = self.step_number(root, &mut bindings, field, direction);
        if let Some(Binding::Number { number, .. }) = bindings.get(&field) {
            let text = schema::format_number(number.value);
            self.typing = Some((setter, text.clone(), text.len()));
        }
        if let Some(slot) = self.popup_bindings() {
            *slot = bindings;
        }
        if let Some(Popup::Reader(state)) = &self.popup {
            let settings = crate::app::reader::reader_settings(&self.workspace.core.settings.get_config());
            root.edit_widget(state.layer, |mut w| crate::ui::reader::ReaderView::set_settings(&mut w.downcast(), settings));
        }
        Some(self.finish(root, effect))
    }

    fn popup_bindings(&mut self) -> Option<&mut HashMap<WidgetId, Binding>> {
        match &mut self.popup {
            Some(Popup::Settings { dialog, menu: None }) => Some(&mut dialog.bindings),
            Some(Popup::Reader(state)) => Some(&mut state.bindings),
            _ => None,
        }
    }
}
