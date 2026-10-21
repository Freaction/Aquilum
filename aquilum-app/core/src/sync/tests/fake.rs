use crate::sync::{Delta, Remote, RemoteError, RemoteFile, RemoteResult};
use crate::text::content::{read_bytes, Content};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Default)]
struct State {
    files: BTreeMap<String, RemoteFile>,
    revisions: HashMap<String, Vec<u8>>,
    log: Vec<String>,
    uploads: usize,
    refuse_uploads: bool,
    expired: bool,
    short_downloads: bool,
    offline: bool,
    slow: Duration,
}

#[derive(Default)]
pub struct FakeRemote {
    state: Mutex<State>,
}

impl FakeRemote {
    pub fn put(&self, path: &str, text: &str) {
        let mut state = self.state.lock().unwrap();
        store(&mut state, path, text.as_bytes().to_vec(), 0);
    }

    pub fn put_quietly(&self, path: &str, text: &str) {
        let mut state = self.state.lock().unwrap();
        store(&mut state, path, text.as_bytes().to_vec(), 0);
        state.log.pop();
    }

    pub fn short_downloads(&self, short: bool) {
        self.state.lock().unwrap().short_downloads = short;
    }

    pub fn text(&self, path: &str) -> Option<String> {
        let state = self.state.lock().unwrap();
        let file = state.files.get(path)?;
        Some(String::from_utf8(state.revisions[&file.rev].clone()).unwrap())
    }

    pub fn paths(&self) -> Vec<String> {
        self.state.lock().unwrap().files.keys().cloned().collect()
    }

    pub fn uploads(&self) -> usize {
        self.state.lock().unwrap().uploads
    }

    pub fn forget_history(&self) {
        let mut state = self.state.lock().unwrap();
        let current: BTreeSet<String> = state.files.values().map(|file| file.rev.clone()).collect();
        state.revisions.retain(|rev, _| current.contains(rev));
    }

    pub fn remove_folder(&self, folder: &str) {
        let mut state = self.state.lock().unwrap();
        state.files.retain(|path, _| !path.starts_with(&format!("{folder}/")));
        state.log.push(folder.to_owned());
    }

    pub fn wipe_and_expire(&self) {
        let mut state = self.state.lock().unwrap();
        state.files.clear();
        state.expired = true;
    }

    pub fn refuse_uploads(&self, refuse: bool) {
        self.state.lock().unwrap().refuse_uploads = refuse;
    }

    pub fn go_offline(&self) {
        self.state.lock().unwrap().offline = true;
    }

    pub fn slow_uploads(&self, slow: Duration) {
        self.state.lock().unwrap().slow = slow;
    }

    fn online(&self) -> RemoteResult<std::sync::MutexGuard<'_, State>> {
        let state = self.state.lock().unwrap();
        if state.offline { Err(RemoteError::Failed("нет сети".into())) } else { Ok(state) }
    }
}

fn store(state: &mut State, path: &str, bytes: Vec<u8>, modified: u64) -> RemoteFile {
    let rev = format!("r{}", state.log.len() + 1);
    let file = RemoteFile {
        path: path.to_owned(),
        id: format!("id:{path}"),
        rev: rev.clone(),
        checksum: blake3::hash(&bytes).to_hex().to_string(),
        size: bytes.len() as u64,
        modified,
    };
    state.revisions.insert(rev, bytes);
    state.files.insert(path.to_owned(), file.clone());
    state.log.push(path.to_owned());
    file
}

impl Remote for FakeRemote {
    fn cursor(&self) -> RemoteResult<String> {
        Ok(self.online()?.log.len().to_string())
    }

    fn list(&self) -> RemoteResult<Vec<RemoteFile>> {
        Ok(self.online()?.files.values().cloned().collect())
    }

    fn delta(&self, cursor: &str) -> RemoteResult<Option<Delta>> {
        let mut state = self.online()?;
        if std::mem::take(&mut state.expired) {
            return Ok(None);
        }
        let from: usize = cursor.parse().map_err(|_| RemoteError::Failed("курсор".into()))?;
        let touched: BTreeSet<&String> = state.log[from..].iter().collect();
        let changed = touched.iter().filter_map(|path| state.files.get(*path).cloned()).collect();
        let removed = touched.iter().filter(|path| !state.files.contains_key(**path)).map(|path| (*path).clone()).collect();
        Ok(Some(Delta { changed, removed, cursor: state.log.len().to_string() }))
    }

    fn download(&self, file: &RemoteFile, out: &mut dyn Write) -> RemoteResult<()> {
        let state = self.state.lock().unwrap();
        let bytes = state.revisions.get(&file.rev).ok_or(RemoteError::Missing)?;
        let length = if state.short_downloads { bytes.len() / 2 } else { bytes.len() };
        Ok(out.write_all(&bytes[..length])?)
    }

    fn upload(&self, path: &str, content: &dyn Content, modified: u64, replaces: Option<&RemoteFile>) -> RemoteResult<RemoteFile> {
        let slow = self.state.lock().unwrap().slow;
        std::thread::sleep(slow);
        let bytes = read_bytes(content, 0..content.byte_len())?;
        let mut state = self.state.lock().unwrap();
        if state.refuse_uploads {
            return Err(RemoteError::Failed("облако недоступно".into()));
        }
        let current = state.files.get(path).map(|file| file.rev.as_str());
        if current != replaces.map(|file| file.rev.as_str()) {
            return Err(RemoteError::Stale);
        }
        state.uploads += 1;
        Ok(store(&mut state, path, bytes, modified))
    }

    fn delete(&self, file: &RemoteFile) -> RemoteResult<()> {
        let mut state = self.state.lock().unwrap();
        if state.files.get(&file.path).map(|current| &current.rev) != Some(&file.rev) {
            return Err(RemoteError::Stale);
        }
        state.files.remove(&file.path);
        state.log.push(file.path.clone());
        Ok(())
    }

    fn checksum(&self, content: &dyn Content) -> io::Result<String> {
        let bytes = read_bytes(content, 0..content.byte_len())?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
}
