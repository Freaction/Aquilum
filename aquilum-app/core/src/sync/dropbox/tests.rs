use super::api::{ascii_json, Api, Body};
use super::hash::content_hash;
use super::{folders, timestamp, Dropbox, CLIENT};
use crate::http::{Connection, Response};
use crate::search::note_date::parse_at;
use crate::sync::{Account, RemoteError, Tokens};
use crate::web_agent::web_agent;
use serde_json::{json, Value};
use std::io::{ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const PAGE: &str = r#"{"entries":[],"cursor":"страница","has_more":false}"#;
const BUSY: &str = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const BROKEN: &str = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

enum Reply {
    Json(&'static str),
    Raw(&'static str),
    HangUp,
}

type Served = (Vec<(String, Vec<u8>)>, TcpListener);

fn account(directory: &Path) -> Account {
    let tokens = Tokens { access: "token".into(), refresh: String::new(), expires: u64::MAX };
    Account::new(CLIENT, web_agent("test"), directory.join("dropbox.json"), tokens).unwrap()
}

fn cloud(replies: Vec<Reply>) -> (&'static str, JoinHandle<Served>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url: &'static str = Box::leak(format!("http://{}/2/", listener.local_addr().unwrap()).into_boxed_str());
    let server = thread::spawn(move || {
        let requests = replies.into_iter().map_while(|reply| answer(&listener, reply)).collect();
        (requests, listener)
    });
    (url, server)
}

fn accept(listener: &TcpListener) -> Option<TcpStream> {
    let deadline = Instant::now() + Duration::from_secs(10);
    listener.set_nonblocking(true).unwrap();
    loop {
        match listener.accept() {
            Ok((stream, _)) => break stream.set_nonblocking(false).ok().map(|()| stream),
            Err(error) if error.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => thread::sleep(Duration::from_millis(1)),
            Err(_) => break None,
        }
    }
}

fn answer(listener: &TcpListener, reply: Reply) -> Option<(String, Vec<u8>)> {
    let stream = accept(listener)?;
    let mut connection = Connection::new(&stream).unwrap();
    let Ok(head) = connection.read_head() else { panic!("запрос не разобран") };
    let Ok(body) = connection.read_body(&head, 1 << 16) else { panic!("тело запроса не прочитано") };
    assert_eq!(head.header("Authorization"), Some("Bearer token"));
    match reply {
        Reply::Json(json) => connection.send(&Response::json(json.to_owned()), false).unwrap(),
        Reply::Raw(raw) => (&stream).write_all(raw.as_bytes()).unwrap(),
        Reply::HangUp => {}
    }
    Some((head.target, body))
}

fn api(url: &'static str, directory: &Path) -> Api {
    Api::at(url, account(directory), web_agent("test"), Duration::from_millis(1))
}

fn untouched(served: JoinHandle<Served>) -> usize {
    let (requests, listener) = served.join().unwrap();
    listener.set_nonblocking(true).unwrap();
    assert!(listener.accept().is_err(), "лишний повтор");
    requests.len()
}

#[test]
fn passing_failures_are_repeated_and_retry_after_is_kept() {
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = cloud(vec![Reply::HangUp, Reply::Raw(BROKEN), Reply::Raw(BUSY), Reply::Json(PAGE)]);
    let started = Instant::now();

    let page: Value = api(url, directory.path()).rpc("files/list_folder", &json!({ "path": "" })).unwrap();

    assert_eq!(page["cursor"], "страница");
    assert!(started.elapsed() >= Duration::from_secs(1), "Retry-After не выдержан: {:?}", started.elapsed());
    assert_eq!(untouched(server), 4);
}

#[test]
fn a_lost_connection_is_offline_after_three_repeats() {
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = cloud((0..4).map(|_| Reply::HangUp).collect());

    let listed = api(url, directory.path()).rpc::<Value>("files/list_folder", &json!({ "path": "" }));

    assert!(matches!(listed, Err(RemoteError::Offline(_))), "{listed:?}");
    assert_eq!(untouched(server), 4);
}

#[test]
fn an_upload_is_not_repeated_after_a_broken_connection() {
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = cloud(vec![Reply::HangUp]);
    let text: &[u8] = b"text";

    let sent = api(url, directory.path()).call(&format!("{url}files/upload"), Some(&json!({ "path": "/Заметка.md" })), Body::Bytes(&text, 0..4));

    assert!(matches!(sent, Err(RemoteError::Offline(_))), "{:?}", sent.err());
    assert_eq!(untouched(server), 1);
}

#[test]
fn the_content_hash_follows_the_dropbox_block_scheme() {
    let data: Vec<u8> = (0..9_000_003u32).map(|index| (index % 251) as u8).collect();

    assert_eq!(content_hash(&data.as_slice()).unwrap(), "3059bcac3d0b8a504296934f7b4cc3b6d88cb1e64b003970498407e2ffc4eba2");
    assert_eq!(content_hash(&&[][..]).unwrap(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn the_api_header_carries_only_ascii() {
    let arg = json!({ "path": "/Заметки/🙂.md" });
    let header = ascii_json(&arg);

    assert!(header.is_ascii());
    assert_eq!(serde_json::from_str::<serde_json::Value>(&header).unwrap(), arg);
}

#[test]
fn the_modification_time_survives_the_round_trip() {
    let stamp = timestamp(1_767_225_601_000);

    assert_eq!(stamp, "2026-01-01T00:00:01Z");
    assert_eq!(parse_at(stamp.trim_end_matches('Z'), 0), Some(1_767_225_601_000_000_000));
}

#[test]
fn paths_are_taken_relative_to_the_sync_folder() {
    let directory = tempfile::tempdir().unwrap();
    let dropbox = Dropbox::new(account(directory.path()), web_agent("test"), "База");

    assert_eq!(dropbox.relative("/База/Папка/Заметка.md").as_deref(), Some("Папка/Заметка.md"));
    assert_eq!(dropbox.relative("/база/Заметка.md").as_deref(), Some("Заметка.md"));
    assert_eq!(dropbox.relative("/База"), None);
    assert_eq!(dropbox.relative("/Другая/Заметка.md"), None);
    assert_eq!(dropbox.relative("/Базар/Заметка.md"), None);
}

#[test]
fn vaults_are_the_visible_top_folders_on_every_page() {
    let first = r#"{"entries":[
        {".tag":"folder","name":"NeuroNet","path_lower":"/neuronet","path_display":"/NeuroNet","id":"id:1"},
        {".tag":"file","name":"Заметка.md","path_lower":"/заметка.md","path_display":"/Заметка.md","id":"id:2","rev":"015f","content_hash":"00","size":1,"client_modified":"2026-01-01T00:00:00Z","server_modified":"2026-01-01T00:00:00Z"},
        {".tag":"folder","name":".obsidian","path_lower":"/.obsidian","path_display":"/.obsidian","id":"id:3"}
    ],"cursor":"страница-1","has_more":true}"#;
    let second = r#"{"entries":[
        {".tag":"folder","name":"aquilum","path_lower":"/aquilum","path_display":"/aquilum","id":"id:4"},
        {".tag":"deleted","name":"Старая","path_lower":"/старая","path_display":"/Старая"},
        {".tag":"folder","name":"._Мусор","path_lower":"/._мусор","path_display":"/._Мусор","id":"id:5"}
    ],"cursor":"страница-2","has_more":false}"#;
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = cloud(vec![Reply::Json(first), Reply::Json(second)]);

    let vaults = folders(&api(url, directory.path())).unwrap();

    assert_eq!(vaults, ["aquilum", "NeuroNet"]);
    let requests: Vec<(String, Value)> = server.join().unwrap().0.into_iter().map(|(target, body)| (target, serde_json::from_slice(&body).unwrap())).collect();
    assert_eq!(requests[0], ("/2/files/list_folder".to_owned(), json!({ "path": "", "recursive": false })));
    assert_eq!(requests[1], ("/2/files/list_folder/continue".to_owned(), json!({ "cursor": "страница-1" })));
}
