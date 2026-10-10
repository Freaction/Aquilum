use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use aquilum_core::bases::{BaseColumn, BaseDefinition, BaseRows};
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId};

use super::App;
use crate::ui::editor::TextEditor;
use crate::ui::kanban::{KanbanAction, KanbanStatus, KanbanView};

struct LoadedBoard {
    definition: BaseDefinition,
    columns: Vec<BaseColumn>,
    selected_view: usize,
}

enum KanbanResult {
    Loaded(u64, PathBuf, Result<Option<LoadedBoard>, String>),
    Mutated(u64, PathBuf, Result<(), String>),
}

pub(super) struct KanbanState {
    path: Option<PathBuf>,
    definition: Option<BaseDefinition>,
    columns: Vec<BaseColumn>,
    selected_view: usize,
    generation: u64,
    loading: bool,
    failed: Option<String>,
    tx: Sender<KanbanResult>,
    rx: Receiver<KanbanResult>,
    actions: HashMap<WidgetId, KanbanAction>,
    pub(super) picker: Option<WidgetId>,
    renaming: Option<PathBuf>,
}

impl Default for KanbanState {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            path: None,
            definition: None,
            columns: Vec::new(),
            selected_view: 0,
            generation: 0,
            loading: false,
            failed: None,
            tx,
            rx,
            actions: HashMap::new(),
            picker: None,
            renaming: None,
        }
    }
}

fn collect_bases(directory: &Path, bases: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_bases(&path, bases)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("base"))
        {
            bases.push(path);
        }
    }
    Ok(())
}

pub(super) fn board_paths(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut bases = Vec::new();
    collect_bases(root, &mut bases)?;
    bases.sort();
    Ok(bases)
}

impl KanbanState {
    fn status(&self, has_workspace: bool) -> KanbanStatus {
        if !has_workspace {
            KanbanStatus::NoWorkspace
        } else if let Some(error) = &self.failed {
            KanbanStatus::Error(error.clone())
        } else if !self.loading && self.definition.is_some() {
            KanbanStatus::Ready
        } else {
            KanbanStatus::Loading
        }
    }

    fn invalidate(&mut self) {
        self.generation += 1;
        self.path = None;
        self.definition = None;
        self.columns.clear();
        self.loading = false;
        self.failed = None;
        self.actions.clear();
        self.renaming = None;
    }

    fn open(
        &mut self,
        root: &Path,
        path: &Path,
        selected_view: usize,
        force: bool,
        core: Arc<aquilum_core::Core>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) {
        if !force
            && self.path.as_deref() == Some(path)
            && (self.loading || self.definition.is_some())
        {
            return;
        }
        self.generation += 1;
        self.path = Some(path.to_path_buf());
        self.definition = None;
        self.columns.clear();
        self.selected_view = selected_view;
        self.loading = true;
        self.failed = None;
        self.actions.clear();
        let generation = self.generation;
        let path = path.to_path_buf();
        let workspace = root.to_string_lossy().into_owned();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let raw = fs::read_to_string(&path).map_err(|error| error.to_string())?;
                let definition = BaseDefinition::parse(&raw).map_err(|error| error.to_string())?;
                let rows = match core
                    .search
                    .base_rows(&workspace)
                    .map_err(|error| error.to_string())?
                {
                    Some(rows) => Some(rows),
                    None => {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        core.search
                            .base_rows(&workspace)
                            .map_err(|error| error.to_string())?
                    }
                };
                let Some(rows) = rows else { return Ok(None) };
                let selected_view = selected_view.min(definition.views.len().saturating_sub(1));
                let columns = BaseRows::evaluate(&definition, selected_view, &rows)
                    .map_err(|error| error.to_string())?;
                Ok(Some(LoadedBoard {
                    definition,
                    columns,
                    selected_view,
                }))
            })();
            if tx
                .send(KanbanResult::Loaded(generation, path, result))
                .is_ok()
            {
                wake();
            }
        });
    }
}

impl App {
    pub(super) fn invalidate_kanban(&mut self) {
        self.kanban.invalidate();
    }

    pub(super) fn kanban_picker_open(&self) -> bool {
        self.kanban.picker.is_some()
    }

    pub(super) fn open_base(
        &mut self,
        root: &mut RenderRoot,
        path: PathBuf,
        new_tab: bool,
    ) -> bool {
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else {
            return false;
        };
        let path = crate::workspace::normalize(&path);
        if !path.starts_with(&workspace)
            || !path.is_file()
            || path
                .extension()
                .is_none_or(|extension| !extension.eq_ignore_ascii_case("base"))
        {
            return false;
        }
        self.leave_note();
        if new_tab {
            self.tabs.open_new(path);
        } else {
            self.tabs.open(path);
        }
        self.show_active(root);
        true
    }

    pub(super) fn show_kanban(&mut self, _root: &mut RenderRoot) {
        let Some(path) = self
            .tabs
            .active()
            .path
            .clone()
            .filter(|_| self.tabs.active().is_base())
        else {
            return;
        };
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else {
            return;
        };
        let selected_view = self.session.as_ref().map_or(0, |session| {
            session.load_base_view(&self.workspace.core, &path, usize::MAX)
        });
        let core = Arc::clone(&self.workspace.core);
        let wake = Arc::clone(&self.workspace.wake);
        self.kanban
            .open(&workspace, &path, selected_view, false, core, wake);
    }

    pub(super) fn kanban_widget(&mut self) -> Option<NewWidget<dyn Widget>> {
        if !self.tabs.active().is_base() {
            self.kanban.actions.clear();
            return None;
        }
        let (widget, actions) = self.kanban_view();
        self.kanban.actions = actions.into_iter().collect();
        Some(widget)
    }

    fn kanban_view(&self) -> (NewWidget<dyn Widget>, Vec<(WidgetId, KanbanAction)>) {
        let editable = self
            .kanban
            .definition
            .as_ref()
            .is_some_and(|definition| definition.can_mutate_group(self.kanban.selected_view));
        let views = self
            .kanban
            .definition
            .as_ref()
            .map(|definition| {
                definition
                    .views
                    .iter()
                    .map(|view| view.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        KanbanView::new(
            &self.kanban.columns,
            self.kanban.status(self.tree.root().is_some()),
            editable,
            &views,
            self.kanban.selected_view,
            self.kanban.renaming.as_deref(),
        )
    }

    fn render_kanban(&mut self, root: &mut RenderRoot) {
        let Some(chrome) = &self.chrome else {
            return;
        };
        let (widget, actions) = self.kanban_view();
        let focus = self.kanban.renaming.as_ref().and_then(|path| {
            actions.iter().find_map(|(id, action)| {
                matches!(action, KanbanAction::CommitRenameCard(target) if target == path)
                    .then_some(*id)
            })
        });
        self.kanban.actions = actions.into_iter().collect();
        chrome.kanban_page.edit(root, |mut page| {
            crate::ui::widgets::Slot::set_child(&mut page, widget)
        });
        if let Some(id) = focus {
            root.edit_widget(id, |mut widget| {
                let mut field = widget.downcast::<TextEditor>();
                let len = field.widget.text().len();
                TextEditor::select_byte_range(&mut field, 0, len);
            });
            root.focus_on(Some(id));
        }
    }

    pub(super) fn on_kanban_results(&mut self, root: &mut RenderRoot) {
        while let Ok(result) = self.kanban.rx.try_recv() {
            let (generation, path) = match &result {
                KanbanResult::Loaded(generation, path, _)
                | KanbanResult::Mutated(generation, path, _) => (*generation, path.clone()),
            };
            if generation != self.kanban.generation
                || self.tabs.active().path.as_deref() != Some(path.as_path())
                || !self.tabs.active().is_base()
            {
                continue;
            }
            match result {
                KanbanResult::Loaded(_, _, Ok(Some(loaded))) => {
                    self.kanban.loading = false;
                    self.kanban.selected_view = loaded.selected_view;
                    self.kanban.columns = loaded.columns;
                    self.kanban.definition = Some(loaded.definition);
                    self.kanban.failed = None;
                }
                KanbanResult::Loaded(_, _, Ok(None)) => {
                    self.kanban.loading = false;
                    self.kanban.failed = Some(crate::i18n::t("kanban.indexing"));
                }
                KanbanResult::Loaded(_, _, Err(error))
                | KanbanResult::Mutated(_, _, Err(error)) => {
                    self.kanban.loading = false;
                    self.kanban.failed = Some(error);
                    self.kanban.definition = None;
                    self.kanban.columns.clear();
                }
                KanbanResult::Mutated(_, _, Ok(())) => {
                    self.reload_kanban();
                    continue;
                }
            }
            self.render_kanban(root);
        }
    }

    pub(super) fn is_kanban_widget(&self, id: WidgetId) -> bool {
        self.kanban.actions.contains_key(&id)
    }

    pub(super) fn on_kanban_action(
        &mut self,
        root: &mut RenderRoot,
        id: WidgetId,
        action: &ErasedAction,
    ) -> bool {
        let Some(kanban_action) = self.kanban.actions.get(&id).cloned() else {
            return false;
        };
        if let Some(text) = action.downcast_ref::<masonry::widgets::TextAction>() {
            let KanbanAction::CommitRenameCard(path) = kanban_action else {
                return false;
            };
            return match text {
                masonry::widgets::TextAction::Entered(name) => {
                    self.kanban.renaming = None;
                    if let (Some(workspace), Some((target, _))) = (
                        self.tree.root().map(PathBuf::from),
                        crate::file_ops::rename_target(&path, name),
                    ) {
                        let source = resolve_path(&workspace, path);
                        let target = resolve_path(&workspace, target);
                        self.mutate_kanban(move |core| {
                            aquilum_core::files::gate::rename(&core, &source, &target)
                                .map(|_| ())
                                .map_err(|error| error.to_string())
                        });
                    } else {
                        self.render_kanban(root);
                    }
                    true
                }
                masonry::widgets::TextAction::Cancelled => {
                    self.kanban.renaming = None;
                    self.render_kanban(root);
                    true
                }
                masonry::widgets::TextAction::Changed(_) => true,
            };
        }
        match kanban_action {
            KanbanAction::OpenBoard(path) => return self.open_base(root, path, true),
            KanbanAction::ClosePicker => return false,
            KanbanAction::CreateBoard => {
                self.create_board(root);
            }
            KanbanAction::RenameCard(path) => {
                self.kanban.renaming = Some(path);
                self.render_kanban(root);
            }
            KanbanAction::DeleteCard(path) => {
                let Some(workspace) = self.tree.root().map(PathBuf::from) else {
                    return false;
                };
                self.confirm_delete(root, vec![resolve_path(&workspace, path)]);
            }
            KanbanAction::CommitRenameCard(_) => return false,
            KanbanAction::DragCard { path, source } => {
                let Some(crate::ui::kanban::CardDragAction::Drop(at)) =
                    action.downcast_ref::<crate::ui::kanban::CardDragAction>()
                else {
                    return true;
                };
                let target = self.kanban.actions.iter().find_map(|(column_id, action)| {
                    let KanbanAction::DropColumn(value) = action else {
                        return None;
                    };
                    root.get_widget(*column_id).and_then(|column| {
                        let local = column.ctx().border_box();
                        let transform = column.ctx().window_transform();
                        let first = transform * masonry::kurbo::Point::new(local.x0, local.y0);
                        let second = transform * masonry::kurbo::Point::new(local.x1, local.y1);
                        let bounds = masonry::kurbo::Rect::new(
                            first.x.min(second.x),
                            first.y.min(second.y),
                            first.x.max(second.x),
                            first.y.max(second.y),
                        );
                        bounds.contains(*at).then(|| value.clone())
                    })
                });
                if let Some(target) = target.filter(|target| target != &source) {
                    let Some(workspace) = self.tree.root().map(PathBuf::from) else {
                        return true;
                    };
                    let Some(definition) = self.kanban.definition.clone() else {
                        return true;
                    };
                    let path = resolve_path(&workspace, path);
                    let view_index = self.kanban.selected_view;
                    self.mutate_kanban(move |core| {
                        core.move_base_card(
                            &workspace,
                            &definition,
                            view_index,
                            &path,
                            &source,
                            &target,
                        )
                        .map(|_| ())
                        .map_err(|error| error.to_string())
                    });
                }
            }
            KanbanAction::DropColumn(_) => return true,
            KanbanAction::OpenNote(path) => {
                let Some(workspace) = self.tree.root().map(PathBuf::from) else {
                    return false;
                };
                return self.open_in(root, resolve_path(&workspace, path), true);
            }
            KanbanAction::SelectView(index) => {
                let Some(board_path) = self.kanban.path.clone() else {
                    return false;
                };
                let Some(definition) = self.kanban.definition.clone() else {
                    return false;
                };
                if index < definition.views.len() {
                    self.kanban.selected_view = index;
                    if let Some(session) = &self.session {
                        session.save_base_view(&self.workspace.core, &board_path, index);
                    }
                    self.reload_kanban();
                }
            }
            KanbanAction::CreateCard { column } => {
                let Some(workspace) = self.tree.root().map(PathBuf::from) else {
                    return false;
                };
                let Some(definition) = self.kanban.definition.clone() else {
                    return false;
                };
                let view_index = self.kanban.selected_view;
                let path = unique_card_path(&workspace);
                let title = crate::i18n::t("kanban.untitled");
                self.mutate_kanban(move |core| {
                    core.create_base_card(
                        &workspace,
                        &definition,
                        view_index,
                        &path,
                        &format!("# {title}\n"),
                        &column,
                    )
                    .map(|_| ())
                    .map_err(|error| error.to_string())
                });
            }
            KanbanAction::MoveCard {
                path,
                expected,
                target,
            } => {
                let Some(workspace) = self.tree.root().map(PathBuf::from) else {
                    return false;
                };
                let Some(definition) = self.kanban.definition.clone() else {
                    return false;
                };
                let view_index = self.kanban.selected_view;
                let path = resolve_path(&workspace, path);
                self.mutate_kanban(move |core| {
                    core.move_base_card(
                        &workspace,
                        &definition,
                        view_index,
                        &path,
                        &expected,
                        &target,
                    )
                    .map(|_| ())
                    .map_err(|error| error.to_string())
                });
            }
        }
        let _ = action;
        true
    }

    pub(super) fn kanban_paths_changed(&mut self, paths: &[PathBuf]) {
        let Some(board) = self.kanban.path.as_deref() else {
            return;
        };
        let changed_board = paths.iter().any(|path| path == board);
        let Some(workspace) = self.tree.root() else {
            return;
        };
        let changed_card = paths.iter().any(|path| {
            self.kanban.columns.iter().any(|column| {
                column
                    .rows
                    .iter()
                    .any(|row| workspace.join(&row.path) == *path)
            })
        });
        if changed_board || changed_card {
            self.reload_kanban();
        }
    }

    pub(super) fn reload_kanban(&mut self) {
        let Some(path) = self.kanban.path.clone() else {
            return;
        };
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else {
            return;
        };
        let selected = self.kanban.selected_view;
        let core = Arc::clone(&self.workspace.core);
        let wake = Arc::clone(&self.workspace.wake);
        self.kanban.generation += 1;
        self.kanban.loading = false;
        self.kanban
            .open(&workspace, &path, selected, true, core, wake);
    }

    pub(super) fn kanban_index_changed(&mut self, workspace: &Path) {
        if self.tree.root().is_some_and(|root| root == workspace) && self.tabs.active().is_base() {
            self.reload_kanban();
        }
    }

    pub(super) fn open_boards(&mut self, root: &mut RenderRoot) {
        let Some(workspace) = self.tree.root() else {
            return;
        };
        match board_paths(workspace) {
            Ok(boards) => self.show_board_picker(root, boards),
            Err(error) => {
                self.kanban.failed = Some(error.to_string());
                self.replace_root(root);
            }
        }
    }

    fn create_board(&mut self, root: &mut RenderRoot) {
        let Some(workspace) = self.tree.root().map(PathBuf::from) else {
            return;
        };
        let template = "views:\n  - type: kanban\n    name: Board\n    groupBy:\n      property: note.status\n";
        let stem = crate::i18n::t("kanban.newBoardName")
            .trim()
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
        for index in 1usize.. {
            let name = if index == 1 {
                format!("{stem}.base")
            } else {
                format!("{stem} {index}.base")
            };
            let path = workspace.join(name);
            match aquilum_core::files::gate::create(
                &self.workspace.core,
                &path,
                template,
                aquilum_core::history::Source::Me,
                None,
            ) {
                Ok(_) => {
                    self.tree.paths_changed(root, std::slice::from_ref(&path));
                    let _ = self.open_base(root, path, true);
                    return;
                }
                Err(aquilum_core::files::error::FileCommandError::AlreadyExists { .. }) => continue,
                Err(error) => {
                    self.kanban.failed = Some(error.to_string());
                    self.replace_root(root);
                    return;
                }
            }
        }
    }

    fn show_board_picker(&mut self, root: &mut RenderRoot, boards: Vec<PathBuf>) {
        let (widget, actions) = crate::ui::kanban::BoardPicker::new(&boards);
        self.kanban.actions = actions.into_iter().collect();
        let layer = widget;
        let id = layer.id();
        root.add_layer(layer, masonry::kurbo::Point::ORIGIN);
        self.kanban.picker = Some(id);
    }

    pub(super) fn on_board_picker_action(&mut self, root: &mut RenderRoot, id: WidgetId) -> bool {
        let Some(action) = self.kanban.actions.get(&id).cloned() else {
            return false;
        };
        if let Some(layer) = self.kanban.picker.take() {
            root.remove_layer(layer);
        }
        match action {
            KanbanAction::OpenBoard(path) => self.open_base(root, path, true),
            KanbanAction::ClosePicker => true,
            KanbanAction::CreateBoard => {
                self.create_board(root);
                true
            }
            _ => false,
        }
    }

    fn mutate_kanban(
        &mut self,
        operation: impl FnOnce(Arc<aquilum_core::Core>) -> Result<(), String> + Send + 'static,
    ) {
        let core = Arc::clone(&self.workspace.core);
        let wake = Arc::clone(&self.workspace.wake);
        let tx = self.kanban.tx.clone();
        let generation = self.kanban.generation;
        let Some(path) = self.kanban.path.clone() else {
            return;
        };
        self.kanban.loading = true;
        std::thread::spawn(move || {
            let result = operation(core);
            if tx
                .send(KanbanResult::Mutated(generation, path, result))
                .is_ok()
            {
                wake();
            }
        });
    }
}

fn unique_card_path(workspace: &Path) -> PathBuf {
    let title = crate::i18n::t("kanban.untitled");
    let stem = title
        .trim()
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
    for index in 1usize.. {
        let name = if index == 1 {
            format!("{stem}.md")
        } else {
            format!("{stem} {index}.md")
        };
        let path = workspace.join(name);
        if !path.exists() {
            return path;
        }
    }
    unreachable!()
}

fn resolve_path(workspace: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{board_paths, resolve_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn resolves_relative_card_paths_from_workspace() {
        assert_eq!(
            resolve_path(Path::new("/vault"), PathBuf::from("nested/note.md")),
            PathBuf::from("/vault/nested/note.md")
        );
    }

    #[test]
    fn lists_nested_base_files_case_insensitively() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("nested/boards")).unwrap();
        std::fs::write(root.path().join("nested/boards/a.BASE"), "views: []").unwrap();
        std::fs::write(root.path().join("board.base"), "views: []").unwrap();
        std::fs::write(root.path().join("ignore.md"), "").unwrap();
        assert_eq!(board_paths(root.path()).unwrap().len(), 2);
    }
}
