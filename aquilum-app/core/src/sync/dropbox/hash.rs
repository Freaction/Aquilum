use crate::text::content::Content;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::io;

const HASH_BLOCK: usize = 4 * 1024 * 1024;

pub fn content_hash(content: &dyn Content) -> io::Result<String> {
    let mut whole = Sha256::new();
    for start in (0..content.byte_len()).step_by(HASH_BLOCK) {
        let mut block = Sha256::new();
        content.read(start..(start + HASH_BLOCK).min(content.byte_len()), &mut |bytes| {
            block.update(bytes);
            Ok(())
        })?;
        whole.update(block.finalize());
    }
    let mut hex = String::with_capacity(64);
    for byte in whole.finalize() {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}
