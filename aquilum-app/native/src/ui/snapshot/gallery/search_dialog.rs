use masonry::accesskit::Role;
use masonry::core::NewWidget;
use masonry::kurbo::{Axis, Point};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::Scene;
use crate::ui::dialog::{CardSize, Modal};
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{self, CardData, ResultCard};
use crate::ui::theme;
use crate::ui::widgets::Rule;

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
    let column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(search_box.widget)
        .with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default))
        .with(NewWidget::new(ScrollArea::new(list)), 1.0)
        .with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default))
        .with_fixed(search::footer());
    let layer = Modal::new(NewWidget::new(column), "Поиск", CardSize::Window(search::card_size), Role::Dialog, false);
    root.add_layer(layer, Point::ORIGIN);
    scene.save(&mut root, "search");
}
