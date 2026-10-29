use masonry::kurbo::Point;

use super::Scene;
use crate::ui::dialog::ConfirmDialog;
use crate::ui::menu::Menu;
use crate::ui::snapshot::shell;

pub(super) fn snapshots(scene: &Scene) {
    let (mut root, _) = scene.shell();
    root.add_layer(Menu::new("Действия с файлом", &["Переименовать", "Дублировать", "Удалить"]), Point::new(150.0, 120.0));
    scene.save(&mut root, "menu");

    let long = crate::test_dir::tempdir();
    for i in 0..40 {
        std::fs::write(long.path().join(format!("Заметка {i:02}.md")), "").unwrap();
    }
    let (mut root, _) = shell(long.path(), scene.data);
    scene.save(&mut root, "scroll");

    let empty = crate::test_dir::tempdir();
    let (mut root, _) = shell(empty.path(), scene.data);
    scene.save(&mut root, "empty");

    let (mut root, mut tree) = scene.shell();
    tree.start_rename(&mut root, scene.vault.join("Aquilum.md"));
    scene.save(&mut root, "rename");

    let (mut root, mut tree) = scene.shell();
    tree.filter(&mut root, "длин");
    std::thread::sleep(std::time::Duration::from_millis(300));
    tree.poll(&mut root);
    scene.save(&mut root, "filter");

    let (mut root, _) = scene.shell();
    let (dialog, _) = ConfirmDialog::new(
        "Удалить заметку?",
        "«Уход от Tauri» переедет в корзину базы знаний.",
        "Отмена",
        "Удалить",
    );
    root.add_layer(dialog, Point::ORIGIN);
    scene.save(&mut root, "dialog");
}
