use aquilum_core::files::content::FileContent;
use aquilum_core::history::Source;
use aquilum_core::text::content::read_bytes;
use masonry::app::RenderRoot;

use super::{Opening, ReaderState};
use crate::app::App;
use crate::frontmatter::BOOK_FILE;
use crate::reader::quotes;
use crate::ui::reader::ReaderView;

impl App {
    pub(super) fn refresh_reader_quotes(&mut self, root: &mut RenderRoot, state: &ReaderState) {
        let found = match state.target.as_deref() {
            Some(path) if self.shows(path) => quotes::find(&crate::rope::Text(&self.text)),
            Some(path) => FileContent::open(path).map(|file| quotes::find(&file)).unwrap_or_default(),
            None => Vec::new(),
        };
        let marks: Vec<(String, String)> = found.into_iter().filter(|q| q.book.as_deref().is_none_or(|b| b == state.key)).map(|q| (q.cfi, q.label)).collect();
        root.edit_widget(state.layer, |mut w| ReaderView::set_quotes(&mut w.downcast(), marks));
    }

    pub(super) fn add_quote(&mut self, root: &mut RenderRoot, state: &ReaderState, text: &str, cfi: &str) {
        let Some(target) = state.target.clone() else { return };
        let written = self.change_note(root, &target, Source::Me, |current| {
            let line = quotes::format(text, cfi, Some(&state.key), quotes::find(current).len() + 1);
            let len = current.byte_len();
            let last = read_bytes(current, len.saturating_sub(1)..len).ok()?.first().copied();
            Some((len..len, quotes::appended(last, &line)))
        });
        if !written {
            return;
        }
        root.edit_widget(state.layer, |mut w| ReaderView::clear_selection(&mut w.downcast()));
        self.refresh_reader_quotes(root, state);
    }

    pub(super) fn reveal_quote(&mut self, root: &mut RenderRoot, cfi: &str) {
        let Some(quote) = quotes::find(&crate::rope::Text(&self.text)).into_iter().find(|q| q.cfi == cfi) else { return };
        let Some(ids) = &self.note_ids else { return };
        let hidden = self.hidden;
        let (start, end) = (quote.text.start.saturating_sub(hidden), quote.text.end.saturating_sub(hidden));
        let body = ids.body.id();
        root.focus_on(Some(body));
        ids.body.edit(root, |mut editor| crate::ui::editor::TextEditor::select_byte_range(&mut editor, start, end));
    }

    pub(in crate::app) fn open_reader_link(&mut self, root: &mut RenderRoot, url: &str) -> bool {
        let Some((cfi, book)) = crate::links::reader::parse_href(url) else { return false };
        let own = self.fields().and_then(|f| f.text(BOOK_FILE).map(str::to_owned));
        let Some(book_file) = book.clone().or_else(|| own.clone()) else { return true };
        let Some(file) = crate::vault_files::resolve(self.tree.root(), &book_file) else { return true };
        let same = book.is_none() || book == own;
        let title = if same { self.note.as_ref().map(|n| n.path.file_stem().unwrap_or_default().to_string_lossy().into_owned()).unwrap_or_default() } else { crate::i18n::t("reader.book") };
        self.open_reader(root, Opening { file, page: None, title, initial: Some(cfi), peek: true });
        true
    }
}
