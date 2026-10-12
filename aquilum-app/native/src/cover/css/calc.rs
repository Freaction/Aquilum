use super::types::Val;

pub fn value(text: &str, font: f64) -> Option<Val> {
    let tokens = tokenize(text)?;
    let mut parser = Calc { tokens, at: 0, font };
    let v = parser.expr()?;
    (parser.at == parser.tokens.len()).then_some(v)
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Num(f64, String),
    Op(char),
    Open,
    Close,
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    let text = text.trim();
    let text = text.strip_prefix("calc").unwrap_or(text);
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let unary = matches!(tokens.last(), None | Some(Token::Op(_)) | Some(Token::Open));
        if c.is_whitespace() {
            i += 1;
        } else if c == '(' {
            tokens.push(Token::Open);
            i += 1;
        } else if c == ')' {
            tokens.push(Token::Close);
            i += 1;
        } else if c.is_ascii_digit() || c == '.' || ((c == '-' || c == '+') && unary && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit() || *n == '.')) {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let number: f64 = chars[start..i].iter().collect::<String>().parse().ok()?;
            let unit_start = i;
            while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
                i += 1;
            }
            tokens.push(Token::Num(number, chars[unit_start..i].iter().collect()));
        } else if "+-*/".contains(c) {
            tokens.push(Token::Op(c));
            i += 1;
        } else {
            return None;
        }
    }
    Some(tokens)
}

struct Calc {
    tokens: Vec<Token>,
    at: usize,
    font: f64,
}

impl Calc {
    fn expr(&mut self) -> Option<Val> {
        let mut left = self.term()?;
        while let Some(Token::Op(op @ ('+' | '-'))) = self.tokens.get(self.at).cloned() {
            self.at += 1;
            let right = self.term()?;
            left = left.add(right, if op == '+' { 1.0 } else { -1.0 });
        }
        Some(left)
    }

    fn term(&mut self) -> Option<Val> {
        let mut left = self.factor()?;
        while let Some(Token::Op(op @ ('*' | '/'))) = self.tokens.get(self.at).cloned() {
            self.at += 1;
            let right = self.factor()?;
            left = match op {
                '*' if right.is_number() => left.scale(right.num),
                '*' if left.is_number() => right.scale(left.num),
                '/' if right.is_number() && right.num != 0.0 => left.scale(1.0 / right.num),
                _ => return None,
            };
        }
        Some(left)
    }

    fn factor(&mut self) -> Option<Val> {
        match self.tokens.get(self.at).cloned()? {
            Token::Open => {
                self.at += 1;
                let v = self.expr()?;
                (self.tokens.get(self.at) == Some(&Token::Close)).then_some(())?;
                self.at += 1;
                Some(v)
            }
            Token::Op('-') => {
                self.at += 1;
                Some(self.factor()?.scale(-1.0))
            }
            Token::Num(n, unit) => {
                self.at += 1;
                let mut v = Val::default();
                match unit.as_str() {
                    "" => v.num = n,
                    "px" => v.px = n,
                    "%" => v.pct = n,
                    "em" => v.px = n * self.font,
                    "deg" => v.deg = n,
                    "turn" => v.deg = n * 360.0,
                    _ => return None,
                }
                Some(v)
            }
            _ => None,
        }
    }
}
