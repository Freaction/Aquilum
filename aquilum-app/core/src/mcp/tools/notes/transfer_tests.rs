use crate::app_core::{Core, CoreEvent};
use crate::mcp::tools::call;
use crate::search::paths::canonical_path;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct Vaults {
    core: Arc<Core>,
    from: PathBuf,
    to: PathBuf,
    _directory: tempfile::TempDir,
}

impl Drop for Vaults {
    fn drop(&mut self) {
        self.core.shutdown();
    }
}

fn vaults() -> Vaults {
    let directory = tempfile::tempdir().unwrap();
    let root = canonical_path(directory.path());
    let (from, to, data) = (root.join("Личная"), root.join("Работа"), root.join("data"));
    for dir in [&from, &to, &data] {
        fs::create_dir_all(dir).unwrap();
    }
    let core = Core::open(&data, Arc::new(|_: CoreEvent| {}));
    for vault in [&from, &to] {
        core.ui_state.resolve_workspace(&vault.to_string_lossy(), 0).unwrap();
    }
    let mut config = core.settings.get_config();
    config.mcp.allow_write = true;
    core.settings.update_config(config).unwrap();
    write(&from, "Созвон.md", "Итоги\n\n![[кадр.png]]\n\n![схема](Files/схема.png)\n\n[[Ретро]] и [[Общая]]\n");
    write(&from, "Ретро.md", "[[Созвон]]\n");
    write(&from, "Общая.md", "[[Созвон]]\n\n![[схема.png]]\n");
    write(&from, "Files/кадр.png", "png");
    write(&from, "Files/схема.png", "png");
    for vault in [&from, &to] {
        let workspace = vault.to_string_lossy();
        core.search.open_guest(&workspace).unwrap();
        let indexed = (0..600).any(|_| {
            let done = !core.search.status(Some(&workspace)).updating;
            if !done {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            done
        });
        assert!(indexed, "база {workspace} не проиндексировалась за минуту");
    }
    Vaults { core, from, to, _directory: directory }
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn move_notes(vaults: &Vaults, notes: &[&str]) -> Result<Value, String> {
    let arguments = json!({ "notes": notes, "workspace": vaults.from.to_string_lossy(), "toWorkspace": vaults.to.to_string_lossy() });
    call(&vaults.core, "move_note", &arguments)
}

#[test]
fn a_batch_moves_its_own_files_copies_shared_ones_and_reports_only_outside_links() {
    let vaults = vaults();

    let moved = move_notes(&vaults, &["Созвон", "Ретро"]).unwrap();

    assert_eq!(moved["moved"], json!(["Созвон.md", "Ретро.md"]));
    assert_eq!(moved["brokenLinks"], json!(["Общая.md"]));
    assert_eq!(moved["files"], json!({ "moved": ["Files/кадр.png"], "copied": ["Files/схема.png"] }));
    for path in ["Созвон.md", "Ретро.md", "Files/кадр.png", "Files/схема.png"] {
        assert!(vaults.to.join(path).is_file(), "{path} не в новой базе");
    }
    assert!(!vaults.from.join("Созвон.md").exists() && !vaults.from.join("Files/кадр.png").exists());
    assert!(vaults.from.join("Files/схема.png").is_file(), "общая картинка осталась у второй заметки");
}

#[test]
fn a_name_taken_in_the_target_stops_the_whole_batch_before_anything_moves() {
    let vaults = vaults();
    write(&vaults.to, "Ретро.md", "уже есть");

    let error = move_notes(&vaults, &["Созвон", "Ретро"]).unwrap_err();

    assert!(error.contains("Ретро.md"), "{error}");
    assert!(vaults.from.join("Созвон.md").is_file() && !vaults.to.join("Созвон.md").exists());
}
