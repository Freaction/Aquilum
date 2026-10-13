use super::*;
use crate::ui::menu::{Menu, MenuChoice};
use crate::ui::tokens::size;
use masonry::core::WidgetId;
use masonry::kurbo::Rect;

fn rect(h: &Harness, id: WidgetId) -> Rect {
    h.root.get_widget(id).unwrap().ctx().bounding_box()
}

fn open_tabs(h: &mut Harness, count: usize) -> Vec<std::path::PathBuf> {
    let vault = h.vault.path().to_path_buf();
    let paths: Vec<_> = (0..count).map(|i| vault.join(format!("вкладка {i:02}.md"))).collect();
    for path in &paths {
        std::fs::write(path, "текст").unwrap();
        h.app.open_in(&mut h.root, path.clone(), true);
    }
    h.settle();
    paths
}

fn open_list(h: &mut Harness) -> WidgetId {
    let button = h.app.chrome.as_ref().unwrap().tab_list;
    let center = rect(h, button).center();
    h.click(center.x, center.y);
    match h.app.popup {
        Some(Popup::TabsMenu { layer, .. }) => layer,
        _ => panic!("список вкладок не открылся"),
    }
}

#[test]
fn the_tab_list_button_sits_next_to_the_plus_and_the_list_picks_a_tab() {
    let mut h = Harness::new();
    let paths = open_tabs(&mut h, 4);
    let chrome = h.app.chrome.as_ref().unwrap();
    let (plus, list) = (rect(&h, chrome.new_tab), rect(&h, chrome.tab_list));
    assert_eq!((list.x0 - plus.x1, list.y0, list.height()), (size::SPACE_2, plus.y0, plus.height()), "кнопки не в одной группе");
    let layer = open_list(&mut h);
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("tab-list.png"), h.frame(W as u16, H as u16, 1.0).png()).unwrap();
    }
    let menu = rect(&h, layer);
    let tabs = h.app.tabs.tabs().len();
    assert_eq!(menu.width(), size::TAB_LIST_WIDTH);
    assert!(menu.x1 <= list.x1 + 0.5 && menu.y0 >= list.y1, "список не под кнопкой справа: {menu:?} против {list:?}");
    let content = Menu::estimated_size(tabs).height;
    assert!((menu.height() - content).abs() < 1.0, "список прокручивается, хотя места хватает: {} против {content}", menu.height());
    let choice = masonry::core::ErasedAction::from(Box::new(MenuChoice::Item(1)));
    h.app.on_action(&mut h.root, layer, choice, Size::new(f64::from(W), f64::from(H)));
    assert!(h.app.popup.is_none(), "список не закрылся после выбора");
    assert_eq!(h.app.tabs.active().path.as_deref(), h.app.tabs.tabs()[1].path.as_deref());
    assert!(paths.iter().any(|p| h.app.tabs.active().path.as_deref() == Some(p.as_path())), "выбрана не вкладка с заметкой");
}

#[test]
fn the_tab_list_scrolls_only_when_the_window_has_no_room_left() {
    let mut h = Harness::new();
    open_tabs(&mut h, 40);
    let layer = open_list(&mut h);
    let menu = rect(&h, layer);
    assert!((menu.y1 - (f64::from(H) - size::SPACE_8)).abs() < 1.0, "длинный список не дошёл до нижней границы окна: {menu:?}");
    let button = h.app.chrome.as_ref().unwrap().tab_list;
    let center = rect(&h, button).center();
    h.click(center.x, center.y);
    assert!(h.app.popup.is_none(), "повторный щелчок по кнопке не закрыл список");
}
