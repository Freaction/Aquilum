use masonry::kurbo::Point;

use super::Scene;
use crate::ui::snapshot::{open_parts, parts};
use crate::ui::workspaces;

pub(super) fn snapshots(scene: &Scene) {
    let (mut root, _) = scene.shell();
    let known = vec![
        (uuid::Uuid::new_v4(), scene.vault.to_string_lossy().into_owned()),
        (uuid::Uuid::new_v4(), r"D:\Заметки\Работа".to_owned()),
        (uuid::Uuid::new_v4(), r"C:\Users\Dmitriy\Documents\Очень длинное имя базы знаний для проверки".to_owned()),
    ];
    let (layer, _) = workspaces::dialog(&known, Some(scene.vault), Some(r"Не удалось открыть «D:\Нет»."));
    root.add_layer(layer, Point::ORIGIN);
    scene.save(&mut root, "workspaces");
    let (mut root, _) = scene.shell();
    let (layer, _) = workspaces::dialog(&[], None, None);
    root.add_layer(layer, Point::ORIGIN);
    scene.save(&mut root, "workspaces-empty");
    let mut p = parts("");
    p.vault_open = false;
    let (mut root, _) = open_parts(None, scene.data, p);
    scene.save(&mut root, "no-vault");
}
