use masonry::core::{NewWidget, PropertySet, Widget, WidgetId};
use masonry::kurbo::Axis;
use crate::ui::editor::TextEditor;
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Background, Gap, Padding};
use masonry::parley::LineHeight;
use masonry::parley::style::StyleProperty;
use masonry::properties::{CaretColor, ContentColor, LineBreaking, SelectionColor};
use masonry::widgets::{Flex, Label, SizedBox};

use super::handle::Handle;
use super::scroll::ScrollArea;
use super::text::label;
use super::titlebar::DragArea;
use super::tokens::size;
use super::widgets::{Chip, DocumentItem, IconButton, LinkButton, MaxWidth, Rule, Slot, Wrap};
use super::{icons, theme};
use crate::i18n::t;
use crate::history::Change;
use crate::links::{Listing, Method, Mode, Target};

pub struct Panel {
    pub modes: Vec<(Handle<IconButton>, Mode)>,
    pub body: Handle<Slot>,
}

pub struct Body {
    pub items: Vec<(WidgetId, Target)>,
    pub chips: Vec<(WidgetId, Method)>,
}

pub fn panel(mode: Mode, has_analysis: bool, open: bool) -> (NewWidget<SizedBox>, Handle<SizedBox>, Panel) {
    let t = theme::current();
    let mut buttons = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
    let mut modes = Vec::new();
    let mut entries = vec![
        (Mode::Backlinks, icons::BACKLINKS, t_owned("backlinks.backlinksTab")),
        (Mode::Outgoing, icons::OUTGOING_LINKS, t_owned("backlinks.outgoingTab")),
    ];
    if has_analysis {
        entries.push((Mode::Analysis, icons::GRAPH_ANALYSIS, t_owned("analysis.tab")));
    }
    entries.push((Mode::History, icons::HISTORY, t_owned("backlinks.historyTab")));
    for (item, icon, label) in entries {
        let button = NewWidget::new(IconButton::mode(icon, label, item == mode));
        modes.push((Handle::of(&button), item));
        buttons = buttons.with_fixed(button);
    }
    let buttons = NewWidget::new(buttons).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))));
    let header = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(buttons).with(DragArea::new(), 1.0);
    let header = NewWidget::new(SizedBox::new(NewWidget::new(header)).height(Length::px(size::TABS_HEIGHT))).with_props(
        PropertySet::new().with(Background::Color(t.tabs_bg)).with(Padding::from_vh(Length::px(size::SPACE_4), Length::px(size::SPACE_6))),
    );
    let body = Slot::new(NewWidget::new(Flex::column()).erased());
    let body_handle = Handle::of(&body);
    let column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(header)
        .with_fixed(Rule::on(Axis::Horizontal, || theme::current().sidebar_border, || theme::current().tabs_bg, false))
        .with(body, 1.0);
    let column = NewWidget::new(column).with_props(PropertySet::new().with(Background::Color(t.sidebar_bg)));
    let row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(Rule::on(Axis::Vertical, || theme::current().sidebar_border, || theme::current().sidebar_bg, false))
        .with(column, 1.0);
    let width = if open { size::BACKLINKS_WIDTH } else { 0.0 };
    let sized = NewWidget::new(SizedBox::new(NewWidget::new(row)).width(Length::px(width)));
    let handle = Handle::of(&sized);
    (sized, handle, Panel { modes, body: body_handle })
}

fn t_owned(key: &str) -> String {
    t(key)
}

pub fn body(mode: Mode, methods: &[Method], method: Method, listing: Option<&Listing>) -> (NewWidget<dyn Widget>, Body) {
    let t = theme::current();
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
                .with(Padding::all(Length::px(size::SPACE_8)))
                .with(Background::Color(t.sidebar_bg)),
        );
        column = column.with_fixed(row);
    }
    let mut items = Vec::new();
    if let Some(listing) = listing {
        let graph = mode == Mode::Analysis && method != Method::Wikixiv;
        let heading = match (mode, method) {
            (Mode::Analysis, Method::Wikixiv) => t_owned("wikixiv.heading"),
            (Mode::Analysis, _) => t_owned("analysis.noteColumn"),
            (Mode::Backlinks, _) => t_owned("backlinks.mentionsTitle"),
            (Mode::Outgoing, _) => t_owned("backlinks.outgoingTitle"),
            (Mode::History, _) => t_owned("history.title"),
        };
        let line = (size::FONT_SIZE_14 * crate::ui::tokens::number::FONT_LINE_HEIGHT_TIGHT) as f32;
        let mut head = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with(label(&heading, t.text_secondary, Some(500.0), line), 1.0);
        if graph {
            head = head.with_fixed(label(&t_owned("analysis.scoreColumn"), t.text_primary.multiply_alpha(0.4), None, size::SIZE_20 as f32));
        }
        let head = NewWidget::new(head).with_props(PropertySet::new().with(Padding {
            top: Length::px(size::SPACE_8),
            bottom: Length::px(size::SPACE_4),
            left: Length::px(size::SPACE_6),
            right: Length::px(size::SPACE_6),
        }));
        let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(head);
        if let Some(notice) = &listing.notice {
            let notice = NewWidget::new(Flex::row().with(label(notice, t.text_tertiary, None, size::SIZE_20 as f32), 1.0))
                .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_6))));
            list = list.with_fixed(notice);
        }
        for item in &listing.items {
            let widget = DocumentItem::new(&item.title, item.counter.clone());
            items.push((widget.id(), item.target.clone()));
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
    let t = theme::current();
    let line = (size::FONT_SIZE_14 * crate::ui::tokens::number::FONT_LINE_HEIGHT_TIGHT) as f32;
    NewWidget::new(Flex::row().with(label(text, t.text_secondary, Some(500.0), line), 1.0)).with_props(PropertySet::new().with(Padding {
        top: Length::px(size::SPACE_8),
        bottom: Length::px(size::SPACE_4),
        left: Length::px(size::SPACE_6),
        right: Length::px(size::SPACE_6),
    }))
}

fn rename_field(name: &str) -> (NewWidget<Flex>, Handle<TextEditor>) {
    let t = theme::current();
    let font = theme::ui_font_settings();
    let field = TextEditor::single_line(name)
        .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(font.size))
        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::SIZE_20 as f32)));
    let field = NewWidget::new(field).with_props(
        PropertySet::new()
            .with(ContentColor::new(t.text_primary))
            .with(CaretColor { color: t.text_primary })
            .with(SelectionColor { color: t.selection_bg }),
    );
    let handle = Handle::of(&field);
    let row = NewWidget::new(Flex::row().with(field, 1.0)).with_props(
        PropertySet::new()
            .with(Padding::all(Length::px(size::SPACE_6)))
            .with(Background::Color(t.sidebar_item_active_bg))
            .with(masonry::properties::CornerRadius { radius: Length::px(size::SIDEBAR_ITEM_RADIUS) }),
    );
    (row, handle)
}

pub fn history_body(current_selected: bool, days: Vec<(String, Vec<(usize, HistoryRow)>)>, empty: bool, more: bool) -> (NewWidget<dyn Widget>, HistoryBody) {
    let t = theme::current();
    let current = DocumentItem::with_selection(&t_owned("history.current"), None, current_selected);
    let current_id = current.id();
    let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(panel_heading(&t_owned("history.title"))).with_fixed(current);
    if empty {
        let notice = NewWidget::new(Flex::row().with(label(&t_owned("history.empty"), t.text_tertiary, None, size::SIZE_20 as f32), 1.0))
            .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_6))));
        list = list.with_fixed(notice);
    }
    let mut rows = Vec::new();
    let mut rename = None;
    for (day, versions) in days {
        let title = NewWidget::new(Flex::row().with(label(&day, t.text_tertiary, None, size::SIZE_20 as f32), 1.0)).with_props(
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
        let button = LinkButton::new(&t_owned("history.more"), true);
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
    let t = theme::current();
    let mut buttons: Vec<NewWidget<dyn Widget>> = Vec::new();
    let mut revert = None;
    if can_revert {
        let button = LinkButton::new(&t_owned("history.revert"), true);
        revert = Some(button.id());
        buttons.push(button.erased());
    }
    let restore = LinkButton::new(&t_owned("history.restore"), can_restore);
    let back = LinkButton::new(&t_owned("history.back"), true);
    let (restore_id, back_id) = (restore.id(), back.id());
    buttons.push(restore.erased());
    buttons.push(back.erased());
    let actions = Wrap::new(buttons, 0.0);
    let font = theme::ui_font_settings();
    let title = Label::new(heading)
        .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(font.size))
        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::SIZE_20 as f32)));
    let title = NewWidget::new(title).with_props(PropertySet::new().with(ContentColor::new(t.text_secondary)).with(LineBreaking::Clip));
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
        text = text.with_fixed(label(notice, t.text_tertiary, None, size::SIZE_20 as f32));
    }
    for (change, line) in lines {
        let shown = if line.is_empty() { " " } else { line.as_str() };
        let widget = Label::new(shown)
            .with_style(StyleProperty::FontSize(16.0))
            .with_style(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.6)));
        let mut props = PropertySet::new().with(ContentColor::new(t.editor_text)).with(LineBreaking::WordWrap);
        match change {
            Change::Added => props = props.with(Background::Color(t.bg_success_subtle)),
            Change::Removed => props = props.with(Background::Color(t.bg_danger_subtle)),
            Change::Same => {}
        }
        text = text.with_fixed(NewWidget::new(widget).with_props(props));
    }
    let column = MaxWidth::new(NewWidget::new(text), 760.0);
    let content = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(column))
        .with_props(PropertySet::new().with(Padding::all(Length::px(32.0))));
    let view = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(bar)
        .with_fixed(Rule::new(Axis::Horizontal, || theme::current().sidebar_border))
        .with(NewWidget::new(ScrollArea::new(content)), 1.0);
    let view = NewWidget::new(view).with_props(PropertySet::new().with(Background::Color(t.editor_bg)));
    (view.erased(), VersionView { revert, restore: restore_id, back: back_id })
}
