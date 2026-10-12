use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::Axis;
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::frame::{border, on_header, on_sidebar};
use super::handle::Handle;
use super::tokens::size;
use super::widgets::{IconButton, Slot, Visible};
use super::{icons, style};
use crate::i18n;

pub(super) struct Rail {
    pub(super) widget: NewWidget<SizedBox>,
    pub(super) toggle_sidebar: WidgetId,
    pub(super) settings: WidgetId,
    pub(super) home_slot: Handle<Slot>,
    pub(super) home_button: Option<WidgetId>,
    pub(super) graph_button: Option<WidgetId>,
    pub(super) focus_hidden: Vec<Handle<Visible>>,
    pub(super) focus_shown: Handle<Visible>,
}

pub fn home_widget(show: bool) -> (NewWidget<dyn Widget>, Option<WidgetId>) {
    if !show {
        return (NewWidget::new(SizedBox::empty().height(Length::ZERO)).erased(), None);
    }
    let button = NewWidget::new(IconButton::new(icons::HOUSE, i18n::t("rail.home")));
    let id = button.id();
    (button.erased(), Some(id))
}

pub fn sidebar_label(open: bool) -> &'static str {
    if open { "rail.hideSidebar" } else { "rail.showSidebar" }
}

pub(super) fn rail(sidebar_open: bool, home: bool, focus_mode: bool) -> Rail {
    let toggle = NewWidget::new(IconButton::new(icons::PANEL_LEFT, i18n::t(sidebar_label(sidebar_open))));
    let (home, home_button) = home_widget(home && !focus_mode);
    let home = Slot::fit(home);
    let home_slot = Handle::of(&home);
    let rail_settings = NewWidget::new(IconButton::new(icons::SETTINGS, i18n::t("common.settings")));
    let toggle_sidebar = toggle.id();
    let settings = rail_settings.id();
    let graph_rail = NewWidget::new(IconButton::new(icons::NETWORK, i18n::t("rail.graph")));
    let graph_button = Some(graph_rail.id());
    let graph_rail = Visible::new(graph_rail, !focus_mode);
    let rail_settings = Visible::new(rail_settings, focus_mode);
    let toggle = Visible::new(toggle, !focus_mode);
    let focus_hidden = vec![Handle::of(&graph_rail), Handle::of(&toggle)];
    let focus_shown = Handle::of(&rail_settings);
    let tools = Flex::column().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(home).with_fixed(graph_rail).with_spacer(1.0).with_fixed(rail_settings);
    let header = SizedBox::new(toggle);
    let header = NewWidget::new(header.height(Length::px(size::TABS_HEIGHT))).with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_4)))).with_class(style::TABS);
    let tools = NewWidget::new(tools).with_props(
        PropertySet::new()
            .with(Gap::new(Length::px(size::SPACE_4)))
            .with(Padding::from_vh(Length::px(size::SPACE_8), Length::px(size::SPACE_4))),
    );
    let tools = Flex::row().cross_axis_alignment(CrossAxisAlignment::Stretch).with(tools, 1.0).with_fixed(border(Axis::Vertical, on_sidebar));
    let rail = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(header)
        .with_fixed(border(Axis::Horizontal, on_header))
        .with(NewWidget::new(tools), 1.0);
    let rail = NewWidget::new(rail).with_class(style::SIDEBAR);
    let widget = NewWidget::new(SizedBox::new(rail).width(Length::px(size::SIDEBAR_RAIL_WIDTH)));
    Rail { widget, toggle_sidebar, settings, home_slot, home_button, graph_button, focus_hidden, focus_shown }
}
