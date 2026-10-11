use std::path::{Path, PathBuf};

pub fn percent_decode(text: &str) -> String {
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

pub fn join(base: &Path, relative: &str) -> PathBuf {
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

pub fn relative(root: &Path, path: &Path) -> Option<String> {
    let rest = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rest.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

pub fn absolute(root: &Path, relative: &str) -> PathBuf {
    root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_joins_and_round_trips() {
        assert_eq!(percent_decode("a%20b%D1%8F"), "a bя");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%FF"), "%FF");
        let root = Path::new("vault");
        assert_eq!(join(&root.join("notes"), "../img/./a.png"), root.join("img").join("a.png"));
        let path = absolute(root, "папка/заметка.md");
        assert_eq!(relative(root, &path).as_deref(), Some("папка/заметка.md"));
        assert_eq!(relative(root, root), None);
    }
}
