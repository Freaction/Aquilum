use super::*;
use crate::ui::menu::MenuChoice;
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
    assert_eq!(menu.width(), size::TAB_LIST_WIDTH);
    assert!(menu.x1 <= list.x1 + 0.5 && menu.y0 >= list.y1, "список не под кнопкой справа: {menu:?} против {list:?}");
    let mut items = h.root.get_widget(layer).unwrap();
    while items.children().len() == 1 {
        items = items.children()[0];
    }
    let content = items.ctx().bounding_box().height();
    assert!((menu.height() - content).abs() < 1.0, "список прокручивается, хотя места хватает: {} против {content}", menu.height());
    let choice = masonry::core::ErasedAction::from(Box::new(MenuChoice::Item(1)));
    h.app.on_action(&mut h.root, layer, choice);
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

#[test]
fn the_active_tab_stays_in_view_when_tabs_overflow_the_strip() {
    let mut h = Harness::new();
    open_tabs(&mut h, 30);
    let strip = h.app.chrome.as_ref().unwrap().tab_strip.id();
    let visible = |h: &Harness| {
        let widget = h.root.get_widget(strip).unwrap();
        let (bounds, tab) = (widget.ctx().bounding_box(), widget.children()[h.app.tabs.active_index()].ctx().bounding_box());
        tab.x0 >= bounds.x0 - 0.5 && tab.x1 <= bounds.x1 + 0.5
    };
    assert!(visible(&h), "новая вкладка в конце ленты не видна");
    let first = h.app.tabs.tabs()[0].id;
    h.app.on_titlebar(&mut h.root, &crate::ui::titlebar::TitlebarAction::Select(first));
    h.settle();
    assert!(visible(&h), "первая вкладка не видна после выбора");
}

#[test]
fn a_dragged_sidebar_leaves_room_for_the_editor_and_resets_on_double_click() {
    use crate::interface::Side;
    use crate::ui::widgets::SashAction;
    let mut h = Harness::new();
    let chrome = h.app.chrome.as_ref().unwrap();
    let (sash, sidebar, panel, rail) = (chrome.sidebar_sash, chrome.sidebar.id(), chrome.panel.id(), chrome.rail.id());
    let send = |h: &mut Harness, action: SashAction| {
        h.app.on_action(&mut h.root, sash, masonry::core::ErasedAction::from(Box::new(action)));
        h.settle();
    };
    let start = rect(&h, sidebar).width();
    for action in [SashAction::Press, SashAction::Drag(40.0), SashAction::Release] {
        send(&mut h, action);
    }
    assert_eq!((rect(&h, sidebar).width(), h.app.interface.width(Side::Sidebar)), (start + 40.0, Some(start + 40.0)));
    for action in [SashAction::Press, SashAction::Drag(10_000.0), SashAction::Release] {
        send(&mut h, action);
    }
    let editor = f64::from(W) - rect(&h, rail).width() - rect(&h, sidebar).width() - rect(&h, panel).width();
    assert!(editor >= 300.0, "панель заняла место редактора: осталось {editor}");
    send(&mut h, SashAction::Reset);
    assert_eq!((rect(&h, sidebar).width(), h.app.interface.width(Side::Sidebar)), (start, None));
}

#[test]
fn panel_modes_scroll_sideways_instead_of_hiding_under_the_window_controls() {
    use crate::ui::widgets::SashAction;
    let mut h = Harness::new();
    let chrome = h.app.chrome.as_ref().unwrap();
    let (sash, panel) = (chrome.panel_parts.sash, chrome.panel.id());
    let modes: Vec<WidgetId> = chrome.panel_parts.modes.iter().map(|(button, _)| button.id()).collect();
    for action in [SashAction::Press, SashAction::Drag(10_000.0), SashAction::Release] {
        h.act(sash, action);
    }
    h.settle();
    let edge = f64::from(W) - crate::ui::controls_width();
    let last = *modes.last().unwrap();
    let first = rect(&h, modes[0]).center();
    h.scroll((first.x, first.y), 1.0, (0.0, -1000.0));
    h.settle();
    let shown = rect(&h, last);
    assert!(shown.x1 <= edge + 0.5 && shown.x0 >= rect(&h, panel).x0, "последняя кнопка панели не докручивается до вида: {shown:?}, кнопки окна с {edge}");
}
