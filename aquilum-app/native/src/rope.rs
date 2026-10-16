use crop::{Rope, RopeSlice};

pub fn floor(text: &Rope, at: usize) -> usize {
    let mut at = at.min(text.byte_len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

pub fn prefix(text: &Rope, len: usize) -> String {
    text.byte_slice(..floor(text, len)).to_string()
}

pub fn each_line(text: RopeSlice<'_>, mut line: impl FnMut(&str) -> bool) {
    let mut part = String::new();
    for chunk in text.chunks() {
        let mut rest = chunk;
        while let Some(i) = rest.find('\n') {
            let go = if part.is_empty() {
                line(&rest[..i])
            } else {
                part.push_str(&rest[..i]);
                let go = line(&part);
                part.clear();
                go
            };
            if !go {
                return;
            }
            rest = &rest[i + 1..];
        }
        part.push_str(rest);
    }
    line(&part);
}

pub fn common_prefix(a: &Rope, b: &Rope) -> usize {
    let (mut left, mut right) = (a.chunks().map(str::as_bytes), b.chunks().map(str::as_bytes));
    let (mut x, mut y): (&[u8], &[u8]) = (&[], &[]);
    let mut same = 0;
    loop {
        if x.is_empty() {
            match left.next() {
                Some(chunk) => x = chunk,
                None => return same,
            }
            continue;
        }
        if y.is_empty() {
            match right.next() {
                Some(chunk) => y = chunk,
                None => return same,
            }
            continue;
        }
        let n = x.len().min(y.len());
        let equal = x[..n].iter().zip(&y[..n]).take_while(|(p, q)| p == q).count();
        same += equal;
        if equal < n {
            return same;
        }
        (x, y) = (&x[n..], &y[n..]);
    }
}

#[cfg(test)]
mod tests {
    use super::{common_prefix, each_line};
    use crop::Rope;

    #[test]
    fn common_prefix_crosses_chunks() {
        let text: String = (0..5000).map(|i| format!("строка {i}\n")).collect();
        let (a, mut b) = (Rope::from(text.as_str()), Rope::from(text.as_str()));
        assert_eq!(common_prefix(&a, &b), text.len());
        b.insert(30_000, "x");
        assert_eq!(common_prefix(&a, &b), 30_000);
        assert_eq!(common_prefix(&a, &Rope::new()), 0);
    }

    #[test]
    fn each_line_matches_split() {
        let text: String = (0..3000).map(|i| format!("строка {i}\n")).collect::<String>() + "хвост";
        let mut lines = Vec::new();
        each_line(Rope::from(text.as_str()).byte_slice(..), |line| {
            lines.push(line.to_owned());
            true
        });
        assert_eq!(lines, text.split('\n').collect::<Vec<_>>());
    }
}
