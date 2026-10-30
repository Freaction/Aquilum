use masonry::core::WidgetId;
use masonry::kurbo::Size;

use super::super::adaptive::{at_width, bounds, hidden};
use super::*;
use crate::app::MenuCommand;
use crate::ui::book::{CoverAction, HeroIds};
use crate::ui::menu::MenuChoice;
use crate::ui::tokens::size;
use crate::ui::widgets::Pressed;

const BOOK: &str = "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: wide.png\n---\nтекст";
const PHOTO: &str = "---\ncover: true\nPage_cover: photo.png\n---\nтекст";

fn open(width: f64, text: &str) -> Harness {
    let mut h = at_width(width);
    image::RgbaImage::from_pixel(300, 200, image::Rgba([200, 60, 60, 255])).save(h.vault.path().join("wide.png")).unwrap();
    image::RgbaImage::from_pixel(1200, 600, image::Rgba([60, 120, 200, 255])).save(h.vault.path().join("photo.png")).unwrap();
    let note = h.write_note("Книга.md", text);
    h.open(note);
    h.settle();
    h
}

fn ids(h: &Harness) -> &HeroIds {
    &h.app.note_ids.as_ref().expect("заметка открыта").hero.1
}

fn button(h: &Harness, wanted: CoverAction) -> WidgetId {
    ids(h).actions.iter().find(|&&(_, action)| action == wanted).map(|&(id, _)| id).expect("кнопка действия")
}

fn hover_cover(h: &mut Harness) {
    let center = bounds(h, ids(h).cover.unwrap().id()).center();
    h.pointer_move(center.x, center.y);
    h.settle();
}

fn shot(h: &mut Harness, name: &str, pattern: bool) {
    if std::env::var_os("AQ_OUT").is_some() {
        h.settle();
        h.wait_cover(pattern);
        dump(name, &h.frame(h.app.window.width as u16, H as u16, 1.0));
    }
}

fn book_sizes(width: f64) -> (f64, Size) {
    let mut h = open(width, BOOK);
    shot(&mut h, &format!("cover-book-{width}.png"), true);
    let ids = ids(&h);
    assert_eq!(ids.replace_book.is_some(), width > 600.0, "кнопка загрузки на обложке книги только на ПК, на телефоне — пункт «⋯»");
    (bounds(&h, ids.cover.unwrap().id()).height(), bounds(&h, ids.thumbnail.unwrap().id()).size())
}

#[test]
fn narrow_screen_lowers_covers_and_keeps_the_book_proportions() {
    let (wide_cover, wide_book) = book_sizes(1000.0);
    let (narrow_cover, narrow_book) = book_sizes(360.0);
    assert_eq!(narrow_cover, wide_cover - size::BOOK_COVER_COMPACT_SHRINK, "обложка страницы ниже на 40 px");
    assert_eq!(narrow_book.height, size::BOOK_THUMB_HEIGHT_COMPACT, "обложка книги постоянной высоты");
    assert!(narrow_book.height < wide_book.height, "{narrow_book:?} не меньше {wide_book:?}");
    for book in [wide_book, narrow_book] {
        assert_eq!(book.width, (book.height * 1.5).round(), "ширина по пропорциям картинки 300×200: {book:?}");
    }
}

#[test]
fn narrow_cover_keeps_its_actions_in_one_menu() {
    let mut h = open(360.0, PHOTO);
    let more = ids(&h).more.expect("кнопка «⋯» на узком экране");
    hover_cover(&mut h);
    assert!(!hidden(&h, more) && hidden(&h, button(&h, CoverAction::Replace)), "наведение не открывает панель");
    h.act(more, Pressed);
    let Some(Popup::Menu { layer, commands }) = &h.app.popup else { panic!("меню обложки не открыто") };
    let expected: Vec<MenuCommand> = CoverAction::list(Some(true), false).into_iter().map(MenuCommand::Cover).collect();
    assert!(*commands == expected, "в меню те же действия, что на панели");
    let (layer, reposition) = (*layer, expected.iter().position(|&c| c == MenuCommand::Cover(CoverAction::Reposition)).unwrap());
    let (menu, trigger) = (bounds(&h, layer), bounds(&h, more));
    assert!(menu.x0 >= 0.0 && menu.x1 <= 360.0 && menu.y0 >= trigger.y1, "меню под кнопкой и в окне: {menu:?}");
    shot(&mut h, "cover-menu-360.png", false);
    h.act(layer, MenuChoice::Item(reposition));
    let done = button(&h, CoverAction::Reposition);
    assert!(hidden(&h, more) && !hidden(&h, done), "пока обложка двигается, видна кнопка «Готово»");
    shot(&mut h, "cover-reposition-360.png", false);
    h.act(done, Pressed);
    assert!(!hidden(&h, more) && hidden(&h, done), "после «Готово» снова одна кнопка «⋯»");
    h.act(more, Pressed);
    let Some(Popup::Menu { layer, .. }) = &h.app.popup else { panic!("меню обложки не открыто") };
    let (layer, random) = (*layer, expected.iter().position(|&c| c == MenuCommand::Cover(CoverAction::Random)).unwrap());
    h.act(layer, MenuChoice::Item(random));
    assert!(h.app.text.to_string().contains("Page_cover: pattern:"), "пункт меню делает то же, что кнопка");
}

fn menu(h: &mut Harness) -> (WidgetId, Vec<CoverAction>) {
    let more = ids(h).more.expect("кнопка «⋯» в шапке");
    assert!(!hidden(h, more), "«⋯» видна без наведения");
    h.act(more, Pressed);
    let Some(Popup::Menu { layer, commands }) = &h.app.popup else { panic!("меню обложки не открыто") };
    (*layer, commands.iter().filter_map(|c| if let MenuCommand::Cover(action) = c { Some(*action) } else { None }).collect())
}

fn note_menu(h: &mut Harness) -> (WidgetId, Vec<CoverAction>) {
    h.act(h.app.note_ids.as_ref().unwrap().more, Pressed);
    let Some(Popup::Menu { layer, commands }) = &h.app.popup else { panic!("меню заметки не открыто") };
    let frame = bounds(h, *layer);
    assert!(frame.x0 >= size::SPACE_8 && frame.x1 <= h.app.window.width - size::SPACE_8 + 0.5, "меню «⋯» вышло за край экрана: {frame:?}");
    let covers: Vec<CoverAction> = commands.iter().map_while(|c| if let MenuCommand::Cover(action) = c { Some(*action) } else { None }).collect();
    let mut list = h.root.get_widget(*layer).unwrap();
    while list.children().len() == 1 {
        list = list.children()[0];
    }
    let rule = list.children().get(covers.len()).is_some_and(|child| child.downcast::<crate::ui::widgets::Rule>().is_some());
    assert!(covers.is_empty() || rule, "пункты обложки не отделены разделителем");
    (*layer, covers)
}

#[test]
fn without_a_cover_its_actions_move_to_the_note_menu() {
    use CoverAction::*;
    {
        let mut h = open(360.0, BOOK);
        assert!(note_menu(&mut h).1.is_empty(), "с обложкой её действия — в «⋯» на обложке");
        h.escape();
        let (layer, actions) = menu(&mut h);
        assert_eq!(actions, [Replace, Random, Remove, UploadBook]);
        h.act(layer, MenuChoice::Item(2));
        assert!(ids(&h).more.is_none(), "без обложки «⋯» в шапке не осталось");
        assert_eq!(note_menu(&mut h).1, [Add, Random, UploadBook], "на телефоне обложку книги загружают из меню заметки");
    }
    for width in [360.0, 1000.0] {
        let mut h = open(width, "текст без свойств");
        let hero = h.app.note_ids.as_ref().unwrap().hero.0.id();
        assert!(ids(&h).more.is_none() && bounds(&h, hero).height() == 0.0, "пустая шапка заняла место ({width} px)");
        let (layer, actions) = note_menu(&mut h);
        assert_eq!(actions, [Add, Random], "обложку можно добавить из меню заметки ({width} px)");
        h.act(layer, MenuChoice::Item(1));
        assert!(ids(&h).cover.is_some() && h.app.text.to_string().ends_with("---\nтекст без свойств"), "случайная обложка создаёт шапку");
        assert_eq!(ids(&h).more.is_some(), width < 600.0, "на ПК с обложкой — прежняя панель по наведению");
    }
}

#[test]
fn removed_cover_comes_back_with_undo() {
    use crate::app::note_page::menu::NoteCommand;
    let mut h = open(360.0, PHOTO);
    h.act(ids(&h).more.unwrap(), Pressed);
    let Some(Popup::Menu { layer, commands }) = &h.app.popup else { panic!("меню обложки не открыто") };
    let (layer, remove) = (*layer, commands.iter().position(|&c| c == MenuCommand::Cover(CoverAction::Remove)).unwrap());
    h.act(layer, MenuChoice::Item(remove));
    let removed = h.app.text.to_string();
    assert!(!removed.contains("Page_cover"));
    h.focus_editor();
    h.type_text("!");
    let menu = |h: &mut Harness, command| {
        super::super::wide::choose(h, command);
        h.deliver();
        h.app.text.to_string()
    };
    assert_eq!(menu(&mut h, NoteCommand::Undo), removed, "сначала отменяется ввод");
    assert_eq!(menu(&mut h, NoteCommand::Undo), PHOTO, "затем возвращается обложка");
    assert!(ids(&h).cover.is_some(), "обложка снова на странице");
    assert_eq!(menu(&mut h, NoteCommand::Redo), removed);
    menu(&mut h, NoteCommand::Undo);
    let page = h.app.note.as_ref().unwrap().path.clone();
    h.app.write_fields(&mut h.root, &page, aquilum_core::history::Source::Reading, &[(crate::frontmatter::PAGES, "3/10".into())]);
    let body = h.app.note_ids.as_ref().unwrap().body;
    assert!(!body.get(&h.root).unwrap().inner().can_undo(false), "прогресс чтения не отменяется и сбрасывает старые шаги");
}

#[test]
fn wide_cover_shows_its_actions_on_hover() {
    let mut h = open(1000.0, PHOTO);
    assert!(ids(&h).more.is_none(), "на широком экране кнопки «⋯» нет");
    let replace = button(&h, CoverAction::Replace);
    assert!(hidden(&h, replace));
    hover_cover(&mut h);
    assert!(!hidden(&h, replace), "наведение показывает панель действий");
    shot(&mut h, "cover-hover-1000.png", false);
}
