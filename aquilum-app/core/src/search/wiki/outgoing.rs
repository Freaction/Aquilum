use super::super::error::SearchError;
use super::super::models::OutgoingLink;
use super::parser::extract;
use super::resolver::resolve_many;
use super::target::split;
use super::LINK_LIST_LIMIT;
use crate::search::pages::Pages;
use crate::search::paths::strip_markdown_extension;
use rusqlite::Connection;
use std::path::Path;

pub fn outgoing_links(
    connection: &Connection,
    root: &Path,
    document: &Path,
) -> Result<Vec<OutgoingLink>, SearchError> {
    let targets = first_targets(document, LINK_LIST_LIMIT)?;
    let paths = resolve_many(connection, root, document, &targets)?;
    let items = targets
        .into_iter()
        .zip(paths)
        .map(|(target, path)| OutgoingLink {
            title: path
                .as_deref()
                .and_then(Path::file_stem)
                .and_then(|value| value.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| target_title(&target)),
            path: path.map(|value| value.to_string_lossy().into_owned()),
            target,
        })
        .collect();

    Ok(items)
}

fn first_targets(document: &Path, limit: usize) -> Result<Vec<String>, SearchError> {
    let mut pages = Pages::open(document)?;
    let mut targets = Vec::new();
    while targets.len() < limit {
        let Some(page) = pages.next()? else { break };
        let room = limit - targets.len();
        targets.extend(extract(&page.text).into_iter().take(room).map(|link| link.target));
    }
    Ok(targets)
}

fn target_title(target: &str) -> String {
    let note = split(target).note;
    let filename = note.rsplit(['/', '\\']).next().unwrap_or_default();
    strip_markdown_extension(filename).to_owned()
}

#[cfg(test)]
mod tests {
    use super::outgoing_links;
    use crate::search::wiki::resolve_many;
    use std::fs;

    #[test]
    fn returns_resolved_and_missing_document_titles() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let source = root.join("Source.md");
        let target = root.join("Folder").join("Target.md");
        fs::create_dir(target.parent().unwrap()).unwrap();
        fs::write(
            &source,
            "[[Folder/Target|alias]] and [[Missing Note]] and [[Research.v2]]",
        )
        .unwrap();
        fs::write(&target, "target").unwrap();

        let mut connection = crate::search::metadata::memory();
        let transaction = connection.transaction().unwrap();
        crate::search::index_document::index_text(
            &transaction,
            root,
            &source,
            "[[Folder/Target|alias]] and [[Missing Note]] and [[Research.v2]]",
        )
        .unwrap();
        crate::search::index_document::index_text(&transaction, root, &target, "target").unwrap();
        transaction.commit().unwrap();

        let links = outgoing_links(&connection, root, &source).unwrap();
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].title, "Target");
        assert_eq!(links[0].path.as_deref(), target.to_str());
        assert_eq!(links[1].title, "Missing Note");
        assert!(links[1].path.is_none());
        assert_eq!(links[2].title, "Research.v2");
    }

    #[test]
    fn anchors_resolve_to_the_note_and_files_are_not_links() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let source = root.join("Source.md");
        let target = root.join("Target.md");
        let body = "[[#Раздел]] ![[pic.png|10]] ![[document.pdf#page=3]] ![[Target]] [[Target#h|x]] [[Missing#h]]";
        fs::write(&source, body).unwrap();
        fs::write(&target, "target").unwrap();
        let mut connection = crate::search::metadata::memory();
        let transaction = connection.transaction().unwrap();
        crate::search::index_document::index_text(&transaction, root, &source, body).unwrap();
        crate::search::index_document::index_text(&transaction, root, &target, "target").unwrap();
        transaction.commit().unwrap();

        let links = outgoing_links(&connection, root, &source).unwrap();
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].path.as_deref(), target.to_str());
        assert_eq!(links[1].target, "Target#h");
        assert_eq!(links[1].title, "Target");
        assert!(links[2].path.is_none());
        assert_eq!(links[2].title, "Missing");

        let own = resolve_many(&connection, root, &source, &["#Раздел".to_owned(), "Target#h".to_owned()]).unwrap();
        assert_eq!(own[0].as_deref(), Some(source.as_path()));
        assert_eq!(own[1].as_deref(), Some(target.as_path()));
        let edges = connection
            .query_row("SELECT COUNT(*) FROM wiki_links", [], |row| row.get::<_, i64>(0))
            .unwrap();
        assert_eq!(edges, 3);
    }
}
