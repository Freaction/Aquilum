use super::*;
use crate::ui::settings::Binding;
use crate::ui::settings::schema::Action;

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

    h.open_section(Section::History);
    assert_eq!(h.section(), Some(Section::History));
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    h.choose(1);
    assert!(!h.menu_open());
    assert_eq!(h.config().history.retention_days, 30);

    h.open_section(Section::Analysis);
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
    h.open_section(Section::Trash);
    assert_eq!(h.section(), Some(Section::Trash));
    let restore = h.binding("в корзине есть заметка", |b| matches!(b, Binding::Button(Action::Restore(_))));
    h.send(restore, crate::ui::widgets::Pressed);
    assert!(vault.join("a.md").exists());
}

#[test]
fn arrows_step_number_fields_like_tauri() {
    let mut h = Harness::new();
    h.app.open_settings(&mut h.root);
    let _ = h.root.redraw();
    let field = h.binding("поле размера шрифта", |b| matches!(b, Binding::Number { number, .. } if number.blank));
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
fn font_size_field_keeps_focus_and_blank_means_default() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    let nav = h.root.get_widget(h.nav(Section::Editor)).unwrap().ctx().bounding_box();
    h.click(nav.center().x, nav.center().y);
    let field = |h: &Harness| h.binding("поле размера шрифта", |b| matches!(b, Binding::Number { number, .. } if number.blank));
    let rect = h.root.get_widget(field(&h)).unwrap().ctx().bounding_box();
    h.click(rect.center().x, rect.center().y);
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 12, "меньше 12 не бывает, как в Таури");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 16, "пустое поле — размер по умолчанию");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
    let text = |h: &Harness| h.root.get_widget(field(h)).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text();
    assert_eq!(text(&h), "");
    h.type_text("20");
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 20);
    assert_eq!(text(&h), "20");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
}

#[test]
fn mcp_code_scrolls_sideways() {
    use masonry::core::WidgetId;
    type Sideways = crate::ui::scroll::ScrollArea<crate::ui::widgets::Label>;
    let mut h = Harness::new();
    let mut config = h.config();
    config.mcp.enabled = true;
    config.mcp.token = "0123456789abcdef0123456789abcdef".into();
    h.app.workspace.core.settings.update_config(config).unwrap();
    h.click(242.0, 578.0);
    h.open_section(Section::Mcp);
    assert_eq!(h.section(), Some(Section::Mcp));
    h.scroll((600.0, 400.0), 1.0, (0.0, -2000.0));
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
    h.drag((x, y), (x + 150.0, y));
    h.settle();
    let dragged = offset(&h, id);
    assert!(dragged > 0.0, "перетаскивание ползунка не сдвинуло код");
    h.scroll((rect.center().x, rect.center().y), 1.0, (60.0, 0.0));
    h.settle();
    assert!(offset(&h, id) < dragged, "горизонтальный жест не прокрутил код");
    h.snapshot("mcp-code.png");
}

#[test]
fn animations_switch_turns_motion_off_and_on() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    assert!(h.config().ui.animations && crate::ui::motion::on());
    let switch = h.binding("переключатель анимаций", |b| matches!(b, Binding::Switch(_)));
    let center = h.root.get_widget(switch).unwrap().ctx().bounding_box().center();
    h.click(center.x, center.y);
    assert!(!h.config().ui.animations);
    assert!(!crate::ui::motion::on(), "после выключения анимации продолжают идти");
    h.click(center.x, center.y);
    assert!(h.config().ui.animations && crate::ui::motion::on());
}
