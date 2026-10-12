use masonry::kurbo::Point;

use super::Scene;
use crate::links::{Item, Listing, Method, Mode, Target};
use crate::ui::backlinks::{self, HistoryRow};
use crate::ui::snapshot::{W, open_parts, parts};
use crate::ui::titlebar::window_controls;
use crate::ui::tokens::size;
use crate::ui::widgets::Slot;

fn item(title: &str, counter: Option<&str>) -> Item {
    Item { title: title.into(), counter: counter.map(str::to_owned), target: Target::Wiki(title.into()), reasons: Vec::new() }
}

fn history_row(label: &str, time: &str, selected: bool) -> HistoryRow {
    HistoryRow { label: label.into(), time: time.into(), selected, renaming: false }
}

pub(super) fn snapshots(scene: &Scene) {
    for mode in [Mode::Backlinks, Mode::Analysis] {
        let mut p = parts("# Заметка");
        p.panel_mode = mode;
        let (mut root, chrome) = open_parts(Some(scene.vault), scene.data, p);
        let listing = Listing {
            generation: 1,
            items: vec![item("Уход от Tauri", Some("92.4%")), item("Aquilum", Some("71.0%")), item("Очень длинное название заметки, которое не влезет", Some("12.5%"))],
            notice: None,
        };
        let methods = [Method::Bm25f, Method::AdamicAdar, Method::Wikixiv];
        let (body, _) = backlinks::body(mode, &methods, Method::Bm25f, Some(&listing));
        chrome.panel_parts.body.edit(&mut root, |mut slot| Slot::set_child(&mut slot, body));
        let (controls, _) = window_controls(false);
        root.add_layer(controls, Point::new(f64::from(W) - size::TITLEBAR_CONTROLS_INSET, 0.0));
        scene.save(&mut root, &format!("panel-{}", mode.key()));
    }
    history(scene);
}

fn history(scene: &Scene) {
    let mut p = parts("");
    p.panel_mode = Mode::History;
    let (mut root, chrome) = open_parts(Some(scene.vault), scene.data, p);
    let days = vec![
        ("Сегодня".to_owned(), vec![(0, history_row("Агент", "14:05", true)), (1, history_row("Вы", "13:40", false))]),
        ("Вчера".to_owned(), vec![(2, history_row("Черновик к созвону", "18:12", false))]),
    ];
    let (body, _) = backlinks::history_body(false, days, false, true);
    chrome.panel_parts.body.edit(&mut root, |mut slot| Slot::set_child(&mut slot, body));
    let lines = crate::history::diff(
        Some("# Заметка\n\nСтарый абзац.\nОбщая строка."),
        "# Заметка\n\nНовый абзац,\nсовсем другой.\nОбщая строка.",
    );
    let (view, _) = backlinks::version_view("Агент · 8 октября в 14:05", true, true, &lines, None);
    chrome.version_page.edit(&mut root, |mut slot| Slot::set_child(&mut slot, view));
    chrome.pages.edit(&mut root, |mut pages| masonry::widgets::IndexedStack::set_active_child(&mut pages, 2));
    scene.save(&mut root, "history");
}
