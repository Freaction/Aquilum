use crate::search::paths::relative_slash_path;
use super::ast::Source;
use crate::search::error::SearchError;
use crate::search::fields::paths_with_tag;
use crate::search::wiki::{candidate_sources, outgoing_links, resolve_many};
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn resolve(
    connection: &Connection,
    root: &Path,
    origin: &Path,
    source: Option<&Source>,
) -> Result<Vec<String>, SearchError> {
    let Some(source) = source else {
        return all_paths(connection);
    };
    if matches!(source, Source::Once) {
        return Ok(Vec::new());
    }
    if is_isolated(source) {
        let selected = select_isolated(connection, root, origin, source)?;
        let mut paths: Vec<String> = selected.into_iter().collect();
        paths.sort();
        return Ok(paths);
    }
    let universe = all_paths(connection)?;
    let selected = select(connection, root, origin, source, &universe)?;
    Ok(universe
        .into_iter()
        .filter(|path| selected.contains(path))
        .collect())
}

fn is_isolated(source: &Source) -> bool {
    match source {
        Source::Once | Source::Tag(_) | Source::LinksTo(_) | Source::LinksFrom(_) => true,
        Source::And(left, right) => is_isolated(left) || is_isolated(right),
        Source::Or(left, right) => is_isolated(left) && is_isolated(right),
        Source::Folder(_) | Source::Not(_) => false,
    }
}

fn select_isolated(
    connection: &Connection,
    root: &Path,
    origin: &Path,
    source: &Source,
) -> Result<HashSet<String>, SearchError> {
    match source {
        Source::Once => Ok(HashSet::new()),
        Source::Tag(tag) => paths_with_tag(connection, tag),
        Source::LinksTo(target) => match note_path(connection, root, origin, target)? {
            Some(path) => Ok(candidate_sources(connection, root, &path)?
                .into_iter()
                .collect()),
            None => Ok(HashSet::new()),
        },
        Source::LinksFrom(target) => match note_path(connection, root, origin, target)? {
            Some(path) => Ok(outgoing_links(connection, root, &path)?
                .into_iter()
                .filter_map(|link| link.path)
                .collect()),
            None => Ok(HashSet::new()),
        },
        Source::And(left, right) => {
            let (first, other) = if is_isolated(left) { (left, right) } else { (right, left) };
            let mut selected = select_isolated(connection, root, origin, first)?;
            if selected.is_empty() {
                return Ok(selected);
            }
            let candidates: Vec<String> = selected.iter().cloned().collect();
            let other = select(connection, root, origin, other, &candidates)?;
            selected.retain(|path| other.contains(path));
            Ok(selected)
        }
        Source::Or(left, right) => {
            let mut left = select_isolated(connection, root, origin, left)?;
            left.extend(select_isolated(connection, root, origin, right)?);
            Ok(left)
        }
        Source::Folder(_) | Source::Not(_) => Ok(HashSet::new()),
    }
}

fn select(
    connection: &Connection,
    root: &Path,
    origin: &Path,
    source: &Source,
    universe: &[String],
) -> Result<HashSet<String>, SearchError> {
    match source {
        Source::Folder(folder) => Ok(in_folder(root, folder, universe)),
        Source::Once | Source::Tag(_) | Source::LinksTo(_) | Source::LinksFrom(_) => select_isolated(connection, root, origin, source),
        Source::And(left, right) => {
            let mut left = select(connection, root, origin, left, universe)?;
            let right = select(connection, root, origin, right, universe)?;
            left.retain(|path| right.contains(path));
            Ok(left)
        }
        Source::Or(left, right) => {
            let mut left = select(connection, root, origin, left, universe)?;
            left.extend(select(connection, root, origin, right, universe)?);
            Ok(left)
        }
        Source::Not(inner) => {
            let inner = select(connection, root, origin, inner, universe)?;
            Ok(universe
                .iter()
                .filter(|path| !inner.contains(*path))
                .cloned()
                .collect())
        }
    }
}

fn in_folder(root: &Path, folder: &str, universe: &[String]) -> HashSet<String> {
    let folder = folder.trim().trim_matches('/');
    if folder.is_empty() {
        return universe.iter().cloned().collect();
    }
    let prefix = format!("{}/", folder.to_lowercase());
    universe
        .iter()
        .filter(|path| {
            let relative = relative_slash_path(root, Path::new(path.as_str())).to_lowercase();
            relative.starts_with(&prefix)
        })
        .cloned()
        .collect()
}

fn note_path(
    connection: &Connection,
    root: &Path,
    origin: &Path,
    target: &str,
) -> Result<Option<PathBuf>, SearchError> {
    Ok(resolve_many(connection, root, origin, &[target.to_owned()])?
        .into_iter()
        .next()
        .flatten())
}

fn all_paths(connection: &Connection) -> Result<Vec<String>, SearchError> {
    let mut statement = connection.prepare("SELECT path FROM documents ORDER BY path")?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::in_folder;
    use std::path::Path;

    #[test]
    fn a_folder_source_takes_the_folder_and_everything_under_it() {
        let root = Path::new("C:/vault");
        let universe = vec![
            "C:/vault/Книги/Пороки.md".to_owned(),
            "C:/vault/Книги/Проза/Ефремов.md".to_owned(),
            "C:/vault/Дневник/Май.md".to_owned(),
        ];
        let found = in_folder(root, "Книги", &universe);
        assert_eq!(found.len(), 2);
        assert!(!found.contains("C:/vault/Дневник/Май.md"));
    }

    #[test]
    fn an_empty_folder_means_the_whole_vault() {
        let root = Path::new("C:/vault");
        let universe = vec!["C:/vault/Книги/Пороки.md".to_owned()];
        assert_eq!(in_folder(root, "", &universe).len(), 1);
        assert_eq!(in_folder(root, "/", &universe).len(), 1);
    }

    #[test]
    fn a_folder_matches_regardless_of_case_and_trailing_slash() {
        let root = Path::new("C:/vault");
        let universe = vec!["C:/vault/Книги/Пороки.md".to_owned()];
        assert_eq!(in_folder(root, "книги", &universe).len(), 1);
        assert_eq!(in_folder(root, "Книги/", &universe).len(), 1);
    }

    #[test]
    fn a_folder_name_is_not_matched_as_a_prefix_of_another_folder() {
        let root = Path::new("C:/vault");
        let universe = vec!["C:/vault/Книгиные/Заметка.md".to_owned()];
        assert!(in_folder(root, "Книги", &universe).is_empty());
    }
}
