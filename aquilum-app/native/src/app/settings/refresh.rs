use masonry::app::RenderRoot;

use super::bindings::{resume_switches, same_setter, switch_travel};
use super::{App, Popup};
use crate::ui::dialog::Modal;
use crate::ui::editor::TextEditor;
use crate::ui::scroll::ScrollArea;
use crate::ui::settings::schema::Section;
use crate::ui::settings::{self, Binding, SettingsDialog};
use crate::ui::widgets::ItemButton;

impl App {
    pub(super) fn reload_settings(&mut self, root: &mut RenderRoot) {
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

    pub(super) fn select_section(&mut self, root: &mut RenderRoot, dialog: &mut SettingsDialog, section: Section) {
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

    pub(in crate::app) fn refresh_settings(&mut self, root: &mut RenderRoot) {
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
}
