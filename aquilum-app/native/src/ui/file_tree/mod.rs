//! Дерево файлов боковой панели — перенос `src/components/Layout/{Sidebar,FileTree}.tsx`.
//!
//! Модель (`model.rs`) хранит кэш папок и состояние, строки (`row.rs`) рисуют себя сами.
//! После раскрытия папки или смены выделения список строк собирается заново и подменяется
//! в прокрутке (`Portal::set_child`): у `Flex` в этой ревизии Masonry нет удаления детей.
//! Положение прокрутки при этом сохраняется, как во фронтенде.

mod ghost;
pub mod model;
mod row;

use std::collections::BTreeMap;
use crate::ui::editor::TextEditor;
use std::path::{Path, PathBuf};

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, PropertySet, WidgetId};
use masonry::layout::Length;
use masonry::parley::LineHeight;
use masonry::parley::style::StyleProperty;
use masonry::properties::{CaretColor, ContentColor, Gap, SelectionColor};

use crate::ui::scroll::{ScrollArea, Scrolled};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, SizedBox, TextAction};
use serde::{Deserialize, Serialize};

pub use model::Press;
use model::{TreeModel, comparable};
use row::{RowAction, TreeRow};

use crate::ui::handle::Handle;
use crate::ui::tokens::size;


/// Что сделать окну после действия в дереве.
pub enum TreeAction {
    /// Меню действий над `targets` у точки `at`; `folder` — щёлкнули по папке.
    ContextMenu { targets: Vec<PathBuf>, clicked: PathBuf, folder: bool, at: masonry::kurbo::Point },
    /// Открыть заметку; Ctrl+Shift+щелчок — в новой вкладке.
    Open { path: PathBuf, new_tab: bool },
    /// Переименовать `from` в `to` (имя уже очищено); дерево показывает новое имя сразу.
    Rename { from: PathBuf, to: PathBuf },
    /// Перенести `sources` в папку `target` (бросок перетаскивания).
    Move { sources: Vec<PathBuf>, target: PathBuf },
    /// «Создать заметку» в пустой базе.
    CreateNote,
}

/// Идущее перетаскивание.
struct Drag {
    source: PathBuf,
    sources: Vec<PathBuf>,
    ghost: WidgetId,
    /// Подсвеченная строка-цель и папка, куда упадёт бросок.
    over: Option<(WidgetId, PathBuf)>,
}

pub struct FileTree {
    model: TreeModel,
    drag: Option<Drag>,
    /// Кнопка «Создать заметку» пустого состояния, если оно показано.
    create_button: Option<WidgetId>,
    /// id строк последней сборки по порядку: по id нажатой строки находим её индекс.
    row_ids: Vec<WidgetId>,
    /// Прокрутка со строками — последняя собранная `widget()`.
    scroll: Option<Handle<ScrollArea<Flex>>>,
    /// Поле переименования, пока оно показано.
    rename: Option<Handle<TextEditor>>,
    store: ExpandedStore,
    first: usize,
    offset: f64,
}

const SPAN: usize = 160;
const MARGIN: usize = 40;

fn step() -> f64 {
    size::SIDEBAR_ITEM_PADDING * 2.0 + size::SIDEBAR_ICON_SIZE + size::SPACE_2
}

impl FileTree {
    pub fn new(root: Option<PathBuf>, data_dir: &Path) -> Self {
        let store = ExpandedStore::load(data_dir);
        let expanded = root.as_deref().map(|r| store.get(r)).unwrap_or_default();
        FileTree {
            model: TreeModel::new(root, expanded),
            drag: None,
            create_button: None,
            row_ids: Vec::new(),
            scroll: None,
            rename: None,
            store,
            first: 0,
            offset: 0.0,
        }
    }

    #[cfg(test)]
    pub fn built_rows(&self) -> std::ops::Range<usize> {
        self.first..self.first + self.row_ids.len()
    }

    #[cfg(test)]
    pub fn scroll_id(&self) -> Option<WidgetId> {
        self.scroll.map(|s| s.id())
    }

    pub fn root(&self) -> Option<&Path> {
        self.model.root()
    }

    pub fn widget(&mut self) -> NewWidget<ScrollArea<Flex>> {
        // `fill`: пустое состояние стоит по центру панели, как `margin: auto` во фронтенде.
        let widget = NewWidget::new(ScrollArea::new(self.list()).fill(true).at(self.offset));
        self.scroll = Some(Handle::of(&widget));
        widget
    }

    fn list(&mut self) -> NewWidget<Flex> {
        self.row_ids.clear();
        self.create_button = None;
        self.rename = None;
        if self.model.root().is_some() && self.model.rows().is_empty() {
            let (state, button) = empty_state();
            self.create_button = Some(button);
            return state;
        }
        let mut list = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        let renaming = self.model.renaming().map(comparable);
        let count = self.model.rows().len();
        self.first = self.first.min(count.saturating_sub(1));
        let last = (self.first + SPAN).min(count);
        if self.first > 0 {
            list = list.with_fixed(NewWidget::new(SizedBox::empty().height(Length::px(self.first as f64 * step() - size::SPACE_2))));
        }
        for row in &self.model.rows()[self.first..last] {
            if renaming.as_deref() == Some(comparable(&row.path).as_str()) {
                let (field, rename) = rename_field(row);
                self.row_ids.push(field.id());
                self.rename = Some(rename);
                list = list.with_fixed(field);
                continue;
            }
            let widget = NewWidget::new(TreeRow::new(row.clone()));
            self.row_ids.push(widget.id());
            list = list.with_fixed(widget);
        }
        if last < count {
            list = list.with_fixed(NewWidget::new(SizedBox::empty().height(Length::px((count - last) as f64 * step() - size::SPACE_2))));
        }
        // `.q-sidebar-content`: строки через 2 px, отступы 8 8 0.
        NewWidget::new(list).with_props(
            PropertySet::new()
                .with(Gap::new(Length::px(size::SPACE_2)))
                .with(masonry::properties::Padding {
                    top: Length::px(size::SPACE_8),
                    left: Length::px(size::SPACE_8),
                    right: Length::px(size::SPACE_8),
                    bottom: Length::ZERO,
                }),
        )
    }

    /// Пересобирает строки в окне; прокрутка остаётся на месте.
    pub fn refresh(&mut self, root: &mut RenderRoot) {
        let list = self.list();
        if let Some(scroll) = self.scroll {
            scroll.edit(root, |mut scroll| ScrollArea::set_child(&mut scroll, list));
        }
    }

    /// Действие строки дерева; `None`, если действие не от строки.
    fn show_rows_from(&mut self, root: &mut RenderRoot, offset: f64) {
        let visible = (offset / step()) as usize;
        let first = visible.saturating_sub(MARGIN);
        if first.abs_diff(self.first) > MARGIN / 2 {
            self.first = first;
            self.refresh(root);
        }
    }

    pub fn on_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<TreeAction> {
        if let Some(Scrolled(offset)) = action.downcast_ref::<Scrolled>() {
            if self.scroll.is_some_and(|s| s.id() == id) {
                self.offset = *offset;
                self.show_rows_from(root, *offset);
            }
            return None;
        }
        if Some(id) == self.create_button && action.downcast_ref::<crate::ui::widgets::Pressed>().is_some() {
            return Some(TreeAction::CreateNote);
        }
        if let Some(text) = action.downcast_ref::<TextAction>() {
            return match text {
                TextAction::Entered(name) => self.commit_rename(root, name),
                TextAction::Cancelled => {
                    self.model.stop_rename();
                    self.refresh(root);
                    None
                }
                TextAction::Changed(_) => None,
            };
        }
        let action = action.downcast_ref::<RowAction>()?;
        // Щелчок по другой строке во время переименования применяет его — как потеря фокуса
        // полем во фронтенде.
        if self.model.renaming().is_some() {
            let name = self.rename.and_then(|field| field.get(root)).map(|f| f.inner().text().to_string());
            if let Some(name) = name {
                return self.commit_rename(root, &name);
            }
        }
        let index = self.first + self.row_ids.iter().position(|&row| row == id)?;
        let modifiers = match action {
            RowAction::Press(modifiers) => *modifiers,
            RowAction::DragStart(at) => {
                self.start_drag(root, index, *at);
                return None;
            }
            RowAction::DragMove(at) => {
                self.drag_to(root, *at);
                return None;
            }
            RowAction::Drop(at) => {
                self.drag_to(root, *at);
                return self.drop(root);
            }
            RowAction::ContextMenu(at) => {
                let row = self.model.rows()[index].clone();
                // Как во фронтенде: меню есть у папок и заметок, у вложений — нет.
                let note = row.path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"));
                if !row.folder && !note {
                    return None;
                }
                let targets = self.model.focus(index);
                self.refresh(root);
                return Some(TreeAction::ContextMenu { targets, clicked: row.path, folder: row.folder, at: *at });
            }
        };
        match self.model.press(index, modifiers)? {
            Press::Rebuild => {
                self.save_expanded();
                self.refresh(root);
                None
            }
            Press::Open { path, new_tab } => Some(TreeAction::Open { path, new_tab }),
        }
    }

    /// Открытая заметка подсвечивается, становится якорем выделения и прокручивается в видимую
    /// область — ровно настолько, чтобы показаться целиком.
    pub fn set_active(&mut self, root: &mut RenderRoot, path: Option<PathBuf>) {
        self.model.set_active(path);
        let active = self.model.rows().iter().position(|r| r.active);
        if let Some(index) = active
            && !(self.first..self.first + SPAN - MARGIN).contains(&index)
        {
            self.first = index.saturating_sub(MARGIN);
        }
        self.refresh(root);
        if let Some(index) = active {
            let top = size::SPACE_8 + index as f64 * step();
            let bottom = top + step() - size::SPACE_2;
            if let Some(offset) = self.scroll.and_then(|scroll| scroll.edit(root, |mut scroll| {
                ScrollArea::reveal(&mut scroll, top, bottom);
                scroll.widget.offset()
            })) {
                self.offset = offset;
            }
        }
    }

    fn start_drag(&mut self, root: &mut RenderRoot, index: usize, at: masonry::kurbo::Point) {
        let Some(row) = self.model.rows().get(index) else { return };
        let source = row.path.clone();
        let sources = self.model.drag_sources(index);
        let label = if sources.len() > 1 { crate::i18n::plural("fileTree.dragCount", sources.len() as u64) } else { row.name.clone() };
        let ghost = ghost::DragGhost::new(&label);
        let id = ghost.id();
        root.add_layer(ghost, at + masonry::kurbo::Vec2::new(ghost::OFFSET, ghost::OFFSET));
        self.drag = Some(Drag { source, sources, ghost: id, over: None });
    }

    /// Плашка идёт за указателем, строка под ним подсвечивается, если принимает бросок.
    fn drag_to(&mut self, root: &mut RenderRoot, at: masonry::kurbo::Point) {
        let Some(drag) = &mut self.drag else { return };
        root.reposition_layer(drag.ghost, at + masonry::kurbo::Vec2::new(ghost::OFFSET, ghost::OFFSET));
        let under = root.get_layer_root(0).find_widget_under_pointer(at).map(|w| w.id());
        let target = under
            .and_then(|id| self.row_ids.iter().position(|&row| row == id))
            .and_then(|index| Some((self.row_ids[index], self.model.drop_target(&drag.source, self.first + index)?)));
        let previous = drag.over.as_ref().map(|(id, _)| *id);
        let next = target.as_ref().map(|(id, _)| *id);
        if previous != next {
            for (id, on) in [(previous, false), (next, true)] {
                if let Some(id) = id {
                    root.edit_widget(id, |mut w| TreeRow::set_drop_over(&mut w.downcast::<TreeRow>(), on));
                }
            }
        }
        drag.over = target;
    }

    fn drop(&mut self, root: &mut RenderRoot) -> Option<TreeAction> {
        let drag = self.drag.take()?;
        root.remove_layer(drag.ghost);
        let (row, target) = drag.over?;
        root.edit_widget(row, |mut w| TreeRow::set_drop_over(&mut w.downcast::<TreeRow>(), false));
        Some(TreeAction::Move { sources: drag.sources, target })
    }

    /// «Переименовать» из меню: строка превращается в поле с выделенным именем.
    pub fn start_rename(&mut self, root: &mut RenderRoot, path: PathBuf) {
        self.model.start_rename(path);
        self.refresh(root);
        let Some(field) = self.rename else { return };
        field.edit(root, |mut field| {
            let len = field.widget.text().to_string().len();
            TextEditor::select_byte_range(&mut field, 0, len);
        });
        root.focus_on(Some(field.id()));
    }

    /// `commitRename`: пустое или прежнее имя — отмена; заметке имя очищается и
    /// добавляется `.md`, папке — как ввели.
    fn commit_rename(&mut self, root: &mut RenderRoot, input: &str) -> Option<TreeAction> {
        let from = self.model.renaming()?.to_path_buf();
        self.model.stop_rename();
        let Some((to, name)) = crate::file_ops::rename_target(&from, input) else {
            self.refresh(root);
            return None;
        };
        self.model.patch(&from, &to, &name);
        self.save_expanded();
        self.refresh(root);
        Some(TreeAction::Rename { from, to })
    }

    pub fn clear_selection(&mut self, root: &mut RenderRoot) {
        self.model.clear_selection();
        self.refresh(root);
    }

    /// Файлы на диске изменились (`workspace-changed` от ядра).
    pub fn paths_changed(&mut self, root: &mut RenderRoot, paths: &[PathBuf]) {
        self.model.refresh_paths(paths);
        self.refresh(root);
    }

    fn save_expanded(&mut self) {
        if let Some(root) = self.model.root().map(Path::to_path_buf) {
            self.store.set(&root, &self.model.expanded_paths());
        }
    }
}

/// Пустая база (`EmptyState` compact в `Sidebar.tsx`): иконка, «Пока ни одной заметки», кнопка
/// «Создать заметку». Возвращает и id кнопки.
fn empty_state() -> (NewWidget<Flex>, WidgetId) {
    use crate::ui::widgets::{IconView, TextButton, Variant};
    let t = crate::ui::theme::current();
    let font = crate::ui::theme::ui_font_settings();
    let icon = IconView::new(crate::ui::icons::FILE_TEXT, size::SIZE_40, size::SIZE_28, 1.4, || crate::ui::theme::current().icon_muted);
    let title = masonry::widgets::Label::new(crate::i18n::t("fileTree.empty"))
        .with_style(StyleProperty::FontFamily(crate::ui::theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(font.size))
        .with_style(StyleProperty::LineHeight(LineHeight::FontSizeRelative(crate::ui::tokens::number::FONT_LINE_HEIGHT_SNUG as f32)));
    let title = NewWidget::new(title).with_props(PropertySet::new().with(ContentColor::new(t.text_secondary)));
    let button = NewWidget::new(TextButton::new(crate::i18n::t("editor.createNote"), Variant::Primary));
    let id = button.id();
    let state = Flex::column()
        .main_axis_alignment(masonry::properties::types::MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(icon)
        .with_fixed(title)
        .with_fixed(button);
    let state = NewWidget::new(state).with_props(
        PropertySet::new()
            .with(Gap::new(Length::px(size::EMPTY_STATE_GAP)))
            .with(masonry::properties::Padding::from_vh(Length::px(size::SPACE_16), Length::px(size::SPACE_8))),
    );
    (state, id)
}

/// Поле переименования на месте строки (`FileTreeRename.tsx`): тот же отступ, что у имени,
/// шрифт и высота строки как у строки дерева, без рамки и фона.
fn rename_field(row: &model::Row) -> (NewWidget<Flex>, Handle<TextEditor>) {
    let t = crate::ui::theme::current();
    let font = crate::ui::theme::ui_font_settings();
    let indent = size::SIDEBAR_ITEM_PADDING
        + row.depth as f64 * size::SIDEBAR_TREE_INDENT
        + size::SIDEBAR_ICON_SIZE
        + size::SIDEBAR_ITEM_GAP;
    let name = if row.folder {
        row.name.clone()
    } else {
        row.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    };
    let field = TextEditor::single_line(&name)
        .with_style(StyleProperty::FontFamily(crate::ui::theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(font.size))
        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(size::SIDEBAR_ICON_SIZE as f32)));
    let field = NewWidget::new(field).with_props(
        PropertySet::new()
            .with(ContentColor::new(t.sidebar_text))
            .with(CaretColor { color: t.text_primary })
            .with(SelectionColor { color: t.selection_bg }),
    );
    let handle = Handle::of(&field);
    let row = Flex::row().with_fixed(NewWidget::new(SizedBox::empty().width(Length::px(indent)))).with(field, 1.0);
    let row = NewWidget::new(row).with_props(PropertySet::new().with(masonry::properties::Padding {
        top: Length::px(size::SIDEBAR_ITEM_PADDING),
        bottom: Length::px(size::SIDEBAR_ITEM_PADDING),
        left: Length::ZERO,
        right: Length::px(size::SIDEBAR_ITEM_PADDING),
    }));
    (row, handle)
}

/// Раскрытые папки по базам знаний — как `aquilum_expanded_folders:<база>` в localStorage
/// фронтенда: пути относительно корня базы. Файл `expanded-folders.json` в каталоге данных.
struct ExpandedStore {
    path: PathBuf,
    workspaces: BTreeMap<String, Vec<String>>,
}

#[derive(Default, Deserialize, Serialize)]
struct StoreFile {
    workspaces: BTreeMap<String, Vec<String>>,
}

impl ExpandedStore {
    fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("expanded-folders.json");
        let workspaces = std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<StoreFile>(&json).ok())
            .unwrap_or_default()
            .workspaces;
        ExpandedStore { path, workspaces }
    }

    fn get(&self, root: &Path) -> Vec<PathBuf> {
        self.workspaces
            .get(&comparable(root))
            .into_iter()
            .flatten()
            .map(|relative| root.join(relative))
            .collect()
    }

    fn set(&mut self, root: &Path, expanded: &[PathBuf]) {
        let relative = expanded
            .iter()
            .filter_map(|p| p.strip_prefix(root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        self.workspaces.insert(comparable(root), relative);
        let file = StoreFile { workspaces: self.workspaces.clone() };
        let Ok(json) = serde_json::to_string(&file) else { return };
        if let Err(e) = aquilum_core::files::document::write_file_atomic_impl(&self.path, &json, None) {
            eprintln!("раскрытые папки не сохранены: {e}");
        }
    }
}
