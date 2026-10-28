use masonry::core::WidgetId;
use masonry::kurbo::Insets;

use super::super::adaptive::{at_width, bounds, hidden, resize};
use super::*;
use crate::ui::editor::TextEditor;
use crate::ui::keymap::Command;
use crate::ui::settings::{Binding, SettingsDialog};
use crate::ui::tokens::size;

const LIST: [bool; 4] = [true, false, false, true];
const DETAIL: [bool; 4] = [false, true, true, false];
const BOTH: [bool; 4] = [true, true, false, true];

fn dialog(h: &Harness) -> &SettingsDialog {
    match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog,
        _ => panic!("настройки не открыты"),
    }
}

fn opened(width: f64) -> Harness {
    let mut h = at_width(width);
    h.app.open_settings(&mut h.root);
    h.settle();
    h
}

fn shown(h: &Harness) -> [bool; 4] {
    let d = dialog(h);
    [d.list.id(), d.content.id(), d.back, d.close].map(|id| !hidden(h, id))
}

fn press(h: &mut Harness, id: WidgetId) {
    let center = bounds(h, id).center();
    h.click(center.x, center.y);
    h.settle();
}

fn back(h: &mut Harness) {
    h.app.command(&mut h.root, Command::Back).expect("«Назад» обработан");
    h.deliver();
    h.settle();
}

#[test]
fn phone_settings_open_on_the_section_list_and_return_to_it() {
    let mut h = opened(360.0);
    assert_eq!(shown(&h), LIST, "сначала список разделов");
    let panes = bounds(&h, dialog(&h).panes.id());
    assert_eq!((panes.x0, panes.y0, panes.x1, panes.y1), (0.0, 0.0, 360.0, f64::from(H)), "во весь экран");
    let row = bounds(&h, h.nav(Section::Editor));
    let wide = 360.0 - 2.0 * size::SPACE_8 - size::SCROLLBAR_WIDTH;
    assert_eq!((row.height(), row.width()), (size::SIZE_48, wide), "крупная строка во всю ширину, кроме полосы прокрутки");

    h.open_section(Section::Editor);
    assert_eq!((h.section(), shown(&h)), (Some(Section::Editor), DETAIL), "раздел вместо списка");
    assert_eq!(dialog(&h).title.get(&h.root).unwrap().inner().text(), Section::Editor.label());
    assert_eq!(bounds(&h, dialog(&h).content.id()).width(), 360.0);

    let arrow = dialog(&h).back;
    press(&mut h, arrow);
    assert_eq!((h.section(), shown(&h)), (Some(Section::Editor), LIST), "стрелка в заголовке — к списку");

    h.open_section(Section::History);
    assert_eq!((h.section(), shown(&h)), (Some(Section::History), DETAIL));
    back(&mut h);
    assert_eq!((h.section(), shown(&h)), (Some(Section::History), LIST), "«Назад» из раздела — к списку");
    back(&mut h);
    assert!(h.app.popup.is_none(), "«Назад» из списка закрывает настройки");
}

#[test]
fn phone_settings_close_sub_closes_directly_from_detail() {
    let mut h = opened(360.0);
    h.open_section(Section::Editor);
    let close_sub = dialog(&h).close_sub;
    assert!(!hidden(&h, close_sub), "крестик виден в деталях раздела");
    press(&mut h, close_sub);
    assert!(h.app.popup.is_none(), "нажатие на крестик в разделе сразу закрывает настройки");
}

#[test]
fn phone_settings_stay_inside_the_safe_area() {
    let mut h = opened(360.0);
    h.app.fit_window(&mut h.root, h.app.window, Insets::new(0.0, 24.0, 0.0, 48.0));
    h.settle();
    assert!(bounds(&h, dialog(&h).close).y0 >= 24.0, "заголовок под строкой состояния");
    assert_eq!(bounds(&h, dialog(&h).list.id()).y1, f64::from(H) - 48.0, "список над панелью навигации");
    h.open_section(Section::Editor);
    assert!(bounds(&h, dialog(&h).back).y0 >= 24.0);
    assert_eq!(bounds(&h, dialog(&h).content.id()).y1, f64::from(H) - 48.0);
}

#[test]
fn wide_settings_keep_both_columns() {
    let mut h = opened(1000.0);
    assert_eq!(shown(&h), BOTH);
    let d = dialog(&h);
    let (panes, list, content) = (bounds(&h, d.panes.id()), bounds(&h, d.list.id()), bounds(&h, d.content.id()));
    assert_eq!((panes.x0, panes.y0, panes.x1, panes.y1), (50.0, 45.0, 950.0, 555.0), "карточка по центру, как раньше");
    assert_eq!((list.width(), content.x0), (size::SETTINGS_NAV_WIDTH, list.x1 + size::BORDER_1));
    assert_eq!(bounds(&h, h.nav(Section::Editor)).height(), size::SIZE_32);
    h.open_section(Section::Editor);
    assert_eq!((h.section(), shown(&h)), (Some(Section::Editor), BOTH));
    back(&mut h);
    assert!(h.app.popup.is_none(), "на широком окне «Назад» сразу закрывает настройки");
}

#[test]
fn width_change_keeps_the_section_and_typed_text() {
    let mut h = opened(360.0);
    h.open_section(Section::Mcp);
    let port = h.binding("поле порта", |b| matches!(b, Binding::Number { number, .. } if number.commit));
    let before = h.config().mcp.port;
    press(&mut h, port);
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    h.type_text("1");
    let text = |h: &Harness| h.root.get_widget(port).and_then(|w| w.downcast::<TextEditor>()).unwrap().inner().text();
    let typed = format!("{before}1");
    assert_eq!(text(&h), typed);

    resize(&mut h, 1000.0);
    assert_eq!((h.section(), shown(&h)), (Some(Section::Mcp), BOTH), "две колонки с тем же разделом");
    assert_eq!((text(&h), h.root.focused_widget()), (typed.clone(), Some(port)), "введённое и фокус на месте");

    resize(&mut h, 360.0);
    assert_eq!((h.section(), shown(&h)), (Some(Section::Mcp), DETAIL), "снова раздел, а не список");
    assert_eq!(text(&h), typed);
    assert_eq!(h.config().mcp.port, before, "порт применяется только по Enter");

    h.app.rebuild(&mut h.root);
    h.settle();
    assert_eq!((h.section(), shown(&h)), (Some(Section::Mcp), DETAIL), "пересборка окна не возвращает к списку");
}
