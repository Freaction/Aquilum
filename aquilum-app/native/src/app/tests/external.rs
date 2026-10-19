use super::*;

fn appears(h: &mut Harness, name: &str) -> bool {
    wait_until(300, 20, || {
        h.app.on_core_events(&mut h.root);
        h.app.tree.names().iter().any(|n| n == name)
    })
}

#[test]
fn files_and_folders_put_into_the_vault_from_outside_show_up_in_the_tree() {
    let mut h = Harness::new();
    h.watch_vault();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("внешняя.md"), "текст").unwrap();
    assert!(appears(&mut h, "внешняя"), "новая заметка не появилась");
    std::fs::create_dir(vault.join("Пустая")).unwrap();
    assert!(appears(&mut h, "Пустая"), "новая пустая папка не появилась");
    std::fs::create_dir(vault.join("С заметкой")).unwrap();
    std::fs::write(vault.join("С заметкой/внутри.md"), "текст").unwrap();
    assert!(appears(&mut h, "С заметкой"), "новая папка с заметкой не появилась");
    std::fs::write(vault.join("картинка.png"), "png").unwrap();
    assert!(appears(&mut h, "картинка.png"), "новая картинка не появилась");
    std::fs::remove_file(vault.join("внешняя.md")).unwrap();
    assert!(wait_until(300, 20, || {
        h.app.on_core_events(&mut h.root);
        !h.app.tree.names().iter().any(|n| n == "внешняя")
    }), "удалённая заметка осталась");
}

#[test]
fn open_note_follows_an_outside_edit_without_waiting_for_the_index() {
    let mut h = Harness::new();
    h.watch_vault();
    let path = h.write_note("внешняя.md", "старый текст");
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&path));
    assert!(h.open(path.clone()));
    std::fs::write(&path, "новый текст снаружи").unwrap();
    assert!(
        wait_until(250, 20, || {
            h.app.on_core_events(&mut h.root);
            h.body_text() == "новый текст снаружи"
        }),
        "правка заметки снаружи не подтянулась в редактор"
    );
}

#[test]
fn a_lost_batch_of_events_still_leaves_the_tree_and_the_open_note_up_to_date() {
    let mut h = Harness::new();
    let path = h.write_note("внешняя.md", "старый текст");
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&path));
    assert!(h.open(path.clone()));
    std::fs::write(&path, "новый текст").unwrap();
    std::fs::write(h.vault.path().join("новая.md"), "").unwrap();
    h.app.workspace.core.emit(aquilum_core::CoreEvent::WorkspaceRescanned);
    h.app.on_core_events(&mut h.root);
    assert_eq!(h.body_text(), "новый текст");
    assert!(h.app.tree.names().iter().any(|name| name == "новая"), "дерево не перечитано после потери событий");
}

#[test]
fn a_note_or_text_file_removed_outside_closes_its_tab() {
    let mut h = Harness::new();
    h.watch_vault();
    for name in ["удаляемая.md", "удаляемая.txt"] {
        let path = h.write_note(name, "текст");
        h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&path));
        assert!(h.open(path.clone()));
        assert!(h.app.shows(&path));
        std::fs::remove_file(&path).unwrap();
        assert!(
            wait_until(250, 20, || {
                h.app.on_core_events(&mut h.root);
                h.app.note.as_ref().is_none_or(|note| !note.is(&path))
            }),
            "{name}: удалённый файл остался открытым"
        );
    }
}

#[test]
fn deleting_a_text_file_from_the_tree_closes_its_tab() {
    let mut h = Harness::new();
    let path = h.write_note("черновик.txt", "текст");
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&path));
    assert!(h.open(path.clone()));
    crate::file_ops::trash_all(&h.app.workspace.core, h.vault.path().to_path_buf(), vec![path.clone()]);
    assert!(
        wait_until(250, 20, || {
            h.app.on_core_events(&mut h.root);
            h.app.note.as_ref().is_none_or(|note| !note.is(&path))
        }),
        "удалённый из дерева .txt остался открытым"
    );
    assert!(!path.exists());
}
