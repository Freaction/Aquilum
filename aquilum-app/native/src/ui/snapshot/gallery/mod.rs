mod hero;
mod panels;
mod search_dialog;
mod settings_dialog;
mod tree;
mod workspaces_dialog;

use std::path::Path;

use masonry::app::RenderRoot;

use super::{save, shell};
use crate::ui::file_tree::FileTree;
use crate::ui::theme;

struct Scene<'a> {
    vault: &'a Path,
    data: &'a Path,
    suffix: &'a str,
}

impl Scene<'_> {
    fn shell(&self) -> (RenderRoot, FileTree) {
        shell(self.vault, self.data)
    }

    fn save(&self, root: &mut RenderRoot, name: &str) {
        save(root, &format!("{name}-{}", self.suffix));
    }
}

#[test]
#[ignore = "снимки для просмотра глазами"]
fn snapshot_tree_menu_dialog() {
    let _theme = theme::test_lock();
    let vault = crate::test_dir::tempdir();
    let data = crate::test_dir::tempdir();
    for dir in ["Проекты", "Архив"] {
        std::fs::create_dir_all(vault.path().join(dir)).unwrap();
    }
    for note in ["Aquilum.md", "Уход от Tauri.md", "Очень длинное название заметки, которое не влезет.md"] {
        std::fs::write(vault.path().join(note), "").unwrap();
    }
    crate::i18n::set_language("ru");
    let config = aquilum_core::settings::models::AppConfig::default();
    for (dark, suffix) in [(true, "dark"), (false, "light")] {
        let mode = if dark { "dark" } else { "light" };
        theme::apply_setting(mode, false, "");
        let scene = Scene { vault: vault.path(), data: data.path(), suffix };
        tree::snapshots(&scene);
        panels::snapshots(&scene);
        search_dialog::snapshot(&scene);
        workspaces_dialog::snapshots(&scene);
        hero::snapshots(&scene);
        theme::apply_setting(mode, false, "#e05a2b");
        settings_dialog::primary(&scene, &config);
        theme::apply_setting(mode, false, "");
        settings_dialog::snapshots(&scene, &config);
    }
}
