use super::super::error::SearchError;
use super::parser::extract;
use super::resolver::LinkResolver;
use super::target::{relative_key, relative_target, select_candidate, target_key, title_key};
use crate::files::document::{hash_bytes, normalize_line_endings};
use crate::search::paths::same_path;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

pub struct LinkRewrite {
    pub path: PathBuf,
    pub original: String,
    pub original_hash: String,
    pub content: String,
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
        let Ok(original) = fs::read_to_string(source_path) else {
            continue;
        };
        let text = normalize_line_endings(original.clone());
        let mut changes = Vec::new();
        for link in extract(&text) {
            if targets_path(
                &mut resolver,
                root,
                &stored_path,
                &link.target,
                old_path,
                fallback_candidates,
            )? {
                let path_style = text[link.note_range.clone()]
                    .replace('\\', "/")
                    .contains('/');
                changes.push((
                    link.note_range,
                    relative_target(root, new_path, path_style),
                ));
            }
        }
        if changes.is_empty() {
            continue;
        }
        let mut content = text;
        for (range, replacement) in changes.into_iter().rev() {
            content.replace_range(range, &replacement);
        }
        let original_hash = hash_bytes(original.as_bytes());
        rewrites.push(LinkRewrite {
            path: stored_path,
            original,
            original_hash,
            content,
        });
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
    if let Some(candidates) = fallback_candidates {
        return Ok(if kind == "path" {
            key == relative_key(root, old_path)
        } else {
            key == title_key(old_path)
                && select_candidate(root, source, candidates)
                    .is_some_and(|path| same_path(&path, old_path))
        });
    }
    if let Some(path) = resolver.resolve(root, source, target)? {
        return Ok(same_path(&path, old_path));
    }
    Ok(if kind == "path" {
        key == relative_key(root, old_path)
    } else {
        key == title_key(old_path)
    })
}
