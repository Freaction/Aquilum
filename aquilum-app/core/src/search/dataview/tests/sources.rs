use std::path::Path;

use crate::search::{dataview, index_document, metadata};

#[test]
fn combined_tag_sources_keep_both_sides_of_an_intersection() {
    let mut connection = metadata::memory();
    let root = Path::new("C:/vault");
    let transaction = connection.transaction().unwrap();
    for (name, tags) in [("both.md", "one, two"), ("one.md", "one"), ("two.md", "two")] {
        index_document::index_text(&transaction, root, &root.join(name), &format!("---\ntags: [{tags}]\n---\n")).unwrap();
    }
    transaction.commit().unwrap();
    for source in ["#one AND #two", "#two AND #one", "#one AND (#two OR #missing)", "#one AND (#two OR !#one)"] {
        let query = dataview::parser::parse(&format!("LIST FROM {source}")).unwrap();
        let paths = dataview::source::resolve(&connection, root, &root.join("home.md"), query.source.as_ref()).unwrap();
        assert_eq!(paths, [root.join("both.md").to_string_lossy().into_owned()], "source {source}");
    }
}
