use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aquilum_core::settings::models::AppConfig;
use aquilum_core::ui_state::models::SaveReaderStateInput;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, PropertySet, WidgetId};
use masonry::kurbo::Point;
use masonry::layout::Length;
use masonry::properties::Padding;

use super::book::synthetic_pages;
use super::{App, Outcome, Popup};
use crate::frontmatter::{self, BOOK_FILE};
use crate::reader::{page, quotes};
use crate::reader::session::{Session, Target};
use crate::ui::reader::{ReaderAction, ReaderIds, ReaderView};
use crate::ui::settings::{self, Binding, schema};
use crate::ui::tokens::size;
use crate::ui::widgets::Pressed;

const READER_POSITION: &str = "reader_position";
const READ_PERCENT: &str = "read_percent";

pub(super) struct ReaderState {
    pub(super) layer: WidgetId,
    pub(super) ids: ReaderIds,
    page: Option<PathBuf>,
    key: String,
    workspace: Option<uuid::Uuid>,
    total: u32,
    pub(super) bindings: HashMap<WidgetId, Binding>,
    peek: bool,
    target: Option<PathBuf>,
}

pub(super) struct Opening {
    pub file: PathBuf,
    pub page: Option<PathBuf>,
    pub title: String,
    pub initial: Option<String>,
    pub peek: bool,
}

pub fn reader_settings(config: &AppConfig) -> page::Settings {
    let r = &config.reader;
    page::Settings {
        family: r.font.font_family.clone(),
        size: r.font.font_size_base as f32,
        weight: r.font.font_weight as f32,
        line_height: r.line_height,
        justify: r.justify,
        hyphenate: r.hyphenate,
        max_width_ch: r.max_width_ch,
        margin: f64::from(r.margin_px),
        scrolled: r.flow == "scrolled",
    }
}

fn parse_pages(value: &str) -> Option<(u32, u32)> {
    let (a, b) = value.split_once('/')?;
    let (current, total) = (a.trim().parse().ok()?, b.trim().parse::<u32>().ok()?);
    (total > 0).then_some((current, total))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

impl App {
    pub(super) fn open_reader(&mut self, root: &mut RenderRoot, opening: Opening) {
        let Opening { file, page, title, initial, peek } = opening;
        let file = file.as_path();
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else { return };
        let fields = page.as_ref().and_then(|p| std::fs::read_to_string(p).ok());
        let fields = fields.as_deref().and_then(frontmatter::parse);
        let key = fields
            .as_ref()
            .and_then(|f| f.text(BOOK_FILE).map(str::to_owned))
            .filter(|f| !f.trim().is_empty())
            .unwrap_or_else(|| file.strip_prefix(&workspace).unwrap_or(file).to_string_lossy().replace('\\', "/"));
        let bytes = std::fs::metadata(file).map_or(0, |m| m.len());
        let existing = fields.as_ref().and_then(|f| f.text("pages").map(str::to_owned));
        let formatted = synthetic_pages(existing.as_deref(), bytes);
        let (current, total) = parse_pages(&formatted).unwrap_or((0, 1));
        if let Some(page) = &page
            && existing.as_deref() != Some(formatted.as_str())
        {
            self.write_book_fields(root, page, &[("pages", formatted.clone())]);
        }
        let workspace_id = self.workspace.core.ui_state.resolve_workspace(&workspace.to_string_lossy(), now_ms()).ok();
        let cached = workspace_id.and_then(|id| self.workspace.core.ui_state.load_reader_state(id, &key).ok().flatten());
        let target = match (initial.filter(|c| !c.trim().is_empty()), cached) {
            (Some(cfi), _) => Target::Cfi(cfi),
            _ if current == 0 => Target::Start,
            (None, Some(state)) if state.current == i64::from(current) && state.cfi.is_some() => Target::Cfi(state.cfi.unwrap_or_default()),
            _ => Target::Fraction(f64::from(current) / f64::from(total)),
        };
        let Some(session) = Session::open(file, target) else {
            eprintln!("книга не открылась: {}", file.display());
            return;
        };
        let settings = reader_settings(&self.workspace.core.settings.get_config());
        let target = self.note.as_ref().map(|n| n.path.clone());
        let (view, ids) = ReaderView::new(session, settings, title, total, target.is_some());
        let layer = view.id();
        root.add_layer(view, Point::ORIGIN);
        root.focus_on(Some(layer));
        let state = ReaderState { layer, ids, page, key, workspace: workspace_id, total, bindings: HashMap::new(), peek, target };
        self.refresh_reader_quotes(root, &state);
        self.popup = Some(Popup::Reader(Box::new(state)));
    }

    fn write_book_fields(&mut self, root: &mut RenderRoot, page: &Path, fields: &[(&str, String)]) {
        if self.note.as_ref().is_some_and(|n| n.is(page)) {
            self.set_fields(root, fields);
            return;
        }
        let core = &self.workspace.core;
        let Ok(snapshot) = aquilum_core::files::document::read_file_snapshot_impl(page) else { return };
        let mut text = snapshot.content.clone();
        for (key, value) in fields {
            match frontmatter::set_field(&text, key, value) {
                Some(next) => text = next,
                None => return,
            }
        }
        if text != snapshot.content
            && let Err(error) = aquilum_core::files::gate::write(core, page, &text, Some(&snapshot.hash), aquilum_core::history::Source::Me, None)
        {
            eprintln!("{} не сохранён: {error:?}", page.display());
        }
    }

    pub(super) fn on_reader_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Reader(mut state)) = self.popup.take() else { return Outcome::default() };
        if let Some(action) = action.downcast_ref::<ReaderAction>() {
            match action {
                ReaderAction::Close => {
                    root.remove_layer(state.layer);
                    return Outcome::default();
                }
                ReaderAction::Settings => self.toggle_reader_settings(root, &mut state),
                ReaderAction::Moved { cfi, fraction } => self.reader_moved(root, &state, cfi, *fraction),
                ReaderAction::QuoteRef(cfi) => {
                    root.remove_layer(state.layer);
                    self.reveal_quote(root, cfi);
                    return Outcome::default();
                }
            }
        } else if action.downcast_ref::<Pressed>().is_some() && Some(id) == state.ids.quote {
            let quote = root.get_widget(state.layer).and_then(|w| w.downcast::<ReaderView>().and_then(|v| v.inner().quote()));
            if let Some((text, cfi)) = quote {
                self.add_quote(root, &state, &text, &cfi);
            }
        } else if action.downcast_ref::<Pressed>().is_some() && id == state.ids.close {
            root.remove_layer(state.layer);
            return Outcome::default();
        } else if action.downcast_ref::<Pressed>().is_some() && id == state.ids.settings {
            self.toggle_reader_settings(root, &mut state);
        } else {
            let (_, _) = self.on_binding(root, &mut state.bindings, id, action);
            self.typing = None;
            let settings = reader_settings(&self.workspace.core.settings.get_config());
            root.edit_widget(state.layer, |mut w| ReaderView::set_settings(&mut w.downcast(), settings));
        }
        self.popup = Some(Popup::Reader(state));
        Outcome::default()
    }

    fn toggle_reader_settings(&mut self, root: &mut RenderRoot, state: &mut ReaderState) {
        let open = root.get_widget(state.layer).is_some_and(|w| w.downcast::<ReaderView>().is_some_and(|v| v.inner().has_popover()));
        if open {
            state.bindings.clear();
            root.edit_widget(state.layer, |mut w| ReaderView::set_popover(&mut w.downcast(), None));
            root.focus_on(Some(state.layer));
            return;
        }
        let config = self.workspace.core.settings.get_config();
        let (rows, bindings) = settings::rows(schema::reader_quick(&config));
        state.bindings = bindings;
        let padded = NewWidget::new(masonry::widgets::Flex::column().cross_axis_alignment(masonry::properties::types::CrossAxisAlignment::Stretch).with_fixed(rows))
            .with_props(PropertySet::new().with(Padding::from_vh(Length::px(size::SPACE_4), Length::px(size::SPACE_12))));
        root.edit_widget(state.layer, |mut w| ReaderView::set_popover(&mut w.downcast(), Some(padded.erased())));
    }

    fn note_text(&self, path: &Path) -> Option<String> {
        if self.note.as_ref().is_some_and(|n| n.is(path)) {
            return Some(self.editor_text.clone());
        }
        std::fs::read_to_string(path).ok()
    }

    fn refresh_reader_quotes(&mut self, root: &mut RenderRoot, state: &ReaderState) {
        let text = state.target.as_deref().and_then(|p| self.note_text(p)).unwrap_or_default();
        let marks: Vec<(String, String)> = quotes::find(&text).into_iter().filter(|q| q.book.as_deref().is_none_or(|b| b == state.key)).map(|q| (q.cfi, q.label)).collect();
        root.edit_widget(state.layer, |mut w| ReaderView::set_quotes(&mut w.downcast(), marks));
    }

    fn add_quote(&mut self, root: &mut RenderRoot, state: &ReaderState, text: &str, cfi: &str) {
        let Some(target) = state.target.clone() else { return };
        let Some(current) = self.note_text(&target) else { return };
        let line = quotes::format(text, cfi, Some(&state.key), quotes::count(&current) + 1);
        let next = quotes::append(&current, &line);
        if self.note.as_ref().is_some_and(|n| n.is(&target)) {
            self.editor_text = next.clone();
            if let Some(note) = &mut self.note {
                note.edit(next, std::time::Duration::ZERO);
                note.save(&self.workspace.core);
            }
            self.sync_body(root);
        } else {
            let core = &self.workspace.core;
            let hash = aquilum_core::files::document::read_file_snapshot_impl(&target).ok().map(|s| s.hash);
            if let Err(error) = aquilum_core::files::gate::write(core, &target, &next, hash.as_deref(), aquilum_core::history::Source::Me, None) {
                eprintln!("{} не сохранён: {error:?}", target.display());
                return;
            }
        }
        root.edit_widget(state.layer, |mut w| ReaderView::clear_selection(&mut w.downcast()));
        self.refresh_reader_quotes(root, state);
    }

    fn reveal_quote(&mut self, root: &mut RenderRoot, cfi: &str) {
        let Some(quote) = quotes::find(&self.editor_text).into_iter().find(|q| q.cfi == cfi) else { return };
        let Some(ids) = &self.note_ids else { return };
        let hidden = self.hidden;
        let (start, end) = (quote.text.start.saturating_sub(hidden), quote.text.end.saturating_sub(hidden));
        let body = ids.body.id();
        root.focus_on(Some(body));
        ids.body.edit(root, |mut editor| crate::ui::editor::TextEditor::select_byte_range(&mut editor, start, end));
    }

    pub(super) fn open_reader_link(&mut self, root: &mut RenderRoot, url: &str) -> bool {
        let Some((cfi, book)) = quotes::parse_href(url) else { return false };
        let own = frontmatter::parse(&self.editor_text).and_then(|f| f.text(BOOK_FILE).map(str::to_owned)).filter(|f| !f.trim().is_empty());
        let Some(book_file) = book.clone().or_else(|| own.clone()) else { return true };
        let Some(file) = crate::vault_files::resolve(self.tree.root(), &book_file) else { return true };
        let same = book.is_none() || book == own;
        let title = if same { self.note.as_ref().map(|n| n.path.file_stem().unwrap_or_default().to_string_lossy().into_owned()).unwrap_or_default() } else { crate::i18n::t("reader.book") };
        self.open_reader(root, Opening { file, page: None, title, initial: Some(cfi), peek: true });
        true
    }

    fn reader_moved(&mut self, root: &mut RenderRoot, state: &ReaderState, cfi: &str, fraction: f64) {
        if state.peek {
            return;
        }
        let current = ((fraction.clamp(0.0, 1.0) * f64::from(state.total)).round() as u32).min(state.total);
        if let Some(workspace_id) = state.workspace {
            let input = SaveReaderStateInput { workspace_id, book_file: state.key.clone(), current: i64::from(current), cfi: Some(cfi.to_owned()), now_ms: now_ms() };
            if let Err(error) = self.workspace.core.ui_state.save_reader_state(&input) {
                eprintln!("позиция чтения не сохранена: {error:?}");
            }
        }
        let Some(page) = state.page.clone() else { return };
        let percent = ((f64::from(current) / f64::from(state.total)) * 100.0).round().clamp(0.0, 100.0);
        self.write_book_fields(root, &page, &[("pages", format!("{current}/{}", state.total)), (READER_POSITION, cfi.to_owned()), (READ_PERCENT, format!("{percent}"))]);
    }
}
