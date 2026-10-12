fn matching(text: &str, open: usize, left: char, right: char) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        if c == left {
            depth += 1;
        } else if c == right {
            depth -= 1;
            if depth == 0 {
                return Some(open + i);
            }
        }
    }
    None
}

pub(super) fn block_end(text: &str, open: usize) -> Option<usize> {
    matching(text, open, '{', '}')
}

pub(super) fn paren_end(text: &str, open: usize) -> Option<usize> {
    matching(text, open, '(', ')')
}

pub(super) fn split_top(text: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        let split = if separator == ' ' { c.is_whitespace() } else { c == separator };
        if split && depth == 0 {
            if !current.trim().is_empty() {
                parts.push(current.trim().to_owned());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_owned());
    }
    parts
}
