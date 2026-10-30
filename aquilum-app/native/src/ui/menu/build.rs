use masonry::core::{NewWidget, PropertySet, WidgetId};
use masonry::kurbo::{Axis, Rect, Size};
use masonry::layout::Length;
use masonry::properties::Padding;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::item::{Kind, MenuItem};
use super::sub::{Sub, Submenu, panel};
use super::{Entry, Menu};
use crate::ui::scroll::ScrollArea;
use crate::ui::theme;
use crate::ui::tokens::size;
use crate::ui::widgets::Rule;

const DIVIDER: f64 = size::BORDER_1 + 2.0 * size::SPACE_4;

impl Menu {
    pub const LIST_MAX_HEIGHT: f64 = 256.0;

    pub fn new(label: &str, items: &[&str]) -> NewWidget<Self> {
        Self::build(label, &plain_entries(items), None, None, None)
    }

    pub fn entries(label: &str, entries: &[Entry<'_>]) -> NewWidget<Self> {
        Self::build(label, entries, None, None, None)
    }

    pub fn with_submenu(label: &str, entries: &[Entry<'_>], sub: &Submenu<'_>) -> NewWidget<Self> {
        Self::build(label, entries, None, None, Some(sub))
    }

    pub fn list(label: &str, items: &[&str], checked: usize, trigger: Rect) -> NewWidget<Self> {
        let mut menu = Self::build(label, &plain_entries(items), Some(checked), Some(trigger.width()), None);
        menu.widget.max_height = Self::LIST_MAX_HEIGHT;
        menu.widget.exclude = Some(trigger);
        menu
    }

    pub fn holding(mut menu: NewWidget<Self>, focused: WidgetId) -> NewWidget<Self> {
        menu.widget.holds = Some(focused);
        menu
    }

    pub fn dropdown(label: &str, items: &[&str], checked: usize, trigger: Rect, window: Size, width: f64) -> NewWidget<Self> {
        let mut menu = Self::list(label, items, checked, trigger);
        menu.widget.width = Some(width);
        menu.widget.max_width = width;
        menu.widget.max_height = (window.height - Self::dropdown_origin(trigger, window, width).y - size::SPACE_8).max(MenuItem::height());
        menu
    }

    fn build(label: &str, items: &[Entry<'_>], checked: Option<usize>, width: Option<f64>, sub: Option<&Submenu<'_>>) -> NewWidget<Self> {
        let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, item) in items.iter().enumerate() {
            if item.divided && index > 0 {
                let rule = Rule::new(Axis::Horizontal, || theme::current().menu_divider);
                column = column.with_fixed(rule.with_props(PropertySet::new().with(Padding::vertical(Length::px(size::SPACE_4)))));
            }
            let kind = match sub {
                Some(sub) if sub.at == index => Kind::Parent,
                Some(_) => Kind::Watched,
                None => Kind::Plain,
            };
            column = column.with_fixed(NewWidget::new(MenuItem::new(index, item.label, item.icon, item.disabled, checked == Some(index), checked.is_some(), kind)));
        }
        let scroll = NewWidget::new(ScrollArea::new(panel(column)));
        NewWidget::new(Menu {
            child: scroll.to_pod(),
            label: label.to_owned(),
            width,
            max_width: f64::INFINITY,
            max_height: f64::INFINITY,
            exclude: None,
            sub: sub.map(|sub| {
                let dividers = items.iter().take(sub.at + 1).skip(1).filter(|item| item.divided).count();
                Sub::new(sub, sub.at as f64 * (MenuItem::height() + size::GAP_XS) + dividers as f64 * (DIVIDER + size::GAP_XS))
            }),
            main: Size::ZERO,
            holds: None,
        })
    }
}

fn plain_entries<'a>(items: &[&'a str]) -> Vec<Entry<'a>> {
    items.iter().map(|&label| Entry { label, icon: None, disabled: false, divided: false }).collect()
}
