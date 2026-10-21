use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};

use super::bindings::same_setter;
use super::{App, Effect, Outcome, Popup};
use crate::ui::dialog::DialogAnswer;
use crate::ui::menu::MenuChoice;
use crate::ui::settings::Binding;
use crate::ui::settings::controls::{Changed, DropdownButton, Picked};
use crate::ui::settings::schema::Value;
use crate::ui::widgets::Pressed;

impl App {
    pub(in crate::app) fn on_settings_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        self.typing = None;
        let Some(Popup::Settings { mut dialog, menu }) = self.popup.take() else { return Outcome::default() };
        let mut effect = Effect::None;

        if let Some((confirm, wipe)) = dialog.confirm.take() {
            let Some(answer) = confirm.answer(id, action) else {
                dialog.confirm = Some((confirm, wipe));
                self.popup = Some(Popup::Settings { dialog, menu });
                return Outcome::default();
            };
            crate::ui::dialog::close(root, confirm.layer);
            root.focus_on(Some(dialog.layer));
            if answer == DialogAnswer::Confirm {
                self.wipe(root, wipe);
                effect = Effect::Content;
            }
            return self.keep(dialog, menu, effect, root);
        }

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
                None | Some(MenuChoice::Hover(_) | MenuChoice::Sub(_)) => self.popup = Some(Popup::Settings { dialog, menu: Some(menu) }),
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
            effect = self.press(root, &mut dialog, button);
        }
        self.keep(dialog, menu, effect, root)
    }
}
