use super::oauth::{Client, Tokens};
use super::remote::RemoteError;
use crate::files::document::write_content_atomic;
use crate::history::store::millis;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use ureq::Agent;

const MARGIN_MS: u64 = 60_000;

pub struct Account {
    client: Client,
    agent: Agent,
    path: PathBuf,
    tokens: Mutex<Tokens>,
}

impl Account {
    pub fn new(client: Client, agent: Agent, path: PathBuf, tokens: Tokens) -> io::Result<Self> {
        save(&path, &tokens)?;
        Ok(Self { client, agent, path, tokens: Mutex::new(tokens) })
    }

    pub fn open(client: Client, agent: Agent, path: PathBuf) -> io::Result<Option<Self>> {
        let tokens = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        Ok(Some(Self { client, agent, path, tokens: Mutex::new(tokens) }))
    }

    pub fn token(&self) -> Result<String, RemoteError> {
        let mut tokens = self.tokens.lock().expect("токены облака");
        if tokens.expires > millis(SystemTime::now()) + MARGIN_MS {
            return Ok(tokens.access.clone());
        }
        self.refresh(&mut tokens)
    }

    pub fn renew(&self) -> Result<String, RemoteError> {
        self.refresh(&mut self.tokens.lock().expect("токены облака"))
    }

    fn refresh(&self, tokens: &mut Tokens) -> Result<String, RemoteError> {
        *tokens = self.client.refresh(&self.agent, &tokens.refresh)?;
        save(&self.path, tokens)?;
        Ok(tokens.access.clone())
    }
}

fn save(path: &Path, tokens: &Tokens) -> io::Result<()> {
    let json = serde_json::to_string(tokens)?;
    write_content_atomic(path, &json, None).map(|_| ()).map_err(|error| io::Error::other(error.to_string()))
}
