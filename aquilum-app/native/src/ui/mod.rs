//! Дерево виджетов окна по эталону `src/App.tsx`: узкая панель инструментов
//! (`SidebarRail`), боковая панель с деревом файлов (`Sidebar`) и основная колонка с полосой
//! вкладок и текстом заметки. Эталон — `aquilum-app/src/` (только чтение).

pub mod backlinks;
pub mod book;
pub mod dialog;
pub mod file_tree;
pub mod handle;
pub mod menu;
pub mod note;
pub mod editor;
pub mod scroll;
pub mod update;
pub mod search;
pub mod find;
pub mod settings;
pub mod style;
pub mod icons;
pub mod text;
pub mod theme;
pub mod titlebar;
pub mod timer;
pub mod graph;
pub mod reader;
pub mod suggest;
pub mod viewer;
pub mod tokens;
pub mod widgets;
pub mod workspaces;

use masonry::core::{NewWidget, PropertySet, Widget, WidgetId, WidgetTag};
use masonry::kurbo::Axis;
use masonry::layout::Length;
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::properties::types::MainAxisAlignment;
use masonry::widgets::{Flex, IndexedStack, SizedBox};

use crate::i18n;
use tokens::size;
use handle::Handle;
use titlebar::{DragArea, TabStrip};
use widgets::{IconButton, ItemButton, LinkButton, Rule, Underlined};

/// Гнездо всего интерфейса: в него кладётся пересобранный интерфейс. Метка — только у него:
/// он создаётся один раз, а метки Masonry не освобождает (см. `handle`).
pub const ROOT_TAG: WidgetTag<widgets::Slot> = WidgetTag::named("root");


/// Части окна, к которым обращается приложение: кнопки оформления (по ним окно узнаёт, что
/// нажато), боковая панель и текст заметки. Главная, граф и переключатель баз добавятся сюда
/// вместе с их экранами.
pub struct Chrome {
    pub toggle_sidebar: WidgetId,
    pub settings: WidgetId,
    pub new_tab: WidgetId,
    pub tab_strip: Handle<TabStrip>,
    pub panel_toggle: WidgetId,
    pub controls_inset: Handle<widgets::Collapse>,
    pub panel: Handle<widgets::Collapse>,
    pub panel_parts: backlinks::Panel,
    pub pages: Handle<IndexedStack>,
    pub version_page: Handle<widgets::Slot>,
    pub create_note: WidgetId,
    pub open_file: WidgetId,
    pub close_tab: WidgetId,
    pub workspaces: WidgetId,
    pub open_vault: WidgetId,
    pub sidebar: Handle<widgets::Collapse>,
    pub note_page: Handle<widgets::Slot>,
    pub rail_settings: WidgetId,
    pub home_slot: Handle<widgets::Slot>,
    pub home_button: Option<WidgetId>,
    pub graph_page: Handle<widgets::Slot>,
    pub graph_button: Option<WidgetId>,
    pub focus_hidden: Vec<Handle<widgets::Visible>>,
    pub focus_shown: Handle<widgets::Visible>,
}

pub fn rescale(root: &mut masonry::app::RenderRoot, scale: f64) {
    let size = root.size();
    root.handle_window_event(masonry::core::WindowEvent::Rescale(scale));
    root.handle_window_event(masonry::core::WindowEvent::Resize(size));
}


fn border(axis: Axis, base: fn() -> Color) -> NewWidget<Rule> {
    Rule::on(axis, || theme::current().sidebar_border, base)
}

fn on_header() -> Color {
    theme::current().tabs_bg
}

fn on_sidebar() -> Color {
    theme::current().sidebar_bg
}

/// `.q-panel-header`: шапка панели высотой с полосу вкладок.
fn panel_header(child: Option<NewWidget<impl Widget>>, padding: f64) -> NewWidget<SizedBox> {
    let header = match child {
        Some(child) => SizedBox::new(child),
        None => SizedBox::empty(),
    };
    NewWidget::new(header.height(Length::px(size::TABS_HEIGHT))).with_props(PropertySet::new().with(Padding::all(Length::px(padding)))).with_class(style::TABS)
}

pub struct RootParts<'a> {
    pub workspace_name: &'a str,
    pub note: NewWidget<dyn Widget>,
    pub sidebar_open: bool,
    pub sidebar_from: bool,
    pub tabs: Vec<NewWidget<dyn Widget>>,
    pub empty_tab: bool,
    pub vault_open: bool,
    pub vault_error: Option<&'a str>,
    pub panel_open: bool,
    pub panel_from: bool,
    pub panel_mode: crate::links::Mode,
    pub has_analysis: bool,
    pub focus_mode: bool,
    pub home: bool,
    pub graph: Option<NewWidget<dyn Widget>>,
    pub graph_active: bool,
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

pub fn panel_label(open: bool) -> &'static str {
    if open { "titlebar.hideRightSidebar" } else { "titlebar.showRightSidebar" }
}

fn controls_inset(panel_open: bool, panel_from: bool) -> NewWidget<widgets::Collapse> {
    let width = if cfg!(target_os = "macos") { 0.0 } else { size::TITLEBAR_CONTROLS_INSET };
    widgets::Collapse::new(NewWidget::new(SizedBox::empty()).erased(), width, !panel_open, !panel_from, on_header)
}

struct Titlebar {
    widget: NewWidget<Flex>,
    new_tab: WidgetId,
    strip: Handle<TabStrip>,
    toggle: WidgetId,
    toggle_shown: Handle<widgets::Visible>,
    inset: Handle<widgets::Collapse>,
}

fn titlebar(tabs: Vec<NewWidget<dyn Widget>>, panel_open: bool, panel_from: bool, focus_mode: bool) -> Titlebar {
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
    let toggle = widgets::Visible::new(toggle, !focus_mode);
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

fn new_tab_view() -> (NewWidget<Flex>, WidgetId, WidgetId, WidgetId) {
    let create = LinkButton::new(&i18n::t("editor.createNote"));
    let open = LinkButton::new(&i18n::t("editor.openFile"));
    let close = LinkButton::new(&i18n::t("editor.close"));
    let (create_id, open_id, close_id) = (create.id(), open.id(), close.id());
    let column = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(create)
        .with_fixed(open)
        .with_fixed(close);
    let column = NewWidget::new(column).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))));
    (column, create_id, open_id, close_id)
}

pub fn root(tree: NewWidget<impl Widget>, parts: RootParts<'_>) -> (NewWidget<impl Widget>, Chrome) {
    let RootParts { workspace_name, note, sidebar_open, sidebar_from, tabs, empty_tab, vault_open, vault_error, panel_open, panel_from, panel_mode, has_analysis, focus_mode, home, graph, graph_active } = parts;

    let toggle = NewWidget::new(IconButton::new(icons::PANEL_LEFT, i18n::t(sidebar_label(sidebar_open))));
    let (home, home_button) = home_widget(home && !focus_mode);
    let home = widgets::Slot::fit(home);
    let home_slot = Handle::of(&home);
    let rail_settings = NewWidget::new(IconButton::new(icons::SETTINGS, i18n::t("common.settings")));
    let toggle_sidebar = toggle.id();
    let rail_settings_id = rail_settings.id();
    let graph_rail = NewWidget::new(IconButton::new(icons::NETWORK, i18n::t("rail.graph")));
    let graph_button = Some(graph_rail.id());
    let graph_rail = widgets::Visible::new(graph_rail, !focus_mode);
    let rail_settings = widgets::Visible::new(rail_settings, focus_mode);
    let toggle = widgets::Visible::new(toggle, !focus_mode);
    let mut focus_hidden = vec![Handle::of(&graph_rail), Handle::of(&toggle)];
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
    let rail = NewWidget::new(SizedBox::new(rail).width(Length::px(size::SIDEBAR_RAIL_WIDTH)));

    // Sidebar: пустая шапка, дерево, подвал с переключателем базы и настройками.
    let workspace = NewWidget::new(ItemButton::new(icons::CHEVRONS_UP_DOWN, workspace_name));
    let workspaces = workspace.id();
    let settings = NewWidget::new(IconButton::new(icons::SETTINGS, i18n::t("settings.title")));
    let settings_id = settings.id();
    let footer = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with(workspace, 1.0).with_fixed(settings);
    let sidebar = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(panel_header(Some(DragArea::new()), 0.0))
        .with_fixed(border(Axis::Horizontal, on_header))
        .with(NewWidget::new(SizedBox::new(tree)).with_class(style::SIDEBAR), 1.0)
        .with_fixed(border(Axis::Horizontal, on_sidebar))
        .with_fixed(NewWidget::new(footer).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_6))).with(Padding::all(Length::px(size::SPACE_6)))).with_class(style::SIDEBAR));
    let sidebar = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with(NewWidget::new(sidebar), 1.0)
        .with_fixed(Rule::on(Axis::Vertical, || theme::current().sidebar_border, on_sidebar));
    let sidebar = widgets::Collapse::new(NewWidget::new(sidebar).erased(), size::SIDEBAR_WIDTH + size::BORDER_1, sidebar_open && !focus_mode, sidebar_from, on_sidebar);
    let sidebar_handle = Handle::of(&sidebar);

    // Основная колонка: полоса вкладок (пока пустая) и текст заметки.
    let note_page = widgets::Slot::new(note);
    let note_handle = Handle::of(&note_page);
    let titlebar = titlebar(tabs, panel_open, panel_from, focus_mode);
    focus_hidden.push(titlebar.toggle_shown);
    let (panel, panel_handle, panel_parts) = backlinks::panel(panel_mode, has_analysis, panel_open, panel_from);
    let (empty, create_note, open_file, close_tab) = new_tab_view();
    let version_page = widgets::Slot::new(NewWidget::new(Flex::column()).erased());
    let version_handle = Handle::of(&version_page);
    let (no_vault, open_vault) = workspaces::no_vault(vault_error);
    let active = if !vault_open {
        3
    } else if graph_active {
        4
    } else {
        usize::from(empty_tab)
    };
    let graph_page = widgets::Slot::new(graph.unwrap_or_else(|| NewWidget::new(SizedBox::empty()).erased()));
    let graph_handle = Handle::of(&graph_page);
    let pages = IndexedStack::new().with(note_page).with(empty).with(version_page).with(no_vault).with(graph_page).with_active_child(active);
    let pages = NewWidget::new(pages);
    let pages_handle = Handle::of(&pages);
    let main = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(titlebar.widget)
        .with(pages, 1.0);

    let window = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(rail)
        .with_fixed(sidebar)
        .with(NewWidget::new(main).with_class(style::CANVAS), 1.0)
        .with_fixed(panel);
    let chrome = Chrome {
        toggle_sidebar,
        settings: settings_id,
        new_tab: titlebar.new_tab,
        tab_strip: titlebar.strip,
        panel_toggle: titlebar.toggle,
        controls_inset: titlebar.inset,
        panel: panel_handle,
        panel_parts,
        pages: pages_handle,
        version_page: version_handle,
        create_note,
        open_file,
        close_tab,
        workspaces,
        open_vault,
        sidebar: sidebar_handle,
        note_page: note_handle,
        rail_settings: rail_settings_id,
        home_slot,
        home_button,
        graph_page: graph_handle,
        graph_button,
        focus_hidden,
        focus_shown,
    };
    (NewWidget::new(window), chrome)
}

/// Снимки интерфейса без окна: `cargo test -p aquilum-native snapshot -- --ignored`, PNG — в
/// каталог из `AQUILUM_SNAPSHOTS` (по умолчанию временный).
#[cfg(test)]
mod snapshot;
