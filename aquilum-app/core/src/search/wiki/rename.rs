use super::super::error::SearchError;
use super::parser::extract;
use super::resolver::LinkResolver;
use super::target::{relative_key, relative_target, select_candidate, target_key, title_key};
use crate::files::document::hash_file;
use crate::search::pages::Pages;
use crate::search::paths::same_path;
use rusqlite::Connection;
use std::ops::Range;
use std::path::{Path, PathBuf};

pub struct LinkRewrite {
    pub path: PathBuf,
    pub original_hash: String,
    pub changes: Vec<(Range<usize>, String)>,
}

pub fn plan_for_rename(
    connection: &Connection,
    root: &Path,
    old_path: &Path,
    new_path: &Path,
    sources: Vec<PathBuf>,
    fallback_candidates: Option<&[String]>,
) -> Result<Vec<LinkRewrite>, SearchError> {
    let mut rewrites = Vec::new();
    let mut resolver = LinkResolver::new(connection)?;
    for stored_path in sources {
        let source_path = if same_path(&stored_path, old_path) {
            new_path
        } else {
            &stored_path
        };
        let Ok(mut pages) = Pages::open(source_path) else {
            continue;
        };
        let mut changes = Vec::new();
        while let Ok(Some(page)) = pages.next() {
            for link in extract(&page.text) {
                if targets_path(&mut resolver, root, &stored_path, &link.target, old_path, fallback_candidates)? {
                    let path_style = page.text[link.note_range.clone()].replace('\\', "/").contains('/');
                    let range = page.start + link.note_range.start..page.start + link.note_range.end;
                    changes.push((range, relative_target(root, new_path, path_style)));
                }
            }
        }
        if changes.is_empty() || !pages.finished() {
            continue;
        }
        let Ok(original_hash) = hash_file(source_path) else {
            continue;
        };
        rewrites.push(LinkRewrite { path: stored_path, original_hash, changes });
    }
    Ok(rewrites)
}

fn targets_path(
    resolver: &mut LinkResolver<'_>,
    root: &Path,
    source: &Path,
    target: &str,
    old_path: &Path,
    fallback_candidates: Option<&[String]>,
) -> Result<bool, SearchError> {
    let (key, kind) = target_key(target);
    let named = match kind == "path" {
        true => key == relative_key(root, old_path),
        false => key == title_key(old_path),
    };
    if let Some(candidates) = fallback_candidates {
        return Ok(named && (kind == "path" || select_candidate(root, source, candidates).is_some_and(|path| same_path(&path, old_path))));
    }
    if let Some(path) = resolver.resolve(root, source, target)? {
        return Ok(same_path(&path, old_path));
    }
    Ok(named)
}
