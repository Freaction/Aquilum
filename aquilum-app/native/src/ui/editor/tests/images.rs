use std::sync::Arc;

use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers, NamedKey};
use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};

use super::support::{click, drag, focused, named, press, selected, text, typing, unfocused, TAG};
use crate::ui::editor::resolve::{Book, Fetch, Grid, Media, Request, Resolver};
use crate::ui::editor::TextEditor;

struct FakeMedia;

impl Resolver for FakeMedia {
    fn dataview(&self, _query: &str) -> (Fetch<Grid>, u64) {
        (Fetch::Pending, 0)
    }
    fn media(&self, _source: &str, _wiki: bool) -> (Fetch<Media>, u64) {
        let data = ImageData {
            data: Blob::new(Arc::new(vec![255u8; 300 * 200 * 4])),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: 300,
            height: 200,
        };
        (Fetch::Ready(Media { image: Some(ImageBrush::new(data)), natural: (300, 200), video: false, path: None }), 0)
    }
    fn book(&self, _target: &str) -> (Fetch<Book>, u64) {
        (Fetch::Pending, 0)
    }
    fn request(&self, _request: Request) {}
}

fn editor_with_images(text: &str) -> TextEditor {
    TextEditor::new(text).markdown().with_resolver(Some(Arc::new(FakeMedia)))
}

fn caret(root: &RenderRoot) -> usize {
    root.get_widget_with_tag(TAG).unwrap().inner().caret_offset()
}

fn source_shown(root: &RenderRoot) -> bool {
    root.get_widget_with_tag(TAG).unwrap().inner().image_source_shown()
}

#[test]
fn images_are_selected_resized_and_deleted() {
    let mut root = unfocused(editor_with_images("![](a.png)\nтекст"), 600, 400);
    drag(&mut root, (430.0, 188.0), (480.0, 188.0));
    assert_eq!(text(&root), "![400](a.png)\nтекст");
    click(&mut root, 300.0, 100.0);
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), "текст");
}

#[test]
fn caret_moves_and_selects_inside_image_source() {
    let mut root = focused(editor_with_images("текст\n![](a.png)\nпосле"), 600, 400);
    let id = super::support::editor_id(&root);
    root.edit_widget(id, |mut w| TextEditor::select_byte_range(&mut w.downcast(), "текст\n".len() + 2, "текст\n".len() + 2));
    let _ = root.redraw();
    assert!(!source_shown(&root), "каретка в строке картинки раскрыла исходник без кнопки кода");
    typing(&mut root, "x");
    named(&mut root, NamedKey::Backspace);
    assert!(source_shown(&root), "правка в строке картинки не показала исходник");
    let start = caret(&root);
    if let Ok(out) = std::env::var("AQ_OUT") {
        let (layers, _) = root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, 600, 400, 1.0, crate::ui::theme::background()).unwrap();
        std::fs::write(std::path::Path::new(&out).join("image-source.png"), r.png()).unwrap();
    }
    named(&mut root, NamedKey::ArrowRight);
    named(&mut root, NamedKey::ArrowRight);
    assert_eq!(caret(&root), start + 2, "стрелка не двигает каретку внутри ссылки на картинку: {}", text(&root));
    press(&mut root, Key::Named(NamedKey::ArrowRight), Modifiers::SHIFT);
    press(&mut root, Key::Named(NamedKey::ArrowRight), Modifiers::SHIFT);
    assert_eq!(selected(&root), "a.", "не выделяется внутри ссылки на картинку");
    named(&mut root, NamedKey::Home);
    assert_eq!(caret(&root), "текст\n".len(), "Home не ведёт в начало строки картинки");
    click(&mut root, 45.0, 244.0);
    let at = caret(&root);
    assert!(at > "текст\n![](".len() && at < "текст\n![](a.png)".len(), "щелчок в исходник не ставит каретку: {at}");
    drag(&mut root, (22.0, 244.0), (60.0, 244.0));
    assert!(selected(&root).contains("a.p"), "мышью не выделяется исходник картинки: {:?}", selected(&root));
}
