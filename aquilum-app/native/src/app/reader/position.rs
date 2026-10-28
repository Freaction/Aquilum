use std::time::Instant;

use aquilum_core::history::Source;
use aquilum_core::ui_state::models::SaveReaderStateInput;
use masonry::app::RenderRoot;

use super::ReaderState;
use crate::app::note_page::view::VIEW_SAVE_DELAY;
use crate::app::{App, Popup};
use crate::frontmatter::{PAGES, READ_PERCENT, READER_POSITION};

pub(super) struct Moved {
    cfi: String,
    fraction: f64,
    due: Instant,
}

impl ReaderState {
    pub(super) fn moved(&mut self, cfi: &str, fraction: f64) {
        if !self.peek {
            self.moved = Some(Moved { cfi: cfi.to_owned(), fraction, due: Instant::now() + VIEW_SAVE_DELAY });
        }
    }
}

impl App {
    pub(in crate::app) fn reader_due(&self) -> Option<Instant> {
        match &self.popup {
            Some(Popup::Reader(state)) => state.moved.as_ref().map(|moved| moved.due),
            _ => None,
        }
    }

    pub(in crate::app) fn settle_reader(&mut self, root: &mut RenderRoot, now: Instant) {
        if self.reader_due().is_some_and(|due| due <= now) {
            self.save_reader(root);
        }
    }

    pub fn save_reader(&mut self, root: &mut RenderRoot) {
        if let Some(Popup::Reader(mut state)) = self.popup.take() {
            self.save_position(root, &mut state);
            self.popup = Some(Popup::Reader(state));
        }
    }

    pub(super) fn close_reader(&mut self, root: &mut RenderRoot, mut state: Box<ReaderState>) {
        self.save_position(root, &mut state);
        root.remove_layer(state.layer);
    }

    fn save_position(&mut self, root: &mut RenderRoot, state: &mut ReaderState) {
        let Some(Moved { cfi, fraction, .. }) = state.moved.take() else { return };
        let current = ((fraction.clamp(0.0, 1.0) * f64::from(state.total)).round() as u32).min(state.total);
        if let Some(workspace_id) = state.workspace {
            let input = SaveReaderStateInput { workspace_id, book_file: state.key.clone(), current: i64::from(current), cfi: Some(cfi.clone()), now_ms: crate::dates::now_ms() };
            if let Err(error) = self.workspace.core.ui_state.save_reader_state(&input) {
                report!("позиция чтения не сохранена: {error:?}");
            }
        }
        let Some(page) = state.page.clone() else { return };
        let percent = ((f64::from(current) / f64::from(state.total)) * 100.0).round().clamp(0.0, 100.0);
        self.write_fields(root, &page, Source::Reading, &[(PAGES, format!("{current}/{}", state.total)), (READER_POSITION, cfi), (READ_PERCENT, format!("{percent}"))]);
    }
}
