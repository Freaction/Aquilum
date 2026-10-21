use super::api::{ascii_json, content_hash};
use super::{timestamp, Dropbox, CLIENT};
use crate::search::note_date::parse_at;
use crate::sync::{Account, Tokens};
use crate::web_agent::web_agent;
use serde_json::json;

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
    let tokens = Tokens { access: String::new(), refresh: String::new(), expires: 0 };
    let account = Account::new(CLIENT, web_agent("test"), directory.path().join("dropbox.json"), tokens).unwrap();
    let dropbox = Dropbox::new(account, web_agent("test"), "База");

    assert_eq!(dropbox.relative("/База/Папка/Заметка.md").as_deref(), Some("Папка/Заметка.md"));
    assert_eq!(dropbox.relative("/база/Заметка.md").as_deref(), Some("Заметка.md"));
    assert_eq!(dropbox.relative("/База"), None);
    assert_eq!(dropbox.relative("/Другая/Заметка.md"), None);
    assert_eq!(dropbox.relative("/Базар/Заметка.md"), None);
}
