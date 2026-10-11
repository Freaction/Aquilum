use std::collections::HashMap;
use std::path::Path;

use aquilum_core::files::gate;
use crate::ui::editor::TextEditor;
use aquilum_core::files::trash::{cleanup_trash_impl, ensure_trash_impl};
use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, NamedKey};
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use crate::ui::editor::TextAction;

use super::{App, Outcome, Popup};

mod apply;
mod bindings;

use bindings::{resume_switches, same_setter, switch_travel};
use crate::system;
use crate::ui::dialog::DialogAnswer;
use crate::ui::menu::{Menu, MenuChoice};
use crate::ui::dialog::Modal;
use crate::ui::settings::controls::{Changed, ColorPicker, Committed, DropdownButton, Picked, Switch};
use crate::ui::theme;
use crate::ui::tokens::size;
use crate::ui::settings::schema::{self, Action, Context, Section, Setter, Value};
use crate::ui::settings::{self, Binding, SettingsDialog};
use crate::ui::scroll::ScrollArea;
use crate::ui::widgets::{ItemButton, Pressed};

pub(super) struct OpenMenu {
    pub(super) layer: WidgetId,
    button: WidgetId,
}

pub(super) enum Effect {
    None,
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
        let home_page = workspace
            .as_ref()
            .and_then(|root| {
                let path = root.to_string_lossy();
                let known = self.workspace.core.ui_state.list_workspaces().ok()?;
                known.into_iter().find(|w| w.path == path).map(|w| w.home_page)
            })
            .unwrap_or_default();
        Context {
            workspace,
            scale: self.interface.scale(),
            home_page,
            mcp: Some(self.workspace.core.mcp.status()),
            copied,
            trash_page,
            autostart: crate::autostart::available().then(crate::autostart::enabled),
            update: crate::updater::available().then(|| self.updater.status()),
        }
    }

    fn show_settings(&mut self, root: &mut RenderRoot, section: Section) {
        let config = self.workspace.core.settings.get_config();
        let ctx = self.settings_context(None, 0);
        let (layer, dialog) = settings::dialog(&config, &ctx, section);
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(dialog.layer));
        self.popup = Some(Popup::Settings { dialog, menu: None });
    }

    pub(super) fn rebuild(&mut self, root: &mut RenderRoot) {
        self.replace_root(root);
        self.reload_settings(root);
    }

    fn reload_settings(&mut self, root: &mut RenderRoot) {
        let Some(Popup::Settings { dialog, menu }) = self.popup.take() else { return };
        if let Some(menu) = menu {
            root.remove_layer(menu.layer);
        }
        let offset = dialog.content.get(root).map(|area| area.inner().offset());
        let switches = switch_travel(root, &dialog.bindings);
        let config = self.workspace.core.settings.get_config();
        let ctx = self.settings_context(dialog.copied, dialog.trash_page);
        let (card, finish) = settings::card(&config, &ctx, dialog.section);
        root.edit_widget(dialog.layer, |mut w| Modal::set_card(&mut w.downcast(), card, &crate::i18n::t("settings.title")));
        let fresh = finish(dialog.layer);
        root.focus_on(Some(fresh.layer));
        if let Some(offset) = offset {
            fresh.content.edit(root, |mut area| ScrollArea::set_offset(&mut area, offset));
        }
        resume_switches(root, &fresh.bindings, &switches);
        self.popup = Some(Popup::Settings { dialog: fresh, menu: None });
        self.refocus(root);
    }

    fn refocus(&mut self, root: &mut RenderRoot) {
        let Some((setter, text, caret)) = self.typing.take() else { return };
        let Some(Popup::Settings { dialog, .. }) = &self.popup else { return };
        let field = dialog.bindings.iter().find_map(|(id, binding)| match binding {
            Binding::Number { setter: s, .. } | Binding::Text { setter: s, .. } if same_setter(*s, setter) => Some(*id),
            _ => None,
        });
        if let Some(field) = field {
            root.focus_on(Some(field));
            root.edit_widget(field, |mut w| {
                let mut editor = w.downcast::<TextEditor>();
                TextEditor::reset_text(&mut editor, &text);
                let caret = caret.min(text.len());
                TextEditor::select_byte_range(&mut editor, caret, caret);
            });
        }
    }

    pub(super) fn on_settings_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        self.typing = None;
        let Some(Popup::Settings { mut dialog, menu }) = self.popup.take() else { return Outcome::default() };
        let mut effect = Effect::None;

        if let Some(picked) = action.downcast_ref::<Picked>() {
            let Some(menu) = menu else {
                self.popup = Some(Popup::Settings { dialog, menu: None });
                return Outcome::default();
            };
            let hex = match picked {
                Picked::Color(hex) => {
                    self.preview_color(root, &mut dialog, menu.button, hex);
                    self.popup = Some(Popup::Settings { dialog, menu: Some(menu) });
                    return Outcome::default();
                }
                Picked::Closed(hex) => hex.clone(),
            };
            root.focus_on(Some(dialog.layer));
            if let Some(&Binding::Color(setter)) = dialog.bindings.get(&menu.button) {
                effect = self.apply(setter, Value::Text(hex));
            }
            return self.keep(dialog, None, effect, root);
        }

        if let Some(menu) = menu {
            if id == menu.button && matches!(action.downcast_ref::<Changed>(), Some(Changed::OpenColor(_))) {
                root.remove_layer(menu.layer);
                root.focus_on(Some(dialog.layer));
                if let Some(&Binding::Color(setter)) = dialog.bindings.get(&menu.button) {
                    let typed = dialog.bindings.values().find_map(|b| match b {
                        Binding::Text { setter: s, value, .. } if same_setter(*s, setter) => Some(value.clone()),
                        _ => None,
                    });
                    if let Some(hex) = typed {
                        effect = self.apply(setter, Value::Text(hex));
                    }
                }
                return self.keep(dialog, None, effect, root);
            }
            if id == menu.button && matches!(action.downcast_ref::<Changed>(), Some(Changed::OpenMenu(_))) {
                root.remove_layer(menu.layer);
                root.edit_widget(menu.button, |mut w| DropdownButton::set_open(&mut w.downcast(), false));
                root.focus_on(Some(dialog.layer));
                self.popup = Some(Popup::Settings { dialog, menu: None });
                return Outcome::default();
            }
            if action.downcast_ref::<MenuChoice>().is_some() {
                root.edit_widget(menu.button, |mut w| DropdownButton::set_open(&mut w.downcast(), false));
            }
            match action.downcast_ref::<MenuChoice>() {
                Some(MenuChoice::Item(index)) => {
                    root.remove_layer(menu.layer);
                    if let Some(Binding::Dropdown { setter, options, selected }) = dialog.bindings.get_mut(&menu.button) {
                        *selected = *index;
                        let label = options.get(*index).cloned().unwrap_or_default();
                        let setter = *setter;
                        root.edit_widget(menu.button, |mut w| DropdownButton::set_label(&mut w.downcast(), &label));
                        effect = self.apply(setter, Value::Index(*index));
                    }
                    root.focus_on(Some(dialog.layer));
                    self.popup = Some(Popup::Settings { dialog, menu: None });
                }
                Some(MenuChoice::Dismissed) => {
                    root.focus_on(Some(dialog.layer));
                    self.popup = Some(Popup::Settings { dialog, menu: None });
                }
                None => self.popup = Some(Popup::Settings { dialog, menu: Some(menu) }),
            }
            return self.finish(root, effect);
        }

        let closing = (id == dialog.layer && matches!(action.downcast_ref::<DialogAnswer>(), Some(DialogAnswer::Cancel)))
            || (id == dialog.close && action.downcast_ref::<Pressed>().is_some());
        if closing {
            crate::ui::dialog::close(root, dialog.layer);
            return Outcome::default();
        }

        if action.downcast_ref::<Pressed>().is_some()
            && let Some(&(_, section)) = dialog.nav.iter().find(|(nav, _)| *nav == id)
        {
            self.select_section(root, &mut dialog, section);
            self.popup = Some(Popup::Settings { dialog, menu: None });
            return Outcome::default();
        }

        let (mut effect, menu) = self.on_binding(root, &mut dialog.bindings, id, action);
        if action.downcast_ref::<Pressed>().is_some()
            && let Some(Binding::Button(button)) = dialog.bindings.get(&id)
        {
            let button = button.clone();
            effect = self.press(&mut dialog, button);
        }
        self.keep(dialog, menu, effect, root)
    }

    fn keep(&mut self, dialog: SettingsDialog, menu: Option<OpenMenu>, effect: Effect, root: &mut RenderRoot) -> Outcome {
        self.popup = Some(Popup::Settings { dialog, menu });
        self.finish(root, effect)
    }

    fn finish(&mut self, root: &mut RenderRoot, effect: Effect) -> Outcome {
        if let Some(ids) = &self.note_ids {
            let smart = crate::ui::editor::Smart::from_config(&self.workspace.core.settings.get_config());
            ids.body.edit(root, |mut body| crate::ui::editor::TextEditor::set_smart(&mut body, smart));
        }
        match effect {
            Effect::None => Outcome::default(),
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
                self.restyle(root, fonts);
                Outcome::default()
            }
            Effect::Everything => {
                self.rebuild(root);
                Outcome::default()
            }
        }
    }

    fn select_section(&mut self, root: &mut RenderRoot, dialog: &mut SettingsDialog, section: Section) {
        if dialog.section == section {
            return;
        }
        for &(nav, item) in &dialog.nav {
            root.edit_widget(nav, |mut w| ItemButton::set_active(&mut w.downcast(), item == section));
        }
        dialog.section = section;
        dialog.copied = None;
        dialog.trash_page = 0;
        self.interface.set_settings_section(section.key());
        let config = self.workspace.core.settings.get_config();
        let ctx = self.settings_context(None, 0);
        let (content, bindings) = settings::content(&config, &ctx, section);
        dialog.bindings = bindings;
        dialog.content.edit(root, |mut area| {
            ScrollArea::set_child(&mut area, content);
            ScrollArea::set_offset(&mut area, 0.0);
        });
    }

    pub(super) fn refresh_settings(&mut self, root: &mut RenderRoot) {
        let Some(Popup::Settings { dialog, .. }) = &self.popup else { return };
        let (section, copied, page) = (dialog.section, dialog.copied, dialog.trash_page);
        let config = self.workspace.core.settings.get_config();
        let ctx = self.settings_context(copied, page);
        let (content, bindings) = settings::content(&config, &ctx, section);
        let Some(Popup::Settings { dialog, .. }) = &mut self.popup else { return };
        let switches = switch_travel(root, &dialog.bindings);
        dialog.bindings = bindings;
        dialog.content.edit(root, |mut area| ScrollArea::set_child(&mut area, content));
        resume_switches(root, &dialog.bindings, &switches);
        self.refocus(root);
    }

    pub fn settings_key(&mut self, root: &mut RenderRoot, key: &Key) -> Option<Outcome> {
        let direction = match key {
            Key::Named(NamedKey::ArrowUp) => 1.0,
            Key::Named(NamedKey::ArrowDown) => -1.0,
            _ => return None,
        };
        let field = root.focused_widget()?;
        let Some(Popup::Settings { dialog, menu: None }) = &mut self.popup else { return None };
        let Some(&Binding::Number { setter, .. }) = dialog.bindings.get(&field) else { return None };
        let mut bindings = std::mem::take(&mut dialog.bindings);
        let effect = self.step_number(root, &mut bindings, field, direction);
        if let Some(Binding::Number { number, .. }) = bindings.get(&field) {
            let text = schema::format_number(number.value);
            self.typing = Some((setter, text.clone(), text.len()));
        }
        if let Some(Popup::Settings { dialog, .. }) = &mut self.popup {
            dialog.bindings = bindings;
        }
        Some(self.finish(root, effect))
    }

}
