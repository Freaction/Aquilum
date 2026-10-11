use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use aquilum_core::Core;
use aquilum_core::search::analysis::models::AnalysisMethod;
use aquilum_core::wikixiv::models::SearchRequest;

use crate::i18n::t;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Backlinks,
    Outgoing,
    Analysis,
    History,
}

impl Mode {
    pub fn key(self) -> &'static str {
        match self {
            Mode::Backlinks => "backlinks",
            Mode::Outgoing => "outgoing",
            Mode::Analysis => "analysis",
            Mode::History => "history",
        }
    }

    pub fn from_key(key: &str) -> Option<Mode> {
        [Mode::Backlinks, Mode::Outgoing, Mode::Analysis, Mode::History].into_iter().find(|m| m.key() == key)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Bm25f,
    AdamicAdar,
    Wikixiv,
}

impl Method {
    pub const ORDER: [Method; 3] = [Method::Bm25f, Method::AdamicAdar, Method::Wikixiv];

    pub fn key(self) -> &'static str {
        match self {
            Method::Bm25f => "bm25f",
            Method::AdamicAdar => "adamicAdar",
            Method::Wikixiv => "wixiv",
        }
    }

    pub fn from_key(key: &str) -> Option<Method> {
        Method::ORDER.into_iter().find(|m| m.key() == key)
    }

    pub fn label(self) -> String {
        t(&format!("analysis.methods.{}", self.key()))
    }

    pub fn enabled(config: &aquilum_core::settings::models::AppConfig) -> Vec<Method> {
        let a = &config.analysis;
        Method::ORDER
            .into_iter()
            .filter(|m| match m {
                Method::Bm25f => a.enable_bm25f,
                Method::AdamicAdar => a.enable_adamic_adar,
                Method::Wikixiv => a.enable_wikixiv,
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Note(PathBuf, Option<usize>),
    Wiki(String),
    Url(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub title: String,
    pub counter: Option<String>,
    pub target: Target,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    pub generation: u64,
    pub items: Vec<Item>,
    pub notice: Option<String>,
}

pub struct Request {
    pub generation: u64,
    pub workspace: PathBuf,
    pub document: PathBuf,
    pub mode: Mode,
    pub method: Method,
    pub text: String,
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn spawn(core: Arc<Core>, request: Request, results: Sender<Listing>, wake: Arc<dyn Fn() + Send + Sync>) {
    std::thread::spawn(move || {
        let listing = run(&core, &request);
        if results.send(listing).is_ok() {
            wake();
        }
    });
}

pub fn run(core: &Core, request: &Request) -> Listing {
    let (workspace, document) = (path_str(&request.workspace), path_str(&request.document));
    let listing = |items: Vec<Item>, empty: &str| Listing {
        generation: request.generation,
        notice: items.is_empty().then(|| t(empty)),
        items,
    };
    let notice = |key: &str| Listing { generation: request.generation, items: Vec::new(), notice: Some(t(key)) };
    match (request.mode, request.method) {
        (Mode::History, _) => listing(Vec::new(), "history.empty"),
        (Mode::Backlinks, _) => {
            let items = core.search.backlinks(&workspace, &document).unwrap_or_default();
            let items = counted(items.into_iter().map(|link| (link.title, Target::Note(PathBuf::from(link.path), Some(link.offset)))));
            listing(items, "backlinks.empty")
        }
        (Mode::Outgoing, _) => {
            let items = core.search.outgoing_links(&workspace, &document).unwrap_or_default();
            let items = counted(items.into_iter().map(|link| {
                let target = link.path.map_or(Target::Wiki(link.target), |p| Target::Note(PathBuf::from(p), None));
                (link.title, target)
            }));
            listing(items, "backlinks.outgoingEmpty")
        }
        (Mode::Analysis, Method::Wikixiv) => {
            let enabled = core.settings.get_config().analysis.enable_wikixiv;
            let search = SearchRequest { text: request.text.clone(), document_path: Some(document), generation: request.generation };
            match core.wikixiv.search(search, enabled) {
                Ok(result) if result.offline => notice("wikixiv.offline"),
                Ok(result) if result.insufficient_text => notice("wikixiv.insufficient"),
                Ok(result) => {
                    let items = result.hits.into_iter().map(|hit| Item { title: hit.title, counter: None, target: Target::Url(hit.url), reasons: hit.matched_terms }).collect();
                    listing(items, "wikixiv.empty")
                }
                Err(_) => notice("wikixiv.error"),
            }
        }
        (Mode::Analysis, method) => {
            let method = if method == Method::AdamicAdar { AnalysisMethod::AdamicAdar } else { AnalysisMethod::Bm25f };
            let config = core.settings.get_config();
            match core.search.analyze_document(&workspace, &document, method, None, &config) {
                Ok(results) => {
                    let items = results
                        .into_iter()
                        .map(|r| Item {
                            counter: Some(match r.confidence {
                                Some(confidence) => format!("{:.1}%", confidence * 100.0),
                                None => format!("{:.2}", r.raw_score),
                            }),
                            title: r.title,
                            target: Target::Note(PathBuf::from(r.path), None),
                            reasons: r.reasons,
                        })
                        .collect();
                    listing(items, "analysis.empty")
                }
                Err(_) => notice("analysis.error"),
            }
        }
    }
}

fn counted(links: impl Iterator<Item = (String, Target)>) -> Vec<Item> {
    let mut items: Vec<(Item, usize)> = Vec::new();
    for (title, target) in links {
        let key = |target: &Target| match target {
            Target::Wiki(name) => Target::Wiki(name.to_lowercase()),
            Target::Note(path, _) => Target::Note(path.clone(), None),
            other => other.clone(),
        };
        let wanted = key(&target);
        let same = |item: &Item| key(&item.target) == wanted;
        match items.iter_mut().find(|(item, _)| same(item)) {
            Some((_, count)) => *count += 1,
            None => items.push((Item { title, counter: None, target, reasons: Vec::new() }, 1)),
        }
    }
    items
        .into_iter()
        .map(|(mut item, count)| {
            item.counter = Some(count.to_string());
            item
        })
        .collect()
}

pub fn find_note(workspace: &Path, target: &str) -> Option<PathBuf> {
    let name = without_md(target.trim().rsplit(['/', '\\']).next()?).to_lowercase();
    let mut folders = vec![workspace.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let hidden = entry.file_name().to_string_lossy().starts_with('.');
            match entry.file_type() {
                Ok(kind) if kind.is_dir() && !hidden => folders.push(path),
                Ok(kind) if kind.is_file() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) && path.file_stem().is_some_and(|s| s.to_string_lossy().to_lowercase() == name) => return Some(path),
                _ => {}
            }
        }
    }
    None
}

pub fn linked_file_path(workspace: &Path, target: &str) -> Option<PathBuf> {
    let normalized = target.trim().replace('\\', "/");
    let segments: Vec<&str> = without_md(&normalized).split('/').collect();
    if segments.iter().any(|s| s.is_empty() || *s != s.trim() || crate::file_ops::sanitize_file_name(s) != *s) {
        return None;
    }
    let (last, folders) = segments.split_last()?;
    let mut path = workspace.to_path_buf();
    path.extend(folders);
    path.push(format!("{last}.md"));
    Some(path)
}

fn without_md(name: &str) -> &str {
    match name.len().checked_sub(3) {
        Some(cut) if name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(".md") => &name[..cut],
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backlinks_and_outgoing_from_index() {
        let vault = crate::test_dir::tempdir();
        let data = crate::test_dir::tempdir();
        std::fs::write(vault.path().join("a.md"), "Ссылка на [[b]], снова [[b]] и [[Нет такой]]").unwrap();
        std::fs::write(vault.path().join("b.md"), "Текст").unwrap();
        let core = Core::open(data.path(), Arc::new(|_| {}));
        let workspace = vault.path().to_string_lossy().into_owned();
        core.search.prepare(&workspace).unwrap();
        for _ in 0..200 {
            let status = core.search.status(Some(&workspace));
            if status.state == aquilum_core::search::models::SearchIndexState::Ready && !status.updating {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let request = |document: &str, mode| Request {
            generation: 1,
            workspace: vault.path().to_path_buf(),
            document: vault.path().join(document),
            mode,
            method: Method::Bm25f,
            text: String::new(),
        };
        let back = run(&core, &request("b.md", Mode::Backlinks));
        assert_eq!(back.items.len(), 1);
        assert_eq!(back.items[0].counter.as_deref(), Some("2"));
        assert!(matches!(&back.items[0].target, Target::Note(p, _) if p.ends_with("a.md")));
        let out = run(&core, &request("a.md", Mode::Outgoing));
        assert_eq!(out.items.len(), 2);
        assert_eq!(out.items[0].counter.as_deref(), Some("2"));
        assert!(out.items.iter().any(|i| matches!(&i.target, Target::Wiki(t) if t == "Нет такой")));
        let none = run(&core, &request("a.md", Mode::Backlinks));
        assert!(none.items.is_empty() && none.notice.is_some());
        core.shutdown();
    }

    #[test]
    fn linked_file_path_rules() {
        let root = Path::new("/kb");
        assert_eq!(linked_file_path(root, "Папка/Заметка"), Some(root.join("Папка").join("Заметка.md")));
        assert_eq!(linked_file_path(root, "a.md"), Some(root.join("a.md")));
        assert_eq!(linked_file_path(root, "плохо: имя"), None);
        assert_eq!(linked_file_path(root, "a//b"), None);
        assert_eq!(linked_file_path(root, "v1.2 план"), Some(root.join("v1.2 план.md")));
        assert_eq!(linked_file_path(root, "Отчёт.MD"), Some(root.join("Отчёт.md")));
    }
}
