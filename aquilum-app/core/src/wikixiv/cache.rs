use blake3::Hash;

pub fn content_hash(document_path: Option<&str>, text: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    if let Some(path) = document_path {
        hasher.update(path.as_bytes());
    }
    hasher.update(b"\0");
    hasher.update(text.as_bytes());
    hasher.finalize()
}
