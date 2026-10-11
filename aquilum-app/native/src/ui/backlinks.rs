use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::Axis;
use crate::ui::editor::TextEditor;
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::{Flex, SizedBox};

use super::handle::Handle;
use super::scroll::ScrollArea;
use super::style;
use super::text::label;
use super::titlebar::DragArea;
use super::tokens::size;
use super::widgets::{Chip, Collapse, DocumentItem, Font, IconButton, Label, Leading, LinkButton, MaxWidth, Rule, Slot, Wrap};
use super::{icons, theme};
use crate::i18n::t;
use crate::history::Change;
use crate::links::{Listing, Method, Mode, Target};

pub struct Panel {
    pub modes: Vec<(Handle<IconButton>, Mode)>,
    pub body: Handle<Slot>,
}

pub struct Body {
    pub items: Vec<(WidgetId, Target, Vec<String>)>,
    pub chips: Vec<(WidgetId, Method)>,
}

pub fn panel(mode: Mode, has_analysis: bool, open: bool, from: bool) -> (NewWidget<Collapse>, Handle<Collapse>, Panel) {
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
    let header = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(buttons).with(DragArea::new(), 1.0);
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
    let sized = Collapse::new(NewWidget::new(row).erased(), size::BACKLINKS_WIDTH, open, from, || theme::current().sidebar_bg);
    let handle = Handle::of(&sized);
    (sized, handle, Panel { modes, body: body_handle })
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
        column = column.with_fixed(row);
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
        let line = (size::FONT_SIZE_14 * crate::ui::tokens::number::FONT_LINE_HEIGHT_TIGHT) as f32;
        let mut head = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with(label(&heading, |t| t.text_secondary, Some(500.0), Leading::Px(line)), 1.0);
        if graph {
            head = head.with_fixed(label(&t("analysis.scoreColumn"), |t| t.text_primary.multiply_alpha(0.4), None, Leading::Px(size::SIZE_20 as f32)));
        }
        let head = NewWidget::new(head).with_props(PropertySet::new().with(Padding {
            top: Length::px(size::SPACE_8),
            bottom: Length::px(size::SPACE_4),
            left: Length::px(size::SPACE_6),
            right: Length::px(size::SPACE_6),
        }));
        let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(head);
        if let Some(notice) = &listing.notice {
            let notice = NewWidget::new(Flex::row().with(label(notice, |t| t.text_tertiary, None, Leading::Px(size::SIZE_20 as f32)), 1.0))
                .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_6))));
            list = list.with_fixed(notice);
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

fn panel_heading(text: &str) -> NewWidget<Flex> {
    let line = (size::FONT_SIZE_14 * crate::ui::tokens::number::FONT_LINE_HEIGHT_TIGHT) as f32;
    NewWidget::new(Flex::row().with(label(text, |t| t.text_secondary, Some(500.0), Leading::Px(line)), 1.0)).with_props(PropertySet::new().with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_4),
        left: Length::px(size::SPACE_6),
        right: Length::px(size::SPACE_6),
    }))
}

fn rename_field(name: &str) -> (NewWidget<Flex>, Handle<TextEditor>) {
    let field = NewWidget::new(TextEditor::single_line(name).styled(|| style::ui_text(size::SIZE_20 as f32))).with_class(style::FIELD);
    let handle = Handle::of(&field);
    let row = NewWidget::new(Flex::row().with(field, 1.0)).with_props(
        PropertySet::new()
            .with(Padding::all(Length::px(size::SPACE_6)))
            .with(masonry::properties::CornerRadius { radius: Length::px(size::SIDEBAR_ITEM_RADIUS) }),
    ).with_class(style::ACTIVE_ITEM);
    (row, handle)
}

pub fn history_body(current_selected: bool, days: Vec<(String, Vec<(usize, HistoryRow)>)>, empty: bool, more: bool) -> (NewWidget<dyn Widget>, HistoryBody) {
    let current = DocumentItem::with_selection(&t("history.current"), None, current_selected);
    let current_id = current.id();
    let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(panel_heading(&t("history.title"))).with_fixed(current);
    if empty {
        let notice = NewWidget::new(Flex::row().with(label(&t("history.empty"), |t| t.text_tertiary, None, Leading::Px(size::SIZE_20 as f32)), 1.0))
            .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_6))));
        list = list.with_fixed(notice);
    }
    let mut rows = Vec::new();
    let mut rename = None;
    for (day, versions) in days {
        let title = NewWidget::new(Flex::row().with(label(&day, |t| t.text_tertiary, None, Leading::Px(size::SIZE_20 as f32)), 1.0)).with_props(
            PropertySet::new().with(Padding {
                top: Length::px(size::SPACE_8),
                bottom: Length::px(size::SPACE_2),
                left: Length::px(size::SPACE_6),
                right: Length::px(size::SPACE_6),
            }),
        );
        list = list.with_fixed(title);
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

pub struct VersionView {
    pub revert: Option<WidgetId>,
    pub restore: WidgetId,
    pub back: WidgetId,
}

pub fn version_view(heading: &str, can_revert: bool, can_restore: bool, lines: &[(Change, String)], notice: Option<&str>) -> (NewWidget<dyn Widget>, VersionView) {
    let mut buttons: Vec<NewWidget<dyn Widget>> = Vec::new();
    let mut revert = None;
    if can_revert {
        let button = LinkButton::new(&t("history.revert"));
        revert = Some(button.id());
        buttons.push(button.erased());
    }
    let restore = LinkButton::new(&t("history.restore")).disabled(!can_restore);
    let back = LinkButton::new(&t("history.back"));
    let (restore_id, back_id) = (restore.id(), back.id());
    buttons.push(restore.erased());
    buttons.push(back.erased());
    let actions = Wrap::new(buttons, 0.0);
    let title = NewWidget::new(Label::new(heading, |t| t.text_secondary).nowrap());
    let title = NewWidget::new(Flex::row().with(title, 1.0)).with_props(PropertySet::new().with(Padding::from_vh(Length::ZERO, Length::px(size::SPACE_12))));
    let bar = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(title).with_fixed(actions);
    let bar = NewWidget::new(bar).with_props(PropertySet::new().with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_2),
        left: Length::px(32.0 - size::SPACE_12),
        right: Length::px(32.0 - size::SPACE_12),
    }));
    let mut text = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    if let Some(notice) = notice {
        text = text.with_fixed(label(notice, |t| t.text_tertiary, None, Leading::Px(size::SIZE_20 as f32)));
    }
    for (change, line) in lines {
        let shown = if line.is_empty() { " " } else { line.as_str() };
        let widget = Label::new(shown, |t| t.editor_text).font(Font::Editor).leading(Leading::Editor);
        let widget = match change {
            Change::Added => widget.background(|t| t.bg_success_subtle),
            Change::Removed => widget.background(|t| t.bg_danger_subtle),
            Change::Same => widget,
        };
        text = text.with_fixed(NewWidget::new(widget));
    }
    let column = MaxWidth::new(NewWidget::new(text), 760.0);
    let content = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(column))
        .with_props(PropertySet::new().with(Padding::all(Length::px(32.0))));
    let view = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(bar)
        .with_fixed(Rule::new(Axis::Horizontal, || theme::current().sidebar_border))
        .with(NewWidget::new(ScrollArea::new(content)), 1.0);
    let view = NewWidget::new(view).with_class(style::EDITOR);
    (view.erased(), VersionView { revert, restore: restore_id, back: back_id })
}
