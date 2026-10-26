use crate::sync::remote::RemoteError;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;
use url::Url;

pub(super) fn poll(links: &Receiver<String>, state: &str) -> Result<Option<String>, RemoteError> {
    match links.recv_timeout(Duration::from_millis(100)) {
        Ok(link) => answer(&link, state),
        Err(RecvTimeoutError::Timeout) => Ok(None),
        Err(RecvTimeoutError::Disconnected) => Err(RemoteError::Failed("вход отменён".into())),
    }
}

fn answer(link: &str, state: &str) -> Result<Option<String>, RemoteError> {
    let Ok(url) = Url::parse(link) else { return Ok(None) };
    let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
    if query.get("not_approved").is_some_and(|value| value == "true") {
        return Err(RemoteError::Failed("вход отклонён в облаке".into()));
    }
    let code = query.get("oauth_token").filter(|token| token.starts_with("oauth2code")).and(query.get("oauth_token_secret"));
    match code {
        Some(code) if query.get("state").map(String::as_str) == Some(state) => Ok(Some(code.clone())),
        Some(_) => Err(RemoteError::Failed("ответ входа не совпал с запросом".into())),
        None => Ok(None),
    }
}
