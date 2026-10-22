use super::stems;

pub const MAX_WORDS: usize = 32;

#[derive(Debug, Default, PartialEq)]
pub struct Word {
    pub stem: String,
    pub headings: bool,
    pub exact: bool,
    pub prefix: bool,
}

impl Word {
    pub fn required(&self) -> bool {
        self.headings || self.exact
    }
}

#[derive(Debug, Default)]
pub struct Parsed {
    pub words: Vec<Word>,
    pub phrases: Vec<Vec<String>>,
    pub excluded: Vec<Vec<String>>,
}

pub fn parse(query: &str) -> Parsed {
    let mut parsed = Parsed::default();
    let mut last = None;
    let mut rest = query.trim_start();
    while !rest.is_empty() {
        let negated = rest.len() > 1 && rest.starts_with('-');
        let body = if negated { &rest[1..] } else { rest };
        let (piece, quoted, after) = match body.strip_prefix('"') {
            Some(inner) => {
                let end = inner.find('"').unwrap_or(inner.len());
                (&inner[..end], true, inner.get(end + 1..).unwrap_or(""))
            }
            None => {
                let end = body.find(|c: char| c.is_whitespace() || c == '"').unwrap_or(body.len());
                (&body[..end], false, &body[end..])
            }
        };
        let headings = !quoted && piece.starts_with('#');
        let stems = stems(piece.trim_start_matches('#'));
        last = None;
        if negated {
            if !stems.is_empty() {
                parsed.excluded.push(stems);
            }
        } else {
            if quoted && stems.len() > 1 {
                parsed.phrases.push(stems.clone());
            }
            for stem in stems {
                last = parsed.add(Word { stem, headings, exact: quoted, prefix: false });
            }
            last = last.filter(|_| !quoted);
        }
        rest = after.trim_start();
    }
    if let Some(index) = last {
        parsed.words[index].prefix = true;
    }
    parsed
}

impl Parsed {
    fn add(&mut self, word: Word) -> Option<usize> {
        if let Some(index) = self.words.iter().position(|known| *known == word) {
            return Some(index);
        }
        (self.words.len() < MAX_WORDS).then(|| {
            self.words.push(word);
            self.words.len() - 1
        })
    }
}
