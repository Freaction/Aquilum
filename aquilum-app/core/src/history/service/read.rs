use super::save::matches;
use super::{locate, report, HistoryService, Roots};
use crate::files::document::hash_file;
use crate::history::changes::{self, Hunk};
use crate::history::rebuild::{rebuild, Rebuilt};
use crate::history::revert::{self, Reverted};
use crate::history::store;
use crate::text::content::{read_string, Content};
use std::cell::OnceCell;
use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

#[derive(Default)]
pub struct HistoryPage {
    pub total: usize,
    pub versions: Vec<store::VersionFile>,
}

const FULL_VIEW: usize = 1 << 20;

pub struct VersionTexts {
    pub text: String,
    pub previous: Option<String>,
}

pub enum VersionView {
    Full(VersionTexts),
    Changes { hunks: Vec<Hunk>, previous: bool },
}

impl HistoryService {
    pub(super) fn write_name(&self, dir: &Path, id: &str, name: &str, at: SystemTime) -> Option<()> {
        store::write_name(dir, &self.device, id, name, at).map_err(|error| report(dir, "не записала название версии", &error.to_string())).ok()
    }

    pub fn name(&self, roots: Roots<'_>, note: &Path, id: Option<&str>, name: &str) -> Option<String> {
        let mut state = self.lock();
        let located = locate(&mut state, roots, note)?;
        let versions = store::versions(&located.dir);
        let target = match id {
            Some(id) => versions.into_iter().find(|version| version.id == id)?,
            None => versions.into_iter().last()?,
        };
        self.write_name(&located.dir, &target.id, name, SystemTime::now()).map(|()| target.id)
    }

    pub fn page(&self, roots: Roots<'_>, note: &Path, current: Option<&str>, offset: usize, limit: usize) -> HistoryPage {
        let mut state = self.lock();
        let Some(located) = locate(&mut state, roots, note) else {
            return HistoryPage::default();
        };
        let dir = located.dir.as_path();
        let len = fs::metadata(note).ok().map(|metadata| metadata.len() as usize);
        let hash = OnceCell::new();
        let current = || hash.get_or_init(|| current.map(str::to_owned).or_else(|| hash_file(note).ok())).clone();
        let names = store::names(dir);
        let versions = store::versions(dir);
        HistoryPage {
            total: versions.len(),
            versions: versions
                .into_iter()
                .rev()
                .skip(offset)
                .take(limit)
                .map(|mut version| {
                    version.name = names.get(&version.id).cloned();
                    version.is_current = len.is_some_and(|len| matches(dir, &version.id, len, current));
                    version
                })
                .collect(),
        }
    }

    pub fn content(&self, roots: Roots<'_>, note: &Path, id: &str) -> Option<Rebuilt> {
        let mut state = self.lock();
        let located = locate(&mut state, roots, note)?;
        store::versions(&located.dir).iter().any(|version| version.id == id).then(|| rebuild(&located.dir, id).ok()).flatten()
    }

    pub fn revert(&self, roots: Roots<'_>, note: &Path, id: &str, current: Box<dyn Content>) -> io::Result<Reverted> {
        let mut state = self.lock();
        let missing = || io::Error::new(io::ErrorKind::NotFound, format!("нет версии {id} или версии перед ней"));
        let located = locate(&mut state, roots, note).ok_or_else(missing)?;
        let versions = store::versions(&located.dir);
        let index = versions.iter().position(|version| version.id == id).filter(|&index| index > 0).ok_or_else(missing)?;
        revert::revert(&located.dir, &versions, index, current)
    }

    pub fn view(&self, roots: Roots<'_>, note: &Path, id: &str) -> Option<VersionView> {
        let mut state = self.lock();
        let located = locate(&mut state, roots, note)?;
        let dir = located.dir.as_path();
        let versions = store::versions(dir);
        let index = versions.iter().position(|version| version.id == id)?;
        let current = rebuild(dir, id).ok()?;
        let previous = match index.checked_sub(1) {
            Some(previous) => Some((versions[previous].id.as_str(), rebuild(dir, &versions[previous].id).ok()?)),
            None => None,
        };
        let largest = previous.as_ref().map_or(0, |(_, previous)| previous.byte_len()).max(current.byte_len());
        if largest <= FULL_VIEW {
            let text_of = |rebuilt: &Rebuilt| read_string(rebuilt, 0..rebuilt.byte_len()).ok();
            return Some(VersionView::Full(VersionTexts { text: text_of(&current)?, previous: previous.and_then(|(_, previous)| text_of(&previous)) }));
        }
        let Some((previous_id, previous)) = previous else {
            return Some(VersionView::Changes { hunks: changes::hunks(&"", &current, &[(0..0, 0..current.byte_len())]).ok()?, previous: false });
        };
        let regions = revert::changes(dir, previous_id, id).ok()?;
        Some(VersionView::Changes { hunks: changes::hunks(&previous, &current, &regions).ok()?, previous: true })
    }
}
