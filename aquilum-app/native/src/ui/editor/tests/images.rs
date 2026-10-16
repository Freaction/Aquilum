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
            data: Blob::new(Arc::new([200u8, 60, 60, 255].repeat(300 * 200))),
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

fn laid(text: &str, caret: Option<usize>) -> crate::ui::editor::document::Document {
    use masonry::parley::{FontContext, LayoutContext};
    let mut doc = crate::ui::editor::document::Document::new(text);
    doc.set_markdown(Some(crate::ui::editor::markdown::Fonts { size: 16.0, mono: crate::ui::theme::editor_font("iA Writer Mono"), ch: 0.0, callouts: true, tags: true }));
    doc.resolver = Some(Arc::new(FakeMedia));
    doc.keep = (0.0, 10_000.0);
    doc.layout(&mut FontContext::default(), &mut LayoutContext::new(), &masonry::core::StyleSet::new(16.0), Some(600.0), None, caret);
    doc
}

fn pictures(decor: &crate::ui::editor::markdown::Decor) -> usize {
    decor.boxes.iter().filter(|b| matches!(b, crate::ui::editor::markdown::Boxed::Image { .. })).count()
}

fn widths(doc: &crate::ui::editor::document::Document) -> Vec<f32> {
    use masonry::parley::PositionedLayoutItem;
    doc.paragraph(0).layout().lines().flat_map(|line| line.items()).filter_map(|item| if let PositionedLayoutItem::InlineBox(b) = item { Some(b.width) } else { None }).collect()
}

#[test]
fn an_image_inside_a_line_is_drawn_in_place_of_its_source() {
    let cases = [("- ![[a.png|120]]", 120.0), ("> ![](a.png)", 300.0), ("текст ![|40](a.png) и дальше", 40.0), ("<img src=\"a.png\" width=\"150\" />", 150.0), ("[![ссылка](a.png)](https://example.com)", 300.0), ("1. ![alt](https://example.com/picture)", 300.0)];
    for (text, width) in cases {
        let doc = laid(text, None);
        assert_eq!((pictures(doc.paragraph(0).decor()), widths(&doc)), (1, vec![width]), "{text}");
        let inside = text.find("png").or_else(|| text.find("picture"));
        assert_eq!(pictures(laid(text, inside).paragraph(0).decor()), 0, "под кареткой нет исходника: {text}");
    }
    let table = laid("| a | b |\n|---|---|\n| ![x](a.png) | t |", None);
    let cells = table.paragraphs().find_map(|p| p.embed).map(|e| e.view.decors().map(pictures).sum::<usize>());
    assert_eq!(cells, Some(1), "картинка в ячейке таблицы");
    assert_eq!(pictures(laid("![[Заметка]] и ![[clip.mp4]]", None).paragraph(0).decor()), 0);
}

#[test]
fn a_linked_image_shows_the_picture_and_its_source_only_under_the_caret() {
    let text = "Изображение-ссылка:\n\n[![Миниатюра](https://via.placeholder.com/120x60.png)](https://example.com)\n\nпосле";
    for (caret, boxes) in [(0, 1), (text.find("\n\nпосле").unwrap(), 0), (text.find("](https://example").unwrap() + 1, 0), (text.find("placeholder").unwrap(), 0)] {
        let mut root = focused(editor_with_images(text), 600, 400);
        let id = super::support::editor_id(&root);
        root.edit_widget(id, |mut w| TextEditor::select_byte_range(&mut w.downcast(), caret, caret));
        let _ = root.redraw();
        assert_eq!(root.get_widget_with_tag(TAG).unwrap().inner().inline_boxes(2), boxes, "каретка на {caret}");
    }
}

struct Later(std::sync::atomic::AtomicBool);

impl Resolver for Later {
    fn dataview(&self, _query: &str) -> (Fetch<Grid>, u64) {
        (Fetch::Pending, 0)
    }
    fn media(&self, source: &str, wiki: bool) -> (Fetch<Media>, u64) {
        match self.0.load(std::sync::atomic::Ordering::Relaxed) {
            true => (FakeMedia.media(source, wiki).0, 7),
            false => (Fetch::Pending, 0),
        }
    }
    fn book(&self, _target: &str) -> (Fetch<Book>, u64) {
        (Fetch::Pending, 0)
    }
    fn request(&self, _request: Request) {}
}

#[test]
fn an_image_that_finishes_loading_replaces_its_source_in_the_line() {
    let later = Arc::new(Later(std::sync::atomic::AtomicBool::new(false)));
    let text = "до\n\n[![Миниатюра](https://via.placeholder.com/120x60.png)](https://example.com)\n\nпосле";
    let mut root = unfocused(TextEditor::new(text).markdown().with_resolver(Some(later.clone())), 600, 400);
    let id = super::support::editor_id(&root);
    let boxes = |root: &RenderRoot| root.get_widget_with_tag(TAG).unwrap().inner().inline_boxes(2);
    assert_eq!(boxes(&root), 0);
    later.0.store(true, std::sync::atomic::Ordering::Relaxed);
    root.edit_widget(id, |mut w| TextEditor::refresh_embeds(&mut w.downcast()));
    let _ = root.redraw();
    assert_eq!(boxes(&root), 1, "догрузившаяся картинка не заменила исходник");
}
