pub mod controls;
pub mod schema;

use std::collections::HashMap;
use crate::ui::editor::TextEditor;

use aquilum_core::settings::models::AppConfig;
use masonry::accesskit::Role;
use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::{Axis, Size};
use masonry::layout::Length;
use masonry::parley::LineHeight;
use masonry::parley::style::StyleProperty;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{
    Background, BorderColor, BorderWidth, CaretColor, ContentColor, CornerRadius, Gap, LineBreaking, Padding,
    SelectionColor,
};
use masonry::widgets::{Flex, SizedBox};

use crate::i18n::t;
use crate::ui::dialog::{CardSize, Modal, header};
use crate::ui::handle::Handle;
use crate::ui::scroll::ScrollArea;
use crate::ui::text::label;
use crate::ui::tokens::size;
use crate::ui::widgets::{IconButton, ItemButton, LinkButton, Rule};
use crate::ui::{icons, theme};
use controls::{DropdownButton, InputBox, Segmented, StepButton, Swatch, Switch};
use schema::{Action, Block, Context, Control, Number, Row, Section, Setter, Tone};

pub enum Binding {
    Switch(Setter),
    Choice(Setter),
    Dropdown { setter: Setter, options: Vec<String>, selected: usize },
    Number { setter: Setter, number: Number, minus: Option<WidgetId>, plus: Option<WidgetId> },
    Step { field: WidgetId, direction: f64 },
    Text { setter: Setter, commit: bool, value: String },
    Commit(Handle<TextEditor>),
    Button(Action),
}

pub struct SettingsDialog {
    pub layer: WidgetId,
    pub close: WidgetId,
    pub content: Handle<ScrollArea<Flex>>,
    pub nav: Vec<(WidgetId, Section)>,
    pub section: Section,
    pub bindings: HashMap<WidgetId, Binding>,
    pub copied: Option<usize>,
    pub trash_page: usize,
}

fn card_size(room: Size) -> Size {
    let window = Size::new(room.width + size::SPACE_32, room.height + size::SPACE_32);
    Size::new(
        room.width.min(window.width * 0.9).min(size::SETTINGS_MAX_WIDTH),
        (window.height * 0.85).min(size::SETTINGS_MAX_HEIGHT).min(room.height),
    )
}

pub fn dialog(config: &AppConfig, ctx: &Context, section: Section) -> (NewWidget<Modal>, SettingsDialog) {
    let close = NewWidget::new(IconButton::new(icons::X, t("settings.close")));
    let close_id = close.id();

    let mut nav = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut nav_ids = Vec::new();
    for item in Section::ALL {
        let button = NewWidget::new(ItemButton::new(item.icon(), item.label()).active(item == section));
        nav_ids.push((button.id(), item));
        nav = nav.with_fixed(button);
    }
    let nav = NewWidget::new(nav).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::SPACE_2))).with(Padding::all(Length::px(size::SPACE_8))),
    );
    let nav = NewWidget::new(SizedBox::new(NewWidget::new(ScrollArea::new(nav))).width(Length::px(size::SETTINGS_NAV_WIDTH)));

    let (blocks, bindings) = content(config, ctx, section);
    let content = NewWidget::new(ScrollArea::new(blocks));
    let content_handle = Handle::of(&content);
    let body = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(nav)
        .with_fixed(Rule::new(Axis::Vertical, || theme::current().dialog_border))
        .with(content, 1.0);

    let card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(header(&t("settings.title"), Some(close)))
        .with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default))
        .with(NewWidget::new(body), 1.0);
    let layer = Modal::new(NewWidget::new(card), &t("settings.title"), CardSize::Window(card_size), Role::Dialog, false);
    let dialog = SettingsDialog {
        layer: layer.id(),
        close: close_id,
        content: content_handle,
        nav: nav_ids,
        section,
        bindings,
        copied: ctx.copied,
        trash_page: ctx.trash_page,
    };
    (layer, dialog)
}

pub fn content(config: &AppConfig, ctx: &Context, section: Section) -> (NewWidget<Flex>, HashMap<WidgetId, Binding>) {
    let mut bindings = HashMap::new();
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    for block in schema::blocks(section, config, ctx) {
        column = column.with_fixed(block_widget(block, &mut bindings));
    }
    let column = NewWidget::new(column).with_props(PropertySet::new().with(Padding {
        top: Length::ZERO,
        bottom: Length::px(size::GAP_XL),
        left: Length::px(size::GAP_XL),
        right: Length::px(size::GAP_XL),
    }));
    (column, bindings)
}

pub fn rows(rows: Vec<Row>) -> (NewWidget<Flex>, HashMap<WidgetId, Binding>) {
    let mut bindings = HashMap::new();
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let count = rows.len();
    for (i, row) in rows.into_iter().enumerate() {
        column = column.with_fixed(row_widget(row, &mut bindings));
        if i + 1 < count {
            column = column.with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default));
        }
    }
    (NewWidget::new(column), bindings)
}

fn block_widget(block: Block, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<Flex> {
    let t = theme::current();
    let title = NewWidget::new(Flex::row().with(label(&block.title, t.text_secondary, Some(600.0), size::SIZE_20 as f32), 1.0))
        .with_props(PropertySet::new().with(Padding {
            top: Length::px(size::GAP_XL),
            bottom: Length::px(size::GAP_LG),
            left: Length::px(size::GAP_LG),
            right: Length::px(size::GAP_LG),
        }));
    let mut card = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let count = block.rows.len();
    for (i, row) in block.rows.into_iter().enumerate() {
        card = card.with_fixed(row_widget(row, bindings));
        if i + 1 < count {
            card = card.with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default));
        }
    }
    let card = NewWidget::new(card).with_props(
        PropertySet::new()
            .with(Background::Color(t.bg_surface_subtle))
            .with(CornerRadius { radius: Length::px(size::RADIUS_2XL) })
            .with(Padding { top: Length::ZERO, bottom: Length::ZERO, left: Length::px(size::GAP_XL), right: Length::px(size::GAP_XL) }),
    );
    NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(title).with_fixed(card))
}

fn row_widget(row: Row, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<Flex> {
    let code = row.code.clone();
    let t = theme::current();
    let line = (theme::ui_font_settings().size * 1.21).round();
    let mut text = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(label(&row.label, t.text_secondary, None, line));
    if let Some(description) = &row.description {
        text = text.with_fixed(label(description, t.text_muted, None, line));
    }
    let text = NewWidget::new(text).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_SM))));
    let strut = NewWidget::new(SizedBox::empty().height(Length::px(size::SETTINGS_ROW_TEXT_MIN_HEIGHT)));
    let control = NewWidget::new(Flex::column().main_axis_alignment(MainAxisAlignment::Center).with_fixed(control_widget(&row, bindings)));
    let flex = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(strut)
        .with(text, 1.0)
        .with_fixed(control);
    let Some(code) = code else {
        return NewWidget::new(flex).with_props(
            PropertySet::new().with(Gap::new(Length::px(size::GAP_LG))).with(Padding::from_vh(Length::px(size::GAP_XL), Length::ZERO)),
        );
    };
    let flex = NewWidget::new(flex).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_LG))).with(Padding {
        top: Length::px(size::GAP_XL),
        bottom: Length::px(size::GAP_LG),
        left: Length::ZERO,
        right: Length::ZERO,
    }));
    let column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(flex).with_fixed(code_block(&code));
    NewWidget::new(column).with_props(PropertySet::new().with(Padding {
        top: Length::ZERO,
        bottom: Length::px(size::GAP_XL),
        left: Length::ZERO,
        right: Length::ZERO,
    }))
}

fn code_block(code: &str) -> NewWidget<Flex> {
    let t = theme::current();
    let font_size = size::FONT_SIZE_UI_SM as f32;
    let text = masonry::widgets::Label::new(code)
        .with_style(StyleProperty::FontFamily(theme::editor_font("iA Writer Mono")))
        .with_style(StyleProperty::FontSize(font_size))
        .with_style(StyleProperty::LineHeight(LineHeight::FontSizeRelative(crate::ui::tokens::number::FONT_LINE_HEIGHT_NORMAL as f32)));
    let text = NewWidget::new(text).with_props(PropertySet::new().with(ContentColor::new(t.text_secondary)).with(LineBreaking::WordWrap));
    NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(text)).with_props(
        PropertySet::new()
            .with(Background::Color(t.block_bg))
            .with(BorderColor { color: t.block_border })
            .with(BorderWidth { width: Length::px(size::BORDER_1) })
            .with(CornerRadius { radius: Length::px(size::BLOCK_RADIUS) })
            .with(Padding::all(Length::px(size::GAP_LG))),
    )
}

fn text_field(value: &str, centered: bool) -> NewWidget<TextEditor> {
    let t = theme::current();
    let font = theme::ui_font_settings();
    let mut field = TextEditor::single_line(value)
        .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(font.size))
        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::INPUT_CONTENT_HEIGHT as f32)));
    if centered {
        field = field.centered();
    }
    NewWidget::new(field).with_props(
        PropertySet::new()
            .with(ContentColor::new(t.text_primary))
            .with(CaretColor { color: t.text_primary })
            .with(SelectionColor { color: t.selection_bg }),
    )
}

fn control_widget(row: &Row, bindings: &mut HashMap<WidgetId, Binding>) -> NewWidget<dyn Widget> {
    let setter = row.setter;
    let bind = |bindings: &mut HashMap<WidgetId, Binding>, id, binding| {
        if setter.is_some() {
            bindings.insert(id, binding);
        }
    };
    match &row.control {
        Control::Switch(on) => {
            let widget = Switch::new(*on, &row.label);
            if let Some(setter) = setter {
                bind(bindings, widget.id(), Binding::Switch(setter));
            }
            widget.erased()
        }
        Control::Locked(on) => Switch::locked(*on, &row.label).erased(),
        Control::Buttons(buttons) => {
            let mut flex = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
            for button in buttons {
                let widget = LinkButton::new(&button.label, button.enabled);
                bindings.insert(widget.id(), Binding::Button(button.action.clone()));
                flex = flex.with_fixed(widget);
            }
            NewWidget::new(flex).erased()
        }
        Control::Status { text, tone } => {
            let t = theme::current();
            let color = match tone {
                Tone::Muted => t.text_muted,
                Tone::Secondary => t.text_secondary,
                Tone::Danger => t.text_danger,
            };
            let line = (theme::ui_font_settings().size * 1.21).round();
            label(text, color, None, line).erased()
        }
        Control::Segmented { options, selected } => {
            let widget = Segmented::new(options.clone(), *selected);
            if let Some(setter) = setter {
                bind(bindings, widget.id(), Binding::Choice(setter));
            }
            widget.erased()
        }
        Control::Dropdown { options, selected } => {
            let widget = DropdownButton::new(options.get(*selected).map_or("", String::as_str));
            if let Some(setter) = setter {
                bind(bindings, widget.id(), Binding::Dropdown { setter, options: options.clone(), selected: *selected });
            }
            widget.erased()
        }
        Control::Number(number) => {
            let field = text_field(&schema::format_number(number.value), number.stepper);
            let field_id = field.id();
            let handle = Handle::of(&field);
            if !number.stepper {
                if let Some(setter) = setter {
                    bind(bindings, field_id, Binding::Number { setter, number: number.clone(), minus: None, plus: None });
                }
                let input = InputBox::new(field, size::INPUT_WIDTH);
                if number.commit {
                    bind(bindings, input.id(), Binding::Commit(handle));
                }
                return input.erased();
            }
            let minus = StepButton::new(icons::MINUS, -1.0, number.value > number.min);
            let plus = StepButton::new(icons::PLUS, 1.0, number.value < number.max);
            let (minus_id, plus_id) = (minus.id(), plus.id());
            if let Some(setter) = setter {
                bind(bindings, field_id, Binding::Number { setter, number: number.clone(), minus: Some(minus_id), plus: Some(plus_id) });
                bind(bindings, minus_id, Binding::Step { field: field_id, direction: -1.0 });
                bind(bindings, plus_id, Binding::Step { field: field_id, direction: 1.0 });
            }
            let inner = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(minus).with(field, 1.0).with_fixed(plus);
            let inner = NewWidget::new(inner).with_props(PropertySet::new().with(Gap::new(Length::px(size::INPUT_GAP))));
            let width = 4.0 * size::FONT_SIZE_UI_BASE * 0.6 + 2.0 * size::INPUT_CONTENT_HEIGHT + 2.0 * size::INPUT_GAP + 2.0 * size::INPUT_PADDING_X;
            InputBox::new(inner, width.ceil()).erased()
        }
        Control::Text { value, commit } => {
            let field = text_field(value, false);
            let handle = Handle::of(&field);
            if let Some(setter) = setter {
                bind(bindings, field.id(), Binding::Text { setter, commit: *commit, value: value.clone() });
            }
            let input = InputBox::new(field, size::INPUT_WIDTH);
            if *commit {
                bind(bindings, input.id(), Binding::Commit(handle));
            }
            input.erased()
        }
        Control::Color(hex) => {
            let field = text_field(&hex.to_uppercase(), false);
            if let Some(setter) = setter {
                bind(bindings, field.id(), Binding::Text { setter, commit: false, value: hex.clone() });
            }
            let color = theme::parse_hex(hex).map(|[r, g, b]| masonry::peniko::Color::from_rgb8(r, g, b));
            let inner = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(Swatch::new(color)).with(field, 1.0);
            let inner = NewWidget::new(inner).with_props(PropertySet::new().with(Gap::new(Length::px(size::GAP_MD))));
            InputBox::new(inner, size::INPUT_WIDTH).erased()
        }
        Control::Kbd(keys) => {
            let t = theme::current();
            let font = theme::ui_font_settings();
            let kbd = masonry::widgets::Label::new(keys.as_str())
                .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
                .with_style(StyleProperty::FontSize(size::FONT_SIZE_UI_SM as f32))
                .with_style(StyleProperty::FontWeight(masonry::parley::FontWeight::new(500.0)))
                .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::SIZE_20 as f32)));
            NewWidget::new(kbd).with_props(PropertySet::new().with(ContentColor::new(t.search_key_hint_color))).erased()
        }
    }
}
