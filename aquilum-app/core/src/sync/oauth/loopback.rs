use super::ADDRESS;
use crate::http::{Connection, Response};
use crate::sync::remote::RemoteError;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use url::Url;

pub(super) fn poll(listener: &TcpListener, state: &str) -> Result<Option<String>, RemoteError> {
    match listener.accept() {
        Ok((stream, _)) => answer(&stream, state),
        Err(error) if error.kind() == ErrorKind::WouldBlock => {
            thread::sleep(Duration::from_millis(100));
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn answer(stream: &TcpStream, state: &str) -> Result<Option<String>, RemoteError> {
    stream.set_nonblocking(false)?;
    let mut connection = Connection::new(stream)?;
    let Ok(head) = connection.read_head() else {
        return Ok(None);
    };
    let query: HashMap<String, String> = Url::parse(&format!("http://{ADDRESS}{}", head.target))
        .map(|url| url.query_pairs().into_owned().collect())
        .unwrap_or_default();
    let (message, outcome) = match (query.get("code"), query.get("error")) {
        (Some(code), _) if query.get("state").map(String::as_str) == Some(state) => ("Вход выполнен, вкладку можно закрыть.", Ok(Some(code.clone()))),
        (Some(_), _) => ("Вход не удался, вернитесь в Aquilum.", Err(RemoteError::Failed("ответ входа не совпал с запросом".into()))),
        (None, Some(error)) => ("Вход не удался, вернитесь в Aquilum.", Err(RemoteError::Failed(format!("облако отказало во входе: {error}")))),
        (None, None) => ("", Ok(None)),
    };
    connection.send(&Response::text(200, message), false)?;
    connection.hang_up();
    outcome
}
