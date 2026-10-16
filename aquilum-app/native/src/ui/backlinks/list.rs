use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::{heading_label, heading_row, notice};
use crate::i18n::t;
use crate::links::{Listing, Method, Mode, Target};
use crate::ui::scroll::{HScroll, ScrollArea};
use crate::ui::style;
use crate::ui::tokens::size;
use crate::ui::widgets::{Caption, Chip, DocumentItem};

pub struct Body {
    pub items: Vec<(WidgetId, Target, Vec<String>)>,
    pub chips: Vec<(WidgetId, Method)>,
}

pub fn body(mode: Mode, methods: &[Method], method: Method, listing: Option<&Listing>) -> (NewWidget<dyn Widget>, Body) {
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut chips = Vec::new();
    if mode == Mode::Analysis {
        let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
        for &m in methods {
            let chip = Chip::new(&m.label(), m == method);
            chips.push((chip.id(), m));
            row = row.with_fixed(chip);
        }
        let row = NewWidget::new(row).with_props(
            PropertySet::new()
                .with(Gap::new(Length::px(size::SPACE_6)))
                .with(Padding::all(Length::px(size::SPACE_8))),
        ).with_class(style::SIDEBAR);
        column = column.with_fixed(HScroll::new(row.erased()));
    }
    let mut items = Vec::new();
    if let Some(listing) = listing {
        let graph = mode == Mode::Analysis && method != Method::Wikixiv;
        let heading = match (mode, method) {
            (Mode::Analysis, Method::Wikixiv) => t("wikixiv.heading"),
            (Mode::Analysis, _) => t("analysis.noteColumn"),
            (Mode::Backlinks, _) => t("backlinks.mentionsTitle"),
            (Mode::Outgoing, _) => t("backlinks.outgoingTitle"),
            (Mode::History, _) => t("history.title"),
        };
        let head = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
        let head = if graph {
            head.with_fixed(heading_label(&heading)).with(NewWidget::new(Caption::trailing(&t("analysis.scoreColumn"), |t| t.text_primary.multiply_alpha(0.4))), 1.0)
        } else {
            head.with(heading_label(&heading), 1.0)
        };
        let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(heading_row(head));
        if let Some(text) = &listing.notice {
            list = list.with_fixed(notice(text));
        }
        for item in &listing.items {
            let widget = DocumentItem::new(&item.title, item.counter.clone());
            items.push((widget.id(), item.target.clone(), item.reasons.clone()));
            list = list.with_fixed(widget);
        }
        let list = NewWidget::new(list).with_props(
            PropertySet::new().with(Gap::new(Length::px(size::SPACE_2))).with(Padding::all(Length::px(size::SPACE_8))),
        );
        column = column.with(NewWidget::new(ScrollArea::new(list)), 1.0);
    }
    (NewWidget::new(column).erased(), Body { items, chips })
}
