use super::{challenge, Client, Tokens, ADDRESS};
use crate::sync::remote::RemoteError;
use crate::http::{Connection, Response};
use crate::sync::Account;
use crate::web_agent::web_agent;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use url::Url;

static PORT: Mutex<()> = Mutex::new(());

fn token_server(reply: &'static str) -> (&'static str, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url: &'static str = Box::leak(format!("http://{}/token", listener.local_addr().unwrap()).into_boxed_str());
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut connection = Connection::new(&stream).unwrap();
        let Ok(head) = connection.read_head() else { panic!("запрос токена не разобран") };
        let Ok(body) = connection.read_body(&head, 1 << 16) else { panic!("тело запроса токена не прочитано") };
        connection.send(&Response::json(reply.to_owned()), false).unwrap();
        String::from_utf8(body).unwrap()
    });
    (url, server)
}

fn client(token: &'static str) -> Client {
    Client { authorize: "https://cloud.example/authorize", connect: "https://cloud.example/1/connect", token, id: "aquilum", secret: "", params: &[("scope", "files")] }
}

#[test]
fn the_challenge_matches_rfc_7636() {
    assert_eq!(challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
}

#[test]
fn the_code_from_the_browser_becomes_tokens() {
    let _port = PORT.lock().unwrap();
    let (token, server) = token_server(r#"{"access_token":"доступ","refresh_token":"обновление","expires_in":60}"#);
    let login = client(token).login(None).unwrap();
    let query: HashMap<String, String> = Url::parse(&login.url).unwrap().query_pairs().into_owned().collect();
    assert_eq!((query["client_id"].as_str(), query["scope"].as_str(), query["code_challenge_method"].as_str()), ("aquilum", "files", "S256"));
    let state = query["state"].clone();
    let browser = thread::spawn(move || {
        let mut stream = TcpStream::connect(ADDRESS).unwrap();
        write!(stream, "GET /?code=abc&state={state} HTTP/1.1\r\nHost: {ADDRESS}\r\n\r\n").unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).unwrap();
        answer
    });

    let tokens = login.finish(&web_agent("test"), &AtomicBool::new(false)).unwrap();

    assert_eq!((tokens.access.as_str(), tokens.refresh.as_str()), ("доступ", "обновление"));
    assert!(browser.join().unwrap().contains("Вход выполнен"));
    let form = server.join().unwrap();
    assert!(form.contains("grant_type=authorization_code") && form.contains("code=abc") && form.contains("code_verifier="));
}

#[test]
fn a_cancelled_login_frees_the_port() {
    let _port = PORT.lock().unwrap();
    let login = client("http://127.0.0.1:9/token").login(None).unwrap();

    assert!(login.finish(&web_agent("test"), &AtomicBool::new(true)).is_err());
    assert!(client("http://127.0.0.1:9/token").login(None).is_ok());
}

#[test]
fn an_expired_token_is_renewed_and_saved() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("dropbox.json");
    let (token, server) = token_server(r#"{"access_token":"новый","refresh_token":"другой","expires_in":3600}"#);
    let stale = Tokens { access: "старый".into(), refresh: "old-refresh".into(), expires: 0 };
    let account = Account::new(client(token), web_agent("test"), path.clone(), stale).unwrap();

    assert_eq!(account.token().unwrap(), "новый");
    assert!(server.join().unwrap().contains("refresh_token=old-refresh"));
    let reopened = Account::open(client(token), web_agent("test"), path).unwrap().unwrap();
    assert_eq!(reopened.token().unwrap(), "новый");
}

fn link_login(token: &'static str, link: impl FnOnce(&str) -> String) -> super::Login {
    let (links, receiver) = channel();
    let login = client(token).login(Some(receiver)).unwrap();
    let state = Url::parse(&login.url).unwrap().query_pairs().find(|(key, _)| key == "state").unwrap().1.into_owned();
    links.send(link(&state)).unwrap();
    login
}

#[test]
fn the_app_link_from_the_cloud_becomes_tokens() {
    let (token, server) = token_server(r#"{"access_token":"доступ","refresh_token":"обновление","expires_in":60}"#);
    let login = link_login(token, |state| format!("db-aquilum://1/connect?oauth_token=oauth2code%3A&oauth_token_secret=abc&uid=7&state={state}"));
    let url = Url::parse(&login.url).unwrap();
    assert_eq!(url.path(), "/1/connect");
    let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
    assert_eq!((query["k"].as_str(), query["n"].as_str()), ("aquilum", "0"));
    let extra: HashMap<String, String> = url::form_urlencoded::parse(query["extra_query_params"].as_bytes()).into_owned().collect();
    assert_eq!((extra["code_challenge_method"].as_str(), extra["scope"].as_str(), extra["response_type"].as_str()), ("S256", "files", "code"));

    let tokens = login.finish(&web_agent("test"), &AtomicBool::new(false)).unwrap();

    assert_eq!((tokens.access.as_str(), tokens.refresh.as_str()), ("доступ", "обновление"));
    let form = server.join().unwrap();
    assert!(form.contains("code=abc") && form.contains("code_verifier=") && !form.contains("redirect_uri"), "{form}");
}

#[test]
fn a_foreign_or_refused_link_fails_the_login() {
    let token = "http://127.0.0.1:9/token";
    let failure = |link: &'static str| match link_login(token, |_| link.into()).finish(&web_agent("test"), &AtomicBool::new(false)) {
        Err(RemoteError::Failed(message)) => message,
        other => panic!("{link}: {:?}", other.map(|tokens| tokens.access)),
    };
    assert_eq!(failure("db-aquilum://1/connect?oauth_token=oauth2code%3A&oauth_token_secret=abc&state=чужое"), "ответ входа не совпал с запросом");
    assert_eq!(failure("db-aquilum://1/connect?not_approved=true"), "вход отклонён в облаке");
}
