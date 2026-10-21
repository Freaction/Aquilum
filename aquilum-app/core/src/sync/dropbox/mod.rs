mod api;
mod types;

use self::api::{content_hash, json, Api, Body, CONTENT};
use self::types::{Cursor, Entry, Metadata, Page, Session};
use super::{Account, Client, Delta, Remote, RemoteError, RemoteFile, RemoteResult};
use crate::search::note_date::{parse_at, to_civil_at};
use crate::text::content::Content;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{self, Write};
use ureq::Agent;

pub const CLIENT: Client = Client {
    authorize: "https://www.dropbox.com/oauth2/authorize",
    token: "https://api.dropboxapi.com/oauth2/token",
    id: "ytvmy13es5psczr",
    secret: "",
    params: &[("token_access_type", "offline")],
};

const SINGLE_UPLOAD: usize = 150 * 1024 * 1024;
const UPLOAD_PART: usize = 64 * 1024 * 1024;

pub struct Dropbox {
    api: Api,
    root: String,
}

impl Dropbox {
    pub fn new(account: Account, agent: Agent, folder: &str) -> Self {
        Self { api: Api::new(account, agent), root: format!("/{folder}") }
    }

    fn file(&self, metadata: Metadata) -> Option<RemoteFile> {
        Some(RemoteFile {
            path: self.relative(&metadata.path_display)?,
            id: metadata.id,
            rev: metadata.rev,
            checksum: metadata.content_hash,
            size: metadata.size,
            modified: parse_at(metadata.client_modified.trim_end_matches('Z'), 0).map_or(0, |nanos| (nanos / 1_000_000) as u64),
        })
    }

    fn relative(&self, display: &str) -> Option<String> {
        let skip = self.root.chars().count() + 1;
        let prefix: String = display.chars().take(skip).collect();
        if prefix.to_lowercase() != format!("{}/", self.root).to_lowercase() {
            return None;
        }
        let relative: String = display.chars().skip(skip).collect();
        (!relative.is_empty()).then_some(relative)
    }

    fn pages(&self, mut page: Page, mut visit: impl FnMut(Entry) -> RemoteResult<()>) -> RemoteResult<String> {
        loop {
            for entry in page.entries {
                visit(entry)?;
            }
            if !page.has_more {
                return Ok(page.cursor);
            }
            page = self.api.rpc("files/list_folder/continue", &json!({ "cursor": page.cursor }))?;
        }
    }

    fn upload_parts(&self, content: &dyn Content, commit: Value) -> RemoteResult<Metadata> {
        let url = |route: &str| format!("{CONTENT}files/upload_session/{route}");
        let length = content.byte_len();
        let part = |start: usize| Body::Bytes(content, start..(start + UPLOAD_PART).min(length));
        let session: Session = json(self.api.call(&url("start"), Some(&json!({ "close": false })), part(0))?)?;
        let mut offset = UPLOAD_PART;
        while offset + UPLOAD_PART < length {
            let cursor = json!({ "cursor": { "session_id": session.session_id, "offset": offset }, "close": false });
            self.api.call(&url("append_v2"), Some(&cursor), part(offset))?;
            offset += UPLOAD_PART;
        }
        let finish = json!({ "cursor": { "session_id": session.session_id, "offset": offset }, "commit": commit });
        json(self.api.call(&url("finish"), Some(&finish), part(offset))?)
    }
}

impl Remote for Dropbox {
    fn cursor(&self) -> RemoteResult<String> {
        let latest = |api: &Api| api.rpc::<Cursor>("files/list_folder/get_latest_cursor", &json!({ "path": self.root, "recursive": true }));
        let cursor = match latest(&self.api) {
            Err(RemoteError::Missing) => {
                self.api.rpc::<Value>("files/create_folder_v2", &json!({ "path": self.root }))?;
                latest(&self.api)?
            }
            cursor => cursor?,
        };
        Ok(cursor.cursor)
    }

    fn list(&self) -> RemoteResult<Vec<RemoteFile>> {
        let mut files = Vec::new();
        let first = self.api.rpc("files/list_folder", &json!({ "path": self.root, "recursive": true }))?;
        self.pages(first, |entry| {
            if let Entry::File(metadata) = entry {
                files.extend(self.file(metadata));
            }
            Ok(())
        })?;
        Ok(files)
    }

    fn delta(&self, cursor: &str) -> RemoteResult<Option<Delta>> {
        let first = match self.api.rpc("files/list_folder/continue", &json!({ "cursor": cursor })) {
            Err(RemoteError::Missing) => return Ok(None),
            page => page?,
        };
        let root = self.root.to_lowercase();
        let (mut changed, mut removed) = (BTreeMap::new(), Vec::new());
        let cursor = self.pages(first, |entry| {
            match entry {
                Entry::File(metadata) => {
                    if let Some(file) = self.file(metadata) {
                        changed.insert(file.path.clone(), file);
                    }
                }
                Entry::Deleted { path_lower, .. } if path_lower == root => {
                    return Err(RemoteError::Failed(format!("папка {} пропала из Dropbox — проход остановлен", self.root)));
                }
                Entry::Deleted { path_display, .. } => {
                    if let Some(path) = self.relative(&path_display) {
                        let folder = format!("{path}/");
                        changed.retain(|known: &String, _| known != &path && !known.starts_with(&folder));
                        removed.push(path);
                    }
                }
                Entry::Folder {} => {}
            }
            Ok(())
        })?;
        Ok(Some(Delta { changed: changed.into_values().collect(), removed, cursor }))
    }

    fn download(&self, file: &RemoteFile, out: &mut dyn Write) -> RemoteResult<()> {
        let arg = json!({ "path": format!("rev:{}", file.rev) });
        let response = self.api.call(&format!("{CONTENT}files/download"), Some(&arg), Body::Empty)?;
        io::copy(&mut response.into_reader(), out)?;
        Ok(())
    }

    fn upload(&self, path: &str, content: &dyn Content, modified: u64, replaces: Option<&RemoteFile>) -> RemoteResult<RemoteFile> {
        let mode = match replaces {
            Some(file) => json!({ ".tag": "update", "update": file.rev }),
            None => json!("add"),
        };
        let commit = json!({ "path": format!("{}/{path}", self.root), "mode": mode, "autorename": false, "strict_conflict": true, "mute": true, "client_modified": timestamp(modified) });
        let metadata = match content.byte_len() {
            length if length <= SINGLE_UPLOAD => json(self.api.call(&format!("{CONTENT}files/upload"), Some(&commit), Body::Bytes(content, 0..length))?)?,
            _ => self.upload_parts(content, commit)?,
        };
        self.file(metadata).ok_or_else(|| RemoteError::Failed(format!("Dropbox записал {path} вне папки синхронизации")))
    }

    fn delete(&self, file: &RemoteFile) -> RemoteResult<()> {
        match self.api.rpc::<Value>("files/delete_v2", &json!({ "path": format!("{}/{}", self.root, file.path), "parent_rev": file.rev })) {
            Err(RemoteError::Missing) => Ok(()),
            deleted => deleted.map(drop),
        }
    }

    fn checksum(&self, content: &dyn Content) -> io::Result<String> {
        content_hash(content)
    }
}

fn timestamp(modified: u64) -> String {
    let (year, month, day, seconds) = to_civil_at(modified as i64 * 1_000_000, 0);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", seconds / 3600, seconds % 3600 / 60, seconds % 60)
}

#[cfg(test)]
mod tests;
