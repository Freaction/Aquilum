use super::*;
use crate::ui::settings::Binding;
use crate::ui::settings::schema::Action;
use masonry::core::CursorIcon;

#[test]
fn a_secondary_button_shows_a_hand() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    h.open_section(Section::System);
    let author = h.binding("кнопка автора", |b| matches!(b, Binding::Button(Action::OpenUrl(_))));
    let center = h.root.get_widget(author).unwrap().ctx().bounding_box().center();

    h.pointer_move(center.x, center.y);

    assert_eq!(h.root.cursor_icon(), CursorIcon::Pointer);
}
