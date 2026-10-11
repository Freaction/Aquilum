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
            smart: Default::default(),
        })
        .0
        .erased(),
        sidebar_open: true,
        sidebar_from: true,
        tabs,
        graph: None,
        graph_active: false,
        empty_tab: false,
        vault_open: true,
        vault_error: None,
        focus_mode: false,
        home: true,
        panel_open: true,
        panel_from: true,
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
                reasons: Vec::new(),
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
                }),
                resolver: None,
                smart: Default::default(),
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
