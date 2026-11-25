use crate::text::is_cyrillic;
use rust_stemmers::{Algorithm, Stemmer as Snowball};
use std::borrow::Cow;
use std::mem;
use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

const MIN_STEM_CHARS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Script {
    Cyrillic,
    Latin,
}

#[derive(Clone, Default)]
pub struct StemFilter;

impl TokenFilter for StemFilter {
    type Tokenizer<T: Tokenizer> = StemTokenizer<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> Self::Tokenizer<T> {
        StemTokenizer {
            inner: tokenizer,
            buffer: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct StemTokenizer<T> {
    inner: T,
    buffer: String,
}

impl<T: Tokenizer> Tokenizer for StemTokenizer<T> {
    type TokenStream<'a> = StemTokenStream<'a, T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        self.buffer.clear();
        StemTokenStream {
            tail: self.inner.token_stream(text),
            cyrillic: Snowball::create(Algorithm::Russian),
            latin: Snowball::create(Algorithm::English),
            buffer: &mut self.buffer,
        }
    }
}

pub struct StemTokenStream<'a, T> {
    tail: T,
    cyrillic: Snowball,
    latin: Snowball,
    buffer: &'a mut String,
}

impl<T: TokenStream> TokenStream for StemTokenStream<'_, T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let token = self.tail.token_mut();
        match stem_script(&token.text) {
            Some(Script::Cyrillic) => apply_stem(&self.cyrillic, token, self.buffer),
            Some(Script::Latin) => apply_stem(&self.latin, token, self.buffer),
            None => {}
        }
        true
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

fn stem_script(text: &str) -> Option<Script> {
    let mut chars = 0usize;
    let mut cyrillic = false;
    let mut latin = false;
    for ch in text.chars() {
        if is_cyrillic(ch) {
            cyrillic = true;
        } else if ch.is_ascii_alphabetic() {
            latin = true;
        } else {
            return None;
        }
        chars += 1;
    }
    if chars < MIN_STEM_CHARS || (cyrillic && latin) {
        return None;
    }
    if cyrillic {
        Some(Script::Cyrillic)
    } else if latin {
        Some(Script::Latin)
    } else {
        None
    }
}

fn apply_stem(stemmer: &Snowball, token: &mut Token, buffer: &mut String) {
    match stemmer.stem(&token.text) {
        Cow::Owned(stemmed) => token.text = stemmed,
        Cow::Borrowed(stemmed) => {
            buffer.clear();
            buffer.push_str(stemmed);
            mem::swap(&mut token.text, buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{stem_script, Script};

    #[test]
    fn selects_a_stemmer_only_for_long_single_script_words() {
        assert_eq!(stem_script("машина"), Some(Script::Cyrillic));
        assert_eq!(stem_script("programming"), Some(Script::Latin));
        assert_eq!(stem_script("на"), None);
        assert_eq!(stem_script("дом"), None);
        assert_eq!(stem_script("машинa"), None);
        assert_eq!(stem_script("abc123"), None);
    }
}
