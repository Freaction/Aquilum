use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::Inner;
use crate::ui::editor::resolve::{Fetch, Media};

const RECENT: usize = 24;

static LOADED: Mutex<VecDeque<(String, PathBuf, bool)>> = Mutex::new(VecDeque::new());

pub(super) fn recent(key: &str) -> Option<Media> {
    let (path, video) = LOADED.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _, _)| k == key).map(|(_, p, v)| (p.clone(), *v))?;
    let picture = crate::images::cached(&path)?;
    Some(Media { image: Some(picture.image), natural: picture.natural, video, path: Some(path) })
}

#[cfg(test)]
pub(super) fn recent_count(prefix: &str) -> usize {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|(k, _, _)| k.starts_with(prefix)).count()
}

pub(super) fn forget_key(key: &str) {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).retain(|(k, _, _)| k != key);
}

pub(super) fn forget(paths: &[PathBuf]) {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).retain(|(_, p, _)| !paths.contains(p));
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

pub(super) fn remember(key: String, media: &Media) {
    let Some(path) = media.path.clone() else { return };
    let mut loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    loaded.retain(|(k, _, _)| *k != key);
    loaded.push_front((key, path, media.video));
    loaded.truncate(RECENT);
}

pub(super) fn locate(inner: &Inner, source: &str) -> Option<PathBuf> {
    let source = source.trim();
    let source = source.strip_prefix("file:///").or_else(|| source.strip_prefix("file://")).unwrap_or(source);
    let decoded = crate::paths::percent_decode(source);
    let path = Path::new(&decoded);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    let mut bases = std::iter::once(inner.root.as_path()).chain(inner.document.ancestors().skip(1).take_while(|d| d.starts_with(&inner.root)));
    bases.find_map(|base| Some(crate::paths::join(base, &decoded)).filter(|p| p.is_file()))
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

fn inline_image(source: &str) -> Option<Fetch<Media>> {
    use base64::Engine;
    let (header, data) = source.strip_prefix("data:image/")?.split_once(',')?;
    let picture = header.ends_with(";base64").then(|| base64::engine::general_purpose::STANDARD.decode(data.trim()).ok()).flatten().and_then(|bytes| crate::images::decode_shown(&bytes));
    Some(match picture {
        Some(picture) => Fetch::Ready(Media { image: Some(picture.image), natural: picture.natural, video: false, path: None }),
        None => Fetch::Missing(source.to_owned()),
    })
}

fn poster(path: &Path) -> Option<crate::images::Picture> {
    if let Some(picture) = crate::images::cached(path) {
        return Some(picture);
    }
    let image = crate::poster::video(path)?;
    let picture = crate::images::Picture { natural: (image.image.width, image.image.height), image };
    crate::images::remember(path, &picture);
    Some(picture)
}

pub(super) fn load(inner: &Inner, source: &str, wiki: bool) -> Fetch<Media> {
    if let Some(fetched) = inline_image(source) {
        return fetched;
    }
    let video = crate::ui::editor::is_video(source);
    let remote = source.starts_with("http://") || source.starts_with("https://");
    let path = if remote {
        if video {
            return Fetch::Ready(Media { image: None, natural: (0, 0), video, path: None });
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
    let picture = if video { poster(&path) } else { crate::images::picture(&path) };
    if picture.is_none() && !video {
        return Fetch::Missing(source.to_owned());
    }
    let natural = picture.as_ref().map_or((0, 0), |p| p.natural);
    Fetch::Ready(Media { image: picture.map(|p| p.image), natural, video, path: Some(path) })
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::{Fetch, inline_image};

    #[test]
    fn data_uri_images_are_decoded() {
        let mut png = Vec::new();
        image::RgbaImage::new(3, 2).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let source = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&png));
        assert!(matches!(inline_image(&source), Some(Fetch::Ready(media)) if media.image.is_some()));
        assert!(matches!(inline_image("data:image/png;base64,!!!"), Some(Fetch::Missing(_))));
        assert!(inline_image("pic.png").is_none());
    }
}
