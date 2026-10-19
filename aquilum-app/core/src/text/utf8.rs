use std::io;

pub fn prefix(bytes: &[u8], ended: bool) -> io::Result<&str> {
    match simdutf8::compat::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(error) if error.error_len().is_none() && !ended => prefix(&bytes[..error.valid_up_to()], true),
        Err(error) => Err(io::Error::new(io::ErrorKind::InvalidData, error)),
    }
}

pub fn string(bytes: Vec<u8>) -> io::Result<String> {
    prefix(&bytes, true)?;
    Ok(unsafe { String::from_utf8_unchecked(bytes) })
}

#[cfg(test)]
mod tests {
    use super::{prefix, string};

    #[test]
    fn a_split_letter_waits_for_the_next_block_and_a_broken_one_fails() {
        let bytes = "аб".as_bytes();
        assert_eq!(prefix(&bytes[..3], false).unwrap(), "а");
        assert!(prefix(&bytes[..3], true).is_err());
        assert!(prefix(b"a\xffb", false).is_err());
        assert_eq!(string(bytes.to_vec()).unwrap(), "аб");
        assert!(string(vec![0xd0]).is_err());
    }
}
