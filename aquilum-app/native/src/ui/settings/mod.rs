pub mod controls;
pub mod schema;

mod control;
mod layout;
mod panes;
mod row;

use std::collections::HashMap;

use aquilum_core::settings::models::AppConfig;
use masonry::accesskit::Role;
use masonry::app::RenderRoot;
use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::{Axis, Size};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use crate::i18n::t;
use crate::interface::{Screen, WidthClass};
use crate::ui::dialog::{CardSize, DialogIds, Modal, header};
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::scroll::ScrollArea;
use crate::ui::text::label;
use crate::ui::tokens::size;
use crate::ui::widgets::{IconButton, ItemButton, Label, Leading, Rule};
use crate::ui::{icons, theme};
use layout::{block_widget, rows_column};
pub use panes::{Pane, Panes};
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
    pub close_sub: WidgetId,
    pub back: WidgetId,
    pub title: Handle<Label>,
    pub panes: Handle<Panes>,
    pub list: Handle<ScrollArea<Flex>>,
    pub content: Handle<ScrollArea<Flex>>,
    pub nav: Vec<(WidgetId, Section)>,
    pub section: Section,
    pub listing: bool,
    pub screen: Screen,
    pub bindings: HashMap<WidgetId, Binding>,
    pub copied: Option<usize>,
    pub trash_page: usize,
    pub confirm: Option<(DialogIds, schema::Wipe)>,
}

impl SettingsDialog {
    pub fn pane(&self) -> Pane {
        Pane::of(self.screen.compact, self.listing)
    }

    pub fn fit(&mut self, root: &mut RenderRoot, screen: Screen) {
        if screen.compact != self.screen.compact {
            let (column, ids) = nav(self.section, screen.compact);
            self.list.edit(root, |mut area| ScrollArea::set_child(&mut area, column));
            self.nav = ids;
        }
        self.screen = screen;
        self.show_list(root, self.listing);
    }

    pub fn is_close(&self, id: WidgetId) -> bool {
        id == self.close || id == self.close_sub
    }

    pub fn show_list(&mut self, root: &mut RenderRoot, listing: bool) {
        self.listing = listing;
        let (pane, insets) = (self.pane(), self.screen.insets);
        self.panes.edit(root, |mut panes| Panes::set(&mut panes, pane, insets));
    }
}

fn card_size(room: Size) -> Size {
    let window = Size::new(room.width + size::SPACE_32, room.height + size::SPACE_32);
    if WidthClass::of(window.width) == WidthClass::Compact {
        return window;
    }
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

fn nav(section: Section, compact: bool) -> (NewWidget<Flex>, Vec<(WidgetId, Section)>) {
    let height = Length::px(if compact { size::SIZE_48 } else { size::SIZE_32 });
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut ids = Vec::new();
    for item in Section::ALL {
        let button = NewWidget::new(ItemButton::new(item.icon(), item.label()).active(item == section));
        ids.push((button.id(), item));
        column = column.with_fixed(NewWidget::new(SizedBox::new(button).height(height)));
    }
    let column = NewWidget::new(column).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::SPACE_2))).with(Padding::all(Length::px(size::SPACE_8))),
    );
    (column, ids)
}

fn ruled(head: NewWidget<impl Widget>) -> NewWidget<Flex> {
    let rule = Rule::new(Axis::Horizontal, || theme::current().border_default);
    NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(head).with_fixed(rule))
}

pub fn card(config: &AppConfig, ctx: &Context, section: Section) -> (NewWidget<Flex>, impl FnOnce(WidgetId) -> SettingsDialog) {
    let close = NewWidget::new(IconButton::new(icons::X, t("settings.close")));
    let close_sub = NewWidget::new(IconButton::new(icons::X, t("settings.close")));
    let back = NewWidget::new(IconButton::new(icons::ARROW_LEFT, t("settings.back")));
    let title = label(&section.label(), |t| t.text_secondary, None, Leading::Px(size::SIZE_20 as f32));
    let (close_id, close_sub_id, back_id, title_handle) = (close.id(), close_sub.id(), back.id(), Handle::of(&title));
    let sub = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(back).with(title, 1.0).with_fixed(close_sub);
    let sub = NewWidget::new(sub).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::GAP_LG))).with(Padding::from_vh(Length::px(size::GAP_MD), Length::px(size::GAP_LG))),
    );

    let (column, nav_ids) = nav(section, false);
    let list = NewWidget::new(ScrollArea::new(column));
    let (blocks, bindings) = content(config, ctx, section);
    let content = NewWidget::new(ScrollArea::new(blocks));
    let (list_handle, content_handle) = (Handle::of(&list), Handle::of(&content));
    let panes = Panes::new(ruled(header(&t("settings.title"), Some(close))), ruled(sub), list, content);
    let panes_handle = Handle::of(&panes);

    let card = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with(panes, 1.0);
    let (copied, trash_page) = (ctx.copied, ctx.trash_page);
    let finish = move |layer| SettingsDialog {
        layer,
        close: close_id,
        close_sub: close_sub_id,
        back: back_id,
        title: title_handle,
        panes: panes_handle,
        list: list_handle,
        content: content_handle,
        nav: nav_ids,
        section,
        listing: true,
        screen: Screen::default(),
        bindings,
        copied,
        trash_page,
        confirm: None,
    };
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
