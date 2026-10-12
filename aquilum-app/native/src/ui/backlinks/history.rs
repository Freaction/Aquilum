use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{CornerRadius, Gap, Padding};
use masonry::widgets::Flex;

use super::{heading_label, heading_row, muted, notice};
use crate::i18n::t;
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::scroll::ScrollArea;
use crate::ui::style;
use crate::ui::tokens::size;
use crate::ui::widgets::{DocumentItem, LinkButton};

pub struct HistoryRow {
    pub label: String,
    pub time: String,
    pub selected: bool,
    pub renaming: bool,
}

pub struct HistoryBody {
    pub current: WidgetId,
    pub rows: Vec<(WidgetId, usize)>,
    pub more: Option<WidgetId>,
    pub rename: Option<Handle<TextEditor>>,
}

fn rename_field(name: &str) -> (NewWidget<Flex>, Handle<TextEditor>) {
    let field = NewWidget::new(TextEditor::single_line(name).styled(|| style::ui_text(size::SIZE_20 as f32))).with_class(style::FIELD);
    let handle = Handle::of(&field);
    let row = NewWidget::new(Flex::row().with(field, 1.0)).with_props(
        PropertySet::new()
            .with(Padding::all(Length::px(size::SPACE_6)))
            .with(CornerRadius { radius: Length::px(size::SIDEBAR_ITEM_RADIUS) }),
    ).with_class(style::ACTIVE_ITEM);
    (row, handle)
}

fn day_title(day: &str) -> NewWidget<Flex> {
    NewWidget::new(Flex::row().with(muted(day), 1.0)).with_props(PropertySet::new().with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_2),
        left: Length::px(size::SPACE_6),
        right: Length::px(size::SPACE_6),
    }))
}

pub fn history_body(current_selected: bool, days: Vec<(String, Vec<(usize, HistoryRow)>)>, empty: bool, more: bool) -> (NewWidget<dyn Widget>, HistoryBody) {
    let current = DocumentItem::with_selection(&t("history.current"), None, current_selected);
    let current_id = current.id();
    let heading = heading_row(Flex::row().with(heading_label(&t("history.title")), 1.0));
    let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(heading).with_fixed(current);
    if empty {
        list = list.with_fixed(notice(&t("history.empty")));
    }
    let mut rows = Vec::new();
    let mut rename = None;
    for (day, versions) in days {
        list = list.with_fixed(day_title(&day));
        for (index, row) in versions {
            if row.renaming {
                let (field, handle) = rename_field(&row.label);
                rename = Some(handle);
                list = list.with_fixed(field);
                continue;
            }
            let item = DocumentItem::with_selection(&row.label, Some(row.time), row.selected);
            rows.push((item.id(), index));
            list = list.with_fixed(item);
        }
    }
    let mut more_id = None;
    if more {
        let button = LinkButton::new(&t("history.more"));
        more_id = Some(button.id());
        list = list.with_fixed(NewWidget::new(Flex::row().with_fixed(button)));
    }
    let list = NewWidget::new(list).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::SPACE_2))).with(Padding::all(Length::px(size::SPACE_8))),
    );
    let body = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with(NewWidget::new(ScrollArea::new(list)), 1.0);
    (NewWidget::new(body).erased(), HistoryBody { current: current_id, rows, more: more_id, rename })
}
