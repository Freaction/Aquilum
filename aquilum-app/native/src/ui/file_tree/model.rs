//! Модель дерева файлов без виджетов — перенос `src/components/Layout/fileTreeModel.ts` и
//! `useFileSelection.ts`: кэш папок, раскрытые папки, выделение с якорем, открытая заметка,
//! видимые строки с направляющими.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use aquilum_core::files::models::{FileItem, FileItemType};
use aquilum_core::files::workspace::read_directory_impl;

/// Путь для сравнения: без учёта регистра и с прямыми слешами, как `comparablePath` во фронтенде.
pub fn comparable(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub path: PathBuf,
    pub name: String,
    pub folder: bool,
    pub depth: usize,
    pub expanded: bool,
    /// Уровни, на которых через строку идёт вертикальная направляющая.
    pub guides: Vec<usize>,
    pub active: bool,
    pub selected: bool,
}

/// Что сделать окну после нажатия на строку.
#[derive(Debug, PartialEq)]
pub enum Press {
    /// Строки изменились (раскрытие папки, выделение) — перестроить дерево.
    Rebuild,
    /// Открыть заметку; `new_tab` — Ctrl+Shift+щелчок.
    Open { path: PathBuf, new_tab: bool },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    /// Ctrl на Windows и Linux, Cmd на macOS.
    pub action: bool,
    pub shift: bool,
}

#[derive(Default)]
pub struct TreeModel {
    root: Option<PathBuf>,
    directories: HashMap<String, Vec<FileItem>>,
    /// Раскрытые папки: ключ для сравнения → путь как на диске (для сохранения).
    expanded: BTreeMap<String, PathBuf>,
    selected: HashSet<String>,
    anchor: Option<PathBuf>,
    active: Option<PathBuf>,
    /// Строка в режиме переименования.
    renaming: Option<PathBuf>,
    rows: Vec<Row>,
}

impl TreeModel {
    pub fn new(root: Option<PathBuf>, expanded: impl IntoIterator<Item = PathBuf>) -> Self {
        let expanded = expanded.into_iter().map(|p| (comparable(&p), p)).collect();
        let mut model = TreeModel { root, expanded, ..Default::default() };
        model.reload_all();
        model
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Все раскрытые папки, в том числе внутри свёрнутых, — для сохранения между запусками.
    pub fn expanded_paths(&self) -> Vec<PathBuf> {
        self.expanded.values().cloned().collect()
    }

    /// Перечитывает корень и все раскрытые папки.
    pub fn reload_all(&mut self) {
        self.directories.clear();
        if let Some(root) = self.root.clone() {
            self.load(&root);
        }
        self.rebuild();
    }

    /// Перечитывает папки, затронутые изменившимися путями (как `affectedDirectories` во фронтенде:
    /// загруженный родитель пути, иначе ближайший загруженный предок).
    pub fn refresh_paths(&mut self, changed: &[PathBuf]) {
        let mut dirs = HashSet::new();
        for path in changed {
            let mut current = path.parent();
            while let Some(dir) = current {
                if self.directories.contains_key(&comparable(dir)) {
                    dirs.insert(dir.to_path_buf());
                    break;
                }
                current = dir.parent();
            }
        }
        for dir in dirs {
            self.load(&dir);
        }
        self.rebuild();
    }

    pub fn set_active(&mut self, path: Option<PathBuf>) {
        // Смена открытой заметки сбрасывает выделение и переносит на неё якорь.
        self.anchor = path.clone();
        self.selected.clear();
        self.active = path;
        self.rebuild();
    }

    /// Щелчок по строке `index` — логика `FileTreeRow` и `useFileSelection`.
    pub fn press(&mut self, index: usize, modifiers: Modifiers) -> Option<Press> {
        let row = self.rows.get(index)?.clone();
        if modifiers.action && modifiers.shift && !row.folder {
            self.selected.clear();
            self.anchor = Some(row.path.clone());
            return Some(Press::Open { path: row.path, new_tab: true });
        }
        if modifiers.shift {
            self.select_range(index);
            self.rebuild();
            return Some(Press::Rebuild);
        }
        if modifiers.action {
            let key = comparable(&row.path);
            if !self.selected.remove(&key) {
                self.selected.insert(key);
            }
            self.anchor = Some(row.path);
            self.rebuild();
            return Some(Press::Rebuild);
        }
        if row.folder {
            let key = comparable(&row.path);
            if self.expanded.remove(&key).is_none() {
                self.expanded.insert(key.clone(), row.path.clone());
                if !self.directories.contains_key(&key) {
                    self.load(&row.path);
                }
            }
            self.rebuild();
            return Some(Press::Rebuild);
        }
        self.selected.clear();
        self.anchor = Some(row.path.clone());
        Some(Press::Open { path: row.path, new_tab: false })
    }

    /// Правый щелчок — `focusRow`: строка становится единственной выделенной, если она не
    /// входит в мультивыделение. Возвращает цели действий меню.
    pub fn focus(&mut self, index: usize) -> Vec<PathBuf> {
        let Some(row) = self.rows.get(index).cloned() else { return Vec::new() };
        let key = comparable(&row.path);
        if !(self.selected.len() > 1 && self.selected.contains(&key)) {
            self.selected = HashSet::from([key]);
            self.anchor = Some(row.path.clone());
        }
        self.rebuild();
        self.rows.iter().filter(|r| r.selected).map(|r| r.path.clone()).collect()
    }

    pub fn renaming(&self) -> Option<&Path> {
        self.renaming.as_deref()
    }

    pub fn start_rename(&mut self, path: PathBuf) {
        self.renaming = Some(path);
    }

    pub fn stop_rename(&mut self) {
        self.renaming = None;
    }

    /// Оптимистичная подмена имени до события ядра — `patchFileEntry` во фронтенде; раскрытые
    /// папки внутри переименованной следуют за ней (`followFolder`).
    pub fn patch(&mut self, from: &Path, to: &Path, name: &str) {
        let from_key = comparable(from);
        for items in self.directories.values_mut() {
            for item in items.iter_mut().filter(|i| comparable(Path::new(&i.id)) == from_key) {
                item.id = to.to_string_lossy().into_owned();
                item.name = name.to_owned();
            }
        }
        let moved: Vec<(String, PathBuf)> = self
            .expanded
            .iter()
            .filter_map(|(key, path)| Some((key.clone(), to.join(path.strip_prefix(from).ok()?))))
            .collect();
        for (key, path) in moved {
            self.expanded.remove(&key);
            self.expanded.insert(comparable(&path), path);
        }
        if let Some(items) = self.directories.remove(&from_key) {
            self.directories.insert(comparable(to), items);
        }
        if self.active.as_deref().is_some_and(|a| comparable(a) == from_key) {
            self.active = Some(to.to_path_buf());
        }
        self.rebuild();
    }

    /// Что переносится, если тащат строку `index`: всё выделение, если строка в нём, иначе она одна
    /// (`targetsFor`).
    pub fn drag_sources(&self, index: usize) -> Vec<PathBuf> {
        let Some(row) = self.rows.get(index) else { return Vec::new() };
        if self.selected.len() > 1 && self.selected.contains(&comparable(&row.path)) {
            self.rows.iter().filter(|r| r.selected).map(|r| r.path.clone()).collect()
        } else {
            vec![row.path.clone()]
        }
    }

    /// Папка, куда упадёт бросок на строку `index`, если её можно принять для `source` —
    /// `canMove`: не в себя, не в текущую папку, не внутрь себя.
    pub fn drop_target(&self, source: &Path, index: usize) -> Option<PathBuf> {
        let row = self.rows.get(index)?;
        let target = if row.folder { row.path.clone() } else { row.path.parent()?.to_path_buf() };
        let (source_key, target_key) = (comparable(source), comparable(&target));
        if comparable(&row.path) == source_key || source.parent().map(comparable).as_deref() == Some(target_key.as_str()) {
            return None;
        }
        let inside = target_key == source_key || target_key.starts_with(&format!("{source_key}/"));
        (!inside).then_some(target)
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.rebuild();
    }

    fn select_range(&mut self, to: usize) {
        let anchor = self.anchor.as_deref().map(comparable);
        let from = anchor.and_then(|a| self.rows.iter().position(|r| comparable(&r.path) == a));
        let Some(from) = from else {
            self.selected = HashSet::from([comparable(&self.rows[to].path)]);
            self.anchor = Some(self.rows[to].path.clone());
            return;
        };
        let (start, end) = if from < to { (from, to) } else { (to, from) };
        self.selected = self.rows[start..=end].iter().map(|r| comparable(&r.path)).collect();
    }

    fn load(&mut self, dir: &Path) {
        match read_directory_impl(dir) {
            Ok(items) => {
                self.directories.insert(comparable(dir), items);
            }
            Err(e) => {
                // Папки больше нет — из кэша её содержимое тоже убирается.
                self.directories.remove(&comparable(dir));
                eprintln!("папка {} не прочитана: {e}", dir.display());
            }
        }
        // Раскрытые вложенные папки подгружаются сразу, как `openWorkspace` во фронтенде.
        let nested: Vec<PathBuf> = self.directories.get(&comparable(dir)).into_iter().flatten()
            .filter(|i| i.item_type == FileItemType::Folder && self.expanded.contains_key(&comparable(Path::new(&i.id))))
            .filter(|i| !self.directories.contains_key(&comparable(Path::new(&i.id))))
            .map(|i| PathBuf::from(&i.id))
            .collect();
        for folder in nested {
            self.load(&folder);
        }
    }

    /// `buildVisibleFileRows`: обход в глубину по раскрытым папкам.
    fn rebuild(&mut self) {
        let mut rows = Vec::new();
        if let Some(root) = &self.root
            && let Some(items) = self.directories.get(&comparable(root))
        {
            self.visit(items, 0, &[], &mut rows);
        }
        self.rows = rows;
    }

    fn visit(&self, items: &[FileItem], depth: usize, continuing: &[usize], rows: &mut Vec<Row>) {
        let active = self.active.as_deref().map(comparable);
        for (index, item) in items.iter().enumerate() {
            let path = PathBuf::from(&item.id);
            let key = comparable(&path);
            let folder = item.item_type == FileItemType::Folder;
            let expanded = folder && self.expanded.contains_key(&key);
            rows.push(Row {
                path,
                name: item.name.clone(),
                folder,
                depth,
                expanded,
                guides: continuing.to_vec(),
                active: active.as_deref() == Some(key.as_str()),
                selected: self.selected.contains(&key),
            });
            if expanded && let Some(children) = self.directories.get(&key).filter(|c| !c.is_empty()) {
                // У последнего элемента направляющая его уровня обрывается.
                let is_last = index == items.len() - 1;
                let mut guides: Vec<usize> = continuing
                    .iter()
                    .copied()
                    .filter(|&g| !(is_last && depth > 0 && g == depth - 1))
                    .collect();
                guides.push(depth);
                self.visit(children, depth + 1, &guides, rows);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> (tempfile::TempDir, TreeModel) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Проекты/Вложенная")).unwrap();
        std::fs::create_dir_all(root.join("Архив/Старое")).unwrap();
        std::fs::write(root.join("Архив/Старое/старое.md"), "").unwrap();
        std::fs::write(root.join("Проекты/Вложенная/глубоко.md"), "").unwrap();
        std::fs::write(root.join("Проекты/Aquilum.md"), "").unwrap();
        std::fs::write(root.join("README.md"), "").unwrap();
        std::fs::write(root.join("картинка.png"), "").unwrap();
        std::fs::write(root.join(".скрытый.md"), "").unwrap();
        let model = TreeModel::new(Some(root.to_path_buf()), []);
        (dir, model)
    }

    fn names(model: &TreeModel) -> Vec<(String, usize)> {
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
        model.press(1, Modifiers::default()); // Проекты
        model.press(2, Modifiers::default()); // Вложенная — не последняя: направляющая уровня 0 идёт дальше
        let deep = model.rows().iter().find(|r| r.name == "глубоко").unwrap();
        assert_eq!(deep.guides, vec![0, 1]);
        model.press(0, Modifiers::default()); // Архив
        model.press(1, Modifiers::default()); // Старое — единственная, значит последняя: уровень 0 обрывается
        let old = model.rows().iter().find(|r| r.name == "старое").unwrap();
        assert_eq!(old.guides, vec![1]);
    }

    #[test]
    fn drop_rules() {
        let (_dir, mut model) = tree();
        model.press(1, Modifiers::default()); // Проекты: Вложенная, Aquilum
        let index = |m: &TreeModel, name: &str| m.rows().iter().position(|r| r.name == name).unwrap();
        let projects = model.rows()[index(&model, "Проекты")].path.clone();
        let readme = model.rows()[index(&model, "README")].path.clone();
        // Заметку из корня — в папку можно, на соседнюю заметку корня (та же папка) — нельзя.
        assert_eq!(model.drop_target(&readme, index(&model, "Проекты")), Some(projects.clone()));
        assert_eq!(model.drop_target(&readme, index(&model, "картинка.png")), None);
        // Папку — в саму себя и в свою вложенную нельзя.
        assert_eq!(model.drop_target(&projects, index(&model, "Проекты")), None);
        assert_eq!(model.drop_target(&projects, index(&model, "Вложенная")), None);
        // Бросок на заметку внутри папки — в её папку.
        assert_eq!(model.drop_target(&readme, index(&model, "Aquilum")), Some(projects));
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
        // Якорь — последняя строка, по которой щёлкали с Ctrl: диапазон от неё до первой.
        assert!((0..=3).all(|i| model.rows()[i].selected));
    }
}
