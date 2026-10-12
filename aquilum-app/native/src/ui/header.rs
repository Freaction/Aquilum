use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::Padding;
use masonry::widgets::{Flex, SizedBox};

use super::frame::on_header;
use super::handle::Handle;
use super::titlebar::TabStrip;
use super::tokens::size;
use super::widgets::{Collapse, IconButton, Underlined, Visible};
use super::{icons, theme};
use crate::i18n;

pub(super) struct Titlebar {
    pub(super) widget: NewWidget<Flex>,
    pub(super) new_tab: WidgetId,
    pub(super) strip: Handle<TabStrip>,
    pub(super) toggle: WidgetId,
    pub(super) toggle_shown: Handle<Visible>,
    pub(super) inset: Handle<Collapse>,
}

pub fn panel_label(open: bool) -> &'static str {
    if open { "titlebar.hideRightSidebar" } else { "titlebar.showRightSidebar" }
}

fn controls_inset(panel_open: bool, panel_from: bool) -> NewWidget<Collapse> {
    let width = if cfg!(target_os = "macos") { 0.0 } else { size::TITLEBAR_CONTROLS_INSET };
    Collapse::new(NewWidget::new(SizedBox::empty()).erased(), width, !panel_open, !panel_from, on_header)
}

pub(super) fn titlebar(tabs: Vec<NewWidget<dyn Widget>>, panel_open: bool, panel_from: bool, focus_mode: bool) -> Titlebar {
    let strip = TabStrip::new(tabs);
    let strip_handle = Handle::of(&strip);
    let plus = NewWidget::new(IconButton::new(icons::PLUS, i18n::t("tabs.newTab")));
    let plus_id = plus.id();
    let plus = NewWidget::new(Flex::row().with_fixed(plus)).with_props(PropertySet::new().with(Padding {
        top: Length::ZERO,
        bottom: Length::ZERO,
        left: Length::px(size::SPACE_6),
        right: Length::px(size::SPACE_6),
    }));
    let toggle = NewWidget::new(IconButton::new(icons::PANEL_RIGHT, i18n::t(panel_label(panel_open))));
    let toggle_id = toggle.id();
    let toggle = Visible::new(toggle, !focus_mode);
    let toggle_shown = Handle::of(&toggle);
    let menu = NewWidget::new(Flex::row().with_fixed(toggle))
        .with_props(PropertySet::new().with(Padding::from_vh(Length::px(size::SPACE_4), Length::px(size::SPACE_6))));
    let inset = controls_inset(panel_open, panel_from);
    let inset_handle = Handle::of(&inset);
    let row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with(strip, 1.0)
        .with_fixed(plus)
        .with_fixed(menu)
        .with_fixed(inset);
    let row = NewWidget::new(SizedBox::new(NewWidget::new(row)).height(Length::px(size::TABS_HEIGHT)));
    let row = Underlined::new(row, on_header, || theme::current().tabs_border);
    let widget = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(row));
    Titlebar { widget, new_tab: plus_id, strip: strip_handle, toggle: toggle_id, toggle_shown, inset: inset_handle }
}
