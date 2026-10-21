pub mod controls;
pub mod schema;

mod control;
mod layout;

use std::collections::HashMap;

use aquilum_core::settings::models::AppConfig;
use masonry::accesskit::Role;
use masonry::core::{NewWidget, PropertySet, WidgetId};
use masonry::kurbo::{Axis, Size};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use crate::i18n::t;
use crate::ui::dialog::{CardSize, DialogIds, Modal, header};
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::scroll::ScrollArea;
use crate::ui::tokens::size;
use crate::ui::widgets::{IconButton, ItemButton, Rule};
use crate::ui::{icons, theme};
use layout::{block_widget, rows_column};
use schema::{Action, Context, Number, Row, Section, Setter};

pub enum Binding {
    Switch(Setter),
    Choice(Setter),
    Dropdown { setter: Setter, options: Vec<String>, selected: usize },
    Number { setter: Setter, number: Number, minus: Option<WidgetId>, plus: Option<WidgetId> },
    Step { field: WidgetId, direction: f64 },
    Text { setter: Setter, commit: bool, value: String },
    Color(Setter),
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
    pub confirm: Option<(DialogIds, schema::Wipe)>,
}

fn card_size(room: Size) -> Size {
    let window = Size::new(room.width + size::SPACE_32, room.height + size::SPACE_32);
    Size::new(
        room.width.min(window.width * 0.9).min(size::SETTINGS_MAX_WIDTH),
        (window.height * 0.85).min(size::SETTINGS_MAX_HEIGHT).min(room.height),
    )
}

pub fn dialog(config: &AppConfig, ctx: &Context, section: Section) -> (NewWidget<Modal>, SettingsDialog) {
    let (card, finish) = card(config, ctx, section);
    let layer = Modal::new(card, &t("settings.title"), CardSize::Window(card_size), Role::Dialog, false);
    let dialog = finish(layer.id());
    (layer, dialog)
}

pub fn card(config: &AppConfig, ctx: &Context, section: Section) -> (NewWidget<Flex>, impl FnOnce(WidgetId) -> SettingsDialog) {
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
    let (copied, trash_page) = (ctx.copied, ctx.trash_page);
    let finish = move |layer| SettingsDialog { layer, close: close_id, content: content_handle, nav: nav_ids, section, bindings, copied, trash_page, confirm: None };
    (NewWidget::new(card), finish)
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
    let column = rows_column(rows, &mut bindings);
    (NewWidget::new(column), bindings)
}
