use super::types::{Entry, Page};
use crate::sync::{Account, RemoteError, RemoteResult};
use crate::text::content::{Content, ContentReader};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::ops::Range;
use std::thread;
use std::time::Duration;
use ureq::{Agent, ErrorKind, Response};

const RPC: &str = "https://api.dropboxapi.com/2/";
pub const CONTENT: &str = "https://content.dropboxapi.com/2/";
const RETRIES: u32 = 3;
const FIRST_PAUSE: Duration = Duration::from_secs(1);
const LONGEST_PAUSE: Duration = Duration::from_secs(60);
const COPY_BLOCK: usize = 64 * 1024;

pub enum Body<'a> {
    Empty,
    Json(&'a Value),
    Bytes(&'a dyn Content, Range<usize>),
}

impl Body<'_> {
    fn resendable(&self) -> bool {
        !matches!(self, Self::Bytes(..))
    }
}

pub struct Api {
    account: Account,
    agent: Agent,
    rpc: &'static str,
    pause: Duration,
}

enum Fault {
    Passing(Option<Duration>, RemoteError),
    Final(RemoteError),
}

#[derive(Deserialize)]
struct Failure {
    error_summary: String,
}

impl Api {
    pub fn new(account: Account, agent: Agent) -> Self {
        Self::at(RPC, account, agent, FIRST_PAUSE)
    }

    pub fn at(rpc: &'static str, account: Account, agent: Agent, pause: Duration) -> Self {
        Self { account, agent, rpc, pause }
    }

    pub fn rpc<T: DeserializeOwned>(&self, route: &str, arg: &Value) -> RemoteResult<T> {
        self.send(&format!("{}{route}", self.rpc), None, Body::Json(arg), json)
    }

    pub fn pages(&self, mut page: Page, mut visit: impl FnMut(Entry) -> RemoteResult<()>) -> RemoteResult<String> {
        loop {
            for entry in page.entries {
                visit(entry)?;
            }
            if !page.has_more {
                return Ok(page.cursor);
            }
            page = self.rpc("files/list_folder/continue", &json!({ "cursor": page.cursor }))?;
        }
    }

    pub fn call(&self, url: &str, arg: Option<&Value>, body: Body<'_>) -> RemoteResult<Response> {
        self.send(url, arg, body, Ok)
    }

    fn send<T>(&self, url: &str, arg: Option<&Value>, body: Body<'_>, read: impl Fn(Response) -> RemoteResult<T>) -> RemoteResult<T> {
        let mut renewed = false;
        let mut attempt = 0;
        loop {
            let token = self.account.token()?;
            let fault = match self.request(url, &token, arg, &body) {
                Ok(response) => match read(response) {
                    Err(RemoteError::Offline(message)) if body.resendable() => Fault::Passing(None, RemoteError::Offline(message)),
                    read => return read,
                },
                Err(ureq::Error::Status(401, _)) if !renewed => {
                    self.account.renew()?;
                    renewed = true;
                    continue;
                }
                Err(error) => fault(error, body.resendable()),
            };
            match fault {
                Fault::Passing(after, _) if attempt < RETRIES => {
                    thread::sleep(after.unwrap_or(self.pause * 2u32.pow(attempt)).min(LONGEST_PAUSE));
                    attempt += 1;
                }
                Fault::Passing(_, error) | Fault::Final(error) => return Err(error),
            }
        }
    }

    fn request(&self, url: &str, token: &str, arg: Option<&Value>, body: &Body<'_>) -> Result<Response, ureq::Error> {
        let mut request = self.agent.post(url).set("Authorization", &format!("Bearer {token}"));
        if let Some(arg) = arg {
            request = request.set("Dropbox-API-Arg", &ascii_json(arg));
        }
        match body {
            Body::Empty => request.call(),
            Body::Json(value) => request.set("Content-Type", "application/json").send_string(&value.to_string()),
            Body::Bytes(content, range) => request
                .set("Content-Type", "application/octet-stream")
                .set("Content-Length", &range.len().to_string())
                .send(ContentReader::range(*content, range.clone())),
        }
    }
}

fn fault(error: ureq::Error, resendable: bool) -> Fault {
    match error {
        ureq::Error::Status(401, _) => Fault::Final(RemoteError::Auth),
        ureq::Error::Status(409, response) => Fault::Final(conflict(response)),
        ureq::Error::Status(status @ (429 | 500..=599), response) => Fault::Passing(retry_after(&response), refused(status, response)),
        ureq::Error::Status(status, response) => Fault::Final(refused(status, response)),
        ureq::Error::Transport(transport) => {
            let passing = matches!(transport.kind(), ErrorKind::Dns | ErrorKind::ConnectionFailed) || resendable && transport.kind() == ErrorKind::Io;
            let error = RemoteError::transport(transport);
            if passing { Fault::Passing(None, error) } else { Fault::Final(error) }
        }
    }
}

fn retry_after(response: &Response) -> Option<Duration> {
    response.header("Retry-After").and_then(|value| value.trim().parse().ok()).map(Duration::from_secs)
}

fn refused(status: u16, response: Response) -> RemoteError {
    RemoteError::Failed(format!("Dropbox {status}: {}", response.into_string().unwrap_or_default()))
}

pub fn json<T: DeserializeOwned>(response: Response) -> RemoteResult<T> {
    serde_json::from_reader(response.into_reader()).map_err(|error| match error.is_io() {
        true => RemoteError::Offline(error.to_string()),
        false => RemoteError::Failed(error.to_string()),
    })
}

pub fn copy(from: &mut dyn Read, to: &mut dyn Write) -> RemoteResult<()> {
    let mut block = [0; COPY_BLOCK];
    loop {
        match from.read(&mut block) {
            Ok(0) => return Ok(()),
            Ok(read) => to.write_all(&block[..read])?,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(RemoteError::Offline(error.to_string())),
        }
    }
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
