use super::fold::{expand, sources, Fold};

pub(super) struct Starts {
    chars: Vec<char>,
    bytes: [bool; 256],
    few: Vec<u8>,
}

impl Starts {
    pub(super) fn new(needles: &[Vec<char>], fold: Fold) -> Self {
        let mut chars = Vec::new();
        for first in needles.iter().map(|needle| needle[0]) {
            let mut keeps = false;
            expand(first, fold, |part| keeps |= part == first);
            if keeps {
                chars.push(first);
            }
            chars.extend_from_slice(sources(first, fold));
        }
        chars.sort_unstable();
        chars.dedup();
        let mut bytes = [false; 256];
        for character in &chars {
            let mut buffer = [0; 4];
            let encoded = character.encode_utf8(&mut buffer).as_bytes();
            bytes[usize::from(encoded[encoded.len() - 1])] = true;
        }
        let few = (0..=u8::MAX).filter(|&byte| bytes[usize::from(byte)]).collect();
        Self { chars, bytes, few }
    }

    pub(super) fn next(&self, text: &str, from: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut at = from;
        while at < bytes.len() {
            let rest = &bytes[at..];
            let hit = at + match self.few[..] {
                [a] => memchr::memchr(a, rest),
                [a, b] => memchr::memchr2(a, b, rest),
                [a, b, c] => memchr::memchr3(a, b, c, rest),
                _ => rest.iter().position(|&byte| self.bytes[usize::from(byte)]),
            }?;
            let mut start = hit;
            while start > from && bytes[start] & 0xC0 == 0x80 {
                start -= 1;
            }
            let character = text[start..].chars().next()?;
            if start + character.len_utf8() == hit + 1 && self.chars.binary_search(&character).is_ok() {
                return Some(start);
            }
            at = hit + 1;
        }
        None
    }
}
