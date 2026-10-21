use super::remote::RemoteError;
use crate::history::store::millis;
use crate::http::{Connection, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};
use ureq::Agent;
use url::Url;

const ADDRESS: &str = "127.0.0.1:47219";

#[derive(Clone, Copy)]
pub struct Client {
    pub authorize: &'static str,
    pub token: &'static str,
    pub id: &'static str,
    pub secret: &'static str,
    pub params: &'static [(&'static str, &'static str)],
}

#[derive(Serialize, Deserialize)]
pub struct Tokens {
    pub access: String,
    pub refresh: String,
    pub expires: u64,
}

#[derive(Deserialize)]
struct Grant {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

pub struct Login {
    pub url: String,
    client: Client,
    listener: TcpListener,
    verifier: String,
    state: String,
}

impl Client {
    pub fn login(&self) -> io::Result<Login> {
        let listener = TcpListener::bind(ADDRESS)?;
        listener.set_nonblocking(true)?;
        let (verifier, state) = (random()?, random()?);
        let mut url = Url::parse(self.authorize).map_err(|error| io::Error::new(ErrorKind::InvalidInput, error))?;
        url.query_pairs_mut()
            .extend_pairs([
                ("response_type", "code"),
                ("client_id", self.id),
                ("redirect_uri", &redirect()),
                ("code_challenge", &challenge(&verifier)),
                ("code_challenge_method", "S256"),
                ("state", &state),
            ])
            .extend_pairs(self.params);
        Ok(Login { url: url.into(), client: *self, listener, verifier, state })
    }

    pub fn refresh(&self, agent: &Agent, refresh: &str) -> Result<Tokens, RemoteError> {
        self.grant(agent, &[("grant_type", "refresh_token"), ("refresh_token", refresh)], refresh)
    }

    fn grant(&self, agent: &Agent, form: &[(&str, &str)], refresh: &str) -> Result<Tokens, RemoteError> {
        let mut form = form.to_vec();
        form.push(("client_id", self.id));
        if !self.secret.is_empty() {
            form.push(("client_secret", self.secret));
        }
        let grant: Grant = match agent.post(self.token).send_form(&form) {
            Ok(response) => serde_json::from_reader(response.into_reader()).map_err(io::Error::from)?,
            Err(ureq::Error::Status(400 | 401, _)) => return Err(RemoteError::Auth),
            Err(error) => return Err(RemoteError::Failed(error.to_string())),
        };
        Ok(Tokens {
            access: grant.access_token,
            refresh: grant.refresh_token.unwrap_or_else(|| refresh.to_owned()),
            expires: millis(SystemTime::now()) + grant.expires_in.unwrap_or(3600) * 1000,
        })
    }
}

impl Login {
    pub fn finish(self, agent: &Agent, cancel: &AtomicBool) -> Result<Tokens, RemoteError> {
        let code = self.code(cancel)?;
        let form = [("grant_type", "authorization_code"), ("code", &code), ("redirect_uri", &redirect()), ("code_verifier", &self.verifier)];
        self.client.grant(agent, &form, "")
    }

    fn code(&self, cancel: &AtomicBool) -> Result<String, RemoteError> {
        while !cancel.load(Ordering::Relaxed) {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(code) = self.answer(&stream)? {
                        return Ok(code);
                    }
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(100)),
                Err(error) => return Err(error.into()),
            }
        }
        Err(RemoteError::Failed("вход отменён".into()))
    }

    fn answer(&self, stream: &TcpStream) -> Result<Option<String>, RemoteError> {
        stream.set_nonblocking(false)?;
        let mut connection = Connection::new(stream)?;
        let Ok(head) = connection.read_head() else {
            return Ok(None);
        };
        let query: HashMap<String, String> = Url::parse(&format!("http://{ADDRESS}{}", head.target))
            .map(|url| url.query_pairs().into_owned().collect())
            .unwrap_or_default();
        let (message, outcome) = match (query.get("code"), query.get("error")) {
            (Some(code), _) if query.get("state") == Some(&self.state) => ("Вход выполнен, вкладку можно закрыть.", Ok(Some(code.clone()))),
            (Some(_), _) => ("Вход не удался, вернитесь в Aquilum.", Err(RemoteError::Failed("ответ входа не совпал с запросом".into()))),
            (None, Some(error)) => ("Вход не удался, вернитесь в Aquilum.", Err(RemoteError::Failed(format!("облако отказало во входе: {error}")))),
            (None, None) => ("", Ok(None)),
        };
        connection.send(&Response::text(200, message), false)?;
        connection.hang_up();
        outcome
    }
}

fn redirect() -> String {
    format!("http://{ADDRESS}/")
}

fn random() -> io::Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier))
}

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;
