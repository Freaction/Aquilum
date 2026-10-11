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
    press(&mut root, Key::Named(NamedKey::End), crate::test_dir::command());
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
    press(&mut root, Key::Named(NamedKey::Backspace), crate::test_dir::command());
    assert_eq!(text(&root), "hello ");
    press(&mut root, Key::Character("a".into()), crate::test_dir::command());
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
    press(&mut root, Key::Named(NamedKey::End), crate::test_dir::command());
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
fn vertical_gesture_over_wide_table_scrolls_the_page() {
    use masonry::core::{PointerEvent, PointerId, PointerInfo, PointerScrollEvent, PointerState, PointerType, ScrollDelta};
    use masonry::dpi::PhysicalPosition;
    let cells: String = (0..6).map(|i| format!(" колонка {i} |")).collect();
    let rule: String = (0..6).map(|_| " --- |").collect();
    let table = format!("|{cells}
|{rule}
|{cells}
<!--q-table:{{\"merges\":[],\"cols\":[300,300,300,300,300,300]}}-->
");
    let body: String = format!("{table}
{}", (0..200).map(|i| format!("Абзац {i}
")).collect::<String>());
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(400, 300),
        scale_factor: 1.0,
        test_font: None,
    };
    let editor = NewWidget::new(TextEditor::new(&body).markdown()).with_tag(TAG);
    const SCROLL: WidgetTag<crate::ui::scroll::ScrollArea<TextEditor>> = WidgetTag::named("страница");
    let scroll = NewWidget::new(crate::ui::scroll::ScrollArea::new(editor)).with_tag(SCROLL);
    let mut root = RenderRoot::new(scroll.erased(), |_| {}, options);
    let _ = root.redraw();
    let mouse = PointerInfo { pointer_id: Some(PointerId::PRIMARY), persistent_device_id: None, pointer_type: PointerType::Mouse };
    let state = PointerState { position: PhysicalPosition::new(200.0, 30.0), count: 1, scale_factor: 1.0, ..PointerState::default() };
    for _ in 0..5 {
        root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: mouse.clone(), delta: ScrollDelta::PixelDelta(PhysicalPosition::new(-1.5, -12.0)), state: state.clone() }));
        let _ = root.redraw();
    }
    let offset = root.get_widget_with_tag(SCROLL).expect("страница").inner().offset();
    assert!(offset >= 50.0, "жест над таблицей не прокрутил страницу: смещение {offset}");
}

#[test]
fn undo_and_redo_group_typing_and_deleting() {
    let mut root = root("");
    typing(&mut root, "привет мир");
    named(&mut root, NamedKey::Backspace);
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), "привет м");
    press(&mut root, Key::Character("z".into()), crate::test_dir::command());
    assert_eq!(text(&root), "привет мир");
    press(&mut root, Key::Character("z".into()), crate::test_dir::command());
    assert_eq!(text(&root), "");
    press(&mut root, Key::Character("y".into()), crate::test_dir::command());
    assert_eq!(text(&root), "привет мир");
    press(&mut root, Key::Character("z".into()), crate::test_dir::command() | Modifiers::SHIFT);
    assert_eq!(text(&root), "привет м");
    typing(&mut root, "!");
    press(&mut root, Key::Character("y".into()), crate::test_dir::command());
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
    press(&mut root, Key::Character("z".into()), crate::test_dir::command());
    press(&mut root, Key::Character("z".into()), crate::test_dir::command());
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
        (super::resolve::Fetch::Ready(super::resolve::Media { image: Some(masonry::peniko::ImageBrush::new(data)), natural: (300, 200), video: false, path: None }), 0)
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
    press(&mut root, Key::Character("a".into()), crate::test_dir::command());
    named(&mut root, NamedKey::Tab);
    assert_eq!(text(&root), "\t- один\n\t- два!\n\tтекст");
    press(&mut root, Key::Character("z".into()), crate::test_dir::command());
    assert_eq!(text(&root), "- один\n- два!\nтекст");
}

#[test]
fn enter_continues_lists_and_quotes() {
    let cases = [
        ("- один", "- один\n- "),
        ("\t- один", "\t- один\n\t- "),
        ("1. один", "1. один\n2. "),
        ("3) один", "1) один\n2) "),
        ("- [ ] дело", "- [ ] дело\n- [ ] "),
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

#[test]
fn shortcuts_toggle_bold_italic_and_strike() {
    let action = if cfg!(target_os = "macos") { Modifiers::META } else { Modifiers::CONTROL };
    let mut root = markdown_root("слово");
    press(&mut root, Key::Character("a".into()), action);
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "**слово**");
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "слово");
    press(&mut root, Key::Character("i".into()), action);
    assert_eq!(text(&root), "*слово*");
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "***слово***");
    press(&mut root, Key::Character("i".into()), action);
    assert_eq!(text(&root), "**слово**");
    press(&mut root, Key::Character("s".into()), action | Modifiers::SHIFT);
    assert_eq!(text(&root), "**~~слово~~**");
    named(&mut root, NamedKey::End);
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "**~~слово~~**", "каретка вне разметки и без выделения ничего не меняет");
    named(&mut root, NamedKey::ArrowLeft);
    named(&mut root, NamedKey::ArrowLeft);
    named(&mut root, NamedKey::ArrowLeft);
    named(&mut root, NamedKey::ArrowLeft);
    named(&mut root, NamedKey::ArrowLeft);
    press(&mut root, Key::Character("b".into()), action);
    assert_eq!(text(&root), "~~слово~~", "каретка внутри жирного снимает его");
}

#[test]
fn pasted_url_becomes_titled_link() {
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(400, 300),
        scale_factor: 1.0,
        test_font: None,
    };
    let mut root = RenderRoot::new(NewWidget::new(TextEditor::new("a_b").markdown()).with_tag(TAG).erased(), |_| {}, options);
    let id = root.get_widget_with_tag(TAG).expect("редактор").id();
    root.focus_on(Some(id));
    let _ = root.redraw();
    press(&mut root, Key::Character("a".into()), crate::test_dir::command());
    root.handle_text_event(TextEvent::ClipboardPaste("https://example.com".into()));
    let _ = root.redraw();
    assert_eq!(text(&root), r"[a\_b](https://example.com)");
    named(&mut root, NamedKey::End);
    root.handle_text_event(TextEvent::ClipboardPaste(" https://x.org/a ".into()));
    let _ = root.redraw();
    let pasted = text(&root);
    let placeholder = pasted.split('[').nth(2).and_then(|s| s.split(']').next()).expect("заглушка").to_owned();
    assert!(placeholder.starts_with("Fetching Title#"));
    root.edit_widget(id, |mut w| TextEditor::resolve_link_title(&mut w.downcast(), &placeholder, "X*Y"));
    assert_eq!(text(&root), r"[a\_b](https://example.com)[X\*Y](https://x.org/a)");
    root.handle_text_event(TextEvent::ClipboardPaste("https://x.org/p.png".into()));
    let _ = root.redraw();
    assert!(text(&root).ends_with(")https://x.org/p.png"));
}

#[test]
fn double_hyphen_and_space_become_dash() {
    let mut root = markdown_root("");
    let id = root.get_widget_with_tag(TAG).expect("редактор").id();
    root.edit_widget(id, |mut w| TextEditor::set_smart(&mut w.downcast(), super::Smart { dashes: false, ..Default::default() }));
    typing(&mut root, "a-- b");
    assert_eq!(text(&root), "a-- b");
    root.edit_widget(id, |mut w| TextEditor::set_smart(&mut w.downcast(), super::Smart::default()));
    typing(&mut root, " c-- d--- e");
    assert_eq!(text(&root), "a-- b c— d--- e");
}

#[test]
fn ordered_lists_renumber_in_one_undo_step() {
    let mut root = markdown_root("1. a\n2. b\n3. c");
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "x");
    assert_eq!(text(&root), "1. a\n2. b\n3. x\n4. c");
    press(&mut root, Key::Character("z".into()), crate::test_dir::command());
    press(&mut root, Key::Character("z".into()), crate::test_dir::command());
    assert_eq!(text(&root), "1. a\n2. b\n3. c");
}

#[test]
fn enter_continues_star_plus_and_task_items() {
    let mut root = markdown_root("* a");
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "b");
    assert_eq!(text(&root), "* a\n* b");
    let mut root = markdown_root("+ [x] a");
    named(&mut root, NamedKey::End);
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "+ [x] a\n+ [ ] ");
    named(&mut root, NamedKey::Enter);
    assert_eq!(text(&root), "+ [x] a\n");
}

#[test]
fn caret_moves_and_selects_inside_image_source() {
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(600, 400),
        scale_factor: 1.0,
        test_font: None,
    };
    let editor = TextEditor::new("текст\n![](a.png)\nпосле").markdown().with_resolver(Some(Arc::new(FakeMedia)));
    let mut root = RenderRoot::new(NewWidget::new(editor).with_tag(TAG).erased(), |_| {}, options);
    let _ = root.redraw();
    let id = root.get_widget_with_tag(TAG).expect("редактор").id();
    root.focus_on(Some(id));
    let _ = root.redraw();
    let caret = |root: &RenderRoot| root.get_widget_with_tag(TAG).unwrap().inner().caret_offset();
    let shown = |root: &RenderRoot| root.get_widget_with_tag(TAG).unwrap().inner().image_source_shown();
    root.edit_widget(id, |mut w| TextEditor::select_byte_range(&mut w.downcast(), "текст\n".len() + 2, "текст\n".len() + 2));
    let _ = root.redraw();
    assert!(!shown(&root), "каретка в строке картинки раскрыла исходник без кнопки кода");
    typing(&mut root, "x");
    named(&mut root, NamedKey::Backspace);
    assert!(shown(&root), "правка в строке картинки не показала исходник");
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

#[test]
fn measuring_above_the_view_does_not_move_the_text() {
    use crate::ui::scroll::ScrollArea;
    const SCROLL: WidgetTag<ScrollArea<TextEditor>> = WidgetTag::named("прокрутка");
    let body: String = (0..3000).map(|i| if i % 7 == 0 { format!("Длинный абзац {i}: {}\n", "слово ".repeat(60)) } else { format!("Абзац {i}\n") }).collect();
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(600, 600),
        scale_factor: 1.0,
        test_font: None,
    };
    let editor = NewWidget::new(TextEditor::new(&body).markdown()).with_tag(TAG);
    let mut root = RenderRoot::new(NewWidget::new(ScrollArea::new(editor)).with_tag(SCROLL).erased(), |_| {}, options);
    let _ = root.redraw();
    let scroll = root.get_widget_with_tag(SCROLL).unwrap().id();
    let set = |root: &mut RenderRoot, offset: f64| {
        root.edit_widget(scroll, |mut w| ScrollArea::set_offset(&mut w.downcast::<ScrollArea<TextEditor>>(), offset));
        for _ in 0..3 {
            let _ = root.redraw();
        }
    };
    let offset = |root: &RenderRoot| root.get_widget_with_tag(SCROLL).unwrap().inner().offset();
    let at = |root: &RenderRoot, y: f64| {
        let w = root.get_widget_with_tag(TAG).unwrap();
        w.inner().offset_at(w.ctx().window_transform().inverse(), masonry::kurbo::Point::new(40.0, y))
    };
    set(&mut root, 1.0e9);
    let mut shifted = false;
    for _ in 0..80 {
        let before = at(&root, 10.0);
        let from = offset(&root);
        set(&mut root, from - 100.0);
        shifted |= (offset(&root) - (from - 100.0)).abs() > 1.0;
        assert_eq!(at(&root, 110.0), before, "текст сдвинулся при прокрутке вверх с {from}");
    }
    assert!(shifted, "оценки выше видимой области ни разу не уточнились");
}
