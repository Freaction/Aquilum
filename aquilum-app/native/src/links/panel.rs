use std::path::{Path, PathBuf};

use aquilum_core::Core;
use aquilum_core::search::analysis::models::AnalysisMethod;
use aquilum_core::wikixiv::models::SearchRequest;

use super::{Item, Listing, Method, Mode, Request, Target};
use crate::i18n::t;

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn indexed(files: &[(&str, &str)]) -> (tempfile::TempDir, tempfile::TempDir, Arc<Core>) {
        let vault = crate::test_dir::tempdir();
        let data = crate::test_dir::tempdir();
        for (name, text) in files {
            std::fs::write(vault.path().join(name), text).unwrap();
        }
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
        (vault, data, core)
    }

    fn request(vault: &Path, document: &str, mode: Mode) -> Request {
        Request { generation: 1, workspace: vault.to_path_buf(), document: vault.join(document), mode, method: Method::Bm25f, text: String::new() }
    }

    #[test]
    fn backlinks_and_outgoing_from_index() {
        let (vault, _data, core) = indexed(&[("a.md", "Ссылка на [[b]], снова [[b]] и [[Нет такой]]"), ("b.md", "Текст")]);
        let back = run(&core, &request(vault.path(), "b.md", Mode::Backlinks));
        assert_eq!(back.items.len(), 1);
        assert_eq!(back.items[0].counter.as_deref(), Some("2"));
        assert!(matches!(&back.items[0].target, Target::Note(p, _) if p.ends_with("a.md")));
        let out = run(&core, &request(vault.path(), "a.md", Mode::Outgoing));
        assert_eq!(out.items.len(), 2);
        assert_eq!(out.items[0].counter.as_deref(), Some("2"));
        assert!(out.items.iter().any(|i| matches!(&i.target, Target::Wiki(t) if t == "Нет такой")));
        let none = run(&core, &request(vault.path(), "a.md", Mode::Backlinks));
        assert!(none.items.is_empty() && none.notice.is_some());
        core.shutdown();
    }

    #[test]
    fn anchors_stay_in_the_target_and_attachments_are_not_listed() {
        let (vault, _data, core) = indexed(&[("a.md", "[[b#Раздел]] ![[pic.png|200]] [[#Тут]] [[Нет#Раздел]]"), ("b.md", "# Раздел")]);
        let out = run(&core, &request(vault.path(), "a.md", Mode::Outgoing));
        assert_eq!(out.items.len(), 2);
        assert!(matches!(&out.items[0].target, Target::Note(p, None) if p.ends_with("b.md")));
        assert!(matches!(&out.items[1].target, Target::Wiki(t) if t == "Нет#Раздел"));
        let back = run(&core, &request(vault.path(), "b.md", Mode::Backlinks));
        assert_eq!(back.items.len(), 1);
        core.shutdown();
    }
}
