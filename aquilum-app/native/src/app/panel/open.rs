use std::path::{Path, PathBuf};

use aquilum_core::files::gate;
use aquilum_core::history::Source;
use aquilum_core::search::wiki::target::split;
use masonry::app::RenderRoot;

use crate::app::App;
use crate::links::{self, Target};
use crate::system;

impl App {
    pub(in crate::app) fn open_target(&mut self, root: &mut RenderRoot, target: Target, new_tab: bool) -> bool {
        match target {
            Target::Note(path, offset) => self.open_in_with_offset(root, path, new_tab, offset),
            Target::Url(url) if crate::links::reader::is_reader(&url) => {
                self.open_reader_link(root, &url);
                false
            }
            Target::Url(url) => {
                system::open_url(&url);
                false
            }
            Target::Wiki(raw) => self.open_wiki(root, &raw, new_tab),
        }
    }

    fn open_wiki(&mut self, root: &mut RenderRoot, raw: &str, new_tab: bool) -> bool {
        let Some(path) = self.wiki_note(root, raw) else { return false };
        let offset = split(raw).anchor.and_then(|anchor| self.anchor_offset(&path, anchor));
        self.open_in_with_offset(root, path, new_tab, offset)
    }

    fn wiki_note(&mut self, root: &mut RenderRoot, raw: &str) -> Option<PathBuf> {
        let current = self.note.as_ref().map(|n| n.path.clone());
        if split(raw).note.is_empty() {
            return current;
        }
        let workspace = self.tree.root()?.to_path_buf();
        let source = current.unwrap_or_else(|| workspace.clone());
        let found = self.workspace.core.search.resolve_wiki_links(&workspace.to_string_lossy(), &source.to_string_lossy(), &[raw.to_owned()]);
        let resolved = found.ok().and_then(|found| found.paths.into_iter().next().flatten()).map(PathBuf::from);
        if let Some(path) = resolved.or_else(|| links::find_note(&workspace, raw)) {
            return Some(path);
        }
        let base = if split(raw).note.contains(['/', '\\']) { workspace } else { crate::vault_files::notes_dir(&self.workspace.core, &workspace) };
        let path = links::linked_file_path(&base, raw)?;
        self.create_linked_note(root, &path).then_some(path)
    }

    fn create_linked_note(&mut self, root: &mut RenderRoot, path: &Path) -> bool {
        if path.exists() {
            return true;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = gate::create(&self.workspace.core, path, &"", Source::Me, None) {
            report!("{} не создана: {error:?}", path.display());
            return false;
        }
        self.tree.paths_changed(root, std::slice::from_ref(&path.to_path_buf()));
        true
    }
}
