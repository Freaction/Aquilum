use crate::sync::{Account, RemoteError, RemoteResult};
use crate::text::content::{Content, ContentReader};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::io;
use std::ops::Range;
use std::thread;
use std::time::Duration;
use ureq::{Agent, Response};

const RPC: &str = "https://api.dropboxapi.com/2/";
pub const CONTENT: &str = "https://content.dropboxapi.com/2/";
const RETRIES: u32 = 4;
const HASH_BLOCK: usize = 4 * 1024 * 1024;

pub enum Body<'a> {
    Empty,
    Json(&'a Value),
    Bytes(&'a dyn Content, Range<usize>),
}

pub struct Api {
    account: Account,
    agent: Agent,
}

#[derive(Deserialize)]
struct Failure {
    error_summary: String,
}

impl Api {
    pub fn new(account: Account, agent: Agent) -> Self {
        Self { account, agent }
    }

    pub fn rpc<T: DeserializeOwned>(&self, route: &str, arg: &Value) -> RemoteResult<T> {
        json(self.call(&format!("{RPC}{route}"), None, Body::Json(arg))?)
    }

    pub fn call(&self, url: &str, arg: Option<&Value>, body: Body<'_>) -> RemoteResult<Response> {
        let mut renewed = false;
        let mut attempt = 0;
        loop {
            let mut request = self.agent.post(url).set("Authorization", &format!("Bearer {}", self.account.token()?));
            if let Some(arg) = arg {
                request = request.set("Dropbox-API-Arg", &ascii_json(arg));
            }
            let sent = match &body {
                Body::Empty => request.call(),
                Body::Json(value) => request.set("Content-Type", "application/json").send_string(&value.to_string()),
                Body::Bytes(content, range) => request
                    .set("Content-Type", "application/octet-stream")
                    .set("Content-Length", &range.len().to_string())
                    .send(ContentReader::range(*content, range.clone())),
            };
            match sent {
                Ok(response) => return Ok(response),
                Err(ureq::Error::Status(401, _)) if !renewed => {
                    self.account.renew()?;
                    renewed = true;
                }
                Err(ureq::Error::Status(429 | 500..=599, response)) if attempt < RETRIES => {
                    thread::sleep(pause(&response, attempt));
                    attempt += 1;
                }
                Err(ureq::Error::Status(401, _)) => return Err(RemoteError::Auth),
                Err(ureq::Error::Status(409, response)) => return Err(conflict(response)),
                Err(ureq::Error::Status(status, response)) => {
                    return Err(RemoteError::Failed(format!("Dropbox {status}: {}", response.into_string().unwrap_or_default())));
                }
                Err(error) => return Err(RemoteError::Failed(error.to_string())),
            }
        }
    }
}

pub fn json<T: DeserializeOwned>(response: Response) -> RemoteResult<T> {
    Ok(serde_json::from_reader(response.into_reader()).map_err(io::Error::from)?)
}

fn conflict(response: Response) -> RemoteError {
    let summary = serde_json::from_reader::<_, Failure>(response.into_reader()).map(|failure| failure.error_summary).unwrap_or_default();
    if summary.starts_with("reset") || summary.contains("not_found") {
        RemoteError::Missing
    } else if summary.contains("conflict") {
        RemoteError::Stale
    } else {
        RemoteError::Failed(format!("Dropbox: {summary}"))
    }
}

fn pause(response: &Response, attempt: u32) -> Duration {
    let seconds = response.header("Retry-After").and_then(|value| value.parse().ok()).unwrap_or(1u64 << attempt);
    Duration::from_secs(seconds.min(60))
}

pub fn ascii_json(value: &Value) -> String {
    let mut out = String::new();
    for symbol in value.to_string().chars() {
        if symbol < '\u{7f}' {
            out.push(symbol);
        } else {
            for unit in symbol.encode_utf16(&mut [0; 2]) {
                let _ = write!(out, "\\u{unit:04x}");
            }
        }
    }
    out
}

pub fn content_hash(content: &dyn Content) -> io::Result<String> {
    let mut whole = Sha256::new();
    for start in (0..content.byte_len()).step_by(HASH_BLOCK) {
        let mut block = Sha256::new();
        content.read(start..(start + HASH_BLOCK).min(content.byte_len()), &mut |bytes| {
            block.update(bytes);
            Ok(())
        })?;
        whole.update(block.finalize());
    }
    let mut hex = String::with_capacity(64);
    for byte in whole.finalize() {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}
