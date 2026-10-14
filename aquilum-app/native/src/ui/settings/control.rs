use std::collections::HashMap;

use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::Gap;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::Binding;
use super::controls::{DropdownButton, InputBox, Segmented, Swatch, Switch};
use super::schema::{self, Control, Row, Tone};
use crate::i18n::t;
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::text::selectable;
use crate::ui::tokens::size;
use crate::ui::widgets::{IconButton, Leading, LinkButton, Paint, kbd};
use crate::ui::{icons, style};

fn text_field(value: &str, centered: bool) -> NewWidget<TextEditor> {
    let mut field = TextEditor::single_line(value).styled(|| style::ui_text(size::INPUT_CONTENT_HEIGHT as f32));
    if centered {
        field = field.centered();
    }
    NewWidget::new(field).with_class(style::FIELD)
}

fn plain_input(
    bindings: &mut HashMap<WidgetId, Binding>,
    field: NewWidget<TextEditor>,
    binding: Option<Binding>,
    commit: bool,
) -> NewWidget<dyn Widget> {
    let handle = Handle::of(&field);
    let bound = binding.is_some();
    if let Some(binding) = binding {
        bindings.insert(field.id(), binding);
    }
    let input = InputBox::new(field, size::INPUT_WIDTH);
    if commit && bound {
        bindings.insert(input.id(), Binding::Commit(handle));
    }
    input.erased()
}

fn stepper_input(bindings: &mut HashMap<WidgetId, Binding>, setter: Option<schema::Setter>, number: &schema::Number) -> NewWidget<dyn Widget> {
    let field = text_field(&schema::format_number(number.value), true);
    let field_id = field.id();
    let step = |icon, label: &str, enabled: bool| NewWidget::new(IconButton::new(icon, t(label)).xs()).disabled(!enabled);
    let minus = step(icons::MINUS, "common.decrease", number.value > number.min);
    let plus = step(icons::PLUS, "common.increase", number.value < number.max);
    let (minus_id, plus_id) = (minus.id(), plus.id());
    if let Some(setter) = setter {
        bindings.insert(field_id, Binding::Number { setter, number: number.clone(), minus: Some(minus_id), plus: Some(plus_id) });
        bindings.insert(minus_id, Binding::Step { field: field_id, direction: -1.0 });
        bindings.insert(plus_id, Binding::Step { field: field_id, direction: 1.0 });
    }
    let inner = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(minus).with(field, 1.0).with_fixed(plus);
    let inner = NewWidget::new(inner).with_props(PropertySet::new().with(Gap::new(Length::px(size::INPUT_GAP))));
    let width = 4.0 * size::FONT_SIZE_UI_BASE * 0.6 + 2.0 * size::INPUT_CONTENT_HEIGHT + 2.0 * size::INPUT_GAP + 2.0 * size::INPUT_PADDING_X;
    InputBox::new(inner, width.ceil()).erased()
}

fn color_input(bindings: &mut HashMap<WidgetId, Binding>, setter: Option<schema::Setter>, hex: &str) -> NewWidget<dyn Widget> {
    let field = text_field(&hex.to_uppercase(), false);
    let swatch = Swatch::new();
    if let Some(setter) = setter {
        bindings.insert(field.id(), Binding::Text { setter, commit: false, value: hex.to_owned() });
        bindings.insert(swatch.id(), Binding::Color(setter));
    }
    let inner = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(swatch).with(field, 1.0);
    let inner = NewWidget::new(inner).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_MD))));
    InputBox::new(inner, size::INPUT_WIDTH).erased()
}

pub(super) fn control_widget(row: &Row, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<dyn Widget> {
    let setter = row.setter;
    match &row.control {
        Control::Switch(on) => {
            let widget = Switch::new(*on, &row.label);
            if let Some(setter) = setter {
                bindings.insert(widget.id(), Binding::Switch(setter));
            }
            widget.erased()
        }
        Control::Locked(on) => Switch::locked(*on, &row.label).erased(),
        Control::Buttons(buttons) => {
            let mut flex = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
            for button in buttons {
                let widget = LinkButton::new(&button.label).disabled(!button.enabled);
                bindings.insert(widget.id(), Binding::Button(button.action.clone()));
                flex = flex.with_fixed(widget);
            }
            NewWidget::new(flex).erased()
        }
        Control::Status { text, tone } => {
            let paint: Paint = match tone {
                Tone::Muted => |t| t.text_muted,
                Tone::Secondary => |t| t.text_secondary,
                Tone::Danger => |t| t.text_danger,
            };
            selectable(text, paint, None, Leading::Times(1.21)).erased()
        }
        Control::Segmented { options, selected } => {
            let widget = Segmented::new(options.clone(), *selected);
            if let Some(setter) = setter {
                bindings.insert(widget.id(), Binding::Choice(setter));
            }
            widget.erased()
        }
        Control::Dropdown { options, selected } => {
            let widget = DropdownButton::new(options.get(*selected).map_or("", String::as_str));
            if let Some(setter) = setter {
                bindings.insert(widget.id(), Binding::Dropdown { setter, options: options.clone(), selected: *selected });
            }
            widget.erased()
        }
        Control::Number(number) if number.stepper => stepper_input(bindings, setter, number),
        Control::Number(number) => {
            let field = text_field(&schema::format_number(number.value), false);
            let binding = setter.map(|setter| Binding::Number { setter, number: number.clone(), minus: None, plus: None });
            plain_input(bindings, field, binding, number.commit)
        }
        Control::Text { value, commit } => {
            let binding = setter.map(|setter| Binding::Text { setter, commit: *commit, value: value.clone() });
            plain_input(bindings, text_field(value, false), binding, *commit)
        }
        Control::Color(hex) => color_input(bindings, setter, hex),
        Control::Kbd(keys) => kbd(keys, size::SPACE_4).erased(),
    }
}
