use serde::Deserialize;

#[derive(Deserialize)]
pub struct Metadata {
    pub path_display: String,
    pub id: String,
    pub rev: String,
    pub content_hash: String,
    pub size: u64,
    pub client_modified: String,
}

#[derive(Deserialize)]
#[serde(tag = ".tag", rename_all = "lowercase")]
pub enum Entry {
    File(Metadata),
    Folder { name: String },
    Deleted { path_display: String, path_lower: String },
}

#[derive(Deserialize)]
pub struct Page {
    pub entries: Vec<Entry>,
    pub cursor: String,
    pub has_more: bool,
}

#[derive(Deserialize)]
pub struct Cursor {
    pub cursor: String,
}

#[derive(Deserialize)]
pub struct Session {
    pub session_id: String,
}
