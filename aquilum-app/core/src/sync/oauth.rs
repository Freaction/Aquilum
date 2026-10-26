use super::remote::RemoteError;
use crate::history::store::millis;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, ErrorKind};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::SystemTime;
use ureq::Agent;
use url::{form_urlencoded, Url};

mod link;
mod loopback;

const ADDRESS: &str = "127.0.0.1:47219";

#[derive(Clone, Copy)]
pub struct Client {
    pub authorize: &'static str,
    pub connect: &'static str,
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

enum Answer {
    Loopback(TcpListener),
    Link(Receiver<String>),
}

pub struct Login {
    pub url: String,
    client: Client,
    answer: Answer,
    verifier: String,
    state: String,
}

impl Client {
    pub fn login(&self, links: Option<Receiver<String>>) -> io::Result<Login> {
        let (verifier, state) = (random()?, random()?);
        let (url, answer) = match links {
            None => {
                let listener = TcpListener::bind(ADDRESS)?;
                listener.set_nonblocking(true)?;
                (self.loopback_url(&verifier, &state)?, Answer::Loopback(listener))
            }
            Some(links) => (self.link_url(&verifier, &state)?, Answer::Link(links)),
        };
        Ok(Login { url, client: *self, answer, verifier, state })
    }

    fn loopback_url(&self, verifier: &str, state: &str) -> io::Result<String> {
        let mut url = parse(self.authorize)?;
        url.query_pairs_mut()
            .extend_pairs([
                ("response_type", "code"),
                ("client_id", self.id),
                ("redirect_uri", &redirect()),
                ("code_challenge", &challenge(verifier)),
                ("code_challenge_method", "S256"),
                ("state", state),
            ])
            .extend_pairs(self.params);
        Ok(url.into())
    }

    fn link_url(&self, verifier: &str, state: &str) -> io::Result<String> {
        let extra = form_urlencoded::Serializer::new(String::new())
            .extend_pairs([("code_challenge", challenge(verifier).as_str()), ("code_challenge_method", "S256")])
            .extend_pairs(self.params)
            .append_pair("response_type", "code")
            .finish();
        let mut url = parse(self.connect)?;
        url.query_pairs_mut().extend_pairs([("k", self.id), ("n", "0"), ("state", state), ("extra_query_params", &extra)]);
        Ok(url.into())
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
            Err(ureq::Error::Transport(transport)) => return Err(RemoteError::transport(transport)),
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
        let redirect = redirect();
        let mut form = vec![("grant_type", "authorization_code"), ("code", code.as_str()), ("code_verifier", self.verifier.as_str())];
        if let Answer::Loopback(_) = self.answer {
            form.push(("redirect_uri", &redirect));
        }
        self.client.grant(agent, &form, "")
    }

    fn code(&self, cancel: &AtomicBool) -> Result<String, RemoteError> {
        while !cancel.load(Ordering::Relaxed) {
            let code = match &self.answer {
                Answer::Loopback(listener) => loopback::poll(listener, &self.state)?,
                Answer::Link(links) => link::poll(links, &self.state)?,
            };
            if let Some(code) = code {
                return Ok(code);
            }
        }
        Err(RemoteError::Failed("вход отменён".into()))
    }
}

fn parse(url: &str) -> io::Result<Url> {
    Url::parse(url).map_err(|error| io::Error::new(ErrorKind::InvalidInput, error))
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
