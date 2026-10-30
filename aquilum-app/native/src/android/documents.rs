use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, PoisonError};

use jni::jni_str;
use jni::strings::JNIStr;

use super::java;

static WAITING: Mutex<Option<Sender<Option<String>>>> = Mutex::new(None);

pub fn deliver(uri: Option<String>) {
    if let Some(sender) = WAITING.lock().unwrap_or_else(PoisonError::into_inner).take() {
        let _ = sender.send(uri);
    }
}

fn ask(method: &JNIStr, argument: &str) -> Option<String> {
    let (sender, receiver) = channel();
    *WAITING.lock().unwrap_or_else(PoisonError::into_inner) = Some(sender);
    if !java::send_text(method, argument) {
        return None;
    }
    receiver.recv().ok().flatten()
}

fn open(uri: &str, mode: &str) -> std::io::Result<File> {
    let fd = java::ask_int(jni_str!("openDocument"), [uri, mode]).ok_or_else(|| std::io::Error::other("документ не открылся"))?;
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub fn pick(extensions: &[&str]) -> Option<PathBuf> {
    let dir = Path::new(&java::read_text(jni_str!("cacheDir"))?).join("picked");
    let _ = std::fs::remove_dir_all(&dir);
    let uri = ask(jni_str!("pickDocument"), &extensions.join(" "))?;
    let name = java::ask_text(jni_str!("documentName"), &uri).unwrap_or_default();
    let path = dir.join(Path::new(&name).file_name().unwrap_or(OsStr::new("document")));
    let copied = std::fs::create_dir_all(&dir).and_then(|()| std::io::copy(&mut open(&uri, "r")?, &mut File::create(&path)?));
    copied.map(|_| path).map_err(|error| report!("{name} не загружен: {error}")).ok()
}

pub fn create(name: &str) -> Option<String> {
    ask(jni_str!("createDocument"), name)
}

pub fn write(uri: &str, bytes: &[u8]) -> Result<(), String> {
    open(uri, "w").and_then(|mut file| file.write_all(bytes)).map_err(|error| error.to_string())
}
