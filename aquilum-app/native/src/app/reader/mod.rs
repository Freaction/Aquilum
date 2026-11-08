mod position;
mod quote;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aquilum_core::history::Source;
use aquilum_core::settings::models::AppConfig;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, PropertySet, WidgetId};
use masonry::kurbo::Point;
use masonry::layout::Length;
use masonry::properties::Padding;

use super::book::synthetic_pages;
use super::{App, Outcome, Popup};
use crate::frontmatter::{self, BOOK_FILE, PAGES};
use crate::reader::page;
use crate::reader::session::{Session, Target};
use crate::ui::reader::{ReaderAction, ReaderIds, ReaderView};
use crate::ui::settings::{self, Binding, schema};
use crate::ui::tokens::size;
use crate::ui::widgets::Pressed;

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
    moved: Option<position::Moved>,
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

impl App {
    pub(super) fn open_reader(&mut self, root: &mut RenderRoot, opening: Opening) {
        let Opening { file, page, title, initial, peek } = opening;
        let file = file.as_path();
        let Some(workspace) = self.tree.root().map(Path::to_path_buf) else { return };
        let fields = page.as_ref().and_then(|p| self.note_head(p));
        let fields = fields.as_deref().and_then(frontmatter::parse);
        let key = fields
            .as_ref()
            .and_then(|f| f.text(BOOK_FILE).map(str::to_owned))
            .filter(|f| !f.trim().is_empty())
            .unwrap_or_else(|| file.strip_prefix(&workspace).unwrap_or(file).to_string_lossy().replace('\\', "/"));
        let bytes = std::fs::metadata(file).map_or(0, |m| m.len());
        let existing = fields.as_ref().and_then(|f| f.text(PAGES).map(str::to_owned));
        let pdf = crate::reader::pdf::is_pdf(file);
        let formatted = if pdf { existing.clone().unwrap_or_else(|| "0/1".into()) } else { synthetic_pages(existing.as_deref(), bytes) };
        let (current, total) = frontmatter::parse_pages(&formatted).unwrap_or((0, 1));
        if let Some(page) = &page
            && !pdf
            && existing.as_deref() != Some(formatted.as_str())
        {
            self.write_fields(root, page, Source::Reading, &[(PAGES, formatted.clone())]);
        }
        let workspace_id = self.workspace.core.ui_state.resolve_workspace(&workspace.to_string_lossy(), crate::dates::now_ms()).ok();
        let cached = workspace_id.and_then(|id| self.workspace.core.ui_state.load_reader_state(id, &key).ok().flatten());
        let active = initial.filter(|c| !c.trim().is_empty());
        let target_position = match (active.clone(), cached) {
            (Some(cfi), _) => Target::Cfi(cfi),
            (None, Some(state)) if pdf && page.is_none() && state.cfi.is_some() => Target::Cfi(state.cfi.unwrap_or_default()),
            _ if current == 0 => Target::Start,
            (None, Some(state)) if state.current == i64::from(current) && state.cfi.is_some() => Target::Cfi(state.cfi.unwrap_or_default()),
            _ => Target::Fraction(f64::from(current) / f64::from(total)),
        };
        let settings = reader_settings(&self.workspace.core.settings.get_config());
        let target = self.note.as_ref().map(|n| n.path.clone());
        let (view, ids) = if pdf {
            let session = crate::reader::pdf::Session::open(file, target_position, std::sync::Arc::clone(&self.workspace.wake));
            ReaderView::new_pdf(session, settings, title, total)
        } else {
            let Some(session) = Session::open(file, target_position) else {
                report!("книга не открылась: {}", file.display());
                return;
            };
            ReaderView::new(session, settings, title, total, target.is_some(), active)
        };
        let layer = view.id();
        root.add_layer(view, Point::ORIGIN);
        root.focus_on(Some(layer));
        let state = ReaderState { layer, ids, page, key, workspace: workspace_id, total, bindings: HashMap::new(), peek, target, moved: None };
        self.refresh_reader_quotes(root, &state);
        self.popup = Some(Popup::Reader(Box::new(state)));
        self.fit_reader(root);
    }

    pub(in crate::app) fn fit_reader(&self, root: &mut RenderRoot) {
        if let Some(Popup::Reader(state)) = &self.popup {
            let insets = self.interface.screen().insets;
            root.edit_widget(state.layer, |mut w| ReaderView::set_insets(&mut w.downcast(), insets));
        }
    }

    pub(super) fn on_reader_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Outcome {
        let Some(Popup::Reader(mut state)) = self.popup.take_if(|popup| matches!(popup, Popup::Reader(_))) else { return Outcome::default() };
        if let Some(action) = action.downcast_ref::<ReaderAction>() {
            match action {
                ReaderAction::Close => {
                    self.close_reader(root, state);
                    return Outcome::default();
                }
                ReaderAction::Settings => self.toggle_reader_settings(root, &mut state),
                ReaderAction::Moved { cfi, fraction } => state.moved(cfi, *fraction),
                ReaderAction::QuoteRef(cfi) => {
                    self.close_reader(root, state);
                    self.reveal_quote(root, cfi);
                    return Outcome::default();
                }
            }
        } else if action.downcast_ref::<Pressed>().is_some() && state.ids.pdf.contains(&id) {
            let index = state.ids.pdf.iter().position(|&control| control == id).unwrap();
            root.edit_widget(state.layer, |mut w| ReaderView::pdf_control(&mut w.downcast(), index));
        } else if action.downcast_ref::<Pressed>().is_some() && Some(id) == state.ids.quote {
            let quote = root.get_widget(state.layer).and_then(|w| w.downcast::<ReaderView>().and_then(|v| v.inner().quote()));
            if let Some((text, cfi)) = quote {
                self.add_quote(root, &state, &text, &cfi);
            }
        } else if action.downcast_ref::<Pressed>().is_some() && id == state.ids.close {
            self.close_reader(root, state);
            return Outcome::default();
        } else if action.downcast_ref::<Pressed>().is_some() && id == state.ids.settings {
            self.toggle_reader_settings(root, &mut state);
        } else {
            self.on_binding(root, &mut state.bindings, id, action);
            self.typing = None;
            let settings = reader_settings(&self.workspace.core.settings.get_config());
            root.edit_widget(state.layer, |mut w| ReaderView::set_settings(&mut w.downcast(), settings));
        }
        self.popup = Some(Popup::Reader(state));
        Outcome::default()
    }

    pub(super) fn poll_reader(&mut self, root: &mut RenderRoot) {
        let Some(Popup::Reader(state)) = &mut self.popup else { return };
        let mut total = None;
        root.edit_widget(state.layer, |mut w| total = ReaderView::poll_pdf(&mut w.downcast()));
        if let Some(loaded) = total.filter(|&n| n > 0) { state.total = loaded; }
    }

    pub(super) fn drop_reader_frames(&self, root: &mut RenderRoot) {
        if let Some(Popup::Reader(state)) = &self.popup {
            root.edit_widget(state.layer, |mut w| ReaderView::trim_pdf(&mut w.downcast()));
        }
    }

    pub fn reader_zoom(&mut self, root: &mut RenderRoot, command: crate::ui::keymap::Command) -> bool {
        let Some(Popup::Reader(state)) = &self.popup else { return false };
        let mut handled = false;
        root.edit_widget(state.layer, |mut w| handled = ReaderView::pdf_zoom(&mut w.downcast(), command));
        handled
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
}
