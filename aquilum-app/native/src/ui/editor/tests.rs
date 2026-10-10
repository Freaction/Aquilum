use std::sync::Arc;

use masonry::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
use masonry::core::keyboard::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey};
use masonry::core::{Ime, NewWidget, TextEvent, WidgetTag};
use masonry::dpi::PhysicalSize;
use masonry::theme::default_property_set;

use super::TextEditor;

const TAG: WidgetTag<TextEditor> = WidgetTag::named("редактор");

fn markdown_root(text: &str) -> RenderRoot {
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(600, 600),
        scale_factor: 1.0,
        test_font: None,
    };
    let mut root = RenderRoot::new(NewWidget::new(TextEditor::new(text).markdown()).with_tag(TAG).erased(), |_| {}, options);
    let _ = root.redraw();
    let id = root.get_widget_with_tag(TAG).expect("редактор").id();
    root.focus_on(Some(id));
    let _ = root.redraw();
    root
}

fn click(root: &mut RenderRoot, x: f64, y: f64) {
    use masonry::core::{PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState, PointerType, PointerUpdate};
    use masonry::dpi::PhysicalPosition;
    let mouse = PointerInfo { pointer_id: Some(PointerId::PRIMARY), persistent_device_id: None, pointer_type: PointerType::Mouse };
    let mut state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
    root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: mouse, current: state.clone(), coalesced: vec![], predicted: vec![] }));
    state.buttons.insert(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state: state.clone() }));
    state.buttons.remove(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state }));
    let _ = root.redraw();
}

fn root(text: &str) -> RenderRoot {
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(400, 300),
        scale_factor: 1.0,
        test_font: None,
    };
    let mut root = RenderRoot::new(NewWidget::new(TextEditor::new(text)).with_tag(TAG).erased(), |_| {}, options);
    let _ = root.redraw();
    let id = root.get_widget_with_tag(TAG).expect("редактор").id();
    root.focus_on(Some(id));
    let _ = root.redraw();
    root
}

fn text(root: &RenderRoot) -> String {
    root.get_widget_with_tag(TAG).expect("редактор").inner().text().to_owned()
}

fn press(root: &mut RenderRoot, key: Key, modifiers: Modifiers) {
    for state in [KeyState::Down, KeyState::Up] {
        let event = KeyboardEvent { state, key: key.clone(), code: Code::Unidentified, location: Location::Standard, modifiers, repeat: false, is_composing: false };
        root.handle_text_event(TextEvent::Keyboard(event));
    }
    let _ = root.redraw();
}

fn named(root: &mut RenderRoot, key: NamedKey) {
    press(root, Key::Named(key), Modifiers::empty());
}

fn typing(root: &mut RenderRoot, s: &str) {
    for c in s.chars() {
        press(root, Key::Character(c.to_string()), Modifiers::empty());
    }
}

#[test]
fn typing_and_enter_build_paragraphs() {
    let mut root = root("");
    typing(&mut root, "привет");
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "мир");
    assert_eq!(text(&root), "привет\nмир");
}

#[test]
fn backspace_at_paragraph_start_joins_paragraphs() {
    let mut root = root("один\nдва");
    press(&mut root, Key::Named(NamedKey::End), Modifiers::CONTROL);
    for _ in 0..3 {
        named(&mut root, NamedKey::ArrowLeft);
    }
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), "одиндва");
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), "одинва");
}

#[test]
fn arrows_cross_paragraphs_and_keep_column() {
    let mut root = root("abcdef\nab\nabcdef");
    typing(&mut root, "");
    for _ in 0..5 {
        named(&mut root, NamedKey::ArrowRight);
    }
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::ArrowDown);
    typing(&mut root, "X");
    assert_eq!(text(&root), "abcdef\nab\nabcdeXf");
    named(&mut root, NamedKey::Home);
    named(&mut root, NamedKey::ArrowLeft);
    typing(&mut root, "Y");
    assert_eq!(text(&root), "abcdef\nabY\nabcdeXf");
}

#[test]
fn selection_is_replaced_by_typing() {
    let mut root = root("hello world");
    named(&mut root, NamedKey::End);
    for _ in 0..5 {
        press(&mut root, Key::Named(NamedKey::ArrowLeft), Modifiers::SHIFT);
    }
    typing(&mut root, "there");
    assert_eq!(text(&root), "hello there");
    press(&mut root, Key::Named(NamedKey::Backspace), Modifiers::CONTROL);
    assert_eq!(text(&root), "hello ");
    press(&mut root, Key::Character("a".into()), Modifiers::CONTROL);
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), "");
}

#[test]
fn paste_and_ime_insert_text() {
    let mut root = root("ab");
    named(&mut root, NamedKey::ArrowRight);
    root.handle_text_event(TextEvent::ClipboardPaste("1\n2".into()));
    let _ = root.redraw();
    assert_eq!(text(&root), "a1\n2b");
    root.handle_text_event(TextEvent::Ime(Ime::Preedit("ко".into(), Some((4, 4)))));
    let _ = root.redraw();
    assert_eq!(text(&root), "a1\n2b");
    root.handle_text_event(TextEvent::Ime(Ime::Commit("кот".into())));
    let _ = root.redraw();
    assert_eq!(text(&root), "a1\n2котb");
}

#[test]
fn edit_relayouts_only_its_paragraph() {
    let body: String = (0..2000).map(|i| format!("Абзац {i}: текст заметки, который переносится на несколько строк в колонке редактора.\n")).collect();
    let mut root = root(&body);
    press(&mut root, Key::Named(NamedKey::End), Modifiers::CONTROL);
    let started = std::time::Instant::now();
    typing(&mut root, "abcdefghij");
    let per_key = started.elapsed() / 10;
    assert!(text(&root).ends_with("abcdefghij"));
    assert!(per_key < std::time::Duration::from_millis(5), "нажатие в большом документе: {per_key:?}");
}

#[test]
fn scrolling_far_repaints_visible_paragraphs() {
    use masonry::core::{PointerEvent, PointerId, PointerInfo, PointerScrollEvent, PointerState, PointerType, ScrollDelta};
    use masonry::dpi::PhysicalPosition;
    let body: String = (0..400).map(|i| format!("Абзац {i}\n")).collect();
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(400, 300),
        scale_factor: 1.0,
        test_font: None,
    };
    let editor = NewWidget::new(TextEditor::new(&body)).with_tag(TAG);
    let scroll = crate::ui::scroll::ScrollArea::new(editor);
    let mut root = RenderRoot::new(NewWidget::new(scroll).erased(), |_| {}, options);
    let _ = root.redraw();
    let mouse = PointerInfo { pointer_id: Some(PointerId::PRIMARY), persistent_device_id: None, pointer_type: PointerType::Mouse };
    let state = PointerState { position: PhysicalPosition::new(200.0, 150.0), count: 1, scale_factor: 1.0, ..PointerState::default() };
    root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: mouse, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -5000.0)), state }));
    let _ = root.redraw();
    let painted = root.get_widget_with_tag(TAG).expect("редактор").inner().painted;
    assert!(painted.0 <= 5000.0 && painted.1 >= 5300.0, "нарисовано {painted:?}, а видно 5000..5300");
}

#[test]
fn undo_and_redo_group_typing_and_deleting() {
    let mut root = root("");
    typing(&mut root, "привет мир");
    named(&mut root, NamedKey::Backspace);
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), "привет м");
    press(&mut root, Key::Character("z".into()), Modifiers::CONTROL);
    assert_eq!(text(&root), "привет мир");
    press(&mut root, Key::Character("z".into()), Modifiers::CONTROL);
    assert_eq!(text(&root), "");
    press(&mut root, Key::Character("y".into()), Modifiers::CONTROL);
    assert_eq!(text(&root), "привет мир");
    press(&mut root, Key::Character("z".into()), Modifiers::CONTROL | Modifiers::SHIFT);
    assert_eq!(text(&root), "привет м");
    typing(&mut root, "!");
    press(&mut root, Key::Character("y".into()), Modifiers::CONTROL);
    assert_eq!(text(&root), "привет м!");
}

#[test]
fn table_cells_are_edited_in_place() {
    let mut root = markdown_root("| a | b |
| --- | --- |
| 1 | 2 |
после");
    click(&mut root, 60.0, 50.0);
    typing(&mut root, "Z");
    assert!(text(&root).starts_with("| aZ | b |"), "{}", text(&root));
    named(&mut root, NamedKey::Tab);
    typing(&mut root, "Y");
    assert!(text(&root).starts_with("| aZ | bY |"), "{}", text(&root));
    named(&mut root, NamedKey::Tab);
    named(&mut root, NamedKey::Tab);
    named(&mut root, NamedKey::Tab);
    typing(&mut root, "Q");
    assert!(text(&root).contains("| Q | <!--q-empty--> |"), "{}", text(&root));
    named(&mut root, NamedKey::Escape);
    typing(&mut root, "!");
    assert!(text(&root).ends_with("!после"), "{}", text(&root));
    press(&mut root, Key::Character("z".into()), Modifiers::CONTROL);
    press(&mut root, Key::Character("z".into()), Modifiers::CONTROL);
    assert!(!text(&root).contains('Q'), "{}", text(&root));
}

struct FakeMedia;

impl super::resolve::Resolver for FakeMedia {
    fn dataview(&self, _query: &str) -> (super::resolve::Fetch<super::resolve::Grid>, u64) {
        (super::resolve::Fetch::Pending, 0)
    }
    fn media(&self, _source: &str, _wiki: bool) -> (super::resolve::Fetch<super::resolve::Media>, u64) {
        let data = masonry::peniko::ImageData {
            data: masonry::peniko::Blob::new(Arc::new(vec![255u8; 300 * 200 * 4])),
            format: masonry::peniko::ImageFormat::Rgba8,
            alpha_type: masonry::peniko::ImageAlphaType::Alpha,
            width: 300,
            height: 200,
        };
        (super::resolve::Fetch::Ready(super::resolve::Media { image: Some(masonry::peniko::ImageBrush::new(data)), video: false, path: None }), 0)
    }
    fn book(&self, _target: &str) -> (super::resolve::Fetch<super::resolve::Book>, u64) {
        (super::resolve::Fetch::Pending, 0)
    }
    fn request(&self, _request: super::resolve::Request) {}
}

fn drag(root: &mut RenderRoot, from: (f64, f64), to: (f64, f64)) {
    use masonry::core::{PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState, PointerType, PointerUpdate};
    use masonry::dpi::PhysicalPosition;
    let mouse = PointerInfo { pointer_id: Some(PointerId::PRIMARY), persistent_device_id: None, pointer_type: PointerType::Mouse };
    let mut state = PointerState { position: PhysicalPosition::new(from.0, from.1), count: 1, scale_factor: 1.0, ..PointerState::default() };
    root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: mouse, current: state.clone(), coalesced: vec![], predicted: vec![] }));
    let _ = root.redraw();
    state.buttons.insert(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state: state.clone() }));
    state.position = PhysicalPosition::new(to.0, to.1);
    root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: mouse, current: state.clone(), coalesced: vec![], predicted: vec![] }));
    let _ = root.redraw();
    state.buttons.remove(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state }));
    let _ = root.redraw();
}

#[test]
fn images_are_selected_resized_and_deleted() {
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(600, 400),
        scale_factor: 1.0,
        test_font: None,
    };
    let editor = TextEditor::new("![](a.png)
текст").markdown().with_resolver(Some(Arc::new(FakeMedia)));
    let mut root = RenderRoot::new(NewWidget::new(editor).with_tag(TAG).erased(), |_| {}, options);
    let _ = root.redraw();
    drag(&mut root, (430.0, 188.0), (480.0, 188.0));
    assert_eq!(text(&root), "![400](a.png)
текст");
    click(&mut root, 300.0, 100.0);
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), "текст");
}

fn tap(root: &mut RenderRoot, x: f64, y: f64, count: u8, jitter: f64) {
    use masonry::core::{PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState, PointerType, PointerUpdate};
    use masonry::dpi::PhysicalPosition;
    let mouse = PointerInfo { pointer_id: Some(PointerId::PRIMARY), persistent_device_id: None, pointer_type: PointerType::Mouse };
    let mut state = PointerState { position: PhysicalPosition::new(x, y), count, scale_factor: 1.0, ..PointerState::default() };
    root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: mouse, current: state.clone(), coalesced: vec![], predicted: vec![] }));
    state.buttons.insert(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state: state.clone() }));
    let _ = root.redraw();
    state.position = PhysicalPosition::new(x + jitter, y + jitter / 2.0);
    root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: mouse, current: state.clone(), coalesced: vec![], predicted: vec![] }));
    state.buttons.remove(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state }));
    let _ = root.redraw();
}

fn selected(root: &RenderRoot) -> String {
    root.get_widget_with_tag(TAG).expect("редактор").inner().selected()
}

#[test]
fn double_tap_selects_word_everywhere() {
    for (source, word) in [("- & Буллит один\n\nтекст", "Буллит"), ("1. Буллит один\n\nтекст", "Буллит"), ("> [!quote] Буллит один\n\nтекст", "Буллит"), ("Буллит один\n\nтекст", "Буллит"), ("\t- Буллит один\n\nтекст", "Буллит")] {
        let mut root = markdown_root(source);
        let x = (0..400).step_by(4).map(f64::from).find(|x| {
            tap(&mut root, *x, 10.0, 2, 0.0);
            selected(&root) == word
        });
        let Some(x) = x else { panic!("{source:?}: слово не выделяется двойным щелчком") };
        tap(&mut root, x, 10.0, 1, 0.0);
        tap(&mut root, x, 10.0, 2, 3.0);
        assert_eq!(selected(&root), word, "{source:?}: после касания с дрожанием");
    }
}

#[test]
fn tab_indents_and_shift_tab_outdents_lines() {
    let mut root = markdown_root("- один\n- два\nтекст");
    click(&mut root, 590.0, 30.0);
    named(&mut root, NamedKey::Tab);
    assert_eq!(text(&root), "- один\n\t- два\nтекст");
    typing(&mut root, "!");
    assert_eq!(text(&root), "- один\n\t- два!\nтекст");
    press(&mut root, Key::Named(NamedKey::Tab), Modifiers::SHIFT);
    assert_eq!(text(&root), "- один\n- два!\nтекст");
    press(&mut root, Key::Character("a".into()), Modifiers::CONTROL);
    named(&mut root, NamedKey::Tab);
    assert_eq!(text(&root), "\t- один\n\t- два!\n\tтекст");
    press(&mut root, Key::Character("z".into()), Modifiers::CONTROL);
    assert_eq!(text(&root), "- один\n- два!\nтекст");
}

#[test]
fn enter_continues_lists_and_quotes() {
    let cases = [
        ("- один", "- один\n- "),
        ("\t- один", "\t- один\n\t- "),
        ("1. один", "1. один\n2. "),
        ("3) один", "3) один\n4) "),
        ("- [ ] дело", "- [ ] дело\n- "),
        ("> цитата", "> цитата\n> "),
        ("текст", "текст\n"),
    ];
    for (source, expected) in cases {
        let mut root = markdown_root(source);
        click(&mut root, 590.0, 10.0);
        named(&mut root, NamedKey::Enter);
        assert_eq!(text(&root), expected, "{source:?}");
    }
    let mut root = markdown_root("\t- один");
    click(&mut root, 590.0, 10.0);
    named(&mut root, NamedKey::Enter);
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "\t- один\n- ");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "\t- один\n");
    let mut root = markdown_root("- один");
    click(&mut root, 590.0, 10.0);
    press(&mut root, Key::Named(NamedKey::Enter), Modifiers::SHIFT);
    typing(&mut root, "x");
    assert_eq!(text(&root), "- один\n  x");
}
