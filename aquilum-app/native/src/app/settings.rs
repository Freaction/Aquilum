use std::collections::HashMap;
use std::path::Path;

use aquilum_core::files::gate;
use crate::ui::editor::TextEditor;
use aquilum_core::files::trash::{cleanup_trash_impl, ensure_trash_impl};
use aquilum_core::history::Source;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Point;
use masonry::widgets::{TextAction};

use super::{App, Outcome, Popup};
use crate::system;
use crate::ui::dialog::DialogAnswer;
use crate::ui::menu::{Menu, MenuChoice};
use crate::ui::settings::controls::{Changed, Committed, DropdownButton, StepButton};
use crate::ui::settings::schema::{self, Action, Context, Section, Setter, Value};
use crate::ui::settings::{self, Binding, SettingsDialog};
use crate::ui::scroll::ScrollArea;
use crate::ui::widgets::{ItemButton, Pressed};

pub(super) struct OpenMenu {
    layer: WidgetId,
    button: WidgetId,
}

pub(super) enum Effect {
    None,
    Rescale,
    Content,
    Everything,
}

const NOTE_STARTER: &str = "---\ncreated: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})\ntags: []\nauthor: \nsource: \n---\n\n\n\n\n***\n\nСсылки:\n- [[]]\n";
const BOOK_STARTER: &str = "---\ncreated: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})\ncover: true\ntype: book\nauthor: \nstatus: to-read\npages: \ntags: [книга, ]\nrating: 0\nBook_cover: \nPage_cover: pattern:tunnel\nПуть к файлу: \nreader_position: \nread_percent: \n---\n\n\n\n***\n\nСсылки:\n- [[]]\n";

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
        let mut config = self.workspace.core.settings.get_config();
        if config.mcp.token.trim().is_empty() {
            config.mcp.token = schema::new_token();
            if let Err(error) = self.workspace.core.settings.update_config(config.clone()) {
                eprintln!("токен MCP не сохранён: {error}");
            }
        }
        let ctx = self.settings_context(None, 0);
        let (layer, dialog) = settings::dialog(&config, &ctx, section);
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(dialog.layer));
        self.popup = Some(Popup::Settings { dialog, menu: None });
    }

    pub(super) fn close_settings(&mut self, root: &mut RenderRoot) -> Option<Section> {
        let Some(Popup::Settings { dialog, menu }) = self.popup.take() else { return None };
        if let Some(menu) = menu {
            root.remove_layer(menu.layer);
        }
        root.remove_layer(dialog.layer);
        Some(dialog.section)
    }

    pub(super) fn rebuild(&mut self, root: &mut RenderRoot) {
        let offset = match &self.popup {
            Some(Popup::Settings { dialog, .. }) => dialog.content.get(root).map(|area| area.inner().offset()),
            _ => None,
        };
        let section = self.close_settings(root);
        self.replace_root(root);
        let Some(section) = section else { return };
        self.show_settings(root, section);
        let typing = self.typing.take();
        let Some(Popup::Settings { dialog, .. }) = &self.popup else { return };
        if let Some(offset) = offset {
            dialog.content.edit(root, |mut area| ScrollArea::set_offset(&mut area, offset));
        }
        let Some((setter, text)) = typing else { return };
        let field = dialog.bindings.iter().find_map(|(id, binding)| match binding {
            Binding::Number { setter: s, .. } if same_setter(*s, setter) => Some(*id),
            _ => None,
        });
        if let Some(field) = field {
            root.focus_on(Some(field));
            root.edit_widget(field, |mut w| {
                let mut editor = w.downcast::<TextEditor>();
                TextEditor::reset_text(&mut editor, &text);
                TextEditor::select_byte_range(&mut editor, text.len(), text.len());
            });
        }
    }

    pub(super) fn on_settings_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        self.typing = None;
        let Some(Popup::Settings { mut dialog, menu }) = self.popup.take() else { return Outcome::default() };
        let mut effect = Effect::None;

        if let Some(menu) = menu {
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
            root.remove_layer(dialog.layer);
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

    pub(super) fn on_binding(&mut self, root: &mut RenderRoot, bindings: &mut HashMap<WidgetId, Binding>, id: WidgetId, action: &ErasedAction) -> (Effect, Option<OpenMenu>) {
        let mut menu = None;
        let mut effect = Effect::None;
        if let Some(changed) = action.downcast_ref::<Changed>() {
            match (bindings.get(&id), changed) {
                (Some(Binding::Switch(setter)), Changed::Bool(on)) => effect = self.apply(*setter, Value::Bool(*on)),
                (Some(Binding::Choice(setter)), Changed::Choice(index)) => effect = self.apply(*setter, Value::Index(*index)),
                (Some(Binding::Dropdown { options, selected, .. }), Changed::OpenMenu(rect)) => {
                    let items: Vec<&str> = options.iter().map(String::as_str).collect();
                    let origin = Menu::list_origin(*rect, items.len(), self.window);
                    let list = Menu::list("", &items, *selected, *rect);
                    let layer = list.id();
                    root.add_layer(list, origin);
                    root.edit_widget(id, |mut w| DropdownButton::set_open(&mut w.downcast(), true));
                    root.focus_on(Some(layer));
                    menu = Some(OpenMenu { layer, button: id });
                }
                (Some(Binding::Step { field, direction }), Changed::Step(_)) => {
                    let (field, direction) = (*field, *direction);
                    effect = self.step_number(root, bindings, field, direction);
                }
                _ => {}
            }
        } else if let Some(text_action) = action.downcast_ref::<TextAction>() {
            let (text, entered) = match text_action {
                TextAction::Changed(text) => (text, false),
                TextAction::Entered(text) => (text, true),
                _ => return (effect, menu),
            };
            effect = self.field_changed(root, bindings, id, text, entered);
        } else if action.downcast_ref::<Committed>().is_some()
            && let Some(Binding::Commit(field)) = bindings.get(&id)
        {
            let field = *field;
            if let Some(text) = field.get(root).map(|f| f.inner().text().to_string()) {
                effect = self.field_changed(root, bindings, field.id(), &text, true);
            }
        }
        (effect, menu)
    }

    fn keep(&mut self, dialog: SettingsDialog, menu: Option<OpenMenu>, effect: Effect, root: &mut RenderRoot) -> Outcome {
        self.popup = Some(Popup::Settings { dialog, menu });
        self.finish(root, effect)
    }

    fn field_changed(&mut self, root: &mut RenderRoot, bindings: &mut HashMap<WidgetId, Binding>, id: WidgetId, text: &str, entered: bool) -> Effect {
        match bindings.get_mut(&id) {
            Some(Binding::Number { setter, number, minus, plus }) => {
                if number.commit != entered {
                    return Effect::None;
                }
                let Some(value) = number.parse(text) else { return Effect::None };
                if value == number.value {
                    return Effect::None;
                }
                number.value = value;
                let (setter, minus, plus, number) = (*setter, *minus, *plus, number.clone());
                self.typing = Some((setter, text.to_owned()));
                update_steps(root, minus, plus, &number);
                self.apply(setter, Value::Number(value))
            }
            Some(Binding::Text { setter, commit, value }) => {
                if *commit != entered || value == text {
                    return Effect::None;
                }
                text.clone_into(value);
                let setter = *setter;
                self.apply(setter, Value::Text(text.to_owned()))
            }
            _ => Effect::None,
        }
    }

    fn press(&mut self, dialog: &mut SettingsDialog, action: Action) -> Effect {
        let core = &self.workspace.core;
        let workspace = self.tree.root().map(Path::to_path_buf);
        match action {
            Action::OpenTrash => {
                if let Some(workspace) = workspace {
                    match ensure_trash_impl(&workspace) {
                        Ok(path) => system::open(&path),
                        Err(error) => eprintln!("корзина не открыта: {error:?}"),
                    }
                }
                Effect::None
            }
            Action::Restore(id) => {
                let Some(workspace) = workspace else { return Effect::None };
                if let Err(error) = gate::restore_deletion(core, &workspace, &id) {
                    eprintln!("заметка не восстановлена: {error:?}");
                }
                Effect::Content
            }
            Action::TrashPage(page) => {
                dialog.trash_page = page;
                Effect::Content
            }
            Action::CreateTemplates => {
                let Some(workspace) = workspace else { return Effect::None };
                let folder = schema::templates_path(&workspace, &core.settings.get_config().templates.folder);
                if let Err(error) = std::fs::create_dir_all(&folder) {
                    eprintln!("папка шаблонов не создана: {error}");
                    return Effect::None;
                }
                for (name, content) in [("Note.md", NOTE_STARTER), ("Book.md", BOOK_STARTER)] {
                    let path = folder.join(name);
                    if !path.exists()
                        && let Err(error) = gate::create(core, &path, content, Source::Me, None)
                    {
                        eprintln!("{} не создан: {error:?}", path.display());
                    }
                }
                Effect::Content
            }
            Action::CheckUpdates => {
                self.updater.check();
                Effect::Content
            }
            Action::InstallUpdate => {
                self.updater.install();
                Effect::Content
            }
            Action::NewToken => self.apply(Setter::Config(|c, _| c.mcp.token = schema::new_token()), Value::Bool(true)),
            Action::Copy(index, code) => {
                match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(code)) {
                    Ok(()) => dialog.copied = Some(index),
                    Err(error) => eprintln!("не скопировано: {error}"),
                }
                Effect::Content
            }
            Action::OpenUrl(url) => {
                system::open(url);
                Effect::None
            }
        }
    }

    fn finish(&mut self, root: &mut RenderRoot, effect: Effect) -> Outcome {
        match effect {
            Effect::None => Outcome::default(),
            Effect::Rescale => Outcome { rescale: true, ..Outcome::default() },
            Effect::Content => {
                self.refresh_settings(root);
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
        dialog.bindings = bindings;
        dialog.content.edit(root, |mut area| ScrollArea::set_child(&mut area, content));
    }

    fn step_number(&mut self, root: &mut RenderRoot, bindings: &mut HashMap<WidgetId, Binding>, field: WidgetId, direction: f64) -> Effect {
        let Some(Binding::Number { setter, number, minus, plus }) = bindings.get_mut(&field) else { return Effect::None };
        let value = number.step_from(number.value, direction);
        if value == number.value {
            return Effect::None;
        }
        number.value = value;
        let (setter, minus, plus, number) = (*setter, *minus, *plus, number.clone());
        let text = schema::format_number(value);
        root.edit_widget(field, |mut w| TextEditor::reset_text(&mut w.downcast(), &text));
        update_steps(root, minus, plus, &number);
        self.apply(setter, Value::Number(value))
    }

    pub fn scale_changed(&mut self, root: &mut RenderRoot) {
        let Some(Popup::Settings { dialog, .. }) = &mut self.popup else { return };
        let percent = (self.interface.scale() * 100.0).round();
        let found = dialog.bindings.iter_mut().find_map(|(id, binding)| match binding {
            Binding::Number { setter: Setter::Scale, number, minus, plus } => {
                number.value = percent;
                Some((*id, *minus, *plus, number.clone()))
            }
            _ => None,
        });
        if let Some((field, minus, plus, number)) = found {
            let text = schema::format_number(percent);
            root.edit_widget(field, |mut w| TextEditor::reset_text(&mut w.downcast(), &text));
            update_steps(root, minus, plus, &number);
        }
    }

    fn apply(&mut self, setter: Setter, value: Value) -> Effect {
        match setter {
            Setter::Scale => {
                let Value::Number(percent) = value else { return Effect::None };
                if self.interface.set_scale(percent / 100.0) { Effect::Rescale } else { Effect::None }
            }
            Setter::Autostart => {
                let Value::Bool(on) = value else { return Effect::None };
                if !crate::autostart::set(on) {
                    eprintln!("автозапуск не изменён");
                }
                Effect::Content
            }
            Setter::HomePage => {
                let (Value::Text(page), Some(root)) = (value, self.tree.root()) else { return Effect::None };
                if let Err(error) = self.workspace.core.ui_state.set_home_page(&root.to_string_lossy(), page.trim()) {
                    eprintln!("домашняя страница не сохранена: {error:?}");
                }
                self.refresh_home();
                Effect::None
            }
            Setter::Config(set) => {
                let core = &self.workspace.core;
                let before = core.settings.get_config();
                let mut after = before.clone();
                set(&mut after, &value);
                let ui_changed = before.ui.theme != after.ui.theme
                    || before.ui.language != after.ui.language
                    || before.ui.font.font_family != after.ui.font.font_family
                    || before.ui.font.font_weight != after.ui.font.font_weight
                    || before.ui.font.font_size_base != after.ui.font.font_size_base
                    || before.ui.primary_color != after.ui.primary_color
                    || before.editor.font.font_family != after.editor.font.font_family
                    || before.editor.font.font_weight != after.editor.font.font_weight
                    || before.editor.font.font_size_base != after.editor.font.font_size_base
                    || before.editor.line_height != after.editor.line_height
                    || before.editor.max_width_ch != after.editor.max_width_ch;
                let mcp_changed = serde_json::to_value(&before.mcp).ok() != serde_json::to_value(&after.mcp).ok();
                let retention = (before.trash.retention_days != after.trash.retention_days).then_some(after.trash.retention_days);
                let rows_changed = before.analysis.enable_bm25f != after.analysis.enable_bm25f
                    || before.updates.auto != after.updates.auto
                    || mcp_changed
                    || retention.is_some();
                if let Err(error) = core.settings.update_config(after) {
                    eprintln!("настройки не сохранены: {error}");
                    return Effect::None;
                }
                if mcp_changed {
                    core.apply_mcp_settings();
                }
                if let (Some(days), Some(workspace)) = (retention, self.tree.root())
                    && let Err(error) = cleanup_trash_impl(workspace, days)
                {
                    eprintln!("корзина не очищена: {error:?}");
                }
                if ui_changed {
                    Effect::Everything
                } else if rows_changed {
                    Effect::Content
                } else {
                    Effect::None
                }
            }
        }
    }
}

fn same_setter(a: Setter, b: Setter) -> bool {
    match (a, b) {
        (Setter::Config(x), Setter::Config(y)) => std::ptr::fn_addr_eq(x, y),
        (Setter::Scale, Setter::Scale) | (Setter::HomePage, Setter::HomePage) | (Setter::Autostart, Setter::Autostart) => true,
        _ => false,
    }
}

fn update_steps(root: &mut RenderRoot, minus: Option<WidgetId>, plus: Option<WidgetId>, number: &schema::Number) {
    if let Some(minus) = minus {
        root.edit_widget(minus, |mut w| StepButton::set_enabled(&mut w.downcast(), number.value > number.min));
    }
    if let Some(plus) = plus {
        root.edit_widget(plus, |mut w| StepButton::set_enabled(&mut w.downcast(), number.value < number.max));
    }
}
