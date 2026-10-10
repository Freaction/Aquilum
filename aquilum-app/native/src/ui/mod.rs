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
pub mod settings;
pub mod icons;
pub mod text;
pub mod theme;
pub mod titlebar;
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
use masonry::properties::{Background, Gap, Padding};
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
    pub controls_inset: Handle<SizedBox>,
    pub panel: Handle<SizedBox>,
    pub panel_parts: backlinks::Panel,
    pub pages: Handle<IndexedStack>,
    pub version_page: Handle<widgets::Slot>,
    pub create_note: WidgetId,
    pub open_file: WidgetId,
    pub close_tab: WidgetId,
    pub workspaces: WidgetId,
    pub open_vault: WidgetId,
    /// Боковая панель: ширина 0, когда она скрыта.
    pub sidebar: Handle<SizedBox>,
    pub sidebar_border: Handle<Rule>,
    pub note_page: Handle<widgets::Slot>,
    pub rail_settings: WidgetId,
    pub home_slot: Handle<widgets::Slot>,
    pub home_button: Option<WidgetId>,
    pub graph_page: Handle<widgets::Slot>,
    pub graph_button: Option<WidgetId>,
}

pub fn rescale(root: &mut masonry::app::RenderRoot, scale: f64) {
    let size = root.size();
    root.handle_window_event(masonry::core::WindowEvent::Rescale(scale));
    root.handle_window_event(masonry::core::WindowEvent::Resize(size));
}

fn background(color: Color) -> PropertySet {
    PropertySet::new().with(Background::Color(color))
}

fn border(axis: Axis, base: fn() -> Color) -> NewWidget<Rule> {
    Rule::on(axis, || theme::current().sidebar_border, base, false)
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
    NewWidget::new(header.height(Length::px(size::TABS_HEIGHT))).with_props(
        background(theme::current().tabs_bg).with(Padding::all(Length::px(padding))),
    )
}

pub struct RootParts<'a> {
    pub workspace_name: &'a str,
    pub note: NewWidget<dyn Widget>,
    pub sidebar_open: bool,
    pub tabs: Vec<NewWidget<dyn Widget>>,
    pub empty_tab: bool,
    pub vault_open: bool,
    pub vault_error: Option<&'a str>,
    pub panel_open: bool,
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

pub fn controls_inset(panel_open: bool) -> f64 {
    if panel_open || cfg!(target_os = "macos") { 0.0 } else { size::TITLEBAR_CONTROLS_INSET }
}

struct Titlebar {
    widget: NewWidget<Flex>,
    new_tab: WidgetId,
    strip: Handle<TabStrip>,
    toggle: WidgetId,
    inset: Handle<SizedBox>,
}

fn titlebar(tabs: Vec<NewWidget<dyn Widget>>, panel_open: bool, focus_mode: bool) -> Titlebar {
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
    let label = if panel_open { "titlebar.hideRightSidebar" } else { "titlebar.showRightSidebar" };
    let toggle = NewWidget::new(IconButton::new(icons::PANEL_RIGHT, i18n::t(label)));
    let toggle_id = toggle.id();
    let mut menu = Flex::row();
    if !focus_mode {
        menu = menu.with_fixed(toggle);
    }
    let menu = NewWidget::new(menu)
        .with_props(PropertySet::new().with(Padding::from_vh(Length::px(size::SPACE_4), Length::px(size::SPACE_6))));
    let inset = NewWidget::new(SizedBox::empty().width(Length::px(controls_inset(panel_open && !focus_mode))));
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
    Titlebar { widget, new_tab: plus_id, strip: strip_handle, toggle: toggle_id, inset: inset_handle }
}

fn new_tab_view() -> (NewWidget<Flex>, WidgetId, WidgetId, WidgetId) {
    let create = LinkButton::new(&i18n::t("editor.createNote"), true);
    let open = LinkButton::new(&i18n::t("editor.openFile"), true);
    let close = LinkButton::new(&i18n::t("editor.close"), true);
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
    let RootParts { workspace_name, note, sidebar_open, tabs, empty_tab, vault_open, vault_error, panel_open, panel_mode, has_analysis, focus_mode, home, graph, graph_active } = parts;
    let t = theme::current();

    let toggle = NewWidget::new(IconButton::new(icons::PANEL_LEFT, "Скрыть левую боковую панель"));
    let (home, home_button) = home_widget(home && !focus_mode);
    let home = widgets::Slot::fit(home);
    let home_slot = Handle::of(&home);
    let rail_settings = NewWidget::new(IconButton::new(icons::SETTINGS, i18n::t("common.settings")));
    let toggle_sidebar = toggle.id();
    let rail_settings_id = rail_settings.id();
    let mut tools = Flex::column().cross_axis_alignment(CrossAxisAlignment::Center);
    tools = tools.with_fixed(home);
    let mut graph_button = None;
    if !focus_mode {
        let button = NewWidget::new(IconButton::new(icons::NETWORK, i18n::t("rail.graph")));
        graph_button = Some(button.id());
        tools = tools.with_fixed(button);
    }
    if focus_mode {
        tools = tools.with_spacer(1.0).with_fixed(rail_settings);
    }
    let header = match focus_mode {
        true => SizedBox::empty(),
        false => SizedBox::new(toggle),
    };
    let header = NewWidget::new(header.height(Length::px(size::TABS_HEIGHT))).with_props(background(t.tabs_bg).with(Padding::all(Length::px(size::SPACE_4))));
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
    let rail = NewWidget::new(rail).with_props(background(t.sidebar_bg));
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
        .with(NewWidget::new(SizedBox::new(tree)).with_props(background(t.sidebar_bg)), 1.0)
        .with_fixed(border(Axis::Horizontal, on_sidebar))
        .with_fixed(NewWidget::new(footer).with_props(
            background(t.sidebar_bg)
                .with(Gap::new(Length::px(size::SPACE_6)))
                .with(Padding::all(Length::px(size::SPACE_6))),
        ));
    let width = if sidebar_open && !focus_mode { size::SIDEBAR_WIDTH } else { 0.0 };
    let sidebar = NewWidget::new(SizedBox::new(NewWidget::new(sidebar)).width(Length::px(width)));
    let sidebar_handle = Handle::of(&sidebar);
    let sidebar_border = Rule::on(Axis::Vertical, || theme::current().sidebar_border, on_sidebar, width == 0.0);
    let sidebar_border_handle = Handle::of(&sidebar_border);

    // Основная колонка: полоса вкладок (пока пустая) и текст заметки.
    let note_page = widgets::Slot::new(note);
    let note_handle = Handle::of(&note_page);
    let titlebar = titlebar(tabs, panel_open, focus_mode);
    let (panel, panel_handle, panel_parts) = backlinks::panel(panel_mode, has_analysis, panel_open);
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
        .with_fixed(sidebar_border)
        .with(NewWidget::new(main).with_props(background(t.bg_canvas)), 1.0)
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
        sidebar_border: sidebar_border_handle,
        note_page: note_handle,
        rail_settings: rail_settings_id,
        home_slot,
        home_button,
        graph_page: graph_handle,
        graph_button,
    };
    (NewWidget::new(window), chrome)
}

/// Снимки интерфейса без окна: `cargo test -p aquilum-native snapshot -- --ignored`, PNG — в
/// каталог из `AQUILUM_SNAPSHOTS` (по умолчанию временный).
#[cfg(test)]
mod snapshot {
    use std::sync::Arc;

    use masonry::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
    use masonry::dpi::PhysicalSize;
    use masonry::kurbo::Point;

    use super::dialog::ConfirmDialog;
    use super::file_tree::FileTree;
    use super::menu::Menu;
    use super::settings::schema::Section;
    use super::theme;
    use crate::render::Renderer;

    const W: u32 = 1000;
    const H: u32 = 600;

    fn parts(text: &str) -> super::RootParts<'_> {
        let tabs = vec![
            super::titlebar::TabButton::new(uuid::Uuid::new_v4(), "Заметка", true, true, None).erased(),
            super::titlebar::TabButton::new(uuid::Uuid::new_v4(), "Уход от Tauri", false, true, None).erased(),
        ];
        super::RootParts {
            workspace_name: "knowledge base",
            note: super::note::page(super::note::NoteView {
                file_name: "Заметка.md",
                title: "Заметка",
                body: text,
                metadata: Some(false),
                can_back: true,
                can_forward: false,
                focus_mode: false,
                hero: None,
                resolver: None,
            })
            .0
            .erased(),
            sidebar_open: true,
            tabs,
            graph: None,
            graph_active: false,
            empty_tab: false,
            vault_open: true,
            vault_error: None,
            focus_mode: false,
            home: true,
            panel_open: true,
            panel_mode: crate::links::Mode::Backlinks,
            has_analysis: true,
        }
    }

    fn shell(vault: &std::path::Path, data: &std::path::Path) -> (RenderRoot, FileTree) {
        shell_at(vault, data, 1.0)
    }

    fn shell_at(vault: &std::path::Path, data: &std::path::Path, scale: f64) -> (RenderRoot, FileTree) {
        let mut tree = FileTree::new(Some(vault.to_path_buf()), data);
        let (widget, _) = super::root(tree.widget(), parts("# Заметка\n\nТекст заметки."));
        let options = RenderRootOptions {
            default_properties: Arc::new(theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new((f64::from(W) * scale) as u32, (f64::from(H) * scale) as u32),
            scale_factor: scale,
            test_font: None,
        };
        let mut root = RenderRoot::new(widget.erased(), |_| {}, options);
        theme::register_fonts(&mut root);
        (root, tree)
    }

    fn save(root: &mut RenderRoot, name: &str) {
        let (layers, _) = root.redraw();
        let mut renderer = Renderer::new(0);
        let size = root.size();
        let scale = f64::from(size.width) / f64::from(W);
        renderer.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).expect("кадр");
        let dir = std::env::var_os("AQUILUM_SNAPSHOTS").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let path = dir.join(format!("{name}.png"));
        std::fs::write(&path, renderer.png()).expect("снимок записан");
        eprintln!("снимок: {}", path.display());
    }

    /// Смена темы, языка или шрифта пересобирает интерфейс внутри гнезда `ROOT_TAG`.
    #[test]
    fn root_can_be_rebuilt() {
        let _theme = theme::test_lock();
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::write(vault.path().join("a.md"), "").unwrap();
        let mut tree = FileTree::new(Some(vault.path().to_path_buf()), data.path());
        let (widget, mut chrome) = super::root(tree.widget(), parts("текст"));
        let options = RenderRootOptions {
            default_properties: Arc::new(theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new(W, H),
            scale_factor: 1.0,
            test_font: None,
        };
        let slot = super::widgets::Slot::new(widget.erased()).with_tag(super::ROOT_TAG);
        let mut root = RenderRoot::new(slot.erased(), |_| {}, options);
        let _ = root.redraw();
        for _ in 0..2 {
            let old = chrome.note_page;
            let (widget, next) = super::root(tree.widget(), parts("текст"));
            root.set_default_properties(Arc::new(theme::default_properties()));
            root.edit_widget_with_tag(super::ROOT_TAG, |mut slot| super::widgets::Slot::set_child(&mut slot, widget.erased()));
            let _ = root.redraw();
            assert!(old.get(&root).is_none(), "старый редактор удалён");
            chrome = next;
        }
        assert!(chrome.note_page.get(&root).is_some());
        // Поле переименования создаётся заново при каждом переименовании.
        for _ in 0..2 {
            tree.start_rename(&mut root, vault.path().join("a.md"));
            assert!(root.focused_widget().is_some(), "поле переименования в фокусе");
            tree.refresh(&mut root);
        }
    }

    #[test]
    fn rescale_matches_fresh_layout() {
        let _theme = theme::test_lock();
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        for note in ["Aquilum.md", "Уход от Tauri.md"] {
            std::fs::write(vault.path().join(note), "").unwrap();
        }
        let frame = |root: &mut RenderRoot| {
            let (layers, _) = root.redraw();
            let size = root.size();
            let mut renderer = Renderer::new(0);
            let scale = 1.75 * 1.05;
            renderer.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
            renderer.png()
        };
        let (mut live, _) = shell_at(vault.path(), data.path(), 1.75);
        let _ = live.redraw();
        super::rescale(&mut live, 1.75 * 1.05);
        let live = frame(&mut live);
        let options = RenderRootOptions {
            default_properties: Arc::new(theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new((f64::from(W) * 1.75) as u32, (f64::from(H) * 1.75) as u32),
            scale_factor: 1.75 * 1.05,
            test_font: None,
        };
        let mut tree = FileTree::new(Some(vault.path().to_path_buf()), data.path());
        let (widget, _) = super::root(tree.widget(), parts("# Заметка\n\nТекст заметки."));
        let mut fresh = RenderRoot::new(widget.erased(), |_| {}, options);
        theme::register_fonts(&mut fresh);
        let fresh = frame(&mut fresh);
        if let Some(dir) = std::env::var_os("AQUILUM_SNAPSHOTS").map(std::path::PathBuf::from) {
            std::fs::write(dir.join("rescale-live.png"), &live).unwrap();
            std::fs::write(dir.join("rescale-fresh.png"), &fresh).unwrap();
        }
        assert!(live == fresh, "кадр после смены масштаба отличается от собранного заново");
    }

    #[test]
    #[ignore = "снимки для просмотра глазами"]
    fn snapshot_tree_menu_dialog() {
        let _theme = theme::test_lock();
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        for dir in ["Проекты", "Архив"] {
            std::fs::create_dir_all(vault.path().join(dir)).unwrap();
        }
        for note in ["Aquilum.md", "Уход от Tauri.md", "Очень длинное название заметки, которое не влезет.md"] {
            std::fs::write(vault.path().join(note), "").unwrap();
        }
        crate::i18n::set_language("ru");
        for (dark, suffix) in [(true, "dark"), (false, "light")] {
            theme::apply_setting(if dark { "dark" } else { "light" }, false, "");
            let (mut root, _) = shell(vault.path(), data.path());
            root.add_layer(Menu::new("Действия с файлом", &["Переименовать", "Дублировать", "Удалить"]), Point::new(150.0, 120.0));
            save(&mut root, &format!("menu-{suffix}"));

            let long = tempfile::tempdir().unwrap();
            for i in 0..40 {
                std::fs::write(long.path().join(format!("Заметка {i:02}.md")), "").unwrap();
            }
            let (mut root, _) = shell(long.path(), data.path());
            save(&mut root, &format!("scroll-{suffix}"));

            let empty = tempfile::tempdir().unwrap();
            let (mut root, _) = shell(empty.path(), data.path());
            save(&mut root, &format!("empty-{suffix}"));

            let (mut root, mut tree) = shell(vault.path(), data.path());
            tree.start_rename(&mut root, vault.path().join("Aquilum.md"));
            save(&mut root, &format!("rename-{suffix}"));

            let (mut root, _) = shell(vault.path(), data.path());
            let (dialog, _) = ConfirmDialog::new(
                "Удалить заметку?",
                "«Уход от Tauri» переедет в корзину базы знаний.",
                "Отмена",
                "Удалить",
            );
            root.add_layer(dialog, Point::ORIGIN);
            save(&mut root, &format!("dialog-{suffix}"));

            let config = aquilum_core::settings::models::AppConfig::default();
            for mode in [crate::links::Mode::Backlinks, crate::links::Mode::Analysis] {
                let mut tree = FileTree::new(Some(vault.path().to_path_buf()), data.path());
                let mut p = parts("# Заметка");
                p.panel_mode = mode;
                let (widget, chrome) = super::root(tree.widget(), p);
                let options = RenderRootOptions {
                    default_properties: Arc::new(theme::default_properties()),
                    use_system_fonts: true,
                    size_policy: WindowSizePolicy::User,
                    size: PhysicalSize::new(W, H),
                    scale_factor: 1.0,
                    test_font: None,
                };
                let mut root = RenderRoot::new(widget.erased(), |_| {}, options);
                theme::register_fonts(&mut root);
                let item = |title: &str, counter: Option<&str>| crate::links::Item {
                    title: title.into(),
                    counter: counter.map(str::to_owned),
                    target: crate::links::Target::Wiki(title.into()),
                };
                let listing = crate::links::Listing {
                    generation: 1,
                    items: vec![item("Уход от Tauri", Some("92.4%")), item("Aquilum", Some("71.0%")), item("Очень длинное название заметки, которое не влезет", Some("12.5%"))],
                    notice: None,
                };
                let methods = [crate::links::Method::Bm25f, crate::links::Method::AdamicAdar, crate::links::Method::Wikixiv];
                let (body, _) = super::backlinks::body(mode, &methods, crate::links::Method::Bm25f, Some(&listing));
                chrome.panel_parts.body.edit(&mut root, |mut slot| super::widgets::Slot::set_child(&mut slot, body));
                let (controls, _) = super::titlebar::window_controls(false);
                root.add_layer(controls, Point::new(f64::from(W) - super::size::TITLEBAR_CONTROLS_INSET, 0.0));
                save(&mut root, &format!("panel-{}-{suffix}", mode.key()));
            }
            {
                let mut tree = FileTree::new(Some(vault.path().to_path_buf()), data.path());
                let mut p = parts("");
                p.panel_mode = crate::links::Mode::History;
                let (widget, chrome) = super::root(tree.widget(), p);
                let options = RenderRootOptions {
                    default_properties: Arc::new(theme::default_properties()),
                    use_system_fonts: true,
                    size_policy: WindowSizePolicy::User,
                    size: PhysicalSize::new(W, H),
                    scale_factor: 1.0,
                    test_font: None,
                };
                let mut root = RenderRoot::new(widget.erased(), |_| {}, options);
                theme::register_fonts(&mut root);
                let row = |label: &str, time: &str, selected| super::backlinks::HistoryRow {
                    label: label.into(),
                    time: time.into(),
                    selected,
                    renaming: false,
                };
                let days = vec![
                    ("Сегодня".to_owned(), vec![(0, row("Агент", "14:05", true)), (1, row("Вы", "13:40", false))]),
                    ("Вчера".to_owned(), vec![(2, row("Черновик к созвону", "18:12", false))]),
                ];
                let (body, _) = super::backlinks::history_body(false, days, false, true);
                chrome.panel_parts.body.edit(&mut root, |mut slot| super::widgets::Slot::set_child(&mut slot, body));
                let lines = crate::history::diff(Some("# Заметка

Старый абзац.
Общая строка."), "# Заметка

Новый абзац,
совсем другой.
Общая строка.");
                let (view, _) = super::backlinks::version_view("Агент · 8 октября в 14:05", true, true, &lines, None);
                chrome.version_page.edit(&mut root, |mut slot| super::widgets::Slot::set_child(&mut slot, view));
                chrome.pages.edit(&mut root, |mut pages| masonry::widgets::IndexedStack::set_active_child(&mut pages, 2));
                save(&mut root, &format!("history-{suffix}"));
            }
            {
                let (mut root, _) = shell(vault.path(), data.path());
                let terms = vec!["кофе".to_owned()];
                let card = |title: &str, snippet: &str, count: &str, selected| {
                    let find = |text: &str| {
                        aquilum_core::search::matching::find_prefix_matches(text, &terms).into_iter().map(|(s, e, _)| (s, e)).collect()
                    };
                    super::search::ResultCard::new(
                        super::search::CardData {
                            title: title.into(),
                            extension: "md".into(),
                            snippet: snippet.into(),
                            count: count.into(),
                            title_marks: find(title),
                            extension_marks: Vec::new(),
                            snippet_marks: find(snippet),
                        },
                        selected,
                    )
                };
                let (list, _) = super::search::results_list(vec![
                    card("Кофе", "Заметка про обжарку кофе: светлая, средняя и тёмная. Кофе лучше молоть перед завариванием.", "2 совпадения", true),
                    card("Чай и кофе", "Сравнение кофеина в чае и кофе, заметки с дегустаций и ссылки на источники.", "3 совпадения", false),
                ]);
                let search_box = super::search::search_box("кофе");
                let card = masonry::widgets::Flex::column()
                    .cross_axis_alignment(masonry::properties::types::CrossAxisAlignment::Stretch)
                    .with_fixed(search_box.widget)
                    .with_fixed(super::widgets::Rule::new(masonry::kurbo::Axis::Horizontal, || theme::current().border_default))
                    .with(masonry::core::NewWidget::new(super::scroll::ScrollArea::new(list)), 1.0)
                    .with_fixed(super::widgets::Rule::new(masonry::kurbo::Axis::Horizontal, || theme::current().border_default))
                    .with_fixed(super::search::footer());
                let layer = super::dialog::Modal::new(
                    masonry::core::NewWidget::new(card),
                    "Поиск",
                    super::dialog::CardSize::Window(super::search::card_size),
                    masonry::accesskit::Role::Dialog,
                    false,
                );
                root.add_layer(layer, Point::ORIGIN);
                save(&mut root, &format!("search-{suffix}"));
            }
            {
                let (mut root, _) = shell(vault.path(), data.path());
                let known = vec![
                    (uuid::Uuid::new_v4(), vault.path().to_string_lossy().into_owned()),
                    (uuid::Uuid::new_v4(), r"D:\Заметки\Работа".to_owned()),
                    (uuid::Uuid::new_v4(), r"C:\Users\Dmitriy\Documents\Очень длинное имя базы знаний для проверки".to_owned()),
                ];
                let (layer, _) = super::workspaces::dialog(&known, Some(vault.path()), Some(r"Не удалось открыть «D:\Нет»."));
                root.add_layer(layer, Point::ORIGIN);
                save(&mut root, &format!("workspaces-{suffix}"));
                let (mut root, _) = shell(vault.path(), data.path());
                let (layer, _) = super::workspaces::dialog(&[], None, None);
                root.add_layer(layer, Point::ORIGIN);
                save(&mut root, &format!("workspaces-empty-{suffix}"));
                let mut tree = FileTree::new(None, data.path());
                let mut p = parts("");
                p.vault_open = false;
                let (widget, _) = super::root(tree.widget(), p);
                let options = RenderRootOptions {
                    default_properties: Arc::new(theme::default_properties()),
                    use_system_fonts: true,
                    size_policy: WindowSizePolicy::User,
                    size: PhysicalSize::new(W, H),
                    scale_factor: 1.0,
                    test_font: None,
                };
                let mut root = RenderRoot::new(widget.erased(), |_| {}, options);
                theme::register_fonts(&mut root);
                save(&mut root, &format!("no-vault-{suffix}"));
            }
            for (name, pattern, book) in [("book", true, true), ("cover", true, false), ("photo", false, true)] {
                let mut tree = FileTree::new(Some(vault.path().to_path_buf()), data.path());
                let mut p = parts("");
                let visual = if pattern {
                    super::book::Visual::Pattern("tunnel")
                } else {
                    super::book::Visual::Image(crate::images::default_book_cover())
                };
                let (page, ids) = super::note::page(super::note::NoteView {
                    file_name: "Дюна.md",
                    title: "Дюна",
                    body: "Фрэнк Герберт, 1965.",
                    metadata: Some(false),
                    can_back: true,
                    can_forward: false,
                    focus_mode: false,
                    hero: Some(super::book::HeroView {
                        cover: Some(visual),
                        position: 30.0,
                        book: book.then(|| super::book::BookView { cover: crate::images::default_book_cover(), has_file: false }),
                        viewport: f64::from(H),
                        motion_enabled: true,
                    }),
                    resolver: None,
                });
                p.note = page.erased();
                let (widget, _) = super::root(tree.widget(), p);
                let options = RenderRootOptions {
                    default_properties: Arc::new(theme::default_properties()),
                    use_system_fonts: true,
                    size_policy: WindowSizePolicy::User,
                    size: PhysicalSize::new(W, H),
                    scale_factor: 1.0,
                    test_font: None,
                };
                let mut root = RenderRoot::new(widget.erased(), |_| {}, options);
                theme::register_fonts(&mut root);
                let _ = root.redraw();
                for _ in 0..500 {
                    if crate::cover::take_finished() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                if let Some(cover) = ids.hero.as_ref().and_then(|(_, hero)| hero.cover) {
                    cover.edit(&mut root, |mut cover| super::book::PageCover::refresh(&mut cover));
                }
                save(&mut root, &format!("hero-{name}-{suffix}"));
            }
            theme::apply_setting(if dark { "dark" } else { "light" }, false, "#e05a2b");
            let (mut root, _) = shell(vault.path(), data.path());
            let (layer, _) = super::settings::dialog(&config, &super::settings::schema::Context::new(1.0), Section::Ui);
            root.add_layer(layer, Point::ORIGIN);
            save(&mut root, &format!("primary-{suffix}"));
            theme::apply_setting(if dark { "dark" } else { "light" }, false, "");
            for scale in [1.75, 1.75 * 1.05] {
                let (mut root, _) = shell_at(vault.path(), data.path(), scale);
                let (layer, _) = super::settings::dialog(&config, &super::settings::schema::Context::new(1.0), Section::Ui);
                root.add_layer(layer, Point::ORIGIN);
                save(&mut root, &format!("scaled-{}-{suffix}", (scale * 100.0).round()));
            }
            for section in [Section::Ui, Section::Editor, Section::Shortcuts, Section::Mcp, Section::System, Section::Trash] {
                let (mut root, _) = shell(vault.path(), data.path());
                let (layer, _) = super::settings::dialog(&config, &super::settings::schema::Context::new(1.0), section);
                root.add_layer(layer, Point::ORIGIN);
                save(&mut root, &format!("settings-{}-{suffix}", section.key()));
            }
        }
    }
}
