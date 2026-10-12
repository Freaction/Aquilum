use super::EXTENSIONS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

const VIDEO: [&str; 7] = ["mp4", "webm", "ogv", "ogg", "mov", "m4v", "mkv"];

pub fn extension(source: &str) -> Option<&str> {
    let path = source.split(['?', '#']).next()?;
    let name = path.rsplit(['/', '\\']).next()?;
    name.rsplit_once('.').filter(|(stem, extension)| !stem.is_empty() && !extension.is_empty()).map(|(_, extension)| extension)
}

pub fn kind(source: &str) -> Option<MediaKind> {
    let extension = extension(source)?;
    let among = |list: &[&str]| list.iter().any(|known| extension.eq_ignore_ascii_case(known));
    if among(&EXTENSIONS) {
        Some(MediaKind::Image)
    } else if among(&VIDEO) {
        Some(MediaKind::Video)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_decodable_images_and_playable_videos_have_a_kind() {
        for image in ["a.png", "Фото.JPG", "a.jpeg", "a.gif", "a.webp", "a.bmp", "https://x.io/a.png?w=1#top", "Files/image 6.png", r"C:\Файлы\a.PNG"] {
            assert_eq!(kind(image), Some(MediaKind::Image), "{image}");
        }
        for video in ["a.mp4", "a.WEBM", "a.ogv", "a.ogg", "a.mov", "a.m4v", "clip.mkv", "https://x.io/v.mp4?t=1"] {
            assert_eq!(kind(video), Some(MediaKind::Video), "{video}");
        }
        for other in ["a.svg", "a.avif", "a.pdf", "note.md", "без расширения", "папка.d/файл", ".png", "a.png.part", "https://x.io/photo", ""] {
            assert_eq!(kind(other), None, "{other}");
        }
    }

    #[test]
    fn extension_ignores_query_and_fragment() {
        assert_eq!(extension("https://x.io/a.tar.gz?x=1.png#y.mp4"), Some("gz"));
        assert_eq!(extension("https://x.io/photo"), None);
        assert_eq!(extension("a."), None);
    }
}
