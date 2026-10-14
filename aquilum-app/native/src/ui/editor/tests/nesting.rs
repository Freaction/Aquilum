use masonry::core::keyboard::{Key, NamedKey};
use masonry::core::TextEvent;

use super::support::{markdown_root, named, press, text, TAG};
use crate::test_dir::command;

const LIST: &str = "Макс. вложенность списка:\n\n- 1\n  - 2\n    - 3\n      - 4\n        - 5\n          - 6\n            - 7\n              - 8\n              - 8";

#[test]
fn backspace_erases_the_last_item_one_character_at_a_time() {
    let mut root = markdown_root(LIST);
    press(&mut root, Key::Named(NamedKey::End), command());
    for removed in ["8", " ", "-", " "] {
        let before = text(&root);
        named(&mut root, NamedKey::Backspace);
        let after = text(&root);
        assert_eq!(before.len() - after.len(), removed.len(), "Backspace стёр не один символ: «{before}» → «{after}»");
        assert_eq!(&before[..after.len()], after, "стёрт не последний символ");
    }
    assert!(text(&root).ends_with(&format!("\n              - 8\n{}", " ".repeat(13))), "часть отступа осталась и её можно стирать дальше: {:?}", text(&root));
}

#[test]
fn the_caret_is_painted_on_every_step_of_erasing_the_deepest_item() {
    let mut root = markdown_root(LIST);
    root.handle_text_event(TextEvent::WindowFocusChange(true));
    press(&mut root, Key::Named(NamedKey::End), command());
    let caret_color = crate::ui::theme::current().caret_color.to_rgba8();
    for step in 0..8 {
        let rect = root.get_widget_with_tag(TAG).expect("редактор").inner().caret_rect();
        let (layers, _) = root.redraw();
        let mut renderer = crate::render::Renderer::new(1);
        renderer.render(&layers, 600, 600, 1.0, crate::ui::theme::background()).unwrap();
        let pixels = renderer.frame_pixels();
        let near = |c: u32, t: u8| (c as i32 - i32::from(t)).abs() < 40;
        let painted = (rect.y0.floor() as usize..rect.y1.ceil() as usize)
            .flat_map(|y| (rect.x0.floor() as usize..=rect.x1.ceil() as usize).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let p = pixels[y * 600 + x];
                near((p >> 16) & 0xff, caret_color.r) && near((p >> 8) & 0xff, caret_color.g) && near(p & 0xff, caret_color.b)
            })
            .count();
        assert!(painted > 10, "шаг {step}: каретки не видно ({rect:?})");
        named(&mut root, NamedKey::Backspace);
    }
}
