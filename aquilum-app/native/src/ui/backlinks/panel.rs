use masonry::core::{NewWidget, PropertySet, WidgetId};
use masonry::kurbo::Axis;
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use crate::i18n::t;
use crate::links::Mode;
use crate::ui::handle::Handle;
use crate::ui::controls_width;
use crate::ui::scroll::HScroll;
use crate::ui::titlebar::DragArea;
use crate::ui::tokens::size;
use crate::ui::widgets::{Collapse, Edge, Edged, IconButton, Rule, Sash, Slot};
use crate::ui::{icons, style, theme};

pub struct Panel {
    pub modes: Vec<(Handle<IconButton>, Mode)>,
    pub body: Handle<Slot>,
    pub sash: WidgetId,
}

pub fn panel(mode: Mode, has_analysis: bool, width: f64, open: bool, from: bool) -> (NewWidget<Collapse>, Handle<Collapse>, Panel) {
    let mut buttons = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
    let mut modes = Vec::new();
    let mut entries = vec![
        (Mode::Backlinks, icons::BACKLINKS, t("backlinks.backlinksTab")),
        (Mode::Outgoing, icons::OUTGOING_LINKS, t("backlinks.outgoingTab")),
    ];
    if has_analysis {
        entries.push((Mode::Analysis, icons::GRAPH_ANALYSIS, t("analysis.tab")));
    }
    entries.push((Mode::History, icons::HISTORY, t("backlinks.historyTab")));
    for (item, icon, label) in entries {
        let button = NewWidget::new(IconButton::mode(icon, label, item == mode));
        modes.push((Handle::of(&button), item));
        buttons = buttons.with_fixed(button);
    }
    let buttons = NewWidget::new(buttons).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))));
    let strip = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(buttons).with(DragArea::new(), 1.0);
    let header = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with(HScroll::new(NewWidget::new(strip).erased()), 1.0)
        .with_fixed(NewWidget::new(SizedBox::empty().width(Length::px(controls_width()))));
    let header = NewWidget::new(SizedBox::new(NewWidget::new(header)).height(Length::px(size::TABS_HEIGHT))).with_props(
        PropertySet::new().with(Padding::from_vh(Length::px(size::SPACE_4), Length::px(size::SPACE_6))),
    ).with_class(style::TABS);
    let body = Slot::new(NewWidget::new(Flex::column()).erased());
    let body_handle = Handle::of(&body);
    let column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(header)
        .with_fixed(Rule::on(Axis::Horizontal, || theme::current().sidebar_border, || theme::current().tabs_bg))
        .with(body, 1.0);
    let column = NewWidget::new(column).with_class(style::SIDEBAR);
    let row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(Rule::on(Axis::Vertical, || theme::current().sidebar_border, || theme::current().sidebar_bg))
        .with(column, 1.0);
    let sash = Sash::new();
    let sash_id = sash.id();
    let content = Edged::new(NewWidget::new(row).erased(), sash, Edge::Left);
    let sized = Collapse::new(content.erased(), width, open, from, || theme::current().sidebar_bg);
    let handle = Handle::of(&sized);
    (sized, handle, Panel { modes, body: body_handle, sash: sash_id })
}
