use std::path::Path;

use aquilum_core::files::models::{FileItem, FileItemType};
use aquilum_core::text::matcher::{EXACT, Fold, Matcher, Mode, Options};

use super::vault::Lists;
use super::{Sort, TreeModel, comparable};

const NAME: Options = Options { fold: Fold::Ascii, mode: Mode::Substring, ..EXACT };

pub struct Filter {
    query: String,
    each: Vec<Matcher>,
    marks: Matcher,
}

impl Filter {
    fn new(query: &str) -> Option<Self> {
        let terms: Vec<&str> = query.split_whitespace().collect();
        (!terms.is_empty()).then(|| Filter { query: query.to_owned(), each: terms.iter().map(|term| Matcher::new(term, NAME)).collect(), marks: Matcher::any(&terms, NAME) })
    }

    fn matches(&self, name: &str) -> bool {
        self.each.iter().all(|matcher| matcher.first(name).is_some())
    }

    pub(super) fn marks(&self, name: &str) -> Vec<(usize, usize)> {
        self.marks.find_all(name).into_iter().map(|found| (found.start, found.end)).collect()
    }
}

fn keep(source: &Lists, filter: &Filter, dir: &Path, kept: &mut Lists) -> bool {
    let Some(items) = source.get(&comparable(dir)) else { return false };
    let survivors: Vec<FileItem> = items
        .iter()
        .filter(|item| {
            let inside = item.item_type == FileItemType::Folder && keep(source, filter, Path::new(&item.id), kept);
            inside || filter.matches(&item.name)
        })
        .cloned()
        .collect();
    let any = !survivors.is_empty();
    if any {
        kept.insert(comparable(dir), survivors);
    }
    any
}

impl TreeModel {
    pub fn set_filter(&mut self, query: &str) {
        self.filter = Filter::new(query);
        if self.filter.is_none() {
            self.index = None;
        }
        self.rebuild();
    }

    pub fn filtering(&self) -> bool {
        self.filter.is_some()
    }

    pub fn filter_query(&self) -> &str {
        self.filter.as_ref().map_or("", |filter| filter.query.as_str())
    }

    pub fn indexed(&self) -> bool {
        self.index.is_some()
    }

    pub fn set_index(&mut self, index: Lists) {
        if self.filter.is_some() {
            self.index = Some(index);
            self.rebuild();
        }
    }

    pub(super) fn survivors(&mut self, root: &Path) -> Lists {
        let mut kept = Lists::new();
        if let Some(filter) = &self.filter {
            keep(self.index.as_ref().unwrap_or(&self.directories), filter, root, &mut kept);
        }
        if self.index.is_some() && self.sort != Sort::NameAsc {
            if self.sort.by_time() {
                let unknown: Vec<FileItem> = kept.values().flatten().filter(|item| !self.times.contains_key(&comparable(Path::new(&item.id)))).cloned().collect();
                self.stamp(&unknown);
            }
            for items in kept.values_mut() {
                self.order(items);
            }
        }
        kept
    }
}
