use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileItemType {
    File,
    Folder,
}

#[derive(Debug, Serialize)]
pub struct FileItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: FileItemType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteResult {
    pub hash: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRenameResult {
    pub updated_paths: Vec<std::path::PathBuf>,
}
