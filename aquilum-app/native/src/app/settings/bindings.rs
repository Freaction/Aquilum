use super::*;

impl App {
    pub(in crate::app) fn on_binding(&mut self, root: &mut RenderRoot, bindings: &mut HashMap<WidgetId, Binding>, id: WidgetId, action: &ErasedAction) -> (Effect, Option<OpenMenu>) {
        let mut menu = None;
        let mut effect = Effect::None;
        if let Some(changed) = action.downcast_ref::<Changed>() {
            match (bindings.get(&id), changed) {
                (Some(Binding::Switch(setter)), Changed::Bool(on)) => effect = self.apply(*setter, Value::Bool(*on)),
                (Some(Binding::Choice(setter)), Changed::Choice(index)) => effect = self.apply(*setter, Value::Index(*index)),
                (Some(Binding::Color(_)), Changed::OpenColor(rect)) => {
                    let hex = self.workspace.core.settings.get_config().ui.primary_color;
                    let picker = ColorPicker::new(&hex, *rect);
                    let layer = picker.id();
                    let size = ColorPicker::size();
                    let below = rect.y1 + size::SPACE_6;
                    let y = if below + size.height > self.window.height { rect.y0 - size::SPACE_6 - size.height } else { below };
                    let x = rect.x0.min(self.window.width - size.width - size::SPACE_8).max(size::SPACE_8);
                    root.add_layer(picker, Point::new(x, y.max(size::SPACE_8)));
                    root.focus_on(Some(layer));
                    menu = Some(OpenMenu { layer, button: id });
                }
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
                _ => {}
            }
        } else if action.downcast_ref::<Pressed>().is_some()
            && let Some(&Binding::Step { field, direction }) = bindings.get(&id)
        {
            effect = self.step_number(root, bindings, field, direction);
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
            if let Some(text) = field.get(root).map(|f| f.inner().text()) {
                effect = self.field_changed(root, bindings, field.id(), &text, true);
            }
        }
        (effect, menu)
    }

    pub(super) fn field_changed(&mut self, root: &mut RenderRoot, bindings: &mut HashMap<WidgetId, Binding>, id: WidgetId, text: &str, entered: bool) -> Effect {
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
                self.typing = Some((setter, text.to_owned(), caret(root, id)));
                update_steps(root, minus, plus, &number);
                self.apply(setter, Value::Number(value))
            }
            Some(Binding::Text { setter, commit, value }) => {
                if *commit != entered || value == text {
                    return Effect::None;
                }
                text.clone_into(value);
                let setter = *setter;
                self.typing = Some((setter, text.to_owned(), caret(root, id)));
                self.apply(setter, Value::Text(text.to_owned()))
            }
            _ => Effect::None,
        }
    }

    pub(super) fn preview_color(&mut self, root: &mut RenderRoot, dialog: &mut SettingsDialog, swatch: WidgetId, hex: &str) {
        let config = self.workspace.core.settings.get_config();
        theme::apply_setting(&config.ui.theme, self.system_dark, hex);
        root.restyle(std::sync::Arc::new(theme::default_properties()), false);
        let Some(&Binding::Color(setter)) = dialog.bindings.get(&swatch) else { return };
        let shown = hex.to_uppercase();
        for (id, binding) in &mut dialog.bindings {
            if let Binding::Text { setter: s, value, .. } = binding
                && same_setter(*s, setter)
            {
                hex.clone_into(value);
                root.edit_widget(*id, |mut w| TextEditor::reset_text(&mut w.downcast(), &shown));
            }
        }
    }

    pub(super) fn step_number(&mut self, root: &mut RenderRoot, bindings: &mut HashMap<WidgetId, Binding>, field: WidgetId, direction: f64) -> Effect {
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
}

pub(super) fn switch_travel(root: &RenderRoot, bindings: &HashMap<WidgetId, Binding>) -> Vec<(Setter, f64)> {
    bindings
        .iter()
        .filter_map(|(id, binding)| match binding {
            Binding::Switch(setter) => root.get_widget(*id)?.downcast::<Switch>().map(|w| (*setter, w.inner().travel())),
            _ => None,
        })
        .collect()
}

pub(super) fn resume_switches(root: &mut RenderRoot, bindings: &HashMap<WidgetId, Binding>, travel: &[(Setter, f64)]) {
    for (id, binding) in bindings {
        if let Binding::Switch(setter) = binding
            && let Some(&(_, travel)) = travel.iter().find(|(s, _)| same_setter(*s, *setter))
        {
            root.edit_widget(*id, |mut w| Switch::resume(&mut w.downcast(), travel));
        }
    }
}

fn caret(root: &RenderRoot, field: WidgetId) -> usize {
    root.get_widget(field).and_then(|w| w.downcast::<TextEditor>().map(|e| e.inner().caret_offset())).unwrap_or(usize::MAX)
}

pub(super) fn same_setter(a: Setter, b: Setter) -> bool {
    match (a, b) {
        (Setter::Config(x), Setter::Config(y)) => std::ptr::fn_addr_eq(x, y),
        (Setter::VaultSync(x), Setter::VaultSync(y)) => std::ptr::fn_addr_eq(x, y),
        (Setter::Scale, Setter::Scale) | (Setter::HomePage, Setter::HomePage) | (Setter::Autostart, Setter::Autostart) => true,
        _ => false,
    }
}

fn update_steps(root: &mut RenderRoot, minus: Option<WidgetId>, plus: Option<WidgetId>, number: &schema::Number) {
    if let Some(minus) = minus {
        root.edit_widget(minus, |mut w| w.ctx.set_disabled(number.value <= number.min));
    }
    if let Some(plus) = plus {
        root.edit_widget(plus, |mut w| w.ctx.set_disabled(number.value >= number.max));
    }
}
