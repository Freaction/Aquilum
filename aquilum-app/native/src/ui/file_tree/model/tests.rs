use std::path::Path;
use std::time::UNIX_EPOCH;

use super::*;

pub(super) fn tree() -> (tempfile::TempDir, TreeModel) {
    let dir = crate::test_dir::tempdir();
    let root = dir.path();
    std::fs::create_dir_all(root.join("Проекты/Вложенная")).unwrap();
    std::fs::create_dir_all(root.join("Архив/Старое")).unwrap();
    std::fs::write(root.join("Архив/Старое/старое.md"), "").unwrap();
    std::fs::write(root.join("Проекты/Вложенная/глубоко.md"), "").unwrap();
    std::fs::write(root.join("Проекты/Aquilum.md"), "").unwrap();
    std::fs::write(root.join("README.md"), "").unwrap();
    std::fs::write(root.join("картинка.png"), "").unwrap();
    std::fs::write(root.join("._README.md"), "").unwrap();
    std::fs::create_dir_all(root.join(".obsidian")).unwrap();
    std::fs::write(root.join(".obsidian/настройки.md"), "").unwrap();
    let model = TreeModel::new(Some(root.to_path_buf()), []);
    (dir, model)
}

pub(super) fn names(model: &TreeModel) -> Vec<(String, usize)> {
    model.rows().iter().map(|r| (r.name.clone(), r.depth)).collect()
}

#[test]
fn folders_first_and_hidden_skipped() {
    let (_dir, model) = tree();
    assert_eq!(
        names(&model),
        [("Архив".into(), 0), ("Проекты".into(), 0), ("README".into(), 0), ("картинка.png".into(), 0)]
    );
}

#[test]
fn click_toggles_folder_and_opens_note() {
    let (_dir, mut model) = tree();
    assert_eq!(model.press(1, Modifiers::default()), Some(Press::Rebuild));
    assert_eq!(names(&model)[2], ("Вложенная".into(), 1));
    assert_eq!(model.rows()[2].guides, vec![0]);
    let readme = model.rows().iter().position(|r| r.name == "README").unwrap();
    let Some(Press::Open { path, new_tab: false }) = model.press(readme, Modifiers::default()) else {
        panic!("заметка не открылась");
    };
    assert!(path.ends_with("README.md"));
    model.press(1, Modifiers::default());
    assert_eq!(model.rows().len(), 4);
}

#[test]
fn last_child_drops_parent_guide() {
    let (_dir, mut model) = tree();
    model.press(1, Modifiers::default());
    model.press(2, Modifiers::default());
    let deep = model.rows().iter().find(|r| r.name == "глубоко").unwrap();
    assert_eq!(deep.guides, vec![0, 1]);
    model.press(0, Modifiers::default());
    model.press(1, Modifiers::default());
    let old = model.rows().iter().find(|r| r.name == "старое").unwrap();
    assert_eq!(old.guides, vec![1]);
}

#[test]
fn drop_rules() {
    let (_dir, mut model) = tree();
    model.press(1, Modifiers::default());
    let index = |m: &TreeModel, name: &str| m.rows().iter().position(|r| r.name == name).unwrap();
    let projects = model.rows()[index(&model, "Проекты")].path.clone();
    let readme = model.rows()[index(&model, "README")].path.clone();
    assert_eq!(model.drop_target(&readme, index(&model, "Проекты")), Some(projects.clone()));
    assert_eq!(model.drop_target(&readme, index(&model, "картинка.png")), None);
    assert_eq!(model.drop_target(&projects, index(&model, "Проекты")), None);
    assert_eq!(model.drop_target(&projects, index(&model, "Вложенная")), None);
    assert_eq!(model.drop_target(&readme, index(&model, "Aquilum")), Some(projects));
}

pub(super) fn touch(path: &Path, seconds: u64) {
    let file = std::fs::File::options().write(true).open(path).unwrap();
    file.set_modified(UNIX_EPOCH + std::time::Duration::from_secs(seconds)).unwrap();
}

#[test]
fn sorts_like_obsidian() {
    let (dir, mut model) = tree();
    touch(&dir.path().join("README.md"), 1_000);
    touch(&dir.path().join("картинка.png"), 2_000);
    let order = |m: &TreeModel| m.rows().iter().map(|r| r.name.clone()).collect::<Vec<_>>();
    assert_eq!(order(&model), ["Архив", "Проекты", "README", "картинка.png"]);
    assert!(model.set_sort(Sort::NameDesc));
    assert_eq!(order(&model), ["Проекты", "Архив", "картинка.png", "README"]);
    assert!(model.set_sort(Sort::ModifiedNew));
    assert_eq!(order(&model), ["Архив", "Проекты", "картинка.png", "README"]);
    assert!(model.set_sort(Sort::ModifiedOld));
    assert_eq!(order(&model), ["Архив", "Проекты", "README", "картинка.png"]);
    assert!(!model.set_sort(Sort::ModifiedOld));
    model.press(1, Modifiers::default());
    let nested = order(&model);
    assert_eq!(nested[2..4], ["Вложенная", "Aquilum"]);
    assert!(model.set_sort(Sort::NameAsc));
    assert_eq!(order(&model)[2..4], ["Вложенная", "Aquilum"]);
}

#[test]
fn folder_click_is_target_and_collapse_all_closes_everything() {
    let (dir, mut model) = tree();
    assert_eq!(model.target_folder().as_deref(), Some(dir.path()));
    model.press(1, Modifiers::default());
    assert!(model.rows()[1].selected, "щелчок по папке её не выделил");
    assert_eq!(model.target_folder(), Some(dir.path().join("Проекты")));
    model.press(0, Modifiers::default());
    assert_eq!(model.target_folder(), Some(dir.path().join("Архив")));
    assert!(model.rows().len() > 4);
    assert!(model.collapse_all());
    assert_eq!(model.rows().len(), 4);
    assert!(model.expanded_paths().is_empty());
    assert!(!model.collapse_all());
    let readme = model.rows().iter().position(|r| r.name == "README").unwrap();
    model.press(readme, Modifiers::default());
    assert_eq!(model.target_folder().as_deref(), Some(dir.path()));
}

#[test]
fn expand_to_opens_every_parent() {
    let (dir, mut model) = tree();
    let deep = dir.path().join("Проекты/Вложенная/глубоко.md");
    model.expand_to(&deep);
    assert!(model.index_of(&deep).is_some(), "строка вложенной заметки не видна");
    assert_eq!(model.rows()[model.index_of(&deep).unwrap()].depth, 2);
}

#[test]
fn ctrl_and_shift_select() {
    let (_dir, mut model) = tree();
    let action = Modifiers { action: true, shift: false };
    model.press(2, action);
    model.press(3, action);
    assert!(model.rows()[2].selected && model.rows()[3].selected);
    model.press(3, action);
    assert!(!model.rows()[3].selected);
    model.press(0, Modifiers { action: false, shift: true });
    assert!((0..=3).all(|i| model.rows()[i].selected));
}

#[test]
fn folder_colors_follow_renames_and_moves() {
    let (_dir, mut model) = tree();
    let projects = model.rows()[1].path.clone();
    model.set_color(std::slice::from_ref(&projects), Some("blue"));
    assert_eq!(model.rows()[1].color, Some("blue"));
    let renamed = projects.with_file_name("Дела");
    model.patch(&projects, &renamed, "Дела");
    assert_eq!(model.color(&renamed), Some("blue"));
    assert_eq!(model.color(&projects), None);
    let archive = model.rows()[0].path.clone();
    model.moved(std::slice::from_ref(&renamed), &archive);
    assert_eq!(model.color(&archive.join("Дела")), Some("blue"));
    model.set_color(&[archive.join("Дела")], None);
    assert!(model.color_entries().is_empty());
}
