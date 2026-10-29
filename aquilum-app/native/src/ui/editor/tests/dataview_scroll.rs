use std::sync::Arc;

use masonry::app::RenderRoot;
use masonry::core::keyboard::Modifiers;
use masonry::core::pointer::PointerButtons;
use masonry::core::{PointerButton, PointerButtonEvent, PointerEvent, PointerInfo, PointerScrollEvent, PointerState, PointerUpdate, ScrollDelta};
use masonry::dpi::PhysicalPosition;
use masonry::kurbo::{Axis, Point, Vec2};

use super::support::{TAG, unfocused};
use crate::test_dir::{MOUSE, TOUCH, pointer_at};
use crate::ui::editor::TextEditor;
use crate::ui::editor::resolve::{Book, Fetch, Grid, Media, Part, Request, Resolver, Shape};
use crate::ui::scroll::bar::{Scrollbar, Span};

struct Wide;

impl Resolver for Wide {
    fn dataview(&self, _query: &str) -> (Fetch<Grid>, u64) {
        let cell = |text: &str| vec![Part::Text(text.into())];
        let row = |n: usize| (0..3).map(|c| cell(&format!("значение {n} столбца {c} с длинным текстом"))).collect::<Vec<_>>();
        let grid = Grid { shape: Shape::Table, title: None, columns: vec!["Первый".into(), "Второй".into(), "Третий".into()], rows: (0..3).map(row).collect::<Vec<_>>().into() };
        (Fetch::Ready(grid), 1)
    }
    fn media(&self, _source: &str, _wiki: bool) -> (Fetch<Media>, u64) {
        (Fetch::Pending, 0)
    }
    fn book(&self, _target: &str) -> (Fetch<Book>, u64) {
        (Fetch::Pending, 0)
    }
    fn request(&self, _request: Request) {}
}

fn card(width: u32) -> RenderRoot {
    let query = "TABLE WITHOUT ID file.link AS \"Название\", author AS \"Автор\", rating AS \"Оценка\", status FROM \"Книги\" SORT rating DESC";
    let editor = TextEditor::new(&format!("```dataview\n{query}\n```\n\nниже\n")).markdown().with_resolver(Some(Arc::new(Wide)));
    let mut root = unfocused(editor, width, 400);
    let _ = root.redraw();
    root
}

fn span(root: &RenderRoot) -> Span {
    root.get_widget_with_tag(TAG).unwrap().inner().card_scrollbar().expect("таблица датавью не прокручивается")
}

fn state(at: Point, buttons: PointerButtons, modifiers: Modifiers) -> PointerState {
    PointerState { buttons, modifiers, ..pointer_at(at.x, at.y, 1.0) }
}

fn scroll(root: &mut RenderRoot, pointer: PointerInfo, delta: (f64, f64), modifiers: Modifiers) {
    let at = span(root).track.center() - Vec2::new(0.0, 30.0);
    let delta = ScrollDelta::PixelDelta(PhysicalPosition::new(delta.0, delta.1));
    root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer, delta, state: state(at, PointerButtons::new(), modifiers) }));
    let _ = root.redraw();
}

fn mouse(root: &mut RenderRoot, at: Point, kind: &str) {
    let button = |buttons| PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: state(at, buttons, Modifiers::empty()) };
    let moved = |buttons| PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state(at, buttons, Modifiers::empty()), coalesced: vec![], predicted: vec![] });
    match kind {
        "down" => root.handle_pointer_event(PointerEvent::Down(button(PointerButton::Primary.into()))),
        "hover" => root.handle_pointer_event(moved(PointerButtons::new())),
        "move" => root.handle_pointer_event(moved(PointerButton::Primary.into())),
        _ => root.handle_pointer_event(PointerEvent::Up(button(PointerButtons::new()))),
    };
    let _ = root.redraw();
}

#[test]
fn a_dataview_table_in_a_narrow_column_keeps_readable_columns_and_scrolls() {
    let narrow = span(&card(340));
    assert!(narrow.max() > 120.0, "таблица втиснулась в узкий экран вместо прокрутки: запас {}", narrow.max());
    let wide = card(1400);
    assert!(wide.get_widget_with_tag(TAG).unwrap().inner().card_scrollbar().is_none(), "на широком экране таблице нечего прокручивать");
}

#[test]
fn wheel_shift_wheel_and_a_sideways_swipe_move_the_table_but_a_vertical_swipe_does_not() {
    let mut root = card(340);
    let max = span(&root).max();
    scroll(&mut root, MOUSE, (-50.0, 0.0), Modifiers::empty());
    assert_eq!(span(&root).offset, 50.0, "горизонтальное колесо не сдвинуло таблицу");
    scroll(&mut root, MOUSE, (0.0, -50.0), Modifiers::SHIFT);
    assert_eq!(span(&root).offset, 100.0, "Shift и колесо не сдвинули таблицу");
    scroll(&mut root, TOUCH, (30.0, -4.0), Modifiers::empty());
    assert_eq!(span(&root).offset, 70.0, "свайп вбок пальцем не вернул таблицу");
    scroll(&mut root, TOUCH, (-3.0, -60.0), Modifiers::empty());
    assert_eq!(span(&root).offset, 70.0, "вертикальный свайп сдвинул таблицу вбок");
    scroll(&mut root, MOUSE, (-5000.0, 0.0), Modifiers::empty());
    assert_eq!(span(&root).offset, max, "прокрутка вышла за край таблицы");
}

#[test]
fn the_scrollbar_is_dragged_after_hovering_it_and_keeps_the_card_in_place() {
    let mut root = card(340);
    let start = span(&root);
    let thumb = Scrollbar::new(Axis::Horizontal).thumb(&start);
    mouse(&mut root, thumb.center(), "hover");
    mouse(&mut root, thumb.center(), "down");
    mouse(&mut root, thumb.center() + Vec2::new(40.0, 8.0), "move");
    let dragged = span(&root);
    assert!(dragged.offset > 0.0 && dragged.offset < start.max(), "перетаскивание полосы не сдвинуло таблицу: {}", dragged.offset);
    assert_eq!(dragged.track, start.track, "таблица сдвинула строки редактора");
    mouse(&mut root, thumb.center() + Vec2::new(40.0, 8.0), "up");
    mouse(&mut root, thumb.center() + Vec2::new(200.0, 0.0), "move");
    assert_eq!(span(&root).offset, dragged.offset, "после отпускания полоса продолжила тянуться");
}
