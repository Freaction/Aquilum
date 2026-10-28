use masonry::kurbo::Point;

use super::Scene;
use crate::ui::snapshot::{open_parts, parts};
use crate::ui::workspaces;

pub(super) fn snapshots(scene: &Scene) {
    let (mut root, _) = scene.shell();
    let vault = scene.vault.to_string_lossy();
    let known = [vault.as_ref(), r"D:\Заметки\Работа", r"C:\Users\Dmitriy\Documents\Очень длинное имя базы знаний для проверки"];
    let (list, _) = workspaces::rows(&known, Some(0), None, Some(r"Не удалось открыть «D:\Нет»."), false);
    let (layer, _) = workspaces::dialog(list, false);
    root.add_layer(layer, Point::ORIGIN);
    scene.save(&mut root, "workspaces");
    let (mut root, _) = scene.shell();
    let (list, _) = workspaces::rows(&known, Some(0), Some(1), None, true);
    let (layer, _) = workspaces::dialog(list, false);
    root.add_layer(layer, Point::ORIGIN);
    scene.save(&mut root, "workspaces-rename");
    let (mut root, _) = scene.shell();
    let (list, _) = workspaces::rows(&[], None, None, None, false);
    let (layer, _) = workspaces::dialog(list, false);
    root.add_layer(layer, Point::ORIGIN);
    scene.save(&mut root, "workspaces-empty");
    let mut p = parts("");
    p.vault_open = false;
    let (mut root, _) = open_parts(None, scene.data, p);
    scene.save(&mut root, "no-vault");
}
