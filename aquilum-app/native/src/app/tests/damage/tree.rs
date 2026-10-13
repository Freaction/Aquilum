use super::*;

const TREE: Spec = Spec { scale: 2.0, width: 2000, height: 1200 };

fn folder_with_notes(h: &mut Harness, name: &str, notes: usize) {
    let folder = h.vault.path().join(name);
    std::fs::create_dir(&folder).unwrap();
    for i in 0..notes {
        std::fs::write(folder.join(format!("заметка {i:03}.md")), "").unwrap();
    }
    h.app.tree.paths_changed(&mut h.root, std::slice::from_ref(&folder));
}

#[test]
fn tree_scroll_with_a_pinned_folder_matches_full_frames() {
    let mut h = setup(TREE, &plain_body(2));
    folder_with_notes(&mut h, "Проекты", 120);
    let row = rect_of(&h, h.app.tree.row_id("Проекты").unwrap(), TREE.scale).center();
    click_at(&mut h, (row.x, row.y), TREE.scale);
    let tree = rect_of(&h, h.app.tree.scroll_id().unwrap(), TREE.scale).center();
    let mut frames = Frames::new(&mut h, TREE, true);
    let mut pinned = 0;
    for i in 0..14 {
        let dy = if i < 10 { -37.0 } else { 23.0 };
        h.scroll((tree.x, tree.y), TREE.scale, (0.0, dy));
        h.anim(std::time::Duration::from_millis(16));
        h.deliver();
        let report = frames.draw(&mut h);
        assert_eq!(report.differ, 0, "шаг {i}: дерево с закреплённой папкой расходится с полной отрисовкой в {} пикселях", report.differ);
        pinned += h.app.tree.pinned_names(&h.root).len();
    }
    assert!(pinned >= 8, "папка не была закреплена при прокрутке");
}
