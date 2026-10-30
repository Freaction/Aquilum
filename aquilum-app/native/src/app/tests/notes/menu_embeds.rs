use masonry::kurbo::{Point, Vec2};

use super::menu::{choose, menu, press};
use super::*;
use crate::app::editor_requests::EditorCommand::*;

fn block_at(h: &Harness, offset: usize) -> (Point, f64) {
    let editor = h.app.note_ids.as_ref().unwrap().body.get(&h.root).unwrap();
    (editor.ctx().window_transform() * Point::new(0.0, editor.inner().top_of(offset).unwrap()), editor.ctx().bounding_box().width())
}

#[test]
fn the_edit_menu_copies_an_image_line_and_table_cells() {
    let text = "начало\n\n![](pic.png)\n\n| яблоко | груша |\n| --- | --- |\n| 1 | 2 |\n<!--q-table:{\"merges\":[],\"cols\":[150,150]}-->\n\nконец";
    let mut h = Harness::new();
    image::RgbaImage::from_pixel(120, 80, image::Rgba([200, 60, 60, 255])).save(h.vault.path().join("pic.png")).unwrap();
    h.open(h.write_note("Вставки.md", text));
    h.focus_editor();
    assert!(h.wait_for(|h| h.app.embeds.as_ref().unwrap().cached_media() == 2), "картинка не загрузилась");
    let (picture, width) = block_at(&h, text.find("![").unwrap());
    press(&mut h, picture + Vec2::new(width / 2.0, 40.0), PointerButton::Secondary, 1);
    assert_eq!(menu(&h).map(|(_, commands)| commands), Some(vec![Cut, Copy, Paste, SelectAll]), "правый щелчок по картинке не выбрал её");
    choose(&mut h, 1);
    assert_eq!(h.copied.as_deref(), Some("![](pic.png)"));

    let apple = block_at(&h, text.find("| яблоко").unwrap()).0 + Vec2::new(60.0, 52.0);
    press(&mut h, apple, PointerButton::Primary, 1);
    assert!(menu(&h).is_none(), "тап по ячейке открыл меню без выделения");
    press(&mut h, apple, PointerButton::Primary, 2);
    choose(&mut h, 1);
    assert_eq!(h.copied.as_deref(), Some("яблоко"));
    h.drag_with((apple.x, apple.y), (apple.x + 150.0, apple.y), TOUCH);
    choose(&mut h, 1);
    assert_eq!(h.copied.as_deref(), Some("яблоко\tгруша"));
}
