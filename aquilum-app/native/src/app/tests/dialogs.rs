use super::*;

#[test]
fn settings_dialog_applies_changes() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    assert_eq!(h.section(), Some(Section::Ui));

    h.click(807.0, 236.0);
    assert_eq!(h.config().ui.theme, "light");
    assert_eq!(h.section(), Some(Section::Ui));

    let outcome = h.click(893.0, 173.0);
    assert!(outcome.rescale);
    assert_eq!(h.app.interface.scale(), 1.05);

    h.click(115.0, 398.0);
    assert_eq!(h.section(), Some(Section::History));
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    h.click(816.0, 240.0);
    assert!(!h.menu_open());
    assert_eq!(h.config().history.retention_days, 30);

    h.click(115.0, 330.0);
    assert_eq!(h.section(), Some(Section::Analysis));
    assert!(h.config().analysis.enable_bm25f);
    h.click(891.0, 173.0);
    assert!(!h.config().analysis.enable_bm25f);

    h.escape();
    assert_eq!(h.section(), None);
    h.click(242.0, 578.0);
    assert_eq!(h.section(), Some(Section::Analysis));
}

#[test]
fn trash_section_restores_deleted_note() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    aquilum_core::files::gate::trash(&h.app.workspace.core, &vault, &vault.join("a.md")).unwrap();
    assert!(!vault.join("a.md").exists());
    h.click(242.0, 578.0);
    h.click(115.0, 432.0);
    assert_eq!(h.section(), Some(Section::Trash));
    let restore = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| match b {
            crate::ui::settings::Binding::Button(crate::ui::settings::schema::Action::Restore(_)) => Some(*id),
            _ => None,
        }),
        _ => None,
    }
    .expect("в корзине есть заметка");
    h.app.on_action(&mut h.root, restore, masonry::core::ErasedAction::from(Box::new(crate::ui::widgets::Pressed)), Size::new(1000.0, 600.0));
    assert!(vault.join("a.md").exists());
}

#[test]
fn primary_color_field_updates_settings() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    assert_eq!(h.section(), Some(Section::Ui));
    let field = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| match b {
            crate::ui::settings::Binding::Text { value, .. } if value.starts_with('#') => Some(*id),
            _ => None,
        }),
        _ => None,
    }
    .expect("поле основного цвета");
    let change = |text: &str| masonry::core::ErasedAction::from(Box::new(crate::ui::editor::TextAction::Changed(text.into())));
    h.app.on_action(&mut h.root, field, change("#E05A"), Size::new(1000.0, 600.0));
    assert_eq!(h.config().ui.primary_color, AppConfig::default().ui.primary_color);
    h.app.on_action(&mut h.root, field, change("#E05A2B"), Size::new(1000.0, 600.0));
    assert_eq!(h.config().ui.primary_color, "#e05a2b");
    assert_eq!(h.section(), Some(Section::Ui));
    assert_eq!(theme::current().bg_accent.to_rgba8().to_u8_array(), [0xe0, 0x5a, 0x2b, 0xff]);
}

#[test]
fn tabs_open_new_select_and_close() {
    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.app.open(&mut h.root, note.clone());
    assert_eq!(h.app.tabs.tabs().len(), 1);
    assert!(h.app.note.is_some());

    let plus = 1000.0 - 320.0 - 44.0 - 6.0 - 16.0;
    let outcome = h.click(plus, 20.0);
    assert!(outcome.title);
    assert_eq!(h.app.tabs.tabs().len(), 2);
    assert!(h.app.tabs.active().path.is_none());
    assert!(h.app.note.is_none());

    let first = h.app.tabs.tabs()[0].id;
    let select = masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::Select(first)));
    let strip = h.app.chrome.as_ref().unwrap().tab_strip.id();
    h.app.on_action(&mut h.root, strip, select, Size::new(1000.0, 600.0));
    assert_eq!(h.app.tabs.active_id(), first);
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&note)));

    let second = h.app.tabs.tabs()[1].id;
    let close = masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::Close(second)));
    h.app.on_action(&mut h.root, strip, close, Size::new(1000.0, 600.0));
    assert_eq!(h.app.tabs.tabs().len(), 1);

    let outcome = h.app.on_action(
        &mut h.root,
        strip,
        masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::CloseWindow)),
        Size::new(1000.0, 600.0),
    );
    assert_eq!(outcome.window, Some(crate::app::WindowCommand::Close));
}

#[test]
fn primary_color_typed_into_field() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    let field = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| match b {
            crate::ui::settings::Binding::Text { value, .. } if value.starts_with('#') => Some(*id),
            _ => None,
        }),
        _ => None,
    }
    .expect("поле основного цвета");
    h.root.focus_on(Some(field));
    h.key(Key::Character("a".into()), Modifiers::CONTROL);
    for ch in "#e05a2b".chars() {
        h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
    }
    assert_eq!(h.config().ui.primary_color, "#e05a2b");
    assert_eq!(theme::current().bg_accent.to_rgba8().to_u8_array(), [0xe0, 0x5a, 0x2b, 0xff]);
}

impl Harness {
    pub(super) fn drag(&mut self, from: (f64, f64), to: (f64, f64)) -> Outcome {
        let state = |x: f64, y: f64, pressed: bool| {
            let mut state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
            if pressed {
                state.buttons.insert(PointerButton::Primary);
            }
            state
        };
        let moved = |s: PointerState| PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: s, coalesced: vec![], predicted: vec![] });
        self.root.handle_pointer_event(moved(state(from.0, from.1, false)));
        self.root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent {
            pointer: MOUSE,
            button: Some(PointerButton::Primary),
            state: state(from.0, from.1, true),
        }));
        for step in 1..=10 {
            let k = f64::from(step) / 10.0;
            self.root.handle_pointer_event(moved(state(from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k, true)));
        }
        self.root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent {
            pointer: MOUSE,
            button: Some(PointerButton::Primary),
            state: state(to.0, to.1, false),
        }));
        self.deliver()
    }

    pub(super) fn tab_paths(&self) -> Vec<Option<String>> {
        self.app
            .tabs
            .tabs()
            .iter()
            .map(|t| t.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()))
            .collect()
    }
}

#[test]
fn ctrl_shift_click_opens_note_in_new_tab_and_tabs_reorder_by_drag() {
    let mut h = Harness::new();
    std::fs::write(h.vault.path().join("b.md"), "").unwrap();
    let vault = h.vault.path().to_path_buf();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("b.md")]);
    h.click(120.0, 65.0);
    assert_eq!(h.tab_paths(), [Some("a".to_owned())]);
    h.click_with(120.0, 99.0, Modifiers::CONTROL | Modifiers::SHIFT);
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("b".to_owned())]);
    assert_eq!(h.app.tabs.active().path.as_deref(), Some(vault.join("b.md").as_path()));

    let first = 266.0 + 40.0;
    h.drag((first, 20.0), (first + 200.0, 20.0));
    assert_eq!(h.tab_paths(), [Some("b".to_owned()), Some("a".to_owned())]);
}

#[test]
fn tab_menu_renames_note_in_tab() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    h.app.open(&mut h.root, vault.join("a.md"));
    let id = h.app.tabs.active_id();
    let strip = h.app.chrome.as_ref().unwrap().tab_strip.id();
    let menu = masonry::core::ErasedAction::from(Box::new(crate::ui::titlebar::TitlebarAction::Menu(id, masonry::kurbo::Point::new(300.0, 20.0))));
    h.app.on_action(&mut h.root, strip, menu, Size::new(1000.0, 600.0));
    let layer = match &h.app.popup {
        Some(Popup::FileMenu { layer, tab: Some(_), .. }) => *layer,
        _ => panic!("меню вкладки"),
    };
    let rename = masonry::core::ErasedAction::from(Box::new(crate::ui::menu::MenuChoice::Item(0)));
    h.app.on_action(&mut h.root, layer, rename, Size::new(1000.0, 600.0));
    let field = h.app.rename_field.expect("поле переименования во вкладке");
    assert_eq!(h.root.focused_widget(), Some(field.id()));
    let entered = masonry::core::ErasedAction::from(Box::new(crate::ui::editor::TextAction::Entered("Новое имя".into())));
    h.app.on_action(&mut h.root, field.id(), entered, Size::new(1000.0, 600.0));
    assert!(h.app.rename_field.is_none());
    let renamed = vault.join("Новое имя.md");
    for _ in 0..250 {
        if renamed.exists() && !vault.join("a.md").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(renamed.exists());
    assert!(!vault.join("a.md").exists());
}

#[test]
fn arrows_step_number_fields_like_tauri() {
    let mut h = Harness::new();
    h.app.open_settings(&mut h.root);
    let _ = h.root.redraw();
    let field = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog
            .bindings
            .iter()
            .find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Number { number, .. } if number.blank).then_some(*id))
            .expect("поле размера шрифта"),
        _ => panic!("настройки не открылись"),
    };
    let before = h.config().ui.font.font_size_base;
    h.root.focus_on(Some(field));
    assert!(h.app.settings_key(&mut h.root, &Key::Named(NamedKey::ArrowUp)).is_some());
    assert_eq!(h.config().ui.font.font_size_base, before + 1);
    let _ = h.root.redraw();
    let field = h.root.focused_widget().expect("фокус остался в поле");
    h.root.focus_on(Some(field));
    h.app.settings_key(&mut h.root, &Key::Named(NamedKey::ArrowDown));
    h.app.settings_key(&mut h.root, &Key::Named(NamedKey::ArrowDown));
    assert_eq!(h.config().ui.font.font_size_base, before - 1);
    assert!(h.app.settings_key(&mut h.root, &Key::Named(NamedKey::ArrowLeft)).is_none());
}

#[test]
fn mcp_code_scrolls_sideways() {
    use masonry::core::{PointerScrollEvent, ScrollDelta, WidgetId};
    type Sideways = crate::ui::scroll::ScrollArea<crate::ui::widgets::Label>;
    let mut h = Harness::new();
    let mut config = h.config();
    config.mcp.enabled = true;
    config.mcp.token = "0123456789abcdef0123456789abcdef".into();
    h.app.workspace.core.settings.update_config(config).unwrap();
    h.click(242.0, 578.0);
    h.click(115.0, 364.0);
    assert_eq!(h.section(), Some(Section::Mcp));
    let state = |x: f64, y: f64| PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
    h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -2000.0)), state: state(600.0, 400.0) }));
    h.settle();
    fn find(root: &RenderRoot, id: WidgetId, out: &mut Vec<WidgetId>) {
        let Some(w) = root.get_widget(id) else { return };
        if w.downcast::<Sideways>().is_some() {
            out.push(id);
        }
        for child in w.children() {
            find(root, child.id(), out);
        }
    }
    let layer = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.layer,
        _ => panic!("настройки закрылись"),
    };
    let mut blocks = Vec::new();
    find(&h.root, layer, &mut blocks);
    assert!(!blocks.is_empty(), "блоков кода нет");
    let offset = |h: &Harness, id| h.root.get_widget(id).unwrap().downcast::<Sideways>().unwrap().inner().offset();
    let visible = blocks.iter().copied().find(|&id| {
        let r = h.root.get_widget(id).unwrap().ctx().bounding_box();
        r.y0 > 60.0 && r.y1 < f64::from(H) - 20.0
    });
    let id = visible.expect("ни один блок кода не виден");
    let rect = h.root.get_widget(id).unwrap().ctx().bounding_box();
    let (x, y) = (rect.x0 + 20.0, rect.y1 - 8.0);
    let mut s = state(x, y);
    h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: s.clone(), coalesced: vec![], predicted: vec![] }));
    s.buttons.insert(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: s.clone() }));
    for step in 1..=10 {
        s.position = PhysicalPosition::new(x + f64::from(step) * 15.0, y);
        h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: s.clone(), coalesced: vec![], predicted: vec![] }));
    }
    s.buttons.remove(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: s }));
    h.settle();
    let dragged = offset(&h, id);
    assert!(dragged > 0.0, "перетаскивание ползунка не сдвинуло код");
    h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(60.0, 0.0)), state: state(rect.center().x, rect.center().y) }));
    h.settle();
    assert!(offset(&h, id) < dragged, "горизонтальный жест не прокрутил код");
    if let Ok(out) = std::env::var("AQ_OUT") {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
        std::fs::write(std::path::Path::new(&out).join("mcp-code.png"), r.png()).unwrap();
    }
}

#[test]
fn restyle_matches_fresh_build() {
    let mut h = Harness::new();
    let note = h.vault.path().join("заметка.md");
    std::fs::write(&note, "# Заголовок\n\nТекст заметки с [[ссылкой]] и **жирным**.\n\n- пункт\n- [ ] задача").unwrap();
    h.app.open(&mut h.root, note);
    h.app.open_settings(&mut h.root);
    let templates = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.nav.iter().find(|(_, s)| *s == Section::Templates).unwrap().0,
        _ => panic!("настройки не открылись"),
    };
    h.act(templates, crate::ui::widgets::Pressed);
    h.settle();
    let mut config = h.config();
    config.ui.theme = "light".into();
    config.ui.primary_color = "#e0457b".into();
    config.ui.font.font_size_base = 16;
    config.ui.font.font_weight = 500;
    config.editor.font.font_size_base = 19;
    config.editor.line_height = 1.8;
    config.editor.max_width_ch = 60;
    h.app.workspace.core.settings.update_config(config).unwrap();
    let ids_before: Vec<_> = h.app.note_ids.as_ref().map(|ids| ids.body.id()).into_iter().collect();
    h.app.restyle(&mut h.root, true);
    h.settle();
    let (layers, _) = h.root.redraw();
    let mut live = crate::render::Renderer::new(2);
    live.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
    assert_eq!(h.app.note_ids.as_ref().map(|ids| ids.body.id()).into_iter().collect::<Vec<_>>(), ids_before, "restyle пересоздал редактор");
    h.app.rebuild(&mut h.root);
    h.settle();
    let (layers, _) = h.root.redraw();
    let mut fresh = crate::render::Renderer::new(2);
    fresh.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("restyle-live.png"), live.png()).unwrap();
        std::fs::write(std::path::Path::new(&out).join("restyle-fresh.png"), fresh.png()).unwrap();
    }
    let channel = |p: u32, shift: u32| ((p >> shift) & 0xff) as i32;
    let differ = live.frame_pixels().iter().zip(fresh.frame_pixels()).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count();
    assert_eq!(differ, 0, "после restyle кадр отличается от собранного заново в {differ} пикселях");
}

#[test]
fn color_picker_previews_live_and_saves_on_close() {
    let mut h = Harness::new();
    h.app.open_settings(&mut h.root);
    h.settle();
    let swatch = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Color(_)).then_some(*id)).expect("образец цвета"),
        _ => panic!("настройки не открылись"),
    };
    let rect = h.root.get_widget(swatch).unwrap().ctx().bounding_box();
    h.click(rect.center().x, rect.center().y);
    assert!(h.menu_open(), "колорпикер не открылся");
    let before = h.config().ui.primary_color;
    let accent = theme::current().bg_accent;
    let picker = h.root.focused_widget().expect("фокус в колорпикере");
    let area = h.root.get_widget(picker).unwrap().ctx().bounding_box();
    let state = |x: f64, y: f64| PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
    let mut s = state(area.x0 + 30.0, area.y0 + 30.0);
    s.buttons.insert(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: s.clone() }));
    s.position = PhysicalPosition::new(area.x0 + 150.0, area.y0 + 60.0);
    h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: s.clone(), coalesced: vec![], predicted: vec![] }));
    s.buttons.remove(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: s }));
    h.deliver();
    assert_ne!(theme::current().bg_accent, accent, "цвет не применился вживую");
    assert_eq!(h.config().ui.primary_color, before, "настройки записаны раньше закрытия");
    if let Ok(out) = std::env::var("AQ_OUT") {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
        std::fs::write(std::path::Path::new(&out).join("picker.png"), r.png()).unwrap();
    }
    h.escape();
    assert!(!h.menu_open(), "колорпикер не закрылся");
    assert_ne!(h.config().ui.primary_color, before, "цвет не сохранился при закрытии");
}
