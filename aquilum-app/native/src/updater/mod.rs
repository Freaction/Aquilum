mod apply;
pub mod attempts;
mod release;
#[cfg(target_os = "macos")]
mod tar;
#[cfg(test)]
mod tests;

use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub use attempts::Attempt;
use release::{PUBLIC_KEY, Release, find, verify};

pub use apply::{UPDATE_DIR, apply};

const CHUNK: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Checking,
    Latest,
    Available(String),
    Downloading { version: String, done: u64, total: Option<u64> },
    Ready(String),
    Installing,
    Failed(String),
    ManualRequired { version: String, url: String },
}

#[derive(Clone)]
pub struct Updater {
    data_dir: PathBuf,
    status: Arc<Mutex<Status>>,
    payload: Arc<Mutex<Option<(String, String, Vec<u8>)>>>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

pub fn available() -> bool {
    cfg!(feature = "installed") && apply::supported()
}

impl Updater {
    pub fn new(data_dir: PathBuf, wake: Arc<dyn Fn() + Send + Sync>) -> Updater {
        let status = match Attempt::check_on_start(&data_dir, env!("CARGO_PKG_VERSION")) {
            Some(attempt) => Status::ManualRequired { version: attempt.version, url: attempt.url },
            None => Status::Idle,
        };
        Updater { data_dir, status: Arc::new(Mutex::new(status)), payload: Arc::new(Mutex::new(None)), wake }
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set(&self, status: Status) {
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
        (self.wake)();
    }

    fn busy(&self) -> bool {
        matches!(self.status(), Status::Checking | Status::Downloading { .. } | Status::Ready(_) | Status::Installing)
    }

    pub fn check(&self) {
        if !available() || self.busy() {
            return;
        }
        Attempt::clear(&self.data_dir);
        self.set(Status::Checking);
        let this = self.clone();
        std::thread::spawn(move || {
            this.set(match find() {
                Ok(Some(release)) => Status::Available(release.version),
                Ok(None) => Status::Latest,
                Err(error) => Status::Failed(error),
            });
        });
    }

    pub fn install(&self) {
        if !available() || self.busy() || matches!(self.status(), Status::ManualRequired { .. }) {
            return;
        }
        self.set(Status::Checking);
        let this = self.clone();
        std::thread::spawn(move || {
            let result = find().and_then(|release| match release {
                Some(release) => this.download(&release).map(|()| Some(release.version)),
                None => Ok(None),
            });
            this.set(match result {
                Ok(Some(version)) => Status::Ready(version),
                Ok(None) => Status::Latest,
                Err(error) => Status::Failed(error),
            });
        });
    }

    fn download(&self, release: &Release) -> Result<(), String> {
        let agent = aquilum_core::web_agent::web_agent("updates");
        let response = agent.get(&release.url).call().map_err(|e| e.to_string())?;
        let total = response.header("Content-Length").and_then(|v| v.parse().ok());
        let mut reader = response.into_reader();
        let mut data = Vec::with_capacity(total.unwrap_or(0) as usize);
        let mut buffer = vec![0u8; CHUNK];
        loop {
            let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..read]);
            self.set(Status::Downloading { version: release.version.clone(), done: data.len() as u64, total });
        }
        verify(&data, &release.signature, PUBLIC_KEY)?;
        *self.payload.lock().unwrap_or_else(|e| e.into_inner()) = Some((release.version.clone(), release.url.clone(), data));
        Ok(())
    }

    pub fn record_attempt(&self, version: &str, url: &str) {
        Attempt::record(&self.data_dir, version, url);
    }

    pub fn fail(&self, error: String) {
        self.set(Status::Failed(error));
    }

    pub fn take_ready(&self) -> Option<(String, String, Vec<u8>)> {
        if !matches!(self.status(), Status::Ready(_)) {
            return None;
        }
        let payload = self.payload.lock().unwrap_or_else(|e| e.into_inner()).take()?;
        self.set(Status::Installing);
        Some(payload)
    }
}
