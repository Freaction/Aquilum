use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers, NamedKey};
use masonry::core::{NewWidget, PointerButton, PointerEvent, PointerScrollEvent, PointerState, ScrollDelta, TextEvent, Widget, WidgetId, WidgetTag};

use crate::test_dir::{MOUSE, moved, pointer_at, primary};
use masonry::dpi::PhysicalPosition;

use crate::test_dir::command;
use crate::ui::editor::TextEditor;
use crate::ui::scroll::ScrollArea;

pub(super) const TAG: WidgetTag<TextEditor> = WidgetTag::named("редактор");
pub(super) const PAGE: WidgetTag<ScrollArea<TextEditor>> = WidgetTag::named("страница");

fn mount(widget: NewWidget<impl Widget + ?Sized>, width: u32, height: u32) -> RenderRoot {
    let options = crate::test_dir::options(width, height, 1.0);
    let mut root = RenderRoot::new(widget, |_| {}, options);
    let _ = root.redraw();
    root
}

pub(super) fn unfocused(editor: TextEditor, width: u32, height: u32) -> RenderRoot {
    mount(NewWidget::new(editor).with_tag(TAG).erased(), width, height)
}

pub(super) fn focused(editor: TextEditor, width: u32, height: u32) -> RenderRoot {
    let mut root = unfocused(editor, width, height);
    root.focus_on(Some(editor_id(&root)));
    let _ = root.redraw();
    root
}

pub(super) fn scrolled(editor: TextEditor, width: u32, height: u32) -> RenderRoot {
    let page = NewWidget::new(ScrollArea::new(NewWidget::new(editor).with_tag(TAG))).with_tag(PAGE);
    mount(page.erased(), width, height)
}

pub(super) fn root(text: &str) -> RenderRoot {
    focused(TextEditor::new(text), 400, 300)
}

pub(super) fn markdown_root(text: &str) -> RenderRoot {
    focused(TextEditor::new(text).markdown(), 600, 600)
}

pub(super) fn editor_id(root: &RenderRoot) -> WidgetId {
    root.get_widget_with_tag(TAG).expect("редактор").id()
}

pub(super) fn text(root: &RenderRoot) -> String {
    root.get_widget_with_tag(TAG).expect("редактор").inner().text()
}

pub(super) fn selected(root: &RenderRoot) -> String {
    root.get_widget_with_tag(TAG).expect("редактор").inner().selected()
}

pub(super) fn press(root: &mut RenderRoot, key: Key, modifiers: Modifiers) {
    crate::test_dir::key(root, key, modifiers);
    let _ = root.redraw();
}

pub(super) fn named(root: &mut RenderRoot, key: NamedKey) {
    press(root, Key::Named(key), Modifiers::empty());
}

pub(super) fn command_key(root: &mut RenderRoot, key: &str) {
    press(root, Key::Character(key.into()), command());
}

pub(super) fn typing(root: &mut RenderRoot, s: &str) {
    for c in s.chars() {
        press(root, Key::Character(c.to_string()), Modifiers::empty());
    }
}

pub(super) fn paste(root: &mut RenderRoot, s: &str) {
    root.handle_text_event(TextEvent::ClipboardPaste(s.into()));
    let _ = root.redraw();
}

fn pointer_move(root: &mut RenderRoot, state: &PointerState) {
    root.handle_pointer_event(moved(state.clone()));
    let _ = root.redraw();
}

fn gesture(root: &mut RenderRoot, from: (f64, f64), to: (f64, f64), count: u8) {
    let mut state = PointerState { count, ..pointer_at(from.0, from.1, 1.0) };
    pointer_move(root, &state);
    state.buttons.insert(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Down(primary(state.clone())));
    state.position = PhysicalPosition::new(to.0, to.1);
    pointer_move(root, &state);
    state.buttons.remove(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Up(primary(state)));
    let _ = root.redraw();
}

pub(super) fn click(root: &mut RenderRoot, x: f64, y: f64) {
    gesture(root, (x, y), (x, y), 1);
}

pub(super) fn drag(root: &mut RenderRoot, from: (f64, f64), to: (f64, f64)) {
    gesture(root, from, to, 1);
}

pub(super) fn tap(root: &mut RenderRoot, x: f64, y: f64, count: u8, jitter: f64) {
    gesture(root, (x, y), (x + jitter, y + jitter / 2.0), count);
}

pub(super) fn wheel(root: &mut RenderRoot, at: (f64, f64), delta: (f64, f64)) {
    let event = PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(delta.0, delta.1)), state: pointer_at(at.0, at.1, 1.0) };
    root.handle_pointer_event(PointerEvent::Scroll(event));
    let _ = root.redraw();
}
