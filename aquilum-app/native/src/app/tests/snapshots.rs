use super::*;

#[test]
#[ignore]
fn chrome_snapshots() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let note = h.vault.path().join("Заметка.md");
    std::fs::write(&note, "Текст заметки\n\nВторой абзац").unwrap();
    h.app.open(&mut h.root, note);
    let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
    let shot = |h: &mut Harness, name: &str| {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    h.root.handle_text_event(masonry::core::TextEvent::WindowFocusChange(true));
    h.root.focus_on(h.app.editor_id());
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    shot(&mut h, "normal.png");
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    shot(&mut h, "focus.png");
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    shot(&mut h, "after.png");
}

#[test]
#[ignore]
fn tree_toolbar_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let folder = h.vault.path().join("Проекты");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("План.md"), "").unwrap();
    std::fs::write(h.vault.path().join("Идеи.md"), "").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[folder.clone(), h.vault.path().join("Идеи.md")]);
    let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
    let shot = |h: &mut Harness, name: &str| {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    shot(&mut h, "tree-toolbar.png");
    let [_, _, sort, _] = h.app.tree.toolbar_ids().unwrap();
    h.app.open_sort_menu(&mut h.root, sort);
    shot(&mut h, "tree-sort-menu.png");
}

#[test]
#[ignore]
fn hotkeys_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
    let shot = |h: &mut Harness, name: &str| {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    h.app.interface.set_settings_section("shortcuts");
    h.app.open_settings(&mut h.root);
    for _ in 0..30 {
        h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(16)));
        h.deliver();
    }
    shot(&mut h, "settings-shortcuts.png");
    let mut h = Harness::with(None, scale);
    h.app.open_search(&mut h.root);
    shot(&mut h, "search-footer.png");
}

#[test]
#[ignore]
fn markdown_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let note = h.vault.path().join("Разметка.md");
    let body = "# Заголовок первого уровня
## Второй уровень
### Третий
Обычный текст с *курсивом*, **жирным**, ~~зачёркнутым~~ и `кодом в строке`, а ещё [ссылка](https://example.com) и [[Вики|алиас]].

- Пункт списка, достаточно длинный, чтобы перенестись на вторую строку и показать висячий отступ под текстом пункта
	- Вложенный пункт
- [ ] Задача
- [x] Готовая задача
1. Первый
2. Второй

> Цитата в две строки, длинная настолько, чтобы перенестись и показать отступ после полосы
> Вторая строка цитаты

---

```rust
fn main() {
    println!(\"привет\");
}
```
Конец";
    std::fs::write(&note, body).unwrap();
    h.app.open(&mut h.root, note);
    let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
    let shot = |h: &mut Harness, name: &str| {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    shot(&mut h, "markdown.png");
    h.root.handle_text_event(masonry::core::TextEvent::WindowFocusChange(true));
    h.root.focus_on(h.app.editor_id());
    h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    shot(&mut h, "markdown-caret.png");
    h.key(Key::Named(NamedKey::End), crate::test_dir::command());
    shot(&mut h, "markdown-end.png");
}

#[test]
#[ignore]
fn note_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let note = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(Some(&vault), scale);
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    for _ in 0..200 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
    let size = h.root.size();
    let (w, hh) = (size.width as u16, size.height as u16);
    let editor = h.app.editor_id().unwrap();
    let carets = std::env::var("AQ_CARET").unwrap_or_default();
    for (i, needle) in std::iter::once("").chain(carets.split('|').filter(|s| !s.is_empty())).enumerate() {
        if !needle.is_empty() {
            let text = h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text().to_owned();
            let at = text.find(needle).unwrap_or(0);
            h.root.handle_text_event(masonry::core::TextEvent::WindowFocusChange(true));
            h.root.focus_on(Some(editor));
            h.root.edit_widget(editor, |mut w| crate::ui::editor::TextEditor::select_byte_range(&mut w.downcast(), at, at));
        }
        if let Some(dy) = std::env::var("AQ_SCROLL").ok().and_then(|v| v.parse::<f64>().ok()).filter(|_| i > 0) {
            use masonry::core::{PointerScrollEvent, ScrollDelta};
            let state = PointerState { position: PhysicalPosition::new(f64::from(w) / 2.0, f64::from(hh) / 2.0), count: 1, scale_factor: scale, ..PointerState::default() };
            h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -dy)), state }));
            h.deliver();
        }
        if let Some((x, y)) = std::env::var("AQ_HOVER").ok().and_then(|v| v.split_once(',').map(|(x, y)| (x.parse::<f64>().unwrap(), y.parse::<f64>().unwrap()))) {
            let state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: scale, ..PointerState::default() };
            h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
        }
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, w, hh, scale, theme::background()).unwrap();
        std::fs::write(out.join(format!("note-{i}.png")), r.png()).unwrap();
    }
}
