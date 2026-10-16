use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::Axis;
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::frame::{border, on_header, on_sidebar};
use super::handle::Handle;
use super::titlebar::DragArea;
use super::tokens::size;
use super::widgets::{Collapse, Edge, Edged, IconButton, ItemButton, Rule, Sash};
use super::{icons, style, theme};
use crate::i18n;

pub(super) struct Sidebar {
    pub(super) widget: NewWidget<Collapse>,
    pub(super) handle: Handle<Collapse>,
    pub(super) workspaces: WidgetId,
    pub(super) settings: WidgetId,
    pub(super) sash: WidgetId,
}

fn panel_header(child: Option<NewWidget<impl Widget>>, padding: f64) -> NewWidget<SizedBox> {
    let header = match child {
        Some(child) => SizedBox::new(child),
        None => SizedBox::empty(),
    };
    NewWidget::new(header.height(Length::px(size::TABS_HEIGHT))).with_props(PropertySet::new().with(Padding::all(Length::px(padding)))).with_class(style::TABS)
}

pub(super) fn sidebar(tree: NewWidget<impl Widget>, workspace_name: &str, width: f64, open: bool, from: bool) -> Sidebar {
    let workspace = NewWidget::new(ItemButton::new(icons::CHEVRONS_UP_DOWN, workspace_name));
    let workspaces = workspace.id();
    let settings = NewWidget::new(IconButton::new(icons::SETTINGS, i18n::t("settings.title")));
    let settings_id = settings.id();
    let footer = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with(workspace, 1.0).with_fixed(settings);
    let column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(panel_header(Some(DragArea::new()), 0.0))
        .with_fixed(border(Axis::Horizontal, on_header))
        .with(NewWidget::new(SizedBox::new(tree)).with_class(style::SIDEBAR), 1.0)
        .with_fixed(border(Axis::Horizontal, on_sidebar))
        .with_fixed(NewWidget::new(footer).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))).with(Padding::all(Length::px(size::SPACE_6)))).with_class(style::SIDEBAR));
    let row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with(NewWidget::new(column), 1.0)
        .with_fixed(Rule::on(Axis::Vertical, || theme::current().sidebar_border, on_sidebar));
    let sash = Sash::new();
    let sash_id = sash.id();
    let content = Edged::new(NewWidget::new(row).erased(), sash, Edge::Right);
    let widget = Collapse::new(content.erased(), width, open, from, on_sidebar);
    let handle = Handle::of(&widget);
    Sidebar { widget, handle, workspaces, settings: settings_id, sash: sash_id }
}
