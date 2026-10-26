use super::*;
use crate::updater::Attempt;

#[test]
fn app_starts_with_manual_update_popup_after_repeated_failures() {
    let h = Harness::new();
    let data_dir = h.app.workspace.data_dir.clone();

    // Симулируем 2 неудачные попытки обновления до новой версии
    Attempt::record(&data_dir, "99.0.0", "https://example.com/download");
    Attempt::record(&data_dir, "99.0.0", "https://example.com/download");

    // Перезапуск приложения с тем же каталогом данных
    let mut h = h.restarted(1.0);
    h.app.on_core_events(&mut h.root);
    let _ = h.root.redraw();

    // Проверяем, что открылся диалог предложения скачать вручную
    assert!(
        matches!(h.app.popup, Some(Popup::ManualUpdate(_, _))),
        "диалог ручного обновления не открылся после неудачных попыток"
    );

    // Закрытие по Escape
    h.escape();
    assert!(h.app.popup.is_none(), "диалог ручного обновления не закрылся по Escape");
}
