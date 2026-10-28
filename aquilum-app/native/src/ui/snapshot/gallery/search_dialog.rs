use masonry::accesskit::Role;
use masonry::core::NewWidget;
use masonry::kurbo::Point;

use super::Scene;
use crate::ui::dialog::{self, CardSize, Modal};
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{self, CardData, ResultCard};

pub(super) fn snapshot(scene: &Scene) {
    let (mut root, _) = scene.shell();
    let terms = vec!["кофе".to_owned()];
    let card = |title: &str, snippet: &str, count: &str, selected| {
        let find = |text: &str| {
            aquilum_core::search::hits::query_matcher(&terms).find_all(text).into_iter().map(|r| (r.start, r.end)).collect()
        };
        ResultCard::new(
            CardData {
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
    let (list, _) = search::results_list(vec![
        card("Кофе", "Заметка про обжарку кофе: светлая, средняя и тёмная. Кофе лучше молоть перед завариванием.", "2 совпадения", true),
        card("Чай и кофе", "Сравнение кофеина в чае и кофе, заметки с дегустаций и ссылки на источники.", "3 совпадения", false),
    ]);
    let search_box = search::search_box("кофе");
    let column = dialog::card(search_box.widget, NewWidget::new(ScrollArea::new(list)), search::footer());
    let layer = Modal::new(column, "Поиск", CardSize::Window(search::card_size), Role::Dialog, false);
    root.add_layer(layer, Point::ORIGIN);
    scene.save(&mut root, "search");
}
