use super::*;
use crate::ui::editor::TextAction;
use crate::ui::settings::Binding;

fn color_field(h: &Harness) -> masonry::core::WidgetId {
    h.binding("поле основного цвета", |b| matches!(b, Binding::Text { value, .. } if value.starts_with('#')))
}

#[test]
fn primary_color_field_updates_settings() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    assert_eq!(h.section(), Some(Section::Ui));
    let field = color_field(&h);
    h.send(field, TextAction::Changed("#E05A".into()));
    assert_eq!(h.config().ui.primary_color, AppConfig::default().ui.primary_color);
    h.send(field, TextAction::Changed("#E05A2B".into()));
    assert_eq!(h.config().ui.primary_color, "#e05a2b");
    assert_eq!(h.section(), Some(Section::Ui));
    assert_eq!(theme::current().bg_accent.to_rgba8().to_u8_array(), [0xe0, 0x5a, 0x2b, 0xff]);
}

#[test]
fn primary_color_typed_into_field() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    let field = color_field(&h);
    h.root.focus_on(Some(field));
    h.key(Key::Character("a".into()), crate::test_dir::command());
    h.type_text("#e05a2b");
    assert_eq!(h.config().ui.primary_color, "#e05a2b");
    assert_eq!(theme::current().bg_accent.to_rgba8().to_u8_array(), [0xe0, 0x5a, 0x2b, 0xff]);
}

#[test]
fn restyle_matches_fresh_build() {
    let mut h = Harness::new();
    let note = h.write_note("заметка.md", "# Заголовок\n\nТекст заметки с [[ссылкой]] и **жирным**.\n\n- пункт\n- [ ] задача");
    h.open(note);
    h.app.open_settings(&mut h.root);
    let templates = h.nav(Section::Data);
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
    h.app.restyle(&mut h.root, true, None);
    h.settle();
    let live = h.frame(W as u16, H as u16, 1.0);
    assert_eq!(h.app.note_ids.as_ref().map(|ids| ids.body.id()).into_iter().collect::<Vec<_>>(), ids_before, "restyle пересоздал редактор");
    h.app.rebuild(&mut h.root);
    h.settle();
    let fresh = h.frame(W as u16, H as u16, 1.0);
    dump("restyle-live.png", &live);
    dump("restyle-fresh.png", &fresh);
    let differ = differing_pixels(live.frame_pixels(), fresh.frame_pixels());
    assert_eq!(differ, 0, "после restyle кадр отличается от собранного заново в {differ} пикселях");
}

#[test]
fn color_picker_previews_live_and_saves_on_close() {
    let mut h = Harness::new();
    h.app.open_settings(&mut h.root);
    h.settle();
    let swatch = h.binding("образец цвета", |b| matches!(b, Binding::Color(_)));
    let rect = h.root.get_widget(swatch).unwrap().ctx().bounding_box();
    h.click(rect.center().x, rect.center().y);
    assert!(h.menu_open(), "колорпикер не открылся");
    let before = h.config().ui.primary_color;
    let accent = theme::current().bg_accent;
    let picker = h.root.focused_widget().expect("фокус в колорпикере");
    let area = h.root.get_widget(picker).unwrap().ctx().bounding_box();
    let mut s = pointer_at(area.x0 + 30.0, area.y0 + 70.0, 1.0);
    s.buttons.insert(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Down(primary(s.clone())));
    s.position = PhysicalPosition::new(area.x0 + 150.0, area.y0 + 100.0);
    h.root.handle_pointer_event(moved(s.clone()));
    s.buttons.remove(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Up(primary(s)));
    h.deliver();
    assert_ne!(theme::current().bg_accent, accent, "цвет не применился вживую");
    assert_eq!(h.config().ui.primary_color, before, "настройки записаны раньше закрытия");
    h.snapshot("picker.png");
    h.escape();
    assert!(!h.menu_open(), "колорпикер не закрылся");
    assert_ne!(h.config().ui.primary_color, before, "цвет не сохранился при закрытии");
}

#[test]
fn a_preset_swatch_sets_the_accent_at_once_and_is_saved_on_close() {
    let mut h = Harness::new();
    h.app.open_settings(&mut h.root);
    h.settle();
    let swatch = h.binding("образец цвета", |b| matches!(b, Binding::Color(_)));
    let rect = h.root.get_widget(swatch).unwrap().ctx().bounding_box();
    h.click(rect.center().x, rect.center().y);
    assert!(h.menu_open(), "колорпикер не открылся");
    let picker = h.root.focused_widget().expect("фокус в колорпикере");
    let area = h.root.get_widget(picker).unwrap().ctx().bounding_box();
    h.click(area.x0 + 12.0 + 22.0 + 15.2 + 11.0, area.y0 + 12.0 + 11.0);
    assert_eq!(theme::current().bg_accent.to_rgba8().to_u8_array(), [0x93, 0x33, 0xea, 0xff], "второй готовый цвет не применился");
    h.escape();
    assert_eq!(h.config().ui.primary_color, "#9333ea");
}
