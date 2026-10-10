use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::Inner;
use crate::ui::editor::resolve::{Fetch, Media};

const RECENT: usize = 24;

static LOADED: Mutex<VecDeque<(String, Media)>> = Mutex::new(VecDeque::new());

pub(super) fn recent(key: &str) -> Option<Media> {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| k == key).map(|(_, m)| m.clone())
}

#[cfg(test)]
pub(super) fn recent_count(prefix: &str) -> usize {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|(k, _)| k.starts_with(prefix)).count()
}

pub(super) fn forget_key(key: &str) {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).retain(|(k, _)| k != key);
}

pub(super) fn forget(paths: &[PathBuf]) {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).retain(|(_, m)| m.path.as_ref().is_none_or(|p| !paths.contains(p)));
}

const REMOTE_DAYS: u64 = 30;

pub fn prune_remote(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let limit = std::time::Duration::from_secs(REMOTE_DAYS * 24 * 60 * 60);
    for entry in entries.flatten() {
        let stale = entry.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > limit);
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

pub(super) fn remember(key: String, media: Media) {
    let mut loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    loaded.retain(|(k, _)| *k != key);
    loaded.push_front((key, media));
    loaded.truncate(RECENT);
}

pub(super) fn locate(inner: &Inner, source: &str) -> Option<PathBuf> {
    let source = source.trim();
    let source = source.strip_prefix("file:///").or_else(|| source.strip_prefix("file://")).unwrap_or(source);
    let decoded = percent_decode(source);
    let path = Path::new(&decoded);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    let mut bases = std::iter::once(inner.root.as_path()).chain(inner.document.ancestors().skip(1).take_while(|d| d.starts_with(&inner.root)));
    bases.find_map(|base| Some(join(base, &decoded)).filter(|p| p.is_file()))
}

fn join(base: &Path, relative: &str) -> PathBuf {
    let mut joined = base.to_path_buf();
    for part in relative.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                joined.pop();
            }
            part => joined.push(part),
        }
    }
    joined
}

fn percent_decode(text: &str) -> String {
    if !text.contains('%') {
        return text.to_owned();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = text.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

fn extension(source: &str) -> String {
    let clean = source.split(['?', '#']).next().unwrap_or(source);
    Path::new(clean).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

fn download(inner: &Inner, url: &str) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    url.hash(&mut hasher);
    let ext = extension(url);
    let name = if ext.is_empty() { format!("{:016x}", hasher.finish()) } else { format!("{:016x}.{ext}", hasher.finish()) };
    let target = inner.cache.join(name);
    if target.is_file() {
        let _ = std::fs::File::options().append(true).open(&target).and_then(|f| f.set_modified(std::time::SystemTime::now()));
        return Some(target);
    }
    std::fs::create_dir_all(&inner.cache).ok()?;
    let partial = target.with_extension("part");
    let response = aquilum_core::web_agent::web_agent("images").get(url).call().ok()?;
    let copied = std::fs::File::create(&partial).and_then(|mut file| std::io::copy(&mut response.into_reader(), &mut file));
    (copied.is_ok() && std::fs::rename(&partial, &target).is_ok()).then_some(target)
}

pub(super) fn load(inner: &Inner, source: &str, wiki: bool) -> Fetch<Media> {
    let video = crate::ui::editor::is_video(source);
    let remote = source.starts_with("http://") || source.starts_with("https://");
    let path = if remote {
        if video {
            return Fetch::Ready(Media { image: None, video, path: None });
        }
        download(inner, source)
    } else if wiki {
        aquilum_core::files::attachments::resolve_attachments_impl(&inner.root, vec![source.to_owned()])
            .into_iter()
            .next()
            .flatten()
            .map(|p| inner.root.join(p))
            .filter(|p| p.is_file())
    } else {
        locate(inner, source)
    };
    let Some(path) = path else { return Fetch::Missing(source.to_owned()) };
    let image = if video { crate::poster::video(&path) } else { crate::images::load(&path) };
    if image.is_none() && !video {
        return Fetch::Missing(source.to_owned());
    }
    Fetch::Ready(Media { image, video, path: Some(path) })
}
