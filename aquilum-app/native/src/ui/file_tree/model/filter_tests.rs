use super::tests::{names, tree};
use super::*;

fn filtered(model: &mut TreeModel, root: &Path, query: &str) -> Vec<(String, usize)> {
    model.set_filter(query);
    model.set_index(index(root));
    names(model)
}

#[test]
fn a_filter_keeps_the_matches_with_their_folders_and_marks_the_name() {
    let (dir, mut model) = tree();
    assert_eq!(filtered(&mut model, dir.path(), "глуб"), [("Проекты".into(), 0), ("Вложенная".into(), 1), ("глубоко".into(), 2)]);
    let found = model.rows().iter().find(|row| row.name == "глубоко").unwrap();
    assert_eq!(found.marks, [(0, 8)]);
    assert!(model.rows()[0].expanded && model.rows()[0].marks.is_empty());
    assert_eq!(model.rows()[0].items, None);
}

#[test]
fn the_filter_ignores_case_and_wants_every_word() {
    let (dir, mut model) = tree();
    assert_eq!(filtered(&mut model, dir.path(), "readme"), [("README".into(), 0)]);
    assert_eq!(model.rows()[0].marks, [(0, 6)]);
    assert_eq!(filtered(&mut model, dir.path(), "aq ui"), [("Проекты".into(), 0), ("Aquilum".into(), 1)]);
    assert_eq!(model.rows()[1].marks, [(0, 2), (2, 4)]);
    assert!(filtered(&mut model, dir.path(), "aq zz").is_empty());
}

#[test]
fn files_in_closed_folders_are_found_only_with_the_index() {
    let (dir, mut model) = tree();
    model.set_filter("старое");
    assert!(names(&model).is_empty() && !model.indexed());
    model.set_index(index(dir.path()));
    assert_eq!(names(&model), [("Архив".into(), 0), ("Старое".into(), 1), ("старое".into(), 2)]);
}

#[test]
fn clearing_the_filter_restores_the_tree_and_drops_the_index() {
    let (dir, mut model) = tree();
    let before = names(&model);
    filtered(&mut model, dir.path(), "aquilum");
    assert!(model.filtering() && model.indexed());
    model.set_filter("  ");
    assert!(!model.filtering() && !model.indexed());
    assert_eq!(names(&model), before);
}

#[test]
fn a_folder_row_does_not_toggle_while_filtering_and_the_order_follows_the_sort() {
    let (dir, mut model) = tree();
    filtered(&mut model, dir.path(), "а");
    let projects = model.rows().iter().position(|row| row.name == "Проекты").unwrap();
    model.press(projects, Modifiers::default());
    assert!(model.expanded_paths().is_empty());
    model.set_sort(Sort::NameDesc);
    let top: Vec<String> = model.rows().iter().filter(|row| row.depth == 0).map(|row| row.name.clone()).collect();
    assert_eq!(top, ["Проекты", "Архив", "картинка.png"]);
}

#[test]
fn a_folder_name_match_shows_the_folder_alone_and_accents_do_not_hide_a_match() {
    let (dir, mut model) = tree();
    assert_eq!(filtered(&mut model, dir.path(), "архив"), [("Архив".into(), 0)]);
    assert_eq!(model.rows()[0].marks, [(0, 10)]);
    assert!(!model.rows()[0].expanded);
    std::fs::write(dir.path().join("Café.md"), "").unwrap();
    assert_eq!(filtered(&mut model, dir.path(), "cafe"), [("Café".into(), 0)]);
    assert_eq!(model.rows()[0].marks, [(0, 5)]);
}

#[test]
fn the_filtered_list_follows_a_time_sort() {
    let dir = crate::test_dir::tempdir();
    for (name, seconds) in [("а1.md", 1_000), ("а2.md", 3_000), ("а3.md", 2_000)] {
        std::fs::write(dir.path().join(name), "").unwrap();
        super::tests::touch(&dir.path().join(name), seconds);
    }
    let mut model = TreeModel::new(Some(dir.path().to_path_buf()), []);
    model.set_sort(Sort::ModifiedNew);
    let order = |model: &TreeModel| model.rows().iter().map(|row| row.name.clone()).collect::<Vec<_>>();
    filtered(&mut model, dir.path(), "а");
    assert_eq!(order(&model), ["а2", "а3", "а1"]);
    model.set_sort(Sort::ModifiedOld);
    assert_eq!(order(&model), ["а1", "а3", "а2"]);
}

#[test]
fn twenty_thousand_names_filter_fast_even_in_a_debug_build() {
    let (dir, mut model) = tree();
    let file = |folder: &Path, name: String, kind| aquilum_core::files::models::FileItem { id: folder.join(&name).to_string_lossy().into_owned(), name, item_type: kind };
    let mut lists = Lists::new();
    let mut folders = Vec::new();
    for number in 0..400 {
        let folder = dir.path().join(format!("папка {number}"));
        let notes = (0..50).map(|at| file(&folder, format!("заметка {at} про кофе и чай {number}"), aquilum_core::files::models::FileItemType::File)).collect();
        lists.insert(comparable(&folder), notes);
        folders.push(file(dir.path(), format!("папка {number}"), aquilum_core::files::models::FileItemType::Folder));
    }
    lists.insert(comparable(dir.path()), folders);
    model.set_filter("к");
    model.set_index(lists);
    let started = std::time::Instant::now();
    model.set_filter("кофе чай");
    assert_eq!(model.rows().len(), 400 + 20_000);
    assert!(started.elapsed() < std::time::Duration::from_secs(1), "фильтр 20 тысяч названий занял {:?}", started.elapsed());
}
