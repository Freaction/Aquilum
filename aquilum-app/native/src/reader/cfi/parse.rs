use super::{Cfi, Part, Path, Steps};

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Bang,
    Comma,
    Slash(u32),
    Colon(u32),
    Tilde(f64),
    At(f64),
    Bracket(String),
    Param(String, String),
}

fn int(value: &str) -> u32 {
    value.parse().unwrap_or(0)
}

fn float(value: &str) -> f64 {
    value.parse().unwrap_or(0.0)
}

fn tokenize(source: &str) -> Vec<Tok> {
    let mut tokens = Vec::new();
    let mut state: Option<String> = None;
    let mut escape = false;
    let mut value = String::new();
    let chars: Vec<Option<char>> = source.trim().chars().map(Some).chain([None]).collect();
    for ch in chars {
        if ch == Some('^') && !escape {
            escape = true;
            continue;
        }
        let digit = ch.is_some_and(|c| c.is_ascii_digit());
        let decimal = digit || ch == Some('.');
        match state.clone().as_deref() {
            Some("!") => {
                tokens.push(Tok::Bang);
                state = None;
                value.clear();
            }
            Some(",") => {
                tokens.push(Tok::Comma);
                state = None;
                value.clear();
            }
            Some(kind @ ("/" | ":")) => {
                if digit {
                    value.extend(ch);
                    escape = false;
                    continue;
                }
                tokens.push(if kind == "/" { Tok::Slash(int(&value)) } else { Tok::Colon(int(&value)) });
                state = None;
                value.clear();
            }
            Some("~") => {
                if decimal {
                    value.extend(ch);
                    escape = false;
                    continue;
                }
                tokens.push(Tok::Tilde(float(&value)));
                state = None;
                value.clear();
            }
            Some("@") => {
                if ch == Some(':') {
                    tokens.push(Tok::At(float(&value)));
                    value.clear();
                    state = Some("@".into());
                    continue;
                }
                if decimal {
                    value.extend(ch);
                    escape = false;
                    continue;
                }
                tokens.push(Tok::At(float(&value)));
                state = None;
                value.clear();
            }
            Some("[") => {
                match ch {
                    Some(';') if !escape => {
                        tokens.push(Tok::Bracket(std::mem::take(&mut value)));
                        state = Some(";".into());
                    }
                    Some(',') if !escape => {
                        tokens.push(Tok::Bracket(std::mem::take(&mut value)));
                        state = Some("[".into());
                    }
                    Some(']') if !escape => {
                        tokens.push(Tok::Bracket(std::mem::take(&mut value)));
                        state = None;
                    }
                    _ => {
                        value.extend(ch);
                        escape = false;
                    }
                }
                continue;
            }
            Some(param) if param.starts_with(';') => {
                match ch {
                    Some('=') if !escape => {
                        state = Some(format!(";{value}"));
                        value.clear();
                    }
                    Some(';') if !escape => {
                        tokens.push(Tok::Param(param.to_owned(), std::mem::take(&mut value)));
                        state = Some(";".into());
                    }
                    Some(']') if !escape => {
                        tokens.push(Tok::Param(param.to_owned(), std::mem::take(&mut value)));
                        state = None;
                    }
                    _ => {
                        value.extend(ch);
                        escape = false;
                    }
                }
                continue;
            }
            _ => {}
        }
        if let Some(c @ ('/' | ':' | '~' | '@' | '[' | '!' | ',')) = ch {
            state = Some(c.to_string());
        }
    }
    tokens
}

fn parts(tokens: &[Tok]) -> Path {
    let mut parts: Path = Vec::new();
    let mut after_slash = false;
    for token in tokens {
        if let Tok::Slash(index) = token {
            parts.push(Part { index: *index, ..Part::default() });
            after_slash = true;
            continue;
        }
        let Some(last) = parts.last_mut() else { continue };
        match token {
            Tok::Colon(offset) => last.offset = Some(*offset),
            Tok::Tilde(t) => last.temporal = Some(*t),
            Tok::At(v) => last.spatial.push(*v),
            Tok::Param(key, value) if key == ";s" => last.side = Some(value.clone()),
            Tok::Bracket(value) => {
                if after_slash && !value.is_empty() {
                    last.id = Some(value.clone());
                } else {
                    last.text.push(value.clone());
                    continue;
                }
            }
            _ => {}
        }
        after_slash = false;
    }
    parts
}

fn split<'a>(tokens: &'a [Tok], at: &Tok) -> Vec<&'a [Tok]> {
    tokens.split(|t| t == at).collect()
}

fn steps(tokens: &[Tok]) -> Steps {
    split(tokens, &Tok::Bang).into_iter().map(parts).collect()
}

pub(super) fn unwrap(source: &str) -> &str {
    source.trim().strip_prefix("epubcfi(").and_then(|s| s.strip_suffix(')')).unwrap_or(source)
}

pub fn parse(source: &str) -> Cfi {
    let tokens = tokenize(unwrap(source));
    let groups = split(&tokens, &Tok::Comma);
    match groups.as_slice() {
        [parent, start, end] => Cfi::Range { parent: steps(parent), start: steps(start), end: steps(end) },
        _ => Cfi::Point(steps(&tokens)),
    }
}
